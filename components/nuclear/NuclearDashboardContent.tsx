import AskBox from "@/components/AskBox";
import { GroupIcon } from "@/components/Dashboard";
import NuclearLivePanel from "@/components/nuclear/NuclearLivePanel";
import {
  NUCLEAR_GROUPS,
  type NuclearDashboardData,
  type NuclearExplainable,
  type NuclearMarketMetric,
  type NuclearPlayer,
  type NuclearTechSignal,
} from "@/lib/nuclearMetrics";
import { STATUS_LABEL, type Status } from "@/lib/metrics";

const DATE_FMT = new Intl.DateTimeFormat("en-GB", {
  day: "numeric",
  month: "short",
  year: "numeric",
  timeZone: "UTC",
});

export default function NuclearDashboardContent({
  data,
  onOpenExplain,
}: {
  data: NuclearDashboardData;
  onOpenExplain: (item: NuclearExplainable) => void;
}) {
  const displayDate = DATE_FMT.format(
    new Date(data.snapshotDate + "T00:00:00Z"),
  );

  const publicPlayers = data.players.filter((p) => p.kind === "public");
  const projects = data.players.filter((p) => p.kind === "project");

  return (
    <main className="mx-auto max-w-[1080px] px-5 py-8 sm:py-10">
      <header className="flex flex-col gap-3 border-b border-[var(--border)] pb-5 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="mono mb-1 text-[11px] font-medium uppercase tracking-[0.16em] text-[var(--accent-cyan)]">
            Power / fuel / infrastructure briefing
          </p>
          <h1 className="mt-1 text-[21px] font-semibold tracking-tight text-[var(--text-primary)] sm:text-[24px]">
            Nuclear Energy Monitor
          </h1>
        </div>
        <p className="mono text-[11.5px] font-medium uppercase tracking-wide text-[var(--text-tertiary)]">
          Curated snapshot · {displayDate}
        </p>
      </header>

      <section
        aria-label="Verdict"
        className="mt-6 border border-[var(--border)] bg-[var(--bg-surface)] p-4 sm:p-5"
        style={{ borderLeft: "3px solid var(--dash-nuclear)" }}
      >
        <p className="mono text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--accent-cyan)]">
          Verdict - firm power is becoming strategic capacity
        </p>
        <p className="mt-2 max-w-[880px] text-[13.5px] leading-relaxed text-[var(--text-primary)]">
          {data.verdict}
        </p>
      </section>

      <StatusLegend />

      <section className="mt-7">
        <h2 className="mono mb-3 flex items-center gap-2 text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--text-secondary)]">
          <GroupIcon name="activity" />
          Live market signal
        </h2>
        <NuclearLivePanel />
      </section>

      <section className="mt-7">
        <h2 className="mono mb-3 flex items-center gap-2 text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--text-secondary)]">
          <GroupIcon name="activity" />
          Market state
        </h2>
        <div className="grid gap-2.5 md:grid-cols-2 xl:grid-cols-3">
          {data.marketMetrics.map((metric) => (
            <MetricTile
              key={metric.id}
              metric={metric}
              onSelect={() => onOpenExplain({ type: "metric", item: metric })}
            />
          ))}
        </div>
      </section>

      <section className="mt-9 grid gap-7 lg:grid-cols-[1fr_1fr]">
        <PlayerPanel
          title="Public-market nuclear leaders"
          players={publicPlayers}
          onSelect={(player) => onOpenExplain({ type: "player", item: player })}
        />
        <PlayerPanel
          title="Strategic customer projects"
          players={projects}
          onSelect={(player) => onOpenExplain({ type: "player", item: player })}
        />
      </section>

      <section className="mt-9">
        <h2 className="mono mb-3 flex items-center gap-2 text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--text-secondary)]">
          <GroupIcon name="trending-up" />
          Technology radar
        </h2>
        <div className="grid gap-2.5 md:grid-cols-2">
          {data.techSignals.map((signal) => (
            <SignalTile
              key={signal.id}
              signal={signal}
              onSelect={() => onOpenExplain({ type: "signal", item: signal })}
            />
          ))}
        </div>
      </section>

      <section className="mt-9">
        <h2 className="mono mb-3 flex items-center gap-2 text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--text-secondary)]">
          <GroupIcon name="message" />
          Ask the nuclear dashboard
        </h2>
        <AskBox
          endpoint="/api/nuclear/ask"
          placeholder="e.g. What is the biggest bottleneck in nuclear energy right now?"
          snapshotNote={`grounded in the ${displayDate} nuclear snapshot`}
        />
      </section>

      <footer className="mt-10 border-t border-[var(--border)] pt-5 text-[11.5px] leading-relaxed text-[var(--text-tertiary)]">
        <p>
          Sources are linked on each item. Public equities show market cap;
          strategic customer projects show announced or targeted project scale.
        </p>
        <p className="mt-2">
          This dashboard is informational market and technology context, not
          financial advice. Project capacities are not operating capacity unless
          explicitly stated.
        </p>
      </footer>
    </main>
  );
}

function StatusLegend() {
  const statuses: Status[] = ["calm", "neutral", "elevated", "stressed"];
  return (
    <div className="mono mt-6 flex flex-wrap items-center gap-x-5 gap-y-2 text-[11px] text-[var(--text-secondary)]">
      {statuses.map((status) => (
        <span key={status} className="inline-flex items-center gap-2">
          <span className={`dot ${status}`} />
          {STATUS_LABEL[status]}
        </span>
      ))}
    </div>
  );
}

export function StatusDot({ status }: { status: Status }) {
  return <span className={`dot ${status}`} />;
}

function MetricTile({
  metric,
  onSelect,
}: {
  metric: NuclearMarketMetric;
  onSelect: () => void;
}) {
  const group = NUCLEAR_GROUPS[metric.group];
  return (
    <button
      type="button"
      onClick={onSelect}
      className="group flex min-h-[142px] flex-col justify-between border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-left transition-colors hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
      style={{ borderRadius: 8 }}
      aria-label={`${metric.label}, ${metric.value}, status ${STATUS_LABEL[metric.status]}. Click to explain.`}
    >
      <div>
        <div className="flex items-center justify-between gap-2">
          <span className="inline-flex min-w-0 items-center gap-1.5 text-[12px] text-[var(--text-secondary)]">
            <StatusDot status={metric.status} />
            <span className="truncate">{metric.label}</span>
          </span>
          <span className="text-[10px] text-[var(--text-tertiary)] opacity-0 transition-opacity group-hover:opacity-100">
            i
          </span>
        </div>
        <p className="mt-2 text-[24px] font-medium tabular-nums text-[var(--text-primary)]">
          {metric.value}
        </p>
        <p className="mt-1 text-[11.5px] text-[var(--text-tertiary)]">
          {group.title} · {metric.context}
        </p>
      </div>
      <p className="mt-3 text-[12.5px] leading-relaxed text-[var(--text-secondary)]">
        {metric.detail}
      </p>
    </button>
  );
}

function PlayerPanel({
  title,
  players,
  onSelect,
}: {
  title: string;
  players: NuclearPlayer[];
  onSelect: (player: NuclearPlayer) => void;
}) {
  return (
    <section>
      <h2 className="mono mb-3 flex items-center gap-2 text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--text-secondary)]">
        <GroupIcon name="building-bank" />
        {title}
      </h2>
      <div
        className="overflow-hidden border border-[var(--border)] bg-[var(--bg-surface)]"
        style={{ borderRadius: 8 }}
      >
        {players.map((player, index) => (
          <button
            key={player.id}
            type="button"
            onClick={() => onSelect(player)}
            className="grid w-full gap-2 border-[var(--border)] p-3 text-left transition-colors hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)] sm:grid-cols-[minmax(120px,0.85fr)_minmax(110px,0.55fr)_1.3fr]"
            style={{ borderTopWidth: index === 0 ? 0 : 1 }}
          >
            <div className="min-w-0">
              <p className="flex items-center gap-1.5 text-[13px] font-medium text-[var(--text-primary)]">
                <StatusDot status={player.status} />
                <span className="truncate">{player.name}</span>
              </p>
              <p className="mt-0.5 text-[11.5px] text-[var(--text-tertiary)]">
                {player.ticker ? `${player.ticker} · ` : ""}
                {player.category}
              </p>
            </div>
            <div>
              <p className="text-[10.5px] font-semibold uppercase tracking-[0.12em] text-[var(--accent-cyan)]">
                {player.kind === "public" ? "Market cap" : "Project scale"}
              </p>
              <p className="mt-0.5 text-[15px] font-medium tabular-nums text-[var(--text-primary)]">
                {player.marketCap ?? player.projectScale}
              </p>
            </div>
            <p className="text-[12.5px] leading-relaxed text-[var(--text-secondary)]">
              {player.nuclearExposure}
            </p>
          </button>
        ))}
      </div>
    </section>
  );
}

function SignalTile({
  signal,
  onSelect,
}: {
  signal: NuclearTechSignal;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className="group min-h-[154px] border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-left transition-colors hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
      style={{ borderRadius: 8 }}
      aria-label={`${signal.track}: ${signal.label}. Click to explain.`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="text-[10.5px] font-semibold uppercase tracking-[0.12em] text-[var(--accent-cyan)]">
            {signal.track}
          </p>
          <h3 className="mt-1 flex items-center gap-1.5 text-[13px] font-medium text-[var(--text-primary)]">
            <StatusDot status={signal.status} />
            <span>{signal.label}</span>
          </h3>
        </div>
        <span className="text-[10px] text-[var(--text-tertiary)] opacity-0 transition-opacity group-hover:opacity-100">
          i
        </span>
      </div>
      <p className="mt-3 text-[12.5px] leading-relaxed text-[var(--text-secondary)]">
        {signal.summary}
      </p>
      <p className="mt-3 text-[11.5px] leading-relaxed text-[var(--text-tertiary)]">
        Watch: {signal.watchNext}
      </p>
    </button>
  );
}
