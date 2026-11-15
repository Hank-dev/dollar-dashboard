import assert from "node:assert/strict";
import test from "node:test";
import { cachedRoute, createCache } from "../lib/routeCache";

test("a result is reused until its TTL passes", async () => {
  let now = 0;
  let loads = 0;
  const cache = createCache();
  const opts = { ttlSeconds: 60, now: () => now };
  const load = async () => ++loads;
  assert.deepEqual(await cache.get("k", opts, load), { value: 1, storedAt: 0, status: "fresh" });
  now = 59_000;
  assert.equal((await cache.get("k", opts, load)).status, "cached");
  now = 60_000;
  assert.equal((await cache.get("k", opts, load)).value, 2);
});

test("concurrent callers share one load", async () => {
  let loads = 0;
  const cache = createCache();
  const load = async () => {
    loads += 1;
    await new Promise((r) => setTimeout(r, 5));
    return "paid call";
  };
  const results = await Promise.all(
    Array.from({ length: 5 }, () => cache.get("k", { ttlSeconds: 60 }, load)),
  );
  assert.equal(loads, 1);
  assert.ok(results.every((r) => r.value === "paid call"));
});

test("a failure is never stored and falls back to the last good value", async () => {
  let now = 0;
  let fail = false;
  const cache = createCache();
  const opts = { ttlSeconds: 60, now: () => now };
  const load = async () => {
    if (fail) throw new Error("upstream down");
    return "good";
  };
  await assert.rejects(
    cache.get("empty", opts, async () => {
      throw new Error("upstream down");
    }),
  );
  assert.equal(cache.size(), 0);
  await cache.get("k", opts, load);
  now = 61_000;
  fail = true;
  assert.deepEqual(await cache.get("k", opts, load), { value: "good", storedAt: 0, status: "stale" });
  fail = false;
  assert.equal((await cache.get("k", opts, load)).status, "fresh");
});

test("a degraded result is kept only briefly", async () => {
  let now = 0;
  let loads = 0;
  const cache = createCache();
  const opts = {
    ttlSeconds: 86_400,
    degradedTtlSeconds: 120,
    degraded: (v: { complete: boolean }) => !v.complete,
    now: () => now,
  };
  const load = async () => ({ complete: ++loads > 1 });
  await cache.get("k", opts, load);
  now = 121_000;
  assert.equal((await cache.get("k", opts, load)).value.complete, true);
  now = 3_600_000;
  assert.equal((await cache.get("k", opts, load)).status, "cached");
  assert.equal(loads, 2);
});

test("a route keeps 200 responses and retries everything else", async () => {
  let calls = 0;
  const GET = cachedRoute("test-route", 60, async () => {
    calls += 1;
    return calls === 1
      ? Response.json({ error: "FRED_API_KEY env var is required" }, { status: 502 })
      : Response.json({ value: calls });
  });
  const first = await GET();
  assert.equal(first.status, 502);
  const second = await GET();
  assert.equal(second.status, 200);
  assert.equal(second.headers.get("X-Cache"), "fresh");
  const third = await GET();
  assert.deepEqual(await third.json(), { value: 2 });
  assert.equal(third.headers.get("X-Cache"), "cached");
  assert.equal(calls, 2);
});
