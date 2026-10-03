//! `roko learn loops`: the M2 loop census (S03 T15; backlog 5107). The CLI
//! and roko-learn's `loop_census` example print the same
//! `census::render_json`, so their output cannot drift.
//!
//! [`LoopsCmd`] adds S03 §5's two other forms (backlog 5134), in-process and
//! at $0: `canary <id>` traces a loop's canary dry and prints P1-P7 with the
//! first failure, and `fault <id> <kind>` breaks the loop in a
//! fault-injection build with `ROKO_FAULTS=1` and shows where the canary
//! finds the break. Without that build it says how to get one and exits 2.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::Context as _;
use roko_learn::loop_audit::faults::FaultKind;
use roko_learn::loop_audit::ledger::CanaryRow;
use roko_learn::loop_audit::{Registry, census};

use crate::loop_canary::DryCanaryRunner;

/// The exit code of a `fault` the build or the environment cannot run.
pub const EXIT_NO_FAULTS: i32 = 2;

/// `roko learn loops canary|fault` (S03 §5; backlog 5134).
#[derive(Debug, Clone, PartialEq, Eq, clap::Subcommand)]
pub enum LoopsCmd {
    /// Trace a loop's canary, dry and in-process: write its nonce artifact,
    /// plan the canary task, and print P1-P7 with the first failure.
    Canary {
        /// The loop, e.g. `L-know` (L-know and L-play have canaries).
        id: String,
    },
    /// Break a loop and show where its canary finds the break
    /// (fault-injection builds with `ROKO_FAULTS=1` only).
    Fault {
        /// The loop, e.g. `L-know`.
        id: String,
        /// How to break it: cut, stale, degenerate, mask, unlogged or
        /// label_only (HARMFUL runs live only, never here).
        kind: String,
    },
}

/// What `roko learn loops canary|fault` prints for `workdir`, and its exit
/// code: 0 once the canary traced, [`EXIT_NO_FAULTS`] when the build or the
/// environment has no fault flags.
///
/// # Errors
///
/// A loop without a canary, an unknown fault kind, a flag that could not be
/// set, or a canary row that could not be written.
pub fn loops_cmd_output(
    workdir: &Path,
    cmd: &LoopsCmd,
    json: bool,
) -> anyhow::Result<(String, i32)> {
    match cmd {
        LoopsCmd::Canary { id } => {
            let row = DryCanaryRunner::new(workdir)
                .trace(id)
                .map_err(anyhow::Error::msg)?;
            Ok((render_canary(id, &row, json)?, 0))
        }
        LoopsCmd::Fault { id, kind } => {
            let kind: FaultKind = serde_json::from_value(serde_json::Value::from(kind.as_str()))
                .with_context(|| format!("unknown fault kind {kind:?}"))?;
            break_loop(workdir, id, kind, json)
        }
    }
}

/// `row`, `loop_id`'s canary trace, as JSON or as one line per probe.
fn render_canary(loop_id: &str, row: &CanaryRow, json: bool) -> anyhow::Result<String> {
    if json {
        return Ok(serde_json::to_string_pretty(row)? + "\n");
    }
    let first = row.first_failure.as_deref().unwrap_or("none");
    let mut out = format!(
        "canary {loop_id} (nonce {}, dry run): first failure: {first}\n",
        row.nonce
    );
    for probe in &row.probes {
        let verdict = if probe.ok { "ok" } else { "FAIL" };
        let evidence = probe.evidence.as_deref().unwrap_or("");
        let _ = writeln!(out, "  {} {verdict:4} {evidence}", probe.p);
    }
    Ok(out)
}

/// Break `loop_id` with `kind` for a dry canary trace, and show where the
/// canary finds the break.
#[cfg(feature = "fault-injection")]
fn break_loop(
    workdir: &Path,
    loop_id: &str,
    kind: FaultKind,
    json: bool,
) -> anyhow::Result<(String, i32)> {
    use roko_learn::loop_audit::faults::{self, FaultSpec};

    if kind == FaultKind::Harmful {
        let why = "HARMFUL runs only on live runs, under its spend cap; this command breaks \
                   loops dry\n";
        return Ok((why.to_string(), EXIT_NO_FAULTS));
    }
    let learn_dir = roko_fs::RokoLayout::for_project(workdir).learn_dir();
    if !faults::enable_from_env(learn_dir.join("cli-faults.jsonl")) {
        let why = format!("set {}=1 to let this process set fault flags\n", faults::FAULTS_ENV);
        return Ok((why, EXIT_NO_FAULTS));
    }
    let spec = FaultSpec {
        loop_id: loop_id.to_string(),
        kind,
        ttl_secs: 600,
        max_decisions: 1_000,
        spend_cap_usd: None,
    };
    faults::set(spec).context("set the fault flag")?;
    let traced = DryCanaryRunner::new(workdir).trace(loop_id);
    faults::clear(loop_id);
    faults::disable();
    let row = traced.map_err(anyhow::Error::msg)?;
    let mut out = render_canary(loop_id, &row, json)?;
    if !json {
        // E1's replay (backlog 5130) measures the time to detection.
        out.push_str("time to detection: not measured by this command (E1, backlog 5130)\n");
    }
    Ok((out, 0))
}

/// A build without fault flags explains how to get one.
#[cfg(not(feature = "fault-injection"))]
fn break_loop(
    _workdir: &Path,
    loop_id: &str,
    kind: FaultKind,
    _json: bool,
) -> anyhow::Result<(String, i32)> {
    let kind = serde_json::to_value(kind)?;
    let why = format!(
        "this roko has no fault flags, so it cannot break {loop_id} with {kind}: build it with \
         `cargo build -p roko-cli --features fault-injection` and run with ROKO_FAULTS=1\n"
    );
    Ok((why, EXIT_NO_FAULTS))
}

/// What `roko learn loops` prints for `workdir`: the report-only census, the
/// only mode until the measured audit (backlog 5123) lands, as
/// `roko.loop_census/1` JSON or as a table.
///
/// # Errors
///
/// Returns an error when the workspace's loop-registry override does not
/// load.
pub fn loops_output(workdir: &Path, json: bool) -> anyhow::Result<String> {
    let registry = Registry::load(workdir).context("load the loop registry")?;
    let sha = census::harness_sha(workdir);
    let report = census::run(workdir, &registry, sha.as_deref());
    if json {
        Ok(census::render_json(&report)?)
    } else {
        Ok(census::render_text(&report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Copy the directory tree at `from` to `to`.
    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).expect("create the copy");
        for entry in std::fs::read_dir(from).expect("read the fixture") {
            let path = entry.expect("a fixture entry").path();
            let target = to.join(path.file_name().expect("a file name"));
            if path.is_dir() {
                copy_tree(&path, &target);
            } else {
                std::fs::copy(&path, &target).expect("copy a fixture file");
            }
        }
    }

    /// S03 T15: on the 09-29 fixture (backlog 5105), `roko learn loops
    /// --census --json` prints exactly the library's census JSON, and the
    /// table has one line per loop.
    #[test]
    fn learn_loops_census_json_matches_library() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../roko-learn/tests/fixtures/loop_census");
        let temp = tempfile::tempdir().expect("tempdir");
        copy_tree(&fixture, &temp.path().join(".roko"));
        let workdir = temp.path();

        let printed = loops_output(workdir, true).expect("the census");
        let registry = Registry::load(workdir).expect("the registry");
        let sha = census::harness_sha(workdir);
        let report = census::run(workdir, &registry, sha.as_deref());
        assert_eq!(printed, census::render_json(&report).expect("render"));
        let parsed: serde_json::Value = serde_json::from_str(&printed).expect("JSON");
        assert_eq!(parsed["schema"], census::CENSUS_SCHEMA);
        let linucb = report.row("L-linucb").expect("an L-linucb row");
        assert_eq!(
            linucb.reason.map(|reason| reason.as_str()),
            Some("dormant:no_learning")
        );

        let table = loops_output(workdir, false).expect("the table");
        assert_eq!(table.lines().count(), registry.loops().len() + 1, "{table}");
        assert!(
            table
                .lines()
                .any(|line| line.starts_with("L-linucb") && line.contains("dormant:no_learning")),
            "{table}"
        );
    }
    /// S03 §5 (backlog 5134): `roko learn loops canary <id>` and `roko learn
    /// loops fault <id> <kind>` parse. The canary traces L-know dry in a fresh
    /// workspace, its first failure P6 (a dry run writes no decision row),
    /// and without the fault-injection feature `fault` says how to build one
    /// and exits 2.
    #[test]
    fn learn_loops_canary_and_fault_subcommands() {
        use clap::Parser as _;

        #[derive(clap::Parser)]
        struct Loops {
            #[command(subcommand)]
            cmd: LoopsCmd,
        }

        let canary = Loops::try_parse_from(["loops", "canary", "L-know"])
            .expect("parse canary")
            .cmd;
        assert_eq!(canary, LoopsCmd::Canary { id: "L-know".into() });
        let fault = Loops::try_parse_from(["loops", "fault", "L-know", "cut"])
            .expect("parse fault")
            .cmd;
        let expected = LoopsCmd::Fault {
            id: "L-know".into(),
            kind: "cut".into(),
        };
        assert_eq!(fault, expected);

        let temp = tempfile::tempdir().expect("tempdir");
        let (out, code) = loops_cmd_output(temp.path(), &canary, false).expect("a canary trace");
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("first failure: P6"), "{out}");
        assert!(out.lines().any(|line| line.contains("P4 ok")), "{out}");

        let (out, code) = loops_cmd_output(temp.path(), &fault, false).expect("a fault answer");
        if !cfg!(feature = "fault-injection") {
            assert_eq!(code, EXIT_NO_FAULTS, "{out}");
            assert!(out.contains("--features fault-injection"), "{out}");
        }
        let unknown = LoopsCmd::Fault {
            id: "L-know".into(),
            kind: "sideways".into(),
        };
        assert!(loops_cmd_output(temp.path(), &unknown, false).is_err());
    }
}
