//! The price board: the newest quote per symbol with its age, the order book for a fill, the
//! symbol rules refreshed daily, and the scanner of the busiest and fastest-moving perpetuals.
//! A quote older than `FRESH` is never traded on.

use crate::market::feed::{BybitFeed, Book, InstrumentRules, Quote, RiskTier, Ticker};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A quote older than this cannot be traded on.
pub const FRESH: Duration = Duration::from_secs(10);
/// How long the scanner's table is kept.
const SCAN_EVERY: Duration = Duration::from_secs(60);
/// How long the symbol list is kept.
const RULES_EVERY: Duration = Duration::from_secs(24 * 60 * 60);
/// Below this 24 h turnover (USDT) a symbol is too thin for the scanner.
const MIN_TURNOVER: f64 = 5_000_000.0;
/// Rows in each half of the scanner's table, and how many movers the companion may trade.
const SCAN_ROWS: usize = 15;
pub const CANDIDATES: usize = 6;
/// What is said when prices cannot be trusted.
pub const STALE: &str = "Prices are not available right now.";

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// One row of the scanner's table.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRow {
    pub symbol: String,
    pub last: f64,
    pub change_24h_pct: f64,
    pub turnover_24h: f64,
}

/// The scanner's table: the strongest movers and the busiest symbols.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    pub movers: Vec<ScanRow>,
    pub busiest: Vec<ScanRow>,
}

impl Scan {
    /// The movers the companion may trade this round.
    pub fn candidates(&self) -> Vec<String> {
        self.movers.iter().take(CANDIDATES).map(|r| r.symbol.clone()).collect()
    }
}

/// Builds the scanner's table from every ticker (pure): thin symbols left out, movers by the size
/// of their 24 h change, busiest by turnover.
pub fn scan_of(tickers: &[Ticker]) -> Scan {
    let row = |t: &Ticker| ScanRow { symbol: t.symbol.clone(), last: t.last, change_24h_pct: t.change_24h_pct, turnover_24h: t.turnover_24h };
    let liquid: Vec<&Ticker> = tickers.iter().filter(|t| t.symbol.ends_with("USDT") && t.turnover_24h >= MIN_TURNOVER && t.last > 0.0).collect();
    let mut movers = liquid.clone();
    movers.sort_by(|a, b| b.change_24h_pct.abs().partial_cmp(&a.change_24h_pct.abs()).unwrap_or(std::cmp::Ordering::Equal).then(a.symbol.cmp(&b.symbol)));
    let mut busiest = liquid;
    busiest.sort_by(|a, b| b.turnover_24h.partial_cmp(&a.turnover_24h).unwrap_or(std::cmp::Ordering::Equal).then(a.symbol.cmp(&b.symbol)));
    Scan { movers: movers.into_iter().take(SCAN_ROWS).map(row).collect(), busiest: busiest.into_iter().take(SCAN_ROWS).map(row).collect() }
}

/// True when a quote read at `at` is still fresh at `now` (both ms).
pub fn is_fresh(at_ms: i64, now_ms: i64) -> bool {
    now_ms - at_ms <= FRESH.as_millis() as i64
}

struct Kept<T> {
    at: Instant,
    value: T,
}

#[derive(Default)]
struct Inner {
    quotes: HashMap<String, (Quote, i64)>,
    scan: Option<Kept<Scan>>,
    symbols: Option<Kept<Vec<InstrumentRules>>>,
    tiers: HashMap<String, Kept<Vec<RiskTier>>>,
}

/// The board, kept for the whole run of Castle.
#[derive(Default)]
pub struct Board(Mutex<Inner>);

impl Board {
    fn feed() -> Result<BybitFeed, String> {
        BybitFeed::new()
    }

    /// A fresh quote: from the board when read within `FRESH`, else asked again.
    pub fn quote(&self, symbol: &str) -> Result<Quote, String> {
        if let Some((q, read_at)) = self.0.lock().unwrap().quotes.get(symbol) {
            if is_fresh(*read_at, now_ms()) {
                return Ok(q.clone());
            }
        }
        let q = Self::feed()?.quote(symbol).map_err(|_| STALE.to_string())?;
        self.0.lock().unwrap().quotes.insert(symbol.to_string(), (q.clone(), now_ms()));
        Ok(q)
    }

    /// The last quote seen and how old it is (ms), without asking: for the screen.
    pub fn last_seen(&self, symbol: &str) -> Option<(Quote, i64)> {
        self.0.lock().unwrap().quotes.get(symbol).map(|(q, at)| (q.clone(), now_ms() - at))
    }

    /// The order book for a market fill, always asked fresh.
    pub fn book(&self, symbol: &str) -> Result<Book, String> {
        Self::feed()?.book(symbol, crate::market::feed::MAX_BOOK_DEPTH).map_err(|_| STALE.to_string())
    }

    /// The scanner's table, kept a minute.
    pub fn scan(&self) -> Result<Scan, String> {
        if let Some(kept) = self.0.lock().unwrap().scan.as_ref() {
            if kept.at.elapsed() < SCAN_EVERY {
                return Ok(kept.value.clone());
            }
        }
        let scan = scan_of(&Self::feed()?.tickers()?);
        self.0.lock().unwrap().scan = Some(Kept { at: Instant::now(), value: scan.clone() });
        Ok(scan)
    }

    /// Every tradable USDT perpetual with its rules, kept a day.
    pub fn symbols(&self) -> Result<Vec<InstrumentRules>, String> {
        if let Some(kept) = self.0.lock().unwrap().symbols.as_ref() {
            if kept.at.elapsed() < RULES_EVERY {
                return Ok(kept.value.clone());
            }
        }
        let list = Self::feed()?.instruments()?;
        self.0.lock().unwrap().symbols = Some(Kept { at: Instant::now(), value: list.clone() });
        Ok(list)
    }

    /// One symbol's rules.
    pub fn rules(&self, symbol: &str) -> Result<InstrumentRules, String> {
        self.symbols()?.into_iter().find(|r| r.symbol == symbol).ok_or_else(|| format!("{symbol} is not a tradable perpetual."))
    }

    /// One symbol's risk tiers, kept a day.
    pub fn tiers(&self, symbol: &str) -> Result<Vec<RiskTier>, String> {
        if let Some(kept) = self.0.lock().unwrap().tiers.get(symbol) {
            if kept.at.elapsed() < RULES_EVERY {
                return Ok(kept.value.clone());
            }
        }
        let tiers = Self::feed()?.risk_tiers(symbol)?;
        self.0.lock().unwrap().tiers.insert(symbol.to_string(), Kept { at: Instant::now(), value: tiers.clone() });
        Ok(tiers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticker(symbol: &str, change: f64, turnover: f64) -> Ticker {
        Ticker { symbol: symbol.into(), last: 1.0, change_24h_pct: change, turnover_24h: turnover, volume_24h: 0.0, high_24h: 1.0, low_24h: 1.0, funding_rate: 0.0 }
    }

    #[test]
    fn the_scanner_leaves_thin_symbols_out_and_ranks_by_move_and_turnover() {
        let all = vec![
            ticker("BTCUSDT", 0.01, 9e9),
            ticker("PEPEUSDT", 0.30, 6e6),
            ticker("THINUSDT", 0.90, 1e5),
            ticker("SOLUSDT", -0.12, 8e8),
            ticker("BTCUSDC", 0.50, 9e9),
        ];
        let scan = scan_of(&all);
        assert_eq!(scan.movers.iter().map(|r| r.symbol.as_str()).collect::<Vec<_>>(), ["PEPEUSDT", "SOLUSDT", "BTCUSDT"]);
        assert_eq!(scan.busiest.iter().map(|r| r.symbol.as_str()).collect::<Vec<_>>(), ["BTCUSDT", "SOLUSDT", "PEPEUSDT"]);
        assert_eq!(scan.candidates(), ["PEPEUSDT", "SOLUSDT", "BTCUSDT"]);
    }

    #[test]
    fn a_quote_is_tradable_for_ten_seconds() {
        assert!(is_fresh(1_000, 11_000));
        assert!(!is_fresh(1_000, 11_001));
    }
}
