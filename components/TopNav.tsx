"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const TABS = [
  { href: "/", label: "HOME", idx: "00", cls: "" },
  { href: "/btc", label: "BTC", idx: "01", cls: "btc" },
  { href: "/dollar", label: "DOLLAR", idx: "02", cls: "dollar" },
  { href: "/ai", label: "AI", idx: "03", cls: "ai" },
] as const;

export function TopNav() {
  const pathname = usePathname();

  return (
    <nav
      className="h-12 border-b border-[var(--border)] bg-[var(--bg-surface)] flex items-stretch sticky top-0 z-10"
      style={{ fontSize: 13 }}
    >
      {/* Brand */}
      <Link
        href="/"
        className="flex items-center gap-2.5 px-4 border-r border-[var(--border)] min-w-[200px] no-underline"
      >
        <div className="w-[18px] h-[18px] border-[1.5px] border-[var(--text-primary)] relative">
          <div className="absolute inset-[3px] bg-[var(--text-primary)]" />
        </div>
        <span className="mono text-[11.5px] tracking-[0.18em] uppercase text-[var(--text-primary)]">
          Market Monitor{" "}
          <span className="text-[var(--text-tertiary)] ml-1">v2</span>
        </span>
      </Link>

      {/* Tabs */}
      <div className="flex items-stretch">
        {TABS.map((tab) => {
          const active = tab.href === "/" ? pathname === "/" : pathname.startsWith(tab.href);
          return (
            <Link
              key={tab.href}
              href={tab.href}
              className={`flex items-center gap-2.5 px-[18px] border-r border-[var(--border)] mono text-[11.5px] tracking-[0.12em] uppercase no-underline relative transition-colors ${
                active
                  ? "text-[var(--text-primary)] bg-[var(--bg-base)]"
                  : "text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)]"
              }`}
            >
              <span className="mono text-[10px] text-[var(--text-tertiary)]">
                {tab.idx}
              </span>
              {tab.label}
              {active && (
                <span
                  className="absolute left-0 right-0 -bottom-px h-0.5"
                  style={{
                    background:
                      tab.cls === "btc"
                        ? "var(--dash-btc)"
                        : tab.cls === "dollar"
                        ? "var(--dash-dollar)"
                        : tab.cls === "ai"
                        ? "var(--dash-ai)"
                        : "var(--text-primary)",
                  }}
                />
              )}
            </Link>
          );
        })}
      </div>

      {/* Right */}
      <div className="ml-auto flex items-center gap-4 px-4 border-l border-[var(--border)] mono text-[11px] text-[var(--text-secondary)]">
        <span className="flex items-center gap-1.5">
          <span className="dot live" />
          <span>WS</span>
        </span>
      </div>
    </nav>
  );
}
