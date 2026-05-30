# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

## Commands

```bash
npm run dev      # Next dev server at http://localhost:3000
npm run build    # production build (also the fastest full type-check)
npm start        # serve the production build
npm run lint     # ESLint (flat config: eslint-config-next core-web-vitals + typescript)
```

There is **no test framework** configured (no test script, no jest/vitest). Verify changes with `npm run build` (type-check) and `npm run lint`. There is no single-test command.

## Environment

Copy `.env.example` → `.env.local`. Keys are read **server-side only** in route handlers and `lib/` — never prefix with `NEXT_PUBLIC_`.

- `ANTHROPIC_API_KEY` — required for the Explain/Ask/briefing features. Models default to `claude-sonnet-4-6`, overridable via `EXPLAIN_MODEL` / `ASK_MODEL`.
- `FRED_API_KEY` — St. Louis Fed. Without it, the dollar dashboard falls back to a hardcoded snapshot (`FALLBACK` in `lib/metrics.ts`).
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

1. **Server component + ISR** — used by `/dollar`. The page is `async`, calls `getDashboardData()` from `lib/fetchers.ts`, and sets `export const revalidate = 900`. Data arrives as props; `lib/fetchers.ts` is `"server-only"`.
2. **Client component + SWR → API route** — used by `/btc`, `/ai`, `/nuclear` panels. Panels are `"use client"`, fetch their own `/api/*` route via `useSWR`. Most route handlers set `export const dynamic = "force-dynamic"` and do their own in-memory caching + `Cache-Control` headers (see `app/api/ai/x-summary/route.ts` for the module-level cache pattern).

The two AI text endpoints differ: `/api/explain` returns JSON (non-streaming); `/api/ask` streams `text/plain` via `anthropic.messages.stream(...)` piped into a `ReadableStream`.

### lib/ layout

- Top level: `anthropic.ts` (shared SDK client), `fetchers.ts` + `metrics.ts` + `prompts.ts` (the `/dollar` dataset, status model, and Claude system prompts), plus parallel `aiMetrics`/`aiPrompts`/`frontierModels`, `nuclearMetrics`/`nuclearPrompts`/`nuclearLive`, and `xSummary`.
- `lib/btc/` is a self-contained subsystem: `data/` (one file per upstream API), `calc/` (powerLaw, regimeScore, marketEmotion, netLiquidity, correlation, indicators), `types.ts`, `format.ts`. BTC API routes compose `data/` + `calc/`.

### Conventions & gotchas

- **AI prompts are snapshot-grounded.** `lib/prompts.ts` injects the live metric JSON into the system prompt and instructs Claude not to invent figures or imply real-time knowledge. Keep that contract when editing prompts.
- **Resilience is the norm.** External fetches use try/catch or `Promise.allSettled` and fall back to cached/hardcoded values. Preserve this — don't let one failed upstream break a dashboard.
- **US-runtime geoblocking is a known constraint.** Several exchange APIs (Binance, Bybit) return 451/403 from US-hosted servers, so spot data uses Kraken/Coinbase instead (see the comment header in `lib/btc/data/binanceKlines.ts`). File/function names were kept for caller compatibility even though the upstream changed.
- **Theming is CSS custom properties**, defined in `app/globals.css` (light + dark). Use `var(--...)` tokens rather than hardcoded colors; `.mono` = JetBrains Mono.
