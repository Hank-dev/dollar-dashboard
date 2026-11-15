//! Every upstream dataset, so the app can say how fresh each one is.
//! A refresh records the time of each successful response in `Snapshot::feeds`.

pub struct Feed {
    pub id: &'static str,
    pub provider: &'static str,
    pub covers: &'static str,
    /// Age after which the dataset counts as stale, in seconds.
    pub stale_after: i64,
}

const HOUR: i64 = 3600;
const DAY: i64 = 24 * HOUR;

pub const FEEDS: &[Feed] = &[
    feed("binance-spot", "Binance", "BTC, ETH and ZEC spot · 24h change", 2 * HOUR),
    feed("binance-daily", "Binance", "Daily candles: BTC, ETH, ZEC, ZEC/BTC", 2 * DAY),
    feed("binance-funding", "Binance", "BTC and ZEC perpetual funding, ZEC open interest", 2 * DAY),
    feed("tv-macro", "TradingView", "Indices, Treasury yields, FX, gold, silver · delayed", 2 * DAY),
    feed("tv-equities", "TradingView", "AI and nuclear equities, uranium ETFs · delayed", 2 * DAY),
    feed("tv-futures", "TradingView", "Brent and WTI front-month futures · delayed", 2 * DAY),
    feed("cnbc", "CNBC", "Daily history for indices, yields, FX, VIX, MOVE, commodities", 4 * DAY),
    feed("treasury", "U.S. Treasury", "Daily par yield curve, 1M to 30Y", 4 * DAY),
    feed("fred-fed", "FRED", "Fed funds target range", 5 * DAY),
    feed("fred-dollar", "FRED", "Broad trade-weighted dollar (DTWEXBGS)", 12 * DAY),
    feed("fred-liquidity", "FRED", "Fed balance sheet, TGA and reverse repo", 12 * DAY),
    feed("fred-macro", "FRED", "Real yield, HY OAS, VIX, S&P 500, Nasdaq 100, M2", 6 * DAY),
    feed("fred-uranium", "FRED", "Uranium spot, monthly (PURANUSDM)", 75 * DAY),
    feed("cboe", "CBOE", "VIX index", 4 * DAY),
    feed("gold-api", "Gold API", "Gold spot", 2 * DAY),
    feed("fx-api", "exchangerate-api", "USD/JPY daily reference rate", 3 * DAY),
    feed("fng", "Alternative.me", "Crypto Fear & Greed index", 3 * DAY),
    feed("coinmetrics", "Coin Metrics", "Bitcoin daily reference price since 2010", 4 * DAY),
    feed("coinmetrics-zec", "Coin Metrics", "Zcash daily reference price since launch (log-log chart)", 4 * DAY),
    feed("blockchain", "Blockchain.com", "Sampled Bitcoin history, used when Coin Metrics fails", 30 * DAY),
    feed("btc-data", "bitcoin-data.com", "MVRV Z-score, NUPL, realized price", 4 * DAY),
    feed("coinbase", "Coinbase", "BTC-USD hourly closes for the premium", 2 * DAY),
    feed("bybit", "Bybit", "BTC perpetual funding", 2 * DAY),
    feed("okx", "OKX", "BTC perpetual funding", 2 * DAY),
    feed("farside", "Farside", "US spot Bitcoin ETF flows", 5 * DAY),
    feed("kraken", "Kraken", "BTC daily volume for market emotion", 4 * DAY),
    feed("coingecko-paxg", "CoinGecko", "PAXG gold proxy for correlations", 4 * DAY),
    feed("yahoo", "Yahoo Finance", "STRC preferred price; MOVE fallback", 4 * DAY),
    feed("edgar", "SEC EDGAR", "Strategy 8-K at-the-market tables", 14 * DAY),
    feed("aa", "Artificial Analysis", "Frontier model leaderboard and token prices", 4 * DAY),
    feed("epoch", "Epoch AI", "Epoch Capabilities Index (ECI) for frontier models", 7 * DAY),
    feed("vals", "Vals.ai", "Vals Index overall accuracy for frontier models", 7 * DAY),
    feed("openrouter", "OpenRouter", "Token prices when the leaderboard is unreadable", 30 * DAY),
    feed("zcashd", "zcashexplorer.app", "Zcash height, supply, value pools, upgrades", 2 * DAY),
    feed("blockchair", "Blockchair", "Zcash transactions, hashrate, nodes, fees", 3 * DAY),
    feed("coingecko-zec", "CoinGecko", "Zcash market cap, rank, volume, ATH", 3 * DAY),
    feed("yahoo-cyph", "Yahoo Finance", "Cypherpunk (CYPH) daily close for equity ÷ ZEC", 5 * DAY),
];

const fn feed(id: &'static str, provider: &'static str, covers: &'static str, stale_after: i64) -> Feed {
    Feed {
        id,
        provider,
        covers,
        stale_after,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
    Never,
}

pub fn freshness(feed: &Feed, last: Option<i64>, now: i64) -> Freshness {
    match last {
        None => Freshness::Never,
        Some(t) if now - t > feed.stale_after => Freshness::Stale,
        Some(_) => Freshness::Fresh,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_ids_are_unique_and_staleness_uses_each_cadence() {
        let mut ids: Vec<_> = FEEDS.iter().map(|f| f.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), FEEDS.len());
        let spot = &FEEDS[0];
        assert_eq!(freshness(spot, None, 10), Freshness::Never);
        assert_eq!(freshness(spot, Some(0), spot.stale_after), Freshness::Fresh);
        assert_eq!(freshness(spot, Some(0), spot.stale_after + 1), Freshness::Stale);
    }
}
