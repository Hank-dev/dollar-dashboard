// Spend controls for the routes that call a paid model API. The app has no
// user accounts, so these bound what an anonymous caller can cost: a missing
// key is reported instead of thrown, questions are length-capped, and calls
// are limited per client and per day.

export const MAX_QUESTION_CHARS = 1000;

export interface LimiterOptions {
  perClientPerHour: number;
  globalPerDay: number;
  now?: () => number;
}

const HOUR_MS = 3_600_000;
const DAY_MS = 24 * HOUR_MS;

export function createLimiter(opts: LimiterOptions) {
  const now = opts.now ?? Date.now;
  const clients = new Map<string, { windowStart: number; count: number }>();
  let day = { windowStart: now(), count: 0 };

  /** Seconds to wait when the call is refused, or null when it may proceed. */
  function take(client: string): number | null {
    const t = now();
    if (t - day.windowStart >= DAY_MS) day = { windowStart: t, count: 0 };
    if (day.count >= opts.globalPerDay) {
      return Math.ceil((day.windowStart + DAY_MS - t) / 1000);
    }
    let slot = clients.get(client);
    if (!slot || t - slot.windowStart >= HOUR_MS) {
      slot = { windowStart: t, count: 0 };
      clients.set(client, slot);
    }
    if (slot.count >= opts.perClientPerHour) {
      return Math.ceil((slot.windowStart + HOUR_MS - t) / 1000);
    }
    slot.count += 1;
    day.count += 1;
    if (clients.size > 5000) {
      for (const [id, s] of clients) {
        if (t - s.windowStart >= HOUR_MS) clients.delete(id);
      }
    }
    return null;
  }

  return { take };
}

function envInt(name: string, fallback: number): number {
  const raw = Number(process.env[name]);
  return Number.isFinite(raw) && raw > 0 ? Math.floor(raw) : fallback;
}

const limiter = createLimiter({
  perClientPerHour: envInt("LLM_PER_CLIENT_HOURLY_LIMIT", 20),
  globalPerDay: envInt("LLM_DAILY_LIMIT", 200),
});

export function clientId(req: Request): string {
  const forwarded = req.headers.get("x-forwarded-for")?.split(",")[0]?.trim();
  return forwarded || req.headers.get("x-real-ip") || "direct";
}

export function hasAnthropicKey(): boolean {
  const key = process.env.ANTHROPIC_API_KEY?.trim();
  return Boolean(key) && key !== "sk-ant-...";
}

function refusal(status: number, error: string, retryAfter?: number): Response {
  return Response.json(
    { error },
    {
      status,
      headers: retryAfter ? { "Retry-After": String(retryAfter) } : undefined,
    },
  );
}

/** A response to return instead of calling the model, or null to proceed. */
export function llmGuard(req: Request): Response | null {
  if (!hasAnthropicKey()) {
    return refusal(503, "AI features are not configured on this server (ANTHROPIC_API_KEY).");
  }
  const wait = limiter.take(clientId(req));
  if (wait != null) {
    return refusal(429, "AI request limit reached. Try again later.", wait);
  }
  return null;
}

/** The validated question, or the 400 response to return. */
export async function readQuestion(req: Request): Promise<string | Response> {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return refusal(400, "Request body must be JSON.");
  }
  const question = (body as { question?: unknown } | null)?.question;
  if (typeof question !== "string" || question.trim() === "") {
    return refusal(400, "Missing question.");
  }
  if (question.length > MAX_QUESTION_CHARS) {
    return refusal(400, `Question is too long (max ${MAX_QUESTION_CHARS} characters).`);
  }
  return question.trim();
}

/** Parse a JSON body without throwing; null when it is not an object. */
export async function readJson(req: Request): Promise<Record<string, unknown> | null> {
  try {
    const body: unknown = await req.json();
    return body && typeof body === "object" ? (body as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}
