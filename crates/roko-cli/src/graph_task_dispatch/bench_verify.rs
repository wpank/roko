//! `bench` verify steps: a task's benchmarks must not get slower
//! (find-49ec18, P0-01).
//!
//! A `[[task.verify]]` step with `phase = "bench"` runs like any other step,
//! for example `cargo bench -p roko-core -- --message-format json`, the
//! command the gate itself runs. Once it passes,
//! [`BenchmarkRegressionGate::judge_output`] judges its Criterion JSON output
//! against the task's baseline in the main workspace,
//! `.roko/bench/baselines/<plan>-<task>.json`, not in the task's worktree,
//! which is removed after the run. The first run records the baseline; a
//! later run fails when a benchmark is slower than it by more than the gate's
//! threshold (10%). Benchmarks are slow and noisy, so only a task that
//! declares a `bench` step runs one.

use std::path::{Path, PathBuf};

use roko_core::Verdict;
use roko_gate::benchmark_gate::BenchmarkRegressionGate;

/// Phase of the verify steps whose output is judged for benchmark
/// regressions.
const BENCH_PHASE: &str = "bench";

/// The baseline of task `task_id` of plan `plan_id` in the workspace at
/// `workdir`.
fn bench_baseline_path(workdir: &Path, plan_id: &str, task_id: &str) -> PathBuf {
    let name = format!("{plan_id}-{task_id}").replace(['/', '\\', ':', ' '], "_");
    workdir
        .join(".roko")
        .join("bench")
        .join("baselines")
        .join(format!("{name}.json"))
}

/// The verdict of a verify step of `phase` once a passed `bench` step's
/// output is judged against its task's baseline. Other steps, and failed
/// ones, keep their verdict. A `bench` step whose output holds no Criterion
/// result fails: it measured nothing.
pub(super) fn judge_bench_step(
    workdir: &Path,
    plan_id: &str,
    task_id: &str,
    phase: &str,
    mut verdict: Verdict,
) -> Verdict {
    if !verdict.passed || !phase.eq_ignore_ascii_case(BENCH_PHASE) {
        return verdict;
    }
    let output = verdict.detail.take().unwrap_or_default();
    let baseline = bench_baseline_path(workdir, plan_id, task_id);
    let judged = BenchmarkRegressionGate::new().judge_output(&output, &baseline);
    let summary = judged.detail.unwrap_or_default();
    verdict.detail = Some(format!("{summary}\n\n{output}"));
    if !judged.passed {
        verdict.passed = false;
        verdict.score = 0.0;
        verdict.reason = judged.reason;
    }
    verdict
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_task_dispatch::GraphFeedbackContext;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, no_auto_fix, verify_step,
    };
    use roko_core::error::RokoError;
    use roko_graph::cell::CellContext;
    use roko_graph::cells::TaskDispatcher;

    /// Criterion JSON for one benchmark, `my_fn`, at `ns` nanoseconds.
    fn criterion_output(ns: u32) -> String {
        let typical = format!(r#"{{"estimate":{ns},"unit":"ns"}}"#);
        format!(r#"{{"reason":"benchmark-complete","id":"my_fn","typical":{typical}}}"#)
    }

    /// A Graph verify step of phase `bench` records the task's baseline on
    /// its first run, passes a run within the threshold, and fails a run
    /// whose benchmark is more than 10% slower. Its command prints canned
    /// Criterion JSON; no real `cargo bench` runs.
    #[tokio::test]
    async fn graph_bench_verify_step_fails_on_regression() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let results = temp.path().join("criterion.json");
        task.verify = vec![verify_step("bench", &format!("cat {}", results.display()))];
        let spec = make_spec(&task);
        let baseline = bench_baseline_path(temp.path(), &spec.plan_id, &task.id);

        for ns in [100, 105] {
            std::fs::write(&results, criterion_output(ns)).expect("write results");
            dispatcher
                .dispatch(&spec, Vec::new(), &CellContext::new())
                .await
                .unwrap_or_else(|error| panic!("{ns} ns passes: {error}"));
        }
        assert!(baseline.is_file(), "the first run records the baseline");

        std::fs::write(&results, criterion_output(120)).expect("write results");
        let error = dispatcher
            .dispatch(&spec, Vec::new(), &CellContext::new())
            .await
            .expect_err("a 20% slowdown fails");
        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(message.contains("regressed beyond 10.0%"), "{message}");
        assert!(message.contains("my_fn"), "{message}");
    }

    #[test]
    fn steps_of_other_phases_and_failed_steps_keep_their_verdict() {
        let temp = tempfile::tempdir().expect("tempdir");
        let empty = Verdict::pass("verify[0:compile]").with_detail("no benchmarks");
        let kept = judge_bench_step(temp.path(), "p", "T1", "compile", empty);
        assert!(kept.passed);

        let failed = Verdict::fail("verify[0:bench]", "exit code: 1");
        let kept = judge_bench_step(temp.path(), "p", "T1", "bench", failed);
        assert_eq!(kept.reason, "exit code: 1");

        let silent = Verdict::pass("verify[0:bench]").with_detail("no benchmarks");
        let judged = judge_bench_step(temp.path(), "p", "T1", "bench", silent);
        assert!(!judged.passed, "a bench step that measured nothing fails");
    }
}
