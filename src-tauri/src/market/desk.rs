//! The companion's desk: what it reads each turn and how its answer is read back. Pure, so the
//! words it gets and the actions it may take are tested without a CLI.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// At most this many actions in one answer.
pub const MAX_ACTIONS: usize = 3;
/// The member's words are cut to this length.
pub const MESSAGE_MAX: usize = 2000;
/// The newest lessons that ride in every desk.
pub const DESK_LESSONS: u32 = 12;
/// The turns before this one that ride in the desk.
pub const DESK_TURNS: u32 = 6;

/// One action the companion asked for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    PlaceOrder {
        symbol: String,
        side: String,
        #[serde(rename = "orderType", default = "market")]
        order_type: String,
        qty: f64,
        #[serde(default)]
        price: Option<f64>,
        #[serde(default)]
        tp: Option<f64>,
        #[serde(default)]
        sl: Option<f64>,
        #[serde(default)]
        why: String,
    },
    ClosePosition {
        symbol: String,
        #[serde(default)]
        why: String,
    },
}

fn market() -> String {
    "MARKET".into()
}

/// The companion's answer: what it says and what it does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    pub reply: String,
    #[serde(default)]
    pub actions: Vec<Action>,
}

/// Reads the answer: the one JSON object in the text (a stray fence or a word around it is
/// tolerated), at most three actions, and only the sides and order types the desk knows.
pub fn parse_answer(text: &str) -> Result<Answer, String> {
    let start = text.find('{').ok_or("The answer had no JSON in it.")?;
    let end = text.rfind('}').ok_or("The answer had no JSON in it.")?;
    if end <= start {
        return Err("The answer had no JSON in it.".into());
    }
    let mut answer: Answer = serde_json::from_str(&text[start..=end]).map_err(|_| "The answer was not in the desk's shape.".to_string())?;
    answer.reply = answer.reply.trim().chars().take(2000).collect();
    if answer.actions.len() > MAX_ACTIONS {
        answer.actions.truncate(MAX_ACTIONS);
    }
    for action in &answer.actions {
        if let Action::PlaceOrder { side, order_type, qty, .. } = action {
            if side != "LONG" && side != "SHORT" {
                return Err("An order asked for a side the desk does not know.".into());
            }
            if order_type != "MARKET" && order_type != "LIMIT" {
                return Err("An order asked for a type the desk does not know.".into());
            }
            if !(qty.is_finite() && *qty > 0.0) {
                return Err("An order asked for no quantity.".into());
            }
        }
    }
    Ok(answer)
}

/// The symbols the member's words name ("buy some eth" -> ETHUSDT), from the tradable list, at most 3.
pub fn named_symbols(message: &str, tradable: &[String]) -> Vec<String> {
    const NOT_A_COIN: [&str; 12] = ["A", "I", "IT", "IS", "ON", "OR", "TO", "UP", "BE", "SO", "AT", "AN"];
    let mut found = Vec::new();
    for word in message.split(|c: char| !c.is_ascii_alphanumeric()) {
        let upper = word.to_ascii_uppercase();
        if upper.len() < 2 || NOT_A_COIN.contains(&upper.as_str()) {
            continue;
        }
        let symbol = if upper.ends_with("USDT") { upper.clone() } else { format!("{upper}USDT") };
        if tradable.iter().any(|s| *s == symbol) && !found.contains(&symbol) {
            found.push(symbol);
        }
        if found.len() == 3 {
            break;
        }
    }
    found
}

/// The member's words, trimmed and cut to the desk's length.
pub fn clean_message(message: &str) -> String {
    message.trim().chars().take(MESSAGE_MAX).collect()
}

/// The words the CLI gets for one turn: the desk, then the member's words or the round's call.
pub fn turn_words(desk: &Value, trigger: &str, message: Option<&str>, member: &str) -> String {
    let ask = match (trigger, message) {
        ("ASKED", Some(m)) => format!("{member} says: {m}"),
        _ => "This is your own round. Look at the desk and decide.".to_string(),
    };
    format!("THE DESK (JSON):\n{}\n\n{ask}\n\nAnswer with the one JSON object only.", serde_json::to_string_pretty(desk).unwrap_or_default())
}

/// Lays out the desk from what the turn gathered.
#[allow(clippy::too_many_arguments)]
pub fn desk(trigger: &str, account: Value, symbols: Value, candles: Value, scan: Value, positions: Value, orders: Value, fees: Value, turns: Value, limits: Value, lessons: Value) -> Value {
    json!({
        "trigger": trigger,
        "account": account,
        "symbols": symbols,
        "candles": candles,
        "scanner": scan,
        "positions": positions,
        "restingOrders": orders,
        "fees": fees,
        "lastTurns": turns,
        "limits": limits,
        "lessons": lessons,
    })
}

/// The words for one lesson after a position closed.
pub fn lesson_words(cycle: &Value, member: &str) -> String {
    format!(
        "A paper position just closed. Here it is (JSON):\n{}\n\nWrite ONE short lesson (at most 30 words) you would keep for your next trades: what went right or wrong and what you would do again or differently. \
         If {member} traded it, learn from their trade too. Answer with the lesson sentence only.",
        serde_json::to_string_pretty(cycle).unwrap_or_default()
    )
}

/// The lesson as kept: one line, no quotes or fence, at most 400 characters.
pub fn clean_lesson(text: &str) -> Option<String> {
    let line = text.trim().trim_matches('`').trim().trim_matches('"').trim().replace(['\r', '\n'], " ");
    let line: String = line.split_whitespace().collect::<Vec<_>>().join(" ");
    let line: String = line.chars().take(400).collect();
    (!line.is_empty()).then_some(line)
}

/// Her emotion engine at the desk: how she sounds in `reply`, never what she decides. The desk is
/// always her and her person, so both audiences' lines apply. Empty when the switch is off or no
/// trait is active, and then the rules carry no heading at all. Pure, tested.
pub fn manner_section(manner: &crate::traits::Manner) -> String {
    if manner.is_empty() {
        return String::new();
    }
    let mut out = String::from("## How you sound\n\nThis is how you sound in `reply` - never what you decide. These words colour your voice; the trading rules below and {{USER_NAME}}'s limits still decide every action.\n\n");
    for line in manner.everyone.iter().chain(manner.owner.iter()) {
        out.push_str(&format!("- {line}\n"));
    }
    out.push('\n');
    out
}

/// The desk's rules with her manner in its slot: the section, or nothing. Pure, tested.
pub fn with_manner(rules: &str, manner: &crate::traits::Manner) -> String {
    let section = manner_section(manner);
    rules.replace("{{MANNER}}\r\n", &section).replace("{{MANNER}}\n", &section)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Manner;

    #[test]
    fn her_traits_colour_the_desk_only_when_they_reach_it() {
        let rules = "intro\n\n{{MANNER}}\n## How you answer\n";
        assert_eq!(with_manner(rules, &Manner::default()), "intro\n\n## How you answer\n", "no trait, no heading left behind");
        let manner = Manner { everyone: vec!["asks one more question, kindly".into()], owner: vec!["a little teasing, never unkind".into()] };
        let text = with_manner(rules, &manner);
        assert!(text.contains("## How you sound"));
        assert!(text.contains("- asks one more question, kindly\n- a little teasing, never unkind\n"), "both audiences: the desk is always her person");
        assert!(text.contains("never what you decide"), "a trait never overrides the rules or the limits");
        assert!(text.ends_with("\n\n## How you answer\n"));
    }

    #[test]
    fn the_desk_rules_carry_the_manner_slot() {
        assert!(include_str!("../../resources/market-rules.md").lines().any(|l| l.trim_end() == "{{MANNER}}"));
    }

    #[test]
    fn an_answer_is_read_even_with_a_fence_around_it() {
        let text = "Sure!\n```json\n{\"reply\":\"Small ETH long, stop below the low.\",\"actions\":[{\"type\":\"place_order\",\"symbol\":\"ETHUSDT\",\"side\":\"LONG\",\"orderType\":\"MARKET\",\"qty\":0.05,\"sl\":3100,\"why\":\"held support\"}]}\n```";
        let a = parse_answer(text).unwrap();
        assert_eq!(a.reply, "Small ETH long, stop below the low.");
        assert_eq!(a.actions.len(), 1);
        assert!(matches!(&a.actions[0], Action::PlaceOrder { symbol, qty, sl: Some(sl), .. } if symbol == "ETHUSDT" && *qty == 0.05 && *sl == 3100.0));
    }

    #[test]
    fn a_wrong_answer_is_refused_plainly() {
        assert!(parse_answer("no json here").is_err());
        assert!(parse_answer("{\"reply\":\"x\",\"actions\":[{\"type\":\"place_order\",\"symbol\":\"BTCUSDT\",\"side\":\"UP\",\"qty\":1}]}").is_err());
        assert!(parse_answer("{\"reply\":\"x\",\"actions\":[{\"type\":\"place_order\",\"symbol\":\"BTCUSDT\",\"side\":\"LONG\",\"qty\":0}]}").is_err());
        assert!(parse_answer("{\"reply\":\"x\",\"actions\":[{\"type\":\"withdraw\"}]}").is_err());
    }

    #[test]
    fn at_most_three_actions_are_taken() {
        let one = "{\"type\":\"close_position\",\"symbol\":\"BTCUSDT\"}";
        let text = format!("{{\"reply\":\"tidy up\",\"actions\":[{one},{one},{one},{one},{one}]}}");
        assert_eq!(parse_answer(&text).unwrap().actions.len(), MAX_ACTIONS);
        assert_eq!(parse_answer("{\"reply\":\"just looking\"}").unwrap().actions, vec![]);
    }

    #[test]
    fn coins_named_in_words_become_symbols() {
        let tradable: Vec<String> = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "AUSDT"].iter().map(|s| s.to_string()).collect();
        assert_eq!(named_symbols("buy a small eth long and maybe sol, what about BTCUSDT?", &tradable), ["ETHUSDT", "SOLUSDT", "BTCUSDT"]);
        assert!(named_symbols("is it up a lot?", &tradable).is_empty());
    }

    #[test]
    fn the_turn_words_carry_the_desk_and_who_asked() {
        let d = desk("ASKED", json!({"balance": 10000}), json!([]), json!({}), json!({}), json!([]), json!([]), json!({}), json!([]), json!({}), json!(["Wait for a pullback."]));
        let words = turn_words(&d, "ASKED", Some("what do you think of BTC?"), "Rin");
        assert!(words.contains("Rin says: what do you think of BTC?") && words.contains("Wait for a pullback."));
        assert!(turn_words(&d, "AUTO", None, "Rin").contains("your own round"));
        assert_eq!(clean_message(&"x".repeat(3000)).len(), MESSAGE_MAX);
    }

    #[test]
    fn a_lesson_is_one_bounded_line() {
        assert_eq!(clean_lesson("\"Wait for confirmation\nbefore entering.\"").unwrap(), "Wait for confirmation before entering.");
        assert_eq!(clean_lesson(&"y".repeat(600)).unwrap().len(), 400);
        assert_eq!(clean_lesson("   "), None);
    }
}
