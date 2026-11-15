use crate::model::{CurveDay, CurvePoint, Obs, Point, Quote};
use serde_json::Value;

pub fn treasury_latest(xml: &str) -> Option<(String, Vec<CurvePoint>)> {
    treasury_history(xml)
        .pop()
        .map(|day| (day.date, day.points))
}

pub fn treasury_history(xml: &str) -> Vec<CurveDay> {
    let mut days = Vec::new();
    for chunk in xml.split("<d:NEW_DATE").skip(1) {
        let Some(date) = first_text(chunk) else {
            continue;
        };
        let date = date.get(0..10).unwrap_or(date).to_string();
        let mut curve = Vec::new();
        for (tag, label) in [
            ("BC_1MONTH", "1M"),
            ("BC_3MONTH", "3M"),
            ("BC_6MONTH", "6M"),
            ("BC_1YEAR", "1Y"),
            ("BC_2YEAR", "2Y"),
            ("BC_3YEAR", "3Y"),
            ("BC_5YEAR", "5Y"),
            ("BC_7YEAR", "7Y"),
            ("BC_10YEAR", "10Y"),
            ("BC_20YEAR", "20Y"),
            ("BC_30YEAR", "30Y"),
        ] {
            if let Some(value) = tag_number(chunk, tag).filter(|v| v.is_finite()) {
                curve.push(CurvePoint {
                    label: label.into(),
                    yield_pct: value,
                });
            }
        }
        if curve.iter().any(|p| p.label == "10Y") {
            days.push(CurveDay {
                date,
                points: curve,
            });
        }
    }
    days.sort_by(|a, b| a.date.cmp(&b.date));
    days
}

/// Join curve days from several responses and the cache; the last source
/// wins on a shared date. Keeps roughly sixteen months for the comparisons.
pub fn merge_curve_days(sources: &[&[CurveDay]]) -> Vec<CurveDay> {
    let mut by_date = std::collections::BTreeMap::new();
    for source in sources {
        for day in *source {
            by_date.insert(day.date.clone(), day.clone());
        }
    }
    let mut days: Vec<CurveDay> = by_date.into_values().collect();
    let keep = days.len().saturating_sub(340);
    days.drain(..keep);
    days
}

fn first_text(chunk: &str) -> Option<&str> {
    let start = chunk.find('>')? + 1;
    let end = chunk[start..].find('<')? + start;
    let text = chunk[start..end].trim();
    (!text.is_empty()).then_some(text)
}

fn tag_number(block: &str, tag: &str) -> Option<f64> {
    let key = format!("<d:{tag}");
    let index = block.find(&key)?;
    let rest = &block[index + key.len()..];
    let start = rest.find('>')? + 1;
    let end = rest[start..].find('<')? + start;
    rest[start..end].trim().parse().ok()
}

pub fn fred_series(csv: &str) -> Vec<Obs> {
    let mut out = Vec::new();
    for line in csv.lines().skip(1) {
        let mut parts = line.split(',');
        let Some(date) = parts.next() else { continue };
        let Some(raw) = parts.next() else { continue };
        let raw = raw.trim();
        if raw.is_empty() || raw == "." {
            continue;
        }
        let Ok(value) = raw.parse::<f64>() else {
            continue;
        };
        if !value.is_finite() {
            continue;
        }
        out.push(Obs {
            date: date.trim().to_string(),
            value,
            source: "FRED".into(),
        });
    }
    out
}

pub fn fred_latest(csv: &str) -> Option<Obs> {
    fred_series(csv).pop()
}

pub fn months_before<'a>(series: &'a [Obs], latest: &Obs, days: i64) -> Option<&'a Obs> {
    let Ok(end) = chrono::NaiveDate::parse_from_str(latest.date.get(0..10)?, "%Y-%m-%d") else {
        return None;
    };
    let target = end - chrono::Duration::days(days);
    series.iter().rev().find(|obs| {
        chrono::NaiveDate::parse_from_str(obs.date.get(0..10).unwrap_or(""), "%Y-%m-%d")
            .map(|d| d <= target)
            .unwrap_or(false)
    })
}

pub fn cboe_vix(body: &str) -> Option<Obs> {
    let json: Value = serde_json::from_str(body).ok()?;
    let price = json["data"]["current_price"].as_f64()?;
    let raw_date = json["data"]["last_trade_time"]
        .as_str()
        .or_else(|| json["timestamp"].as_str())
        .unwrap_or("");
    let date = raw_date.get(0..10).unwrap_or("").to_string();
    Some(Obs {
        date,
        value: price,
        source: "CBOE".into(),
    })
}

pub fn gold_spot(body: &str) -> Option<Obs> {
    let json: Value = serde_json::from_str(body).ok()?;
    let price = json["price"].as_f64()?;
    let date = json["updatedAt"]
        .as_str()
        .and_then(|s| s.get(0..10))
        .unwrap_or("")
        .to_string();
    Some(Obs {
        date,
        value: price,
        source: "Gold API".into(),
    })
}

pub fn usd_jpy(body: &str) -> Option<Obs> {
    let json: Value = serde_json::from_str(body).ok()?;
    let value = json["rates"]["JPY"].as_f64()?;
    let date = json["time_last_update_unix"]
        .as_i64()
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    Some(Obs {
        date,
        value,
        source: "exchangerate-api".into(),
    })
}

pub fn tradingview_quotes(body: &str) -> Vec<Quote> {
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let Some(rows) = json["data"].as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows {
        let Some(cells) = row["d"].as_array() else {
            continue;
        };
        let symbol = cells
            .get(3)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| {
                row["s"]
                    .as_str()
                    .and_then(|s| s.rsplit(':').next())
                    .map(|s| s.trim_end_matches('!').to_string())
            });
        let Some(symbol) = symbol else { continue };
        let symbol = symbol.trim_end_matches('!').to_string();
        let Some(price) = cells.first().and_then(Value::as_f64) else {
            continue;
        };
        if !price.is_finite() || price <= 0.0 {
            continue;
        }
        let change_pct = cells.get(1).and_then(Value::as_f64).unwrap_or(0.0);
        let market_cap = cells
            .get(2)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v > 0.0);
        let change_abs = cells
            .get(4)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite());
        let symbol = match symbol.as_str() {
            "BZ1" => "BRENT".to_string(),
            "CL1" => "WTI".to_string(),
            "UX1" => "URANIUM".to_string(),
            _ => symbol,
        };
        out.push(Quote {
            symbol,
            price,
            change_pct,
            market_cap,
            change_abs,
        });
    }
    out
}

/// CNBC chart bars (`barData.priceBars`), one close per trading day, oldest first.
pub fn cnbc_bars(body: &str) -> Vec<Point> {
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let Some(rows) = json["barData"]["priceBars"].as_array() else {
        return Vec::new();
    };
    let mut out: Vec<Point> = Vec::new();
    for row in rows {
        let Some(ms) = row["tradeTimeinMills"].as_i64().or_else(|| {
            row["tradeTimeinMills"]
                .as_str()
                .and_then(|s| s.parse().ok())
        }) else {
            continue;
        };
        let Some(close) = row["close"]
            .as_f64()
            .or_else(|| row["close"].as_str().and_then(|s| s.parse().ok()))
            .filter(|v: &f64| v.is_finite() && *v > 0.0)
        else {
            continue;
        };
        // Bars carry the session date at midnight New York; key them by UTC day.
        let t = (ms / 1000).div_euclid(86_400) * 86_400;
        match out.last_mut() {
            Some(last) if last.t == t => last.v = close,
            _ => out.push(Point { t, v: close }),
        }
    }
    out.sort_by_key(|p| p.t);
    out.dedup_by_key(|p| p.t);
    out
}

pub type TickerPrint = (String, f64, f64, f64, f64);

/// A successful response retains its own receipt time and each ticker's close
/// time, even when it waits in the UI queue or in a longer background refresh.
#[derive(Clone, Debug)]
pub struct TickerBatch {
    pub rows: Vec<TickerPrint>,
    pub acquired_unix: i64,
    pub observed_unix: std::collections::BTreeMap<String, i64>,
}

pub fn binance_batch(body: &str, acquired_unix: i64) -> TickerBatch {
    let rows = binance_tickers(body);
    let mut observed_unix = std::collections::BTreeMap::new();
    if let Ok(json) = serde_json::from_str::<Value>(body) {
        let values = json.as_array().cloned().unwrap_or_else(|| vec![json]);
        for row in values {
            if let (Some(symbol), Some(close_ms)) =
                (row["symbol"].as_str(), row["closeTime"].as_i64())
            {
                let t = close_ms / 1000;
                if t > 0 && t <= acquired_unix + 5 && rows.iter().any(|r| r.0 == symbol) {
                    observed_unix.insert(symbol.trim_end_matches("USDT").into(), t);
                }
            }
        }
    }
    TickerBatch {
        rows,
        acquired_unix,
        observed_unix,
    }
}

pub fn binance_tickers(body: &str) -> Vec<TickerPrint> {
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let rows = if json.is_array() {
        json.as_array().cloned().unwrap_or_default()
    } else {
        vec![json]
    };
    let mut out = Vec::new();
    for row in rows {
        let Some(symbol) = row["symbol"].as_str() else {
            continue;
        };
        let Some(price) = row["lastPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|p| p.is_finite() && *p > 0.0)
        else {
            continue;
        };
        let change = row["priceChangePercent"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .filter(|v: &f64| v.is_finite())
            .unwrap_or(0.0);
        let high = row["highPrice"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .filter(|v: &f64| v.is_finite() && *v > 0.0)
            .unwrap_or(price);
        let low = row["lowPrice"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .filter(|v: &f64| v.is_finite() && *v > 0.0)
            .unwrap_or(price);
        out.push((symbol.to_string(), price, change, high, low));
    }
    out
}

pub fn klines(body: &str) -> Vec<Point> {
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let Some(rows) = json.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows {
        let Some(cells) = row.as_array() else {
            continue;
        };
        let Some(open_ms) = cells.first().and_then(Value::as_i64) else {
            continue;
        };
        let Some(close) = cells
            .get(4)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
        else {
            continue;
        };
        if close > 0.0 {
            out.push(Point {
                t: open_ms / 1000,
                v: close,
            });
        }
    }
    out.sort_by_key(|p| p.t);
    out
}

pub fn funding_rates(body: &str) -> Vec<f64> {
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let Some(rows) = json.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows {
        if let Some(rate) = row["fundingRate"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
        {
            if rate.is_finite() {
                out.push(rate);
            }
        }
    }
    out
}

pub fn fear_greed(body: &str) -> Option<(i32, String, Vec<f64>)> {
    let json: Value = serde_json::from_str(body).ok()?;
    let rows = json["data"].as_array()?;
    let mut hist = Vec::new();
    let mut current = None;
    for row in rows.iter().rev() {
        let Some(value) = row["value"].as_str().and_then(|s| s.parse::<i32>().ok()) else {
            continue;
        };
        hist.push(value as f64);
        let label = row["value_classification"]
            .as_str()
            .unwrap_or("")
            .to_string();
        current = Some((value, label));
    }
    let (value, label) = current?;
    Some((value, label, hist))
}

pub fn blockchain_prices(body: &str) -> Vec<Point> {
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let Some(rows) = json["values"].as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows {
        let Some(t) = row["x"].as_i64() else { continue };
        let Some(v) = row["y"].as_f64() else { continue };
        if v > 0.0 && v.is_finite() {
            out.push(Point { t, v });
        }
    }
    out.sort_by_key(|p| p.t);
    out
}

pub fn quote_obs(quotes: &[Quote], symbol: &str, source: &str, date: &str) -> Option<Obs> {
    let quote = quotes.iter().find(|q| q.symbol == symbol)?;
    Some(Obs {
        date: date.into(),
        value: quote.price,
        source: source.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spot_batch_keeps_per_instrument_close_times_and_rejects_bad_prices() {
        let batch = binance_batch(
            r#"[
            {"symbol":"BTCUSDT","lastPrice":"100000","closeTime":1700000000000},
            {"symbol":"ETHUSDT","lastPrice":"3000","closeTime":1699999999000},
            {"symbol":"ZECUSDT","lastPrice":"NaN","closeTime":1700000000000}
        ]"#,
            1700000001,
        );
        assert_eq!(batch.acquired_unix, 1700000001);
        assert_eq!(batch.rows.len(), 2);
        assert_eq!(batch.observed_unix.get("BTC"), Some(&1700000000));
        assert_eq!(batch.observed_unix.get("ETH"), Some(&1699999999));
        let undated = binance_batch(r#"{"symbol":"BTCUSDT","lastPrice":"100000"}"#, 1700000001);
        assert!(undated.observed_unix.is_empty());
    }

    #[test]
    fn treasury_history_orders_closes_and_skips_incomplete_days() {
        let xml = "<d:NEW_DATE>2026-09-28</d:NEW_DATE><d:BC_10YEAR>4.3</d:BC_10YEAR>\
            <d:NEW_DATE>2026-09-25</d:NEW_DATE><d:BC_10YEAR>4.2</d:BC_10YEAR>\
            <d:NEW_DATE>2026-09-29</d:NEW_DATE><d:BC_2YEAR>4.1</d:BC_2YEAR>";
        let days = treasury_history(xml);
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].date, "2026-09-25");
        assert_eq!(treasury_latest(xml).unwrap().0, "2026-09-28");
    }

    #[test]
    fn curve_history_spans_the_year_boundary() {
        let day = |date: &str, y10: f64| CurveDay {
            date: date.into(),
            points: vec![CurvePoint {
                label: "10Y".into(),
                yield_pct: y10,
            }],
        };
        let cached = vec![day("2026-12-30", 4.0), day("2026-12-31", 4.1)];
        let last_year = vec![day("2026-10-01", 3.9), day("2026-12-31", 4.15)];
        let this_year = vec![day("2027-01-04", 4.2)];
        let days = merge_curve_days(&[&cached, &last_year, &this_year]);
        let dates: Vec<_> = days.iter().map(|d| d.date.as_str()).collect();
        assert_eq!(dates, ["2026-10-01", "2026-12-30", "2026-12-31", "2027-01-04"]);
        assert_eq!(days[2].points[0].yield_pct, 4.15);
        assert!(merge_curve_days(&[&[], &this_year]).len() == 1);
    }

    #[test]
    fn treasury_parser_reads_the_last_complete_day() {
        let xml = r#"
            <d:NEW_DATE m:type="Edm.DateTime">2026-09-24T00:00:00</d:NEW_DATE>
            <d:BC_2YEAR m:type="Edm.Double">4.70</d:BC_2YEAR>
            <d:BC_10YEAR m:type="Edm.Double">5.00</d:BC_10YEAR>
            <d:BC_30YEAR m:type="Edm.Double">5.40</d:BC_30YEAR>
            <d:NEW_DATE m:type="Edm.DateTime">2026-09-25T00:00:00</d:NEW_DATE>
            <d:BC_1MONTH m:type="Edm.Double">4.04</d:BC_1MONTH>
            <d:BC_2YEAR m:type="Edm.Double">4.81</d:BC_2YEAR>
            <d:BC_10YEAR m:type="Edm.Double">5.17</d:BC_10YEAR>
            <d:BC_30YEAR m:type="Edm.Double">5.49</d:BC_30YEAR>
        "#;
        let (date, curve) = treasury_latest(xml).unwrap();
        assert_eq!(date, "2026-09-25");
        assert_eq!(
            curve.iter().find(|p| p.label == "10Y").unwrap().yield_pct,
            5.17
        );
        assert_eq!(
            curve.iter().find(|p| p.label == "1M").unwrap().yield_pct,
            4.04
        );
    }

    #[test]
    fn fred_skips_missing_prints() {
        let csv = "observation_date,DGS10\n2026-09-24,4.90\n2026-09-25,.\n2026-09-26,5.17\n";
        let latest = fred_latest(csv).unwrap();
        assert_eq!(latest.date, "2026-09-26");
        assert_eq!(latest.value, 5.17);
    }

    #[test]
    fn tradingview_keeps_null_market_caps() {
        let body = r#"{"data":[{"s":"NASDAQ:NVDA","d":[225.07,0.22,5424187176514,"NVDA"]},{"s":"AMEX:URA","d":[40.91,0.12,null,"URA"]},{"s":"NYMEX:BZ1!","d":[104.32,-2.1,null,"BZ1!"]},{"s":"TVC:US10Y","d":[5.081,0.04,null,"US10Y",0.002]},{"s":"NYMEX:CL1!","d":[99.02,1.07,null,"CL1!",1.05]},{"s":"COMEX:UX1!","d":[90.05,0.17,null,"UX1!",0.15]}]}"#;
        let quotes = tradingview_quotes(body);
        assert_eq!(quotes[0].symbol, "NVDA");
        assert_eq!(quotes[0].market_cap.unwrap() as u64, 5_424_187_176_514);
        assert!(quotes[1].market_cap.is_none());
        assert_eq!(quotes[2].symbol, "BRENT");
        assert_eq!(quotes[3].change_abs, Some(0.002));
        assert_eq!(quotes[4].symbol, "WTI");
        assert_eq!(quotes[5].symbol, "URANIUM");
    }

    #[test]
    fn cnbc_bars_read_string_closes_by_day() {
        let body = r#"{"barData":{"priceBars":[
            {"close":"103.4930","tradeTimeinMills":1729224000000},
            {"close":"103.4640","tradeTimeinMills":"1729396800000"},
            {"close":"bad","tradeTimeinMills":1729483200000},
            {"close":104.075,"tradeTimeinMills":1729569600000}]}}"#;
        let bars = cnbc_bars(body);
        assert_eq!(bars.len(), 3);
        assert_eq!(bars[0].t % 86_400, 0);
        assert_eq!(bars[2].v, 104.075);
        assert!(cnbc_bars("{}").is_empty());
    }

    #[test]
    fn fear_and_greed_is_oldest_first() {
        let body = r#"{"data":[{"value":"70","value_classification":"Greed"},{"value":"74","value_classification":"Greed"}]}"#;
        let (value, label, hist) = fear_greed(body).unwrap();
        assert_eq!(value, 70);
        assert_eq!(label, "Greed");
        assert_eq!(hist, vec![74.0, 70.0]);
    }
}
