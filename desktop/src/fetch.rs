use crate::model::{now_unix, BtcBook, Obs, Snapshot};
use crate::{markets, parse};
use chrono::Datelike;
use reqwest::blocking::Client;
use std::thread;
use std::time::Duration;

const HISTORY_TTL: i64 = 6 * 60 * 60;
const AMERICA: &[&str] = &[
    "NASDAQ:NVDA",
    "NASDAQ:GOOGL",
    "NASDAQ:MSFT",
    "NASDAQ:AMZN",
    "NASDAQ:AVGO",
    "NASDAQ:META",
    "NASDAQ:SPCX",
    "NASDAQ:CEG",
    "NYSE:VST",
    "NYSE:CCJ",
    "NYSE:BWXT",
    "NYSE:OKLO",
    "AMEX:UEC",
    "NYSE:SMR",
    "NYSE:LEU",
    "AMEX:URA",
    "AMEX:URNM",
    "AMEX:NLR",
];
pub const SPOT_TICKERS: &str =
    "https://api.binance.com/api/v3/ticker/24hr?symbols=%5B%22BTCUSDT%22,%22ETHUSDT%22,%22ZECUSDT%22%5D";

pub fn refresh(prev: &Snapshot) -> Snapshot {
    let client = match client() {
        Ok(client) => client,
        Err(err) => {
            let mut next = prev.clone();
            next.notes = vec![err];
            return next;
        }
    };
    let now = now_unix();
    let year = chrono::Utc::now().format("%Y");
    let broad_start = (chrono::Utc::now() - chrono::Duration::days(200))
        .format("%Y-%m-%d")
        .to_string();
    let fed_start = (chrono::Utc::now() - chrono::Duration::days(400))
        .format("%Y-%m-%d")
        .to_string();
    // The Treasury file holds one calendar year. Early in a year the 1m/3m
    // curve comparisons need last year's closes too.
    let curve_cutoff = (chrono::Utc::now() - chrono::Duration::days(100))
        .format("%Y-%m-%d")
        .to_string();
    let need_prev_curve = prev
        .dollar
        .curve_history
        .first()
        .is_none_or(|day| day.date > curve_cutoff);
    let prev_year = chrono::Utc::now().year() - 1;
    let need_history = prev.btc.daily.len() < 100 || now - prev.history_unix > HISTORY_TTL;
    let need_weekly = prev.btc.weekly.len() < 200 || now - prev.weekly_unix > HISTORY_TTL;
    let need_markets = markets::history_due(&prev.histories, prev.histories_unix, now);

    let (
        treasury,
        treasury_prev,
        fred_broad,
        fred_upper,
        fred_lower,
        vix,
        gold,
        jpy,
        america,
        global,
        futures,
        tickers,
        funding,
        fng,
        spark,
        eth_daily,
        history,
        weekly,
        zec,
        histories,
    ) = thread::scope(|scope| {
        let treasury = scope.spawn(|| {
            get_text(
                &client,
                &format!(
                    "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/pages/xml?data=daily_treasury_yield_curve&field_tdr_date_value={year}"
                ),
            )
        });
        let treasury_prev = scope.spawn(|| {
            if !need_prev_curve {
                return Ok(String::new());
            }
            get_text(
                &client,
                &format!(
                    "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/pages/xml?data=daily_treasury_yield_curve&field_tdr_date_value={prev_year}"
                ),
            )
        });
        let fred_broad = scope.spawn(|| fred(&client, "DTWEXBGS", &broad_start));
        let fred_upper = scope.spawn(|| fred(&client, "DFEDTARU", &fed_start));
        let fred_lower = scope.spawn(|| fred(&client, "DFEDTARL", &fed_start));
        let vix = scope.spawn(|| {
            get_text(
                &client,
                "https://cdn.cboe.com/api/global/delayed_quotes/quotes/_VIX.json",
            )
        });
        let gold = scope.spawn(|| get_text(&client, "https://api.gold-api.com/price/XAU"));
        let jpy = scope.spawn(|| get_text(&client, "https://open.er-api.com/v6/latest/USD"));
        let america =
            scope.spawn(|| scan(&client, "america", AMERICA).map(|body| (body, now_unix())));
        let global =
            scope.spawn(|| scan(&client, "global", markets::QUOTES).map(|body| (body, now_unix())));
        let futures = scope
            .spawn(|| scan(&client, "futures", markets::FUTURES).map(|body| (body, now_unix())));
        let tickers =
            scope.spawn(|| get_text(&client, SPOT_TICKERS).map(|body| (body, now_unix())));
        let funding = scope.spawn(|| {
            get_text(
                &client,
                "https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT&limit=21",
            )
        });
        let fng = scope.spawn(|| get_text(&client, "https://api.alternative.me/fng/?limit=90"));
        let spark = scope.spawn(|| {
            get_text(
                &client,
                "https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1d&limit=120",
            )
        });
        let eth_daily = scope.spawn(|| {
            get_text(
                &client,
                "https://api.binance.com/api/v3/klines?symbol=ETHUSDT&interval=1d&limit=400",
            )
        });
        let history = scope.spawn(|| {
            if !need_history {
                return Ok(String::new());
            }
            get_text(
                &client,
                "https://api.blockchain.info/charts/market-price?timespan=all&format=json&sampled=true",
            )
        });
        let weekly = scope.spawn(|| {
            if !need_weekly {
                return Ok(String::new());
            }
            get_text(
                &client,
                "https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1w&limit=1000",
            )
        });
        let zec = scope.spawn(|| crate::zcash::refresh(&client, &prev.zec));
        let histories = scope
            .spawn(|| need_markets.then(|| markets::refresh_histories(&client, &prev.histories)));
        (
            treasury.join(),
            treasury_prev.join(),
            fred_broad.join(),
            fred_upper.join(),
            fred_lower.join(),
            vix.join(),
            gold.join(),
            jpy.join(),
            america.join(),
            global.join(),
            futures.join(),
            tickers.join(),
            funding.join(),
            fng.join(),
            spark.join(),
            eth_daily.join(),
            history.join(),
            weekly.join(),
            zec.join(),
            histories.join(),
        )
    });

    let mut next = prev.clone();
    next.notes.clear();
    let mut ok = 0u32;
    let mut mark = |next: &mut Snapshot, id: &str| {
        next.feeds.insert(id.to_string(), now);
        ok += 1;
    };

    let older_curve = match flatten(treasury_prev) {
        Ok(body) => parse::treasury_history(&body),
        Err(err) => {
            next.notes.push(format!("Treasury (previous year): {err}"));
            Vec::new()
        }
    };
    match flatten(treasury) {
        Ok(body) => {
            let days = parse::merge_curve_days(&[
                &prev.dollar.curve_history,
                &older_curve,
                &parse::treasury_history(&body),
            ]);
            match days.last().cloned().map(|day| (day.date, day.points)) {
            Some((date, curve)) => {
                let source = "U.S. Treasury";
                next.dollar.curve = curve.clone();
                next.dollar.curve_history = days;
                next.dollar.curve_date = date.clone();
                for point in &curve {
                    let item = Obs {
                        date: date.clone(),
                        value: point.yield_pct,
                        source: source.into(),
                    };
                    match point.label.as_str() {
                        "2Y" => next.dollar.ust2 = Some(item),
                        "10Y" => next.dollar.ust10 = Some(item),
                        "30Y" => next.dollar.ust30 = Some(item),
                        _ => {}
                    }
                }
                mark(&mut next, "treasury");
            }
            None => next
                .notes
                .push("Treasury yield curve had no usable day.".into()),
            }
        }
        Err(err) => {
            if !older_curve.is_empty() {
                next.dollar.curve_history =
                    parse::merge_curve_days(&[&prev.dollar.curve_history, &older_curve]);
            }
            next.notes.push(format!("Treasury: {err}"));
        }
    }

    match flatten(fred_broad) {
        Ok(body) => {
            let series = parse::fred_series(&body);
            if let Some(latest) = series.last() {
                let mut latest = latest.clone();
                latest.source = "FRED DTWEXBGS".into();
                next.dollar.dxy_broad = Some(latest);
                next.dollar.dxy_spark = series.iter().map(|obs| obs.value).collect();
                mark(&mut next, "fred-dollar");
            }
        }
        Err(err) => next.notes.push(format!("FRED broad dollar: {err}")),
    }
    if apply_fed(&mut next, flatten(fred_upper), flatten(fred_lower)) {
        mark(&mut next, "fred-fed");
    }
    match flatten(vix) {
        Ok(body) => match parse::cboe_vix(&body) {
            Some(obs) => {
                next.dollar.vix = Some(obs);
                mark(&mut next, "cboe");
            }
            None => next
                .notes
                .push("CBOE VIX response was not readable.".into()),
        },
        Err(err) => next.notes.push(format!("CBOE VIX: {err}")),
    }
    match flatten(gold) {
        Ok(body) => match parse::gold_spot(&body) {
            Some(obs) => {
                next.dollar.gold = Some(obs);
                mark(&mut next, "gold-api");
            }
            None => next
                .notes
                .push("Gold spot response was not readable.".into()),
        },
        Err(err) => next.notes.push(format!("Gold: {err}")),
    }
    match flatten(jpy) {
        Ok(body) => match parse::usd_jpy(&body) {
            Some(obs) => {
                next.dollar.usdjpy = Some(obs);
                mark(&mut next, "fx-api");
            }
            None => next.notes.push("USD/JPY response was not readable.".into()),
        },
        Err(err) => next.notes.push(format!("USD/JPY: {err}")),
    }

    let mut quotes = Vec::new();
    for (label, id, result) in [
        ("TradingView equities", "tv-equities", america),
        ("TradingView macro", "tv-macro", global),
        ("TradingView futures", "tv-futures", futures),
    ] {
        match flatten(result) {
            Ok((body, acquired_unix)) => {
                let parsed = parse::tradingview_quotes(&body);
                if parsed.is_empty() {
                    next.notes.push(format!("{label} returned no quotes."));
                } else {
                    for q in &parsed {
                        next.price_stamps.insert(
                            q.symbol.clone(),
                            crate::model::PriceStamp {
                                acquired_unix,
                                observed_unix: None,
                                source: label.into(),
                            },
                        );
                    }
                    quotes.extend(parsed);
                    mark(&mut next, id);
                }
            }
            Err(err) => next.notes.push(format!("{label}: {err}")),
        }
    }
    if !quotes.is_empty() {
        if let Some(dxy) = quote_receipt_obs(&next, &quotes, "DXY", "TradingView ICE DXY") {
            next.dollar.dxy = Some(dxy);
        }
        if let Some(jgb) = quote_receipt_obs(&next, &quotes, "JP10Y", "TradingView") {
            next.dollar.jgb10 = Some(jgb);
        }
        if let Some(spx) = quote_receipt_obs(&next, &quotes, "SPX", "TradingView") {
            next.dollar.spx = Some(spx);
        }
        if let Some(brent) = quote_receipt_obs(&next, &quotes, "BRENT", "TradingView NYMEX Brent") {
            next.dollar.brent = Some(brent);
        }
        // The FX feed is a live session quote; the reference-rate API is the fallback.
        if let Some(jpy) = quote_receipt_obs(&next, &quotes, "USDJPY", "TradingView FX") {
            next.dollar.usdjpy = Some(jpy);
        }
        if let Some(boj) = quote_receipt_obs(&next, &quotes, "JPINTR", "TradingView economics") {
            next.dollar.boj = Some(boj);
        }
        if next.dollar.gold.is_none() {
            next.dollar.gold = quote_receipt_obs(&next, &quotes, "GOLD", "TradingView");
        }
        // Keep previous quotes for any market whose scan failed this round.
        let fresh: Vec<String> = quotes.iter().map(|q| q.symbol.clone()).collect();
        let mut merged: Vec<_> = prev
            .quotes
            .iter()
            .filter(|q| !fresh.contains(&q.symbol))
            .cloned()
            .collect();
        merged.extend(quotes);
        next.quotes = merged
            .into_iter()
            .filter(|q| {
                q.symbol
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            })
            .collect();
    }

    match flatten(tickers) {
        Ok((body, acquired_unix)) => {
            let batch = parse::binance_batch(&body, acquired_unix);
            if !apply_ticker_batch(&mut next, &batch).is_empty() {
                mark(&mut next, "binance-spot");
            }
        }
        Err(err) => next.notes.push(format!("Binance ticker: {err}")),
    }
    match flatten(funding) {
        Ok(body) => {
            let rates = parse::funding_rates(&body);
            if let Some(latest) = rates.last().copied() {
                next.btc.funding = Some(latest);
                next.btc.funding_hist = rates;
                mark(&mut next, "binance-funding");
            }
        }
        Err(err) => next.notes.push(format!("Binance funding: {err}")),
    }
    match flatten(fng) {
        Ok(body) => match parse::fear_greed(&body) {
            Some((value, label, hist)) => {
                next.btc.fng = Some(value);
                next.btc.fng_label = label;
                next.btc.fng_hist = hist;
                mark(&mut next, "fng");
            }
            None => next
                .notes
                .push("Fear and Greed response was not readable.".into()),
        },
        Err(err) => next.notes.push(format!("Fear and Greed: {err}")),
    }
    match flatten(spark) {
        Ok(body) => {
            let points = parse::klines(&body);
            if points.len() >= 2 {
                next.btc.spark = points.iter().map(|p| p.v).collect();
                mark(&mut next, "binance-daily");
            }
        }
        Err(err) => next.notes.push(format!("Binance daily history: {err}")),
    }
    match flatten(eth_daily) {
        Ok(body) => {
            let points = parse::klines(&body);
            if points.len() >= 30 {
                next.btc.eth_daily = points;
            }
        }
        Err(err) => next.notes.push(format!("Binance ETH history: {err}")),
    }
    if need_history {
        match flatten(history) {
            Ok(body) if !body.is_empty() => {
                let points = parse::blockchain_prices(&body);
                if points.len() >= 100 {
                    next.btc.daily = points;
                    next.history_unix = now;
                    mark(&mut next, "blockchain");
                } else {
                    next.notes
                        .push("Bitcoin history was too short for the power-law fit.".into());
                }
            }
            Ok(_) => {}
            Err(err) => next.notes.push(format!("Bitcoin history: {err}")),
        }
    }
    if need_weekly {
        match flatten(weekly) {
            Ok(body) if !body.is_empty() => {
                let points = parse::klines(&body);
                if points.len() >= 200 {
                    next.btc.weekly = points;
                    next.weekly_unix = now;
                } else {
                    next.notes
                        .push("Weekly Bitcoin history was shorter than 200 weeks.".into());
                }
            }
            Ok(_) => {}
            Err(err) => next.notes.push(format!("Binance weekly history: {err}")),
        }
    }

    match zec {
        Ok((book, notes, feeds)) => {
            next.zec = book;
            next.notes.extend(notes);
            for id in feeds {
                mark(&mut next, id);
            }
        }
        Err(_) => next.notes.push("Zcash: request thread panicked".into()),
    }

    match histories {
        Ok(Some((series, count, failed))) => {
            next.histories = series;
            if count > 0 {
                next.histories_unix = now;
                mark(&mut next, "cnbc");
            }
            if !failed.is_empty() {
                next.notes.push(format!(
                    "CNBC history unavailable for {}; previous closes kept.",
                    failed.join(", ")
                ));
            }
        }
        Ok(None) => {}
        Err(_) => next
            .notes
            .push("CNBC history: request thread panicked".into()),
    }

    let enriched = crate::readouts::enrich(&client, &next);
    for id in &enriched.feeds {
        mark(&mut next, id);
    }
    next.notes.extend(enriched.notes);
    // Coin Metrics publishes one reference price per day since 2010; prefer it to
    // the sampled Blockchain.com history for charts and the power-law fit.
    if enriched.btc_daily.len() >= 1000 {
        next.btc.daily = enriched.btc_daily;
        next.history_unix = now;
    }
    next.terminal = crate::readouts::merge(&prev.terminal, enriched.terminal);

    // Only a successful response this round counts as a refresh; merged values
    // from earlier rounds must not make an offline refresh look current.
    if ok > 0 {
        next.fetched_unix = now;
    } else if next.notes.is_empty() {
        next.notes.push("No source returned fresh data.".into());
    }
    next.notes.dedup();
    next.notes.truncate(16);
    next
}

/// Scanner dates describe receipt, because TradingView does not supply a
/// last-trade time in this response. Keep that distinction in the metric source.
fn quote_receipt_obs(
    next: &Snapshot,
    quotes: &[crate::model::Quote],
    symbol: &str,
    source: &str,
) -> Option<Obs> {
    let received = next.price_acquired_unix(symbol)?;
    let date = chrono::DateTime::from_timestamp(received, 0)?
        .format("%Y-%m-%d")
        .to_string();
    parse::quote_obs(
        quotes,
        symbol,
        &format!("{source} · receipt date; trade time unavailable"),
        &date,
    )
}

/// Returns true when the target range was read.
fn apply_fed(
    next: &mut Snapshot,
    upper: Result<String, String>,
    lower: Result<String, String>,
) -> bool {
    let mut ok = false;
    match upper {
        Ok(body) => {
            let series = parse::fred_series(&body);
            if let Some(latest) = series.last().cloned() {
                let mut latest = latest;
                latest.source = "FRED DFEDTARU".into();
                next.dollar.fed_hi_6m = parse::months_before(&series, &latest, 182).cloned();
                next.dollar.fed_hi = Some(latest);
                ok = true;
            }
        }
        Err(err) => next.notes.push(format!("FRED fed upper: {err}")),
    }
    match lower {
        Ok(body) => {
            if let Some(mut latest) = parse::fred_latest(&body) {
                latest.source = "FRED DFEDTARL".into();
                next.dollar.fed_lo = Some(latest);
                ok = true;
            }
        }
        Err(err) => next.notes.push(format!("FRED fed lower: {err}")),
    }
    ok
}

/// Raw value projection for callers that do not claim freshness. Response paths
/// must use `apply_ticker_batch` so values and per-symbol provenance stay paired.
pub fn apply_tickers(btc: &mut BtcBook, rows: &[crate::parse::TickerPrint]) {
    for (symbol, price, change, high, low) in rows {
        match symbol.as_str() {
            "BTCUSDT" => {
                btc.price = Some(*price);
                btc.price_source = "Binance BTCUSDT".into();
                btc.change_24h_pct = Some(*change);
                btc.high_24h = Some(*high);
                btc.low_24h = Some(*low);
            }
            "ETHUSDT" => {
                btc.eth_price = Some(*price);
                btc.eth_change_24h_pct = Some(*change);
            }
            "ZECUSDT" => {
                btc.zec_price = Some(*price);
                btc.zec_change_24h_pct = Some(*change);
            }
            _ => {}
        }
    }
}

fn later_than(a: &crate::model::PriceStamp, b: &crate::model::PriceStamp) -> bool {
    match (a.observed_unix, b.observed_unix) {
        (Some(a), Some(b)) if a != b => a > b,
        (None, Some(_)) => false,
        (Some(_), None) => true,
        _ => a.acquired_unix > b.acquired_unix,
    }
}

/// Apply only the symbols actually returned, with the times captured by the
/// response worker. An old batch cannot overwrite a newer observation.
pub fn apply_ticker_batch(
    snapshot: &mut Snapshot,
    batch: &parse::TickerBatch,
) -> Vec<parse::TickerPrint> {
    let mut applied = Vec::new();
    for row in &batch.rows {
        let symbol = match row.0.as_str() {
            "BTCUSDT" => "BTC",
            "ETHUSDT" => "ETH",
            "ZECUSDT" => "ZEC",
            _ => continue,
        };
        if !row.1.is_finite() || row.1 <= 0.0 {
            continue;
        }
        let stamp = crate::model::PriceStamp {
            acquired_unix: batch.acquired_unix,
            observed_unix: batch.observed_unix.get(symbol).copied(),
            source: format!("Binance {}", row.0),
        };
        if snapshot
            .price_stamp(symbol)
            .is_some_and(|old| !later_than(&stamp, old))
        {
            continue;
        }
        apply_tickers(&mut snapshot.btc, std::slice::from_ref(row));
        snapshot.price_stamps.insert(symbol.into(), stamp);
        applied.push(row.clone());
    }
    applied
}

/// The UI can receive a completed background refresh after newer spot batches.
/// Carry each newer price, its 24h fields and its stamp across as one reading.
pub fn preserve_newer_spot(next: &mut Snapshot, current: &Snapshot) {
    for symbol in ["BTC", "ETH", "ZEC"] {
        let Some(stamp) = current.price_stamp(symbol) else {
            continue;
        };
        if next
            .price_stamp(symbol)
            .is_some_and(|incoming| !later_than(stamp, incoming))
        {
            continue;
        }
        match symbol {
            "BTC" => {
                next.btc.price = current.btc.price;
                next.btc.price_source = current.btc.price_source.clone();
                next.btc.change_24h_pct = current.btc.change_24h_pct;
                next.btc.high_24h = current.btc.high_24h;
                next.btc.low_24h = current.btc.low_24h;
            }
            "ETH" => {
                next.btc.eth_price = current.btc.eth_price;
                next.btc.eth_change_24h_pct = current.btc.eth_change_24h_pct;
            }
            _ => {
                next.btc.zec_price = current.btc.zec_price;
                next.btc.zec_change_24h_pct = current.btc.zec_change_24h_pct;
            }
        }
        next.price_stamps.insert(symbol.into(), stamp.clone());
        if let Some(t) = current.feeds.get("binance-spot") {
            let old = next.feeds.entry("binance-spot".into()).or_insert(*t);
            *old = (*old).max(*t);
        }
    }
}

pub fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(25))
        .user_agent("Mozilla/5.0 (compatible; dollar-dashboard/0.1; +https://github.com/Hank-dev/dollar-dashboard)")
        .build()
        .map_err(|err| err.to_string())
}

pub(crate) fn get_text(client: &Client, url: &str) -> Result<String, String> {
    let response = client.get(url).send().map_err(|err| err.to_string())?;
    let status = response.status();
    let body = response.text().map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    Ok(body)
}

fn fred(client: &Client, series: &str, start: &str) -> Result<String, String> {
    get_text(
        client,
        &format!("https://fred.stlouisfed.org/graph/fredgraph.csv?id={series}&cosd={start}"),
    )
}

fn scan(client: &Client, market: &str, tickers: &[&str]) -> Result<String, String> {
    let response = client
        .post(format!("https://scanner.tradingview.com/{market}/scan"))
        .json(&serde_json::json!({
            "symbols": { "tickers": tickers },
            "columns": ["close", "change", "market_cap_basic", "name", "change_abs"]
        }))
        .send()
        .map_err(|err| err.to_string())?;
    let status = response.status();
    let body = response.text().map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    Ok(body)
}

pub(crate) fn flatten<T>(result: thread::Result<Result<T, String>>) -> Result<T, String> {
    match result {
        Ok(inner) => inner,
        Err(_) => Err("request thread panicked".into()),
    }
}

pub fn summary(snapshot: &Snapshot) -> String {
    let btc = snapshot
        .btc
        .price
        .map(|v| format!("BTC {}", crate::model::fmt_usd(v)))
        .unwrap_or_else(|| "BTC —".into());
    let dxy = snapshot
        .dollar
        .dxy
        .as_ref()
        .map(|v| format!("DXY {:.2}", v.value))
        .unwrap_or_else(|| "DXY —".into());
    let y10 = snapshot
        .dollar
        .ust10
        .as_ref()
        .map(|v| format!("US10Y {:.2}%", v.value))
        .unwrap_or_else(|| "US10Y —".into());
    let vix = snapshot
        .dollar
        .vix
        .as_ref()
        .map(|v| format!("VIX {:.1}", v.value))
        .unwrap_or_else(|| "VIX —".into());
    let notes = if snapshot.notes.is_empty() {
        "sources ok".to_string()
    } else {
        snapshot.notes.join("; ")
    };
    let extra = snapshot
        .terminal
        .macro_strip
        .iter()
        .find(|row| row.label == "HY Spread")
        .and_then(|row| row.current)
        .map(|v| format!(" · HY {v:.2}%"))
        .unwrap_or_default();
    let onchain = snapshot
        .terminal
        .mvrv_z
        .map(|v| format!(" · MVRV {v:+.2}"))
        .unwrap_or_default();
    let models = snapshot
        .terminal
        .models
        .first()
        .map(|m| format!(" · top model {}", m.label))
        .unwrap_or_default();
    format!(
        "{btc} · {dxy} · {y10} · {vix}{extra}{onchain}{models} · quotes {} · histories {} · {notes}",
        snapshot.quotes.len(),
        snapshot.histories.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(symbol: &str, price: f64, observed: i64, acquired: i64) -> parse::TickerBatch {
        parse::TickerBatch {
            rows: vec![(
                format!("{symbol}USDT"),
                price,
                2.5,
                price + 100.0,
                price - 100.0,
            )],
            acquired_unix: acquired,
            observed_unix: [(symbol.into(), observed)].into(),
        }
    }

    #[test]
    fn partial_spot_batch_does_not_freshen_unreturned_symbols() {
        let mut snap = crate::model::fallback_snapshot();
        apply_ticker_batch(&mut snap, &batch("BTC", 100_000.0, 1000, 1001));
        apply_ticker_batch(&mut snap, &batch("ETH", 3000.0, 1100, 1101));
        assert_eq!(snap.price_observed_unix("BTC"), Some(1000));
        assert_eq!(snap.price_acquired_unix("BTC"), Some(1001));
        assert_eq!(snap.price_observed_unix("ETH"), Some(1100));
        assert!(snap.price_stamp("ZEC").is_none());
    }

    #[test]
    fn background_result_preserves_only_newer_spot_groups() {
        let mut current = crate::model::fallback_snapshot();
        apply_ticker_batch(&mut current, &batch("BTC", 100_100.0, 2000, 2001));
        apply_ticker_batch(&mut current, &batch("ETH", 3000.0, 1900, 1901));
        current.feeds.insert("binance-spot".into(), 2001);
        let mut incoming = crate::model::fallback_snapshot();
        apply_ticker_batch(&mut incoming, &batch("BTC", 99_000.0, 1800, 1801));
        apply_ticker_batch(&mut incoming, &batch("ETH", 3200.0, 2100, 2101));
        preserve_newer_spot(&mut incoming, &current);
        assert_eq!(incoming.btc.price, Some(100_100.0));
        assert_eq!(incoming.btc.change_24h_pct, Some(2.5));
        assert_eq!(incoming.btc.high_24h, Some(100_200.0));
        assert_eq!(incoming.btc.low_24h, Some(100_000.0));
        assert_eq!(incoming.btc.eth_price, Some(3200.0));
        assert_eq!(incoming.price_observed_unix("BTC"), Some(2000));
        assert_eq!(incoming.price_observed_unix("ETH"), Some(2100));
        assert_eq!(incoming.feeds.get("binance-spot"), Some(&2001));
    }

    #[test]
    fn later_received_old_sample_does_not_replace_newer_observation() {
        let mut snap = crate::model::fallback_snapshot();
        apply_ticker_batch(&mut snap, &batch("BTC", 100_100.0, 3000, 3001));
        assert!(apply_ticker_batch(&mut snap, &batch("BTC", 99_000.0, 2900, 3101)).is_empty());
        assert_eq!(snap.btc.price, Some(100_100.0));
        assert_eq!(snap.price_acquired_unix("BTC"), Some(3001));
    }

    #[test]
    fn queued_batch_keeps_receipt_time_and_cannot_become_live_when_drained() {
        let mut snap = crate::model::fallback_snapshot();
        apply_ticker_batch(&mut snap, &batch("BTC", 100_100.0, 4000, 4001));
        let stamp = snap.price_stamp("BTC").unwrap();
        assert!(stamp.live_spot(4002));
        assert!(!stamp.live_spot(4300));
        assert_ne!(snap.price_as_of("BTC", 4300), "live");
    }

    #[test]
    #[ignore = "calls every live upstream feed"]
    fn live_refresh_reports_every_feed() {
        let snap = refresh(&crate::model::fallback_snapshot());
        println!("{}", summary(&snap));
        let now = now_unix();
        for feed in crate::feeds::FEEDS {
            println!(
                "{:<16} {}",
                feed.id,
                snap.feeds
                    .get(feed.id)
                    .map(|t| format!("{}s ago", now - t))
                    .unwrap_or_else(|| "no answer".into())
            );
        }
        println!("notes: {:#?}", snap.notes);
        assert!(snap.fetched_unix > 0);
    }
}
