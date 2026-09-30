//! Turn caps and attempt timeouts per task tier, learned from settled
//! attempts (gap-5a6e01; CASE-004 in the field evidence).
//!
//! A tier's learned limit is the p95 over its last [`WINDOW`] passed attempts
//! (`learning_label = 1`; unverified attempts carry no label), times 1.25,
//! rounded up: turns to a whole turn, time to a whole second. Passed attempts
//! bias the p95 low, because the attempts that needed more failed at the cap.
//! So a learned limit stays within [0.5×, 2×] of the configured one, and a
//! limit that more than 10% of the tier's recent attempts reached is never
//! lowered. [`crate::timeout_tracker`] is a different estimate: an EMA over
//! every dispatch of an operation.
//!
//! [`LearnedTierLimits::load`] reads a workspace's settled attempts from
//! `.roko/runs/*/attempts.jsonl`. `[pipeline] learned_limits` sets whether
//! dispatch uses them (`on`), only logs them (`shadow`, the default) or reads
//! nothing (`off`).

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use roko_core::config::gates::LearnedLimitsMode;
use roko_core::config::schema::RokoConfig;
use roko_core::task::TaskTier;
use serde::Deserialize;

use crate::telemetry::records::{
    ATTEMPT_OPEN_SCHEMA, ATTEMPTS_FILE, AttemptOutcome, AttemptVerdictRecord, Stamped,
    VERDICT_SCHEMA,
};

/// Attempts a tier's limits read: its last `WINDOW` passed attempts, and for
/// the cap rule its last `WINDOW` attempts.
pub const WINDOW: usize = 100;

/// Fewest passed attempts that must report a measure before it is learned.
pub const MIN_PASSED: usize = 20;

/// One settled attempt that carries a learning label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierAttempt {
    /// The task's tier.
    pub tier: TaskTier,
    /// How the attempt ended.
    pub outcome: AttemptOutcome,
    /// `true` when the attempt passed (`learning_label = 1`).
    pub passed: bool,
    /// Agent turns taken, when the provider reported them.
    pub turns: Option<u32>,
    /// Provider-call wall time in ms, from dispatch start to end.
    pub agent_ms: Option<u64>,
    /// Unix ms the attempt settled. The windows keep the latest attempts.
    pub settled_at: Option<i64>,
}

/// A tier's configured limits, which its learned ones stay near.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierLimits {
    /// `[pipeline.<tier>] max_turns`.
    pub max_turns: u32,
    /// The attempt timeout in ms (`timeouts.agent_dispatch_secs`).
    pub timeout_ms: u64,
}

impl TierLimits {
    /// `tier`'s configured limits in `config`.
    #[must_use]
    pub fn configured(config: &RokoConfig, tier: TaskTier) -> Self {
        Self {
            max_turns: config.pipeline.max_turns_for_tier(tier),
            timeout_ms: config.timeouts.agent_dispatch_secs.max(1).saturating_mul(1_000),
        }
    }
}

/// A tier's learned limits. Each is `None` until [`MIN_PASSED`] passed
/// attempts have reported its measure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SuggestedLimits {
    /// Learned turn cap.
    pub max_turns: Option<u32>,
    /// Learned attempt timeout in ms.
    pub timeout_ms: Option<u64>,
}

/// Learn `tier`'s limits from `attempts`, in any order, around `configured`.
#[must_use]
pub fn suggest(
    attempts: &[TierAttempt],
    tier: TaskTier,
    configured: TierLimits,
) -> SuggestedLimits {
    let mut of_tier: Vec<&TierAttempt> =
        attempts.iter().filter(|attempt| attempt.tier == tier).collect();
    of_tier.sort_by_key(|attempt| attempt.settled_at);
    let recent = &of_tier[of_tier.len().saturating_sub(WINDOW)..];
    let passed: Vec<&TierAttempt> = of_tier
        .iter()
        .rev()
        .filter(|attempt| attempt.passed)
        .take(WINDOW)
        .copied()
        .collect();
    // More than 10% of the recent attempts stopped at this limit.
    let reached = |outcome: AttemptOutcome| {
        let hits = recent.iter().filter(|attempt| attempt.outcome == outcome);
        hits.count() * 10 > recent.len()
    };
    let max_turns = learned_limit(
        passed.iter().filter_map(|attempt| attempt.turns).map(u64::from),
        u64::from(configured.max_turns),
        reached(AttemptOutcome::TurnCap),
        1,
    );
    SuggestedLimits {
        max_turns: max_turns.map(|turns| u32::try_from(turns).unwrap_or(u32::MAX)),
        timeout_ms: learned_limit(
            passed.iter().filter_map(|attempt| attempt.agent_ms),
            configured.timeout_ms,
            reached(AttemptOutcome::Timeout),
            1_000,
        ),
    }
}

/// The p95 of `samples` times 1.25, rounded up to a multiple of `unit`, kept
/// within [0.5×, 2×] of `configured`, and never below `configured` when
/// `reached`. `None` with fewer than [`MIN_PASSED`] samples.
fn learned_limit(
    samples: impl Iterator<Item = u64>,
    configured: u64,
    reached: bool,
    unit: u64,
) -> Option<u64> {
    let mut samples: Vec<u64> = samples.collect();
    if samples.len() < MIN_PASSED {
        return None;
    }
    samples.sort_unstable();
    // Nearest-rank p95.
    let p95 = samples[(samples.len() * 95).div_ceil(100) - 1];
    let with_headroom = p95.saturating_mul(5).div_ceil(4);
    let learned = with_headroom.div_ceil(unit).saturating_mul(unit);
    let bounded = learned.clamp(configured.div_ceil(2), configured.saturating_mul(2));
    Some(if reached {
        bounded.max(configured)
    } else {
        bounded
    })
}

/// Each tier's learned limits, and whether dispatch uses them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LearnedTierLimits {
    /// `[pipeline] learned_limits`.
    pub mode: LearnedLimitsMode,
    /// Learned limits by tier.
    pub by_tier: BTreeMap<TaskTier, SuggestedLimits>,
}

impl LearnedTierLimits {
    /// Learn from a workspace's settled attempts under `runs_dir`
    /// (`.roko/runs`). An attempt whose open line lacks its tier takes it from
    /// `costs_path` (`learn/costs.jsonl`), when given. Logs each tier's
    /// configured and learned limits; with `learned_limits = "off"` it reads
    /// nothing.
    #[must_use]
    pub fn load(config: &RokoConfig, runs_dir: &Path, costs_path: Option<&Path>) -> Self {
        let mode = config.pipeline.learned_limits;
        if mode == LearnedLimitsMode::Off {
            return Self {
                mode,
                by_tier: BTreeMap::new(),
            };
        }
        let tiers = costs_path.map(cost_row_tiers).unwrap_or_default();
        let learned = Self::from_attempts(config, &load_tier_attempts(runs_dir, &tiers));
        for (tier, suggested) in &learned.by_tier {
            let configured = TierLimits::configured(config, *tier);
            tracing::info!(
                ?mode,
                tier = %tier,
                configured_max_turns = configured.max_turns,
                learned_max_turns = ?suggested.max_turns,
                configured_timeout_ms = configured.timeout_ms,
                learned_timeout_ms = ?suggested.timeout_ms,
                "tier limits from the p95 of passed attempts"
            );
        }
        learned
    }

    /// Learn from `attempts` under `config`'s limits and mode.
    #[must_use]
    pub fn from_attempts(config: &RokoConfig, attempts: &[TierAttempt]) -> Self {
        let by_tier = TaskTier::ALL
            .into_iter()
            .map(|tier| {
                let configured = TierLimits::configured(config, tier);
                (tier, suggest(attempts, tier, configured))
            })
            .collect();
        Self {
            mode: config.pipeline.learned_limits,
            by_tier,
        }
    }

    /// The learned limits dispatch uses for `tier`: none unless the mode is
    /// `on`.
    #[must_use]
    pub fn applied(&self, tier: TaskTier) -> SuggestedLimits {
        if self.mode == LearnedLimitsMode::On {
            self.by_tier.get(&tier).copied().unwrap_or_default()
        } else {
            SuggestedLimits::default()
        }
    }
}

/// The fields of a run-file line the loader reads before parsing it whole.
#[derive(Debug, Deserialize)]
struct LineProbe {
    #[serde(default)]
    schema_version: Option<String>,
    #[serde(default)]
    attempt_key: Option<String>,
    #[serde(default)]
    tier: Option<String>,
}

/// Settled attempts with a learning label from every run under `runs_dir`
/// (`.roko/runs`). An attempt's tier comes from its open line, else from
/// `tiers` (attempt key to tier, see [`cost_row_tiers`]). Attempts of unknown
/// tier are skipped, as are unreadable files and lines.
#[must_use]
pub fn load_tier_attempts(runs_dir: &Path, tiers: &HashMap<String, TaskTier>) -> Vec<TierAttempt> {
    let Ok(runs) = std::fs::read_dir(runs_dir) else {
        return Vec::new();
    };
    let mut attempts = Vec::new();
    for run in runs.flatten() {
        let Ok(file) = File::open(run.path().join(ATTEMPTS_FILE)) else {
            continue;
        };
        let mut open_tiers = HashMap::new();
        let mut verdicts = Vec::new();
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(probe) = serde_json::from_str::<LineProbe>(&line) else {
                continue;
            };
            match probe.schema_version.as_deref() {
                Some(ATTEMPT_OPEN_SCHEMA) => {
                    let tier = probe.tier.as_deref().and_then(TaskTier::parse);
                    if let (Some(key), Some(tier)) = (probe.attempt_key, tier) {
                        open_tiers.insert(key, tier);
                    }
                }
                Some(VERDICT_SCHEMA) => {
                    let verdict = serde_json::from_str::<Stamped<AttemptVerdictRecord>>(&line);
                    verdicts.extend(verdict.ok().map(|stamped| stamped.record));
                }
                _ => {}
            }
        }
        for verdict in verdicts {
            let key = &verdict.identity.attempt_key;
            let tier = open_tiers.get(key).or_else(|| tiers.get(key)).copied();
            let (Some(tier), Some(label)) = (tier, verdict.learning_label) else {
                continue;
            };
            let timing = &verdict.timing;
            let agent_ms = timing
                .dispatch_started_at
                .zip(timing.dispatch_ended_at)
                .and_then(|(start, end)| end.checked_sub(start))
                .and_then(|ms| u64::try_from(ms).ok());
            attempts.push(TierAttempt {
                tier,
                outcome: verdict.outcome,
                passed: label == 1,
                turns: verdict.executed.turns,
                agent_ms,
                settled_at: timing.settled_at,
            });
        }
    }
    attempts
}

/// Attempt key to tier, from `learn/costs.jsonl` rows, which carry both the
/// attempt key and the task's tier (`complexity_band`). This covers runs
/// whose open lines predate their `tier`. A missing file gives no tiers.
#[must_use]
pub fn cost_row_tiers(costs_path: &Path) -> HashMap<String, TaskTier> {
    #[derive(Deserialize)]
    struct CostRow {
        #[serde(default)]
        attempt_key: Option<String>,
        #[serde(default)]
        complexity_band: Option<String>,
    }

    let Ok(file) = File::open(costs_path) else {
        return HashMap::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<CostRow>(&line).ok())
        .filter_map(|row| {
            let tier = TaskTier::parse(row.complexity_band.as_deref()?)?;
            Some((row.attempt_key?, tier))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::records::AttemptOutcome::{
        GateFailed, Passed, Timeout, TurnCap, Unverified,
    };
    use crate::telemetry::records::{AttemptIdentity, AttemptKey, AttemptOpenRecord};

    const CONFIGURED: TierLimits = TierLimits {
        max_turns: 60,
        timeout_ms: 600_000,
    };

    fn attempt(tier: TaskTier, outcome: AttemptOutcome, turns: u32, agent_ms: u64) -> TierAttempt {
        TierAttempt {
            tier,
            outcome,
            passed: outcome == Passed,
            turns: Some(turns),
            agent_ms: Some(agent_ms),
            settled_at: None,
        }
    }

    /// gap-5a6e01: a tier's limits are the p95 of its last passed attempts
    /// times 1.25, within [0.5×, 2×] of the configured ones, and a limit that
    /// more than 10% of recent attempts reached is never lowered.
    #[test]
    fn tier_limits_follow_p95_of_passed_attempts() {
        let focused = TaskTier::Focused;
        // 100 passes of 1..=100 turns and 4..=400 s: the p95 is 95 turns and
        // 380 s, so 119 turns (118.75 rounded up) and 475 s.
        let passes: Vec<TierAttempt> = (1..=100)
            .map(|n| attempt(focused, Passed, n, u64::from(n) * 4_000))
            .collect();
        let learned = SuggestedLimits {
            max_turns: Some(119),
            timeout_ms: Some(475_000),
        };
        assert_eq!(suggest(&passes, focused, CONFIGURED), learned);

        // Older passes, failures and other tiers leave it alone.
        let mut history = vec![attempt(focused, Passed, 900, 3_000_000); 50];
        history.push(attempt(TaskTier::Mechanical, Passed, 1, 1_000));
        history.extend(passes.iter().copied());
        history.push(attempt(focused, GateFailed, 400, 2_000_000));
        assert_eq!(suggest(&history, focused, CONFIGURED), learned);

        // Fewer than 20 passes teach nothing; 20 are enough.
        assert_eq!(
            suggest(&passes[..19], focused, CONFIGURED),
            SuggestedLimits::default()
        );
        assert!(suggest(&passes[..20], focused, CONFIGURED).max_turns.is_some());

        // A learned limit stays within [0.5×, 2×] of the configured one.
        let quick = vec![attempt(focused, Passed, 2, 1_000); 30];
        let floor = SuggestedLimits {
            max_turns: Some(30),
            timeout_ms: Some(300_000),
        };
        assert_eq!(suggest(&quick, focused, CONFIGURED), floor);
        let slow = vec![attempt(focused, Passed, 500, 5_000_000); 30];
        let ceiling = SuggestedLimits {
            max_turns: Some(120),
            timeout_ms: Some(1_200_000),
        };
        assert_eq!(suggest(&slow, focused, CONFIGURED), ceiling);

        // 4 turn-cap stops in 34 attempts (12%) keep the cap at 60; 3 in 33
        // (9%) let it fall to 30. Timeouts work the same way.
        let mut capped = quick.clone();
        capped.extend([attempt(focused, TurnCap, 60, 1_000); 4]);
        assert_eq!(suggest(&capped, focused, CONFIGURED).max_turns, Some(60));
        capped.truncate(33);
        assert_eq!(suggest(&capped, focused, CONFIGURED).max_turns, Some(30));
        let mut timed_out = quick;
        timed_out.extend([attempt(focused, Timeout, 10, 600_000); 4]);
        assert_eq!(
            suggest(&timed_out, focused, CONFIGURED).timeout_ms,
            Some(600_000)
        );
    }

    fn line<T: serde::Serialize>(schema: &str, record: T) -> String {
        serde_json::to_string(&Stamped {
            schema_version: schema.to_string(),
            record_id: String::new(),
            seq: 0,
            ts: String::new(),
            record,
        })
        .expect("serialize line")
    }

    fn key(task: &str) -> AttemptKey {
        AttemptKey::new("run-1", "plan", task, 1)
    }

    fn open(task: &str, tier: Option<&str>) -> AttemptOpenRecord {
        let mut record = AttemptOpenRecord::new(AttemptIdentity::new(&key(task)), 1_000);
        record.tier = tier.map(str::to_string);
        record
    }

    fn verdict(task: &str, outcome: AttemptOutcome, turns: u32) -> AttemptVerdictRecord {
        let mut record =
            AttemptVerdictRecord::settle(AttemptIdentity::new(&key(task)), outcome, true);
        record.executed.turns = Some(turns);
        record.timing.dispatch_started_at = Some(1_000);
        record.timing.dispatch_ended_at = Some(4_000);
        record.timing.settled_at = Some(5_000);
        record
    }

    /// The loader reads each verdict with its tier from the open line, or from
    /// the cost rows for older lines, and skips attempts with no learning label
    /// or no known tier.
    #[test]
    fn tier_attempts_load_from_run_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run_dir = dir.path().join("run-1");
        std::fs::create_dir_all(&run_dir).expect("run dir");
        let lines = [
            line(ATTEMPT_OPEN_SCHEMA, open("T1", Some("mechanical"))),
            line(VERDICT_SCHEMA, verdict("T1", Passed, 7)),
            // T2's open line predates `tier`; its cost row names it.
            line(ATTEMPT_OPEN_SCHEMA, open("T2", None)),
            line(VERDICT_SCHEMA, verdict("T2", GateFailed, 12)),
            // T3's tier is unknown, and T4 was never verified.
            line(ATTEMPT_OPEN_SCHEMA, open("T3", None)),
            line(VERDICT_SCHEMA, verdict("T3", Passed, 5)),
            line(ATTEMPT_OPEN_SCHEMA, open("T4", Some("focused"))),
            line(VERDICT_SCHEMA, verdict("T4", Unverified, 3)),
            "not a record".to_string(),
        ];
        std::fs::write(run_dir.join(ATTEMPTS_FILE), lines.join("\n")).expect("attempts");
        let t2 = key("T2").attempt_key();
        let costs = dir.path().join("costs.jsonl");
        let cost_rows = format!(
            "{{\"attempt_key\":\"{t2}\",\"complexity_band\":\"Integrative\"}}\n\
             {{\"complexity_band\":\"focused\"}}\n"
        );
        std::fs::write(&costs, cost_rows).expect("costs");

        let attempts = load_tier_attempts(dir.path(), &cost_row_tiers(&costs));

        let passed = TierAttempt {
            tier: TaskTier::Mechanical,
            outcome: Passed,
            passed: true,
            turns: Some(7),
            agent_ms: Some(3_000),
            settled_at: Some(5_000),
        };
        let failed = TierAttempt {
            tier: TaskTier::Integrative,
            outcome: GateFailed,
            passed: false,
            turns: Some(12),
            ..passed
        };
        assert_eq!(attempts, [passed, failed]);

        let mut config = RokoConfig::default();
        config.pipeline.learned_limits = LearnedLimitsMode::Off;
        let off = LearnedTierLimits::load(&config, dir.path(), Some(&costs));
        assert!(off.by_tier.is_empty());
        config.pipeline.learned_limits = LearnedLimitsMode::On;
        let learned = LearnedTierLimits::load(&config, dir.path(), Some(&costs));
        assert_eq!(learned.by_tier.len(), 4);
        assert_eq!(
            learned.applied(TaskTier::Mechanical),
            SuggestedLimits::default(),
            "one pass is too few"
        );
    }
}
