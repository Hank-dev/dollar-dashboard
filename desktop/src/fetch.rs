use crate::model::{now_unix, BtcBook, Obs, Snapshot};
use crate::parse;
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
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let year = chrono::Utc::now().format("%Y");
    let broad_start = (chrono::Utc::now() - chrono::Duration::days(200))
        .format("%Y-%m-%d")
        .to_string();
    let fed_start = (chrono::Utc::now() - chrono::Duration::days(400))
        .format("%Y-%m-%d")
        .to_string();
    let need_history = prev.btc.daily.len() < 100 || now - prev.history_unix > HISTORY_TTL;
    let need_weekly = prev.btc.weekly.len() < 200 || now - prev.weekly_unix > HISTORY_TTL;

    let (
        treasury,
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
        history,
        weekly,
    ) = thread::scope(|scope| {
        let treasury = scope.spawn(|| {
            get_text(
                &client,
                &format!(
                    "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/pages/xml?data=daily_treasury_yield_curve&field_tdr_date_value={year}"
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
        let america = scope.spawn(|| scan(&client, "america", AMERICA));
        let global = scope.spawn(|| {
            scan(
                &client,
                "global",
                &["TVC:DXY", "TVC:JP10Y", "TVC:GOLD", "SP:SPX"],
            )
        });
        let futures = scope.spawn(|| scan(&client, "futures", &["NYMEX:BZ1!"]));
        let tickers = scope.spawn(|| {
            get_text(
                &client,
                "https://api.binance.com/api/v3/ticker/24hr?symbols=%5B%22BTCUSDT%22,%22ETHUSDT%22%5D",
            )
        });
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
        (
            treasury.join(),
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
            history.join(),
            weekly.join(),
        )
    });

    let mut next = prev.clone();
    next.notes.clear();
    let mut ok = 0u32;

    match flatten(treasury) {
        Ok(body) => match parse::treasury_latest(&body) {
            Some((date, curve)) => {
                let source = "U.S. Treasury";
                next.dollar.curve = curve.clone();
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
                ok += 1;
            }
            None => next
                .notes
                .push("Treasury yield curve had no usable day.".into()),
        },
        Err(err) => next.notes.push(format!("Treasury: {err}")),
    }

    match flatten(fred_broad) {
        Ok(body) => {
            let series = parse::fred_series(&body);
            if let Some(latest) = series.last() {
                let mut latest = latest.clone();
                latest.source = "FRED DTWEXBGS".into();
                next.dollar.dxy_broad = Some(latest);
                next.dollar.dxy_spark = series.iter().map(|obs| obs.value).collect();
                ok += 1;
            }
        }
        Err(err) => next.notes.push(format!("FRED broad dollar: {err}")),
    }
    apply_fed(&mut next, flatten(fred_upper), flatten(fred_lower), &mut ok);
    match flatten(vix) {
        Ok(body) => match parse::cboe_vix(&body) {
            Some(obs) => {
                next.dollar.vix = Some(obs);
                ok += 1;
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
                ok += 1;
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
                ok += 1;
            }
            None => next.notes.push("USD/JPY response was not readable.".into()),
        },
        Err(err) => next.notes.push(format!("USD/JPY: {err}")),
    }

    let mut quotes = Vec::new();
    for (label, result) in [
        ("TradingView equities", america),
        ("TradingView macro", global),
        ("TradingView futures", futures),
    ] {
        match flatten(result) {
            Ok(body) => {
                let parsed = parse::tradingview_quotes(&body);
                if parsed.is_empty() {
                    next.notes.push(format!("{label} returned no quotes."));
                } else {
                    quotes.extend(parsed);
                    ok += 1;
                }
            }
            Err(err) => next.notes.push(format!("{label}: {err}")),
        }
    }
    if !quotes.is_empty() {
        if let Some(dxy) = parse::quote_obs(&quotes, "DXY", "TradingView ICE DXY", &today) {
            next.dollar.dxy = Some(dxy);
        }
        if let Some(jgb) = parse::quote_obs(&quotes, "JP10Y", "TradingView", &today) {
            next.dollar.jgb10 = Some(jgb);
        }
        if let Some(spx) = parse::quote_obs(&quotes, "SPX", "TradingView", &today) {
            next.dollar.spx = Some(spx);
        }
        if let Some(brent) = parse::quote_obs(&quotes, "BRENT", "TradingView NYMEX Brent", &today) {
            next.dollar.brent = Some(brent);
        }
        if next.dollar.gold.is_none() {
            next.dollar.gold = parse::quote_obs(&quotes, "GOLD", "TradingView", &today);
        }
        next.quotes = quotes
            .into_iter()
            .filter(|q| {
                q.symbol
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            })
            .collect();
    }

    match flatten(tickers) {
        Ok(body) => {
            apply_tickers(&mut next.btc, &parse::binance_tickers(&body));
            if next.btc.price.is_some() {
                ok += 1;
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
                ok += 1;
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
                ok += 1;
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
                ok += 1;
            }
        }
        Err(err) => next.notes.push(format!("Binance daily history: {err}")),
    }
    if need_history {
        match flatten(history) {
            Ok(body) if !body.is_empty() => {
                let points = parse::blockchain_prices(&body);
                if points.len() >= 100 {
                    next.btc.daily = points;
                    next.history_unix = now;
                    ok += 1;
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
                    ok += 1;
                } else {
                    next.notes
                        .push("Weekly Bitcoin history was shorter than 200 weeks.".into());
                }
            }
            Ok(_) => {}
            Err(err) => next.notes.push(format!("Binance weekly history: {err}")),
        }
    }

    if ok > 0 {
        next.fetched_unix = now;
    } else if next.notes.is_empty() {
        next.notes.push("No source returned fresh data.".into());
    }
    next.notes.truncate(8);
    next
}

fn apply_fed(
    next: &mut Snapshot,
    upper: Result<String, String>,
    lower: Result<String, String>,
    ok: &mut u32,
) {
    match upper {
        Ok(body) => {
            let series = parse::fred_series(&body);
            if let Some(latest) = series.last().cloned() {
                let mut latest = latest;
                latest.source = "FRED DFEDTARU".into();
                next.dollar.fed_hi_6m = parse::months_before(&series, &latest, 182).cloned();
                next.dollar.fed_hi = Some(latest);
                *ok += 1;
            }
        }
        Err(err) => next.notes.push(format!("FRED fed upper: {err}")),
    }
    match lower {
        Ok(body) => {
            if let Some(mut latest) = parse::fred_latest(&body) {
                latest.source = "FRED DFEDTARL".into();
                next.dollar.fed_lo = Some(latest);
                *ok += 1;
            }
        }
        Err(err) => next.notes.push(format!("FRED fed lower: {err}")),
    }
}

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
            _ => {}
        }
    }
}

fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(25))
        .user_agent("Mozilla/5.0 (compatible; dollar-dashboard/0.1; +https://github.com/Hank-dev/dollar-dashboard)")
        .build()
        .map_err(|err| err.to_string())
}

fn get_text(client: &Client, url: &str) -> Result<String, String> {
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
            "columns": ["close", "change", "market_cap_basic", "name"]
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

fn flatten(result: thread::Result<Result<String, String>>) -> Result<String, String> {
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
    format!(
        "{btc} · {dxy} · {y10} · {vix} · quotes {} · {notes}",
        snapshot.quotes.len()
    )
}
