use crate::model::{CurvePoint, Obs, Point, Quote};
use serde_json::Value;

pub fn treasury_latest(xml: &str) -> Option<(String, Vec<CurvePoint>)> {
    let mut last = None;
    for chunk in xml.split("<d:NEW_DATE").skip(1) {
        let date = first_text(chunk)?;
        let date = date.get(0..10).unwrap_or(date).to_string();
        let mut curve = Vec::new();
        for (tag, label) in [
            ("BC_1MONTH", "1M"),
            ("BC_3MONTH", "3M"),
            ("BC_1YEAR", "1Y"),
            ("BC_2YEAR", "2Y"),
            ("BC_5YEAR", "5Y"),
            ("BC_10YEAR", "10Y"),
            ("BC_30YEAR", "30Y"),
        ] {
            if let Some(value) = tag_number(chunk, tag) {
                curve.push(CurvePoint {
                    label: label.into(),
                    yield_pct: value,
                });
            }
        }
        if curve.iter().any(|p| p.label == "10Y") {
            last = Some((date, curve));
        }
    }
    last
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
        let symbol = if symbol == "BZ1" {
            "BRENT".to_string()
        } else {
            symbol
        };
        out.push(Quote {
            symbol,
            price,
            change_pct,
            market_cap,
        });
    }
    out
}

pub type TickerPrint = (String, f64, f64, f64, f64);

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
        let Some(price) = row["lastPrice"].as_str().and_then(|s| s.parse().ok()) else {
            continue;
        };
        let change = row["priceChangePercent"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let high = row["highPrice"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(price);
        let low = row["lowPrice"]
            .as_str()
            .and_then(|s| s.parse().ok())
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
        let body = r#"{"data":[{"s":"NASDAQ:NVDA","d":[225.07,0.22,5424187176514,"NVDA"]},{"s":"AMEX:URA","d":[40.91,0.12,null,"URA"]},{"s":"NYMEX:BZ1!","d":[104.32,-2.1,null,"BZ1!"]}]}"#;
        let quotes = tradingview_quotes(body);
        assert_eq!(quotes[0].symbol, "NVDA");
        assert_eq!(quotes[0].market_cap.unwrap() as u64, 5_424_187_176_514);
        assert!(quotes[1].market_cap.is_none());
        assert_eq!(quotes[2].symbol, "BRENT");
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
