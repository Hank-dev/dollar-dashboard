# Dollar & global financial system monitor

A single-page Next.js dashboard showing ~14 macro-market metrics describing the
health of the US dollar and the global financial system, with a Claude-powered
explanation layer:

- **Click any metric card** → slide-in drawer with a 2–4 sentence explanation
  grounded in the current snapshot value.
- **Ask the dashboard** → free-text Q&A, streamed token-by-token.

Data is a static snapshot dated **2026-05-22** (close of session). Live data
feeds are out of scope for v1.

## Prerequisites

- Node.js 20.9 or newer
- An Anthropic API key (`sk-ant-…`)

## Setup

```bash
npm install
cp .env.example .env.local
# edit .env.local and set ANTHROPIC_API_KEY=sk-ant-...
npm run dev
```

Open <http://localhost:3000>.

## Environment variables

| Var | Default | Purpose |
|---|---|---|
| `ANTHROPIC_API_KEY` | — (required) | Server-side only. Never prefixed with `NEXT_PUBLIC_`. |
| `EXPLAIN_MODEL` | `claude-sonnet-4-6` | Model used by `/api/explain`. |
| `ASK_MODEL` | `claude-sonnet-4-6` | Model used by `/api/ask`. Set to `claude-opus-4-7` for deeper analysis. |
| `ENABLE_WEB_SEARCH` | `false` | (Reserved) gate live web search in the ask endpoint. |

The dashboard itself renders fully even if the API key is missing or invalid —
only the Explain drawer and AskBox surface a friendly error.

## Project structure

```
app/
  layout.tsx           – root layout, fonts, metadata
  page.tsx             – mounts <Dashboard />
  globals.css          – Tailwind v4 + theme tokens (light + dark)
  api/
    explain/route.ts   – per-metric explanation (non-streaming)
    ask/route.ts       – free-form Q&A (text/plain stream)
components/
  Dashboard.tsx        – layout, header, verdict, legend, footer
  MetricGroup.tsx      – section heading + responsive grid
  MetricCard.tsx       – card surface + status dot + click-to-explain
  YieldCurveChart.tsx  – Chart.js line with theme-aware colors
  ExplainDrawer.tsx    – right-side drawer, focus trap, esc-to-close, cached
  AskBox.tsx           – textarea + streamed answer
lib/
  metrics.ts           – the dataset (source of truth for UI and AI context)
  prompts.ts           – shared snapshot + per-feature system prompts
  anthropic.ts         – shared SDK client
```

## Notes

- The API key is read inside route handlers via `process.env.ANTHROPIC_API_KEY`
  and never leaves the server. The client bundle contains no Anthropic
  credentials.
- Explanations are cached client-side per `metricId` so re-opening a drawer
  doesn't re-call the API.
- `max_tokens` is capped at 400 (explain) and 800 (ask) for cost control.
- Status is announced to screen readers via an `sr-only` text label — not just
  the dot color.
- The Japan 10Y JGB figure (~2.8%) rests on a single recent source; the footer
  surfaces this and the system prompt instructs Claude to flag it.

## Out of scope (v1)

- Live market data feeds (future: FRED, quotes provider).
- User accounts, saved questions, conversation history.
- Multi-turn chat — AskBox is single-question, single-answer.
