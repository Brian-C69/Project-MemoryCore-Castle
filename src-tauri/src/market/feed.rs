//! Bybit public prices for USDT perpetuals (v5 REST, `category=linear`).
//!
//! Keyless GETs and nothing else: this file can read prices and can do nothing to any account.
//! The parsers are pure functions over the JSON Bybit sends, so they are tested on saved real answers;
//! `BybitFeed` is the small blocking client that fetches those answers.

// The MARKET room calls into this file in later tasks; until then its items are not used yet.
#![allow(dead_code)]

use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

const BASE: &str = "https://api.bybit.com";
const CATEGORY: &str = "linear";
const TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = concat!("MemoryCoreCastle/", env!("CARGO_PKG_VERSION"));
/// The deepest book this feed keeps on each side.
pub const MAX_BOOK_DEPTH: usize = 50;
const MAX_INSTRUMENT_PAGES: usize = 5;
const MAX_RISK_PAGES: usize = 10;

/// The latest prices of one perpetual. Times are epoch milliseconds.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub symbol: String,
    pub last: f64,
    pub mark: f64,
    pub index: f64,
    pub bid: f64,
    pub ask: f64,
    pub funding_rate: f64,
    pub next_funding_ms: i64,
    /// When Bybit answered.
    pub at_ms: i64,
}

/// One coin in the whole market's reading. `change_24h_pct` is a fraction: 0.05 is up five percent.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticker {
    pub symbol: String,
    pub last: f64,
    pub change_24h_pct: f64,
    pub turnover_24h: f64,
    pub volume_24h: f64,
    pub high_24h: f64,
    pub low_24h: f64,
    pub funding_rate: f64,
}

/// One price on one side of the book.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookLevel {
    pub price: f64,
    pub qty: f64,
}

/// Both sides of the public book, best price first.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
    pub at_ms: i64,
}

/// One candle. Lists are always oldest first. Volume is zero for a mark price candle.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candle {
    pub open_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// A funding rate that has already settled.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRate {
    pub symbol: String,
    pub rate: f64,
    pub time_ms: i64,
}

/// What Bybit publishes about how one perpetual may be ordered. Never typed by hand.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentRules {
    pub symbol: String,
    pub tick_size: f64,
    pub qty_step: f64,
    pub min_qty: f64,
    /// Largest limit order.
    pub max_qty: f64,
    /// Largest market order, when Bybit gives one.
    pub max_market_qty: Option<f64>,
    pub min_notional: Option<f64>,
    pub min_leverage: f64,
    pub max_leverage: f64,
    pub leverage_step: f64,
    pub funding_interval_minutes: u32,
}

/// One rung of a perpetual's risk table. Rates are fractions (0.005 is 0.5 %).
/// `maintenance_deduction` is zero on the lowest rung, where Bybit sends it blank.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskTier {
    pub id: u32,
    /// The largest position value this rung covers.
    pub risk_limit_value: f64,
    pub maintenance_margin_rate: f64,
    pub initial_margin_rate: f64,
    pub max_leverage: f64,
    pub maintenance_deduction: f64,
}

/// The Bybit interval name for a chart button ("1m", "5m", "15m", "1H", "4H", "1D").
pub fn interval_name(ui: &str) -> Option<&'static str> {
    match ui {
        "1m" => Some("1"),
        "5m" => Some("5"),
        "15m" => Some("15"),
        "1H" | "1h" => Some("60"),
        "4H" | "4h" => Some("240"),
        "1D" | "1d" => Some("D"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Reading numbers and fields. Bybit sends most numbers as strings.
// ---------------------------------------------------------------------------

/// A number that must be there and must not be blank.
fn num(row: &Value, key: &str) -> Result<f64, String> {
    match row.get(key) {
        None | Some(Value::Null) => Err(format!("Bybit sent no {key}")),
        Some(Value::String(s)) if s.trim().is_empty() => Err(format!("Bybit sent a blank {key}")),
        Some(v) => read_number(v, key),
    }
}

/// A number that reads as zero when missing or blank (a new coin's ticker, the lowest risk rung's deduction).
fn num_or_zero(row: &Value, key: &str) -> Result<f64, String> {
    Ok(opt_num(row, key)?.unwrap_or(0.0))
}

/// A number that may be missing or blank.
fn opt_num(row: &Value, key: &str) -> Result<Option<f64>, String> {
    match row.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(None),
        Some(v) => read_number(v, key).map(Some),
    }
}

fn read_number(v: &Value, key: &str) -> Result<f64, String> {
    let n = match v {
        Value::String(s) => s.trim().parse::<f64>().ok(),
        Value::Number(n) => n.as_f64(),
        _ => None,
    };
    match n {
        Some(n) if n.is_finite() => Ok(n),
        _ => Err(format!("Bybit sent an unreadable {key}")),
    }
}

/// A whole number (a time in milliseconds, an id), sent as a string or a number.
fn int(row: &Value, key: &str) -> Result<i64, String> {
    let n = match row.get(key) {
        None | Some(Value::Null) => return Err(format!("Bybit sent no {key}")),
        Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
        Some(Value::Number(n)) => n.as_i64(),
        Some(_) => None,
    };
    n.ok_or_else(|| format!("Bybit sent an unreadable {key}"))
}

fn text<'a>(row: &'a Value, key: &str) -> Result<&'a str, String> {
    match row.get(key).and_then(Value::as_str) {
        Some(s) if !s.is_empty() => Ok(s),
        _ => Err(format!("Bybit sent no {key}")),
    }
}

/// One cell of an array row, as a number.
fn cell(row: &[Value], at: usize, what: &str) -> Result<f64, String> {
    let v = row.get(at).ok_or_else(|| format!("Bybit sent a {what} with too few fields"))?;
    read_number(v, what)
}

/// The `result` of a Bybit answer, or the reason Bybit gave for refusing.
fn result_of(answer: &Value) -> Result<&Value, String> {
    let code = answer.get("retCode").and_then(Value::as_i64).ok_or("Bybit sent no retCode")?;
    if code != 0 {
        let reason = answer.get("retMsg").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("no reason given");
        return Err(format!("Bybit said: {reason}"));
    }
    answer.get("result").filter(|r| r.is_object()).ok_or_else(|| "Bybit sent no result".to_string())
}

/// The rows of `result.list`.
fn list_of(answer: &Value) -> Result<&Vec<Value>, String> {
    result_of(answer)?.get("list").and_then(Value::as_array).ok_or_else(|| "Bybit sent no list".to_string())
}

/// When Bybit answered, from the top-level `time`; zero when it is missing.
fn answer_time(answer: &Value) -> i64 {
    int(answer, "time").unwrap_or(0)
}

/// The cursor for the next page, when there is one.
pub fn next_cursor(answer: &Value) -> Option<String> {
    answer
        .get("result")
        .and_then(|r| r.get("nextPageCursor"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

// ---------------------------------------------------------------------------
// Parsers, one per endpoint. Each takes Bybit's whole answer.
// ---------------------------------------------------------------------------

/// `/v5/market/tickers` for the whole market. A new coin's blank numbers read as zero.
pub fn parse_tickers(answer: &Value) -> Result<Vec<Ticker>, String> {
    list_of(answer)?
        .iter()
        .map(|row| {
            Ok(Ticker {
                symbol: text(row, "symbol")?.to_string(),
                last: num_or_zero(row, "lastPrice")?,
                change_24h_pct: num_or_zero(row, "price24hPcnt")?,
                turnover_24h: num_or_zero(row, "turnover24h")?,
                volume_24h: num_or_zero(row, "volume24h")?,
                high_24h: num_or_zero(row, "highPrice24h")?,
                low_24h: num_or_zero(row, "lowPrice24h")?,
                funding_rate: num_or_zero(row, "fundingRate")?,
            })
        })
        .collect()
}

/// `/v5/market/tickers` read for one symbol.
pub fn parse_quote(answer: &Value, symbol: &str) -> Result<Quote, String> {
    let row = list_of(answer)?
        .iter()
        .find(|row| row.get("symbol").and_then(Value::as_str) == Some(symbol))
        .ok_or_else(|| format!("Bybit does not know {symbol}"))?;
    Ok(Quote {
        symbol: symbol.to_string(),
        last: num(row, "lastPrice")?,
        mark: num(row, "markPrice")?,
        index: num(row, "indexPrice")?,
        bid: num(row, "bid1Price")?,
        ask: num(row, "ask1Price")?,
        funding_rate: num_or_zero(row, "fundingRate")?,
        next_funding_ms: int(row, "nextFundingTime")?,
        at_ms: answer_time(answer),
    })
}

/// `/v5/market/orderbook`, best price first, at most 50 levels a side.
pub fn parse_book(answer: &Value) -> Result<Book, String> {
    let result = result_of(answer)?;
    let side = |key: &str| -> Result<Vec<BookLevel>, String> {
        let rows = result.get(key).and_then(Value::as_array).ok_or_else(|| format!("Bybit sent no book side {key}"))?;
        rows.iter()
            .take(MAX_BOOK_DEPTH)
            .map(|level| {
                let level = level.as_array().ok_or("Bybit sent an unreadable book level")?;
                Ok(BookLevel { price: cell(level, 0, "book price")?, qty: cell(level, 1, "book quantity")? })
            })
            .collect()
    };
    let at_ms = int(result, "ts").unwrap_or_else(|_| answer_time(answer));
    Ok(Book { bids: side("b")?, asks: side("a")?, at_ms })
}

/// Candle rows, oldest first. Bybit lists newest first.
fn candle_rows(answer: &Value, with_volume: bool) -> Result<Vec<Candle>, String> {
    let mut candles = list_of(answer)?
        .iter()
        .map(|row| {
            let row = row.as_array().ok_or("Bybit sent an unreadable candle")?;
            let open_ms = row
                .get(0)
                .and_then(|v| v.as_str().and_then(|s| s.parse::<i64>().ok()).or_else(|| v.as_i64()))
                .ok_or("Bybit sent a candle with no start time")?;
            Ok(Candle {
                open_ms,
                open: cell(row, 1, "candle")?,
                high: cell(row, 2, "candle")?,
                low: cell(row, 3, "candle")?,
                close: cell(row, 4, "candle")?,
                volume: if with_volume { cell(row, 5, "candle")? } else { 0.0 },
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    candles.sort_by_key(|c| c.open_ms);
    Ok(candles)
}

/// `/v5/market/kline`, oldest first.
pub fn parse_candles(answer: &Value) -> Result<Vec<Candle>, String> {
    candle_rows(answer, true)
}

/// `/v5/market/mark-price-kline`, oldest first, volume zero.
pub fn parse_mark_candles(answer: &Value) -> Result<Vec<Candle>, String> {
    candle_rows(answer, false)
}

/// `/v5/market/funding/history`, oldest first.
pub fn parse_funding_history(answer: &Value) -> Result<Vec<FundingRate>, String> {
    let mut rates = list_of(answer)?
        .iter()
        .map(|row| {
            Ok(FundingRate {
                symbol: text(row, "symbol")?.to_string(),
                rate: num(row, "fundingRate")?,
                time_ms: int(row, "fundingRateTimestamp")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    rates.sort_by_key(|r| r.time_ms);
    Ok(rates)
}

/// `/v5/market/instruments-info`: the rules of every USDT perpetual trading now on this page.
/// Other contracts (USDC, dated futures, closed coins) are left out.
pub fn parse_instruments(answer: &Value) -> Result<Vec<InstrumentRules>, String> {
    let is_usdt_perp = |row: &&Value| {
        let field = |k: &str| row.get(k).and_then(Value::as_str);
        field("status") == Some("Trading") && field("contractType") == Some("LinearPerpetual") && field("settleCoin") == Some("USDT")
    };
    list_of(answer)?
        .iter()
        .filter(is_usdt_perp)
        .map(|row| {
            let empty = Value::Null;
            let price = row.get("priceFilter").unwrap_or(&empty);
            let lot = row.get("lotSizeFilter").unwrap_or(&empty);
            let leverage = row.get("leverageFilter").unwrap_or(&empty);
            let interval = int(row, "fundingInterval")?;
            Ok(InstrumentRules {
                symbol: text(row, "symbol")?.to_string(),
                tick_size: num(price, "tickSize")?,
                qty_step: num(lot, "qtyStep")?,
                min_qty: num(lot, "minOrderQty")?,
                max_qty: num(lot, "maxOrderQty")?,
                max_market_qty: opt_num(lot, "maxMktOrderQty")?,
                min_notional: opt_num(lot, "minNotionalValue")?,
                min_leverage: num(leverage, "minLeverage")?,
                max_leverage: num(leverage, "maxLeverage")?,
                leverage_step: num(leverage, "leverageStep")?,
                funding_interval_minutes: u32::try_from(interval).map_err(|_| "Bybit sent an unreadable fundingInterval".to_string())?,
            })
        })
        .collect()
}

/// The rows of one risk-limit page, as sent.
fn risk_rows(answer: &Value) -> Result<Vec<RiskTier>, String> {
    list_of(answer)?
        .iter()
        .map(|row| {
            Ok(RiskTier {
                id: u32::try_from(int(row, "id")?).map_err(|_| "Bybit sent an unreadable id".to_string())?,
                risk_limit_value: num(row, "riskLimitValue")?,
                maintenance_margin_rate: num(row, "maintenanceMargin")?,
                initial_margin_rate: num(row, "initialMargin")?,
                max_leverage: num(row, "maxLeverage")?,
                maintenance_deduction: num_or_zero(row, "mmDeduction")?,
            })
        })
        .collect()
}

fn sorted_tiers(mut tiers: Vec<RiskTier>) -> Result<Vec<RiskTier>, String> {
    if tiers.is_empty() {
        return Err("Bybit has no risk table for this symbol".to_string());
    }
    tiers.sort_by(|a, b| a.risk_limit_value.total_cmp(&b.risk_limit_value));
    Ok(tiers)
}

/// `/v5/market/risk-limit`, lowest rung first.
pub fn parse_risk_tiers(answer: &Value) -> Result<Vec<RiskTier>, String> {
    sorted_tiers(risk_rows(answer)?)
}

// ---------------------------------------------------------------------------
// The client.
// ---------------------------------------------------------------------------

/// A blocking reader of Bybit's public market data. No key, no account, GETs only.
pub struct BybitFeed {
    base: String,
    client: reqwest::blocking::Client,
}

impl BybitFeed {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(TIMEOUT)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|_| "Could not start the price reader".to_string())?;
        Ok(Self { base: BASE.to_string(), client })
    }

    /// One GET under `/v5/market/`, always for linear contracts.
    fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value, String> {
        let response = self
            .client
            .get(format!("{}/v5/market/{path}", self.base))
            .header(reqwest::header::ACCEPT, "application/json")
            .query(&[("category", CATEGORY)])
            .query(query)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    format!("Bybit took longer than {} seconds to answer", TIMEOUT.as_secs())
                } else {
                    "Could not reach Bybit".to_string()
                }
            })?;
        let status = response.status().as_u16();
        match status {
            403 | 451 => return Err(format!("Bybit refused this computer (HTTP {status})")),
            418 | 429 => return Err("Bybit asked us to slow down".to_string()),
            200..=299 => {}
            _ => return Err(format!("Bybit answered HTTP {status}")),
        }
        response.json::<Value>().map_err(|_| "Bybit did not answer in JSON".to_string())
    }

    /// Every coin at once, in one large answer.
    pub fn tickers(&self) -> Result<Vec<Ticker>, String> {
        parse_tickers(&self.get("tickers", &[])?)
    }

    pub fn quote(&self, symbol: &str) -> Result<Quote, String> {
        parse_quote(&self.get("tickers", &[("symbol", symbol.to_string())])?, symbol)
    }

    /// The book, `depth` levels a side (1 to 50).
    pub fn book(&self, symbol: &str, depth: usize) -> Result<Book, String> {
        let depth = depth.clamp(1, MAX_BOOK_DEPTH);
        parse_book(&self.get("orderbook", &[("symbol", symbol.to_string()), ("limit", depth.to_string())])?)
    }

    /// The newest `limit` traded candles (Bybit allows up to 1000), oldest first.
    /// `interval` is Bybit's name: "1", "5", "15", "60", "240", "D".
    pub fn candles(&self, symbol: &str, interval: &str, limit: u32) -> Result<Vec<Candle>, String> {
        let query = [("symbol", symbol.to_string()), ("interval", interval.to_string()), ("limit", limit.clamp(1, 1000).to_string())];
        parse_candles(&self.get("kline", &query)?)
    }

    /// The `limit` traded candles right before `end_ms` (Bybit allows up to 1000), oldest first -
    /// scrolling a chart back past what it already has asks for more this way, one page at a time.
    pub fn candles_before(&self, symbol: &str, interval: &str, limit: u32, end_ms: i64) -> Result<Vec<Candle>, String> {
        let query = [
            ("symbol", symbol.to_string()),
            ("interval", interval.to_string()),
            ("end", end_ms.to_string()),
            ("limit", limit.clamp(1, 1000).to_string()),
        ];
        parse_candles(&self.get("kline", &query)?)
    }

    /// Mark price candles in a window (at most 1000), oldest first.
    pub fn mark_candles(&self, symbol: &str, interval: &str, start_ms: i64, end_ms: i64) -> Result<Vec<Candle>, String> {
        let query = [
            ("symbol", symbol.to_string()),
            ("interval", interval.to_string()),
            ("start", start_ms.to_string()),
            ("end", end_ms.to_string()),
            ("limit", "1000".to_string()),
        ];
        parse_mark_candles(&self.get("mark-price-kline", &query)?)
    }

    /// Settled funding rates in a window (at most 200), oldest first.
    pub fn funding_history(&self, symbol: &str, start_ms: i64, end_ms: i64) -> Result<Vec<FundingRate>, String> {
        let query = [
            ("symbol", symbol.to_string()),
            ("startTime", start_ms.to_string()),
            ("endTime", end_ms.to_string()),
            ("limit", "200".to_string()),
        ];
        parse_funding_history(&self.get("funding/history", &query)?)
    }

    /// The rules of every USDT perpetual trading now, by symbol. A large answer: read it rarely and keep it.
    pub fn instruments(&self) -> Result<Vec<InstrumentRules>, String> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..MAX_INSTRUMENT_PAGES {
            let mut query = vec![("limit", "1000".to_string())];
            if let Some(c) = &cursor {
                query.push(("cursor", c.clone()));
            }
            let answer = self.get("instruments-info", &query)?;
            all.extend(parse_instruments(&answer)?);
            cursor = next_cursor(&answer);
            if cursor.is_none() {
                break;
            }
        }
        if all.is_empty() {
            return Err("Bybit listed no USDT perpetuals".to_string());
        }
        all.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        Ok(all)
    }

    /// The risk table, lowest rung first.
    pub fn risk_tiers(&self, symbol: &str) -> Result<Vec<RiskTier>, String> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..MAX_RISK_PAGES {
            let mut query = vec![("symbol", symbol.to_string())];
            if let Some(c) = &cursor {
                query.push(("cursor", c.clone()));
            }
            let answer = self.get("risk-limit", &query)?;
            all.extend(risk_rows(&answer)?);
            cursor = next_cursor(&answer);
            if cursor.is_none() {
                break;
            }
        }
        sorted_tiers(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Real Bybit answers, saved once and trimmed.
    const TICKERS_BTCUSDT: &str = r#"{"retCode":0,"retMsg":"OK","result":{"category":"linear","list":[{"symbol":"BTCUSDT","lastPrice":"80370.90","indexPrice":"80378.84","markPrice":"80369.70","prevPrice24h":"78427.30","price24hPcnt":"0.024782","highPrice24h":"81181.10","lowPrice24h":"77966.00","prevPrice1h":"80252.40","openInterest":"67767.772","openInterestValue":"5446475505.31","singleOpenInterest":"33883.886","singleOpenInterestValue":"2723237752.65","turnover24h":"5751546648.4583","volume24h":"71863.1120","fundingRate":"0.0001","nextFundingTime":"1793894400000","predictedDeliveryPrice":"","basisRate":"","deliveryFeeRate":"","deliveryTime":"0","ask1Size":"2.875","bid1Price":"80370.90","ask1Price":"80371.00","bid1Size":"20.854","basis":"","preOpenPrice":"","preQty":"","curPreListingPhase":"","fundingIntervalHour":"8","basisRateYear":"","fundingCap":"0.00333"}]},"retExtInfo":{},"time":1793879980231}"#;

    const KLINE_BTCUSDT: &str = r#"{"retCode":0,"retMsg":"OK","result":{"symbol":"BTCUSDT","category":"linear","list":[["1793879940000","80370","80380.1","80365.3","80370.9","16.247","1305811.7688"],["1793879880000","80357.1","80370","80356.5","80370","9.76","784310.2047"],["1793879820000","80353.7","80370","80353.7","80357.1","8.688","698219.5121"]]},"retExtInfo":{},"time":1793879981125}"#;

    const MARK_KLINE_BTCUSDT: &str = r#"{"retCode":0,"retMsg":"OK","result":{"symbol":"BTCUSDT","category":"linear","list":[["1793879940000","80369.3","80380.1","80369.3","80370.9"],["1793879880000","80361.42","80369.3","80360","80369.3"],["1793879820000","80359.11","80371.66","80359.11","80361.42"]]},"retExtInfo":{},"time":1793879981604}"#;

    const FUNDING_BTCUSDT: &str = r#"{"retCode":0,"retMsg":"OK","result":{"category":"linear","list":[{"symbol":"BTCUSDT","fundingRate":"0.0001","fundingRateTimestamp":"1793865600000"},{"symbol":"BTCUSDT","fundingRate":"0.00005932","fundingRateTimestamp":"1793836800000"},{"symbol":"BTCUSDT","fundingRate":"0.0001","fundingRateTimestamp":"1793808000000"}]},"retExtInfo":{},"time":1793879982816}"#;

    const INSTRUMENT_BTCUSDT: &str = r#"{"retCode":0,"retMsg":"OK","result":{"category":"linear","list":[{"symbol":"BTCUSDT","contractType":"LinearPerpetual","status":"Trading","baseCoin":"BTC","quoteCoin":"USDT","launchTime":"1584230400000","deliveryTime":"0","deliveryFeeRate":"","priceScale":"2","leverageFilter":{"minLeverage":"1","maxLeverage":"150.00","leverageStep":"0.01"},"priceFilter":{"minPrice":"0.10","maxPrice":"1999999.80","tickSize":"0.10"},"lotSizeFilter":{"maxOrderQty":"1500.000","minOrderQty":"0.001","qtyStep":"0.001","postOnlyMaxOrderQty":"1500.000","maxMktOrderQty":"150.000","minNotionalValue":"5"},"unifiedMarginTrade":true,"fundingInterval":480,"settleCoin":"USDT","copyTrading":"both","upperFundingRate":"0.00333","lowerFundingRate":"-0.00333","isPreListing":false,"preListingInfo":null,"riskParameters":{"priceLimitRatioX":"0.01","priceLimitRatioY":"0.02"},"displayName":"BTCUSDT","symbolType":"","forbidUplWithdrawal":false,"symbolId":5,"fullName":"Bitcoin","marketRegion":"","underlyingTicker":"","tags":[]}],"nextPageCursor":""},"retExtInfo":{},"time":1793879641617}"#;

    const INSTRUMENT_UNKNOWN: &str = r#"{"retCode":10001,"retMsg":"params error: symbol invalid","result":{"category":"","list":[],"nextPageCursor":""},"retExtInfo":{},"time":1793879983185}"#;

    const ORDERBOOK_BTCUSDT: &str = r#"{"retCode":0,"retMsg":"OK","result":{"s":"BTCUSDT","a":[["80371.00","1.500"],["80372.50","0.002"],["80372.60","0.001"],["80373.30","0.002"],["80373.80","0.002"],["80374.10","0.002"],["80374.50","0.001"],["80374.60","0.001"],["80374.90","0.002"],["80375.30","0.002"],["80375.60","0.003"],["80375.70","0.002"],["80376.00","0.003"],["80376.20","0.399"],["80376.30","0.001"],["80376.40","0.001"],["80376.50","0.002"],["80376.70","0.041"],["80376.80","1.011"],["80377.50","0.001"],["80378.10","0.093"],["80378.50","0.141"],["80379.00","0.002"],["80379.30","0.001"],["80379.50","0.002"],["80379.80","0.001"],["80380.00","0.002"],["80380.10","0.009"],["80380.20","1.663"],["80380.30","3.380"],["80380.40","2.088"],["80380.50","0.001"],["80380.70","0.033"],["80380.90","0.003"],["80381.40","0.004"],["80381.60","0.070"],["80381.80","0.003"],["80382.20","0.003"],["80382.40","0.002"],["80382.50","1.036"],["80382.60","2.912"],["80382.70","0.002"],["80382.80","0.001"],["80383.00","0.700"],["80383.10","0.002"],["80383.40","0.004"],["80383.80","0.002"],["80383.90","4.480"],["80384.00","2.536"],["80384.10","2.156"]],"b":[["80370.90","23.948"],["80370.80","0.599"],["80370.70","0.002"],["80370.60","0.002"],["80370.30","0.003"],["80370.00","0.001"],["80369.80","0.529"],["80369.70","0.256"],["80369.40","0.002"],["80368.60","0.053"],["80368.00","0.001"],["80367.90","0.002"],["80367.70","0.002"],["80367.50","0.001"],["80367.10","0.002"],["80367.00","1.004"],["80366.90","0.125"],["80366.70","0.070"],["80366.00","1.592"],["80365.40","0.013"],["80365.30","0.079"],["80365.00","0.002"],["80364.80","0.001"],["80364.70","0.139"],["80364.50","0.002"],["80364.10","0.001"],["80364.00","1.788"],["80363.70","0.002"],["80363.60","0.001"],["80363.50","0.052"],["80362.60","0.002"],["80362.50","0.001"],["80362.40","0.001"],["80362.10","0.024"],["80362.00","0.003"],["80361.70","0.002"],["80361.60","0.001"],["80361.50","1.738"],["80361.40","0.052"],["80360.90","0.001"],["80360.40","0.017"],["80360.10","5.656"],["80360.00","3.985"],["80359.90","0.001"],["80359.70","0.467"],["80359.60","0.017"],["80359.10","1.320"],["80359.00","0.038"],["80358.90","0.003"],["80358.80","0.044"]],"ts":1793879980559,"u":10027417,"seq":839353123043,"cts":1793879980558},"retExtInfo":{},"time":1793879980695}"#;

    const RISK_LIMIT_BTCUSDT: &str = r#"{"retCode":0,"retMsg":"OK","result":{"category":"linear","list":[{"id":1,"symbol":"BTCUSDT","riskLimitValue":"300000","maintenanceMargin":"0.0033","initialMargin":"0.0066","isLowestRisk":1,"maxLeverage":"150.00","mmDeduction":""},{"id":2,"symbol":"BTCUSDT","riskLimitValue":"2000000","maintenanceMargin":"0.005","initialMargin":"0.01","isLowestRisk":0,"maxLeverage":"100.00","mmDeduction":"510"},{"id":3,"symbol":"BTCUSDT","riskLimitValue":"2600000","maintenanceMargin":"0.0056","initialMargin":"0.0111","isLowestRisk":0,"maxLeverage":"90.00","mmDeduction":"1710"},{"id":4,"symbol":"BTCUSDT","riskLimitValue":"3200000","maintenanceMargin":"0.0063","initialMargin":"0.0125","isLowestRisk":0,"maxLeverage":"80.00","mmDeduction":"3530"},{"id":5,"symbol":"BTCUSDT","riskLimitValue":"3800000","maintenanceMargin":"0.0067","initialMargin":"0.0133","isLowestRisk":0,"maxLeverage":"75.00","mmDeduction":"4810"},{"id":6,"symbol":"BTCUSDT","riskLimitValue":"4400000","maintenanceMargin":"0.0071","initialMargin":"0.0143","isLowestRisk":0,"maxLeverage":"70.00","mmDeduction":"6330"},{"id":7,"symbol":"BTCUSDT","riskLimitValue":"5000000","maintenanceMargin":"0.0077","initialMargin":"0.0154","isLowestRisk":0,"maxLeverage":"65.00","mmDeduction":"8970"},{"id":8,"symbol":"BTCUSDT","riskLimitValue":"5600000","maintenanceMargin":"0.0091","initialMargin":"0.0182","isLowestRisk":0,"maxLeverage":"55.00","mmDeduction":"15970"},{"id":9,"symbol":"BTCUSDT","riskLimitValue":"8500000","maintenanceMargin":"0.01","initialMargin":"0.02","isLowestRisk":0,"maxLeverage":"50.00","mmDeduction":"21010"},{"id":10,"symbol":"BTCUSDT","riskLimitValue":"10000000","maintenanceMargin":"0.013","initialMargin":"0.0222","isLowestRisk":0,"maxLeverage":"45.00","mmDeduction":"46510"},{"id":11,"symbol":"BTCUSDT","riskLimitValue":"14000000","maintenanceMargin":"0.015","initialMargin":"0.025","isLowestRisk":0,"maxLeverage":"40.00","mmDeduction":"66510"},{"id":12,"symbol":"BTCUSDT","riskLimitValue":"20000000","maintenanceMargin":"0.016","initialMargin":"0.0286","isLowestRisk":0,"maxLeverage":"35.00","mmDeduction":"80510"},{"id":13,"symbol":"BTCUSDT","riskLimitValue":"28000000","maintenanceMargin":"0.02","initialMargin":"0.0333","isLowestRisk":0,"maxLeverage":"30.00","mmDeduction":"160510"},{"id":14,"symbol":"BTCUSDT","riskLimitValue":"38000000","maintenanceMargin":"0.025","initialMargin":"0.04","isLowestRisk":0,"maxLeverage":"25.00","mmDeduction":"300510"},{"id":15,"symbol":"BTCUSDT","riskLimitValue":"50000000","maintenanceMargin":"0.029","initialMargin":"0.05","isLowestRisk":0,"maxLeverage":"20.00","mmDeduction":"452510"},{"id":16,"symbol":"BTCUSDT","riskLimitValue":"55000000","maintenanceMargin":"0.03","initialMargin":"0.0526","isLowestRisk":0,"maxLeverage":"19.00","mmDeduction":"502510"},{"id":17,"symbol":"BTCUSDT","riskLimitValue":"60000000","maintenanceMargin":"0.035","initialMargin":"0.0556","isLowestRisk":0,"maxLeverage":"18.00","mmDeduction":"777510"},{"id":18,"symbol":"BTCUSDT","riskLimitValue":"65000000","maintenanceMargin":"0.036","initialMargin":"0.0588","isLowestRisk":0,"maxLeverage":"17.00","mmDeduction":"837510"},{"id":19,"symbol":"BTCUSDT","riskLimitValue":"70000000","maintenanceMargin":"0.038","initialMargin":"0.0625","isLowestRisk":0,"maxLeverage":"16.00","mmDeduction":"967510"},{"id":20,"symbol":"BTCUSDT","riskLimitValue":"75000000","maintenanceMargin":"0.04","initialMargin":"0.0667","isLowestRisk":0,"maxLeverage":"15.00","mmDeduction":"1107510"},{"id":21,"symbol":"BTCUSDT","riskLimitValue":"80000000","maintenanceMargin":"0.045","initialMargin":"0.0714","isLowestRisk":0,"maxLeverage":"14.00","mmDeduction":"1482510"},{"id":22,"symbol":"BTCUSDT","riskLimitValue":"85000000","maintenanceMargin":"0.05","initialMargin":"0.0769","isLowestRisk":0,"maxLeverage":"13.00","mmDeduction":"1882510"},{"id":23,"symbol":"BTCUSDT","riskLimitValue":"90000000","maintenanceMargin":"0.055","initialMargin":"0.0833","isLowestRisk":0,"maxLeverage":"12.00","mmDeduction":"2307510"},{"id":24,"symbol":"BTCUSDT","riskLimitValue":"95000000","maintenanceMargin":"0.06","initialMargin":"0.0909","isLowestRisk":0,"maxLeverage":"11.00","mmDeduction":"2757510"},{"id":25,"symbol":"BTCUSDT","riskLimitValue":"100000000","maintenanceMargin":"0.065","initialMargin":"0.1","isLowestRisk":0,"maxLeverage":"10.00","mmDeduction":"3232510"},{"id":26,"symbol":"BTCUSDT","riskLimitValue":"105000000","maintenanceMargin":"0.07","initialMargin":"0.1111","isLowestRisk":0,"maxLeverage":"9.00","mmDeduction":"3732510"},{"id":27,"symbol":"BTCUSDT","riskLimitValue":"110000000","maintenanceMargin":"0.085","initialMargin":"0.125","isLowestRisk":0,"maxLeverage":"8.00","mmDeduction":"5307510"},{"id":28,"symbol":"BTCUSDT","riskLimitValue":"115000000","maintenanceMargin":"0.095","initialMargin":"0.1429","isLowestRisk":0,"maxLeverage":"7.00","mmDeduction":"6407510"},{"id":29,"symbol":"BTCUSDT","riskLimitValue":"120000000","maintenanceMargin":"0.1","initialMargin":"0.1667","isLowestRisk":0,"maxLeverage":"6.00","mmDeduction":"6982510"},{"id":30,"symbol":"BTCUSDT","riskLimitValue":"150000000","maintenanceMargin":"0.12","initialMargin":"0.2","isLowestRisk":0,"maxLeverage":"5.00","mmDeduction":"9382510"},{"id":31,"symbol":"BTCUSDT","riskLimitValue":"200000000","maintenanceMargin":"0.14","initialMargin":"0.25","isLowestRisk":0,"maxLeverage":"4.00","mmDeduction":"12382510"},{"id":32,"symbol":"BTCUSDT","riskLimitValue":"250000000","maintenanceMargin":"0.18","initialMargin":"0.3333","isLowestRisk":0,"maxLeverage":"3.00","mmDeduction":"20382510"},{"id":33,"symbol":"BTCUSDT","riskLimitValue":"400000000","maintenanceMargin":"0.3","initialMargin":"0.5","isLowestRisk":0,"maxLeverage":"2.00","mmDeduction":"50382510"},{"id":34,"symbol":"BTCUSDT","riskLimitValue":"800000000","maintenanceMargin":"0.42","initialMargin":"0.6993","isLowestRisk":0,"maxLeverage":"1.43","mmDeduction":"98382510"},{"id":35,"symbol":"BTCUSDT","riskLimitValue":"1200000000","maintenanceMargin":"0.6","initialMargin":"1","isLowestRisk":0,"maxLeverage":"1.00","mmDeduction":"242382510"}],"nextPageCursor":""},"retExtInfo":{},"time":1793879641949}"#;

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).expect("saved answer is JSON")
    }

    #[test]
    fn reads_a_quote() {
        let q = parse_quote(&parse(TICKERS_BTCUSDT), "BTCUSDT").unwrap();
        assert_eq!(q.symbol, "BTCUSDT");
        assert_eq!(q.last, 80370.90);
        assert_eq!(q.mark, 80369.70);
        assert_eq!(q.index, 80378.84);
        assert_eq!(q.bid, 80370.90);
        assert_eq!(q.ask, 80371.00);
        assert!(q.ask >= q.bid);
        assert_eq!(q.funding_rate, 0.0001);
        assert_eq!(q.next_funding_ms, 1_793_894_400_000);
        assert_eq!(q.at_ms, 1_793_879_980_231);
    }

    #[test]
    fn a_symbol_not_in_the_answer_is_unknown() {
        let err = parse_quote(&parse(TICKERS_BTCUSDT), "ETHUSDT").unwrap_err();
        assert!(err.contains("does not know ETHUSDT"), "{err}");
        let empty = json!({"retCode": 0, "result": {"list": []}});
        assert!(parse_quote(&empty, "BTCUSDT").unwrap_err().contains("does not know BTCUSDT"));
    }

    #[test]
    fn reads_the_saved_ticker_as_a_market_row() {
        let t = parse_tickers(&parse(TICKERS_BTCUSDT)).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].change_24h_pct, 0.024782);
        assert_eq!(t[0].turnover_24h, 5_751_546_648.4583);
        assert_eq!(t[0].volume_24h, 71_863.112);
        assert_eq!(t[0].high_24h, 81_181.10);
        assert_eq!(t[0].low_24h, 77_966.00);
    }

    #[test]
    fn reads_the_whole_market_with_a_new_coins_blanks_as_zero() {
        let all = json!({"retCode": 0, "result": {"list": [
            {"symbol": "SOLUSDT", "lastPrice": "180.5", "price24hPcnt": "-0.0312", "turnover24h": "900000000", "fundingRate": "0.0001"},
            {"symbol": "NEWUSDT", "lastPrice": "", "price24hPcnt": "", "turnover24h": "", "fundingRate": ""}
        ]}});
        let t = parse_tickers(&all).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].symbol, "SOLUSDT");
        assert_eq!(t[0].last, 180.5);
        assert_eq!(t[0].change_24h_pct, -0.0312);
        assert_eq!(t[0].turnover_24h, 900_000_000.0);
        assert_eq!(t[1].last, 0.0);
    }

    #[test]
    fn reads_both_sides_of_the_book_best_first() {
        let book = parse_book(&parse(ORDERBOOK_BTCUSDT)).unwrap();
        assert_eq!(book.bids.len(), 50);
        assert_eq!(book.asks.len(), 50);
        assert!(book.bids[0].price > book.bids[1].price);
        assert!(book.asks[0].price < book.asks[1].price);
        assert!(book.asks[0].price > book.bids[0].price);
        assert_eq!(book.bids[0], BookLevel { price: 80370.90, qty: 23.948 });
        assert_eq!(book.at_ms, 1_793_879_980_559);
    }

    #[test]
    fn the_book_keeps_at_most_fifty_levels() {
        let side: Vec<Value> = (0..60).map(|i| json!([format!("{}", 100 + i), "1"])).collect();
        let deep = json!({"retCode": 0, "result": {"a": side, "b": side, "ts": 1}});
        let book = parse_book(&deep).unwrap();
        assert_eq!(book.asks.len(), MAX_BOOK_DEPTH);
        assert_eq!(book.bids.len(), MAX_BOOK_DEPTH);
    }

    #[test]
    fn candles_come_oldest_first() {
        let c = parse_candles(&parse(KLINE_BTCUSDT)).unwrap();
        assert_eq!(c.len(), 3);
        assert!(c[0].open_ms < c[1].open_ms && c[1].open_ms < c[2].open_ms);
        assert_eq!(c[0], Candle { open_ms: 1_793_879_820_000, open: 80353.7, high: 80370.0, low: 80353.7, close: 80357.1, volume: 8.688 });
        assert!(c[2].volume > 0.0);
    }

    #[test]
    fn mark_candles_have_no_volume_and_come_oldest_first() {
        let c = parse_mark_candles(&parse(MARK_KLINE_BTCUSDT)).unwrap();
        assert_eq!(c.len(), 3);
        assert!(c[0].open_ms < c[2].open_ms);
        assert!(c.iter().all(|c| c.volume == 0.0));
        assert_eq!(c[2].close, 80370.9);
    }

    #[test]
    fn funding_comes_oldest_first() {
        let r = parse_funding_history(&parse(FUNDING_BTCUSDT)).unwrap();
        assert_eq!(r.len(), 3);
        assert!(r[0].time_ms < r[2].time_ms);
        assert_eq!(r[1], FundingRate { symbol: "BTCUSDT".into(), rate: 0.00005932, time_ms: 1_793_836_800_000 });
    }

    #[test]
    fn reads_the_instrument_rules() {
        let rules = parse_instruments(&parse(INSTRUMENT_BTCUSDT)).unwrap();
        assert_eq!(rules.len(), 1);
        let r = &rules[0];
        assert_eq!(r.symbol, "BTCUSDT");
        assert_eq!(r.tick_size, 0.10);
        assert_eq!(r.qty_step, 0.001);
        assert_eq!(r.min_qty, 0.001);
        assert_eq!(r.max_qty, 1500.0);
        assert_eq!(r.max_market_qty, Some(150.0));
        assert_eq!(r.min_notional, Some(5.0));
        assert_eq!(r.min_leverage, 1.0);
        assert_eq!(r.max_leverage, 150.0);
        assert_eq!(r.leverage_step, 0.01);
        assert_eq!(r.funding_interval_minutes, 480);
        assert_eq!(next_cursor(&parse(INSTRUMENT_BTCUSDT)), None);
    }

    #[test]
    fn lists_only_trading_usdt_perpetuals_and_reads_the_cursor() {
        let mut row = parse(INSTRUMENT_BTCUSDT)["result"]["list"][0].clone();
        let mut page = vec![];
        for (symbol, status, kind, settle) in [
            ("SOLUSDT", "Trading", "LinearPerpetual", "USDT"),
            ("BTCUSDC", "Trading", "LinearPerpetual", "USDC"),
            ("BTC-26DEC", "Trading", "LinearFutures", "USDT"),
            ("OLDUSDT", "Closed", "LinearPerpetual", "USDT"),
        ] {
            row["symbol"] = json!(symbol);
            row["status"] = json!(status);
            row["contractType"] = json!(kind);
            row["settleCoin"] = json!(settle);
            page.push(row.clone());
        }
        let answer = json!({"retCode": 0, "result": {"nextPageCursor": "next", "list": page}});
        let rules = parse_instruments(&answer).unwrap();
        assert_eq!(rules.iter().map(|r| r.symbol.as_str()).collect::<Vec<_>>(), ["SOLUSDT"]);
        assert_eq!(next_cursor(&answer).as_deref(), Some("next"));
    }

    #[test]
    fn reads_the_risk_table_lowest_first_with_a_blank_deduction_as_zero() {
        let tiers = parse_risk_tiers(&parse(RISK_LIMIT_BTCUSDT)).unwrap();
        assert_eq!(tiers.len(), 35);
        assert_eq!(tiers[0].id, 1);
        assert_eq!(tiers[0].risk_limit_value, 300_000.0);
        assert_eq!(tiers[0].maintenance_margin_rate, 0.0033);
        assert_eq!(tiers[0].initial_margin_rate, 0.0066);
        assert_eq!(tiers[0].max_leverage, 150.0);
        assert_eq!(tiers[0].maintenance_deduction, 0.0);
        assert_eq!(tiers[1].maintenance_deduction, 510.0);
        assert_eq!(tiers[34].risk_limit_value, 1_200_000_000.0);
        assert!(tiers.windows(2).all(|w| w[0].risk_limit_value <= w[1].risk_limit_value));
    }

    #[test]
    fn an_empty_risk_table_is_an_error() {
        let empty = json!({"retCode": 0, "result": {"list": []}});
        assert!(parse_risk_tiers(&empty).unwrap_err().contains("no risk table"));
    }

    #[test]
    fn a_refusal_says_what_bybit_said() {
        let answer = parse(INSTRUMENT_UNKNOWN);
        let err = parse_instruments(&answer).unwrap_err();
        assert!(err.contains("symbol invalid"), "{err}");
        assert!(parse_quote(&answer, "NOPEUSDT").unwrap_err().contains("symbol invalid"));
        assert!(parse_book(&answer).is_err());
        assert!(parse_candles(&answer).is_err());
        assert!(parse_funding_history(&answer).is_err());
        assert!(parse_risk_tiers(&answer).is_err());
    }

    #[test]
    fn string_numbers_are_parsed_and_junk_is_refused() {
        let row = json!({"a": "80370.90", "b": " 0.0001 ", "c": 5, "d": "abc", "e": "NaN", "f": ""});
        assert_eq!(num(&row, "a").unwrap(), 80370.90);
        assert_eq!(num(&row, "b").unwrap(), 0.0001);
        assert_eq!(num(&row, "c").unwrap(), 5.0);
        assert!(num(&row, "d").unwrap_err().contains("unreadable d"));
        assert!(num(&row, "e").is_err());
        assert!(num(&row, "f").is_err());
        assert_eq!(num_or_zero(&row, "f").unwrap(), 0.0);
        assert_eq!(opt_num(&row, "missing").unwrap(), None);
    }

    #[test]
    fn a_missing_field_is_an_error_not_a_panic() {
        let mut q = parse(TICKERS_BTCUSDT);
        q["result"]["list"][0].as_object_mut().unwrap().remove("markPrice");
        assert!(parse_quote(&q, "BTCUSDT").unwrap_err().contains("markPrice"));

        let mut i = parse(INSTRUMENT_BTCUSDT);
        i["result"]["list"][0]["priceFilter"].as_object_mut().unwrap().remove("tickSize");
        assert!(parse_instruments(&i).unwrap_err().contains("tickSize"));

        let short = json!({"retCode": 0, "result": {"list": [["1793879940000", "1", "2"]]}});
        assert!(parse_candles(&short).unwrap_err().contains("too few"));
        let no_volume = json!({"retCode": 0, "result": {"list": [["1793879940000", "1", "2", "0.5", "1.5"]]}});
        assert!(parse_candles(&no_volume).is_err());
        assert!(parse_mark_candles(&no_volume).is_ok());

        let half_level = json!({"retCode": 0, "result": {"a": [["1"]], "b": []}});
        assert!(parse_book(&half_level).is_err());

        assert!(parse_tickers(&json!({})).unwrap_err().contains("retCode"));
        assert!(parse_tickers(&json!({"retCode": 0})).unwrap_err().contains("no result"));
        assert!(parse_funding_history(&json!({"retCode": 0, "result": {"list": [{"symbol": "BTCUSDT", "fundingRate": "0.0001"}]}})).is_err());
    }

    #[test]
    fn interval_names_match_bybit() {
        assert_eq!(interval_name("1m"), Some("1"));
        assert_eq!(interval_name("5m"), Some("5"));
        assert_eq!(interval_name("15m"), Some("15"));
        assert_eq!(interval_name("1H"), Some("60"));
        assert_eq!(interval_name("4H"), Some("240"));
        assert_eq!(interval_name("1D"), Some("D"));
        assert_eq!(interval_name("1M"), None);
        assert_eq!(interval_name("2H"), None);
    }

    #[test]
    fn quotes_serialize_in_camel_case() {
        let q = parse_quote(&parse(TICKERS_BTCUSDT), "BTCUSDT").unwrap();
        let v = serde_json::to_value(&q).unwrap();
        assert!(v.get("fundingRate").is_some() && v.get("nextFundingMs").is_some() && v.get("atMs").is_some());
    }

    /// Reads the real Bybit API. Run with `cargo test --lib market::feed -- --ignored`.
    #[test]
    #[ignore]
    fn live_btcusdt_quote() {
        let q = BybitFeed::new().unwrap().quote("BTCUSDT").unwrap();
        assert_eq!(q.symbol, "BTCUSDT");
        assert!(q.last > 0.0 && q.mark > 0.0 && q.ask >= q.bid);
    }
}
