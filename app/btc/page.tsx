import { HeaderBar } from "@/components/btc/layout/HeaderBar";
import { PowerLawPanel } from "@/components/btc/panels/PowerLawPanel";
import { RegimePanel } from "@/components/btc/panels/RegimePanel";
import { NetLiquidityPanel } from "@/components/btc/panels/NetLiquidityPanel";
import { FngPanel } from "@/components/btc/panels/FngPanel";
import { FundingPanel } from "@/components/btc/panels/FundingPanel";
import { CoinbasePremiumPanel } from "@/components/btc/panels/CoinbasePremiumPanel";
import { CorrelationHeatmapPanel } from "@/components/btc/panels/CorrelationHeatmapPanel";
import { MacroStripPanel } from "@/components/btc/panels/MacroStripPanel";
import { MarketEmotionPanel } from "@/components/btc/panels/MarketEmotionPanel";
import { RegimeScorePanel } from "@/components/btc/panels/RegimeScorePanel";
import { EtfFlowsPanel } from "@/components/btc/panels/EtfFlowsPanel";
import { StrategyFlowsPanel } from "@/components/btc/panels/StrategyFlowsPanel";
import { InterpretationPanel } from "@/components/btc/panels/InterpretationPanel";

export default function BtcPage() {
  return (
    <div className="flex flex-col min-h-screen">
      <HeaderBar />

      <main className="mx-auto w-full max-w-[1400px] flex flex-col flex-1 gap-3 p-3 sm:p-4">
        {/* Row 1: Power Law + Regime */}
        <div className="grid gap-3 grid-cols-1 md:grid-cols-[3fr_2fr]">
          <div className="min-h-[380px] sm:min-h-[420px]">
            <PowerLawPanel />
          </div>
          <div className="min-h-[340px]">
            <RegimePanel />
          </div>
        </div>

        {/* Row 2: Net Liquidity + F&G + Funding + CB Premium */}
        <div className="grid gap-3 grid-cols-1 sm:grid-cols-2 xl:grid-cols-4">
          <div className="min-h-[300px]">
            <NetLiquidityPanel />
          </div>
          <div className="min-h-[300px]">
            <FngPanel />
          </div>
          <div className="min-h-[300px]">
            <FundingPanel />
          </div>
          <div className="min-h-[300px]">
            <CoinbasePremiumPanel />
          </div>
        </div>

        {/* Row 3: Correlations */}
        <div className="min-h-[320px]">
          <CorrelationHeatmapPanel />
        </div>

        {/* Row 4: Macro strip */}
        <MacroStripPanel />

        {/* Row 5: Market Emotion */}
        <div className="min-h-[480px]">
          <MarketEmotionPanel />
        </div>

        {/* Row 6: ETF Flows + Regime Score */}
        <div className="grid gap-3 grid-cols-1 md:grid-cols-[3fr_2fr]">
          <div className="min-h-[340px]">
            <EtfFlowsPanel />
          </div>
          <div className="min-h-[340px]">
            <RegimeScorePanel />
          </div>
        </div>

        {/* Row 7: Strategy Flows */}
        <div className="min-h-[340px]">
          <StrategyFlowsPanel />
        </div>

        {/* Row 8: AI Interpretation */}
        <div className="min-h-[180px]">
          <InterpretationPanel />
        </div>
      </main>

      <footer className="border-t border-[var(--border-strong)] bg-[var(--bg-panel)] px-3 py-1 flex flex-wrap items-center justify-between gap-2">
        <span className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
          BTC TERMINAL
        </span>
        <span className="mono text-[10px] text-[var(--text-tertiary)]">
          data: CoinGecko &middot; CoinMetrics &middot; FRED &middot; Yahoo &middot; alternative.me &middot; Binance/Bybit/OKX &middot; Coinbase &middot; Farside &middot; SEC EDGAR
        </span>
      </footer>
    </div>
  );
}
