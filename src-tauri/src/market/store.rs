//! The one place that reads and writes the MARKET room's tables, in the companion's own database.
//! Every function takes any `Queryable` (a connection or an open transaction), so the trade door can
//! hold one run locked (`lock_run`) while it writes an order, a fill, the position and the ledger.
//!
//! Amounts are DECIMAL in the database and `f64` here; times are UTC `DATETIME`s in the database and
//! milliseconds since 1970 here.

use mysql::prelude::Queryable;
use mysql::{Conn, Row, Value};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

/// The one market so far.
pub const CRYPTO: &str = "CRYPTO";

// ---------- shapes ----------

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: u64,
    pub market: String,
    pub run_no: u32,
    pub start_balance: f64,
    pub balance: f64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: u64,
    pub run_id: u64,
    pub client_order_id: String,
    pub source: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub reduce_only: bool,
    pub margin_mode: Option<String>,
    pub leverage: Option<f64>,
    pub qty: f64,
    pub price: Option<f64>,
    pub reserved_cost: f64,
    pub tp_price: Option<f64>,
    pub sl_price: Option<f64>,
    pub status: String,
    pub note: Option<String>,
    pub created_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub id: u64,
    pub run_id: u64,
    pub symbol: String,
    pub side: String,
    pub margin_mode: Option<String>,
    pub leverage: Option<f64>,
    pub qty: f64,
    pub entry_price: f64,
    pub extra_margin: f64,
    pub tp_price: Option<f64>,
    pub sl_price: Option<f64>,
    pub realised_pnl: f64,
    pub fees_paid: f64,
    pub funding_paid: f64,
    pub opened_by: String,
    pub opened_at: i64,
    pub closed_at: Option<i64>,
}

/// A position as it is first written.
#[derive(Debug, Clone)]
pub struct NewPosition<'a> {
    pub run_id: u64,
    pub symbol: &'a str,
    pub side: &'a str,
    pub margin_mode: &'a str,
    pub leverage: f64,
    pub qty: f64,
    pub entry_price: f64,
    pub tp_price: Option<f64>,
    pub sl_price: Option<f64>,
    pub opened_by: &'a str,
}

/// An order as it is first written.
#[derive(Debug, Clone)]
pub struct NewOrder<'a> {
    pub run_id: u64,
    pub client_order_id: &'a str,
    pub source: &'a str,
    pub symbol: &'a str,
    pub side: &'a str,
    pub order_type: &'a str,
    pub reduce_only: bool,
    pub margin_mode: &'a str,
    pub leverage: f64,
    pub qty: f64,
    pub price: Option<f64>,
    pub reserved_cost: f64,
    pub tp_price: Option<f64>,
    pub sl_price: Option<f64>,
    pub status: &'a str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fill {
    pub id: u64,
    pub order_id: Option<u64>,
    pub position_id: u64,
    pub symbol: String,
    pub side: String,
    pub qty: f64,
    pub price: f64,
    pub fee: f64,
    pub fee_role: String,
    pub kind: String,
    pub realised_pnl: f64,
    pub created_at: i64,
}

/// A fill as it is first written.
#[derive(Debug, Clone)]
pub struct NewFill<'a> {
    pub run_id: u64,
    pub order_id: Option<u64>,
    pub position_id: u64,
    pub symbol: &'a str,
    pub side: &'a str,
    pub qty: f64,
    pub price: f64,
    pub fee: f64,
    pub fee_role: &'a str,
    pub kind: &'a str,
    pub realised_pnl: f64,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub mark: Option<f64>,
    pub feed_age_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerRow {
    pub id: u64,
    pub kind: String,
    pub amount: f64,
    pub balance_after: f64,
    pub symbol: Option<String>,
    pub note: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRow {
    pub id: u64,
    pub position_id: u64,
    pub symbol: String,
    pub funding_time: i64,
    pub rate: f64,
    pub mark: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub taker_rate: f64,
    pub maker_rate: f64,
    pub feed_id: String,
    pub auto_on: bool,
    pub auto_every_minutes: u32,
    pub auto_round_cap: u32,
    pub auto_daily_loss: f64,
    pub auto_daily_target: Option<f64>,
    pub auto_balance_target: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    pub symbol: String,
    pub margin_mode: String,
    pub leverage: f64,
    pub rules_json: Option<String>,
    pub tiers_json: Option<String>,
    pub refreshed_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub id: u64,
    pub run_id: u64,
    pub trigger_kind: String,
    pub cli: String,
    pub message: Option<String>,
    pub reply: Option<String>,
    pub actions_json: Option<String>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    pub id: u64,
    pub position_id: u64,
    pub symbol: String,
    pub side: String,
    pub result_pct: f64,
    pub traded_by: String,
    pub lesson: String,
    pub created_at: i64,
}

// ---------- reading values ----------

fn text(row: &Row, i: usize) -> Option<String> {
    match row.as_ref(i)? {
        Value::NULL => None,
        Value::Bytes(b) => Some(String::from_utf8_lossy(b).into_owned()),
        Value::Int(n) => Some(n.to_string()),
        Value::UInt(n) => Some(n.to_string()),
        Value::Float(n) => Some(n.to_string()),
        Value::Double(n) => Some(n.to_string()),
        Value::Date(y, mo, d, h, mi, s, _) => Some(format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")),
        Value::Time(..) => None,
    }
}

fn num(row: &Row, i: usize) -> f64 {
    opt_num(row, i).unwrap_or(0.0)
}

fn opt_num(row: &Row, i: usize) -> Option<f64> {
    text(row, i).and_then(|t| t.trim().parse().ok())
}

fn id(row: &Row, i: usize) -> u64 {
    text(row, i).and_then(|t| t.parse().ok()).unwrap_or(0)
}

fn opt_id(row: &Row, i: usize) -> Option<u64> {
    text(row, i).and_then(|t| t.parse().ok())
}

fn int(row: &Row, i: usize) -> i64 {
    text(row, i).and_then(|t| t.parse().ok()).unwrap_or(0)
}

fn s(row: &Row, i: usize) -> String {
    text(row, i).unwrap_or_default()
}

fn flag(row: &Row, i: usize) -> bool {
    text(row, i).is_some_and(|t| t != "0")
}

/// A UTC `DATETIME` ("2026-11-16 03:00:00") in milliseconds.
fn when(row: &Row, i: usize) -> i64 {
    opt_when(row, i).unwrap_or(0)
}

fn opt_when(row: &Row, i: usize) -> Option<i64> {
    text(row, i).and_then(|t| crate::quota::iso_ms(&t))
}

fn sql(what: &'static str) -> impl Fn(mysql::Error) -> String {
    move |_| format!("{what} could not be done in the database.")
}

fn opt(v: Option<f64>) -> Value {
    v.map(Value::from).unwrap_or(Value::NULL)
}

/// The id the last insert on this connection made.
fn last_id<Q: Queryable>(q: &mut Q) -> Result<u64, String> {
    let id: Option<u64> = q.query_first("SELECT LAST_INSERT_ID()").map_err(sql("Reading a new row"))?;
    Ok(id.unwrap_or(0))
}

// ---------- opening ----------

static MIGRATED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// The companion's database, with the market's tables in place (migrated once per run of Castle).
pub fn open(memory: &Path) -> Result<Conn, String> {
    let mut conn = crate::memdb::open_for(memory).map_err(|_| "The market needs your memory database. Set it up in Settings > Database.".to_string())?;
    let key = memory.to_string_lossy().into_owned();
    let done = MIGRATED.lock().unwrap().as_ref().is_some_and(|set| set.contains(&key));
    if !done {
        crate::memdb::migrate(&mut conn)?;
        MIGRATED.lock().unwrap().get_or_insert_with(HashSet::new).insert(key);
    }
    Ok(conn)
}

// ---------- runs ----------

const RUN: &str = "id, market, run_no, start_balance, balance, started_at, ended_at";

fn run_of(row: &Row) -> Run {
    Run {
        id: id(row, 0),
        market: s(row, 1),
        run_no: int(row, 2) as u32,
        start_balance: num(row, 3),
        balance: num(row, 4),
        started_at: when(row, 5),
        ended_at: opt_when(row, 6),
    }
}

/// The market's open run, if any.
pub fn open_run<Q: Queryable>(q: &mut Q, market: &str) -> Result<Option<Run>, String> {
    let row: Option<Row> = q.exec_first(format!("SELECT {RUN} FROM market_run WHERE open_market = ?"), (market,)).map_err(sql("Reading the account"))?;
    Ok(row.as_ref().map(run_of))
}

/// The open run, locked until the transaction ends: a click and a companion action never race.
pub fn lock_run<Q: Queryable>(q: &mut Q, run_id: u64) -> Result<Run, String> {
    let row: Option<Row> =
        q.exec_first(format!("SELECT {RUN} FROM market_run WHERE id = ? AND ended_at IS NULL FOR UPDATE"), (run_id,)).map_err(sql("Locking the account"))?;
    row.as_ref().map(run_of).ok_or_else(|| "This account is closed.".to_string())
}

/// Every run of the market, newest first.
pub fn runs<Q: Queryable>(q: &mut Q, market: &str) -> Result<Vec<Run>, String> {
    let rows: Vec<Row> = q.exec(format!("SELECT {RUN} FROM market_run WHERE market = ? ORDER BY run_no DESC"), (market,)).map_err(sql("Reading the accounts"))?;
    Ok(rows.iter().map(run_of).collect())
}

/// A new run with its paper money and its START line.
pub fn new_run<Q: Queryable>(q: &mut Q, market: &str, start_balance: f64) -> Result<Run, String> {
    let next: Option<u32> = q.exec_first("SELECT COALESCE(MAX(run_no), 0) + 1 FROM market_run WHERE market = ?", (market,)).map_err(sql("Numbering the account"))?;
    q.exec_drop(
        "INSERT INTO market_run (market, run_no, start_balance, balance, started_at, open_market) VALUES (?, ?, ?, ?, UTC_TIMESTAMP(), ?)",
        (market, next.unwrap_or(1), start_balance, start_balance, market),
    )
    .map_err(|_| "An account is already open.".to_string())?;
    let run_id = last_id(q)?;
    add_ledger(q, run_id, "START", start_balance, start_balance, None, Some("Paper money set"))?;
    lock_run(q, run_id)
}

/// The run's balance, written after a change.
pub fn set_balance<Q: Queryable>(q: &mut Q, run_id: u64, balance: f64) -> Result<(), String> {
    q.exec_drop("UPDATE market_run SET balance = ? WHERE id = ?", (balance, run_id)).map_err(sql("Writing the balance"))
}

/// Ends the run: it stays readable, and a new one may begin.
pub fn end_run<Q: Queryable>(q: &mut Q, run_id: u64) -> Result<(), String> {
    q.exec_drop("UPDATE market_run SET ended_at = UTC_TIMESTAMP(), open_market = NULL WHERE id = ?", (run_id,)).map_err(sql("Closing the account"))
}

// ---------- the ledger ----------

/// One change of balance: START, FEE, REALISED, FUNDING or FLOOR.
pub fn add_ledger<Q: Queryable>(q: &mut Q, run_id: u64, kind: &str, amount: f64, balance_after: f64, symbol: Option<&str>, note: Option<&str>) -> Result<(), String> {
    q.exec_drop(
        "INSERT INTO market_ledger (run_id, kind, amount, balance_after, symbol, note, created_at) VALUES (?, ?, ?, ?, ?, ?, UTC_TIMESTAMP())",
        (run_id, kind, amount, balance_after, symbol, note),
    )
    .map_err(sql("Writing the ledger"))
}

pub fn ledger<Q: Queryable>(q: &mut Q, run_id: u64, limit: u32, offset: u32) -> Result<Vec<LedgerRow>, String> {
    let rows: Vec<Row> = q
        .exec(
            "SELECT id, kind, amount, balance_after, symbol, note, created_at FROM market_ledger WHERE run_id = ? ORDER BY id DESC LIMIT ? OFFSET ?",
            (run_id, limit, offset),
        )
        .map_err(sql("Reading the ledger"))?;
    Ok(rows
        .iter()
        .map(|r| LedgerRow { id: id(r, 0), kind: s(r, 1), amount: num(r, 2), balance_after: num(r, 3), symbol: text(r, 4), note: text(r, 5), created_at: when(r, 6) })
        .collect())
}

/// The REALISED, FEE and FUNDING total of the run since `since_ms` (for the daily limits).
pub fn result_since<Q: Queryable>(q: &mut Q, run_id: u64, since_ms: i64) -> Result<f64, String> {
    let since = crate::link::utc_label((since_ms.max(0) / 1000) as u64);
    let total: Option<String> = q
        .exec_first(
            "SELECT CAST(COALESCE(SUM(amount), 0) AS CHAR) FROM market_ledger WHERE run_id = ? AND kind IN ('REALISED', 'FEE', 'FUNDING') AND created_at >= ?",
            (run_id, since.replace('T', " ").trim_end_matches('Z').to_string()),
        )
        .map_err(sql("Reading today's result"))?;
    Ok(total.and_then(|t| t.parse().ok()).unwrap_or(0.0))
}

// ---------- orders ----------

const ORDER: &str = "id, run_id, client_order_id, source, symbol, side, order_type, reduce_only, margin_mode, leverage, qty, price, reserved_cost, tp_price, sl_price, status, note, created_at, finished_at";

fn order_of(r: &Row) -> Order {
    Order {
        id: id(r, 0),
        run_id: id(r, 1),
        client_order_id: s(r, 2),
        source: s(r, 3),
        symbol: s(r, 4),
        side: s(r, 5),
        order_type: s(r, 6),
        reduce_only: flag(r, 7),
        margin_mode: text(r, 8),
        leverage: opt_num(r, 9),
        qty: num(r, 10),
        price: opt_num(r, 11),
        reserved_cost: num(r, 12),
        tp_price: opt_num(r, 13),
        sl_price: opt_num(r, 14),
        status: s(r, 15),
        note: text(r, 16),
        created_at: when(r, 17),
        finished_at: opt_when(r, 18),
    }
}

/// Writes a new order; an order with the same client id is never written twice.
pub fn add_order<Q: Queryable>(q: &mut Q, o: &NewOrder) -> Result<u64, String> {
    let params: Vec<Value> = vec![
        o.run_id.into(),
        o.client_order_id.into(),
        o.source.into(),
        o.symbol.into(),
        o.side.into(),
        o.order_type.into(),
        o.reduce_only.into(),
        o.margin_mode.into(),
        o.leverage.into(),
        o.qty.into(),
        opt(o.price),
        o.reserved_cost.into(),
        opt(o.tp_price),
        opt(o.sl_price),
        o.status.into(),
    ];
    q.exec_drop(
        "INSERT INTO market_order (run_id, client_order_id, source, symbol, side, order_type, reduce_only, margin_mode, leverage, qty, price, reserved_cost, tp_price, sl_price, status, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP())",
        params,
    )
    .map_err(|_| "This order was already placed.".to_string())?;
    Ok(last_id(q)?)
}

pub fn order_by_client<Q: Queryable>(q: &mut Q, client_order_id: &str) -> Result<Option<Order>, String> {
    let row: Option<Row> = q.exec_first(format!("SELECT {ORDER} FROM market_order WHERE client_order_id = ?"), (client_order_id,)).map_err(sql("Reading an order"))?;
    Ok(row.as_ref().map(order_of))
}

pub fn order<Q: Queryable>(q: &mut Q, order_id: u64) -> Result<Option<Order>, String> {
    let row: Option<Row> = q.exec_first(format!("SELECT {ORDER} FROM market_order WHERE id = ?"), (order_id,)).map_err(sql("Reading an order"))?;
    Ok(row.as_ref().map(order_of))
}

/// The run's resting orders (NEW), oldest first.
pub fn resting_orders<Q: Queryable>(q: &mut Q, run_id: u64) -> Result<Vec<Order>, String> {
    let rows: Vec<Row> = q.exec(format!("SELECT {ORDER} FROM market_order WHERE run_id = ? AND status = 'NEW' ORDER BY id"), (run_id,)).map_err(sql("Reading the orders"))?;
    Ok(rows.iter().map(order_of).collect())
}

pub fn orders<Q: Queryable>(q: &mut Q, run_id: u64, limit: u32, offset: u32) -> Result<Vec<Order>, String> {
    let rows: Vec<Row> =
        q.exec(format!("SELECT {ORDER} FROM market_order WHERE run_id = ? ORDER BY id DESC LIMIT ? OFFSET ?"), (run_id, limit, offset)).map_err(sql("Reading the orders"))?;
    Ok(rows.iter().map(order_of).collect())
}

/// Finishes an order: FILLED, CANCELLED or REJECTED, with a plain note.
pub fn finish_order<Q: Queryable>(q: &mut Q, order_id: u64, status: &str, note: Option<&str>) -> Result<(), String> {
    q.exec_drop("UPDATE market_order SET status = ?, note = COALESCE(?, note), finished_at = UTC_TIMESTAMP() WHERE id = ?", (status, note, order_id))
        .map_err(sql("Writing an order"))
}

// ---------- positions ----------

const POSITION: &str = "id, run_id, symbol, side, margin_mode, leverage, qty, entry_price, extra_margin, tp_price, sl_price, realised_pnl, fees_paid, funding_paid, opened_by, opened_at, closed_at";

fn position_of(r: &Row) -> Position {
    Position {
        id: id(r, 0),
        run_id: id(r, 1),
        symbol: s(r, 2),
        side: s(r, 3),
        margin_mode: text(r, 4),
        leverage: opt_num(r, 5),
        qty: num(r, 6),
        entry_price: num(r, 7),
        extra_margin: num(r, 8),
        tp_price: opt_num(r, 9),
        sl_price: opt_num(r, 10),
        realised_pnl: num(r, 11),
        fees_paid: num(r, 12),
        funding_paid: num(r, 13),
        opened_by: s(r, 14),
        opened_at: when(r, 15),
        closed_at: opt_when(r, 16),
    }
}

/// The run's open positions.
pub fn open_positions<Q: Queryable>(q: &mut Q, run_id: u64) -> Result<Vec<Position>, String> {
    let rows: Vec<Row> =
        q.exec(format!("SELECT {POSITION} FROM market_position WHERE run_id = ? AND closed_at IS NULL ORDER BY id"), (run_id,)).map_err(sql("Reading the positions"))?;
    Ok(rows.iter().map(position_of).collect())
}

pub fn position<Q: Queryable>(q: &mut Q, position_id: u64) -> Result<Option<Position>, String> {
    let row: Option<Row> = q.exec_first(format!("SELECT {POSITION} FROM market_position WHERE id = ?"), (position_id,)).map_err(sql("Reading a position"))?;
    Ok(row.as_ref().map(position_of))
}

pub fn closed_positions<Q: Queryable>(q: &mut Q, run_id: u64, limit: u32) -> Result<Vec<Position>, String> {
    let rows: Vec<Row> = q
        .exec(format!("SELECT {POSITION} FROM market_position WHERE run_id = ? AND closed_at IS NOT NULL ORDER BY closed_at DESC, id DESC LIMIT ?"), (run_id, limit))
        .map_err(sql("Reading the positions"))?;
    Ok(rows.iter().map(position_of).collect())
}

/// Opens a position (one open position per symbol and run).
pub fn add_position<Q: Queryable>(q: &mut Q, p: &NewPosition) -> Result<u64, String> {
    let params: Vec<Value> = vec![
        p.run_id.into(),
        p.symbol.into(),
        p.side.into(),
        p.margin_mode.into(),
        p.leverage.into(),
        p.qty.into(),
        p.entry_price.into(),
        opt(p.tp_price),
        opt(p.sl_price),
        p.opened_by.into(),
        p.symbol.into(),
    ];
    q.exec_drop(
        "INSERT INTO market_position (run_id, symbol, side, margin_mode, leverage, qty, entry_price, tp_price, sl_price, opened_by, opened_at, open_symbol) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP(), ?)",
        params,
    )
    .map_err(|_| "A position on this symbol is already open.".to_string())?;
    Ok(last_id(q)?)
}

/// Writes a position's size, entry, money lines and exits after a fill.
pub fn save_position<Q: Queryable>(q: &mut Q, p: &Position) -> Result<(), String> {
    let params: Vec<Value> = vec![
        p.qty.into(),
        p.entry_price.into(),
        p.extra_margin.into(),
        opt(p.tp_price),
        opt(p.sl_price),
        p.realised_pnl.into(),
        p.fees_paid.into(),
        p.funding_paid.into(),
        p.id.into(),
    ];
    q.exec_drop(
        "UPDATE market_position SET qty = ?, entry_price = ?, extra_margin = ?, tp_price = ?, sl_price = ?, realised_pnl = ?, fees_paid = ?, funding_paid = ? WHERE id = ?",
        params,
    )
    .map_err(sql("Writing a position"))
}

/// Closes a position: it stays readable, and the symbol is free for a new one.
pub fn close_position<Q: Queryable>(q: &mut Q, position_id: u64) -> Result<(), String> {
    q.exec_drop("UPDATE market_position SET closed_at = UTC_TIMESTAMP(), open_symbol = NULL, qty = 0 WHERE id = ?", (position_id,)).map_err(sql("Closing a position"))
}

// ---------- fills and funding ----------

pub fn add_fill<Q: Queryable>(q: &mut Q, f: &NewFill) -> Result<u64, String> {
    let params: Vec<Value> = vec![
        f.run_id.into(),
        f.order_id.map(Value::from).unwrap_or(Value::NULL),
        f.position_id.into(),
        f.symbol.into(),
        f.side.into(),
        f.qty.into(),
        f.price.into(),
        f.fee.into(),
        f.fee_role.into(),
        f.kind.into(),
        f.realised_pnl.into(),
        opt(f.bid),
        opt(f.ask),
        opt(f.mark),
        f.feed_age_ms.map(Value::from).unwrap_or(Value::NULL),
    ];
    q.exec_drop(
        "INSERT INTO market_fill (run_id, order_id, position_id, symbol, side, qty, price, fee, fee_role, kind, realised_pnl, bid, ask, mark, feed_age_ms, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP())",
        params,
    )
    .map_err(sql("Writing a fill"))?;
    Ok(last_id(q)?)
}

pub fn fills<Q: Queryable>(q: &mut Q, run_id: u64, limit: u32, offset: u32) -> Result<Vec<Fill>, String> {
    let rows: Vec<Row> = q
        .exec(
            "SELECT id, order_id, position_id, symbol, side, qty, price, fee, fee_role, kind, realised_pnl, created_at FROM market_fill WHERE run_id = ? ORDER BY id DESC LIMIT ? OFFSET ?",
            (run_id, limit, offset),
        )
        .map_err(sql("Reading the fills"))?;
    Ok(rows
        .iter()
        .map(|r| Fill {
            id: id(r, 0),
            order_id: opt_id(r, 1),
            position_id: id(r, 2),
            symbol: s(r, 3),
            side: s(r, 4),
            qty: num(r, 5),
            price: num(r, 6),
            fee: num(r, 7),
            fee_role: s(r, 8),
            kind: s(r, 9),
            realised_pnl: num(r, 10),
            created_at: when(r, 11),
        })
        .collect())
}

/// Writes a funding settlement once; false when this position and time were already settled.
pub fn add_funding<Q: Queryable>(q: &mut Q, run_id: u64, position_id: u64, symbol: &str, funding_time: i64, rate: f64, mark: f64, amount: f64) -> Result<bool, String> {
    q.exec_drop(
        "INSERT IGNORE INTO market_funding (run_id, position_id, symbol, funding_time, rate, mark, amount, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP())",
        (run_id, position_id, symbol, funding_time, rate, mark, amount),
    )
    .map_err(sql("Writing a funding settlement"))?;
    let changed: Option<u64> = q.query_first("SELECT ROW_COUNT()").map_err(sql("Writing a funding settlement"))?;
    Ok(changed == Some(1))
}

pub fn funding<Q: Queryable>(q: &mut Q, run_id: u64, limit: u32, offset: u32) -> Result<Vec<FundingRow>, String> {
    let rows: Vec<Row> = q
        .exec("SELECT id, position_id, symbol, funding_time, rate, mark, amount FROM market_funding WHERE run_id = ? ORDER BY id DESC LIMIT ? OFFSET ?", (run_id, limit, offset))
        .map_err(sql("Reading the funding"))?;
    Ok(rows
        .iter()
        .map(|r| FundingRow { id: id(r, 0), position_id: id(r, 1), symbol: s(r, 2), funding_time: int(r, 3), rate: num(r, 4), mark: num(r, 5), amount: num(r, 6) })
        .collect())
}

// ---------- settings, feed state, instruments ----------

pub fn setting<Q: Queryable>(q: &mut Q, market: &str) -> Result<Setting, String> {
    let row: Option<Row> = q
        .exec_first(
            "SELECT taker_rate, maker_rate, feed_id, auto_on, auto_every_minutes, auto_round_cap, auto_daily_loss, auto_daily_target, auto_balance_target FROM market_setting WHERE market = ?",
            (market,),
        )
        .map_err(sql("Reading the market's settings"))?;
    let r = row.ok_or("The market's settings are missing.")?;
    Ok(Setting {
        taker_rate: num(&r, 0),
        maker_rate: num(&r, 1),
        feed_id: s(&r, 2),
        auto_on: flag(&r, 3),
        auto_every_minutes: int(&r, 4) as u32,
        auto_round_cap: int(&r, 5) as u32,
        auto_daily_loss: num(&r, 6),
        auto_daily_target: opt_num(&r, 7),
        auto_balance_target: opt_num(&r, 8),
    })
}

pub fn save_setting<Q: Queryable>(q: &mut Q, market: &str, v: &Setting) -> Result<(), String> {
    let params: Vec<Value> = vec![
        v.taker_rate.into(),
        v.maker_rate.into(),
        v.feed_id.as_str().into(),
        v.auto_on.into(),
        v.auto_every_minutes.into(),
        v.auto_round_cap.into(),
        v.auto_daily_loss.into(),
        opt(v.auto_daily_target),
        opt(v.auto_balance_target),
        market.into(),
    ];
    q.exec_drop(
        "UPDATE market_setting SET taker_rate = ?, maker_rate = ?, feed_id = ?, auto_on = ?, auto_every_minutes = ?, auto_round_cap = ?, auto_daily_loss = ?, \
         auto_daily_target = ?, auto_balance_target = ?, updated_at = UTC_TIMESTAMP() WHERE market = ?",
        params,
    )
    .map_err(sql("Saving the market's settings"))
}

/// When the engine last judged the market (ms), for the gap replay after Castle was closed.
pub fn last_evaluated<Q: Queryable>(q: &mut Q, market: &str) -> Result<Option<i64>, String> {
    let at: Option<Option<i64>> = q.exec_first("SELECT last_evaluated_ms FROM market_feed_state WHERE market = ?", (market,)).map_err(sql("Reading the feed state"))?;
    Ok(at.flatten())
}

pub fn set_feed_state<Q: Queryable>(q: &mut Q, market: &str, status: &str, last_evaluated_ms: Option<i64>, note: Option<&str>) -> Result<(), String> {
    q.exec_drop(
        "INSERT INTO market_feed_state (market, status, status_since, last_evaluated_ms, note) VALUES (?, ?, UTC_TIMESTAMP(), ?, ?) \
         ON DUPLICATE KEY UPDATE status_since = IF(status = VALUES(status), status_since, UTC_TIMESTAMP()), status = VALUES(status), \
         last_evaluated_ms = COALESCE(VALUES(last_evaluated_ms), last_evaluated_ms), note = VALUES(note)",
        (market, status, last_evaluated_ms, note),
    )
    .map_err(sql("Writing the feed state"))
}

pub fn instrument<Q: Queryable>(q: &mut Q, market: &str, symbol: &str) -> Result<Option<Instrument>, String> {
    let row: Option<Row> = q
        .exec_first("SELECT symbol, margin_mode, leverage, rules_json, tiers_json, refreshed_at FROM market_instrument WHERE market = ? AND symbol = ?", (market, symbol))
        .map_err(sql("Reading a symbol's rules"))?;
    Ok(row.map(|r| Instrument { symbol: s(&r, 0), margin_mode: s(&r, 1), leverage: num(&r, 2), rules_json: text(&r, 3), tiers_json: text(&r, 4), refreshed_at: opt_when(&r, 5) }))
}

/// Keeps a symbol's rules and risk tiers (refreshed daily) without touching its leverage and mode.
pub fn save_instrument_rules<Q: Queryable>(q: &mut Q, market: &str, symbol: &str, rules_json: &str, tiers_json: &str) -> Result<(), String> {
    q.exec_drop(
        "INSERT INTO market_instrument (market, symbol, rules_json, tiers_json, refreshed_at) VALUES (?, ?, ?, ?, UTC_TIMESTAMP()) \
         ON DUPLICATE KEY UPDATE rules_json = VALUES(rules_json), tiers_json = VALUES(tiers_json), refreshed_at = UTC_TIMESTAMP()",
        (market, symbol, rules_json, tiers_json),
    )
    .map_err(sql("Writing a symbol's rules"))
}

/// The member's leverage and margin mode for a symbol.
pub fn save_instrument_choice<Q: Queryable>(q: &mut Q, market: &str, symbol: &str, margin_mode: &str, leverage: f64) -> Result<(), String> {
    q.exec_drop(
        "INSERT INTO market_instrument (market, symbol, margin_mode, leverage) VALUES (?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE margin_mode = VALUES(margin_mode), leverage = VALUES(leverage)",
        (market, symbol, margin_mode, leverage),
    )
    .map_err(sql("Writing a symbol's leverage"))
}

// ---------- the companion's turns and lessons ----------

/// Writes a turn the moment it starts (RUNNING); `finish_turn` fills it in.
pub fn add_turn<Q: Queryable>(q: &mut Q, run_id: u64, client_turn_id: &str, trigger_kind: &str, cli: &str, message: Option<&str>) -> Result<u64, String> {
    q.exec_drop(
        "INSERT INTO market_turn (run_id, client_turn_id, trigger_kind, cli, message, status, created_at) VALUES (?, ?, ?, ?, ?, 'RUNNING', UTC_TIMESTAMP())",
        (run_id, client_turn_id, trigger_kind, cli, message),
    )
    .map_err(sql("Writing a turn"))?;
    Ok(last_id(q)?)
}

pub fn finish_turn<Q: Queryable>(q: &mut Q, turn_id: u64, status: &str, reply: Option<&str>, desk_json: Option<&str>, actions_json: Option<&str>, duration_ms: i64) -> Result<(), String> {
    q.exec_drop(
        "UPDATE market_turn SET status = ?, reply = ?, desk_json = ?, actions_json = ?, duration_ms = ? WHERE id = ?",
        (status, reply, desk_json, actions_json, duration_ms, turn_id),
    )
    .map_err(sql("Writing a turn"))
}

const TURN: &str = "id, run_id, trigger_kind, cli, message, reply, actions_json, duration_ms, status, created_at";

fn turn_of(r: &Row) -> Turn {
    Turn {
        id: id(r, 0),
        run_id: id(r, 1),
        trigger_kind: s(r, 2),
        cli: s(r, 3),
        message: text(r, 4),
        reply: text(r, 5),
        actions_json: text(r, 6),
        duration_ms: text(r, 7).and_then(|t| t.parse().ok()),
        status: s(r, 8),
        created_at: when(r, 9),
    }
}

/// The run's turns, newest first.
pub fn turns<Q: Queryable>(q: &mut Q, run_id: u64, limit: u32, offset: u32) -> Result<Vec<Turn>, String> {
    let rows: Vec<Row> =
        q.exec(format!("SELECT {TURN} FROM market_turn WHERE run_id = ? ORDER BY id DESC LIMIT ? OFFSET ?"), (run_id, limit, offset)).map_err(sql("Reading the trade log"))?;
    Ok(rows.iter().map(turn_of).collect())
}

/// The AUTO rounds of the run since `since_ms` (for the round cap).
pub fn auto_rounds_since<Q: Queryable>(q: &mut Q, run_id: u64, since_ms: i64) -> Result<u32, String> {
    let since = crate::link::utc_label((since_ms.max(0) / 1000) as u64).replace('T', " ").trim_end_matches('Z').to_string();
    let n: Option<u32> = q
        .exec_first("SELECT COUNT(*) FROM market_turn WHERE run_id = ? AND trigger_kind = 'AUTO' AND created_at >= ?", (run_id, since))
        .map_err(sql("Counting the rounds"))?;
    Ok(n.unwrap_or(0))
}

/// Keeps one lesson per closed position (a second one for the same position is ignored).
pub fn add_lesson<Q: Queryable>(q: &mut Q, run_id: u64, position_id: u64, symbol: &str, side: &str, result_pct: f64, traded_by: &str, lesson: &str) -> Result<(), String> {
    q.exec_drop(
        "INSERT IGNORE INTO market_lesson (run_id, position_id, symbol, side, result_pct, traded_by, lesson, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP())",
        (run_id, position_id, symbol, side, result_pct, traded_by, lesson),
    )
    .map_err(sql("Keeping a lesson"))
}

/// The newest lessons first, across every run.
pub fn lessons<Q: Queryable>(q: &mut Q, limit: u32) -> Result<Vec<Lesson>, String> {
    let rows: Vec<Row> = q
        .exec("SELECT id, position_id, symbol, side, result_pct, traded_by, lesson, created_at FROM market_lesson ORDER BY id DESC LIMIT ?", (limit,))
        .map_err(sql("Reading the lessons"))?;
    Ok(rows
        .iter()
        .map(|r| Lesson { id: id(r, 0), position_id: id(r, 1), symbol: s(r, 2), side: s(r, 3), result_pct: num(r, 4), traded_by: s(r, 5), lesson: s(r, 6), created_at: when(r, 7) })
        .collect())
}

pub fn has_lesson<Q: Queryable>(q: &mut Q, position_id: u64) -> Result<bool, String> {
    let n: Option<u32> = q.exec_first("SELECT COUNT(*) FROM market_lesson WHERE position_id = ?", (position_id,)).map_err(sql("Reading the lessons"))?;
    Ok(n.unwrap_or(0) > 0)
}

pub fn delete_lesson<Q: Queryable>(q: &mut Q, lesson_id: u64) -> Result<(), String> {
    q.exec_drop("DELETE FROM market_lesson WHERE id = ?", (lesson_id,)).map_err(sql("Deleting a lesson"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs a memory database: set CASTLE_MARKET_TEST_MEMORY to a test memory folder"]
    fn live_a_run_an_order_a_position_and_a_lesson_round_trip() {
        let folder = std::path::PathBuf::from(std::env::var("CASTLE_MARKET_TEST_MEMORY").expect("a test memory folder"));
        let mut conn = open(&folder).unwrap();
        let mut tx = conn.start_transaction(mysql::TxOpts::default()).unwrap();
        if let Some(old) = open_run(&mut tx, CRYPTO).unwrap() {
            end_run(&mut tx, old.id).unwrap();
        }
        let run = new_run(&mut tx, CRYPTO, 1000.0).unwrap();
        assert_eq!(open_run(&mut tx, CRYPTO).unwrap().map(|r| r.id), Some(run.id));
        let client = format!("test-{}", run.id);
        let order = NewOrder {
            run_id: run.id, client_order_id: &client, source: "MEMBER", symbol: "BTCUSDT", side: "LONG", order_type: "MARKET", reduce_only: false,
            margin_mode: "CROSS", leverage: 10.0, qty: 0.001, price: None, reserved_cost: 0.0, tp_price: None, sl_price: Some(1.0), status: "NEW",
        };
        let order_id = add_order(&mut tx, &order).unwrap();
        assert!(add_order(&mut tx, &order).is_err(), "the same client id is never written twice");
        let pos = NewPosition { run_id: run.id, symbol: "BTCUSDT", side: "LONG", margin_mode: "CROSS", leverage: 10.0, qty: 0.001, entry_price: 60000.0, tp_price: None, sl_price: None, opened_by: "MEMBER" };
        let position_id = add_position(&mut tx, &pos).unwrap();
        assert!(add_position(&mut tx, &pos).is_err(), "one open position per symbol");
        finish_order(&mut tx, order_id, "FILLED", None).unwrap();
        close_position(&mut tx, position_id).unwrap();
        add_lesson(&mut tx, run.id, position_id, "BTCUSDT", "LONG", 1.5, "MEMBER", "A test lesson.").unwrap();
        add_lesson(&mut tx, run.id, position_id, "BTCUSDT", "LONG", 1.5, "MEMBER", "Ignored.").unwrap();
        assert!(has_lesson(&mut tx, position_id).unwrap());
        tx.rollback().unwrap();
    }
}
