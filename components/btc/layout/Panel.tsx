import type { ReactNode } from "react";

export function Panel({
  title,
  right,
  children,
  className = "",
}: {
  title: string;
  right?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section
      className={`bg-[var(--bg-panel)] border border-[var(--border)] flex flex-col ${className}`}
    >
      <header className="flex items-center justify-between px-3 py-1.5 border-b border-[var(--border)]">
        <h2 className="mono text-[11px] tracking-wider uppercase text-[var(--text-secondary)]">
          {title}
        </h2>
        {right ? (
          <div className="mono text-[11px] text-[var(--text-tertiary)]">
            {right}
          </div>
        ) : null}
      </header>
      <div className="flex-1 min-h-0">{children}</div>
    </section>
  );
}
