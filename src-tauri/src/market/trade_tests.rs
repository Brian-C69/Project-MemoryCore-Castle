//! The trade door's tests (kept beside trade.rs so each file stays readable).

use super::*;
use Side::{Long, Short};

const TAKER: f64 = 0.00055;
const MAKER: f64 = 0.0002;

fn same(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-9, "expected {expected}, got {actual}");
}

fn held(side: Side, qty: f64, entry: f64) -> Option<Held> {
    Some(Held { side, qty, entry })
}

fn input(side: &str, kind: &str, qty: f64, price: Option<f64>) -> OrderInput {
    OrderInput {
        symbol: " btcusdt ".into(),
        side: side.into(),
        order_type: kind.into(),
        qty,
        price,
        tp: None,
        sl: None,
        reduce_only: false,
        client_order_id: "c-1".into(),
    }
}

// ---- netting in one-way mode

#[test]
fn a_fill_on_a_flat_symbol_opens_a_position_and_pays_its_fee() {
    let plan = plan_fill(None, Long, 0.5, 60_000.0, TAKER, false);
    same(plan.closing, 0.0);
    same(plan.opening, 0.5);
    same(plan.open_fee, 30_000.0 * TAKER);
    assert_eq!(plan.after, held(Long, 0.5, 60_000.0));
    assert_eq!(plan.ledger(), vec![("FEE", -16.5)]);
}

#[test]
fn adding_to_a_position_averages_the_entry() {
    let plan = plan_fill(held(Long, 1.0, 100.0), Long, 1.0, 110.0, TAKER, false);
    same(plan.closing, 0.0);
    let after = plan.after.unwrap();
    same(after.qty, 2.0);
    same(after.entry, 105.0);
}

#[test]
fn reducing_realises_the_profit_on_the_closed_part_only() {
    let plan = plan_fill(held(Long, 2.0, 100.0), Short, 0.5, 120.0, TAKER, false);
    same(plan.closing, 0.5);
    same(plan.realised, 10.0);
    same(plan.close_fee, 60.0 * TAKER);
    same(plan.opening, 0.0);
    assert_eq!(plan.after, held(Long, 1.5, 100.0), "the entry of what is left does not move");
    assert_eq!(plan.ledger(), vec![("REALISED", 10.0), ("FEE", -0.033)]);
}

#[test]
fn closing_exactly_leaves_nothing() {
    let plan = plan_fill(held(Short, 1.0, 100.0), Long, 1.0, 90.0, TAKER, false);
    same(plan.realised, 10.0);
    assert_eq!(plan.after, None);
}

#[test]
fn a_larger_opposite_fill_flips_the_position_at_the_fill_price() {
    let plan = plan_fill(held(Long, 1.0, 100.0), Short, 3.0, 90.0, MAKER, false);
    same(plan.closing, 1.0);
    same(plan.realised, -10.0);
    same(plan.opening, 2.0);
    same(plan.close_fee, 90.0 * MAKER);
    same(plan.open_fee, 180.0 * MAKER);
    assert_eq!(plan.after, held(Short, 2.0, 90.0));
    let lines = plan.ledger();
    assert_eq!(lines.iter().map(|(k, _)| *k).collect::<Vec<_>>(), ["REALISED", "FEE", "FEE"]);
    same(plan.balance_change(), -10.0 - 270.0 * MAKER);
}

#[test]
fn reduce_only_never_opens_the_other_way() {
    let plan = plan_fill(held(Long, 1.0, 100.0), Short, 3.0, 90.0, TAKER, true);
    same(plan.closing, 1.0);
    same(plan.opening, 0.0);
    assert_eq!(plan.after, None);
    let nothing = plan_fill(None, Short, 1.0, 90.0, TAKER, true);
    assert!(nothing.ledger().is_empty(), "nothing to reduce writes nothing");
}

#[test]
fn a_short_profits_when_the_price_falls() {
    let plan = plan_fill(held(Short, 2.0, 50.0), Long, 2.0, 40.0, 0.0, false);
    same(plan.realised, 20.0);
    same(plan.balance_change(), 20.0);
}

// ---- the wallet

#[test]
fn the_wallet_stops_at_zero_with_a_floor_line() {
    assert_eq!(floor_line(-12.5), Some(12.5));
    assert_eq!(floor_line(0.0), None);
    assert_eq!(floor_line(3.0), None);
}

#[test]
fn an_account_gives_way_only_when_nothing_is_left() {
    assert!(nothing_left(0.4, 0, 0));
    assert!(!nothing_left(0.4, 1, 0));
    assert!(!nothing_left(0.4, 0, 1));
    assert!(!nothing_left(100.0, 0, 0));
}

#[test]
fn paper_money_is_above_zero_at_most_a_billion_with_two_decimals() {
    assert_eq!(check_start(1000.0), Ok(1000.0));
    assert_eq!(check_start(10.25), Ok(10.25));
    assert_eq!(check_start(1_000_000_000.0), Ok(1_000_000_000.0));
    assert!(check_start(0.0).is_err());
    assert!(check_start(-5.0).is_err());
    assert!(check_start(1_000_000_000.01).is_err());
    assert!(check_start(10.255).is_err());
    assert!(check_start(f64::NAN).is_err());
}

#[test]
fn fee_rates_stay_between_zero_and_one_percent() {
    assert!(check_fees(0.00055, 0.0002).is_ok());
    assert!(check_fees(0.0, 0.01).is_ok());
    assert!(check_fees(0.011, 0.0).is_err());
    assert!(check_fees(0.001, -0.0001).is_err());
}

// ---- reading an order

#[test]
fn an_order_is_read_in_plain_words() {
    let a = read_input(&input("long", "market", 0.01, Some(5.0))).unwrap();
    assert_eq!(a.symbol, "BTCUSDT");
    assert_eq!(a.side, Long);
    assert_eq!(a.kind, OrderType::Market);
    assert_eq!(a.price, None, "a market order's price is ignored");
    assert!(read_input(&input("LONG", "LIMIT", 0.01, None)).is_err(), "a limit needs a price");
    assert!(read_input(&input("UP", "LIMIT", 0.01, Some(1.0))).is_err());
    assert!(read_input(&input("SHORT", "LIMIT", 0.0, Some(1.0))).is_err());
    let mut no_id = input("SHORT", "LIMIT", 1.0, Some(1.0));
    no_id.client_order_id = "  ".into();
    assert!(read_input(&no_id).is_err());
}

#[test]
fn the_screen_sends_camel_case() {
    let json = r#"{"symbol":"ETHUSDT","side":"SHORT","orderType":"LIMIT","qty":0.1,"price":3000.5,"reduceOnly":true,"clientOrderId":"x"}"#;
    let order: OrderInput = serde_json::from_str(json).unwrap();
    assert_eq!(order.order_type, "LIMIT");
    assert!(order.reduce_only);
    assert_eq!(order.tp, None);
}

#[test]
fn exits_sit_on_the_right_side_and_on_the_tick() {
    assert!(check_exits(Long, Some(110.0), Some(90.0), 100.0, 0.1).is_ok());
    assert!(check_exits(Long, Some(95.0), None, 100.0, 0.1).is_err());
    assert!(check_exits(Long, None, Some(105.0), 100.0, 0.1).is_err());
    assert!(check_exits(Short, Some(90.0), Some(110.0), 100.0, 0.1).is_ok());
    assert!(check_exits(Short, Some(105.0), None, 100.0, 0.1).is_err());
    assert!(check_exits(Long, Some(110.05), None, 100.0, 0.1).is_err(), "off the tick");
    assert!(check_exits(Long, None, None, 100.0, 0.1).is_ok());
}

#[test]
fn a_limit_crosses_when_it_reaches_the_other_side() {
    assert!(crosses(Long, 101.0, 99.0, 100.0));
    assert!(crosses(Long, 100.0, 99.0, 100.0));
    assert!(!crosses(Long, 99.5, 99.0, 100.0));
    assert!(crosses(Short, 99.0, 99.0, 100.0));
    assert!(!crosses(Short, 99.5, 99.0, 100.0));
}

#[test]
fn a_buy_walks_the_asks_and_a_limit_stops_at_its_price() {
    let level = |price: f64, qty: f64| feed::BookLevel { price, qty };
    let book = feed::Book { bids: vec![level(99.0, 1.0), level(98.0, 1.0)], asks: vec![level(100.0, 1.0), level(101.0, 1.0), level(102.0, 1.0)], at_ms: 0 };
    assert_eq!(walkable(&book, Long, None).len(), 3);
    assert_eq!(walkable(&book, Long, Some(101.0)).len(), 2);
    assert_eq!(walkable(&book, Short, Some(98.5)).len(), 1);
    let walk = rules::walk_book(&walkable(&book, Long, None), 1.5);
    same(walk.average_price, (100.0 + 50.5) / 1.5);
    assert!(walk.complete);
}

// ---- the judge's view of stored rows

fn stored_position(id: u64, side: &str, mode: &str, qty: f64, entry: f64) -> store::Position {
    store::Position {
        id,
        run_id: 1,
        symbol: "BTCUSDT".into(),
        side: side.into(),
        margin_mode: Some(mode.into()),
        leverage: Some(10.0),
        qty,
        entry_price: entry,
        extra_margin: 0.0,
        tp_price: None,
        sl_price: None,
        realised_pnl: 0.0,
        fees_paid: 0.0,
        funding_paid: 0.0,
        opened_by: "MEMBER".into(),
        opened_at: 0,
        closed_at: None,
    }
}

fn btc_prices(mark: f64) -> Prices {
    let rules = rules::Rules {
        tick_size: 0.1,
        qty_step: 0.001,
        min_qty: 0.001,
        max_qty: 1500.0,
        max_market_qty: 150.0,
        min_notional: 5.0,
        min_leverage: 1.0,
        max_leverage: 100.0,
        leverage_step: 0.01,
    };
    let tiers = vec![rules::Tier { risk_limit_value: 2_000_000.0, maintenance_margin_rate: 0.005, initial_margin_rate: 0.01, max_leverage: 100.0, mm_deduction: 0.0 }];
    let mut p = Prices::default();
    p.instruments.insert("BTCUSDT".into(), rules::Instrument { rules, tiers });
    p.quotes.insert("BTCUSDT".into(), rules::Quote { last: mark, mark, bid: mark, ask: mark });
    p
}

#[test]
fn stored_rows_read_as_the_judge_expects() {
    let p = judged_position(&stored_position(7, "SHORT", "ISOLATED", 0.2, 50_000.0));
    assert_eq!((p.id, p.side, p.mode), (7, Short, MarginMode::Isolated));
    let fees = Fees { taker: TAKER, maker: MAKER };
    let snap = snapshot_of(1_000.0, &[stored_position(1, "LONG", "CROSS", 0.1, 50_000.0)], &[], &btc_prices(50_000.0), fees);
    assert_eq!(snap.positions.len(), 1);
    // 1,000 less 500 initial margin less the fee kept for closing
    same(rules::available(&snap), 1_000.0 - 500.0);
}

#[test]
fn closing_in_the_same_order_frees_its_margin_first() {
    let fees = Fees { taker: TAKER, maker: MAKER };
    let snap = snapshot_of(1_000.0, &[stored_position(1, "LONG", "CROSS", 0.1, 50_000.0)], &[], &btc_prices(50_000.0), fees);
    let before = available_after_closing(&snap, None, 0.0);
    let after = available_after_closing(&snap, Some(1), -20.0);
    assert!(after > before);
    same(after, 980.0);
}

#[test]
fn sources_are_written_as_the_tables_expect() {
    assert_eq!(Source::Member.as_str(), "MEMBER");
    assert_eq!(Source::CompanionAsked.as_str(), "COMPANION_ASKED");
    assert_eq!(Source::CompanionAuto.as_str(), "COMPANION_AUTO");
}

// ---- live: paper money on the real public book, in a test memory's database

#[test]
#[ignore = "needs a test memory folder with a database: set CASTLE_MARKET_TEST_MEMORY"]
fn live_paper_money_a_tiny_market_order_closed_and_the_run_closed() {
    let folder = std::path::PathBuf::from(std::env::var("CASTLE_MARKET_TEST_MEMORY").expect("a test memory folder"));
    let mut conn = store::open(&folder).unwrap();
    let board = Board::default();
    // Leave no earlier run open
    if store::open_run(&mut conn, MARKET).unwrap().is_some() {
        close_run(&mut conn, &board).unwrap();
    }
    let run = new_run(&mut conn, 1000.0).unwrap();
    assert!(new_run(&mut conn, 500.0).is_err(), "a run still holding money is not replaced");
    let rules = board.rules("BTCUSDT").unwrap();
    let order = OrderInput {
        symbol: "BTCUSDT".into(),
        side: "LONG".into(),
        order_type: "MARKET".into(),
        qty: rules.min_qty,
        price: None,
        tp: None,
        sl: None,
        reduce_only: false,
        client_order_id: format!("live-test-{}", run.id),
    };
    let preview = preview(&mut conn, &board, &order).unwrap();
    assert_eq!(preview.refusal, None, "{preview:?}");
    let placed = place(&mut conn, &board, &order, Source::Member).unwrap();
    assert_eq!(placed.status, "FILLED");
    let again = place(&mut conn, &board, &order, Source::Member).unwrap();
    assert_eq!(again.id, placed.id, "the same client id is never placed twice");
    let positions = store::open_positions(&mut conn, run.id).unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].opened_by, "MEMBER");
    close(&mut conn, &board, positions[0].id, Source::Member).unwrap();
    assert!(store::open_positions(&mut conn, run.id).unwrap().is_empty());
    let lines = store::ledger(&mut conn, run.id, 50, 0).unwrap();
    let kinds: Vec<&str> = lines.iter().rev().map(|l| l.kind.as_str()).collect();
    assert_eq!(kinds, ["START", "FEE", "REALISED", "FEE"]);
    let balance = store::open_run(&mut conn, MARKET).unwrap().unwrap().balance;
    let total: f64 = lines.iter().map(|l| l.amount).sum();
    assert!((balance - total).abs() < 1e-6, "the balance is the sum of its ledger");
    close_run(&mut conn, &board).unwrap();
    assert!(store::open_run(&mut conn, MARKET).unwrap().is_none());
}
