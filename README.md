# Dollar & global financial system monitor

A single-page Next.js dashboard showing ~14 macro-market metrics describing the
health of the US dollar and the global financial system, with a Claude-powered
explanation layer:

- **Click any metric card** → slide-in drawer with a 2–4 sentence explanation
  grounded in the current snapshot value.
- **Ask the dashboard** → free-text Q&A, streamed token-by-token.

Data is a static snapshot dated **2026-05-22** (close of session). Live data
feeds are out of scope for v1.

> **Status note:** the deployed v2 has grown beyond this README — sections for BTC, Dollar & Macro, AI models, and Nuclear Energy, plus a daily generated Claude briefing. Screenshot of the live app: [docs/screenshot-dashboard.png](docs/screenshot-dashboard.png)

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

## VPS Deployment

The app can run on a VPS with Docker Compose. It builds Next.js in standalone
mode, runs the production server on container port `3000`, and maps it to host
port `3001` by default so it can run beside other dashboards.

On the VPS:

```bash
git clone <your-repo-url>
cd <repo-directory>
cp .env.example .env
```

Edit `.env` and set `ANTHROPIC_API_KEY`. You can keep `APP_PORT=3001` unless
that port is already in use.

Start the dashboard:

```bash
docker compose up -d --build
docker compose ps
curl http://127.0.0.1:3001/api/health
```

If you are not using a domain yet, open `http://<your-vps-ip>:3001`. If you are
using Nginx, proxy your domain to the local container port:

```nginx
server {
  listen 80;
  server_name your-domain.com;

  location / {
    proxy_pass http://127.0.0.1:3001;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
  }
}
```

To deploy updates:

```bash
git pull
docker compose up -d --build
```

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

## Snapshot refresh

The `/ai` dashboard's slow-moving data lives in `lib/aiSnapshot.json` — each value carries an `asOf` date, a `source`, and a `confidence`. `lib/aiMetrics.ts` holds only the fixed roster (which players/metrics/signals exist) plus the builder. Public-company market caps are overlaid with live quotes at request time; everything else is read from the JSON.

`scripts/refresh-snapshots.ts` uses Claude + web search to re-verify and rewrite that JSON:

```bash
# preview changes without writing
ENABLE_WEB_SEARCH=true ANTHROPIC_API_KEY=sk-... npm run refresh:snapshots -- --dry-run

# write the updated file
ENABLE_WEB_SEARCH=true ANTHROPIC_API_KEY=sk-... npm run refresh:snapshots
```

A weekly GitHub Action (`.github/workflows/refresh-snapshots.yml`) runs the script and opens a PR with the diff for review. It requires an `ANTHROPIC_API_KEY` repository secret (Settings → Secrets and variables → Actions).

It also requires **Settings → Actions → General → "Allow GitHub Actions to create and approve pull requests"** to be enabled — without that (or without the secret) the scheduled run fails and opens no PR. If the snapshot is unchanged, no PR is opened. Set `REFRESH_MODEL` to override the default `claude-sonnet-4-6` model used by the refresh script.

## Out of scope (v1)

- Live market data feeds (future: FRED, quotes provider).
- User accounts, saved questions, conversation history.
- Multi-turn chat — AskBox is single-question, single-answer.
