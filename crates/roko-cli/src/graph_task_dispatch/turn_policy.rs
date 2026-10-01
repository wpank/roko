//! Limits and failure reasons of one Graph task attempt: express tasks, turn caps,
//! attempt timeouts, and the class-prefixed reason a failed attempt records.

use roko_learn::tier_limits::LearnedTierLimits;

use super::*;

/// P3-AGT-2: Express mode turn limit for mechanical/trivial tasks.
///
/// When express mode is active, these tasks get a 5-turn budget instead of
/// their tier's `[pipeline.<tier>] max_turns`. Mechanical fixes typically
/// need only 1-3 turns (read files, write patch, done).
const EXPRESS_MAX_TURNS: u32 = 5;

/// Return `true` when the task qualifies for express dispatch.
///
/// Express mode is enabled when:
/// - `conductor.express_mode = true` in the workspace config, AND
/// - The task tier reads as mechanical ([`TaskDef::tier_class`]: `mechanical`,
///   `trivial`, `fast`, ...).
///
/// When active, the dispatcher:
/// - Routes to `routing.fast_task_model` (cheapest available model).
/// - Skips the eval-generation pre-dispatch step.
/// - Caps the agent turn limit at [`EXPRESS_MAX_TURNS`].
pub(super) fn is_express_task(
    config: &roko_core::config::schema::RokoConfig,
    task: &TaskDef,
) -> bool {
    config.conductor.express_mode && task.tier_class() == roko_core::task::TaskTier::Mechanical
}

/// [`task_turn_limit_with`] without learned tier limits.
#[cfg(test)]
pub(super) fn task_turn_limit(config: &RokoConfig, task: &TaskDef, express_active: bool) -> u32 {
    task_turn_limit_with(config, None, task, express_active)
}

/// Provider turn cap for one Graph task dispatch.
///
/// Every task gets its tier's `[pipeline.<tier>] max_turns` (unknown tiers
/// read as focused, so the cap is never unbounded). With the workspace's
/// `learned` tier limits and `[pipeline] learned_limits = "on"`, a tier with
/// enough history gets its learned cap instead (gap-5a6e01). Express
/// dispatch lowers the cap further to [`EXPRESS_MAX_TURNS`]. The provider
/// adapter decides how the cap binds (`ProviderAdapter::turn_cap_enforcement`),
/// and agent construction warns when a provider can treat it only as
/// advisory.
pub(super) fn task_turn_limit_with(
    config: &RokoConfig,
    learned: Option<&LearnedTierLimits>,
    task: &TaskDef,
    express_active: bool,
) -> u32 {
    let tier = task.tier_class();
    let tier_limit = learned
        .and_then(|learned| learned.applied(tier).max_turns)
        .unwrap_or_else(|| config.pipeline.max_turns_for_tier(tier));
    if express_active {
        tier_limit.min(EXPRESS_MAX_TURNS)
    } else {
        tier_limit
    }
}

/// An attempt that stopped at its turn cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct TurnCapRetry {
    /// Cap the stopped attempt ran with.
    pub(super) cap: u32,
    /// Turns the provider reported using, when it said.
    pub(super) num_turns: Option<u32>,
}

/// Cap for the attempt after one that stopped at `cap`: half again, and
/// always at least one more turn.
pub(super) fn raised_turn_cap(cap: u32) -> u32 {
    cap.saturating_add(cap.div_ceil(2))
        .max(cap.saturating_add(1))
}

/// Section appended to the user prompt of the attempt after a turn-cap stop,
/// so the agent continues the partial work instead of starting over.
pub(super) fn turn_cap_resume_note(previous: TurnCapRetry, cap: u32) -> String {
    let used = previous
        .num_turns
        .map_or_else(String::new, |turns| format!(" after {turns} turns"));
    format!(
        "\n\n# Resuming a partially completed task\n\n\
         Your previous attempt at this task stopped at its {prev_cap}-turn cap{used}. \
         Its edits are still in the working tree. Inspect the current state of the \
         files in scope (for example with `git diff`), keep what is already correct, \
         and continue from there instead of starting over. This attempt has a \
         {cap}-turn cap.\n",
        prev_cap = previous.cap,
    )
}

/// Ceiling on timeout escalation, as a multiple of a task's base attempt
/// timeout.
const MAX_TIMEOUT_ESCALATION: u64 = 4;

/// [`base_attempt_timeout_ms_with`] without learned tier limits.
#[cfg(test)]
pub(super) fn base_attempt_timeout_ms(config: &RokoConfig, spec: &TaskExecutionSpec) -> u64 {
    base_attempt_timeout_ms_with(config, None, spec)
}

/// Wall-clock budget of one Graph task attempt, in ms: the task's authored
/// `timeout_secs`, which always wins; else, with the workspace's `learned`
/// tier limits and `[pipeline] learned_limits = "on"`, the learned timeout of
/// a tier with enough history (gap-5a6e01); else
/// `timeouts.agent_dispatch_secs`.
pub(super) fn base_attempt_timeout_ms_with(
    config: &RokoConfig,
    learned: Option<&LearnedTierLimits>,
    spec: &TaskExecutionSpec,
) -> u64 {
    if spec.timeout_secs != 0 {
        return spec.timeout_secs.saturating_mul(1_000);
    }
    let tier = roko_core::task::TaskTier::parse(&spec.tier).unwrap_or_default();
    let configured = config
        .timeouts
        .agent_dispatch_secs
        .max(1)
        .saturating_mul(1_000);
    learned
        .and_then(|learned| learned.applied(tier).timeout_ms)
        .unwrap_or(configured)
}

impl GraphTaskDispatcher {
    /// Owe `retry` to `plan_id/task_id`'s next attempt, kept with the run's
    /// retry state so a resumed run raises the cap as well (gap-34b2ed).
    pub(super) fn keep_turn_cap_retry(&self, plan_id: &str, task_id: &str, retry: TurnCapRetry) {
        let key = format!("{plan_id}/{task_id}");
        self.turn_cap_retries.lock().insert(key, retry);
        self.gate_retry_context
            .set_turn_cap(plan_id, task_id, Some(retry));
    }

    /// The turn-cap retry owed to `plan_id/task_id`'s next attempt, which
    /// takes it.
    pub(super) fn take_turn_cap_retry(&self, plan_id: &str, task_id: &str) -> Option<TurnCapRetry> {
        let key = format!("{plan_id}/{task_id}");
        let retry = self.turn_cap_retries.lock().remove(&key);
        if retry.is_some() {
            let kept = &self.gate_retry_context;
            kept.set_turn_cap(plan_id, task_id, None);
        }
        retry
    }

    /// The workspace's learned tier limits (gap-5a6e01), read on the first
    /// dispatch from the settled attempts under the feedback `runs_dir` and
    /// the tiers in its `costs_path`. None without a runs directory.
    pub(super) fn learned_tier_limits(&self) -> &LearnedTierLimits {
        self.learned_tier_limits.get_or_init(|| {
            self.feedback
                .runs_dir
                .as_deref()
                .map_or_else(LearnedTierLimits::default, |runs_dir| {
                    LearnedTierLimits::load(
                        &self.config,
                        runs_dir,
                        self.feedback.costs_path.as_deref(),
                    )
                })
        })
    }
}

/// Timeout for the attempt after one that ran out of `timeout_ms`: half
/// again, at most [`MAX_TIMEOUT_ESCALATION`] times the base, and never less
/// than the timeout that ran out.
pub(super) fn raised_attempt_timeout_ms(timeout_ms: u64, base_ms: u64) -> u64 {
    timeout_ms
        .saturating_add(timeout_ms.div_ceil(2))
        .min(base_ms.saturating_mul(MAX_TIMEOUT_ESCALATION))
        .max(timeout_ms)
}

/// Section appended to the user prompt of the attempt after a timeout, so the
/// agent continues the partial work instead of starting over.
pub(super) fn timeout_resume_note(previous_ms: u64, timeout_ms: u64) -> String {
    format!(
        "\n\n# Resuming a timed-out task\n\n\
         Your previous attempt at this task was stopped when it ran out of its \
         {previous:?} time limit. Its edits are still in the working tree. Inspect the \
         current state of the files in scope (for example with `git diff`), keep what is \
         already correct, and continue from there instead of starting over. This attempt \
         has {timeout:?}.\n",
        previous = std::time::Duration::from_millis(previous_ms),
        timeout = std::time::Duration::from_millis(timeout_ms),
    )
}

/// Longest failure reason recorded on an episode, in bytes.
const MAX_FAILURE_REASON_BYTES: usize = 2_048;

/// Class-prefixed reason for a failed attempt (`"<class>: <detail>"`),
/// recorded on its episode. A reason within [`MAX_FAILURE_REASON_BYTES`]
/// keeps every line, so a verify summary keeps the step that failed; a
/// longer one keeps its first lines and its tail around an omission marker.
pub(super) fn attempt_failure_reason(class: &str, detail: &str) -> String {
    let detail = detail.trim();
    let detail = if detail.is_empty() {
        "no detail"
    } else {
        detail
    };
    let budget = MAX_FAILURE_REASON_BYTES.saturating_sub(class.len() + 2);
    format!("{class}: {}", head_and_tail(detail, budget))
}

/// `text` when it fits in `max` bytes; otherwise its head and tail, cut at
/// line breaks near the cut points, joined by `… N bytes omitted …`.
pub(super) fn head_and_tail(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    // Room for the "\n… N bytes omitted …\n" marker.
    let budget = max.saturating_sub(48);
    let mut head_end = budget / 2;
    while !text.is_char_boundary(head_end) {
        head_end -= 1;
    }
    let head = &text[..head_end];
    let head = head
        .rfind('\n')
        .filter(|&cut| cut >= head_end * 3 / 4)
        .map_or(head, |cut| &head[..cut]);
    let mut tail_start = text.len() - (budget - budget / 2);
    while !text.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    let tail = &text[tail_start..];
    let tail = tail
        .find('\n')
        .filter(|&cut| cut <= tail.len() / 4)
        .map_or(tail, |cut| &tail[cut + 1..]);
    let omitted = text.len() - head.len() - tail.len();
    format!("{head}\n… {omitted} bytes omitted …\n{tail}")
}

/// [`attempt_failure_reason`] for failed verification: the verify summary
/// itself, which leads with any `blocked_by_sibling = <task>` blame, without
/// the error's `gate error (…)` wrapper.
pub(super) fn verify_failure_reason(error: &RokoError) -> String {
    match error {
        RokoError::Verify { message, .. } => attempt_failure_reason("verify", message),
        other => attempt_failure_reason("verify", &other.to_string()),
    }
}

/// How an unsuccessful provider result, or a provider call that errored,
/// ended (S01 §4.3), read from its message.
pub(super) fn provider_failure_outcome(message: &str) -> AttemptOutcome {
    use roko_agent::provider::error_classify::{
        detect_attempt_timeout, detect_provider_exhaustion, detect_turn_cap,
    };

    if detect_turn_cap(message).is_some() {
        AttemptOutcome::TurnCap
    } else if detect_provider_exhaustion(message).is_some() {
        AttemptOutcome::ProviderExhausted
    } else if detect_attempt_timeout(message) {
        AttemptOutcome::Timeout
    } else {
        AttemptOutcome::ProviderError
    }
}

/// [`attempt_failure_reason`] for an attempt the stall watchdog cancelled
/// (bug-4c553b).
pub(super) fn stall_failure_reason(message: &str) -> String {
    attempt_failure_reason("timeout", message)
}

/// [`attempt_failure_reason`] for an unsuccessful provider result.
pub(super) fn provider_failure_reason(message: &str) -> String {
    let class = match provider_failure_outcome(message) {
        AttemptOutcome::TurnCap => "turn_cap",
        AttemptOutcome::ProviderExhausted => "provider_exhausted",
        AttemptOutcome::Timeout => "timeout",
        _ => "provider",
    };
    attempt_failure_reason(class, message)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        STREAMS_THEN_TIMES_OUT_PROVIDER, TIMEOUT_SECS_UNDER_LOAD, batch_ctx, make_batch_dispatcher,
        make_scripted_batch_dispatcher, make_spec, make_task_def,
    };
    use crate::graph_task_dispatch::verification::verify_failure_summary;

    // ── P3-AGT-2: Express mode unit tests ─────────────────────────────────

    #[test]
    fn express_mode_disabled_by_default() {
        let config = roko_core::config::schema::RokoConfig::default();
        assert!(
            !config.conductor.express_mode,
            "express_mode must be false by default"
        );
        let task = make_task_def("mechanical");
        assert!(
            !is_express_task(&config, &task),
            "is_express_task must return false when express_mode = false"
        );
    }

    #[test]
    fn express_mode_activates_for_mechanical_and_trivial() {
        let mut config = roko_core::config::schema::RokoConfig::default();
        config.conductor.express_mode = true;
        config.routing.fast_task_model = "claude-haiku-4-5".to_string();

        for tier in &["mechanical", "trivial", "Mechanical", "TRIVIAL"] {
            let task = make_task_def(tier);
            assert!(
                is_express_task(&config, &task),
                "is_express_task must return true for tier={tier} when express_mode=true"
            );
        }
    }

    #[test]
    fn express_mode_does_not_activate_for_standard_or_above() {
        let mut config = roko_core::config::schema::RokoConfig::default();
        config.conductor.express_mode = true;

        for tier in &["focused", "standard", "integrative", "architectural", ""] {
            let task = make_task_def(tier);
            assert!(
                !is_express_task(&config, &task),
                "is_express_task must return false for tier={tier}"
            );
        }
    }

    #[test]
    fn express_max_turns_is_less_than_theta_default() {
        let theta_default = roko_core::operating_frequency::OperatingFrequency::Theta.turn_limit();
        assert!(
            EXPRESS_MAX_TURNS < theta_default,
            "EXPRESS_MAX_TURNS ({EXPRESS_MAX_TURNS}) must be less than Theta default ({theta_default})"
        );
    }

    #[test]
    fn every_task_gets_a_bounded_tier_turn_cap() {
        let mut config = RokoConfig::default();
        let turns =
            |config: &RokoConfig, tier: &str| task_turn_limit(config, &make_task_def(tier), false);
        assert_eq!(turns(&config, "mechanical"), 40);
        assert_eq!(turns(&config, "focused"), 60);
        assert_eq!(turns(&config, "integrative"), 90);
        assert_eq!(turns(&config, "architectural"), 120);
        assert_eq!(
            turns(&config, "unheard-of"),
            60,
            "unknown tiers use the focused band"
        );

        config.pipeline.integrative.max_turns = 45;
        assert_eq!(turns(&config, "integrative"), 45);

        config.conductor.express_mode = true;
        let mechanical = make_task_def("mechanical");
        assert!(is_express_task(&config, &mechanical));
        assert_eq!(
            task_turn_limit(&config, &mechanical, true),
            EXPRESS_MAX_TURNS
        );
        config.pipeline.mechanical.max_turns = 3;
        assert_eq!(
            task_turn_limit(&config, &mechanical, true),
            3,
            "express mode never raises the tier cap"
        );
    }

    #[tokio::test]
    async fn batch_dispatch_passes_the_tier_turn_cap_to_the_provider() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |config| {
            config.pipeline.focused.max_turns = 7;
        })
        .await;
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
            .await
            .expect("dispatch");
        let args = std::fs::read_to_string(temp.path().join("provider-args"))
            .expect("the provider recorded its arguments");
        assert!(args.contains("--max-turns 7"), "provider args: {args}");
    }

    #[test]
    fn a_turn_cap_retry_raises_the_cap_by_half() {
        assert_eq!(raised_turn_cap(60), 90);
        assert_eq!(raised_turn_cap(1), 2);
        assert_eq!(raised_turn_cap(2), 3);
        assert_eq!(raised_turn_cap(u32::MAX), u32::MAX);
    }

    #[test]
    fn provider_failures_get_a_class_prefixed_reason() {
        let hit = roko_agent::provider::error_classify::TurnCapHit {
            num_turns: Some(61),
            cap: Some(60),
        };
        assert!(provider_failure_reason(&hit.to_string()).starts_with("turn_cap: agent turn cap"));
        assert!(
            provider_failure_reason("provider usage exhausted: You've hit your session limit")
                .starts_with("provider_exhausted: ")
        );
        assert_eq!(
            provider_failure_reason("timed out after 600000 ms"),
            "timeout: timed out after 600000 ms"
        );
        assert_eq!(
            provider_failure_reason("exit 1: claude failed\nmore"),
            "provider: exit 1: claude failed\nmore"
        );
    }

    #[test]
    fn a_verify_failure_reason_keeps_the_failing_step() {
        let summary = verify_failure_summary(
            "Write the greeting",
            3,
            &[
                "verify[1:test] `cargo test -p greet` failed: exit code: 101\n\
               thread 'greets' panicked at src/lib.rs:4:5"
                    .to_string(),
            ],
            &["verify[2:lint] (`cargo clippy`)".to_string()],
        );
        let reason = verify_failure_reason(&RokoError::Verify {
            gate: "graph-verify".into(),
            message: summary,
        });
        assert!(reason.starts_with("verify: 1/3 verify step(s) failed for task"));
        assert!(reason.contains("verify[1:test] `cargo test -p greet` failed"));
        assert!(reason.contains("panicked at src/lib.rs:4:5"));
        assert!(
            reason.ends_with("Skipped after the first failure: verify[2:lint] (`cargo clippy`)")
        );
        assert_eq!(
            attempt_failure_reason("verify", "  \n"),
            "verify: no detail"
        );
    }

    #[test]
    fn a_long_failure_reason_keeps_its_head_and_tail_within_the_bound() {
        let detail = (0..400)
            .map(|line| format!("line {line:03} of the failing é output"))
            .collect::<Vec<_>>()
            .join("\n");
        let reason = attempt_failure_reason("verify", &detail);

        assert!(reason.len() <= MAX_FAILURE_REASON_BYTES, "{}", reason.len());
        assert!(reason.starts_with("verify: line 000 of the failing é output\n"));
        assert!(reason.ends_with("line 399 of the failing é output"));
        assert!(reason.contains(" bytes omitted …\n"));
        // Cuts land on line breaks, so no line is kept in part.
        assert!(
            reason.lines().all(|line| (line.starts_with("verify: line ")
                || line.starts_with("line "))
                && line.ends_with(" output")
                || line.contains("bytes omitted")),
            "{reason}"
        );
    }

    #[test]
    fn a_timeout_retry_raises_the_timeout_by_half_up_to_four_times_the_base() {
        assert_eq!(raised_attempt_timeout_ms(600_000, 600_000), 900_000);
        assert_eq!(raised_attempt_timeout_ms(900_000, 600_000), 1_350_000);
        assert_eq!(raised_attempt_timeout_ms(2_025_000, 600_000), 2_400_000);
        assert_eq!(raised_attempt_timeout_ms(2_400_000, 600_000), 2_400_000);
        assert_eq!(
            raised_attempt_timeout_ms(3_000_000, 600_000),
            3_000_000,
            "never below the timeout that ran out"
        );
        assert_eq!(raised_attempt_timeout_ms(1_000, 1_000), 1_500);
        assert_eq!(raised_attempt_timeout_ms(u64::MAX, u64::MAX), u64::MAX);
    }

    /// gap-5a6e01: with `learned_limits = "on"` a tier's learned cap and
    /// timeout replace the configured ones; `shadow` and `off` change nothing,
    /// and an authored timeout always wins.
    #[test]
    fn learned_tier_limits_reach_turn_cap_and_timeout() {
        use roko_core::config::gates::LearnedLimitsMode;
        use roko_core::task::TaskTier;
        use roko_learn::telemetry::AttemptOutcome;
        use roko_learn::tier_limits::TierAttempt;

        let passed = |turns: u32, agent_ms: u64| TierAttempt {
            tier: TaskTier::Focused,
            outcome: AttemptOutcome::Passed,
            passed: true,
            turns: Some(turns),
            agent_ms: Some(agent_ms),
            settled_at: None,
        };
        // 40 focused passes, half at 20 turns and 200 s, half at 40 turns and
        // 400 s: the p95 times 1.25 is 50 turns and 500 s.
        let attempts: Vec<TierAttempt> = (0..40)
            .map(|n| {
                if n % 2 == 0 {
                    passed(20, 200_000)
                } else {
                    passed(40, 400_000)
                }
            })
            .collect();
        let mut config = RokoConfig::default();
        config.timeouts.agent_dispatch_secs = 600;
        config.pipeline.learned_limits = LearnedLimitsMode::On;
        let learned = LearnedTierLimits::from_attempts(&config, &attempts);
        let focused = make_task_def("focused");
        let spec = make_spec(&focused);

        assert_eq!(
            task_turn_limit_with(&config, Some(&learned), &focused, false),
            50
        );
        assert_eq!(
            base_attempt_timeout_ms_with(&config, Some(&learned), &spec),
            500_000
        );
        assert_eq!(
            task_turn_limit_with(&config, Some(&learned), &focused, true),
            EXPRESS_MAX_TURNS,
            "express still lowers the learned cap"
        );
        let mechanical = make_task_def("mechanical");
        assert_eq!(
            task_turn_limit_with(&config, Some(&learned), &mechanical, false),
            40,
            "a tier without history keeps its configured cap"
        );
        let mut authored = focused.clone();
        authored.timeout_secs = 900;
        assert_eq!(
            base_attempt_timeout_ms_with(&config, Some(&learned), &make_spec(&authored)),
            900_000,
            "an authored timeout wins"
        );

        for mode in [LearnedLimitsMode::Shadow, LearnedLimitsMode::Off] {
            config.pipeline.learned_limits = mode;
            let learned = LearnedTierLimits::from_attempts(&config, &attempts);
            assert_eq!(
                task_turn_limit_with(&config, Some(&learned), &focused, false),
                60
            );
            assert_eq!(
                base_attempt_timeout_ms_with(&config, Some(&learned), &spec),
                600_000
            );
        }
        assert_eq!(task_turn_limit(&config, &focused, false), 60);
        assert_eq!(base_attempt_timeout_ms(&config, &spec), 600_000);
    }

    /// gap-5a6e01: dispatch learns its tier limits from the settled attempts
    /// under its runs directory, so with `learned_limits = "on"` a focused
    /// task gets its tier's learned turn cap; its own open line names the
    /// tier for the next run to learn from.
    #[tokio::test]
    async fn dispatch_reads_learned_tier_limits_from_its_runs() {
        use roko_core::config::gates::LearnedLimitsMode;
        use roko_learn::telemetry::records::ATTEMPTS_FILE;
        use roko_learn::telemetry::{
            AttemptIdentity, AttemptKey, AttemptOpenRecord, AttemptOutcome, AttemptVerdictRecord,
            Stamped, TelemetryRecord,
        };

        fn stamped<T: TelemetryRecord>(record: T) -> String {
            serde_json::to_string(&Stamped {
                schema_version: T::SCHEMA.to_string(),
                record_id: String::new(),
                seq: 0,
                ts: String::new(),
                record,
            })
            .expect("serialize line")
        }

        // 40 focused passes, half at 20 turns and half at 40: the p95 times
        // 1.25 is 50 turns.
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let history: Vec<String> = (1..=40)
            .flat_map(|ordinal| {
                let key = AttemptKey::new("history", "plan", "T", ordinal);
                let mut open = AttemptOpenRecord::new(AttemptIdentity::new(&key), 1_000);
                open.tier = Some("focused".to_string());
                let mut verdict = AttemptVerdictRecord::settle(
                    AttemptIdentity::new(&key),
                    AttemptOutcome::Passed,
                    true,
                );
                verdict.executed.turns = Some(if ordinal % 2 == 0 { 20 } else { 40 });
                [stamped(open), stamped(verdict)]
            })
            .collect();
        std::fs::create_dir_all(runs_dir.join("history")).expect("history run");
        std::fs::write(
            runs_dir.join("history").join(ATTEMPTS_FILE),
            history.join("\n"),
        )
        .expect("write history");

        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |config| {
            config.pipeline.learned_limits = LearnedLimitsMode::On;
        })
        .await;
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        });
        let ctx = batch_ctx().with_run_id("now".to_string());
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("dispatch");
        let args = std::fs::read_to_string(temp.path().join("provider-args"))
            .expect("the provider recorded its arguments");
        assert!(args.contains("--max-turns 50"), "provider args: {args}");

        dispatcher.close_run_attempts("now");
        let lines = std::fs::read_to_string(runs_dir.join("now").join(ATTEMPTS_FILE))
            .expect("this run's attempt log");
        let open: serde_json::Value = lines
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .find(|line: &serde_json::Value| line["schema_version"] == "roko.attempt_open/1")
            .expect("an open line");
        assert_eq!(open["tier"], "focused");
    }

    #[test]
    fn the_base_attempt_timeout_is_the_authored_timeout_else_the_dispatch_default() {
        let mut config = RokoConfig::default();
        config.timeouts.agent_dispatch_secs = 600;
        let mut task = make_task_def("focused");
        assert_eq!(base_attempt_timeout_ms(&config, &make_spec(&task)), 600_000);
        task.timeout_secs = 2_700;
        assert_eq!(
            base_attempt_timeout_ms(&config, &make_spec(&task)),
            2_700_000
        );
    }

    /// A fake Claude CLI that stops at its turn cap on the first call and
    /// finishes on the second, recording each call's args and prompt.
    const TURN_CAP_THEN_SUCCESS_PROVIDER: &str = r#"#!/bin/sh
dir=$(dirname -- "$0")
n=$(( $(cat "$dir/calls" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$dir/calls"
printf '%s\n' "$*" >> "$dir/provider-args"
cat > "$dir/prompt-$n"
if [ ! -f "$dir/stopped-once" ]; then
  touch "$dir/stopped-once"
  printf '%s\n' '{"type":"result","subtype":"error_max_turns","is_error":true,"num_turns":61,"total_cost_usd":0.02}'
  exit 1
fi
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"finished"}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"num_turns":7,"total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    #[tokio::test]
    async fn a_turn_cap_stop_is_classified_then_resumed_with_a_raised_cap() {
        let temp = tempdir().expect("tempdir");
        let episodes_path = temp.path().join("episodes.jsonl");
        let (dispatcher, task) =
            make_scripted_batch_dispatcher(&temp, TURN_CAP_THEN_SUCCESS_PROVIDER, |_| {}).await;
        let facade = crate::runtime_feedback::FeedbackFacade::new().with_sink(Arc::new(
            crate::runtime_feedback::EpisodeSink::at(&episodes_path),
        ));
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            feedback_facade: Some(Arc::new(facade)),
            ..GraphFeedbackContext::default()
        });
        let spec = make_spec(&task);

        let error = dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect_err("the first attempt stops at the focused cap");
        assert!(
            matches!(
                error,
                RokoError::TurnLimitReached {
                    limit: 60,
                    num_turns: 61,
                    ..
                }
            ),
            "got {error:?}"
        );

        dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect("the resumed attempt finishes");
        let args = std::fs::read_to_string(temp.path().join("provider-args")).expect("args");
        let caps: Vec<&str> = args
            .split("--max-turns ")
            .skip(1)
            .filter_map(|rest| rest.split_whitespace().next())
            .collect();
        assert_eq!(caps, ["60", "90"], "the retry must not rerun the same cap");
        let resumed_prompt =
            std::fs::read_to_string(temp.path().join("prompt-2")).expect("second prompt");
        assert!(resumed_prompt.contains("Resuming a partially completed task"));
        assert!(
            !std::fs::read_to_string(temp.path().join("prompt-1"))
                .expect("first prompt")
                .contains("Resuming")
        );

        let episodes: Vec<serde_json::Value> = std::fs::read_to_string(&episodes_path)
            .expect("episodes")
            .lines()
            .map(|line| serde_json::from_str(line).expect("episode json"))
            .collect();
        assert_eq!(episodes.len(), 2);
        assert_eq!(episodes[0]["success"], false);
        assert_eq!(episodes[0]["turns"], 61);
        assert!(
            episodes[0]["failure_reason"]
                .as_str()
                .is_some_and(|reason| reason.starts_with("turn_cap: agent turn cap reached")),
            "{}",
            episodes[0]
        );
        assert_eq!(episodes[0]["extra"]["failure_class"], "turn_cap");
        assert_eq!(episodes[1]["success"], true);
        assert_eq!(episodes[1]["turns"], 7);
        assert!(episodes[1]["failure_reason"].is_null());
    }

    #[tokio::test]
    async fn a_timed_out_attempt_is_resumed_with_an_escalated_timeout() {
        for timeout_secs in TIMEOUT_SECS_UNDER_LOAD {
            let temp = tempdir().expect("tempdir");
            let (dispatcher, mut task) =
                make_scripted_batch_dispatcher(&temp, STREAMS_THEN_TIMES_OUT_PROVIDER, |_| {})
                    .await;
            task.timeout_secs = timeout_secs;
            let spec = make_spec(&task);
            let (first_ms, resumed_ms) = (timeout_secs * 1_000, timeout_secs * 1_500);

            for timeout_ms in [first_ms, resumed_ms] {
                let expected = format!("timed out after {timeout_ms} ms");
                let error = dispatcher
                    .dispatch(&spec, Vec::new(), &batch_ctx())
                    .await
                    .expect_err("the attempt runs out of time");
                assert!(
                    matches!(&error, RokoError::Agent { message, .. } if *message == expected),
                    "the retry must not rerun the same timeout: got {error:?}"
                );
            }

            let prompt = |n: u32| std::fs::read_to_string(temp.path().join(format!("prompt-{n}")));
            let (Ok(first_prompt), Ok(resumed_prompt)) = (prompt(1), prompt(2)) else {
                // A provider ran out of time before it recorded its prompt.
                continue;
            };
            assert!(!first_prompt.contains("Resuming"));
            assert!(
                resumed_prompt.contains("# Resuming a timed-out task"),
                "{resumed_prompt}"
            );
            let limit = |ms| format!("{:?}", std::time::Duration::from_millis(ms));
            assert!(
                resumed_prompt.contains(&format!("ran out of its {} time limit", limit(first_ms))),
                "{resumed_prompt}"
            );
            assert!(
                resumed_prompt.contains(&format!("This attempt has {}.", limit(resumed_ms))),
                "{resumed_prompt}"
            );
            return;
        }
        panic!("no provider recorded its prompt before its time ran out");
    }
}
