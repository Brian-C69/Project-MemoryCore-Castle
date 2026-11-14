//! DEV's live servers: which TCP ports are listening on this computer, which program holds each,
//! and which DEV tab (if any) owns it. Read only - nothing here ever ends or signals a process.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// One process as the server list needs it.
#[derive(Debug, Clone)]
pub struct Proc {
    #[allow(dead_code)] // the table's key, kept beside the rest for reading a row on its own
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub folder: Option<PathBuf>,
}

/// Who a listening process belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// Started inside the companion's session of this DEV tab slot.
    Tab(u8),
    /// Runs inside the project folder of this DEV tab slot.
    Folder(u8),
    /// A developer program that belongs to no tab.
    Other,
    /// A system or desktop program - not shown.
    Hidden,
}

/// One live server row for the DEV tab.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub port: u16,
    pub pid: u32,
    pub program: String,
    pub folder: Option<String>,
    pub slot: Option<u8>,
    pub started_by: String,
}

/// Program names (lowercase, no `.exe`) that count as developer servers.
const DEV_PROGRAMS: &[&str] = &[
    "node", "deno", "bun", "java", "javaw", "python", "python3", "pythonw", "php", "php-cgi", "ruby", "go",
    "dotnet", "uvicorn", "gunicorn", "flask", "rails", "nginx", "httpd", "caddy", "mysqld", "postgres",
    "redis-server", "mongod", "cargo", "rustc", "air", "hugo", "ng", "vite", "esbuild",
];

/// The longest parent chain we follow before giving up.
const MAX_HOPS: usize = 64;

/// Listening TCP ports with the owning pid, IPv4 and IPv6, one row per port, sorted by port.
/// System pids 0 and 4 are skipped. Empty on non-Windows systems.
pub fn listeners() -> Vec<(u16, u32)> {
    let mut rows = raw_listeners();
    rows.retain(|&(_, pid)| pid != 0 && pid != 4);
    rows.sort();
    rows.dedup_by_key(|row| row.0);
    rows
}

#[cfg(windows)]
fn raw_listeners() -> Vec<(u16, u32)> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};

    let mut out = Vec::new();
    if let Some(buf) = tcp_table(AF_INET as u32, TCP_TABLE_OWNER_PID_LISTENER) {
        for row in table_rows::<MIB_TCPROW_OWNER_PID>(&buf) {
            out.push((port_of(row.dwLocalPort), row.dwOwningPid));
        }
    }
    if let Some(buf) = tcp_table(AF_INET6 as u32, TCP_TABLE_OWNER_PID_LISTENER) {
        for row in table_rows::<MIB_TCP6ROW_OWNER_PID>(&buf) {
            out.push((port_of(row.dwLocalPort), row.dwOwningPid));
        }
    }
    out
}

#[cfg(not(windows))]
fn raw_listeners() -> Vec<(u16, u32)> {
    Vec::new()
}

/// The table keeps the port in network byte order in the low 16 bits.
#[cfg(windows)]
fn port_of(raw: u32) -> u16 {
    u16::from_be(raw as u16)
}

/// Asks IP Helper for one TCP table, growing the buffer until it fits.
/// The buffer is u32-aligned so the rows can be read in place.
#[cfg(windows)]
fn tcp_table(family: u32, class: i32) -> Option<Vec<u32>> {
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
    use windows_sys::Win32::NetworkManagement::IpHelper::GetExtendedTcpTable;

    let mut size: u32 = 0;
    for _ in 0..5 {
        let mut buf: Vec<u32> = vec![0; (size as usize).div_ceil(4).max(1)];
        let mut bytes = (buf.len() * 4) as u32;
        let result =
            unsafe { GetExtendedTcpTable(buf.as_mut_ptr() as *mut c_void, &mut bytes, 0, family, class, 0) };
        if result == NO_ERROR {
            return Some(buf);
        }
        if result != ERROR_INSUFFICIENT_BUFFER {
            return None;
        }
        // The table can grow between calls; leave some room.
        size = bytes + 1024;
    }
    None
}

/// Reads the rows that follow the `dwNumEntries` count of a `MIB_*TABLE_OWNER_PID` buffer.
#[cfg(windows)]
fn table_rows<Row: Copy>(buf: &[u32]) -> Vec<Row> {
    let count = buf[0] as usize;
    let first = std::mem::size_of::<u32>().max(std::mem::align_of::<Row>());
    let bytes = buf.len() * 4;
    let fits = (bytes.saturating_sub(first)) / std::mem::size_of::<Row>();
    let base = buf.as_ptr() as *const u8;
    (0..count.min(fits))
        .map(|i| unsafe { std::ptr::read_unaligned(base.add(first + i * std::mem::size_of::<Row>()) as *const Row) })
        .collect()
}

/// Every running process with its name, parent and working folder.
fn processes() -> HashMap<u32, Proc> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

    let Ok(mut system) = System::new() else {
        return HashMap::new();
    };
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always),
    );
    system
        .processes()
        .iter()
        .map(|(pid, process)| {
            let pid = pid.as_u32();
            let proc = Proc {
                pid,
                parent: process.parent().map(|p| p.as_u32()),
                name: process.name().to_string_lossy().into_owned(),
                folder: process.cwd().filter(|p| !p.as_os_str().is_empty()).map(Path::to_path_buf),
            };
            (pid, proc)
        })
        .collect()
}

/// Splits a path into comparable parts: no `\\?\` prefix, no empty parts, lowercase on Windows.
fn parts(path: &Path) -> Vec<String> {
    let text = path.to_string_lossy();
    let text = text.strip_prefix(r"\\?\").unwrap_or(&text);
    text.split(['\\', '/'])
        .filter(|part| !part.is_empty() && *part != ".")
        .map(|part| if cfg!(windows) { part.to_lowercase() } else { part.to_string() })
        .collect()
}

/// True when `child` is `parent` itself or lies anywhere below it, part by part.
fn is_inside(child: &[String], parent: &[String]) -> bool {
    !parent.is_empty() && child.len() >= parent.len() && child[..parent.len()] == *parent
}

/// The program name as compared against the developer list: lowercase, no `.exe`.
fn program_key(name: &str) -> String {
    let lower = name.to_lowercase();
    lower.strip_suffix(".exe").unwrap_or(&lower).to_string()
}

/// Decides who owns `pid`: a tab whose session started it, a tab whose project folder it runs in,
/// some other developer program, or nobody worth showing.
pub fn owner_of(pid: u32, procs: &HashMap<u32, Proc>, sessions: &[(u8, u32)], projects: &[(u8, PathBuf)]) -> Owner {
    // 1. Walk up the parent chain (pid itself first) looking for a tab's session.
    let mut seen = HashSet::new();
    let mut current = Some(pid);
    for _ in 0..MAX_HOPS {
        let Some(at) = current else { break };
        if !seen.insert(at) {
            break;
        }
        if let Some(&(slot, _)) = sessions.iter().find(|&&(_, session)| session == at) {
            return Owner::Tab(slot);
        }
        current = procs.get(&at).and_then(|p| p.parent);
    }

    let proc = procs.get(&pid);

    // 2. Its working folder inside a project folder - the deepest project wins.
    if let Some(folder) = proc.and_then(|p| p.folder.as_deref()) {
        let here = parts(folder);
        let best = projects
            .iter()
            .map(|(slot, root)| (*slot, parts(root)))
            .filter(|(_, root)| is_inside(&here, root))
            .max_by_key(|(_, root)| root.len());
        if let Some((slot, _)) = best {
            return Owner::Folder(slot);
        }
    }

    // 3. A developer program nobody owns.
    match proc {
        Some(p) if DEV_PROGRAMS.contains(&program_key(&p.name).as_str()) => Owner::Other,
        _ => Owner::Hidden,
    }
}

/// The live dev servers on this computer, sorted by port. System and desktop programs are left out.
/// `sessions` is (slot, session pid) per open DEV tab; `projects` is (slot, project folder);
/// `companion` names who started a tab's server.
pub fn servers(sessions: &[(u8, u32)], projects: &[(u8, PathBuf)], companion: &str) -> Vec<Server> {
    let ports = listeners();
    if ports.is_empty() {
        return Vec::new();
    }
    let procs = processes();
    let mut out: Vec<Server> = ports
        .into_iter()
        .filter_map(|(port, pid)| {
            let (slot, started_by) = match owner_of(pid, &procs, sessions, projects) {
                Owner::Tab(slot) => (Some(slot), format!("{companion} (its tab)")),
                Owner::Folder(slot) => (Some(slot), "you (its folder)".to_string()),
                Owner::Other => (None, String::new()),
                Owner::Hidden => return None,
            };
            let proc = procs.get(&pid);
            Some(Server {
                port,
                pid,
                program: proc.map(|p| p.name.clone()).unwrap_or_default(),
                folder: proc.and_then(|p| p.folder.as_ref()).map(|f| f.to_string_lossy().into_owned()),
                slot,
                started_by,
            })
        })
        .collect();
    out.sort_by_key(|s| s.port);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, parent: Option<u32>, name: &str, folder: Option<&str>) -> (u32, Proc) {
        (pid, Proc { pid, parent, name: name.to_string(), folder: folder.map(PathBuf::from) })
    }

    fn table(rows: Vec<(u32, Proc)>) -> HashMap<u32, Proc> {
        rows.into_iter().collect()
    }

    #[test]
    fn grandchild_of_a_session_is_its_tab() {
        let procs = table(vec![
            proc(100, Some(1), "claude.exe", Some(r"C:\Users\Lyra")),
            proc(200, Some(100), "cmd.exe", None),
            proc(300, Some(200), "node.exe", Some(r"D:\elsewhere")),
        ]);
        let projects = vec![(1, PathBuf::from(r"D:\elsewhere"))];
        assert_eq!(owner_of(300, &procs, &[(3, 100)], &projects), Owner::Tab(3));
    }

    #[test]
    fn folder_inside_a_project_is_its_folder() {
        let procs = table(vec![proc(10, None, "node.exe", Some(r"d:\Work\Shop\frontend"))]);
        let projects = vec![(2, PathBuf::from(r"D:\work\shop"))];
        assert_eq!(owner_of(10, &procs, &[], &projects), Owner::Folder(2));
    }

    #[test]
    fn sibling_prefix_folder_is_not_inside() {
        let procs = table(vec![proc(10, None, "svchost.exe", Some(r"D:\ab"))]);
        let projects = vec![(0, PathBuf::from(r"D:\a"))];
        assert_eq!(owner_of(10, &procs, &[], &projects), Owner::Hidden);
    }

    #[test]
    fn longest_project_wins() {
        let procs = table(vec![proc(10, None, "java.exe", Some(r"D:\a\b\c"))]);
        let projects = vec![(0, PathBuf::from(r"D:\a")), (5, PathBuf::from(r"D:\a\b")), (6, PathBuf::from(r"D:\a\x"))];
        assert_eq!(owner_of(10, &procs, &[], &projects), Owner::Folder(5));
    }

    #[test]
    fn developer_program_is_other() {
        let procs = table(vec![proc(10, Some(1), "MySQLd.exe", Some(r"C:\xampp\mysql"))]);
        let projects = vec![(0, PathBuf::from(r"D:\a"))];
        assert_eq!(owner_of(10, &procs, &[(0, 999)], &projects), Owner::Other);
    }

    #[test]
    fn svchost_is_hidden() {
        let procs = table(vec![proc(10, Some(1), "svchost.exe", Some(r"C:\Windows\System32"))]);
        assert_eq!(owner_of(10, &procs, &[], &[]), Owner::Hidden);
        assert_eq!(owner_of(77, &procs, &[], &[]), Owner::Hidden);
    }

    #[test]
    fn parent_loop_terminates() {
        let procs = table(vec![
            proc(1, Some(3), "a.exe", None),
            proc(2, Some(1), "b.exe", None),
            proc(3, Some(2), "node.exe", None),
        ]);
        assert_eq!(owner_of(3, &procs, &[(0, 999)], &[]), Owner::Other);
        let self_parent = table(vec![proc(5, Some(5), "x.exe", None)]);
        assert_eq!(owner_of(5, &self_parent, &[], &[]), Owner::Hidden);
    }

    #[test]
    #[ignore]
    fn live_servers() {
        for (port, pid) in listeners() {
            println!("listen {port:>5}  pid {pid}");
        }
        for s in servers(&[], &[], "Lyra") {
            println!("{:>5}  {:>6}  {:<20} {:<16} {:?}", s.port, s.pid, s.program, s.started_by, s.folder);
        }
    }
}
