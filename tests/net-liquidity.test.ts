import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { computeNetLiquidity } from "../lib/btc/calc/netLiquidity";

type Point = { t: number; v: number };
type LiquidityCase = {
  name: string;
  walcl: Point[];
  tga: Point[];
  rrp: Point[];
  expected: Point[];
};

// Rust consumes this same fixture, so the two clients share the source-unit contract.
const fixture = JSON.parse(
  readFileSync(new URL("../testdata/net-liquidity.json", import.meta.url), "utf8"),
) as { cases: LiquidityCase[] };

assert.ok(fixture.cases.length > 0, "the shared liquidity contract must have cases");
for (const example of fixture.cases) {
  test(example.name, () => {
    const actual = computeNetLiquidity(example.walcl, example.tga, example.rrp);
    assert.equal(actual.length, example.expected.length);
    for (let i = 0; i < actual.length; i++) {
      assert.equal(actual[i].t, example.expected[i].t);
      assert.ok(
        Math.abs(actual[i].v - example.expected[i].v) < 1e-9,
        `at ${actual[i].t}: expected ${example.expected[i].v} billion, got ${actual[i].v}`,
      );
    }
  });
}
