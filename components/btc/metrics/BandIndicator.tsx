type Zone = {
  /** lower bound (inclusive) */
  from: number;
  /** color (CSS string) */
  color: string;
  /** optional label shown above this zone */
  label?: string;
};

/**
 * Horizontal color-banded indicator with a triangle marker for the current value.
 * `zones` must be sorted by `from` ascending. `min` and `max` bound the scale.
 */
export function BandIndicator({
  label,
  value,
  min,
  max,
  zones,
  formatValue,
}: {
  label: string;
  value: number | null;
  min: number;
  max: number;
  zones: Zone[];
  formatValue: (v: number) => string;
}) {
  const span = max - min;
  const pct = (v: number) =>
    Math.max(0, Math.min(100, ((v - min) / span) * 100));

  const segs = zones.map((z, i) => {
    const next = zones[i + 1]?.from ?? max;
    const left = pct(z.from);
    const width = Math.max(0, pct(next) - left);
    return { ...z, left, width };
  });

  const markerLeft = value != null ? pct(value) : null;

  return (
    <div className="px-3 py-2 flex flex-col gap-1 border-b border-[var(--border)] last:border-b-0">
      <div className="flex items-baseline justify-between">
        <span className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
          {label}
        </span>
        <span className="mono text-sm tnum">
          {value == null ? "—" : formatValue(value)}
        </span>
      </div>
      <div className="relative h-2 w-full bg-[var(--bg-base)] border border-[var(--border)]">
        {segs.map((s, i) => (
          <div
            key={i}
            className="absolute top-0 bottom-0"
            style={{
              left: `${s.left}%`,
              width: `${s.width}%`,
              background: s.color,
              opacity: 0.7,
            }}
            title={s.label}
          />
        ))}
        {markerLeft != null ? (
          <div
            className="absolute top-[-2px] bottom-[-2px] w-[2px] bg-[var(--text-primary)]"
            style={{ left: `calc(${markerLeft}% - 1px)` }}
          />
        ) : null}
      </div>
    </div>
  );
}
