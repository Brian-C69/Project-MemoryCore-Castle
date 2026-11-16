//! The trade door's words and pure helpers: what an order asks, how a fill changes a position, and
//! the checks before anything is written. Kept apart from trade.rs (the doors themselves) so each
//! file stays readable; trade.rs brings these in whole.

#![allow(unused_imports)]

use std::collections::HashMap;

use mysql::prelude::Queryable;
use serde::{Deserialize, Serialize};

use crate::market::board::Board;
use crate::market::feed;
use crate::market::rules::{self, FeeRole, Fees, MarginMode, OrderType, Side, Snapshot, Trigger, TriggerKind, EPS};
use crate::market::store;
use crate::market::trade::{DEFAULT_LEVERAGE, LARGEST_START, NOTHING_LEFT_UNDER, TOP_FEE_RATE};

// ---------------------------------------------------------------- the words

/// Who placed an order or opened a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Member,
    CompanionAsked,
    CompanionAuto,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Member => "MEMBER",
            Source::CompanionAsked => "COMPANION_ASKED",
            Source::CompanionAuto => "COMPANION_AUTO",
        }
    }
}

pub fn side_word(side: Side) -> &'static str {
    match side {
        Side::Long => "LONG",
        Side::Short => "SHORT",
    }
}

pub fn parse_side(word: &str) -> Result<Side, String> {
    match word.trim().to_ascii_uppercase().as_str() {
        "LONG" | "BUY" => Ok(Side::Long),
        "SHORT" | "SELL" => Ok(Side::Short),
        _ => Err("Choose long or short.".into()),
    }
}

pub fn mode_word(mode: MarginMode) -> &'static str {
    match mode {
        MarginMode::Cross => "CROSS",
        MarginMode::Isolated => "ISOLATED",
    }
}

pub fn parse_mode(word: &str) -> Result<MarginMode, String> {
    match word.trim().to_ascii_uppercase().as_str() {
        "CROSS" => Ok(MarginMode::Cross),
        "ISOLATED" => Ok(MarginMode::Isolated),
        _ => Err("Choose cross or isolated.".into()),
    }
}

pub(crate) fn mode_of(word: Option<&str>) -> MarginMode {
    word.and_then(|w| parse_mode(w).ok()).unwrap_or(MarginMode::Cross)
}

pub(crate) fn role_word(role: FeeRole) -> &'static str {
    match role {
        FeeRole::Taker => "TAKER",
        FeeRole::Maker => "MAKER",
    }
}

/// A symbol as the feed spells it.
pub fn clean_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_uppercase()
}

/// Money and prices rounded the way the database keeps them (10 places).
pub(crate) fn money(value: f64) -> f64 {
    (value * 1e10).round() / 1e10
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// A number the way a member reads it.
pub(crate) fn plain(value: f64) -> String {
    format!("{}", money(value))
}

pub(crate) fn sql(what: &'static str) -> impl Fn(mysql::Error) -> String {
    move |_| format!("{what} could not be done in the database.")
}

// ---------------------------------------------------------------- what comes in and goes out

/// An order as the screen or the companion sends it.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInput {
    pub symbol: String,
    /// "LONG" or "SHORT".
    pub side: String,
    /// "MARKET" or "LIMIT".
    pub order_type: String,
    pub qty: f64,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub tp: Option<f64>,
    #[serde(default)]
    pub sl: Option<f64>,
    #[serde(default)]
    pub reduce_only: bool,
    pub client_order_id: String,
}

/// What an order would cost, for the form; nothing is written. `refusal` says why it would be refused.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub cost: f64,
    pub fee: f64,
    pub margin: f64,
    pub available: f64,
    pub refusal: Option<String>,
}

/// One order, read and checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Asked {
    pub symbol: String,
    pub side: Side,
    pub kind: OrderType,
    pub qty: f64,
    pub price: Option<f64>,
    pub tp: Option<f64>,
    pub sl: Option<f64>,
    pub reduce_only: bool,
    pub client_order_id: String,
}

/// Reads an order's words; a market order's price is ignored.
pub fn read_input(input: &OrderInput) -> Result<Asked, String> {
    let client = input.client_order_id.trim();
    if client.is_empty() || client.len() > 64 {
        return Err("An order needs a client id of at most 64 characters.".into());
    }
    let symbol = clean_symbol(&input.symbol);
    if symbol.is_empty() || symbol.len() > 32 {
        return Err("Choose a symbol.".into());
    }
    let side = parse_side(&input.side)?;
    let kind = match input.order_type.trim().to_ascii_uppercase().as_str() {
        "MARKET" => OrderType::Market,
        "LIMIT" => OrderType::Limit,
        _ => return Err("Choose market or limit.".into()),
    };
    let price = match kind {
        OrderType::Market => None,
        OrderType::Limit => Some(input.price.filter(|p| p.is_finite() && *p > 0.0).ok_or("A limit order needs a price above zero.")?),
    };
    if !input.qty.is_finite() || input.qty <= 0.0 {
        return Err("The quantity must be above zero.".into());
    }
    let exit = |v: Option<f64>| v.filter(|p| p.is_finite() && *p > 0.0);
    Ok(Asked {
        symbol,
        side,
        kind,
        qty: input.qty,
        price,
        tp: exit(input.tp),
        sl: exit(input.sl),
        reduce_only: input.reduce_only,
        client_order_id: client.to_string(),
    })
}

// ---------------------------------------------------------------- pure helpers

/// What is held on one symbol.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held {
    pub side: Side,
    pub qty: f64,
    pub entry: f64,
}

/// What one fill does in one-way mode: closes what it is against, then opens or adds what is left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillPlan {
    pub closing: f64,
    pub realised: f64,
    pub close_fee: f64,
    pub opening: f64,
    pub open_fee: f64,
    /// What is held afterwards; None when flat.
    pub after: Option<Held>,
}

impl FillPlan {
    /// The ledger lines this fill writes, in order: REALISED and FEE for the closing part, FEE for the opening part.
    pub fn ledger(&self) -> Vec<(&'static str, f64)> {
        let mut lines = Vec::new();
        if self.closing > EPS {
            lines.push(("REALISED", self.realised));
            lines.push(("FEE", -self.close_fee));
        }
        if self.opening > EPS {
            lines.push(("FEE", -self.open_fee));
        }
        lines
    }

    /// What the fill changes the balance by.
    pub fn balance_change(&self) -> f64 {
        money(self.ledger().iter().map(|(_, a)| a).sum())
    }
}

/// Plans one fill of `qty` at `price` and fee rate `rate` against what is held.
pub fn plan_fill(held: Option<Held>, side: Side, qty: f64, price: f64, rate: f64, reduce_only: bool) -> FillPlan {
    let against = held.filter(|h| h.side != side && h.qty > EPS);
    let closing = against.map_or(0.0, |h| qty.min(h.qty));
    let realised = against.map_or(0.0, |h| rules::unrealised_pnl(h.side, h.entry, price, closing));
    let close_fee = if closing > EPS { rules::fee(rules::notional(closing, price), rate) } else { 0.0 };
    let opening = if reduce_only { 0.0 } else { money((qty - closing).max(0.0)) };
    let open_fee = if opening > EPS { rules::fee(rules::notional(opening, price), rate) } else { 0.0 };
    let left = held.map(|h| Held { qty: money(h.qty - closing), ..h }).filter(|h| h.qty > EPS);
    let after = if opening > EPS {
        match left {
            Some(h) if h.side == side => Some(Held { side, qty: money(h.qty + opening), entry: rules::average_entry(h.qty, h.entry, opening, price) }),
            _ => Some(Held { side, qty: opening, entry: price }),
        }
    } else {
        left
    };
    FillPlan { closing, realised, close_fee, opening, open_fee, after }
}

/// The FLOOR line a wallet below zero needs: there is no insurance fund to owe.
pub fn floor_line(balance: f64) -> Option<f64> {
    (balance < -EPS).then(|| money(-balance))
}

/// A run with nothing open and less than a unit left may give way to a new one.
pub fn nothing_left(balance: f64, open_positions: usize, resting_orders: usize) -> bool {
    open_positions == 0 && resting_orders == 0 && balance < NOTHING_LEFT_UNDER
}

/// The paper money a run may start with: above zero, at most a billion, at most two decimals.
pub fn check_start(amount: f64) -> Result<f64, String> {
    if !amount.is_finite() || amount <= 0.0 || amount > LARGEST_START {
        return Err("The amount must be above 0 and at most 1,000,000,000 USDT.".into());
    }
    let cents = amount * 100.0;
    if (cents - cents.round()).abs() > 1e-6 {
        return Err("The amount can have at most 2 decimal places.".into());
    }
    Ok((cents.round()) / 100.0)
}

/// Fee rates as fractions, 0 to 1 %.
pub fn check_fees(taker: f64, maker: f64) -> Result<(), String> {
    let fine = |r: f64| r.is_finite() && (0.0..=TOP_FEE_RATE).contains(&r);
    if fine(taker) && fine(maker) {
        Ok(())
    } else {
        Err("A fee rate must be between 0 and 1 %.".into())
    }
}

/// Whether a take profit and a stop loss sit on the right side of the price, and on the tick.
pub fn check_exits(side: Side, tp: Option<f64>, sl: Option<f64>, reference: f64, tick: f64) -> Result<(), String> {
    let on_tick = |p: f64| tick <= 0.0 || ((p / tick) - (p / tick).round()).abs() < 1e-6;
    let long = side == Side::Long;
    if let Some(tp) = tp {
        if !tp.is_finite() || tp <= 0.0 || !on_tick(tp) {
            return Err(format!("The take profit must be above zero and move in steps of {}.", plain(tick)));
        }
        if if long { tp <= reference } else { tp >= reference } {
            return Err(if long { "A long's take profit must be above the price." } else { "A short's take profit must be below the price." }.into());
        }
    }
    if let Some(sl) = sl {
        if !sl.is_finite() || sl <= 0.0 || !on_tick(sl) {
            return Err(format!("The stop loss must be above zero and move in steps of {}.", plain(tick)));
        }
        if if long { sl >= reference } else { sl <= reference } {
            return Err(if long { "A long's stop loss must be below the price." } else { "A short's stop loss must be above the price." }.into());
        }
    }
    Ok(())
}

/// Whether a limit order would trade at once: a buy at or above the ask, a sell at or below the bid.
pub fn crosses(side: Side, price: f64, bid: f64, ask: f64) -> bool {
    match side {
        Side::Long => price >= ask - EPS,
        Side::Short => price <= bid + EPS,
    }
}

/// The side of the book an order takes from (a buy takes the asks), limited to a limit's price.
pub fn walkable(book: &feed::Book, side: Side, limit: Option<f64>) -> Vec<rules::BookLevel> {
    let levels = match side {
        Side::Long => &book.asks,
        Side::Short => &book.bids,
    };
    levels
        .iter()
        .filter(|l| match (limit, side) {
            (None, _) => true,
            (Some(p), Side::Long) => l.price <= p + EPS,
            (Some(p), Side::Short) => l.price >= p - EPS,
        })
        .map(|l| rules::BookLevel { price: l.price, size: l.qty })
        .collect()
}

/// The feed's rules in the judge's words. A missing market-order limit is the limit-order limit.
pub fn rules_of(r: &feed::InstrumentRules) -> rules::Rules {
    rules::Rules {
        tick_size: r.tick_size,
        qty_step: r.qty_step,
        min_qty: r.min_qty,
        max_qty: r.max_qty,
        max_market_qty: r.max_market_qty.unwrap_or(r.max_qty),
        min_notional: r.min_notional.unwrap_or(0.0),
        min_leverage: r.min_leverage,
        max_leverage: r.max_leverage,
        leverage_step: r.leverage_step,
    }
}

pub fn tiers_of(tiers: &[feed::RiskTier]) -> Vec<rules::Tier> {
    tiers
        .iter()
        .map(|t| rules::Tier {
            risk_limit_value: t.risk_limit_value,
            maintenance_margin_rate: t.maintenance_margin_rate,
            initial_margin_rate: t.initial_margin_rate,
            max_leverage: t.max_leverage,
            mm_deduction: t.maintenance_deduction,
        })
        .collect()
}

pub fn quote_of(q: &feed::Quote) -> rules::Quote {
    rules::Quote { last: q.last, mark: q.mark, bid: q.bid, ask: q.ask }
}

pub fn fees_of(setting: &store::Setting) -> Fees {
    Fees { taker: setting.taker_rate, maker: setting.maker_rate }
}

/// A stored position in the judge's words.
pub fn judged_position(p: &store::Position) -> rules::Position {
    rules::Position {
        id: p.id,
        symbol: p.symbol.clone(),
        side: parse_side(&p.side).unwrap_or(Side::Long),
        qty: p.qty,
        entry: p.entry_price,
        leverage: p.leverage.filter(|l| *l > 0.0).unwrap_or(DEFAULT_LEVERAGE),
        mode: mode_of(p.margin_mode.as_deref()),
        extra_margin: p.extra_margin,
        tp: p.tp_price,
        sl: p.sl_price,
    }
}

/// A stored resting order in the judge's words.
pub fn judged_order(o: &store::Order) -> rules::RestingOrder {
    rules::RestingOrder {
        id: o.id,
        symbol: o.symbol.clone(),
        side: parse_side(&o.side).unwrap_or(Side::Long),
        qty: o.qty,
        price: o.price,
        reduce_only: o.reduce_only,
        mode: mode_of(o.margin_mode.as_deref()),
        reserved_cost: o.reserved_cost,
    }
}

/// The prices and rules a decision is made on.
#[derive(Debug, Clone, Default)]
pub struct Prices {
    pub quotes: HashMap<String, rules::Quote>,
    pub instruments: HashMap<String, rules::Instrument>,
}

/// Everything open in a run, as the judge reads it.
pub fn snapshot_of(balance: f64, positions: &[store::Position], orders: &[store::Order], prices: &Prices, fees: Fees) -> Snapshot {
    Snapshot {
        wallet_balance: balance,
        positions: positions.iter().map(judged_position).collect(),
        orders: orders.iter().map(judged_order).collect(),
        quotes: prices.quotes.clone(),
        instruments: prices.instruments.clone(),
        fees,
    }
}

/// What is available once the position this order closes has closed, when that is part of the same order.
pub fn available_after_closing(snapshot: &Snapshot, closing_position: Option<u64>, realised: f64) -> f64 {
    let Some(id) = closing_position else {
        return rules::available(snapshot);
    };
    let mut after = snapshot.clone();
    after.positions.retain(|p| p.id != id);
    after.wallet_balance = money(after.wallet_balance + realised);
    rules::available(&after)
}

pub(crate) fn held_of(p: &store::Position) -> Held {
    Held { side: parse_side(&p.side).unwrap_or(Side::Long), qty: p.qty, entry: p.entry_price }
}

