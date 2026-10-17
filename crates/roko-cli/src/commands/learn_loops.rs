//! `roko learn loops`: the M2 loop census (S03 T15; backlog 5107). The CLI
//! and roko-learn's `loop_census` example print the same
//! `census::render_json`. The CLI judges each loop with the workspace's
//! `[learning.audit]`, as the audit tick does, so that it agrees with the
//! ledger it summarizes (gap-ca63ee); the example judges with S03's
//! defaults.
//!
//! [`LoopsCmd`] adds S03 §5's two other forms (backlog 5134), in-process and
//! at $0: `canary <id>` traces a loop's canary dry and prints P1-P7 with the
//! first failure, and `fault <id> <kind>` breaks the loop in a
//! fault-injection build with `ROKO_FAULTS=1` and shows where the canary
//! finds the break. Without that build it says how to get one and exits 2.
//! `clear --reason <why>` clears a tripped audit (gap-1cf555), on the record.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::Context as _;
use roko_learn::loop_audit::faults::FaultKind;
use roko_learn::loop_audit::ledger::{CanaryRow, LoopAuditRow};
use roko_learn::loop_audit::{LoopAuditor, Registry, census};

use crate::loop_canary::DryCanaryRunner;

/// The exit code of a `fault` the build or the environment cannot run.
pub const EXIT_NO_FAULTS: i32 = 2;

/// `roko learn loops canary|fault|clear` (S03 §5; backlog 5134).
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
    /// Clear a tripped loop audit (an SRM alarm, a moved placebo), which
    /// freezes every loop's transitions and enforcement until a person
    /// clears it. The loop-audit ledger records who cleared it and why; the
    /// next audit tick trips it again if the cause remains.
    Clear {
        /// Why the audit may run again.
        #[arg(long)]
        reason: String,
        /// Who clears it (default: `$USER`).
        #[arg(long)]
        by: Option<String>,
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
        LoopsCmd::Clear { reason, by } => clear_audit(workdir, by.as_deref(), reason, json),
    }
}

/// Clear the tripped loop audit of `workdir` for `reason`, as `by` (else
/// `$USER`), and say what was cleared; an audit that is not tripped is left
/// as it is.
fn clear_audit(
    workdir: &Path,
    by: Option<&str>,
    reason: &str,
    json: bool,
) -> anyhow::Result<(String, i32)> {
    let by = by
        .map(str::to_string)
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_default();
    let cleared = LoopAuditor::clear_trip(workdir, &by, reason, chrono::Utc::now())
        .context("clear the loop audit (pass --by and --reason)")?;
    if json {
        return Ok((serde_json::to_string_pretty(&cleared)? + "\n", 0));
    }
    let Some(record) = cleared else {
        let idle = "the loop audit is not tripped: nothing to clear\n";
        return Ok((idle.to_string(), 0));
    };
    let tripped = match &record.row {
        LoopAuditRow::AuditCleared(row) => row.tripped.join(", "),
        _ => String::new(),
    };
    let by = by.trim();
    let reason = reason.trim();
    let out = format!("cleared the tripped loop audit ({tripped}) as {by}: {reason}\n");
    Ok((out, 0))
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
        let why = format!(
            "set {}=1 to let this process set fault flags\n",
            faults::FAULTS_ENV
        );
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
        out.push_str(
            "time to detection: measured by E1's replay, not by this command (cargo test -p \
             roko-cli --features fault-injection --test loop_audit_faults)\n",
        );
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

/// What `roko learn loops` prints for `workdir`: the report-only census, as
/// `roko.loop_census/1` JSON or as a table. It folds the logs, the
/// registry's declared findings and the runs' decision rows, whose measured
/// ε, ι and β (gap-03fcd0) decide a loop's row once they are judged, with
/// the parameters the workspace's `[learning.audit]` sets, as the audit
/// tick judges (gap-ca63ee).
///
/// # Errors
///
/// Returns an error when the workspace's loop-registry override or its
/// config does not load.
pub fn loops_output(workdir: &Path, json: bool) -> anyhow::Result<String> {
    let registry = Registry::load(workdir).context("load the loop registry")?;
    let config = roko_core::config::loader::load_config_unified(workdir)
        .context("load the workspace config for [learning.audit]")?;
    let sha = census::harness_sha(workdir);
    let audit = &config.learning.audit;
    let report = census::run_audited(workdir, &registry, sha.as_deref(), audit);
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
    /// --census --json` prints exactly the library's census JSON, judged
    /// with the workspace's `[learning.audit]`, and the table has one line
    /// per loop.
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
        let config = roko_core::config::loader::load_config_unified(workdir).expect("the config");
        let audit = &config.learning.audit;
        let report = census::run_audited(workdir, &registry, sha.as_deref(), audit);
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

    /// `record` as line `seq` of a run file, in the writer's envelope.
    fn run_line<T: serde::Serialize>(schema: &str, seq: u64, record: T) -> String {
        let stamped = roko_learn::telemetry::Stamped {
            schema_version: schema.to_string(),
            record_id: format!("b3:{seq}"),
            seq,
            ts: "2026-10-16T09:00:00Z".to_string(),
            record,
        };
        serde_json::to_string(&stamped).expect("serialize a run line") + "\n"
    }

    /// Write run `run_id` of the workspace `workdir`: `chains` chains of one
    /// attempt, every fifth on L-know's default arm (h = 0.2), each with an
    /// L-know row whose entry the request carried on the learned arm, and a
    /// verdict that passes on most learned-arm chains and on every other
    /// default-arm one.
    fn write_knowledge_run(workdir: &Path, run_id: &str, chains: usize) {
        use roko_learn::routing_log::DecisionState;
        use roko_learn::telemetry::records::{
            AttemptOutcome, AttemptVerdictRecord, AuditFields, ContentProposals, DECISION_SCHEMA,
            DecisionAssignment, DecisionReceipt, VERDICT_SCHEMA,
        };
        use roko_learn::telemetry::{
            Arm, Assignment, AssignmentUnit, AttemptIdentity, AttemptKey, ContentCandidate,
            ContentDecisionPoint, ContentDecisionRecord, RunFile,
        };

        let run_dir = workdir.join(".roko/runs").join(run_id);
        std::fs::create_dir_all(&run_dir).expect("the run dir");
        let (mut decisions, mut verdicts) = (String::new(), String::new());
        for chain in 0..chains {
            let key = AttemptKey::new(run_id, "plan", format!("t{chain}"), 1);
            let learned = chain % 5 != 0;
            let draw = Assignment {
                unit: AssignmentUnit::Chain,
                layer: "knowledge".to_string(),
                salt_id: "knowledge@2026-10-16".to_string(),
                u: 0.5,
                h: 0.2,
                g: 0.0,
                arm: if learned { Arm::Learned } else { Arm::Default },
                propensity: if learned { 0.8 } else { 0.2 },
            };
            let entry = || vec!["kn-1".to_string()];
            let row = ContentDecisionRecord {
                identity: AttemptIdentity::new(&key),
                decision_point: ContentDecisionPoint::Knowledge,
                policy: "keyword_overlap_top3".to_string(),
                candidates: vec![ContentCandidate {
                    id: "kn-1".to_string(),
                    rank: Some(1),
                    score: None,
                    eligible: true,
                    p: None,
                }],
                chosen: if learned { entry() } else { Vec::new() },
                chosen_propensity: Some(1.0),
                source: None,
                state: Some(DecisionState {
                    read: true,
                    version: "v1".to_string(),
                    digest: "b3:state".to_string(),
                    age_s: None,
                    n_obs: 1,
                    stale: false,
                }),
                thresholds_digest: None,
                arm_set: None,
                section_draws: Vec::new(),
                proposals: Some(ContentProposals {
                    learned: Some(entry()),
                    default: Some(Vec::new()),
                }),
                audit: AuditFields {
                    layer: Some("knowledge".to_string()),
                    assignment: Some(DecisionAssignment::new(draw, &key, 1)),
                    decided_at: Some(2),
                    receipt: Some(DecisionReceipt {
                        kind: "content".to_string(),
                        ok: true,
                        request_hash: Some("b3:request".to_string()),
                        exposure_hashes: vec!["b3:kn-1".to_string()],
                    }),
                    ..AuditFields::default()
                },
            };
            let seq = 2 * chain as u64;
            decisions.push_str(&run_line(DECISION_SCHEMA, seq + 1, row));
            let passed = if learned {
                chain % 7 != 0
            } else {
                chain % 10 == 0
            };
            let outcome = if passed {
                AttemptOutcome::Passed
            } else {
                AttemptOutcome::GateFailed
            };
            let verdict = AttemptVerdictRecord::settle(AttemptIdentity::new(&key), outcome, true);
            verdicts.push_str(&run_line(VERDICT_SCHEMA, seq + 2, verdict));
        }
        std::fs::write(RunFile::Decisions.path_in(&run_dir), decisions).expect("decisions");
        std::fs::write(RunFile::Attempts.path_in(&run_dir), verdicts).expect("verdicts");
    }

    /// gap-ca63ee: `roko learn loops` judges with the workspace's
    /// `[learning.audit]`, as the audit tick does. With `n_beta = 100` in
    /// `roko.toml`, L-know's 150 opportunities are enough for β: the census
    /// JSON reports it, and it is the β of the tick's health row over the
    /// same runs. S03's defaults (N_β = 200) leave it unjudged.
    #[test]
    fn learn_loops_census_reads_learning_audit_params() {
        use roko_learn::loop_audit::RowOrigin;

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::write(
            workdir.join("roko.toml"),
            "[learning.audit]\nn_beta = 100\n",
        )
        .expect("write roko.toml");
        write_knowledge_run(workdir, "gr-ca63ee", 150);

        let printed = loops_output(workdir, true).expect("the census");
        let parsed: serde_json::Value = serde_json::from_str(&printed).expect("JSON");
        let rows = parsed["rows"].as_array().expect("the census rows");
        let know = rows
            .iter()
            .find(|row| row["loop"] == "L-know")
            .expect("L-know's row");
        let measured = &know["measured"];
        assert_eq!(measured["n_opp"], 150, "{know}");
        let printed_beta = measured["beta"]["est"].as_f64();
        assert!(printed_beta.is_some(), "β is judged: {}", measured["beta"]);

        // The tick, under the same config, folds the same runs to the same β.
        let config = roko_core::config::loader::load_config_unified(workdir).expect("the config");
        assert_eq!(config.learning.audit.n_beta, 100);
        let registry = Registry::load(workdir).expect("the registry");
        let mut auditor = LoopAuditor::from_records(registry.clone(), &config.learning.audit, &[]);
        let runs = census::read_runs(&workdir.join(".roko/runs"));
        let observations = auditor.observe(&runs, &RowOrigin::default(), chrono::Utc::now());
        let health = observations
            .iter()
            .filter(|seen| seen.health.loop_id == "L-know")
            .find_map(|seen| match &seen.health.row {
                LoopAuditRow::Health(health) => Some(health.clone()),
                _ => None,
            })
            .expect("L-know's health row");
        let tick_beta = health.beta.est.expect("the tick judges β");
        let close = printed_beta.is_some_and(|beta| (beta - tick_beta).abs() < 1e-12);
        assert!(close, "census {printed_beta:?}, tick {tick_beta}");

        // S03's defaults wait for N_β = 200 opportunities.
        let defaults = census::run(workdir, &registry, None);
        let unjudged = defaults.row("L-know").and_then(|row| row.measured.clone());
        let reason = unjudged.and_then(|measured| measured.beta.reason);
        assert_eq!(reason.as_deref(), Some("n_below_n_beta"));
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
        assert_eq!(
            canary,
            LoopsCmd::Canary {
                id: "L-know".into()
            }
        );
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

    /// A loop-audit ledger whose placebo's health row tripped the audit.
    const TRIPPED: &str = concat!(
        r#"{"schema_version":"roko.loop_audit/1","kind":"loop.health","loop_id":"L-placebo","#,
        r#""state":"probation","h":0.5,"n_opp":0,"n_L":0,"n_D":0,"#,
        r#""eps":{"est":0.0,"ucb":0.0,"read":0.0,"reach":0.0,"honest":0.0,"receipt":0.0},"#,
        r#""iota":{"act":0.0,"aa":0.0,"net":0.0,"lcb":0.0},"beta":{"est":null},"#,
        r#""srm_evalue":40.0,"placebo_ok":false,"evidence":"measured"}"#,
        "\n"
    );

    /// gap-1cf555: `roko learn loops clear --reason <why> --by <who>`
    /// parses, says so when nothing is tripped, and clears a tripped audit
    /// with a `loop.audit_cleared` row in the ledger.
    #[test]
    fn learn_loops_clear_records_who_and_why() {
        use clap::Parser as _;

        #[derive(clap::Parser)]
        struct Loops {
            #[command(subcommand)]
            cmd: LoopsCmd,
        }

        let args = ["loops", "clear", "--reason", "srm fixture", "--by", "will"];
        let clear = Loops::try_parse_from(args).expect("parse clear").cmd;
        let expected = LoopsCmd::Clear {
            reason: "srm fixture".into(),
            by: Some("will".into()),
        };
        assert_eq!(clear, expected);

        let temp = tempfile::tempdir().expect("tempdir");
        let (out, code) = loops_cmd_output(temp.path(), &clear, false).expect("nothing to clear");
        assert_eq!(code, 0);
        assert!(out.contains("nothing to clear"), "{out}");

        let learn = temp.path().join(".roko/learn");
        std::fs::create_dir_all(&learn).expect("the learn dir");
        std::fs::write(learn.join("loop-audit.jsonl"), TRIPPED).expect("a tripped ledger");
        let (out, code) = loops_cmd_output(temp.path(), &clear, false).expect("the clear");
        assert_eq!(code, 0);
        assert!(
            out.contains("cleared the tripped loop audit (L-placebo) as will: srm fixture"),
            "{out}"
        );
        let ledger = std::fs::read_to_string(learn.join("loop-audit.jsonl")).expect("the ledger");
        assert!(ledger.contains("loop.audit_cleared"), "{ledger}");
    }
}
