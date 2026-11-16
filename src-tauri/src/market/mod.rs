//! The MARKET room: paper money on public Bybit prices, and a companion who trades beside the member.
//! Nothing real is ever ordered; no exchange key is ever kept.

// The room's commands (Phase 56) and the companion (Phase 57) use these; until then parts are unused.
#![allow(dead_code)]

pub mod board;
pub mod feed;
pub mod rules;
pub mod store;
