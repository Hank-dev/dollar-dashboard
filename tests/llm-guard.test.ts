import assert from "node:assert/strict";
import test from "node:test";
import { createLimiter, MAX_QUESTION_CHARS, readQuestion } from "../lib/llmGuard";

const post = (body: string) =>
  new Request("http://localhost/api/ask", { method: "POST", body });

test("each client gets an hourly allowance", () => {
  let now = 0;
  const limiter = createLimiter({ perClientPerHour: 2, globalPerDay: 100, now: () => now });
  assert.equal(limiter.take("a"), null);
  assert.equal(limiter.take("a"), null);
  assert.equal(limiter.take("a"), 3600);
  assert.equal(limiter.take("b"), null);
  now = 3_600_000;
  assert.equal(limiter.take("a"), null);
});

test("the daily cap bounds total spend whatever the client says it is", () => {
  let now = 0;
  const limiter = createLimiter({ perClientPerHour: 100, globalPerDay: 3, now: () => now });
  for (const client of ["a", "b", "c"]) assert.equal(limiter.take(client), null);
  assert.equal(limiter.take("d"), 86_400);
  now = 86_400_000;
  assert.equal(limiter.take("d"), null);
});

test("questions are validated before any model call", async () => {
  assert.equal(await readQuestion(post(JSON.stringify({ question: "  Why is the VIX calm?  " }))), "Why is the VIX calm?");
  for (const body of ["not json", "{}", JSON.stringify({ question: 5 }), JSON.stringify({ question: "   " })]) {
    const res = await readQuestion(post(body));
    assert.ok(res instanceof Response && res.status === 400, body);
  }
  const long = await readQuestion(post(JSON.stringify({ question: "x".repeat(MAX_QUESTION_CHARS + 1) })));
  assert.ok(long instanceof Response && long.status === 400);
});
