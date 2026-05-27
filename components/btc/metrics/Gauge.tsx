// Half-arc gauge for Fear & Greed (0 = Extreme Fear, 100 = Extreme Greed).
export function Gauge({
  value,
  label,
  classification,
}: {
  value: number | null;
  label: string;
  classification?: string;
}) {
  const v = value == null ? 0 : Math.max(0, Math.min(100, value));
  // 180-degree arc, from -90deg to +90deg.
  const angle = (v / 100) * 180 - 90;
  const color =
    v < 25
      ? "var(--accent-red)"
      : v < 45
        ? "var(--accent-amber)"
        : v < 55
          ? "#a3a3a3"
          : v < 75
            ? "#84cc16"
            : "var(--accent-green)";

  const cx = 100;
  const cy = 100;
  const r = 80;

  // Build arc segments with color zones.
  const stops: { from: number; to: number; c: string }[] = [
    { from: 0, to: 25, c: "var(--accent-red)" },
    { from: 25, to: 45, c: "var(--accent-amber)" },
    { from: 45, to: 55, c: "#a3a3a3" },
    { from: 55, to: 75, c: "#84cc16" },
    { from: 75, to: 100, c: "var(--accent-green)" },
  ];

  const polarPoint = (pct: number) => {
    const a = (pct / 100) * Math.PI - Math.PI; // 0% = -180°, 100% = 0°
    return [cx + r * Math.cos(a), cy + r * Math.sin(a)] as const;
  };

  const arcPath = (from: number, to: number) => {
    const [x1, y1] = polarPoint(from);
    const [x2, y2] = polarPoint(to);
    const large = to - from > 50 ? 1 : 0;
    return `M ${x1} ${y1} A ${r} ${r} 0 ${large} 1 ${x2} ${y2}`;
  };

  return (
    <div className="flex flex-col items-center p-3 gap-2">
      <div className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)] self-stretch">
        {label}
      </div>
      <svg viewBox="0 0 200 120" className="w-full max-w-[220px]">
        {stops.map((s) => (
          <path
            key={s.from}
            d={arcPath(s.from, s.to)}
            stroke={s.c}
            strokeWidth={10}
            fill="none"
            opacity={0.55}
          />
        ))}
        {/* Needle */}
        <g transform={`translate(${cx} ${cy}) rotate(${angle})`}>
          <line
            x1={0}
            y1={0}
            x2={0}
            y2={-r + 6}
            stroke={color}
            strokeWidth={2}
          />
          <circle cx={0} cy={0} r={4} fill={color} />
        </g>
      </svg>
      <div className="flex flex-col items-center -mt-2">
        <span className="mono text-3xl tnum">{value == null ? "—" : v.toFixed(0)}</span>
        {classification ? (
          <span className="mono text-[11px] uppercase tracking-wider text-[var(--text-secondary)]">
            {classification}
          </span>
        ) : null}
      </div>
    </div>
  );
}
