//! Which part of the `roko` process handles SIGINT, SIGTERM and SIGHUP.
//!
//! The binary installs a process-wide SIGTERM handler that drains its shutdown
//! hooks and exits 143. A command that stops gracefully by itself — a CLI plan
//! run, `roko serve` (gap-fb1594) — claims the signals for as long as it runs
//! ([`claim_termination_signals`]), and the process-wide handler defers to it.

use std::sync::atomic::{AtomicUsize, Ordering};

/// How many commands of this process handle SIGINT/SIGTERM themselves now.
static TERMINATION_SIGNAL_CLAIMS: AtomicUsize = AtomicUsize::new(0);

/// One command's claim on SIGINT/SIGTERM; dropping it hands them back.
#[derive(Debug)]
#[must_use = "the claim ends when it is dropped"]
pub struct TerminationSignalClaim(());

impl Drop for TerminationSignalClaim {
    fn drop(&mut self) {
        TERMINATION_SIGNAL_CLAIMS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Claim SIGINT/SIGTERM for a command that stops gracefully on them by
/// itself, until the returned claim drops.
pub fn claim_termination_signals() -> TerminationSignalClaim {
    TERMINATION_SIGNAL_CLAIMS.fetch_add(1, Ordering::SeqCst);
    TerminationSignalClaim(())
}

/// Whether a command of this process handles SIGINT/SIGTERM itself now. The
/// binary's process-wide SIGTERM handler defers to it while this is `true`.
#[must_use]
pub fn cli_owns_termination_signals() -> bool {
    TERMINATION_SIGNAL_CLAIMS.load(Ordering::SeqCst) > 0
}

/// Whether SIGHUP is ignored, as `nohup` leaves it for the command it starts:
/// a command that handles SIGHUP leaves it ignored then.
#[cfg(unix)]
#[allow(unsafe_code)]
#[must_use]
pub fn hangup_ignored() -> bool {
    // SAFETY: with a null new action, sigaction(2) only reads the current
    // disposition into `current`, a zero-initialized plain C struct.
    unsafe {
        let mut current: libc::sigaction = std::mem::zeroed();
        libc::sigaction(libc::SIGHUP, std::ptr::null(), &raw mut current) == 0
            && current.sa_sigaction == libc::SIG_IGN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claims_nest_and_end_when_dropped() {
        let before = cli_owns_termination_signals();
        let first = claim_termination_signals();
        let second = claim_termination_signals();
        assert!(cli_owns_termination_signals());
        drop(first);
        assert!(cli_owns_termination_signals(), "the second claim holds");
        drop(second);
        assert_eq!(cli_owns_termination_signals(), before);
    }
}
