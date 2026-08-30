"use client";

import { useEffect, useState } from "react";

export function UtcClock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(id);
  }, []);
  const hh = String(now.getUTCHours()).padStart(2, "0");
  const mm = String(now.getUTCMinutes()).padStart(2, "0");
  const ss = String(now.getUTCSeconds()).padStart(2, "0");
  return (
    <div
      className="mono text-[36px] leading-none font-medium tracking-tight text-[var(--text-primary)] tabular-nums"
      suppressHydrationWarning
    >
      {hh}:{mm}:{ss}
    </div>
  );
}
