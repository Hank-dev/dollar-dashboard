"use client";

import { TIMEFRAMES, type Timeframe } from "./timeframe";

export function TimeframeSelector({
  value,
  onChange,
}: {
  value: Timeframe;
  onChange: (tf: Timeframe) => void;
}) {
  return (
    <div className="inline-flex items-center gap-px bg-[var(--border)]">
      {TIMEFRAMES.map((tf) => {
        const active = tf === value;
        return (
          <button
            key={tf}
            type="button"
            onClick={() => onChange(tf)}
            aria-pressed={active}
            className={`mono text-[10px] tracking-wider uppercase px-1.5 py-0.5 transition-colors ${
              active
                ? "bg-[var(--bg-panel-hover)] text-[var(--text-primary)]"
                : "bg-[var(--bg-panel)] text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]"
            }`}
          >
            {tf}
          </button>
        );
      })}
    </div>
  );
}
