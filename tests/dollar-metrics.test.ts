import assert from "node:assert/strict";
import test from "node:test";
import { parseCnbcBars } from "../lib/btc/data/cnbc";
import { buildDashboard, FALLBACK, type RawData } from "../lib/metrics";

const NOTHING = Object.fromEntries(Object.keys(FALLBACK).map((k) => [k, null])) as RawData;
const metric = (raw: RawData, id: string) => buildDashboard(raw).metrics.find((m) => m.id === id)!;

test("with no source answering, every card says it is a fallback and gives its date", () => {
  const data = buildDashboard(NOTHING);
  assert.equal(data.fallbackCount, data.metrics.length);
  for (const m of data.metrics) {
    assert.ok(m.fallback, m.id);
    assert.match(m.context, /fallback snapshot · \d+ \w+/, m.id);
  }
  assert.equal(data.oldestDate, "2026-09-02");
});

test("the fallback dollar index is on the same scale as the live broad index", () => {
  // The live series is FRED DTWEXBGS (~120). An ICE DXY value (~100) here would
  // silently change what the card measures whenever FRED fails.
  assert.ok(FALLBACK.dxyBroad!.value > 110);
  assert.match(metric(NOTHING, "dxy").context, /not ICE DXY/);
});

test("a live reading keeps its own date even when the note changes", () => {
  const raw: RawData = {
    ...NOTHING,
    gold: { date: "2026-11-11", value: 4012.4 },
    bitcoin: { date: "2026-11-11", value: 73_515 },
    bojRate: { date: "2026-11-11", value: 1.25 },
    jgb10: { date: "2026-11-11", value: 2.807 },
  };
  const gold = metric(raw, "gold");
  assert.equal(gold.fallback, false);
  assert.equal(gold.context, "haven / debasement bid · as of 11 Nov");
  assert.equal(metric(raw, "bitcoin").context, "CoinGecko spot · as of 11 Nov");
  const boj = metric(raw, "bojrate");
  assert.equal(boj.value, "1.25%");
  assert.equal(boj.status, "stressed");
  assert.equal(metric(raw, "jgb10").value, "2.81%");
  const data = buildDashboard(raw);
  assert.equal(data.snapshotDate, "2026-11-11");
  assert.equal(data.fallbackCount, data.metrics.length - 4);
});

test("a derived metric is a fallback if either input is", () => {
  const raw: RawData = { ...NOTHING, ust10: { date: "2026-11-10", value: 4.98 } };
  const curve = metric(raw, "curve");
  assert.ok(curve.fallback);
  assert.equal(curve.asOf, "2026-09-02");
});

test("CNBC bars become one close per UTC day", () => {
  const bars = parseCnbcBars({
    barData: {
      priceBars: [
        { close: "106.0240", tradeTimeinMills: 1731387600000 },
        { close: "106.4810", tradeTimeinMills: 1731474000000 },
        { close: "106.5000", tradeTimeinMills: 1731474000000 },
        { close: "bad", tradeTimeinMills: 1731560400000 },
      ],
    },
  });
  assert.deepEqual(bars.map((b) => b.v), [106.024, 106.5]);
  assert.ok(bars[0].t % 86_400 === 0 && bars[1].t - bars[0].t === 86_400);
  assert.deepEqual(parseCnbcBars({}), []);
});
