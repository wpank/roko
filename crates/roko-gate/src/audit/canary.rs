//! The canary scanner (S05 §4.4, SC4): every hidden suite carries
//! `ROKO-CANARY-<suite_id>-<32 hex digits>` in a comment, and a canary found
//! where an agent's words go means the suite leaked.
//!
//! [`scan_text`] finds canaries in any text, ignoring case as the bench's
//! `canary.py` does; [`scan_diff`] reads a diff's added lines only, since a
//! removed line is not new text; [`scan_files`] streams files line by line
//! (`.roko/episodes.jsonl`, knowledge-store files, playbooks).
//! [`CanaryScanner::report`] appends `audit.leak_canary { place, suite_id }`
//! to the ledger for each (place, suite) pair once, with `suite_id`
//! `unknown` for a canary the store does not hold, and burns an active
//! suite: exposed, then retired, so B1 writes a replacement. The scanner
//! holds no secret: the canary form is public, and only a suite's body is
//! hidden. Its callers are the dispatch-time scan of the composed prompt
//! (7130), the settle-time scan of the output (7121), and the audit
//! worker's scan of the diff and its run-close sweep (7123).

use std::collections::HashSet;
use std::io::BufRead as _;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::hidden::{HiddenError, HiddenStore, SuiteState};
use super::ledger::{AuditEvent, AuditLedger};

/// The suite id of a canary no store holds.
pub const UNKNOWN_SUITE: &str = "unknown";

static CANARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)ROKO-CANARY-([a-z0-9][a-z0-9._-]*?)-([0-9a-f]{32})\b")
        .expect("a valid canary pattern")
});

/// One canary found in some text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanaryHit {
    /// The canary as written.
    pub canary: String,
    /// The suite id it names, as written.
    pub suite_id: String,
    /// The 1-based line it is on.
    pub line: usize,
}

/// What [`CanaryScanner::report`] logged for one (place, suite) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeakReport {
    /// Where the canary was.
    pub place: String,
    /// The suite, or [`UNKNOWN_SUITE`].
    pub suite_id: String,
    /// Whether the report burned the suite: exposed, then retired.
    pub exposed: bool,
}

/// Every canary in `text`.
#[must_use]
pub fn scan_text(text: &str) -> Vec<CanaryHit> {
    text.lines()
        .enumerate()
        .flat_map(|(index, line)| hits_in(line, index + 1))
        .collect()
}

/// Every canary on an added line of a unified diff (`+`, not `+++`).
#[must_use]
pub fn scan_diff(diff: &str) -> Vec<CanaryHit> {
    diff.lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with('+') && !line.starts_with("+++"))
        .flat_map(|(index, line)| hits_in(line, index + 1))
        .collect()
}

/// Every canary in the files at `paths`, read line by line; a missing file
/// holds none.
///
/// # Errors
///
/// A file that exists cannot be read.
pub fn scan_files(paths: &[PathBuf]) -> std::io::Result<Vec<(PathBuf, CanaryHit)>> {
    let mut found = Vec::new();
    for path in paths {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        for (index, line) in std::io::BufReader::new(file).split(b'\n').enumerate() {
            let line = String::from_utf8_lossy(&line?).into_owned();
            found.extend(hits_in(&line, index + 1).map(|hit| (path.clone(), hit)));
        }
    }
    Ok(found)
}

fn hits_in(line: &str, number: usize) -> impl Iterator<Item = CanaryHit> + '_ {
    CANARY.captures_iter(line).map(move |captures| CanaryHit {
        canary: captures[0].to_string(),
        suite_id: captures[1].to_string(),
        line: number,
    })
}

/// Logs canary hits once per (place, suite) and exposes leaked suites.
#[derive(Debug)]
pub struct CanaryScanner<'a> {
    store: &'a HiddenStore,
    reported: HashSet<(String, String)>,
}

impl<'a> CanaryScanner<'a> {
    /// A scanner over the suites of `store`.
    #[must_use]
    pub fn new(store: &'a HiddenStore) -> Self {
        Self {
            store,
            reported: HashSet::new(),
        }
    }

    /// Log `hits`, found at `place`, and burn each active suite they name:
    /// exposed, then retired. A (place, suite) pair already reported is
    /// skipped.
    ///
    /// # Errors
    ///
    /// The store or the ledger cannot be read or written.
    pub fn report(
        &mut self,
        ledger: &mut AuditLedger,
        place: &str,
        hits: &[CanaryHit],
    ) -> Result<Vec<LeakReport>, HiddenError> {
        let mut reports = Vec::new();
        for hit in hits {
            let suite = self.store.by_canary(&hit.canary)?;
            let suite_id = suite
                .as_ref()
                .map_or_else(|| UNKNOWN_SUITE.to_string(), |meta| meta.suite_id.clone());
            if !self.reported.insert((place.to_string(), suite_id.clone())) {
                continue;
            }
            ledger.append(AuditEvent::LeakCanary {
                place: place.to_string(),
                suite_id: suite_id.clone(),
            })?;
            let exposed = match suite {
                Some(meta) if meta.state == SuiteState::Active => {
                    for to in [SuiteState::Exposed, SuiteState::Retired] {
                        self.store
                            .transition(ledger, &meta.suite_id, to, "canary_hit")?;
                    }
                    true
                }
                _ => false,
            };
            reports.push(LeakReport {
                place: place.to_string(),
                suite_id,
                exposed,
            });
        }
        Ok(reports)
    }

    /// [`scan_files`] and [`Self::report`] in one, with each file's path
    /// as its place.
    ///
    /// # Errors
    ///
    /// As both.
    pub fn sweep(
        &mut self,
        ledger: &mut AuditLedger,
        paths: &[PathBuf],
    ) -> Result<Vec<LeakReport>, HiddenError> {
        let mut reports = Vec::new();
        for (path, hit) in scan_files(paths)? {
            let place = place_of(&path);
            reports.extend(self.report(ledger, &place, std::slice::from_ref(&hit))?);
        }
        Ok(reports)
    }
}

fn place_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::hidden::tests::draft;

    #[test]
    fn a_canary_in_an_episode_line_exposes_its_suite() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = HiddenStore::open_dir(&temp.path().join("hidden")).expect("a store");
        let mut ledger = AuditLedger::open_dir(&temp.path().join("ledger")).expect("a ledger");
        let mut active = |task: &str| {
            let meta = store.draft(&mut ledger, draft(task)).expect("a draft");
            let id = meta.suite_id.clone();
            for to in [SuiteState::Validated, SuiteState::Active] {
                store
                    .transition(&mut ledger, &id, to, "ok")
                    .expect("a move");
            }
            meta
        };
        let leaked = active("T1");
        let kept = active("T2");

        // A canary in an episode line, written in lower case.
        let episodes = temp.path().join("episodes.jsonl");
        let line = format!(
            "{{\"output\":\"see {}\"}}",
            leaked.canary.to_ascii_lowercase()
        );
        std::fs::write(&episodes, format!("{{\"output\":\"clean\"}}\n{line}\n")).expect("write");
        let hits = scan_files(std::slice::from_ref(&episodes)).expect("scan");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1.line, 2);
        let mut scanner = CanaryScanner::new(&store);
        let reports = scanner
            .sweep(&mut ledger, &[episodes.clone()])
            .expect("a sweep");
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].suite_id, leaked.suite_id);
        assert!(reports[0].exposed);
        let state = store.meta(&leaked.suite_id).expect("meta").state;
        assert_eq!(state, SuiteState::Retired, "a leaked suite is burned");
        let again = scanner
            .sweep(&mut ledger, &[episodes])
            .expect("a second sweep");
        assert!(again.is_empty(), "each (place, suite) is reported once");

        // On a removed diff line, a canary is not new text.
        let diff = format!("--- a/x.py\n+++ b/x.py\n-# {}\n+print(1)\n", kept.canary);
        assert!(scan_diff(&diff).is_empty());
        let added = format!("+# {}\n", kept.canary);
        assert_eq!(scan_diff(&added).len(), 1);
        let state = store.meta(&kept.suite_id).expect("meta").state;
        assert_eq!(state, SuiteState::Active);

        // A canary no store holds is logged as unknown.
        let stranger = format!("ROKO-CANARY-hs-ffffffffffff-{}", "a".repeat(32));
        let hits = scan_text(&format!("prompt text {stranger} more"));
        assert_eq!(hits.len(), 1);
        let reports = scanner
            .report(&mut ledger, "prompt", &hits)
            .expect("a report");
        assert_eq!(reports[0].suite_id, UNKNOWN_SUITE);
        assert!(!reports[0].exposed);

        let ledger_text: String = std::fs::read_dir(temp.path().join("ledger"))
            .expect("ledger")
            .filter_map(Result::ok)
            .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
            .collect();
        let leaks: Vec<&str> = ledger_text
            .lines()
            .filter(|line| line.contains("audit.leak_canary"))
            .collect();
        assert_eq!(leaks.len(), 2, "{leaks:?}");
        assert!(leaks[0].contains(&leaked.suite_id) && leaks[0].contains("episodes.jsonl"));
        assert!(
            leaks[1].contains("\"suite_id\":\"unknown\""),
            "{}",
            leaks[1]
        );
    }
}
