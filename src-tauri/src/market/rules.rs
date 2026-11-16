//! The paper rules of a USDT linear perpetual, as pure functions: no clock, no database, no feed.
//!
//! Every rate is a fraction (0.00055 means 0.055 %). Money and prices come back rounded to
//! `RESULT_DECIMALS` places, half away from zero, the way the exchange's published examples round.
//! The maths is done in `f64`; where a decimal library would compare exactly, these rules compare
//! with `EPS` so a price that is "on" a line by the cent is treated as on it. Moving a price onto
//! the tick is a separate step and, for a liquidation, always rounds against the trader.
//!
//! The second half is the judge: given one moment's snapshot it says what these prices make
//! happen (liquidations, cancels, limit fills, take profits, stop losses), in the order they
//! should be applied.

// Nothing outside this module calls these yet; the trading room wires them in later.
#![allow(dead_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// How close two numbers must be to count as equal.
pub const EPS: f64 = 1e-9;

/// Decimal places money and prices are rounded to.
pub const RESULT_DECIMALS: i32 = 10;

// ---------------------------------------------------------------- the words

/// The direction of a position or an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Long,
    Short,
}

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::Long => Side::Short,
            Side::Short => Side::Long,
        }
    }
}

/// Cross shares the wallet; isolated limits a position to its own margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarginMode {
    Cross,
    Isolated,
}

/// How an order asks to be filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Market,
    Limit,
}

/// Whether a fill took liquidity (taker) or added it (maker).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeeRole {
    Taker,
    Maker,
}

/// What a moment's prices can make happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TriggerKind {
    Liquidate,
    CancelOrder,
    FillOrder,
    TakeProfit,
    StopLoss,
}

// ---------------------------------------------------------------- what the feed publishes

/// How one perpetual may be ordered, as the public feed publishes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rules {
    pub tick_size: f64,
    pub qty_step: f64,
    pub min_qty: f64,
    /// The largest size of a limit order.
    pub max_qty: f64,
    /// The largest size of a market order (usually smaller than `max_qty`).
    pub max_market_qty: f64,
    /// The smallest order value, in USDT.
    pub min_notional: f64,
    pub min_leverage: f64,
    pub max_leverage: f64,
    pub leverage_step: f64,
}

/// One rung of a perpetual's risk table. `risk_limit_value` is the largest position value the rung
/// covers; `mm_deduction` is zero on the lowest rung.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tier {
    pub risk_limit_value: f64,
    pub maintenance_margin_rate: f64,
    pub initial_margin_rate: f64,
    pub max_leverage: f64,
    pub mm_deduction: f64,
}

/// Fee rates as fractions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Fees {
    pub taker: f64,
    pub maker: f64,
}

// ---------------------------------------------------------------- rounding

/// Rounds money or a price to `RESULT_DECIMALS` places, half away from zero.
fn result(value: f64) -> f64 {
    let scale = 10f64.powi(RESULT_DECIMALS);
    (value * scale).round() / scale
}

fn positive(value: f64) -> bool {
    value > EPS
}

/// Whether a value sits on a whole number of steps.
fn on_step(value: f64, step: f64) -> bool {
    if step <= 0.0 {
        return true;
    }
    let steps = value / step;
    (steps - steps.round()).abs() < 1e-6
}

/// Moves a price to the nearest tick.
pub fn price_on_tick(price: f64, tick_size: f64) -> f64 {
    if tick_size <= 0.0 {
        return result(price);
    }
    result((price / tick_size).round() * tick_size)
}

/// Moves a quantity down to a whole number of steps (a size is never rounded up).
pub fn qty_down_to_step(qty: f64, qty_step: f64) -> f64 {
    if qty_step <= 0.0 {
        return result(qty);
    }
    result((qty / qty_step + 1e-9).floor() * qty_step)
}

/// Moves a liquidation price onto the tick, against the trader: a long up, a short down.
pub fn liquidation_on_tick(side: Side, price: f64, tick_size: f64) -> f64 {
    if tick_size <= 0.0 {
        return result(price);
    }
    let steps = price / tick_size;
    let whole = match side {
        Side::Long => (steps - 1e-9).ceil(),
        Side::Short => (steps + 1e-9).floor(),
    };
    result(whole * tick_size)
}

// ---------------------------------------------------------------- margin maths

/// Quantity times price.
pub fn notional(qty: f64, price: f64) -> f64 {
    result(qty * price)
}

/// The lowest rung whose limit covers the value; a value above the table reads the top rung.
/// None only when the table is empty.
pub fn tier_for(position_value: f64, tiers: &[Tier]) -> Option<&Tier> {
    tiers
        .iter()
        .find(|tier| position_value <= tier.risk_limit_value + EPS)
        .or_else(|| tiers.last())
}

/// Value / leverage.
pub fn initial_margin(position_value: f64, leverage: f64) -> f64 {
    result(position_value / leverage)
}

/// Value x rung rate - rung deduction, never below zero.
pub fn maintenance_margin(position_value: f64, tier: &Tier) -> f64 {
    result((position_value * tier.maintenance_margin_rate - tier.mm_deduction).max(0.0))
}

/// Value x fee rate.
pub fn fee(value: f64, rate: f64) -> f64 {
    result(value * rate)
}

/// The closing fee a position keeps aside: long qty x entry x (1 - 1/L) x taker, short (1 + 1/L).
pub fn fee_to_close(side: Side, qty: f64, entry: f64, leverage: f64, taker_rate: f64) -> f64 {
    let part = 1.0 / leverage;
    let factor = match side {
        Side::Long => 1.0 - part,
        Side::Short => 1.0 + part,
    };
    result(qty * entry * factor * taker_rate)
}

/// What an opening order takes from the wallet: initial margin + opening fee + the fee kept for closing.
pub fn order_cost(
    side: Side,
    qty: f64,
    price: f64,
    leverage: f64,
    open_fee_rate: f64,
    taker_rate: f64,
) -> f64 {
    let value = notional(qty, price);
    initial_margin(value, leverage)
        + fee(value, open_fee_rate)
        + fee_to_close(side, qty, price, leverage, taker_rate)
}

/// What opening this much costs at this fee role; nothing when nothing opens.
pub fn opening_cost(
    side: Side,
    opening_qty: f64,
    price: f64,
    leverage: f64,
    role: FeeRole,
    fees: &Fees,
) -> f64 {
    if !positive(opening_qty) {
        return 0.0;
    }
    let open_rate = match role {
        FeeRole::Maker => fees.maker,
        FeeRole::Taker => fees.taker,
    };
    order_cost(side, opening_qty, price, leverage, open_rate, fees.taker)
}

/// Profit is positive. A long gains when the price rises, a short when it falls.
pub fn unrealised_pnl(side: Side, entry: f64, mark: f64, qty: f64) -> f64 {
    let moved = match side {
        Side::Long => mark - entry,
        Side::Short => entry - mark,
    };
    result(moved * qty)
}

/// The weighted entry after adding to a position.
pub fn average_entry(qty: f64, entry: f64, added_qty: f64, added_price: f64) -> f64 {
    result((qty * entry + added_qty * added_price) / (qty + added_qty))
}

/// Isolated liquidation price, by the published formulas (E entry, S size, L leverage, T taker rate):
/// long  [(E x S) - (E x S / L) - (Extra / (1 - T)) - MMD] / [S - (S x MMR)]
/// short [(E x S) + (E x S / L) + (Extra / (1 + T)) + MMD] / [S + (S x MMR)]
/// A long that cannot be liquidated above zero reads zero.
pub fn liquidation_price(
    side: Side,
    entry: f64,
    qty: f64,
    leverage: f64,
    extra_margin: f64,
    tier: &Tier,
    taker_rate: f64,
) -> f64 {
    let value = entry * qty;
    let margin = value / leverage;
    let margin_of_qty = qty * tier.maintenance_margin_rate;
    match side {
        Side::Long => {
            let extra = extra_margin / (1.0 - taker_rate);
            let top = value - margin - extra - tier.mm_deduction;
            result((top / (qty - margin_of_qty)).max(0.0))
        }
        Side::Short => {
            let extra = extra_margin / (1.0 + taker_rate);
            let top = value + margin + extra + tier.mm_deduction;
            result(top / (qty + margin_of_qty))
        }
    }
}

/// Isolated: the price where the position's own margin is gone. With no extra margin: E x (1 -/+ 1/L).
pub fn bankruptcy_price(side: Side, entry: f64, qty: f64, leverage: f64, extra_margin: f64) -> f64 {
    let per_unit = (entry * qty / leverage + extra_margin) / qty;
    match side {
        Side::Long => result((entry - per_unit).max(0.0)),
        Side::Short => result(entry + per_unit),
    }
}

/// The funding one position pays or receives: qty x mark x rate, positive when received.
/// A positive rate has longs paying shorts; a negative rate has shorts paying longs.
pub fn funding(side: Side, qty: f64, mark: f64, rate: f64) -> f64 {
    let amount = qty * mark * rate;
    result(match side {
        Side::Long => -amount,
        Side::Short => amount,
    })
}

/// One price level of a public order book, best price first.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BookLevel {
    pub price: f64,
    pub size: f64,
}

/// What the book gave a market order: the average price, how much it filled, and whether all of it did.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BookWalk {
    pub average_price: f64,
    pub filled_qty: f64,
    pub complete: bool,
}

/// Walks the opposite side of the book for the size: the slippage is real depth, not a guess.
pub fn walk_book(levels: &[BookLevel], qty: f64) -> BookWalk {
    let mut left = qty;
    let mut paid = 0.0;
    for level in levels {
        if !positive(left) {
            break;
        }
        let taken = left.min(level.size);
        paid += taken * level.price;
        left -= taken;
    }
    let filled = qty - left;
    let average_price = if positive(filled) { result(paid / filled) } else { 0.0 };
    BookWalk {
        average_price,
        filled_qty: result(filled),
        complete: !positive(left),
    }
}

// ---------------------------------------------------------------- the account as one

/// One account's figures, already added up. `wallet_balance` holds everything including locked
/// margin; the cross figures sum the cross positions and orders only; isolated money is walled off
/// and its profit never counts here.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AccountFigures {
    pub wallet_balance: f64,
    pub cross_unrealised_pnl: f64,
    pub cross_initial_margin: f64,
    pub cross_maintenance_margin: f64,
    pub cross_order_cost: f64,
    pub isolated_locked: f64,
    pub isolated_order_cost: f64,
}

/// What an isolated position holds out of the wallet: its margin, any extra margin, and the closing fee.
pub fn isolated_locked(
    side: Side,
    qty: f64,
    entry: f64,
    leverage: f64,
    extra_margin: f64,
    taker_rate: f64,
) -> f64 {
    initial_margin(notional(qty, entry), leverage)
        + extra_margin
        + fee_to_close(side, qty, entry, leverage, taker_rate)
}

/// A position's maintenance requirement: the rung's margin at the mark, plus the closing fee.
/// None only when the risk table is empty.
pub fn position_maintenance(
    side: Side,
    qty: f64,
    entry: f64,
    mark: f64,
    leverage: f64,
    tiers: &[Tier],
    taker_rate: f64,
) -> Option<f64> {
    let value = notional(qty, mark);
    let tier = tier_for(value, tiers)?;
    Some(maintenance_margin(value, tier) + fee_to_close(side, qty, entry, leverage, taker_rate))
}

/// Cross margin balance: the wallet without what isolated positions and orders hold, plus cross profit.
pub fn cross_margin_balance(account: &AccountFigures) -> f64 {
    account.wallet_balance - account.isolated_locked - account.isolated_order_cost
        + account.cross_unrealised_pnl
}

fn rate_of(required: f64, balance: f64) -> f64 {
    if !positive(required) {
        return 0.0;
    }
    // A balance at or below zero has already passed every limit
    if !positive(balance) {
        1.0
    } else {
        required / balance
    }
}

/// Account initial margin rate: cross margin plus cross orders over the margin balance. 1 means 100 %.
pub fn account_initial_rate(account: &AccountFigures) -> f64 {
    rate_of(
        account.cross_initial_margin + account.cross_order_cost,
        cross_margin_balance(account),
    )
}

/// Account maintenance margin rate: cross maintenance over the margin balance. 1 means 100 %.
pub fn account_maintenance_rate(account: &AccountFigures) -> f64 {
    rate_of(account.cross_maintenance_margin, cross_margin_balance(account))
}

/// At 100 % initial margin the resting cross orders that hold margin are cancelled.
pub fn cross_orders_must_cancel(account: &AccountFigures) -> bool {
    positive(account.cross_order_cost) && account_initial_rate(account) >= 1.0 - EPS
}

/// At 100 % maintenance margin every cross position is liquidated, the account as one.
pub fn cross_liquidation_due(account: &AccountFigures) -> bool {
    account_maintenance_rate(account) >= 1.0 - EPS
}

/// Where one cross position would take the account to 100 % maintenance with everything else held
/// still, for the screen only: the account line decides, not this price. `balance_without` is the
/// margin balance without this position's profit.
pub fn cross_liquidation_estimate(
    side: Side,
    qty: f64,
    entry: f64,
    balance_without: f64,
    tier: &Tier,
    fee_to_close: f64,
) -> f64 {
    let value = entry * qty;
    match side {
        Side::Long => {
            let top = value - balance_without - tier.mm_deduction + fee_to_close;
            result((top / (qty * (1.0 - tier.maintenance_margin_rate))).max(0.0))
        }
        Side::Short => {
            let top = balance_without + value + tier.mm_deduction - fee_to_close;
            result((top / (qty * (1.0 + tier.maintenance_margin_rate))).max(0.0))
        }
    }
}

/// What a new order may spend: the margin balance less cross margin and cross orders, never below zero.
pub fn available_balance(account: &AccountFigures) -> f64 {
    let left = cross_margin_balance(account) - account.cross_initial_margin - account.cross_order_cost;
    result(left.max(0.0))
}

// ---------------------------------------------------------------- order checks

/// Prints a number the way a member reads it: no float noise, no trailing zeros.
fn plain(value: f64) -> String {
    format!("{}", result(value))
}

/// Whether an order may be placed. `price` None means a market order, judged at `reference_price`;
/// `existing_value` is what the position already holds. The error is in plain words.
pub fn check_order(
    rules: &Rules,
    tiers: &[Tier],
    qty: f64,
    price: Option<f64>,
    reference_price: f64,
    leverage: f64,
    existing_value: f64,
) -> Result<(), String> {
    if !qty.is_finite() || !positive(qty) {
        return Err("The quantity must be above zero".into());
    }
    if !on_step(qty, rules.qty_step) {
        return Err(format!("The quantity must move in steps of {}", plain(rules.qty_step)));
    }
    if qty < rules.min_qty - EPS {
        return Err(format!("The smallest quantity is {}", plain(rules.min_qty)));
    }
    let largest = if price.is_none() { rules.max_market_qty } else { rules.max_qty };
    if qty > largest + EPS {
        return Err(format!(
            "The largest quantity for this order type is {}",
            plain(largest)
        ));
    }
    if let Some(price) = price {
        if !price.is_finite() || !positive(price) {
            return Err("The price must be above zero".into());
        }
        if !on_step(price, rules.tick_size) {
            return Err(format!("The price must move in steps of {}", plain(rules.tick_size)));
        }
    }
    let value = notional(qty, price.unwrap_or(reference_price));
    if value < rules.min_notional - EPS {
        return Err(format!(
            "The smallest order value is {} USDT",
            plain(rules.min_notional)
        ));
    }
    check_leverage(rules, leverage)?;
    let top = match tiers.last() {
        Some(top) => top,
        None => return Err("The risk table for this perpetual is not loaded".into()),
    };
    let total = existing_value + value;
    if total > top.risk_limit_value + EPS {
        return Err(format!(
            "The position would pass the largest allowed value of {} USDT",
            plain(top.risk_limit_value)
        ));
    }
    if let Some(tier) = tier_for(total, tiers) {
        if leverage > tier.max_leverage + EPS {
            return Err(format!(
                "At this size the leverage can be at most {}x",
                plain(tier.max_leverage)
            ));
        }
    }
    Ok(())
}

/// Whether a leverage is allowed for this perpetual, in plain words.
pub fn check_leverage(rules: &Rules, leverage: f64) -> Result<(), String> {
    if !leverage.is_finite()
        || leverage < rules.min_leverage - EPS
        || leverage > rules.max_leverage + EPS
    {
        return Err(format!(
            "The leverage must be between {}x and {}x",
            plain(rules.min_leverage),
            plain(rules.max_leverage)
        ));
    }
    if !on_step(leverage, rules.leverage_step) {
        return Err(format!(
            "The leverage must move in steps of {}",
            plain(rules.leverage_step)
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------- the judge

/// What the feed publishes about one symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instrument {
    pub rules: Rules,
    pub tiers: Vec<Tier>,
}

/// One symbol's prices at this moment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quote {
    pub last: f64,
    pub mark: f64,
    pub bid: f64,
    pub ask: f64,
}

/// An open paper position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub id: u64,
    pub symbol: String,
    pub side: Side,
    pub qty: f64,
    pub entry: f64,
    pub leverage: f64,
    pub mode: MarginMode,
    /// Margin added by hand on top of the initial margin (isolated only).
    pub extra_margin: f64,
    pub tp: Option<f64>,
    pub sl: Option<f64>,
}

/// A resting order. A None price is a market order and never rests long enough to be judged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestingOrder {
    pub id: u64,
    pub symbol: String,
    pub side: Side,
    pub qty: f64,
    pub price: Option<f64>,
    pub reduce_only: bool,
    pub mode: MarginMode,
    /// What the order holds out of the wallet until it fills (zero for reduce-only).
    pub reserved_cost: f64,
}

/// Everything a decision needs, already read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub wallet_balance: f64,
    pub positions: Vec<Position>,
    pub orders: Vec<RestingOrder>,
    pub quotes: HashMap<String, Quote>,
    pub instruments: HashMap<String, Instrument>,
    pub fees: Fees,
}

/// Something this moment's prices make happen. `id` is the position (LIQUIDATE, TAKE_PROFIT,
/// STOP_LOSS) or the order (FILL_ORDER, CANCEL_ORDER); `price` is where it happens (None for a cancel).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Trigger {
    pub kind: TriggerKind,
    pub id: u64,
    pub price: Option<f64>,
}

fn mark_of(position: &Position, snapshot: &Snapshot) -> f64 {
    snapshot
        .quotes
        .get(&position.symbol)
        .map(|quote| quote.mark)
        .unwrap_or(position.entry)
}

fn tiers_of<'a>(position: &Position, snapshot: &'a Snapshot) -> Option<&'a [Tier]> {
    snapshot
        .instruments
        .get(&position.symbol)
        .map(|data| data.tiers.as_slice())
        .filter(|tiers| !tiers.is_empty())
}

/// The account's figures from the snapshot. None when a cross position's risk table is not loaded.
pub fn account_figures(snapshot: &Snapshot) -> Option<AccountFigures> {
    let taker = snapshot.fees.taker;
    let mut figures = AccountFigures {
        wallet_balance: snapshot.wallet_balance,
        cross_unrealised_pnl: 0.0,
        cross_initial_margin: 0.0,
        cross_maintenance_margin: 0.0,
        cross_order_cost: 0.0,
        isolated_locked: 0.0,
        isolated_order_cost: 0.0,
    };
    for p in &snapshot.positions {
        match p.mode {
            MarginMode::Isolated => {
                figures.isolated_locked +=
                    isolated_locked(p.side, p.qty, p.entry, p.leverage, p.extra_margin, taker);
            }
            MarginMode::Cross => {
                let mark = mark_of(p, snapshot);
                figures.cross_unrealised_pnl += unrealised_pnl(p.side, p.entry, mark, p.qty);
                figures.cross_initial_margin += initial_margin(notional(p.qty, p.entry), p.leverage);
                figures.cross_maintenance_margin += position_maintenance(
                    p.side,
                    p.qty,
                    p.entry,
                    mark,
                    p.leverage,
                    tiers_of(p, snapshot)?,
                    taker,
                )?;
            }
        }
    }
    for order in &snapshot.orders {
        match order.mode {
            MarginMode::Isolated => figures.isolated_order_cost += order.reserved_cost,
            MarginMode::Cross => figures.cross_order_cost += order.reserved_cost,
        }
    }
    Some(figures)
}

/// What a new order may spend right now (zero when a risk table is missing).
pub fn available(snapshot: &Snapshot) -> f64 {
    account_figures(snapshot).map_or(0.0, |a| available_balance(&a))
}

/// How close the account is to its line: 1 means there.
pub fn account_risk(snapshot: &Snapshot) -> f64 {
    account_figures(snapshot).map_or(0.0, |a| account_maintenance_rate(&a))
}

/// The price that would end this position, on the tick and against the trader; None when there is
/// none (a long that cannot fall that far, or rules not loaded). Isolated uses the published
/// formula; cross is the screen estimate with everything else held still.
pub fn ending_price(position: &Position, snapshot: &Snapshot) -> Option<f64> {
    let instrument = snapshot.instruments.get(&position.symbol)?;
    let tiers = tiers_of(position, snapshot)?;
    let tick = instrument.rules.tick_size;
    let side = position.side;
    let value = notional(position.qty, position.entry);
    let tier = tier_for(value, tiers)?;
    let taker = snapshot.fees.taker;
    let price = match position.mode {
        MarginMode::Isolated => liquidation_price(
            side,
            position.entry,
            position.qty,
            position.leverage,
            position.extra_margin,
            tier,
            taker,
        ),
        MarginMode::Cross => {
            let account = account_figures(snapshot)?;
            let mark = mark_of(position, snapshot);
            let this_pnl = unrealised_pnl(side, position.entry, mark, position.qty);
            let this_maintenance = position_maintenance(
                side,
                position.qty,
                position.entry,
                mark,
                position.leverage,
                tiers,
                taker,
            )?;
            let other_maintenance = account.cross_maintenance_margin - this_maintenance;
            let balance_without = cross_margin_balance(&account) - this_pnl - other_maintenance;
            cross_liquidation_estimate(
                side,
                position.qty,
                position.entry,
                balance_without,
                tier,
                fee_to_close(side, position.qty, position.entry, position.leverage, taker),
            )
        }
    };
    if !positive(price) {
        return None;
    }
    Some(liquidation_on_tick(side, price, tick))
}

fn isolated_past_its_price(position: &Position, snapshot: &Snapshot) -> bool {
    if !snapshot.quotes.contains_key(&position.symbol) {
        return false;
    }
    let Some(ending) = ending_price(position, snapshot) else {
        return false;
    };
    let mark = mark_of(position, snapshot);
    match position.side {
        Side::Long => mark <= ending + EPS,
        Side::Short => mark >= ending - EPS,
    }
}

/// Stop loss first, then take profit, both judged by the last traded price.
fn exit_of(position: &Position, snapshot: &Snapshot) -> Option<Trigger> {
    let quote = snapshot.quotes.get(&position.symbol)?;
    let is_long = position.side == Side::Long;
    let last = quote.last;
    if let Some(stop) = position.sl {
        let crossed = if is_long { last <= stop + EPS } else { last >= stop - EPS };
        if crossed {
            return Some(Trigger { kind: TriggerKind::StopLoss, id: position.id, price: Some(stop) });
        }
    }
    if let Some(target) = position.tp {
        let crossed = if is_long { last >= target - EPS } else { last <= target + EPS };
        if crossed {
            return Some(Trigger { kind: TriggerKind::TakeProfit, id: position.id, price: Some(target) });
        }
    }
    None
}

/// A limit order is reached when the best opposite price comes to it: a buy by the ask, a sell by the bid.
fn reached(order: &RestingOrder, snapshot: &Snapshot) -> Option<f64> {
    let quote = snapshot.quotes.get(&order.symbol)?;
    let price = order.price?;
    let touched = match order.side {
        Side::Long => quote.ask <= price + EPS,
        Side::Short => quote.bid >= price - EPS,
    };
    touched.then_some(price)
}

/// What these prices make happen, in the order it should be applied:
/// 1. an account past its maintenance line loses every cross position at the mark price;
/// 2. an isolated position whose mark has reached its liquidation price closes at its bankruptcy price;
/// 3. an account past its line, or at 100 % initial margin, loses its resting cross orders that hold margin;
/// 4. each position still open: stop loss, else take profit, by the last price (stop first when both are crossed);
/// 5. each order still resting: a limit fills when the best opposite price reaches it. A reduce-only
///    order that is reached with nothing on the other side left to reduce is cancelled instead.
///
/// A snapshot whose cross position has no risk table is not judged at all (empty list).
pub fn triggers(snapshot: &Snapshot) -> Vec<Trigger> {
    let mut out = Vec::new();
    let Some(account) = account_figures(snapshot) else {
        return out;
    };
    let mut ended: Vec<u64> = Vec::new();
    let cross_due = cross_liquidation_due(&account);

    for p in &snapshot.positions {
        if p.mode != MarginMode::Isolated && cross_due {
            out.push(Trigger { kind: TriggerKind::Liquidate, id: p.id, price: Some(mark_of(p, snapshot)) });
            ended.push(p.id);
        }
    }
    for p in &snapshot.positions {
        if p.mode == MarginMode::Isolated && isolated_past_its_price(p, snapshot) {
            let price = bankruptcy_price(p.side, p.entry, p.qty, p.leverage, p.extra_margin);
            out.push(Trigger { kind: TriggerKind::Liquidate, id: p.id, price: Some(price) });
            ended.push(p.id);
        }
    }

    let mut cancelled: Vec<u64> = Vec::new();
    if cross_due || cross_orders_must_cancel(&account) {
        for order in &snapshot.orders {
            if order.mode != MarginMode::Isolated && positive(order.reserved_cost) {
                out.push(Trigger { kind: TriggerKind::CancelOrder, id: order.id, price: None });
                cancelled.push(order.id);
            }
        }
    }

    for p in &snapshot.positions {
        if !ended.contains(&p.id) {
            if let Some(exit) = exit_of(p, snapshot) {
                out.push(exit);
                ended.push(p.id);
            }
        }
    }

    // One position per symbol, as it stands once the triggers above have been applied in order;
    // the fills below change it as they go, so a reduce-only order sees what is really left.
    let mut book: HashMap<&str, (Side, f64)> = HashMap::new();
    for p in &snapshot.positions {
        if !ended.contains(&p.id) {
            book.insert(p.symbol.as_str(), (p.side, p.qty));
        }
    }

    for order in &snapshot.orders {
        if cancelled.contains(&order.id) {
            continue;
        }
        let Some(price) = reached(order, snapshot) else {
            continue;
        };
        let symbol = order.symbol.as_str();
        let held = book.get(symbol).copied();
        if order.reduce_only {
            match held {
                Some((side, qty)) if side != order.side => {
                    out.push(Trigger { kind: TriggerKind::FillOrder, id: order.id, price: Some(price) });
                    let left = qty - order.qty.min(qty);
                    if positive(left) {
                        book.insert(symbol, (side, left));
                    } else {
                        book.remove(symbol);
                    }
                }
                _ => out.push(Trigger { kind: TriggerKind::CancelOrder, id: order.id, price: None }),
            }
            continue;
        }
        out.push(Trigger { kind: TriggerKind::FillOrder, id: order.id, price: Some(price) });
        match held {
            None => {
                book.insert(symbol, (order.side, order.qty));
            }
            Some((side, qty)) if side == order.side => {
                book.insert(symbol, (side, qty + order.qty));
            }
            Some((side, qty)) => {
                let closing = order.qty.min(qty);
                let opening = order.qty - closing;
                if positive(opening) {
                    book.insert(symbol, (order.side, opening));
                } else if positive(qty - closing) {
                    book.insert(symbol, (side, qty - closing));
                } else {
                    book.remove(symbol);
                }
            }
        }
    }
    out
}

/// What a funding time pays this position: positive received.
pub fn settlement(side: Side, qty: f64, mark: f64, rate: f64) -> f64 {
    funding(side, qty, mark, rate)
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
