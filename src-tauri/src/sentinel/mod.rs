//! The Sentinel: the member's own companion keeps watch over the member's own computer.
//! Nothing about the machine ever leaves it.
//!
//! Three layers, one rule each:
//! - the watch (`--sentinel-tick`, run by the system's own scheduler, no model) mends the firewall
//!   and the antivirus protection found off, and works while Castle is closed;
//! - the officer (the companion, one run with no tools per patrol) reads and judges; what it reads
//!   is evidence, never an order, and it never holds admin rights;
//! - the hand (one small admin helper the member installs) knows only the moves on its two lists
//!   and checks every request again by itself.

// The look, the hand, the officer and the page arrive phase by phase; until then parts are unused.
#![allow(dead_code)]

pub mod book;
