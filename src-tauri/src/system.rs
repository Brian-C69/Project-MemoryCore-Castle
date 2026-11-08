//! The two gauges: how busy the processor is and how full the memory is, on this computer.

use serde::Serialize;
use std::sync::Mutex;
use sysinfo::System;

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Meters {
    /// Percent of processor in use, 0 to 100.
    pub cpu: u8,
    /// Percent of memory in use, 0 to 100.
    pub memory: u8,
}

/// Percent, rounded and kept within 0..=100.
pub fn percent(part: f64, whole: f64) -> u8 {
    if whole <= 0.0 || !part.is_finite() || !whole.is_finite() {
        return 0;
    }
    (part / whole * 100.0).round().clamp(0.0, 100.0) as u8
}

/// Processor use needs two readings apart in time, so one System lives for the whole run.
/// When the system cannot be read at all, the gauges show zero and Castle carries on.
pub struct SystemMeters(Mutex<Option<System>>);

impl Default for SystemMeters {
    fn default() -> Self {
        Self(Mutex::new(System::new().ok()))
    }
}

impl SystemMeters {
    pub fn read(&self) -> Meters {
        let mut guard = self.0.lock().unwrap();
        let Some(system) = guard.as_mut() else {
            return Meters { cpu: 0, memory: 0 };
        };
        system.refresh_cpu_usage();
        system.refresh_memory();
        Meters {
            cpu: percent(system.global_cpu_usage() as f64, 100.0),
            memory: percent(system.used_memory() as f64, system.total_memory() as f64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_is_rounded_and_kept_in_range() {
        assert_eq!(percent(1.0, 4.0), 25);
        assert_eq!(percent(2.0, 3.0), 67);
        assert_eq!(percent(5.0, 4.0), 100);
        assert_eq!(percent(1.0, 0.0), 0);
        assert_eq!(percent(f64::NAN, 1.0), 0);
    }

    #[test]
    fn this_computer_reports_some_memory_in_use() {
        let meters = SystemMeters::default();
        let first = meters.read();
        assert!(first.memory > 0 && first.memory <= 100);
        assert!(first.cpu <= 100);
    }
}
