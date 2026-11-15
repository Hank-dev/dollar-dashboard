# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

## Commands

```bash
npm run dev      # Next dev server at http://localhost:3000
npm run build    # production build (also the fastest full type-check)
npm start        # serve the production build
npm run lint     # ESLint (flat config: eslint-config-next core-web-vitals + typescript)
npm test         # node:test + tsx over tests/*.test.ts; net-liquidity fixtures are shared with the Rust app
```

The focused data-contract tests use Node's built-in test runner through `tsx`. `testdata/net-liquidity.json` is consumed by both TypeScript and Rust. Verify web changes with `npm test`, `npm run build` (type-check), and `npm run lint`; run `cargo test --offline` in `desktop/` for desktop changes.

## Environment

Copy `.env.example` → `.env.local`. Keys are read **server-side only** in route handlers and `lib/` — never prefix with `NEXT_PUBLIC_`.

- `ANTHROPIC_API_KEY` — required for the Explain/Ask/briefing features. Models default to `claude-sonnet-4-6`, overridable via `EXPLAIN_MODEL` / `ASK_MODEL`.
- `FRED_API_KEY` — St. Louis Fed. Without it, FRED-sourced cards on `/dollar` use `FALLBACK` in `lib/metrics.ts`; every such card is labelled "fallback snapshot" with its date. Gold, Japan 10Y, the BOJ rate and Bitcoin have keyless live sources.
- `LLM_PER_CLIENT_HOURLY_LIMIT` / `LLM_DAILY_LIMIT` — spend limits for the unauthenticated ask/explain routes (defaults 20 and 200). `SEC_CONTACT` — contact string for the EDGAR User-Agent.
- `XAI_API_KEY` / `XAI_SUMMARY_MODEL` — optional, powers the Grok "X summary" panels on `/ai` and `/nuclear`.

Every surface degrades gracefully: dashboards render even when keys are missing or upstream APIs fail.

## Architecture

A single Next.js 16 (App Router) + React 19 app — internally "Market Monitor v2", despite the `dollar-dashboard` repo name. Tailwind v4, TypeScript strict. Path alias `@/*` maps to the repo root.

### Four dashboards

Routes under `app/`, switched via `components/TopNav.tsx`. Each has an accent color (`--dash-btc/dollar/ai/nuclear`):

- `/` — home: ticker strip, dashboard cards, AI daily briefing.
- `/btc` — the largest surface; a "BTC terminal" of ~14 panels (`components/btc/`).
- `/dollar` — the original macro/dollar dashboard.
- `/ai` — frontier-model intelligence/price chart + live X narrative.
- `/nuclear` — nuclear-energy market panel + X narrative.

### Two data-flow patterns (know which one you're touching)

1. **Server component** — used by `/dollar`. The page is `async` and `force-dynamic`, and calls `getDashboardData()` from `lib/fetchers.ts`, which keeps its own 15-minute cache. Data arrives as props; `lib/fetchers.ts` is `"server-only"`, the pure builder is `lib/metrics.ts`.
2. **Client component + SWR → API route** — used by `/btc`, `/ai`, `/nuclear` panels. Panels are `"use client"`, fetch their own `/api/*` route via `useSWR`.

**Never use `export const revalidate` on a route handler or data page.** It prerenders during `next build`, where the Docker image has no keys, and serves the baked "missing key" response until the window passes. Routes are `force-dynamic` and cache through `lib/routeCache.ts`: `cachedRoute(key, ttl, handler)` for GET handlers, `cached(key, opts, load)` for a single paid call. Only complete results are kept for the full TTL, failures are never stored, a failed refresh serves the last good value, and concurrent callers share one load.

**Every route that calls a paid model goes through the guards.** Ask routes use `answerQuestion` and explain routes use `explainItem` (both in `lib/llmRoutes.ts`), which apply `lib/llmGuard.ts`: key check, 1000-character question cap, per-client and daily limits. Briefing, verdict and interpretation are GET routes cached for hours through `cached`. The X-summary routes share `lib/xSummaryRoute.ts`. Model ids live in `MODELS` in `lib/anthropic.ts`.

The two AI text endpoints differ: explain routes return JSON (non-streaming); ask routes stream `text/plain` via `anthropic.messages.stream(...)` piped into a `ReadableStream`.

### lib/ layout

- Top level: `anthropic.ts` (shared SDK client), `fetchers.ts` + `metrics.ts` + `prompts.ts` (the `/dollar` dataset, status model, and Claude system prompts), plus parallel `aiMetrics`/`aiPrompts`/`frontierModels`, `nuclearMetrics`/`nuclearPrompts`/`nuclearLive`, and `xSummary`.
- `lib/btc/` is a self-contained subsystem: `data/` (one file per upstream API), `calc/` (powerLaw, regimeScore, marketEmotion, netLiquidity, correlation, indicators), `types.ts`, `format.ts`. BTC API routes compose `data/` + `calc/`.

### Conventions & gotchas

- **Two dollar indices, never mixed.** FRED `DTWEXBGS` is the broad trade-weighted index (~120) and is labelled "Broad USD"; ICE DXY (~100) comes from CNBC (`lib/btc/data/cnbc.ts`) and is the only series labelled "DXY". Thresholds written for one do not apply to the other.
- **A missing input is not a zero.** The regime score reports no composite and no allocation unless all five inputs answered (web and desktop), like net liquidity.
- **AI prompts are snapshot-grounded.** `lib/prompts.ts` injects the live metric JSON into the system prompt and instructs Claude not to invent figures or imply real-time knowledge. Keep that contract when editing prompts.
- **Resilience is the norm.** External fetches use try/catch or `Promise.allSettled` and fall back to cached/hardcoded values. Preserve this — don't let one failed upstream break a dashboard.
- **US-runtime geoblocking is a known constraint.** Several exchange APIs (Binance, Bybit) return 451/403 from US-hosted servers, so spot data uses Kraken/Coinbase instead (see the comment header in `lib/btc/data/binanceKlines.ts`). File/function names were kept for caller compatibility even though the upstream changed.
- **Theming is CSS custom properties**, defined in `app/globals.css` (light + dark). Use `var(--...)` tokens rather than hardcoded colors; `.mono` = JetBrains Mono.
