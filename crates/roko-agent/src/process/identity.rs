//! Process identity probes: tell a live process apart from a recycled PID.
//!
//! A PID alone does not name a process — once a process exits, the kernel may
//! hand its PID to an unrelated one. [`process_identity`] reads the kernel's
//! start fingerprint for a PID, so code that persisted a PID can later check
//! that it still names the same process before signaling it.
//!
//! macOS uses `proc_pidinfo(PROC_PIDTBSDINFO)`; Linux reads `/proc/<pid>/stat`.
//! Other targets report no identity, so callers must treat their PIDs as
//! unverifiable.

/// Kernel facts about one process incarnation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessIdentity {
    /// Opaque start fingerprint: two observations of one PID with equal values
    /// are the same process (macOS: start time in microseconds since the Unix
    /// epoch; Linux: start time in clock ticks since boot).
    pub start: u64,
    /// Wall-clock start time in milliseconds since the Unix epoch, when the
    /// platform exposes it.
    pub started_at_ms: Option<u64>,
    /// Parent PID; 1 once the process has been orphaned to init/launchd.
    pub ppid: u32,
    /// Short command name, when available.
    pub command: Option<String>,
}

/// Identity of the process currently holding `pid`, or `None` when no such
/// process exists or it cannot be inspected.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn process_identity(pid: u32) -> Option<ProcessIdentity> {
    let pid = libc::c_int::try_from(pid).ok().filter(|pid| *pid > 0)?;
    let size = libc::c_int::try_from(std::mem::size_of::<libc::proc_bsdinfo>()).ok()?;
    // SAFETY: `proc_bsdinfo` is plain old data, so all-zero bytes are a valid value.
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    // SAFETY: `info` is a writable buffer of exactly `size` bytes and the kernel
    // writes at most `size` bytes into it.
    let written =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDTBSDINFO, 0, (&raw mut info).cast(), size) };
    if written != size {
        return None;
    }
    let start = info
        .pbi_start_tvsec
        .checked_mul(1_000_000)?
        .checked_add(info.pbi_start_tvusec)?;
    Some(ProcessIdentity {
        start,
        started_at_ms: Some(start / 1_000),
        ppid: info.pbi_ppid,
        command: c_chars_to_string(&info.pbi_name).or_else(|| c_chars_to_string(&info.pbi_comm)),
    })
}

#[cfg(target_os = "macos")]
fn c_chars_to_string(chars: &[libc::c_char]) -> Option<String> {
    let bytes: Vec<u8> = chars
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    (!bytes.is_empty()).then(|| String::from_utf8_lossy(&bytes).into_owned())
}

/// Identity of the process currently holding `pid`, or `None` when no such
/// process exists or it cannot be inspected.
#[cfg(target_os = "linux")]
pub fn process_identity(pid: u32) -> Option<ProcessIdentity> {
    if pid == 0 {
        return None;
    }
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // `comm` is parenthesized and may itself contain spaces or parentheses.
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    let command = stat
        .get(open + 1..close)
        .filter(|comm| !comm.is_empty())
        .map(str::to_string);
    let fields: Vec<&str> = stat.get(close + 1..)?.split_whitespace().collect();
    // `fields[0]` is field 3 (`state`); `ppid` is field 4, `starttime` field 22.
    let ppid = fields.get(1)?.parse().ok()?;
    let start = fields.get(19)?.parse().ok()?;
    Some(ProcessIdentity {
        start,
        started_at_ms: linux_started_at_ms(start),
        ppid,
        command,
    })
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn linux_started_at_ms(start_ticks: u64) -> Option<u64> {
    // SAFETY: sysconf has no preconditions.
    let ticks_per_sec = u64::try_from(unsafe { libc::sysconf(libc::_SC_CLK_TCK) })
        .ok()
        .filter(|ticks| *ticks > 0)?;
    let boot_secs: u64 = std::fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("btime "))?
        .trim()
        .parse()
        .ok()?;
    boot_secs
        .checked_mul(1_000)?
        .checked_add(start_ticks.checked_mul(1_000)? / ticks_per_sec)
}

/// Identity of the process currently holding `pid`; always `None` on targets
/// without a supported probe.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn process_identity(_pid: u32) -> Option<ProcessIdentity> {
    None
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    use super::*;

    #[test]
    fn identity_of_current_process_is_stable() {
        let pid = std::process::id();
        let first = process_identity(pid).expect("identity of the current process");
        let second = process_identity(pid).expect("identity of the current process");
        assert_eq!(first.start, second.start);
        assert!(first.ppid > 0);
        assert!(first.started_at_ms.is_some());
    }

    #[test]
    fn identity_of_missing_process_is_none() {
        assert!(process_identity(0).is_none());
        assert!(process_identity(99_999_999).is_none());
    }

    #[test]
    fn child_identity_reports_parent_and_command() {
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn sleep");
        let identity = process_identity(child.id()).expect("identity of a live child");
        child.kill().expect("kill sleep");
        child.wait().expect("reap sleep");
        assert_eq!(identity.ppid, std::process::id());
        assert_eq!(identity.command.as_deref(), Some("sleep"));
    }
}
