//! The engine: watches prices and makes what they cause happen (liquidations, limit fills, take
//! profits, stop losses, funding). It has its own thread and ticks every two seconds, but it only works
//! while a run holds positions or resting orders, or while the room is on screen; otherwise it costs
//! nothing. Each write goes through the trade door, which locks the run and judges again from the rows.
//!
//! After Castle was closed, the minutes it did not see are replayed (at most a day) on one-minute mark
//! candles, worst price first, so a stop or a liquidation beats a profit in the same minute.
//! No memory folder, or no memory database: the engine sleeps quietly.

use std::collections::{HashMap, HashSet};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use mysql::Conn;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::market::board::{Board, STALE};
use crate::market::feed::{BybitFeed, Candle, FundingRate, Quote};
use crate::market::rules::{self, Fees};
use crate::market::store;
use crate::market::trade::{self, Prices, MARKET};
use crate::settings::SettingsStore;

/// How often the engine looks.
pub const TICK: Duration = Duration::from_secs(2);
/// What the screen and the trade door listen for.
pub const CHANGED_EVENT: &str = "castle://market-changed";
/// The room counts as on screen this long after it last asked for its state.
const ON_SCREEN_MS: i64 = 60_000;
/// With nothing open, the run is read again this often (or at once after a write).
const REFRESH_MS: i64 = 30_000;
/// How often how far the engine has looked is written down.
const SAVE_STATE_MS: i64 = 60_000;
/// A gap shorter than this is not replayed.
const GAP_MS: i64 = 60_000;
/// The longest gap replayed.
const LONGEST_REPLAY_MS: i64 = 24 * 60 * 60 * 1000;
/// One feed window of one-minute candles.
const WINDOW_MINUTES: i64 = 1000;
const MINUTE_MS: i64 = 60_000;
/// A funding time is looked at this long after it passed, and again this often until its rate is published.
const FUNDING_WAIT_MS: i64 = 5_000;
const FUNDING_RETRY_MS: i64 = 60_000;
/// After the database could not be opened, wait this long before trying again.
const REOPEN_MS: i64 = 30_000;

const AWAY_NOTE: &str = "Castle was closed for more than a day: only the last 24 hours were replayed.";

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

// ---------------------------------------------------------------- shared with the commands

/// When the room last asked for its state (ms); it is on screen while that is recent.
#[derive(Debug, Default)]
pub struct Viewing(pub AtomicI64);

impl Viewing {
    pub fn seen(&self) {
        self.0.store(now_ms(), Ordering::Relaxed);
    }

    pub fn on_screen(&self, now: i64) -> bool {
        now - self.0.load(Ordering::Relaxed) <= ON_SCREEN_MS
    }
}

static POKED: AtomicBool = AtomicBool::new(false);

/// Something was written: read the run again on the next tick.
pub fn poke() {
    POKED.store(true, Ordering::Relaxed);
}

/// How the prices are doing, for the screen.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedView {
    pub status: String,
    pub note: Option<String>,
}

static FEED: Mutex<Option<FeedView>> = Mutex::new(None);

pub fn feed_view() -> FeedView {
    FEED.lock().unwrap().clone().unwrap_or(FeedView { status: "OK".into(), note: None })
}

// ---------------------------------------------------------------- pure helpers

/// Where a replay starts and the note it leaves: never more than a day back.
pub fn replay_start(last_evaluated: i64, now: i64) -> (i64, Option<&'static str>) {
    if now - last_evaluated > LONGEST_REPLAY_MS {
        (now - LONGEST_REPLAY_MS, Some(AWAY_NOTE))
    } else {
        (last_evaluated, None)
    }
}

/// The feed windows (start, end inclusive, ms) that cover `from..=to` in one-minute candles, oldest first.
pub fn replay_windows(from: i64, to: i64) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    let mut start = from.div_euclid(MINUTE_MS) * MINUTE_MS;
    while start <= to {
        let end = (start + WINDOW_MINUTES * MINUTE_MS - 1).min(to);
        out.push((start, end));
        start = end + 1;
    }
    out
}

/// The minutes to replay, oldest first, once each: a candle that ended before `from` or starts after `to` is left out.
pub fn minutes_to_replay(candles: &[Candle], from: i64, to: i64) -> Vec<Candle> {
    let mut out: Vec<Candle> = candles.iter().filter(|c| c.open_ms + MINUTE_MS > from && c.open_ms <= to).cloned().collect();
    out.sort_by_key(|c| c.open_ms);
    out.dedup_by_key(|c| c.open_ms);
    out
}

/// A minute's two extremes, worst first: a long fears the low, a short the high.
pub fn worst_first(candle: &Candle, short_held: bool) -> [f64; 2] {
    if short_held {
        [candle.high, candle.low]
    } else {
        [candle.low, candle.high]
    }
}

/// Whether to look at a symbol's funding now: at once the first time, then a little after the funding time.
pub fn funding_look(pending: Option<i64>, not_before: i64, now: i64) -> bool {
    if now < not_before {
        return false;
    }
    pending.is_none_or(|time| now >= time + FUNDING_WAIT_MS)
}

/// The published rates a position held since `held_since` still owes, oldest first, once each.
pub fn due_rates(rates: &[FundingRate], held_since: i64, settled: &HashSet<i64>) -> Vec<(i64, f64)> {
    let mut out: Vec<(i64, f64)> = rates.iter().filter(|r| r.time_ms >= held_since && !settled.contains(&r.time_ms)).map(|r| (r.time_ms, r.rate)).collect();
    out.sort_by_key(|(t, _)| *t);
    out.dedup_by_key(|(t, _)| *t);
    out
}

// ---------------------------------------------------------------- the thread

/// Starts the engine's thread. One bad tick never stops the next one.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("market-engine".into()).spawn(move || {
        let mut engine = Engine::default();
        loop {
            std::thread::sleep(TICK);
            match std::panic::catch_unwind(AssertUnwindSafe(|| engine.tick(&app))) {
                Ok(Ok(true)) => {
                    let _ = app.emit(CHANGED_EVENT, ());
                }
                Ok(Ok(false)) => {}
                Ok(Err(_)) | Err(_) => engine.conn = None,
            }
        }
    });
}

/// The engine's own memory, touched only on its thread.
#[derive(Default)]
struct Engine {
    folder: Option<String>,
    conn: Option<Conn>,
    open_failed_at: Option<i64>,
    feed: Option<BybitFeed>,
    refreshed_at: i64,
    run_id: Option<u64>,
    balance: f64,
    fees: Option<Fees>,
    positions: Vec<store::Position>,
    orders: Vec<store::Order>,
    quotes: HashMap<String, Quote>,
    last_evaluated: Option<i64>,
    loaded_last: bool,
    state_saved_at: i64,
    status: String,
    saved_status: String,
    note: Option<String>,
    pending_funding: HashMap<String, i64>,
    funding_not_before: HashMap<String, i64>,
}

impl Engine {
    fn open(&self) -> bool {
        !self.positions.is_empty() || !self.orders.is_empty()
    }

    /// One tick. True when something changed that the screen should read again.
    fn tick(&mut self, app: &AppHandle) -> Result<bool, String> {
        let folder = app.state::<SettingsStore>().get().memory_folder;
        if folder != self.folder {
            // A different memory is a different account: start over
            *self = Engine { folder: folder.clone(), ..Engine::default() };
        }
        let Some(folder) = folder else {
            return Ok(false);
        };
        let now = now_ms();
        let viewer = app.state::<Viewing>().on_screen(now);
        let poked = POKED.swap(false, Ordering::Relaxed);
        let due = poked || viewer || self.open() || now - self.refreshed_at >= REFRESH_MS;
        if !due {
            return Ok(false);
        }
        let mut conn = match self.conn.take() {
            Some(c) => c,
            None => {
                if self.open_failed_at.is_some_and(|at| now - at < REOPEN_MS) {
                    return Ok(false);
                }
                match store::open(std::path::Path::new(&folder)) {
                    Ok(c) => {
                        self.open_failed_at = None;
                        c
                    }
                    Err(_) => {
                        self.open_failed_at = Some(now);
                        return Ok(false);
                    }
                }
            }
        };
        let board = app.state::<Board>();
        let out = self.work(&mut conn, board.inner(), now, viewer);
        if out.is_ok() {
            self.conn = Some(conn);
        }
        out
    }

    fn work(&mut self, conn: &mut Conn, board: &Board, now: i64, viewer: bool) -> Result<bool, String> {
        self.refresh(conn, now)?;
        let open = self.open();
        if !open {
            // With nothing open there is no gap to replay later: the next thing opened starts from now
            self.last_evaluated = Some(now);
        }
        if !open && !viewer {
            self.save_state(conn, now)?;
            return Ok(false);
        }
        let symbols = self.open_symbols();
        if !self.read_quotes(&symbols) {
            let changed = self.set_status("STALE", Some(STALE.to_string()));
            self.save_state(conn, now)?;
            return Ok(changed);
        }
        let mut changed = self.set_status("OK", None);
        if open {
            let Some(instruments) = self.instruments(conn, board) else {
                self.save_state(conn, now)?;
                return Ok(changed);
            };
            let (replayed, moved) = self.replay_gap(conn, board, &instruments, now)?;
            changed |= moved;
            if !replayed {
                self.save_state(conn, now)?;
                return Ok(changed);
            }
            changed |= self.evaluate(conn, board, &instruments, now)?;
            changed |= self.settle_funding(conn, &instruments, now)?;
        }
        self.last_evaluated = Some(now);
        self.save_state(conn, now)?;
        Ok(changed)
    }

    fn refresh(&mut self, conn: &mut Conn, now: i64) -> Result<(), String> {
        self.refreshed_at = now;
        self.fees = Some(trade::fees_of(&store::setting(conn, MARKET)?));
        match store::open_run(conn, MARKET)? {
            None => {
                self.run_id = None;
                self.balance = 0.0;
                self.positions.clear();
                self.orders.clear();
            }
            Some(run) => {
                if self.run_id != Some(run.id) {
                    self.pending_funding.clear();
                    self.funding_not_before.clear();
                }
                self.run_id = Some(run.id);
                self.balance = run.balance;
                self.positions = store::open_positions(conn, run.id)?;
                self.orders = store::resting_orders(conn, run.id)?;
            }
        }
        Ok(())
    }

    fn open_symbols(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for s in self.positions.iter().map(|p| &p.symbol).chain(self.orders.iter().map(|o| &o.symbol)) {
            if !out.contains(s) {
                out.push(s.clone());
            }
        }
        out
    }

    fn feed(&mut self) -> Option<&BybitFeed> {
        if self.feed.is_none() {
            self.feed = BybitFeed::new().ok();
        }
        self.feed.as_ref()
    }

    /// Asks for every symbol held. False when a fresh price did not come, so nothing is judged on an old one.
    fn read_quotes(&mut self, symbols: &[String]) -> bool {
        if symbols.is_empty() {
            return true;
        }
        let Some(feed) = self.feed() else {
            return false;
        };
        let mut read = Vec::new();
        let mut all = true;
        for s in symbols {
            match feed.quote(s) {
                Ok(q) => read.push(q),
                Err(e) => {
                    all = false;
                    if e.contains("refused") || e.contains("slow down") {
                        break;
                    }
                }
            }
        }
        for q in read {
            self.quotes.insert(q.symbol.clone(), q);
        }
        all
    }

    /// The rules of every symbol held; None when one cannot be read (nothing is judged without them).
    fn instruments(&self, conn: &mut Conn, board: &Board) -> Option<HashMap<String, rules::Instrument>> {
        let mut out = HashMap::new();
        for s in self.open_symbols() {
            out.insert(s.clone(), trade::instrument_data(conn, board, &s).ok()?);
        }
        Some(out)
    }

    fn prices(&self, instruments: &HashMap<String, rules::Instrument>) -> Prices {
        Prices {
            quotes: self.quotes.iter().filter(|(s, _)| instruments.contains_key(*s)).map(|(s, q)| (s.clone(), trade::quote_of(q))).collect(),
            instruments: instruments.clone(),
        }
    }

    fn fees(&self) -> Fees {
        self.fees.unwrap_or(Fees { taker: 0.00055, maker: 0.0002 })
    }

    fn judge(&self, prices: &Prices) -> bool {
        let snapshot = trade::snapshot_of(self.balance, &self.positions, &self.orders, prices, self.fees());
        !rules::triggers(&snapshot).is_empty()
    }

    /// Compares this tick's prices with what is open. Nothing fires: nothing is written.
    fn evaluate(&mut self, conn: &mut Conn, board: &Board, instruments: &HashMap<String, rules::Instrument>, now: i64) -> Result<bool, String> {
        let prices = self.prices(instruments);
        if !self.judge(&prices) {
            return Ok(false);
        }
        let changed = trade::apply_triggers(conn, board, &prices, false)?;
        self.refresh(conn, now)?;
        Ok(changed)
    }

    /// Walks the minutes the engine did not see. (done, changed): not done when the feed could not answer,
    /// so the gap is tried again rather than forgotten.
    fn replay_gap(&mut self, conn: &mut Conn, board: &Board, instruments: &HashMap<String, rules::Instrument>, now: i64) -> Result<(bool, bool), String> {
        if !self.loaded_last {
            if let Some(saved) = store::last_evaluated(conn, MARKET)? {
                self.last_evaluated = Some(saved);
            }
            self.loaded_last = true;
        }
        let Some(last) = self.last_evaluated else {
            return Ok((true, false));
        };
        if now - last <= GAP_MS {
            return Ok((true, false));
        }
        let (from, note) = replay_start(last, now);
        let mut changed = false;
        for symbol in self.open_symbols() {
            let start = from.max(self.opened_since(&symbol));
            if start >= now {
                continue;
            }
            for (a, b) in replay_windows(start, now) {
                let Some(feed) = self.feed() else {
                    return Ok((false, changed));
                };
                let Ok(candles) = feed.mark_candles(&symbol, "1", a, b) else {
                    return Ok((false, changed));
                };
                for candle in minutes_to_replay(&candles, start, now) {
                    changed |= self.replay_minute(conn, board, instruments, &symbol, &candle, now)?;
                }
            }
        }
        if let Some(note) = note {
            self.note = Some(note.to_string());
            self.state_saved_at = 0;
            *FEED.lock().unwrap() = Some(FeedView { status: self.status_word().into(), note: self.note.clone() });
            changed = true;
        }
        Ok((true, changed))
    }

    /// A position or an order is never judged on prices from before it existed.
    fn opened_since(&self, symbol: &str) -> i64 {
        let positions = self.positions.iter().filter(|p| p.symbol == symbol).map(|p| p.opened_at);
        let orders = self.orders.iter().filter(|o| o.symbol == symbol).map(|o| o.created_at);
        positions.chain(orders).min().unwrap_or(i64::MAX)
    }

    fn replay_minute(&mut self, conn: &mut Conn, board: &Board, instruments: &HashMap<String, rules::Instrument>, symbol: &str, candle: &Candle, now: i64) -> Result<bool, String> {
        let short_held = self.positions.iter().any(|p| p.symbol == symbol && p.side == "SHORT");
        let mut changed = false;
        for price in worst_first(candle, short_held) {
            if !self.open() {
                break;
            }
            let mut prices = self.prices(instruments);
            prices.quotes.insert(symbol.to_string(), rules::Quote { last: price, mark: price, bid: price, ask: price });
            if self.judge(&prices) {
                changed |= trade::apply_triggers(conn, board, &prices, true)?;
                self.refresh(conn, now)?;
            }
        }
        Ok(changed)
    }

    /// Settles funding a little after each funding time; on the first look after a start, whatever was missed.
    fn settle_funding(&mut self, conn: &mut Conn, instruments: &HashMap<String, rules::Instrument>, now: i64) -> Result<bool, String> {
        let prices = self.prices(instruments);
        let mut changed = false;
        for position in self.positions.clone() {
            let Some(quote) = self.quotes.get(&position.symbol).cloned() else {
                continue;
            };
            if quote.next_funding_ms <= 0 {
                continue;
            }
            let symbol = position.symbol.clone();
            let pending = self.pending_funding.get(&symbol).copied();
            let not_before = self.funding_not_before.get(&symbol).copied().unwrap_or(0);
            if !funding_look(pending, not_before, now) {
                continue;
            }
            match self.settle_symbol(conn, &position, now, &quote, &prices) {
                Ok((latest, settled)) => {
                    changed |= settled;
                    if pending.is_none_or(|p| latest >= p) {
                        self.pending_funding.insert(symbol, quote.next_funding_ms);
                    } else {
                        // The feed has not published that rate yet
                        self.funding_not_before.insert(symbol, now + FUNDING_RETRY_MS);
                    }
                }
                Err(_) => {
                    self.funding_not_before.insert(symbol, now + FUNDING_RETRY_MS);
                }
            }
        }
        if changed {
            self.refresh(conn, now)?;
        }
        Ok(changed)
    }

    /// Settles every published rate this position has not paid yet. (latest published time, settled any).
    fn settle_symbol(&mut self, conn: &mut Conn, position: &store::Position, now: i64, quote: &Quote, prices: &Prices) -> Result<(i64, bool), String> {
        let run_id = self.run_id.ok_or("No account is open.")?;
        let settled: HashSet<i64> = store::funding(conn, run_id, 1000, 0)?.into_iter().filter(|f| f.position_id == position.id).map(|f| f.funding_time).collect();
        let since = settled.iter().max().map_or(position.opened_at, |t| (t + 1).max(position.opened_at));
        let feed = self.feed().ok_or(STALE)?;
        let rates = feed.funding_history(&position.symbol, since, now)?;
        let latest = rates.iter().map(|r| r.time_ms).max().unwrap_or(0);
        let mut any = false;
        for (time, rate) in due_rates(&rates, position.opened_at, &settled) {
            let mark = feed
                .mark_candles(&position.symbol, "1", time, time + MINUTE_MS)
                .ok()
                .and_then(|c| c.into_iter().find(|c| c.open_ms >= time))
                .map_or(quote.mark, |c| c.open);
            any |= trade::settle_funding(conn, position.id, time, rate, mark, prices)?;
        }
        Ok((latest, any))
    }

    fn status_word(&self) -> &str {
        if self.status.is_empty() {
            "OK"
        } else {
            &self.status
        }
    }

    /// Changes the feed status; true when it changed. The note stays until the status changes.
    fn set_status(&mut self, status: &str, note: Option<String>) -> bool {
        if self.status_word() == status && !self.status.is_empty() {
            return false;
        }
        let was = self.status_word().to_string();
        self.status = status.to_string();
        self.note = note;
        *FEED.lock().unwrap() = Some(FeedView { status: self.status.clone(), note: self.note.clone() });
        was != status
    }

    /// Keeps how far the engine has looked and how the feed is doing: once a minute, or when the status changes.
    fn save_state(&mut self, conn: &mut Conn, now: i64) -> Result<(), String> {
        let status = self.status_word().to_string();
        if status == self.saved_status && now - self.state_saved_at < SAVE_STATE_MS {
            return Ok(());
        }
        store::set_feed_state(conn, MARKET, &status, self.last_evaluated, self.note.as_deref())?;
        self.saved_status = status;
        self.state_saved_at = now;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(open_ms: i64, low: f64, high: f64) -> Candle {
        Candle { open_ms, open: low, high, low, close: high, volume: 0.0 }
    }

    #[test]
    fn a_replay_goes_back_a_day_at_most_and_says_so() {
        let now = 10 * LONGEST_REPLAY_MS;
        assert_eq!(replay_start(now - 5 * MINUTE_MS, now), (now - 5 * MINUTE_MS, None));
        assert_eq!(replay_start(now - 2 * LONGEST_REPLAY_MS, now), (now - LONGEST_REPLAY_MS, Some(AWAY_NOTE)));
    }

    #[test]
    fn replay_windows_cover_the_gap_oldest_first_without_overlap() {
        let from = 1_000 * MINUTE_MS + 30_000;
        let to = from + 2_500 * MINUTE_MS;
        let windows = replay_windows(from, to);
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].0, 1_000 * MINUTE_MS, "starts on the minute");
        for pair in windows.windows(2) {
            assert_eq!(pair[1].0, pair[0].1 + 1);
        }
        assert_eq!(windows.last().unwrap().1, to);
        assert!(windows.iter().all(|(a, b)| b - a < WINDOW_MINUTES * MINUTE_MS));
    }

    #[test]
    fn replayed_minutes_come_oldest_first_once_each_and_inside_the_gap() {
        let from = 10 * MINUTE_MS + 20_000;
        let to = 14 * MINUTE_MS;
        let candles = vec![
            candle(13 * MINUTE_MS, 1.0, 2.0),
            candle(9 * MINUTE_MS, 1.0, 2.0),
            candle(10 * MINUTE_MS, 1.0, 2.0),
            candle(11 * MINUTE_MS, 1.0, 2.0),
            candle(11 * MINUTE_MS, 1.0, 2.0),
            candle(15 * MINUTE_MS, 1.0, 2.0),
        ];
        let minutes: Vec<i64> = minutes_to_replay(&candles, from, to).iter().map(|c| c.open_ms / MINUTE_MS).collect();
        assert_eq!(minutes, [10, 11, 13]);
    }

    #[test]
    fn the_worst_price_of_a_minute_comes_first() {
        let c = candle(0, 90.0, 110.0);
        assert_eq!(worst_first(&c, false), [90.0, 110.0], "a long sees the low first");
        assert_eq!(worst_first(&c, true), [110.0, 90.0], "a short sees the high first");
    }

    #[test]
    fn funding_is_looked_at_once_at_start_then_after_each_funding_time() {
        assert!(funding_look(None, 0, 1_000));
        assert!(!funding_look(Some(10_000), 0, 10_000 + FUNDING_WAIT_MS - 1));
        assert!(funding_look(Some(10_000), 0, 10_000 + FUNDING_WAIT_MS));
        assert!(!funding_look(None, 5_000, 4_999), "a retry waits its turn");
    }

    #[test]
    fn a_funding_time_is_owed_once_and_only_while_held() {
        let rate = |t: i64, r: f64| FundingRate { symbol: "BTCUSDT".into(), rate: r, time_ms: t };
        let rates = vec![rate(3_000, 0.0003), rate(1_000, 0.0001), rate(2_000, 0.0002), rate(2_000, 0.0002)];
        let none = HashSet::new();
        assert_eq!(due_rates(&rates, 1_500, &none), vec![(2_000, 0.0002), (3_000, 0.0003)]);
        let settled: HashSet<i64> = [2_000].into_iter().collect();
        assert_eq!(due_rates(&rates, 0, &settled), vec![(1_000, 0.0001), (3_000, 0.0003)]);
    }

    #[test]
    fn the_room_is_on_screen_for_a_minute_after_it_asked() {
        let v = Viewing::default();
        assert!(!v.on_screen(now_ms()));
        v.seen();
        assert!(v.on_screen(now_ms()));
        assert!(!v.on_screen(now_ms() + ON_SCREEN_MS + 1_000));
    }
}
