//! The one door every paper order comes through: a member's click, the companion's ask and the
//! companion's own round all land here, and so does the engine when prices make something happen.
//!
//! Prices, rules and the order book are read first, with no lock held. Then the run's row is locked
//! (`store::lock_run`) and only under that lock is the balance read, the decision made and the rows
//! written, in one transaction: a click and the engine never act on the same balance at once. Every
//! change of the balance leaves a ledger line (START, FEE, REALISED, FUNDING, FLOOR). Paper money
//! only: nothing here can reach an exchange account.

use mysql::prelude::Queryable;
use mysql::{Conn, TxOpts};

use crate::market::board::{Board, STALE};
use crate::market::feed;
use crate::market::rules::{self, FeeRole, Fees, MarginMode, OrderType, Side, Trigger, TriggerKind, EPS};
use crate::market::store;
pub use crate::market::trade_plan::*;

/// The one market so far.
pub const MARKET: &str = store::CRYPTO;
/// Leverage and mode for a symbol the member has not set yet.
pub const DEFAULT_LEVERAGE: f64 = 10.0;
/// The most paper money a run may start with.
pub const LARGEST_START: f64 = 1_000_000_000.0;
/// A stop-out can leave a few cents: under this, with nothing open, an account cannot trade on.
pub const NOTHING_LEFT_UNDER: f64 = 1.0;
/// The highest fee rate the member may set (1 %).
pub const TOP_FEE_RATE: f64 = 0.01;

const NO_RUN: &str = "Set your paper money first.";
const NO_POSITION: &str = "That position no longer exists.";
const POSITION_CLOSED: &str = "That position is already closed.";

// ---------------------------------------------------------------- reading before the lock

/// One symbol's rules and risk table: from the board (kept a day), written to the database once a day
/// so a restart without the feed still knows them; read back from there when the feed cannot answer.
pub fn instrument_data<Q: Queryable>(q: &mut Q, board: &Board, symbol: &str) -> Result<rules::Instrument, String> {
    let fresh = board.rules(symbol).and_then(|r| board.tiers(symbol).map(|t| (r, t)));
    match fresh {
        Ok((r, t)) => {
            let data = rules::Instrument { rules: rules_of(&r), tiers: tiers_of(&t) };
            let kept = store::instrument(q, MARKET, symbol).ok().flatten();
            let old = kept.as_ref().and_then(|k| k.refreshed_at).is_none_or(|at| now_ms() - at > 24 * 60 * 60 * 1000);
            if old {
                if let (Ok(rj), Ok(tj)) = (serde_json::to_string(&data.rules), serde_json::to_string(&data.tiers)) {
                    let _ = store::save_instrument_rules(q, MARKET, symbol, &rj, &tj);
                }
            }
            Ok(data)
        }
        Err(e) => {
            let kept = store::instrument(q, MARKET, symbol)?;
            let parsed = kept.and_then(|k| {
                let r: rules::Rules = serde_json::from_str(k.rules_json.as_deref()?).ok()?;
                let t: Vec<rules::Tier> = serde_json::from_str(k.tiers_json.as_deref()?).ok()?;
                Some(rules::Instrument { rules: r, tiers: t })
            });
            parsed.ok_or_else(|| format!("The rules of {symbol} could not be read: {e}"))
        }
    }
}

/// The member's mode and leverage for a symbol; cross 10x (or the symbol's top, if lower) until set.
pub fn choice<Q: Queryable>(q: &mut Q, symbol: &str, rules: &rules::Rules) -> Result<(MarginMode, f64), String> {
    Ok(match store::instrument(q, MARKET, symbol)? {
        Some(row) if row.leverage > 0.0 => (mode_of(Some(&row.margin_mode)), row.leverage),
        _ => (MarginMode::Cross, DEFAULT_LEVERAGE.min(rules.max_leverage.max(1.0))),
    })
}

/// The prices and rules for every symbol a run holds and for `symbol`; the asked symbol's quote is fresh.
fn prices_for(conn: &mut Conn, board: &Board, run_id: Option<u64>, symbol: Option<&str>) -> Result<(Prices, Option<feed::Quote>), String> {
    let mut symbols: Vec<String> = Vec::new();
    if let Some(s) = symbol {
        symbols.push(s.to_string());
    }
    if let Some(run_id) = run_id {
        for p in store::open_positions(conn, run_id)? {
            symbols.push(p.symbol);
        }
        for o in store::resting_orders(conn, run_id)? {
            symbols.push(o.symbol);
        }
    }
    symbols.dedup();
    let mut prices = Prices::default();
    let mut asked = None;
    for s in &symbols {
        if prices.instruments.contains_key(s) {
            continue;
        }
        prices.instruments.insert(s.clone(), instrument_data(conn, board, s)?);
        if Some(s.as_str()) == symbol {
            let q = board.quote(s).map_err(|_| STALE.to_string())?;
            prices.quotes.insert(s.clone(), quote_of(&q));
            asked = Some(q);
        } else if let Some(q) = board.quote(s).ok().or_else(|| board.last_seen(s).map(|(q, _)| q)) {
            prices.quotes.insert(s.clone(), quote_of(&q));
        }
    }
    Ok((prices, asked))
}

// ---------------------------------------------------------------- writing under the lock

/// The run's balance while a transaction changes it.
struct Wallet {
    run_id: u64,
    balance: f64,
}

/// What the market looked like as a fill happened; kept on the fill row for a later review.
#[derive(Debug, Clone, Copy, Default)]
struct Journal {
    bid: Option<f64>,
    ask: Option<f64>,
    mark: Option<f64>,
    feed_age_ms: Option<i64>,
}

impl Journal {
    fn of(q: Option<&rules::Quote>, age: Option<i64>) -> Journal {
        Journal { bid: q.map(|q| q.bid), ask: q.map(|q| q.ask), mark: q.map(|q| q.mark), feed_age_ms: age }
    }
}

/// One change of the balance and its ledger line, always together.
fn post<Q: Queryable>(q: &mut Q, w: &mut Wallet, kind: &str, amount: f64, symbol: Option<&str>, note: Option<&str>) -> Result<(), String> {
    w.balance = money(w.balance + amount);
    store::add_ledger(q, w.run_id, kind, money(amount), w.balance, symbol, note)
}

/// A wallet stops at zero; the difference is written down, not hidden.
fn floor_at_zero<Q: Queryable>(q: &mut Q, w: &mut Wallet, symbol: Option<&str>) -> Result<(), String> {
    if let Some(amount) = floor_line(w.balance) {
        post(q, w, "FLOOR", amount, symbol, Some("The wallet stops at zero"))?;
    }
    Ok(())
}

fn open_position_on<Q: Queryable>(q: &mut Q, run_id: u64, symbol: &str) -> Result<Option<store::Position>, String> {
    Ok(store::open_positions(q, run_id)?.into_iter().find(|p| p.symbol == symbol))
}

/// How one fill is written.
struct FillWrite<'a> {
    order_id: Option<u64>,
    symbol: &'a str,
    side: Side,
    qty: f64,
    price: f64,
    role: FeeRole,
    kind: &'a str,
    mode: MarginMode,
    leverage: f64,
    tp: Option<f64>,
    sl: Option<f64>,
    opened_by: &'a str,
    reduce_only: bool,
    journal: Journal,
}

/// One fill: closes what it is against, opens or adds what is left. A flip reads as two honest fill rows.
fn apply_fill<Q: Queryable>(q: &mut Q, w: &mut Wallet, f: &FillWrite, fees: Fees) -> Result<(), String> {
    let rate = match f.role {
        FeeRole::Maker => fees.maker,
        FeeRole::Taker => fees.taker,
    };
    let mut position = open_position_on(q, w.run_id, f.symbol)?;
    let plan = plan_fill(position.as_ref().map(held_of), f.side, f.qty, f.price, rate, f.reduce_only);
    if plan.closing > EPS {
        if let Some(p) = position.as_mut() {
            close_part(q, w, p, f.order_id, plan.closing, f.price, f.role, f.kind, f.journal, plan.realised, plan.close_fee)?;
            if p.closed_at.is_some() {
                position = None;
            }
        }
    }
    if plan.opening <= EPS {
        return Ok(());
    }
    let held = plan.after.unwrap_or(Held { side: f.side, qty: plan.opening, entry: f.price });
    let mut position = match position {
        Some(mut p) => {
            p.qty = held.qty;
            p.entry_price = held.entry;
            p
        }
        None => {
            let id = store::add_position(
                q,
                &store::NewPosition {
                    run_id: w.run_id,
                    symbol: f.symbol,
                    side: side_word(f.side),
                    margin_mode: mode_word(f.mode),
                    leverage: f.leverage,
                    qty: plan.opening,
                    entry_price: f.price,
                    tp_price: f.tp,
                    sl_price: f.sl,
                    opened_by: f.opened_by,
                },
            )?;
            store::position(q, id)?.ok_or("A new position could not be read back.")?
        }
    };
    if f.tp.is_some() {
        position.tp_price = f.tp;
    }
    if f.sl.is_some() {
        position.sl_price = f.sl;
    }
    position.fees_paid = money(position.fees_paid + plan.open_fee);
    store::save_position(q, &position)?;
    post(q, w, "FEE", -plan.open_fee, Some(f.symbol), Some(f.kind))?;
    record_fill(q, w.run_id, f.order_id, &position, f.side, plan.opening, f.price, plan.open_fee, f.role, f.kind, 0.0, f.journal)?;
    Ok(())
}

/// Closes `qty` of a position at `price`: REALISED and FEE lines, the position's money lines, one fill row.
#[allow(clippy::too_many_arguments)]
fn close_part<Q: Queryable>(
    q: &mut Q,
    w: &mut Wallet,
    p: &mut store::Position,
    order_id: Option<u64>,
    qty: f64,
    price: f64,
    role: FeeRole,
    kind: &str,
    journal: Journal,
    realised: f64,
    fee: f64,
) -> Result<(), String> {
    post(q, w, "REALISED", realised, Some(&p.symbol), Some(kind))?;
    post(q, w, "FEE", -fee, Some(&p.symbol), Some(kind))?;
    p.realised_pnl = money(p.realised_pnl + realised);
    p.fees_paid = money(p.fees_paid + fee);
    let left = money(p.qty - qty);
    let side = parse_side(&p.side).unwrap_or(Side::Long);
    if left <= EPS {
        store::save_position(q, p)?;
        store::close_position(q, p.id)?;
        p.closed_at = Some(now_ms());
    } else {
        p.qty = left;
        store::save_position(q, p)?;
    }
    record_fill(q, w.run_id, order_id, p, side.opposite(), qty, price, fee, role, kind, realised, journal)
}

#[allow(clippy::too_many_arguments)]
fn record_fill<Q: Queryable>(
    q: &mut Q,
    run_id: u64,
    order_id: Option<u64>,
    p: &store::Position,
    side: Side,
    qty: f64,
    price: f64,
    fee: f64,
    role: FeeRole,
    kind: &str,
    realised: f64,
    j: Journal,
) -> Result<(), String> {
    store::add_fill(
        q,
        &store::NewFill {
            run_id,
            order_id,
            position_id: p.id,
            symbol: &p.symbol,
            side: side_word(side),
            qty,
            price,
            fee,
            fee_role: role_word(role),
            kind,
            realised_pnl: realised,
            bid: j.bid,
            ask: j.ask,
            mark: j.mark,
            feed_age_ms: j.feed_age_ms,
        },
    )
    .map(|_| ())
}

// ---------------------------------------------------------------- placing

/// Places one order. The same client id twice returns the first order, never a second one.
pub fn place(conn: &mut Conn, board: &Board, input: &OrderInput, source: Source) -> Result<store::Order, String> {
    let asked = read_input(input)?;
    if let Some(seen) = store::order_by_client(conn, &asked.client_order_id)? {
        return Ok(seen);
    }
    let run = store::open_run(conn, MARKET)?.ok_or(NO_RUN)?;
    let (prices, quote) = prices_for(conn, board, Some(run.id), Some(&asked.symbol))?;
    let quote = quote.ok_or(STALE)?;
    let fills_at_once = asked.kind == OrderType::Market || asked.price.is_some_and(|p| crosses(asked.side, p, quote.bid, quote.ask));
    let book = if fills_at_once {
        Some(board.book(&asked.symbol).map_err(|_| "The order book could not be read, so nothing was placed.".to_string())?)
    } else {
        None
    };
    let fees = fees_of(&store::setting(conn, MARKET)?);
    let instrument = prices.instruments.get(&asked.symbol).cloned().ok_or(STALE)?;
    let chosen = choice(conn, &asked.symbol, &instrument.rules)?;

    let placed = {
        let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the account"))?;
        let done = place_locked(&mut tx, run.id, &asked, source, &quote, book.as_ref(), &prices, &instrument, chosen, fees);
        match done {
            Ok(Placed::Seen(order)) => return Ok(order),
            Ok(Placed::New(id)) => tx.commit().map_err(sql("Saving the order")).map(|_| id),
            Err(e) => Err(e),
        }
    };
    match placed {
        Ok(id) => store::order(conn, id)?.ok_or_else(|| "The order could not be read back.".to_string()),
        // Two clicks raced past the first look: the unique client id let one through, and that one is the answer.
        Err(e) => store::order_by_client(conn, &asked.client_order_id)?.ok_or(e),
    }
}

enum Placed {
    Seen(store::Order),
    New(u64),
}

#[allow(clippy::too_many_arguments)]
fn place_locked<Q: Queryable>(
    q: &mut Q,
    run_id: u64,
    a: &Asked,
    source: Source,
    quote: &feed::Quote,
    book: Option<&feed::Book>,
    prices: &Prices,
    instrument: &rules::Instrument,
    chosen: (MarginMode, f64),
    fees: Fees,
) -> Result<Placed, String> {
    let run = store::lock_run(q, run_id)?;
    if let Some(seen) = store::order_by_client(q, &a.client_order_id)? {
        return Ok(Placed::Seen(seen));
    }
    let positions = store::open_positions(q, run.id)?;
    let orders = store::resting_orders(q, run.id)?;
    let snapshot = snapshot_of(run.balance, &positions, &orders, prices, fees);
    let position = positions.iter().find(|p| p.symbol == a.symbol);
    let held = position.map(held_of);

    let mut qty = a.qty;
    if a.reduce_only {
        match held {
            Some(h) if h.side != a.side => qty = qty.min(h.qty),
            _ => return Err("Reduce only needs an open position on the other side.".into()),
        }
    }
    let adding = held.is_some_and(|h| h.side == a.side);
    let against = held.is_some_and(|h| h.side != a.side);
    let pos_mode = position.map(|p| mode_of(p.margin_mode.as_deref()));
    let pos_leverage = position.and_then(|p| p.leverage).unwrap_or(chosen.1);
    let mode = if adding { pos_mode.unwrap_or(chosen.0) } else { chosen.0 };
    let leverage = if adding || a.reduce_only { pos_leverage } else { chosen.1 };

    let reference = match a.side {
        Side::Long => quote.ask,
        Side::Short => quote.bid,
    };
    let existing = if adding { rules::notional(held.map_or(0.0, |h| h.qty), quote.mark) } else { 0.0 };
    rules::check_order(&instrument.rules, &instrument.tiers, qty, a.price, reference, leverage, existing).map_err(|e| format!("{e}."))?;
    check_exits(a.side, a.tp, a.sl, a.price.unwrap_or(reference), instrument.rules.tick_size)?;

    let walk = book.map(|b| rules::walk_book(&walkable(b, a.side, a.price), qty));
    let fills_now = walk.is_some_and(|w| w.complete);
    if a.kind == OrderType::Market && !fills_now {
        return Err("The order book is too thin for this size right now.".into());
    }
    let opening = if a.reduce_only {
        0.0
    } else if against {
        money((qty - held.map_or(0.0, |h| h.qty)).max(0.0))
    } else {
        qty
    };

    let mut new = store::NewOrder {
        run_id: run.id,
        client_order_id: &a.client_order_id,
        source: source.as_str(),
        symbol: &a.symbol,
        side: side_word(a.side),
        order_type: if a.kind == OrderType::Market { "MARKET" } else { "LIMIT" },
        reduce_only: a.reduce_only,
        margin_mode: mode_word(mode),
        leverage,
        qty,
        price: a.price,
        reserved_cost: 0.0,
        tp_price: a.tp,
        sl_price: a.sl,
        status: "NEW",
    };

    if !fills_now {
        let price = a.price.unwrap_or(reference);
        let cost = rules::opening_cost(a.side, opening, price, leverage, FeeRole::Maker, &fees);
        require_money(cost, rules::available(&snapshot))?;
        new.reserved_cost = money(cost);
        return store::add_order(q, &new).map(Placed::New);
    }

    let walk = walk.ok_or(STALE)?;
    let price = walk.average_price;
    let cost = rules::opening_cost(a.side, opening, price, leverage, FeeRole::Taker, &fees);
    let realised = match held {
        Some(h) if against => rules::unrealised_pnl(h.side, h.entry, price, h.qty.min(qty)),
        _ => 0.0,
    };
    let closing_id = position.filter(|_| against).map(|p| p.id);
    require_money(cost, available_after_closing(&snapshot, closing_id, realised))?;
    new.status = "FILLED";
    let order_id = store::add_order(q, &new)?;
    store::finish_order(q, order_id, "FILLED", None)?;
    let mut wallet = Wallet { run_id: run.id, balance: run.balance };
    let rq = quote_of(quote);
    let fill = FillWrite {
        order_id: Some(order_id),
        symbol: &a.symbol,
        side: a.side,
        qty,
        price,
        role: FeeRole::Taker,
        kind: "TRADE",
        mode,
        leverage,
        tp: a.tp,
        sl: a.sl,
        opened_by: source.as_str(),
        reduce_only: a.reduce_only,
        journal: Journal::of(Some(&rq), Some((now_ms() - quote.at_ms).max(0))),
    };
    apply_fill(q, &mut wallet, &fill, fees)?;
    floor_at_zero(q, &mut wallet, Some(&a.symbol))?;
    store::set_balance(q, run.id, wallet.balance)?;
    Ok(Placed::New(order_id))
}

fn require_money(cost: f64, available: f64) -> Result<(), String> {
    if cost > available + EPS {
        return Err(format!("Not enough paper money: this needs {} USDT and {} USDT is available.", plain(cost), plain(available)));
    }
    Ok(())
}

/// What an order would cost and do, without writing anything.
pub fn preview(conn: &mut Conn, board: &Board, input: &OrderInput) -> Result<Preview, String> {
    let refused = |why: String, available: f64| Ok(Preview { available, refusal: Some(why), ..Preview::default() });
    let a = match read_input(&OrderInput { client_order_id: "preview".into(), ..input.clone() }) {
        Ok(a) => a,
        Err(e) => return refused(e, 0.0),
    };
    let Some(run) = store::open_run(conn, MARKET)? else {
        return refused(NO_RUN.into(), 0.0);
    };
    let (prices, quote) = match prices_for(conn, board, Some(run.id), Some(&a.symbol)) {
        Ok((p, Some(q))) => (p, q),
        Ok((_, None)) => return refused(STALE.into(), 0.0),
        Err(e) => return refused(e, 0.0),
    };
    let fees = fees_of(&store::setting(conn, MARKET)?);
    let Some(instrument) = prices.instruments.get(&a.symbol).cloned() else {
        return refused(STALE.into(), 0.0);
    };
    let chosen = choice(conn, &a.symbol, &instrument.rules)?;
    let positions = store::open_positions(conn, run.id)?;
    let orders = store::resting_orders(conn, run.id)?;
    let snapshot = snapshot_of(run.balance, &positions, &orders, &prices, fees);
    let available = rules::available(&snapshot);
    let position = positions.iter().find(|p| p.symbol == a.symbol);
    let held = position.map(held_of);
    let adding = held.is_some_and(|h| h.side == a.side);
    let leverage = if adding || a.reduce_only { position.and_then(|p| p.leverage).unwrap_or(chosen.1) } else { chosen.1 };
    let reference = match a.side {
        Side::Long => quote.ask,
        Side::Short => quote.bid,
    };
    let at = a.price.unwrap_or(reference);
    let mut qty = a.qty;
    if a.reduce_only {
        match held {
            Some(h) if h.side != a.side => qty = qty.min(h.qty),
            _ => return refused("Reduce only needs an open position on the other side.".into(), available),
        }
    }
    let existing = if adding { rules::notional(held.map_or(0.0, |h| h.qty), quote.mark) } else { 0.0 };
    if let Err(e) = rules::check_order(&instrument.rules, &instrument.tiers, qty, a.price, reference, leverage, existing) {
        return refused(format!("{e}."), available);
    }
    if let Err(e) = check_exits(a.side, a.tp, a.sl, at, instrument.rules.tick_size) {
        return refused(e, available);
    }
    let rests = a.price.is_some_and(|p| !crosses(a.side, p, quote.bid, quote.ask));
    let role = if rests { FeeRole::Maker } else { FeeRole::Taker };
    let rate = if rests { fees.maker } else { fees.taker };
    let opening = if a.reduce_only {
        0.0
    } else if held.is_some_and(|h| h.side != a.side) {
        money((qty - held.map_or(0.0, |h| h.qty)).max(0.0))
    } else {
        qty
    };
    let cost = rules::opening_cost(a.side, opening, at, leverage, role, &fees);
    let fee = rules::fee(rules::notional(qty, at), rate);
    let margin = rules::initial_margin(rules::notional(opening, at), leverage);
    let refusal = (cost > available + EPS).then(|| "Not enough paper money for this order.".to_string());
    Ok(Preview { cost: money(cost), fee, margin, available, refusal })
}

// ---------------------------------------------------------------- cancelling, closing, changing

/// Takes a resting order off the book; what it held is available again at once.
pub fn cancel(conn: &mut Conn, order_id: u64) -> Result<(), String> {
    let seen = store::order(conn, order_id)?.ok_or("That order no longer exists.")?;
    let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the account"))?;
    store::lock_run(&mut tx, seen.run_id)?;
    let order = store::order(&mut tx, order_id)?.ok_or("That order no longer exists.")?;
    if order.status != "NEW" {
        return Err("That order is no longer resting.".into());
    }
    store::finish_order(&mut tx, order_id, "CANCELLED", None)?;
    tx.commit().map_err(sql("Cancelling the order"))
}

/// Closes a whole position at the market, walking the real book. Who asked is written on a closing order row.
pub fn close(conn: &mut Conn, board: &Board, position_id: u64, source: Source) -> Result<(), String> {
    close_at_market(conn, board, position_id, source, "TRADE")
}

/// Closes a whole position at the market with a fill kind (TRADE, TAKE_PROFIT or STOP_LOSS).
fn close_at_market(conn: &mut Conn, board: &Board, position_id: u64, source: Source, kind: &str) -> Result<(), String> {
    let seen = store::position(conn, position_id)?.ok_or(NO_POSITION)?;
    if seen.closed_at.is_some() {
        return Err(POSITION_CLOSED.into());
    }
    let quote = board.quote(&seen.symbol).map_err(|_| format!("{STALE} Nothing was closed."))?;
    let book = board.book(&seen.symbol).map_err(|_| "The order book could not be read, so nothing was closed.".to_string())?;
    let fees = fees_of(&store::setting(conn, MARKET)?);
    let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the account"))?;
    let run = store::lock_run(&mut tx, seen.run_id)?;
    let Some(mut position) = store::position(&mut tx, position_id)?.filter(|p| p.closed_at.is_none()) else {
        return Err(POSITION_CLOSED.into());
    };
    let side = parse_side(&position.side)?;
    let levels = walkable(&book, side.opposite(), None);
    let walk = rules::walk_book(&levels, position.qty);
    if !walk.complete {
        return Err("The order book is too thin to close this size right now.".into());
    }
    let client = format!("close-{}-{}", position.id, now_ms());
    let order_id = store::add_order(
        &mut tx,
        &store::NewOrder {
            run_id: run.id,
            client_order_id: &client,
            source: source.as_str(),
            symbol: &position.symbol,
            side: side_word(side.opposite()),
            order_type: "MARKET",
            reduce_only: true,
            margin_mode: position.margin_mode.as_deref().unwrap_or("CROSS"),
            leverage: position.leverage.unwrap_or(DEFAULT_LEVERAGE),
            qty: position.qty,
            price: None,
            reserved_cost: 0.0,
            tp_price: None,
            sl_price: None,
            status: "FILLED",
        },
    )?;
    store::finish_order(&mut tx, order_id, "FILLED", (kind != "TRADE").then_some(kind))?;
    let price = walk.average_price;
    let realised = rules::unrealised_pnl(side, position.entry_price, price, position.qty);
    let fee = rules::fee(rules::notional(position.qty, price), fees.taker);
    let mut wallet = Wallet { run_id: run.id, balance: run.balance };
    let rq = quote_of(&quote);
    let journal = Journal::of(Some(&rq), Some((now_ms() - quote.at_ms).max(0)));
    let qty = position.qty;
    close_part(&mut tx, &mut wallet, &mut position, Some(order_id), qty, price, FeeRole::Taker, kind, journal, realised, fee)?;
    floor_at_zero(&mut tx, &mut wallet, Some(&position.symbol))?;
    store::set_balance(&mut tx, run.id, wallet.balance)?;
    tx.commit().map_err(sql("Closing the position"))
}

/// Sets or clears a position's take profit and stop loss (None clears).
pub fn set_exits(conn: &mut Conn, board: &Board, position_id: u64, tp: Option<f64>, sl: Option<f64>) -> Result<(), String> {
    let seen = store::position(conn, position_id)?.ok_or(NO_POSITION)?;
    let instrument = instrument_data(conn, board, &seen.symbol)?;
    let mark = board.quote(&seen.symbol).map_err(|_| STALE.to_string())?.mark;
    let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the account"))?;
    store::lock_run(&mut tx, seen.run_id)?;
    let mut position = store::position(&mut tx, position_id)?.filter(|p| p.closed_at.is_none()).ok_or(POSITION_CLOSED)?;
    check_exits(parse_side(&position.side)?, tp, sl, mark, instrument.rules.tick_size)?;
    position.tp_price = tp;
    position.sl_price = sl;
    store::save_position(&mut tx, &position)?;
    tx.commit().map_err(sql("Saving the exits"))
}

/// Moves money from what is available onto one isolated position, pushing its liquidation price away.
pub fn add_margin(conn: &mut Conn, board: &Board, position_id: u64, amount: f64) -> Result<(), String> {
    if !amount.is_finite() || amount <= 0.0 {
        return Err("The amount must be above zero.".into());
    }
    let seen = store::position(conn, position_id)?.ok_or(NO_POSITION)?;
    let (prices, _) = prices_for(conn, board, Some(seen.run_id), Some(&seen.symbol))?;
    let fees = fees_of(&store::setting(conn, MARKET)?);
    let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the account"))?;
    let run = store::lock_run(&mut tx, seen.run_id)?;
    let mut position = store::position(&mut tx, position_id)?.filter(|p| p.closed_at.is_none()).ok_or(POSITION_CLOSED)?;
    if mode_of(position.margin_mode.as_deref()) != MarginMode::Isolated {
        return Err("Only an isolated position holds its own margin.".into());
    }
    let positions = store::open_positions(&mut tx, run.id)?;
    let orders = store::resting_orders(&mut tx, run.id)?;
    require_money(amount, rules::available(&snapshot_of(run.balance, &positions, &orders, &prices, fees)))?;
    position.extra_margin = money(position.extra_margin + amount);
    store::save_position(&mut tx, &position)?;
    tx.commit().map_err(sql("Adding the margin"))
}

/// The member's mode and leverage for a symbol, only while it holds no position and no resting order.
pub fn set_leverage(conn: &mut Conn, board: &Board, symbol: &str, margin_mode: &str, leverage: f64) -> Result<(), String> {
    let symbol = clean_symbol(symbol);
    let mode = parse_mode(margin_mode)?;
    let instrument = instrument_data(conn, board, &symbol)?;
    rules::check_leverage(&instrument.rules, leverage).map_err(|e| format!("{e}."))?;
    let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the account"))?;
    if let Some(run) = store::open_run(&mut tx, MARKET)? {
        store::lock_run(&mut tx, run.id)?;
        let busy = store::open_positions(&mut tx, run.id)?.iter().any(|p| p.symbol == symbol)
            || store::resting_orders(&mut tx, run.id)?.iter().any(|o| o.symbol == symbol);
        if busy {
            return Err(format!("Close the position and orders on {symbol} first."));
        }
    }
    store::save_instrument_choice(&mut tx, MARKET, &symbol, mode_word(mode), leverage)?;
    tx.commit().map_err(sql("Saving the leverage"))
}

/// The fee rates (fractions, 0 to 1 %).
pub fn set_fees(conn: &mut Conn, taker: f64, maker: f64) -> Result<(), String> {
    check_fees(taker, maker)?;
    let mut setting = store::setting(conn, MARKET)?;
    setting.taker_rate = taker;
    setting.maker_rate = maker;
    store::save_setting(conn, MARKET, &setting)
}

// ---------------------------------------------------------------- runs

/// Starts a run with paper money. Refused while the open run still holds anything.
pub fn new_run(conn: &mut Conn, start_balance: f64) -> Result<store::Run, String> {
    let amount = check_start(start_balance)?;
    let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the account"))?;
    if let Some(open) = store::open_run(&mut tx, MARKET)? {
        let current = store::lock_run(&mut tx, open.id)?;
        let positions = store::open_positions(&mut tx, current.id)?;
        let orders = store::resting_orders(&mut tx, current.id)?;
        if !nothing_left(current.balance, positions.len(), orders.len()) {
            return Err("This account is still going. Close it first if you want to start over.".into());
        }
        end_locked(&mut tx, current.id)?;
    }
    let run = store::new_run(&mut tx, MARKET, amount)?;
    tx.commit().map_err(sql("Starting the account"))?;
    Ok(run)
}

/// Closes the account: everything open is sold at the market, resting orders are cancelled, the run ends
/// and stays readable.
pub fn close_run(conn: &mut Conn, board: &Board) -> Result<(), String> {
    let open = store::open_run(conn, MARKET)?.ok_or("There is no account to close.")?;
    // Each close reads the book outside the lock and takes its own turn
    for position in store::open_positions(conn, open.id)? {
        close(conn, board, position.id, Source::Member)?;
    }
    let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the account"))?;
    store::lock_run(&mut tx, open.id)?;
    end_locked(&mut tx, open.id)?;
    tx.commit().map_err(sql("Closing the account"))
}

fn end_locked<Q: Queryable>(q: &mut Q, run_id: u64) -> Result<(), String> {
    if !store::open_positions(q, run_id)?.is_empty() {
        return Err("A position opened while the account was closing: try again.".into());
    }
    for order in store::resting_orders(q, run_id)? {
        store::finish_order(q, order.id, "CANCELLED", Some("Cancelled: the account closed"))?;
    }
    store::end_run(q, run_id)
}

// ---------------------------------------------------------------- the engine's door

/// Applies what these prices make happen, under the lock, judging again from the rows just read so a
/// click that got there first is respected: liquidations, cancels and limit fills (maker) here; a take
/// profit or stop loss is then closed at the market walking the book, outside the lock. In a gap replay
/// (`replay`) a take profit or stop loss fills at its own price instead. True when anything changed.
pub fn apply_triggers(conn: &mut Conn, board: &Board, prices: &Prices, replay: bool) -> Result<bool, String> {
    let Some(open) = store::open_run(conn, MARKET)? else {
        return Ok(false);
    };
    let fees = fees_of(&store::setting(conn, MARKET)?);
    let mut hand_back: Vec<Trigger> = Vec::new();
    let mut changed = false;
    {
        let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the account"))?;
        let run = store::lock_run(&mut tx, open.id)?;
        let positions = store::open_positions(&mut tx, run.id)?;
        let orders = store::resting_orders(&mut tx, run.id)?;
        let snapshot = snapshot_of(run.balance, &positions, &orders, prices, fees);
        let mut wallet = Wallet { run_id: run.id, balance: run.balance };
        for trigger in rules::triggers(&snapshot) {
            match trigger.kind {
                TriggerKind::Liquidate => changed |= close_at_price(&mut tx, &mut wallet, &trigger, "LIQUIDATION", prices, fees)?,
                TriggerKind::CancelOrder => {
                    if let Some(order) = store::order(&mut tx, trigger.id)?.filter(|o| o.status == "NEW") {
                        let note = if order.reduce_only { "Cancelled: nothing left to reduce" } else { "Cancelled: the account's margin ran out" };
                        store::finish_order(&mut tx, order.id, "CANCELLED", Some(note))?;
                        changed = true;
                    }
                }
                TriggerKind::FillOrder => changed |= fill_resting(&mut tx, &mut wallet, &trigger, prices, fees)?,
                TriggerKind::TakeProfit | TriggerKind::StopLoss => {
                    let kind = if trigger.kind == TriggerKind::TakeProfit { "TAKE_PROFIT" } else { "STOP_LOSS" };
                    if replay {
                        changed |= close_at_price(&mut tx, &mut wallet, &trigger, kind, prices, fees)?;
                    } else {
                        hand_back.push(trigger);
                    }
                }
            }
        }
        if changed {
            floor_at_zero(&mut tx, &mut wallet, None)?;
            store::set_balance(&mut tx, run.id, wallet.balance)?;
        }
        tx.commit().map_err(sql("Saving what the prices did"))?;
    }
    for trigger in hand_back {
        let kind = if trigger.kind == TriggerKind::TakeProfit { "TAKE_PROFIT" } else { "STOP_LOSS" };
        let opened_by = store::position(conn, trigger.id)?.map(|p| p.opened_by).unwrap_or_default();
        let source = match opened_by.as_str() {
            "COMPANION_ASKED" => Source::CompanionAsked,
            "COMPANION_AUTO" => Source::CompanionAuto,
            _ => Source::Member,
        };
        // The next tick sees the same price and tries again, unless the position was closed by hand
        if close_at_market(conn, board, trigger.id, source, kind).is_ok() {
            changed = true;
        }
    }
    Ok(changed)
}

/// Closes a whole position at the trigger's price (a liquidation, or a replayed exit). False when already gone.
fn close_at_price<Q: Queryable>(q: &mut Q, w: &mut Wallet, trigger: &Trigger, kind: &str, prices: &Prices, fees: Fees) -> Result<bool, String> {
    let Some(mut position) = store::position(q, trigger.id)?.filter(|p| p.closed_at.is_none()) else {
        return Ok(false);
    };
    let side = parse_side(&position.side)?;
    let price = trigger.price.unwrap_or(position.entry_price);
    let qty = position.qty;
    let realised = rules::unrealised_pnl(side, position.entry_price, price, qty);
    let fee = rules::fee(rules::notional(qty, price), fees.taker);
    let journal = Journal::of(prices.quotes.get(&position.symbol), None);
    close_part(q, w, &mut position, None, qty, price, FeeRole::Taker, kind, journal, realised, fee)?;
    Ok(true)
}

/// Fills a resting limit at its own price as a maker; a reduce-only order with nothing left to reduce is cancelled.
fn fill_resting<Q: Queryable>(q: &mut Q, w: &mut Wallet, trigger: &Trigger, prices: &Prices, fees: Fees) -> Result<bool, String> {
    let Some(order) = store::order(q, trigger.id)?.filter(|o| o.status == "NEW") else {
        return Ok(false);
    };
    let side = parse_side(&order.side)?;
    let held = open_position_on(q, w.run_id, &order.symbol)?.map(|p| held_of(&p));
    let mut qty = order.qty;
    if order.reduce_only {
        match held {
            Some(h) if h.side != side => qty = qty.min(h.qty),
            _ => {
                store::finish_order(q, order.id, "CANCELLED", Some("Cancelled: nothing left to reduce"))?;
                return Ok(true);
            }
        }
    }
    let price = order.price.or(trigger.price).ok_or("A resting order has no price.")?;
    store::finish_order(q, order.id, "FILLED", None)?;
    let fill = FillWrite {
        order_id: Some(order.id),
        symbol: &order.symbol,
        side,
        qty,
        price,
        role: FeeRole::Maker,
        kind: "TRADE",
        mode: mode_of(order.margin_mode.as_deref()),
        leverage: order.leverage.unwrap_or(DEFAULT_LEVERAGE),
        tp: order.tp_price,
        sl: order.sl_price,
        opened_by: &order.source,
        reduce_only: order.reduce_only,
        journal: Journal::of(prices.quotes.get(&order.symbol), None),
    };
    apply_fill(q, w, &fill, fees)?;
    Ok(true)
}

/// Settles one funding time on one position, once: the database's unique key refuses a second time.
/// Paid from what is available first; what is short comes out of an isolated position's own margin.
/// True when it was settled now.
pub fn settle_funding(conn: &mut Conn, position_id: u64, funding_time: i64, rate: f64, mark: f64, prices: &Prices) -> Result<bool, String> {
    let fees = fees_of(&store::setting(conn, MARKET)?);
    let Some(seen) = store::position(conn, position_id)? else {
        return Ok(false);
    };
    let mut tx = conn.start_transaction(TxOpts::default()).map_err(sql("Opening the account"))?;
    let Ok(run) = store::lock_run(&mut tx, seen.run_id) else {
        return Ok(false);
    };
    let Some(mut position) = store::position(&mut tx, position_id)?.filter(|p| p.closed_at.is_none()) else {
        return Ok(false);
    };
    let side = parse_side(&position.side)?;
    let amount = rules::settlement(side, position.qty, mark, rate);
    if !store::add_funding(&mut tx, run.id, position.id, &position.symbol, funding_time, rate, mark, amount)? {
        return Ok(false);
    }
    let paying = money(-amount);
    if paying > EPS && mode_of(position.margin_mode.as_deref()) == MarginMode::Isolated {
        let positions = store::open_positions(&mut tx, run.id)?;
        let orders = store::resting_orders(&mut tx, run.id)?;
        let available = rules::available(&snapshot_of(run.balance, &positions, &orders, prices, fees));
        let short_of = (paying - available).max(0.0);
        position.extra_margin = money(position.extra_margin - short_of);
    }
    position.funding_paid = money(position.funding_paid + paying);
    store::save_position(&mut tx, &position)?;
    let mut wallet = Wallet { run_id: run.id, balance: run.balance };
    let note = format!("Rate {}", plain(rate));
    post(&mut tx, &mut wallet, "FUNDING", amount, Some(&position.symbol), Some(&note))?;
    floor_at_zero(&mut tx, &mut wallet, Some(&position.symbol))?;
    store::set_balance(&mut tx, run.id, wallet.balance)?;
    tx.commit().map_err(sql("Settling the funding"))?;
    Ok(true)
}

#[cfg(test)]
#[path = "trade_tests.rs"]
mod tests;
