//! The MARKET room's commands. Each runs off the main thread, opens the companion's database and goes
//! through the trade door; each write tells the screen (`castle://market-changed`) and the engine.

use std::path::PathBuf;

use mysql::Conn;
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::market::board::{Board, STALE};
use crate::market::engine::{self, FeedView, Viewing};
use crate::market::feed::{self, BybitFeed};
use crate::market::rules::{self, MarginMode};
use crate::market::store;
use crate::market::trade::{self, OrderInput, Preview, Prices, Source, MARKET};
use crate::settings::SettingsStore;

pub const NEEDS_DATABASE: &str = "The market needs your memory database. Set it up in Settings > Database.";
/// Rows in one page of history.
pub const PAGE_SIZE: u32 = 20;
/// The symbol the room opens on.
pub const DEFAULT_SYMBOL: &str = "BTCUSDT";

// ---------------------------------------------------------------- what the screen reads

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub start_balance: f64,
    pub balance: f64,
    pub equity: f64,
    pub available: f64,
    pub unrealised: f64,
    pub risk: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteView {
    pub symbol: String,
    pub last: f64,
    pub mark: f64,
    pub index: f64,
    pub bid: f64,
    pub ask: f64,
    pub funding_rate: f64,
    pub next_funding_ms: i64,
    pub age_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionView {
    #[serde(flatten)]
    pub position: store::Position,
    pub mark: f64,
    pub unrealised_pnl: f64,
    pub liquidation_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentView {
    pub symbol: String,
    pub margin_mode: String,
    pub leverage: f64,
    pub max_leverage: f64,
    pub tick_size: f64,
    pub qty_step: f64,
    pub min_qty: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolView {
    pub symbol: String,
    pub max_leverage: f64,
    pub tick_size: f64,
    pub qty_step: f64,
    pub min_qty: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketState {
    pub run: Option<store::Run>,
    pub account: Option<Account>,
    pub quote: Option<QuoteView>,
    pub positions: Vec<PositionView>,
    pub orders: Vec<store::Order>,
    pub feed: FeedView,
    pub setting: store::Setting,
    pub instrument: InstrumentView,
}

// ---------------------------------------------------------------- the plumbing

fn open_db(app: &AppHandle) -> Result<Conn, String> {
    let folder = app.state::<SettingsStore>().get().memory_folder.ok_or(NEEDS_DATABASE)?;
    store::open(&PathBuf::from(folder))
}

/// Tells the screen and the engine that the market changed.
pub fn changed(app: &AppHandle) {
    let _ = app.emit(engine::CHANGED_EVENT, ());
    engine::poke();
}

/// Runs `work` off the main thread with the database and the board; a write tells the screen afterwards.
async fn with_db<T, F>(app: AppHandle, write: bool, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&mut Conn, &Board) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let mut conn = open_db(&app)?;
        let board = app.state::<Board>();
        let out = work(&mut conn, board.inner());
        if write {
            changed(&app);
        }
        out
    })
    .await
    .map_err(|_| "The market could not answer.".to_string())?
}

async fn off_thread<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work).await.map_err(|_| "The market could not answer.".to_string())?
}

fn symbol_or_default(symbol: &str) -> String {
    let s = trade::clean_symbol(symbol);
    if s.is_empty() {
        DEFAULT_SYMBOL.to_string()
    } else {
        s
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

// ---------------------------------------------------------------- the state

/// Everything the room shows for one symbol.
pub fn state(conn: &mut Conn, board: &Board, symbol: &str) -> Result<MarketState, String> {
    let symbol = symbol_or_default(symbol);
    let asked = board.quote(&symbol);
    let setting = store::setting(conn, MARKET)?;
    let fees = trade::fees_of(&setting);
    let data = trade::instrument_data(conn, board, &symbol).ok();
    let (mode, leverage) = match &data {
        Some(d) => trade::choice(conn, &symbol, &d.rules)?,
        None => store::instrument(conn, MARKET, &symbol)?
            .map(|i| (trade::parse_mode(&i.margin_mode).unwrap_or(MarginMode::Cross), i.leverage))
            .unwrap_or((MarginMode::Cross, trade::DEFAULT_LEVERAGE)),
    };
    let instrument = InstrumentView {
        symbol: symbol.clone(),
        margin_mode: trade::mode_word(mode).into(),
        leverage,
        max_leverage: data.as_ref().map_or(0.0, |d| d.rules.max_leverage),
        tick_size: data.as_ref().map_or(0.0, |d| d.rules.tick_size),
        qty_step: data.as_ref().map_or(0.0, |d| d.rules.qty_step),
        min_qty: data.as_ref().map_or(0.0, |d| d.rules.min_qty),
    };
    let quote = asked.as_ref().ok().map(|q| QuoteView {
        symbol: symbol.clone(),
        last: q.last,
        mark: q.mark,
        index: q.index,
        bid: q.bid,
        ask: q.ask,
        funding_rate: q.funding_rate,
        next_funding_ms: q.next_funding_ms,
        age_ms: board.last_seen(&symbol).map_or_else(|| (now_ms() - q.at_ms).max(0), |(_, age)| age.max(0)),
    });
    let feed = if asked.is_err() { FeedView { status: "STALE".into(), note: Some(STALE.into()) } } else { engine::feed_view() };

    let run = store::open_run(conn, MARKET)?;
    let mut positions_view = Vec::new();
    let mut orders = Vec::new();
    let mut account = None;
    if let Some(run) = &run {
        let positions = store::open_positions(conn, run.id)?;
        orders = store::resting_orders(conn, run.id)?;
        let mut prices = Prices::default();
        if let (Some(d), Ok(q)) = (&data, &asked) {
            prices.instruments.insert(symbol.clone(), d.clone());
            prices.quotes.insert(symbol.clone(), trade::quote_of(q));
        }
        let held: Vec<String> = positions.iter().map(|p| p.symbol.clone()).chain(orders.iter().map(|o| o.symbol.clone())).collect();
        for s in held {
            if !prices.instruments.contains_key(&s) {
                if let Ok(d) = trade::instrument_data(conn, board, &s) {
                    prices.instruments.insert(s.clone(), d);
                }
            }
            if !prices.quotes.contains_key(&s) {
                if let Some(q) = board.quote(&s).ok().or_else(|| board.last_seen(&s).map(|(q, _)| q)) {
                    prices.quotes.insert(s.clone(), trade::quote_of(&q));
                }
            }
        }
        let snapshot = trade::snapshot_of(run.balance, &positions, &orders, &prices, fees);
        let mut unrealised = 0.0;
        for p in positions {
            let judged = trade::judged_position(&p);
            let mark = prices.quotes.get(&p.symbol).map_or(p.entry_price, |q| q.mark);
            let pnl = rules::unrealised_pnl(judged.side, judged.entry, mark, judged.qty);
            unrealised += pnl;
            let liquidation_price = rules::ending_price(&judged, &snapshot);
            positions_view.push(PositionView { position: p, mark, unrealised_pnl: pnl, liquidation_price });
        }
        account = Some(Account {
            start_balance: run.start_balance,
            balance: run.balance,
            equity: run.balance + unrealised,
            available: rules::available(&snapshot),
            unrealised,
            risk: rules::account_risk(&snapshot),
        });
    }
    Ok(MarketState { run, account, quote, positions: positions_view, orders, feed, setting, instrument })
}

/// One page of a run's history: fills, orders, ledger or funding, newest first. `page` starts at 0.
pub fn history(conn: &mut Conn, kind: &str, run_id: Option<u64>, page: u32) -> Result<serde_json::Value, String> {
    let run_id = match run_id {
        Some(id) => Some(id),
        None => store::open_run(conn, MARKET)?.map(|r| r.id),
    };
    let Some(run_id) = run_id else {
        return Ok(json!({ "rows": [], "page": page, "hasMore": false }));
    };
    let (limit, offset) = (PAGE_SIZE + 1, page.saturating_mul(PAGE_SIZE));
    let mut rows: Vec<serde_json::Value> = match kind {
        "fills" => store::fills(conn, run_id, limit, offset)?.into_iter().map(|r| json!(r)).collect(),
        "orders" => store::orders(conn, run_id, limit, offset)?.into_iter().map(|r| json!(r)).collect(),
        "ledger" => store::ledger(conn, run_id, limit, offset)?.into_iter().map(|r| json!(r)).collect(),
        "funding" => store::funding(conn, run_id, limit, offset)?.into_iter().map(|r| json!(r)).collect(),
        _ => return Err("Choose fills, orders, ledger or funding.".into()),
    };
    let more = rows.len() > PAGE_SIZE as usize;
    rows.truncate(PAGE_SIZE as usize);
    Ok(json!({ "rows": rows, "page": page, "hasMore": more }))
}

// ---------------------------------------------------------------- for the companion

/// Places an order for the companion (or anyone but a click), through the same door.
pub fn place_as(app: &AppHandle, input: &OrderInput, source: Source) -> Result<store::Order, String> {
    let mut conn = open_db(app)?;
    let board = app.state::<Board>();
    let out = trade::place(&mut conn, board.inner(), input, source);
    changed(app);
    out
}

/// Closes a position at the market for the companion, through the same door.
pub fn close_as(app: &AppHandle, position_id: u64, source: Source) -> Result<(), String> {
    let mut conn = open_db(app)?;
    let board = app.state::<Board>();
    let out = trade::close(&mut conn, board.inner(), position_id, source);
    changed(app);
    out
}

// ---------------------------------------------------------------- the commands

/// The room's state for a symbol; asking also tells the engine the room is on screen.
#[tauri::command]
pub async fn market_state(app: AppHandle, symbol: String) -> Result<MarketState, String> {
    app.state::<Viewing>().seen();
    with_db(app, false, move |conn, board| state(conn, board, &symbol)).await
}

/// Every tradable USDT perpetual (public prices, no database needed).
#[tauri::command]
pub async fn market_symbols(app: AppHandle) -> Result<Vec<SymbolView>, String> {
    off_thread(move || {
        let board = app.state::<Board>();
        Ok(board
            .symbols()?
            .into_iter()
            .map(|r| SymbolView { symbol: r.symbol, max_leverage: r.max_leverage, tick_size: r.tick_size, qty_step: r.qty_step, min_qty: r.min_qty })
            .collect())
    })
    .await
}

/// Traded candles for the chart, oldest first (public prices, no database needed).
#[tauri::command]
pub async fn market_candles(symbol: String, interval: String, limit: u32) -> Result<Vec<feed::Candle>, String> {
    off_thread(move || {
        let name = feed::interval_name(&interval).ok_or("Choose 1m, 5m, 15m, 1H, 4H or 1D.")?;
        BybitFeed::new()?.candles(&symbol_or_default(&symbol), name, limit.clamp(1, 1000))
    })
    .await
}

/// Older candles for the chart, the page right before `before_ms`, oldest first - scrolling the
/// chart back past what it already has asks for more this way (public prices, no database needed).
#[tauri::command]
pub async fn market_candles_before(symbol: String, interval: String, limit: u32, before_ms: i64) -> Result<Vec<feed::Candle>, String> {
    off_thread(move || {
        let name = feed::interval_name(&interval).ok_or("Choose 1m, 5m, 15m, 1H, 4H or 1D.")?;
        BybitFeed::new()?.candles_before(&symbol_or_default(&symbol), name, limit.clamp(1, 1000), before_ms)
    })
    .await
}

#[tauri::command]
pub async fn market_preview(app: AppHandle, order: OrderInput) -> Result<Preview, String> {
    with_db(app, false, move |conn, board| trade::preview(conn, board, &order)).await
}

#[tauri::command]
pub async fn market_place(app: AppHandle, order: OrderInput) -> Result<store::Order, String> {
    with_db(app, true, move |conn, board| trade::place(conn, board, &order, Source::Member)).await
}

#[tauri::command]
pub async fn market_cancel(app: AppHandle, order_id: u64) -> Result<(), String> {
    with_db(app, true, move |conn, _| trade::cancel(conn, order_id)).await
}

#[tauri::command]
pub async fn market_close(app: AppHandle, position_id: u64) -> Result<(), String> {
    with_db(app, true, move |conn, board| trade::close(conn, board, position_id, Source::Member)).await
}

#[tauri::command]
pub async fn market_exits(app: AppHandle, position_id: u64, tp: Option<f64>, sl: Option<f64>) -> Result<(), String> {
    with_db(app, true, move |conn, board| trade::set_exits(conn, board, position_id, tp, sl)).await
}

#[tauri::command]
pub async fn market_margin(app: AppHandle, position_id: u64, amount: f64) -> Result<(), String> {
    with_db(app, true, move |conn, board| trade::add_margin(conn, board, position_id, amount)).await
}

#[tauri::command]
pub async fn market_leverage(app: AppHandle, symbol: String, margin_mode: String, leverage: f64) -> Result<(), String> {
    with_db(app, true, move |conn, board| trade::set_leverage(conn, board, &symbol, &margin_mode, leverage)).await
}

#[tauri::command]
pub async fn market_new_run(app: AppHandle, start_balance: f64) -> Result<store::Run, String> {
    with_db(app, true, move |conn, _| trade::new_run(conn, start_balance)).await
}

#[tauri::command]
pub async fn market_close_run(app: AppHandle) -> Result<(), String> {
    with_db(app, true, move |conn, board| trade::close_run(conn, board)).await
}

#[tauri::command]
pub async fn market_runs(app: AppHandle) -> Result<Vec<store::Run>, String> {
    with_db(app, false, move |conn, _| store::runs(conn, MARKET)).await
}

#[tauri::command]
pub async fn market_history(app: AppHandle, kind: String, run_id: Option<u64>, page: u32) -> Result<serde_json::Value, String> {
    with_db(app, false, move |conn, _| history(conn, &kind, run_id, page)).await
}

#[tauri::command]
pub async fn market_fees(app: AppHandle, taker: f64, maker: f64) -> Result<(), String> {
    trade::check_fees(taker, maker)?;
    with_db(app, true, move |conn, _| trade::set_fees(conn, taker, maker)).await
}
