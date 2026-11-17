//! Graphics-card gauges beside CPU and RAM on HOME's Gauges card. NVIDIA cards are read through
//! `nvidia-smi`, which ships with the driver; a computer without one simply shows no graphics gauges
//! and Castle does not keep looking for it. The member can switch the graphics cards off from the
//! gear on the Gauges card; that choice is saved by `gpu_gauges_save`, never by the Settings page.

use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::State;

/// Most cards drawn (the first ones, in nvidia-smi's own order).
pub const MAX_CARDS: usize = 4;
/// A reading is reused for this long, so several windows asking never start nvidia-smi twice at once.
const CACHE_FOR: Duration = Duration::from_millis(2500);
/// nvidia-smi is given this long to answer before it is stopped.
const SMI_LIMIT: Duration = Duration::from_secs(3);
/// When nvidia-smi is not installed it is looked for again only this often (a driver can be installed
/// while Castle runs, but it is not worth starting a process every few seconds to find out).
const MISSING_RECHECK: Duration = Duration::from_secs(300);

/// What the member chose for the graphics-card gauges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GpuGauges {
    pub on: bool,
}

impl Default for GpuGauges {
    fn default() -> Self {
        Self { on: true }
    }
}

/// One card as nvidia-smi reports it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub index: u32,
    pub name: String,
    /// percent of the card's processing in use, 0 to 100
    pub util: u8,
    pub mem_used_mb: u32,
    pub mem_total_mb: u32,
    pub temp_c: Option<u8>,
}

/// "NVIDIA GeForce RTX 3070" -> "RTX 3070": the label that fits under a gauge. Pure, tested.
pub fn short_name(full: &str) -> String {
    let mut words: Vec<&str> = full.split_whitespace().collect();
    while words.first().is_some_and(|w| matches!(w.to_ascii_lowercase().as_str(), "nvidia" | "geforce")) {
        words.remove(0);
    }
    let short = words.join(" ");
    if short.is_empty() { full.trim().to_string() } else { short }
}

/// Parses `nvidia-smi --query-gpu=index,name,utilization.gpu,memory.used,memory.total,temperature.gpu
/// --format=csv,noheader,nounits`. A line that does not parse is skipped; "[N/A]" counts as unknown.
/// Pure, tested.
pub fn parse_smi(csv: &str) -> Vec<Reading> {
    csv.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            if f.len() < 6 {
                return None;
            }
            let number = |s: &str| s.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0);
            Some(Reading {
                index: f[0].parse().ok()?,
                name: f[1].to_string(),
                util: number(f[2]).map_or(0, |v| v.round().min(100.0) as u8),
                mem_used_mb: number(f[3]).map_or(0, |v| v as u32),
                mem_total_mb: number(f[4]).map_or(0, |v| v as u32),
                temp_c: number(f[5]).map(|v| v.round().min(255.0) as u8),
            })
        })
        .collect()
}

/// What running nvidia-smi came to.
enum Probe {
    /// it ran: the cards (none is possible)
    Cards(Vec<Reading>),
    /// it is not installed on this computer
    Missing,
}

/// Runs nvidia-smi once and parses it. A failure or a slow answer is "no cards", not "missing".
fn probe_nvidia() -> Probe {
    let mut command = std::process::Command::new("nvidia-smi");
    command
        .args(["--query-gpu=index,name,utilization.gpu,memory.used,memory.total,temperature.gpu", "--format=csv,noheader,nounits"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // no console window
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Probe::Missing,
        Err(_) => return Probe::Cards(Vec::new()),
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < SMI_LIMIT => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Probe::Cards(Vec::new());
            }
        }
    }
    let mut text = String::new();
    if let Some(mut out) = child.stdout.take() {
        use std::io::Read;
        let _ = out.read_to_string(&mut text);
    }
    Probe::Cards(parse_smi(&text))
}

#[derive(Default)]
pub struct GpuCache(Mutex<Option<(Instant, Option<Vec<Reading>>)>>);

impl GpuCache {
    /// A reading no older than CACHE_FOR (or, when nvidia-smi is not installed, no older than
    /// MISSING_RECHECK), else a fresh one, run off the page's thread (nvidia-smi takes a moment).
    async fn read(&self) -> Vec<Reading> {
        if let Some(readings) = self.fresh() {
            return readings;
        }
        let probe = tauri::async_runtime::spawn_blocking(probe_nvidia).await.unwrap_or(Probe::Cards(Vec::new()));
        let (readings, remembered) = match probe {
            Probe::Cards(cards) => (cards.clone(), Some(cards)),
            Probe::Missing => (Vec::new(), None),
        };
        *self.0.lock().unwrap() = Some((Instant::now(), remembered));
        readings
    }

    /// The last reading whatever its age; one is taken only when there has never been any.
    async fn last_or_once(&self) -> Vec<Reading> {
        let last = self.0.lock().unwrap().as_ref().map(|(_, r)| r.clone().unwrap_or_default());
        match last {
            Some(readings) => readings,
            None => self.read().await,
        }
    }

    fn fresh(&self) -> Option<Vec<Reading>> {
        let guard = self.0.lock().unwrap();
        let (at, readings) = guard.as_ref()?;
        match readings {
            Some(cards) if at.elapsed() < CACHE_FOR => Some(cards.clone()),
            None if at.elapsed() < MISSING_RECHECK => Some(Vec::new()),
            _ => None,
        }
    }
}

/// One gauge as the page draws it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gauge {
    pub index: u32,
    /// what is written under the gauge: the card's short name
    pub label: String,
    /// the card's full name, for the tooltip
    pub name: String,
    pub util: u8,
    pub mem_used_mb: u32,
    pub mem_total_mb: u32,
    pub temp_c: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuView {
    /// the gauges to draw (empty when switched off or no card was found)
    pub gauges: Vec<Gauge>,
    pub on: bool,
    /// how many cards were found (the gear says so)
    pub found: usize,
}

/// Which gauges to draw from the readings and the member's choice. Pure, tested.
pub fn view_of(readings: &[Reading], settings: &GpuGauges) -> GpuView {
    let gauges = if settings.on {
        readings
            .iter()
            .take(MAX_CARDS)
            .map(|r| Gauge {
                index: r.index,
                label: short_name(&r.name),
                name: r.name.clone(),
                util: r.util,
                mem_used_mb: r.mem_used_mb,
                mem_total_mb: r.mem_total_mb,
                temp_c: r.temp_c,
            })
            .collect()
    } else {
        Vec::new()
    };
    GpuView { gauges, on: settings.on, found: readings.len() }
}

#[tauri::command]
pub async fn gpu_meters(store: State<'_, SettingsStore>, cache: State<'_, GpuCache>) -> Result<GpuView, String> {
    let settings = store.get().gpu_gauges;
    // switched off: nothing is started again; the gear still learns how many cards there are from the
    // last look (nvidia-smi is run once if there has been none)
    let readings = if settings.on { cache.read().await } else { cache.last_or_once().await };
    Ok(view_of(&readings, &settings))
}

#[tauri::command]
pub async fn gpu_gauges_save(store: State<'_, SettingsStore>, cache: State<'_, GpuCache>, on: bool) -> Result<GpuView, String> {
    let chosen = GpuGauges { on };
    let mut next = store.get();
    next.gpu_gauges = chosen.clone();
    crate::settings::save(&store.path, &next).map_err(|_| "The graphics-card gauges could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    let readings = cache.last_or_once().await;
    Ok(view_of(&readings, &chosen))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMI: &str = "0, NVIDIA GeForce RTX 3070, 4, 7781, 8192, 45\n1, NVIDIA GeForce RTX 4060, 0, 6574, 8188, 42\n";

    #[test]
    fn nvidia_smis_lines_become_readings() {
        let r = parse_smi(SMI);
        assert_eq!(r.len(), 2);
        assert_eq!((r[0].index, r[0].name.as_str(), r[0].util, r[0].mem_used_mb, r[0].mem_total_mb, r[0].temp_c), (0, "NVIDIA GeForce RTX 3070", 4, 7781, 8192, Some(45)));
        assert_eq!((r[1].index, r[1].name.as_str(), r[1].util), (1, "NVIDIA GeForce RTX 4060", 0));
    }

    #[test]
    fn odd_lines_are_skipped_and_unknown_values_do_not_break_a_card() {
        let r = parse_smi("not a card\n\n0, NVIDIA GeForce RTX 3070, [N/A], [N/A], 8192, [N/A]\n2, short, 5\n1, X, 250, 1, 2, 3\n");
        assert_eq!(r.len(), 2);
        assert_eq!((r[0].util, r[0].mem_used_mb, r[0].temp_c), (0, 0, None), "[N/A] is unknown, not a crash");
        assert_eq!(r[1].util, 100, "a percent never goes past 100");
        assert!(parse_smi("").is_empty());
    }

    #[test]
    fn a_name_is_shortened_to_fit_under_a_gauge() {
        assert_eq!(short_name("NVIDIA GeForce RTX 3070"), "RTX 3070");
        assert_eq!(short_name("NVIDIA GeForce RTX 4060 Ti"), "RTX 4060 Ti");
        assert_eq!(short_name("NVIDIA GeForce GTX 1080"), "GTX 1080");
        assert_eq!(short_name("NVIDIA"), "NVIDIA", "never shortened to nothing");
    }

    #[test]
    fn the_gauges_show_every_card_found_up_to_the_most_and_only_when_on() {
        let readings = parse_smi(SMI);
        let on = view_of(&readings, &GpuGauges { on: true });
        assert_eq!(on.gauges.len(), 2);
        assert_eq!((on.gauges[0].label.as_str(), on.gauges[1].label.as_str()), ("RTX 3070", "RTX 4060"));
        assert_eq!(on.gauges[0].name, "NVIDIA GeForce RTX 3070", "the full name stays for the tooltip");
        let off = view_of(&readings, &GpuGauges { on: false });
        assert!(off.gauges.is_empty());
        assert_eq!((off.on, off.found), (false, 2), "the gear still knows how many cards there are");
        assert!(view_of(&[], &GpuGauges::default()).gauges.is_empty(), "no card, no gauges, no trouble");
        let many: Vec<Reading> = (0..9).map(|i| Reading { index: i, name: format!("Card {i}"), util: 1, mem_used_mb: 1, mem_total_mb: 2, temp_c: None }).collect();
        assert_eq!(view_of(&many, &GpuGauges::default()).gauges.len(), MAX_CARDS);
    }

    #[test]
    fn the_gauges_start_on_and_an_older_settings_file_reads_as_on() {
        assert!(GpuGauges::default().on);
        let from_old: GpuGauges = serde_json::from_str("{}").unwrap();
        assert!(from_old.on);
        let off: GpuGauges = serde_json::from_str(r#"{"on":false}"#).unwrap();
        assert!(!off.on);
    }

    #[test]
    fn a_missing_nvidia_smi_is_remembered_so_it_is_not_looked_for_every_few_seconds() {
        let cache = GpuCache::default();
        *cache.0.lock().unwrap() = Some((Instant::now(), None));
        assert_eq!(cache.fresh(), Some(Vec::new()), "missing: answered at once, no process started");
        *cache.0.lock().unwrap() = Some((Instant::now() - MISSING_RECHECK - Duration::from_secs(1), None));
        assert!(cache.fresh().is_none(), "looked for again after a while");
        *cache.0.lock().unwrap() = Some((Instant::now() - CACHE_FOR - Duration::from_millis(1), Some(parse_smi(SMI))));
        assert!(cache.fresh().is_none(), "a reading goes stale after a couple of seconds");
    }
}
