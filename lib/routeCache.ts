// In-process TTL cache for route handlers and paid upstream calls.
//
// Why not `export const revalidate`: that prerenders the handler during
// `next build`, where the Docker image has no keys, and serves the baked
// "missing key" response until the window passes. Why not the fetch data
// cache: `dynamic = "force-dynamic"` turns it off. So handlers run at request
// time and keep their own result here.
//
// Only complete results are kept for the full TTL. Degraded ones get a short
// TTL, failures are never stored, and a failure falls back to the last good
// value when there is one. Concurrent callers share one load.

type Entry = { value: unknown; storedAt: number; expiresAt: number };

export interface CacheOptions<T> {
  ttlSeconds: number;
  /** A result that is usable but incomplete; kept only for degradedTtlSeconds. */
  degraded?: (value: T) => boolean;
  degradedTtlSeconds?: number;
  now?: () => number;
}

export interface Cached<T> {
  value: T;
  storedAt: number;
  /** "stale" means the load failed and an expired value was returned instead. */
  status: "fresh" | "cached" | "stale";
}

const MAX_ENTRIES = 300;

export function createCache() {
  const store = new Map<string, Entry>();
  const inflight = new Map<string, Promise<Cached<unknown>>>();

  async function get<T>(
    key: string,
    opts: CacheOptions<T>,
    load: () => Promise<T>,
  ): Promise<Cached<T>> {
    const now = opts.now ?? Date.now;
    const hit = store.get(key);
    if (hit && hit.expiresAt > now()) {
      return { value: hit.value as T, storedAt: hit.storedAt, status: "cached" };
    }
    const pending = inflight.get(key);
    if (pending) return pending as Promise<Cached<T>>;

    const run = (async (): Promise<Cached<T>> => {
      try {
        const value = await load();
        const storedAt = now();
        const ttl =
          opts.degraded?.(value) === true
            ? Math.min(opts.ttlSeconds, opts.degradedTtlSeconds ?? 120)
            : opts.ttlSeconds;
        store.delete(key);
        store.set(key, { value, storedAt, expiresAt: storedAt + ttl * 1000 });
        while (store.size > MAX_ENTRIES) {
          const oldest = store.keys().next().value;
          if (oldest === undefined) break;
          store.delete(oldest);
        }
        return { value, storedAt, status: "fresh" };
      } catch (err) {
        const stale = store.get(key);
        if (stale) {
          return { value: stale.value as T, storedAt: stale.storedAt, status: "stale" };
        }
        throw err;
      } finally {
        inflight.delete(key);
      }
    })();
    inflight.set(key, run as Promise<Cached<unknown>>);
    return run;
  }

  return { get, clear: () => store.clear(), size: () => store.size };
}

const shared = createCache();

export function cached<T>(
  key: string,
  opts: CacheOptions<T>,
  load: () => Promise<T>,
): Promise<Cached<T>> {
  return shared.get(key, opts, load);
}

class NotCacheable extends Error {
  constructor(readonly response: StoredResponse) {
    super("response not cacheable");
  }
}

type StoredResponse = { status: number; body: string; headers: [string, string][] };

export interface RouteCacheOptions {
  /** Parsed JSON body of a 200 response that should only be kept briefly. */
  degraded?: (body: unknown) => boolean;
  degradedTtlSeconds?: number;
}

/**
 * Wrap a GET handler: 200 responses are kept for `ttlSeconds`, anything else
 * is returned as-is and retried on the next request (or replaced by the last
 * good response, marked `X-Cache: stale`).
 */
export function cachedRoute(
  key: string,
  ttlSeconds: number,
  handler: () => Promise<Response>,
  opts: RouteCacheOptions = {},
): () => Promise<Response> {
  return async function GET() {
    try {
      const hit = await cached<StoredResponse>(
        `route:${key}`,
        {
          ttlSeconds,
          degradedTtlSeconds: opts.degradedTtlSeconds,
          degraded: opts.degraded
            ? (stored) => {
                try {
                  return opts.degraded!(JSON.parse(stored.body));
                } catch {
                  return true;
                }
              }
            : undefined,
        },
        async () => {
          const res = await handler();
          const stored: StoredResponse = {
            status: res.status,
            body: await res.text(),
            headers: [...res.headers.entries()],
          };
          if (res.status !== 200) throw new NotCacheable(stored);
          return stored;
        },
      );
      const headers = new Headers(hit.value.headers);
      headers.set("X-Cache", hit.status);
      return new Response(hit.value.body, { status: hit.value.status, headers });
    } catch (err) {
      if (err instanceof NotCacheable) {
        return new Response(err.response.body, {
          status: err.response.status,
          headers: err.response.headers,
        });
      }
      return Response.json(
        { error: err instanceof Error ? err.message : String(err) },
        { status: 502 },
      );
    }
  };
}
