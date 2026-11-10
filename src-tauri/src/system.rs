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

/// Processor time the whole computer has spent since it started, in 100-nanosecond ticks.
/// On Windows the kernel figure already includes the idle time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CpuTimes {
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
}

/// How busy the processor was between two readings. None when no time has passed or the
/// counters went backwards, so the caller keeps its last answer instead of showing a wrong one.
pub fn busy_percent(before: CpuTimes, after: CpuTimes) -> Option<u8> {
    let idle = after.idle.checked_sub(before.idle)?;
    let total = after.kernel.checked_sub(before.kernel)? + after.user.checked_sub(before.user)?;
    if total == 0 {
        return None;
    }
    Some(percent(total.saturating_sub(idle) as f64, total as f64))
}

/// Reads processor time straight from the Windows kernel. Unlike sysinfo's CPU figure, this does
/// not use Windows performance counters, which are missing or damaged on some computers. There
/// sysinfo quietly reports 0% while Task Manager still works.
#[cfg(windows)]
fn cpu_times() -> Option<CpuTimes> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetSystemTimes;
    let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut idle, mut kernel, mut user) = (zero, zero, zero);
    // SAFETY: the three pointers are to live, writable FILETIME values for the whole call.
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    (ok != 0).then(|| CpuTimes { idle: ticks(idle), kernel: ticks(kernel), user: ticks(user) })
}

#[cfg(not(windows))]
fn cpu_times() -> Option<CpuTimes> {
    None
}

struct State {
    system: System,
    last_times: Option<CpuTimes>,
    last_cpu: u8,
}

/// Processor use needs two readings apart in time, so the state lives for the whole run.
/// When the system cannot be read at all, the gauges show zero and Castle carries on.
pub struct SystemMeters(Mutex<Option<State>>);

impl Default for SystemMeters {
    fn default() -> Self {
        let state = System::new().ok().map(|system| State { system, last_times: cpu_times(), last_cpu: 0 });
        Self(Mutex::new(state))
    }
}

impl SystemMeters {
    pub fn read(&self) -> Meters {
        let mut guard = self.0.lock().unwrap();
        let Some(state) = guard.as_mut() else {
            return Meters { cpu: 0, memory: 0 };
        };
        state.system.refresh_memory();
        let cpu = match cpu_times() {
            // Windows: the kernel's own processor times, no performance counters needed.
            Some(now) => {
                if let Some(busy) = state.last_times.and_then(|before| busy_percent(before, now)) {
                    state.last_cpu = busy;
                }
                state.last_times = Some(now);
                state.last_cpu
            }
            None => {
                state.system.refresh_cpu_usage();
                percent(state.system.global_cpu_usage() as f64, 100.0)
            }
        };
        Meters {
            cpu,
            memory: percent(state.system.used_memory() as f64, state.system.total_memory() as f64),
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

    fn times(idle: u64, kernel: u64, user: u64) -> CpuTimes {
        CpuTimes { idle, kernel, user }
    }

    #[test]
    fn busy_percent_counts_kernel_and_user_time_minus_idle() {
        // 1000 ticks passed: 600 kernel (of which 500 idle) + 400 user, so 500 of 1000 were busy.
        assert_eq!(busy_percent(times(0, 0, 0), times(500, 600, 400)), Some(50));
        assert_eq!(busy_percent(times(100, 100, 0), times(1100, 1100, 0)), Some(0));
        assert_eq!(busy_percent(times(0, 0, 0), times(0, 300, 700)), Some(100));
    }

    #[test]
    fn busy_percent_refuses_readings_it_cannot_trust() {
        assert_eq!(busy_percent(times(5, 5, 5), times(5, 5, 5)), None);
        assert_eq!(busy_percent(times(9, 9, 9), times(1, 1, 1)), None);
    }

    #[cfg(windows)]
    #[test]
    fn windows_reports_processor_time_without_performance_counters() {
        let first = cpu_times().expect("GetSystemTimes answers");
        let mut spin = 0u64;
        for i in 0..20_000_000u64 {
            spin = spin.wrapping_add(i ^ (spin >> 3));
        }
        std::hint::black_box(spin);
        let second = cpu_times().expect("GetSystemTimes answers");
        assert!(second.kernel + second.user > first.kernel + first.user);
        assert!(busy_percent(first, second).is_some());
    }
}
