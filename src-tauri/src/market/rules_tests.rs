//! The perpetual rules' tests (kept beside rules.rs so each file stays readable).

use super::*;
use Side::{Long, Short};

const TAKER: f64 = 0.00055;
const CENT: f64 = 0.01;

fn close(actual: f64, expected: f64, within: f64) {
    assert!(
        (actual - expected).abs() <= within,
        "expected {expected} within {within}, got {actual}"
    );
}

fn same(actual: f64, expected: f64) {
    close(actual, expected, 1e-9);
}

fn tier(limit: f64, mmr: f64, imr: f64, mmd: f64, max_lev: f64) -> Tier {
    Tier {
        risk_limit_value: limit,
        maintenance_margin_rate: mmr,
        initial_margin_rate: imr,
        max_leverage: max_lev,
        mm_deduction: mmd,
    }
}

fn tier_half_percent() -> Tier {
    tier(2_000_000.0, 0.005, 0.01, 0.0, 100.0)
}

/// The published BTCUSDT risk table the reference tests read: limit, mmr, imr, deduction, max leverage.
const REAL_TABLE: [(f64, f64, f64, f64, f64); 35] = [
    (300000.0, 0.0033, 0.0066, 0.0, 150.00),
    (2000000.0, 0.005, 0.01, 510.0, 100.00),
    (2600000.0, 0.0056, 0.0111, 1710.0, 90.00),
    (3200000.0, 0.0063, 0.0125, 3530.0, 80.00),
    (3800000.0, 0.0067, 0.0133, 4810.0, 75.00),
    (4400000.0, 0.0071, 0.0143, 6330.0, 70.00),
    (5000000.0, 0.0077, 0.0154, 8970.0, 65.00),
    (5600000.0, 0.0091, 0.0182, 15970.0, 55.00),
    (8500000.0, 0.01, 0.02, 21010.0, 50.00),
    (10000000.0, 0.013, 0.0222, 46510.0, 45.00),
    (14000000.0, 0.015, 0.025, 66510.0, 40.00),
    (20000000.0, 0.016, 0.0286, 80510.0, 35.00),
    (28000000.0, 0.02, 0.0333, 160510.0, 30.00),
    (38000000.0, 0.025, 0.04, 300510.0, 25.00),
    (50000000.0, 0.029, 0.05, 452510.0, 20.00),
    (55000000.0, 0.03, 0.0526, 502510.0, 19.00),
    (60000000.0, 0.035, 0.0556, 777510.0, 18.00),
    (65000000.0, 0.036, 0.0588, 837510.0, 17.00),
    (70000000.0, 0.038, 0.0625, 967510.0, 16.00),
    (75000000.0, 0.04, 0.0667, 1107510.0, 15.00),
    (80000000.0, 0.045, 0.0714, 1482510.0, 14.00),
    (85000000.0, 0.05, 0.0769, 1882510.0, 13.00),
    (90000000.0, 0.055, 0.0833, 2307510.0, 12.00),
    (95000000.0, 0.06, 0.0909, 2757510.0, 11.00),
    (100000000.0, 0.065, 0.1, 3232510.0, 10.00),
    (105000000.0, 0.07, 0.1111, 3732510.0, 9.00),
    (110000000.0, 0.085, 0.125, 5307510.0, 8.00),
    (115000000.0, 0.095, 0.1429, 6407510.0, 7.00),
    (120000000.0, 0.1, 0.1667, 6982510.0, 6.00),
    (150000000.0, 0.12, 0.2, 9382510.0, 5.00),
    (200000000.0, 0.14, 0.25, 12382510.0, 4.00),
    (250000000.0, 0.18, 0.3333, 20382510.0, 3.00),
    (400000000.0, 0.3, 0.5, 50382510.0, 2.00),
    (800000000.0, 0.42, 0.6993, 98382510.0, 1.43),
    (1200000000.0, 0.6, 1.0, 242382510.0, 1.00),
];

fn real_tiers() -> Vec<Tier> {
    REAL_TABLE
        .iter()
        .map(|&(limit, mmr, imr, mmd, lev)| tier(limit, mmr, imr, mmd, lev))
        .collect()
}

/// The published BTCUSDT instrument rules.
fn real_rules() -> Rules {
    Rules {
        tick_size: 0.10,
        qty_step: 0.001,
        min_qty: 0.001,
        max_qty: 1500.0,
        max_market_qty: 150.0,
        min_notional: 5.0,
        min_leverage: 1.0,
        max_leverage: 150.0,
        leverage_step: 0.01,
    }
}

// ---- the exchange's own worked examples

#[test]
fn published_long_liquidation_example() {
    let liq = liquidation_price(Long, 40000.0, 1.0, 50.0, 3000.0, &tier_half_percent(), TAKER);
    close(liq, 36380.25, CENT);
}

#[test]
fn published_fee_to_close_example() {
    close(fee_to_close(Long, 1.0, 40000.0, 50.0, TAKER), 21.56, CENT);
}

#[test]
fn published_bankruptcy_example() {
    same(bankruptcy_price(Long, 60000.0, 1.0, 50.0, 0.0), 58800.0);
}

#[test]
fn published_funding_example() {
    same(funding(Long, 1.0, 10000.0, 0.0001), -1.0);
    same(funding(Short, 1.0, 10000.0, 0.0001), 1.0);
}

// ---- liquidation: the loss at the price leaves exactly the maintenance margin

#[test]
fn at_the_liquidation_price_the_loss_is_the_margin_less_maintenance() {
    let t = tier(2_000_000.0, 0.005, 0.01, 510.0, 100.0);
    for side in [Long, Short] {
        let (entry, qty, leverage, extra) = (80000.0, 10.0, 20.0, 1000.0);
        let liq = liquidation_price(side, entry, qty, leverage, extra, &t, TAKER);
        let factor = if side == Long { 1.0 - TAKER } else { 1.0 + TAKER };
        let margin = initial_margin(notional(qty, entry), leverage) + extra / factor;
        let maintenance = maintenance_margin(notional(qty, liq), &t);
        let loss = -unrealised_pnl(side, entry, liq, qty);
        close(loss, margin - maintenance, CENT);
    }
}

#[test]
fn a_long_is_liquidated_below_entry_and_a_short_above() {
    let entry = 80000.0;
    let t = tier_half_percent();
    let long_liq = liquidation_price(Long, entry, 0.1, 10.0, 0.0, &t, TAKER);
    let short_liq = liquidation_price(Short, entry, 0.1, 10.0, 0.0, &t, TAKER);
    assert!(long_liq < entry);
    assert!(short_liq > entry);
    // Liquidation comes before the margin is all gone
    assert!(long_liq > bankruptcy_price(Long, entry, 0.1, 10.0, 0.0));
    assert!(short_liq < bankruptcy_price(Short, entry, 0.1, 10.0, 0.0));
}

#[test]
fn extra_margin_pushes_the_liquidation_price_away() {
    let t = tier_half_percent();
    let without = liquidation_price(Long, 80000.0, 0.1, 10.0, 0.0, &t, TAKER);
    let with = liquidation_price(Long, 80000.0, 0.1, 10.0, 100.0, &t, TAKER);
    assert!(with < without);
}

#[test]
fn bankruptcy_counts_extra_margin() {
    same(bankruptcy_price(Long, 80000.0, 0.1, 10.0, 100.0), 71000.0);
    same(bankruptcy_price(Short, 80000.0, 0.1, 10.0, 100.0), 89000.0);
}

#[test]
fn a_tiny_leverage_long_cannot_go_below_zero() {
    same(bankruptcy_price(Long, 100.0, 1.0, 1.0, 500.0), 0.0);
    same(liquidation_price(Long, 100.0, 1.0, 1.0, 500.0, &tier_half_percent(), TAKER), 0.0);
}

#[test]
fn the_liquidation_tick_rounds_against_the_trader() {
    same(liquidation_on_tick(Long, 36380.2503, 0.10), 36380.3);
    same(liquidation_on_tick(Short, 36380.2503, 0.10), 36380.2);
    // already on the tick: stays
    same(liquidation_on_tick(Long, 36380.3, 0.10), 36380.3);
    same(liquidation_on_tick(Short, 36380.3, 0.10), 36380.3);
}

#[test]
fn prices_and_sizes_move_onto_their_steps() {
    same(price_on_tick(80000.14, 0.10), 80000.1);
    same(price_on_tick(80000.16, 0.10), 80000.2);
    same(qty_down_to_step(0.0159, 0.001), 0.015);
    same(qty_down_to_step(0.015, 0.001), 0.015);
    same(qty_down_to_step(0.0009, 0.001), 0.0);
}

// ---- margins against the published BTCUSDT table

#[test]
fn the_real_table_carries_its_deductions_up_every_rung() {
    let tiers = real_tiers();
    assert_eq!(tiers.len(), 35);
    for i in 1..tiers.len() {
        let below = &tiers[i - 1];
        let t = &tiers[i];
        let expected = below.mm_deduction
            + (t.maintenance_margin_rate - below.maintenance_margin_rate) * below.risk_limit_value;
        close(t.mm_deduction, expected, 1e-4);
    }
}

#[test]
fn maintenance_margin_is_continuous_across_a_rung() {
    let tiers = real_tiers();
    let limit = tiers[0].risk_limit_value;
    let at_top = maintenance_margin(limit, &tiers[0]);
    let from_next = maintenance_margin(limit, &tiers[1]);
    same(at_top, from_next);
    same(at_top, 990.0);
}

#[test]
fn tier_for_picks_the_lowest_covering_rung() {
    let tiers = real_tiers();
    same(tier_for(300000.0, &tiers).unwrap().max_leverage, 150.0);
    same(tier_for(300000.01, &tiers).unwrap().max_leverage, 100.0);
    same(tier_for(99999999999.0, &tiers).unwrap().risk_limit_value, 1200000000.0);
    assert!(tier_for(1.0, &[]).is_none());
}

// ---- costs and profit

#[test]
fn order_cost_is_margin_plus_both_fees() {
    // 0.1 at 80,000 at 10x: margin 800, opening fee 4.40, closing fee 0.1 x 80,000 x 0.9 x 0.055 % = 3.96
    same(order_cost(Long, 0.1, 80000.0, 10.0, TAKER, TAKER), 808.36);
    // The same order resting as a maker opens at the maker rate
    same(order_cost(Long, 0.1, 80000.0, 10.0, 0.0002, TAKER), 805.56);
    // A short keeps more for closing, because the price it closes at is higher
    same(order_cost(Short, 0.1, 80000.0, 10.0, TAKER, TAKER), 809.24);
    // The fee role picks the opening rate; nothing opening costs nothing
    let fees = Fees { taker: TAKER, maker: 0.0002 };
    same(opening_cost(Long, 0.1, 80000.0, 10.0, FeeRole::Maker, &fees), 805.56);
    same(opening_cost(Long, 0.0, 80000.0, 10.0, FeeRole::Taker, &fees), 0.0);
}

#[test]
fn pnl_has_the_right_sign() {
    same(unrealised_pnl(Long, 80000.0, 81000.0, 0.1), 100.0);
    same(unrealised_pnl(Long, 80000.0, 79000.0, 0.1), -100.0);
    same(unrealised_pnl(Short, 80000.0, 81000.0, 0.1), -100.0);
    same(unrealised_pnl(Short, 80000.0, 79000.0, 0.1), 100.0);
}

#[test]
fn average_entry_is_weighted_by_size() {
    same(average_entry(1.0, 80000.0, 3.0, 84000.0), 83000.0);
}

#[test]
fn negative_funding_has_shorts_paying_longs() {
    same(funding(Long, 1.0, 10000.0, -0.0001), 1.0);
    same(funding(Short, 1.0, 10000.0, -0.0001), -1.0);
}

// ---- the book

fn level(price: f64, size: f64) -> BookLevel {
    BookLevel { price, size }
}

#[test]
fn walking_the_book_averages_across_levels() {
    let asks = [level(100.0, 1.0), level(101.0, 1.0), level(110.0, 5.0)];
    let walk = walk_book(&asks, 3.0);
    assert!(walk.complete);
    same(walk.filled_qty, 3.0);
    close(walk.average_price, 103.6666666667, 1e-9);
}

#[test]
fn a_size_within_the_top_level_has_no_slippage() {
    let walk = walk_book(&[level(100.0, 1.0)], 0.5);
    same(walk.average_price, 100.0);
    assert!(walk.complete);
}

#[test]
fn a_thin_book_says_so() {
    let walk = walk_book(&[level(100.0, 1.0)], 2.0);
    assert!(!walk.complete);
    same(walk.filled_qty, 1.0);
    assert!(!walk_book(&[], 1.0).complete);
    same(walk_book(&[], 1.0).average_price, 0.0);
}

// ---- cross margin: the account as one

fn account(
    wallet: f64,
    upl: f64,
    initial: f64,
    maintenance: f64,
    orders: f64,
    isolated: f64,
    isolated_orders: f64,
) -> AccountFigures {
    AccountFigures {
        wallet_balance: wallet,
        cross_unrealised_pnl: upl,
        cross_initial_margin: initial,
        cross_maintenance_margin: maintenance,
        cross_order_cost: orders,
        isolated_locked: isolated,
        isolated_order_cost: isolated_orders,
    }
}

#[test]
fn one_winner_pays_for_one_loser() {
    let tiers = real_tiers();
    // Marked at 75,000: one position from 70,000 is up 500, another from 79,000 is down 400
    let winner = unrealised_pnl(Long, 70000.0, 75000.0, 0.1);
    let loser = unrealised_pnl(Long, 79000.0, 75000.0, 0.1);
    same(winner, 500.0);
    same(loser, -400.0);

    // The loser alone takes this small wallet past the line; the winner holds it up
    let maintenance =
        position_maintenance(Long, 0.1, 79000.0, 75000.0, 10.0, &tiers, TAKER).unwrap();
    let alone = account(420.0, loser, 790.0, maintenance, 0.0, 0.0, 0.0);
    assert!(cross_liquidation_due(&alone));

    let both = account(420.0, loser + winner, 790.0, maintenance, 0.0, 0.0, 0.0);
    same(cross_margin_balance(&both), 520.0);
    assert!(!cross_liquidation_due(&both));
}

#[test]
fn cross_liquidation_fires_at_one_hundred_percent() {
    let tiers = real_tiers();
    // 1,000 wallet, 0.1 long from 80,000 at 10x
    let entry = 80000.0;

    let safe_mark = 75000.0;
    let safe_mm = position_maintenance(Long, 0.1, entry, safe_mark, 10.0, &tiers, TAKER).unwrap();
    same(safe_mm, 28.71);
    let safe = account(1000.0, unrealised_pnl(Long, entry, safe_mark, 0.1), 800.0, safe_mm, 0.0, 0.0, 0.0);
    assert!(!cross_liquidation_due(&safe));

    let dead_mark = 70100.0;
    let dead_mm = position_maintenance(Long, 0.1, entry, dead_mark, 10.0, &tiers, TAKER).unwrap();
    let dead = account(1000.0, unrealised_pnl(Long, entry, dead_mark, 0.1), 800.0, dead_mm, 0.0, 0.0, 0.0);
    same(cross_margin_balance(&dead), 10.0);
    assert!(account_maintenance_rate(&dead) > 1.0);
    assert!(cross_liquidation_due(&dead));
}

#[test]
fn a_balance_below_zero_has_passed_the_line() {
    let gone = account(1000.0, -1500.0, 800.0, 30.0, 0.0, 0.0, 0.0);
    same(account_maintenance_rate(&gone), 1.0);
    assert!(cross_liquidation_due(&gone));
    same(available_balance(&gone), 0.0);
}

#[test]
fn no_cross_position_means_no_liquidation() {
    let empty = account(1000.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    same(account_maintenance_rate(&empty), 0.0);
    same(account_initial_rate(&empty), 0.0);
    assert!(!cross_liquidation_due(&empty));
    same(available_balance(&empty), 1000.0);
    // No cross requirement at all, on an empty wallet too
    assert!(!cross_liquidation_due(&account(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)));
}

#[test]
fn unrealised_profit_moves_what_is_available() {
    same(available_balance(&account(1000.0, 0.0, 800.0, 30.0, 0.0, 0.0, 0.0)), 200.0);
    same(available_balance(&account(1000.0, -150.0, 800.0, 30.0, 0.0, 0.0, 0.0)), 50.0);
    same(available_balance(&account(1000.0, -300.0, 800.0, 30.0, 0.0, 0.0, 0.0)), 0.0);
    same(available_balance(&account(1000.0, 250.0, 800.0, 30.0, 0.0, 0.0, 0.0)), 450.0);
}

#[test]
fn resting_orders_and_isolated_money_hold_their_share() {
    same(available_balance(&account(1000.0, 0.0, 500.0, 20.0, 100.0, 200.0, 50.0)), 150.0);
}

#[test]
fn orders_cancel_at_one_hundred_percent_initial_margin_before_positions_close() {
    // Balance 700 against 500 of margin + 300 of orders: orders go, the position stays
    let stretched = account(1000.0, -300.0, 500.0, 20.0, 300.0, 0.0, 0.0);
    assert!(cross_orders_must_cancel(&stretched));
    assert!(!cross_liquidation_due(&stretched));
    // No orders, nothing to cancel, however stretched
    assert!(!cross_orders_must_cancel(&account(1000.0, -300.0, 1200.0, 20.0, 0.0, 0.0, 0.0)));
}

#[test]
fn an_isolated_position_is_walled_off_from_cross_and_back() {
    // 900 of the wallet sits in an isolated position; cross has only the other 100
    let walled = account(1000.0, -80.0, 50.0, 27.0, 0.0, 900.0, 0.0);
    same(cross_margin_balance(&walled), 20.0);
    assert!(cross_liquidation_due(&walled));
    // With nothing in cross, a large isolated position alone never fires the account line
    assert!(!cross_liquidation_due(&account(1000.0, 0.0, 0.0, 0.0, 0.0, 900.0, 0.0)));
}

#[test]
fn the_cross_estimate_is_where_the_account_reaches_the_line() {
    let t = tier(300000.0, 0.0033, 0.0066, 0.0, 150.0);
    let (entry, qty, wallet, closing) = (80000.0, 0.1, 1000.0, 3.96);
    for side in [Long, Short] {
        let price = cross_liquidation_estimate(side, qty, entry, wallet, &t, closing);
        let balance = wallet + unrealised_pnl(side, entry, price, qty);
        let required = maintenance_margin(notional(qty, price), &t) + closing;
        close(balance, required, CENT);
    }
}

#[test]
fn isolated_locked_is_margin_extra_and_the_closing_fee() {
    // 0.1 at 80,000 at 10x: margin 800, extra 50, closing fee 3.96
    same(isolated_locked(Long, 0.1, 80000.0, 10.0, 50.0, TAKER), 853.96);
}

// ---- order checks against the published BTCUSDT rules

fn check(qty: f64, price: Option<f64>, leverage: f64) -> Result<(), String> {
    check_order(&real_rules(), &real_tiers(), qty, price, 80000.0, leverage, 0.0)
}

fn says(outcome: Result<(), String>, words: &str) {
    match outcome {
        Err(message) => assert!(message.contains(words), "'{message}' should say '{words}'"),
        Ok(()) => panic!("expected a refusal saying '{words}'"),
    }
}

#[test]
fn a_good_order_passes() {
    assert_eq!(check(0.010, None, 10.0), Ok(()));
    assert_eq!(check(0.010, Some(80000.1), 10.0), Ok(()));
    assert_eq!(check(0.010, None, 10.25), Ok(()));
}

#[test]
fn bad_orders_say_why_in_plain_words() {
    says(check(0.0, None, 10.0), "above zero");
    says(check(0.0015, None, 10.0), "steps of 0.001");
    says(check(0.010, Some(80000.15), 10.0), "steps of 0.1");
    says(check(0.010, Some(-1.0), 10.0), "price must be above zero");
    says(check(0.010, None, 151.0), "between 1x and 150x");
    says(check(0.010, None, 0.5), "between 1x and 150x");
    says(check(0.010, None, 10.005), "steps of 0.01");
    says(check(0.010, None, f64::NAN), "between");
}

#[test]
fn an_order_below_the_smallest_quantity_is_refused() {
    let rules = Rules { min_qty: 0.01, ..real_rules() };
    says(
        check_order(&rules, &real_tiers(), 0.005, None, 80000.0, 10.0, 0.0),
        "The smallest quantity is 0.01",
    );
}

#[test]
fn an_order_above_the_largest_leverage_is_refused() {
    says(check(0.010, Some(80000.0), 151.0), "The leverage must be between 1x and 150x");
    assert_eq!(check_leverage(&real_rules(), 150.0), Ok(()));
}

#[test]
fn the_smallest_order_value_counts() {
    // 0.001 BTC at 1,000 is 1 USDT, below the 5 USDT minimum
    says(check(0.001, Some(1000.0), 10.0), "smallest order value is 5");
}

#[test]
fn a_market_order_has_its_own_largest_size() {
    let refused = check(151.0, None, 1.0);
    says(refused.clone(), "largest quantity");
    says(refused, "150");
    assert_eq!(check(151.0, Some(80000.0), 1.0), Ok(()));
}

#[test]
fn a_bigger_position_caps_the_leverage() {
    // 5 BTC at 80,000 is 400,000: past the first rung, so 150x is out and 100x is the most
    says(check(5.0, None, 150.0), "at most 100x");
    assert_eq!(check(5.0, None, 100.0), Ok(()));
}

#[test]
fn the_existing_position_counts_toward_the_cap() {
    let problem = check_order(&real_rules(), &real_tiers(), 1.0, None, 80000.0, 150.0, 250000.0);
    says(problem, "at most 100x");
}

#[test]
fn a_position_cannot_pass_the_top_of_the_table() {
    let problem =
        check_order(&real_rules(), &real_tiers(), 1.0, None, 80000.0, 1.0, 1200000000.0);
    says(problem, "largest allowed value");
}

// ---- the judge

const BTC: &str = "BTCUSDT";

fn quote(last: f64, mark: f64, bid: f64, ask: f64) -> Quote {
    Quote { last, mark, bid, ask }
}

fn flat(price: f64) -> Quote {
    quote(price, price, price - 0.1, price + 0.1)
}

fn position(id: u64, side: Side, mode: MarginMode) -> Position {
    Position {
        id,
        symbol: BTC.into(),
        side,
        qty: 0.1,
        entry: 80000.0,
        leverage: 10.0,
        mode,
        extra_margin: 0.0,
        tp: None,
        sl: None,
    }
}

fn limit(id: u64, side: Side, price: f64) -> RestingOrder {
    RestingOrder {
        id,
        symbol: BTC.into(),
        side,
        qty: 0.1,
        price: Some(price),
        reduce_only: false,
        mode: MarginMode::Cross,
        reserved_cost: 808.36,
    }
}

fn reduce_only(id: u64, side: Side, price: f64) -> RestingOrder {
    RestingOrder { reduce_only: true, reserved_cost: 0.0, ..limit(id, side, price) }
}

fn snapshot(wallet: f64, positions: Vec<Position>, orders: Vec<RestingOrder>, q: Quote) -> Snapshot {
    let mut quotes = HashMap::new();
    quotes.insert(BTC.to_string(), q);
    let mut instruments = HashMap::new();
    instruments.insert(BTC.to_string(), Instrument { rules: real_rules(), tiers: real_tiers() });
    Snapshot {
        wallet_balance: wallet,
        positions,
        orders,
        quotes,
        instruments,
        fees: Fees { taker: TAKER, maker: 0.0002 },
    }
}

fn fired(kind: TriggerKind, id: u64, price: Option<f64>) -> Trigger {
    Trigger { kind, id, price }
}

#[test]
fn an_isolated_long_is_liquidated_when_the_mark_reaches_its_price() {
    let p = position(1, Long, MarginMode::Isolated);
    // (8,000 - 800) / (0.1 - 0.1 x 0.33 %) = 72,238.387..., on the tick against the trader: 72,238.4
    let s = snapshot(10000.0, vec![p.clone()], vec![], flat(75000.0));
    same(ending_price(&p, &s).unwrap(), 72238.4);
    assert!(triggers(&s).is_empty());

    let above = snapshot(10000.0, vec![p.clone()], vec![], flat(72238.5));
    assert!(triggers(&above).is_empty());

    // It closes at its bankruptcy price, 80,000 x (1 - 1/10)
    let at = snapshot(10000.0, vec![p.clone()], vec![], flat(72238.4));
    assert_eq!(triggers(&at), vec![fired(TriggerKind::Liquidate, 1, Some(72000.0))]);
}

#[test]
fn an_isolated_short_is_liquidated_on_the_way_up() {
    let p = position(2, Short, MarginMode::Isolated);
    let s = snapshot(10000.0, vec![p.clone()], vec![], flat(80000.0));
    let ending = ending_price(&p, &s).unwrap();
    assert!(ending > 80000.0);
    let past = snapshot(10000.0, vec![p], vec![], flat(ending + 1.0));
    assert_eq!(triggers(&past), vec![fired(TriggerKind::Liquidate, 2, Some(88000.0))]);
}

#[test]
fn the_liquidation_reads_the_mark_not_the_last() {
    let p = position(1, Long, MarginMode::Isolated);
    // last has crashed through, mark has not: nothing
    let s = snapshot(10000.0, vec![p], vec![], quote(70000.0, 75000.0, 69999.9, 70000.1));
    assert!(triggers(&s).is_empty());
}

#[test]
fn a_cross_account_past_its_line_loses_everything_cross_at_the_mark() {
    let p = position(1, Long, MarginMode::Cross);
    let order = limit(7, Long, 60000.0);
    let safe = snapshot(1000.0, vec![p.clone()], vec![], flat(75000.0));
    assert!(triggers(&safe).is_empty());

    let dead = snapshot(1000.0, vec![p], vec![order], quote(70100.0, 70100.0, 70000.0, 70200.0));
    assert_eq!(
        triggers(&dead),
        vec![
            fired(TriggerKind::Liquidate, 1, Some(70100.0)),
            fired(TriggerKind::CancelOrder, 7, None),
        ]
    );
}

#[test]
fn a_stretched_account_loses_its_orders_before_its_position() {
    // Marked at 75,000: balance 500 against 800 of margin plus an order holding margin
    let p = position(1, Long, MarginMode::Cross);
    let order = limit(7, Long, 60000.0);
    let s = snapshot(1000.0, vec![p], vec![order], flat(75000.0));
    assert_eq!(triggers(&s), vec![fired(TriggerKind::CancelOrder, 7, None)]);
}

#[test]
fn take_profit_and_stop_loss_for_a_long() {
    let p = Position { tp: Some(85000.0), sl: Some(78000.0), ..position(1, Long, MarginMode::Isolated) };
    let quiet = snapshot(10000.0, vec![p.clone()], vec![], flat(80000.0));
    assert!(triggers(&quiet).is_empty());
    let up = snapshot(10000.0, vec![p.clone()], vec![], flat(85000.0));
    assert_eq!(triggers(&up), vec![fired(TriggerKind::TakeProfit, 1, Some(85000.0))]);
    let down = snapshot(10000.0, vec![p], vec![], flat(77950.0));
    assert_eq!(triggers(&down), vec![fired(TriggerKind::StopLoss, 1, Some(78000.0))]);
}

#[test]
fn take_profit_and_stop_loss_for_a_short() {
    let p = Position { tp: Some(75000.0), sl: Some(82000.0), ..position(1, Short, MarginMode::Isolated) };
    let down = snapshot(10000.0, vec![p.clone()], vec![], flat(74900.0));
    assert_eq!(triggers(&down), vec![fired(TriggerKind::TakeProfit, 1, Some(75000.0))]);
    let up = snapshot(10000.0, vec![p], vec![], flat(82000.0));
    assert_eq!(triggers(&up), vec![fired(TriggerKind::StopLoss, 1, Some(82000.0))]);
}

#[test]
fn take_profit_reads_the_last_price_and_the_stop_wins_a_tie() {
    let p = Position { tp: Some(85000.0), ..position(1, Long, MarginMode::Isolated) };
    // the mark is there, the last is not: nothing
    let s = snapshot(10000.0, vec![p], vec![], quote(84000.0, 85500.0, 83999.9, 84000.1));
    assert!(triggers(&s).is_empty());

    // both crossed at once: the stop first, and only once
    let odd = Position { tp: Some(79000.0), sl: Some(81000.0), ..position(1, Long, MarginMode::Isolated) };
    let s = snapshot(10000.0, vec![odd], vec![], flat(80000.0));
    assert_eq!(triggers(&s), vec![fired(TriggerKind::StopLoss, 1, Some(81000.0))]);
}

#[test]
fn a_buy_limit_fills_only_when_the_ask_reaches_it() {
    let order = limit(5, Long, 79000.0);
    let above = snapshot(10000.0, vec![], vec![order.clone()], quote(79000.0, 79000.0, 78999.9, 79000.1));
    assert!(triggers(&above).is_empty());
    // the last trade alone is not enough: a buy fills against the ask
    let last_only = snapshot(10000.0, vec![], vec![order.clone()], quote(78000.0, 79500.0, 79499.9, 79500.0));
    assert!(triggers(&last_only).is_empty());
    let at = snapshot(10000.0, vec![], vec![order], quote(79000.1, 79000.0, 78999.9, 79000.0));
    assert_eq!(triggers(&at), vec![fired(TriggerKind::FillOrder, 5, Some(79000.0))]);
}

#[test]
fn a_sell_limit_fills_on_the_way_up_by_the_bid() {
    let order = limit(6, Short, 81000.0);
    let below = snapshot(10000.0, vec![], vec![order.clone()], quote(81000.0, 81000.0, 80999.9, 81000.1));
    assert!(triggers(&below).is_empty());
    let at = snapshot(10000.0, vec![], vec![order], quote(81000.0, 81000.0, 81000.0, 81000.1));
    assert_eq!(triggers(&at), vec![fired(TriggerKind::FillOrder, 6, Some(81000.0))]);
}

#[test]
fn a_reduce_only_order_with_nothing_left_to_reduce_is_cancelled() {
    let order = reduce_only(8, Short, 81000.0);
    let reached = quote(81000.0, 81000.0, 81000.0, 81000.1);

    // the position is gone: cancelled when the price comes to it
    let gone = snapshot(10000.0, vec![], vec![order.clone()], reached);
    assert_eq!(triggers(&gone), vec![fired(TriggerKind::CancelOrder, 8, None)]);

    // not reached yet: left alone
    let early = snapshot(10000.0, vec![], vec![order.clone()], flat(80000.0));
    assert!(triggers(&early).is_empty());

    // a long to reduce: it fills
    let held = position(1, Long, MarginMode::Isolated);
    let s = snapshot(10000.0, vec![held.clone()], vec![order.clone()], reached);
    assert_eq!(triggers(&s), vec![fired(TriggerKind::FillOrder, 8, Some(81000.0))]);

    // the long was taken by its own take profit in this same moment: nothing left to reduce
    let taken = Position { tp: Some(81000.0), ..held };
    let s = snapshot(10000.0, vec![taken], vec![order.clone()], reached);
    assert_eq!(
        triggers(&s),
        vec![fired(TriggerKind::TakeProfit, 1, Some(81000.0)), fired(TriggerKind::CancelOrder, 8, None)]
    );

    // a position on the same side is nothing to reduce either
    let same_side = position(1, Short, MarginMode::Isolated);
    let s = snapshot(10000.0, vec![same_side], vec![order], reached);
    assert_eq!(triggers(&s), vec![fired(TriggerKind::CancelOrder, 8, None)]);
}

#[test]
fn a_second_reduce_only_order_finds_the_first_already_closed_it() {
    let held = position(1, Long, MarginMode::Isolated);
    let first = reduce_only(8, Short, 81000.0);
    let second = reduce_only(9, Short, 81000.0);
    let s = snapshot(10000.0, vec![held], vec![first, second], quote(81000.0, 81000.0, 81000.0, 81000.1));
    assert_eq!(
        triggers(&s),
        vec![fired(TriggerKind::FillOrder, 8, Some(81000.0)), fired(TriggerKind::CancelOrder, 9, None)]
    );
}

#[test]
fn no_price_or_no_rules_means_no_judgement() {
    let p = Position { sl: Some(90000.0), ..position(1, Long, MarginMode::Cross) };
    let mut s = snapshot(10000.0, vec![p.clone()], vec![limit(5, Long, 90000.0)], flat(80000.0));
    assert_eq!(triggers(&s).len(), 2);

    s.quotes.clear();
    assert!(triggers(&s).is_empty());

    let mut s = snapshot(10000.0, vec![p], vec![], flat(80000.0));
    s.instruments.clear();
    assert!(triggers(&s).is_empty());
    assert_eq!(account_figures(&s), None);
}

#[test]
fn a_cross_ending_price_is_where_the_account_reaches_the_line() {
    let p = position(1, Long, MarginMode::Cross);
    let s = snapshot(1000.0, vec![p.clone()], vec![], flat(75000.0));
    let ending = ending_price(&p, &s).unwrap();
    assert!(ending < 80000.0 && ending > 70100.0);
    // just past it, the account line fires
    let past = snapshot(1000.0, vec![p.clone()], vec![], flat(ending - 1.0));
    assert_eq!(triggers(&past)[0].kind, TriggerKind::Liquidate);
    // well before it, nothing
    let before = snapshot(1000.0, vec![p], vec![], flat(ending + 50.0));
    assert!(triggers(&before).is_empty());
}

#[test]
fn the_snapshot_adds_up_the_account() {
    let cross = position(1, Long, MarginMode::Cross);
    let isolated = Position { symbol: BTC.into(), extra_margin: 50.0, ..position(2, Long, MarginMode::Isolated) };
    let s = snapshot(2000.0, vec![cross, isolated], vec![limit(5, Long, 60000.0)], flat(80000.0));
    let a = account_figures(&s).unwrap();
    same(a.cross_initial_margin, 800.0);
    same(a.isolated_locked, 853.96);
    same(a.cross_order_cost, 808.36);
    same(a.cross_maintenance_margin, 26.4 + 3.96);
    // 2,000 - 853.96 isolated = 1,146.04 of balance, less 800 of margin and 808.36 of orders: none left
    same(available(&s), 0.0);
    assert!(account_risk(&s) > 0.0);

    let roomy = snapshot(5000.0, vec![position(1, Long, MarginMode::Cross)], vec![], flat(80000.0));
    same(available(&roomy), 4200.0);
}

#[test]
fn the_words_travel_in_capitals() {
    assert_eq!(serde_json::to_string(&TriggerKind::TakeProfit).unwrap(), "\"TAKE_PROFIT\"");
    assert_eq!(serde_json::to_string(&Side::Long).unwrap(), "\"LONG\"");
    assert_eq!(serde_json::from_str::<MarginMode>("\"ISOLATED\"").unwrap(), MarginMode::Isolated);
    assert_eq!(Side::Long.opposite(), Side::Short);
}
