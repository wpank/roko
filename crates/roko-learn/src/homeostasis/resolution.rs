//! The task-resolution fold (S06 §4.2; S01 addendum A-RES). M1's unit is a
//! task resolution: one task after all its attempts.
//!
//! - [`ResolutionFold`] folds settled verdicts (`roko.verdict/1`, one per
//!   attempt) into resolutions. A repeated `settlement_id` is skipped and
//!   counted. A chain resolves at its first `passed` verdict, or at its last
//!   settled attempt when the run closes ([`ResolutionFold::close`]).
//! - [`fold_historical`] folds the logs written before S01's telemetry,
//!   `learn/efficiency.jsonl` and `learn/costs.jsonl`, and marks every
//!   resolution `pre_instrumentation`.
//!
//! Only `passed` is a verified success. `unverified`, `already_satisfied`
//! and `forced_accept` resolve as not verified: they count in E1's
//! denominator and add no success to E2.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::DateTime;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::telemetry::{Arm, AttemptOutcome, AttemptVerdictRecord, Blame, CostSource};

/// A resolution's attempts by `cost.source` (S01 §4.4). Historical
/// resolutions count their `costs.jsonl` rows instead.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CostSourceMix {
    /// Usage an API provider reported.
    pub provider_usage: u32,
    /// Usage a CLI agent reported, priced API-equivalent (D1).
    pub cli_usage: u32,
    /// Usage estimated locally.
    pub estimated: u32,
    /// A test provider.
    pub mock: u32,
    /// Unknown provenance.
    pub unknown: u32,
}

impl CostSourceMix {
    /// Count one attempt priced from `source`.
    pub fn add(&mut self, source: CostSource) {
        let slot = match source {
            CostSource::ProviderUsage => &mut self.provider_usage,
            CostSource::CliUsage => &mut self.cli_usage,
            CostSource::Estimated => &mut self.estimated,
            CostSource::Mock => &mut self.mock,
            CostSource::Unknown => &mut self.unknown,
        };
        *slot += 1;
    }

    /// Everything counted.
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.provider_usage + self.cli_usage + self.estimated + self.mock + self.unknown
    }

    /// Whether every count is reported usage, from a provider or a CLI agent,
    /// so cost per verified success (E2) may use the resolution.
    #[must_use]
    pub const fn all_reported(&self) -> bool {
        self.estimated == 0 && self.mock == 0 && self.unknown == 0
    }
}

/// One task after all its attempts (S01 addendum A-RES).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskResolution {
    /// The chain's key, `run:plan:task` (S01 §4.2). A historical chain is
    /// `historical:plan:task@<unix ms of its first row>`.
    pub chain_key: String,
    /// The outcome of the attempt the chain resolved at.
    pub final_verdict: AttemptOutcome,
    /// Settled attempts in the chain.
    pub attempts: u32,
    /// Σ `cost.api_equiv_usd` over the chain's attempts, failed ones
    /// included; `None` when any attempt's is unknown.
    pub api_equiv_usd: Option<f64>,
    /// Where the chain's priced usage came from.
    pub cost_source_mix: CostSourceMix,
    /// From the first attempt's start to the last settlement; `None` when
    /// unknown.
    pub wall_ms: Option<u64>,
    /// Attempts charged to infrastructure: provider errors, exhausted
    /// failover, timeouts before the first token. An auxiliary signal that
    /// only ranks moves.
    pub provider_errors: u32,
    /// Conductor restarts in the chain, an auxiliary signal that only ranks
    /// moves. A verdict does not tell a conductor cancel apart yet, so the
    /// fold leaves it 0 and the plan-run sink fills it (8122).
    pub conductor_restarts: u32,
    /// The θ digest the chain ran under; `None` until decision rows carry it
    /// (8123).
    pub params_digest: Option<String>,
    /// The chain's `harness_policy` arm; `None` when no holdout was drawn,
    /// which counts as adaptive.
    pub arm: Option<Arm>,
    /// Whether M4 audited the chain's pass.
    pub audited: bool,
    /// What the audit found: `Some(true)` when the pass failed hidden or
    /// stronger checks.
    pub audit_false_green: Option<bool>,
    /// Folded from logs written before S01's telemetry.
    pub pre_instrumentation: bool,
    /// Unix ms of the chain's last settlement; `None` when unknown.
    pub resolved_at: Option<i64>,
}

impl TaskResolution {
    /// Whether the chain ended in a verified success. Only `passed` is one.
    #[must_use]
    pub const fn verified_success(&self) -> bool {
        matches!(self.final_verdict, AttemptOutcome::Passed)
    }

    /// Whether the chain ran on the holdout or all-off arm, θ₀ throughout.
    /// Such rows are kept for M2 and evaluation but never steer M1.
    #[must_use]
    pub const fn holdout(&self) -> bool {
        matches!(self.arm, Some(Arm::Default | Arm::GlobalOff))
    }
}

// ── Settled verdicts ──────────────────────────────────────────────────

/// Folds settled verdicts into task resolutions.
#[derive(Debug, Default)]
pub struct ResolutionFold {
    settlements: HashSet<String>,
    open: IndexMap<String, ChainState>,
    resolved: HashSet<String>,
    duplicates: u64,
    late: u64,
}

impl ResolutionFold {
    /// An empty fold.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one settled verdict. Returns the chain's resolution when the
    /// verdict is the chain's first pass. A settlement seen before is
    /// skipped and counted in [`Self::duplicates`]; a verdict for a chain
    /// that already resolved is counted in [`Self::late`].
    pub fn push(&mut self, verdict: &AttemptVerdictRecord) -> Option<TaskResolution> {
        if !self.settlements.insert(verdict.settlement_id.clone()) {
            self.duplicates += 1;
            return None;
        }
        let chain_key = &verdict.identity.chain_key;
        if self.resolved.contains(chain_key) {
            self.late += 1;
            return None;
        }
        self.open.entry(chain_key.clone()).or_default().add(verdict);
        if verdict.outcome != AttemptOutcome::Passed {
            return None;
        }
        let state = self.open.shift_remove(chain_key)?;
        self.resolved.insert(chain_key.clone());
        Some(state.resolve(chain_key.clone()))
    }

    /// Resolve the open chain `chain_key` at its last settled attempt, as
    /// when the run has no retry left for it. `None` when the chain is not
    /// open.
    pub fn resolve(&mut self, chain_key: &str) -> Option<TaskResolution> {
        let state = self.open.shift_remove(chain_key)?;
        self.resolved.insert(chain_key.to_string());
        Some(state.resolve(chain_key.to_string()))
    }

    /// The run closed: resolve every open chain at its last settled
    /// attempt, in the order the chains first settled.
    pub fn close(&mut self) -> Vec<TaskResolution> {
        let open = std::mem::take(&mut self.open);
        open.into_iter()
            .map(|(chain_key, state)| {
                self.resolved.insert(chain_key.clone());
                state.resolve(chain_key)
            })
            .collect()
    }

    /// Settlements skipped because they were seen before.
    #[must_use]
    pub const fn duplicates(&self) -> u64 {
        self.duplicates
    }

    /// Verdicts for chains that had already resolved.
    #[must_use]
    pub const fn late(&self) -> u64 {
        self.late
    }

    /// Chains with settled attempts that have not resolved yet.
    #[must_use]
    pub fn open_chains(&self) -> usize {
        self.open.len()
    }
}

/// Fold one run's verdicts, in file order, then close the run.
pub fn fold_verdicts<'a, I>(verdicts: I) -> Vec<TaskResolution>
where
    I: IntoIterator<Item = &'a AttemptVerdictRecord>,
{
    let mut fold = ResolutionFold::new();
    let mut resolutions: Vec<TaskResolution> = verdicts
        .into_iter()
        .filter_map(|verdict| fold.push(verdict))
        .collect();
    resolutions.extend(fold.close());
    resolutions
}

/// The attempts of one unresolved chain.
#[derive(Debug, Default)]
struct ChainState {
    attempts: u32,
    cost_usd: f64,
    cost_unknown: bool,
    mix: CostSourceMix,
    first_started: Option<i64>,
    last_settled: Option<i64>,
    provider_errors: u32,
    last_outcome: Option<AttemptOutcome>,
}

impl ChainState {
    fn add(&mut self, verdict: &AttemptVerdictRecord) {
        self.attempts += 1;
        match verdict.cost.api_equiv_usd {
            Some(usd) => self.cost_usd += usd,
            None => self.cost_unknown = true,
        }
        self.mix.add(verdict.cost.source);
        self.first_started = self
            .first_started
            .into_iter()
            .chain(verdict.timing.attempt_started_at)
            .min();
        self.last_settled = self
            .last_settled
            .into_iter()
            .chain(verdict.timing.settled_at)
            .max();
        if verdict.blame == Blame::Infra {
            self.provider_errors += 1;
        }
        self.last_outcome = Some(verdict.outcome);
    }

    fn resolve(self, chain_key: String) -> TaskResolution {
        TaskResolution {
            chain_key,
            final_verdict: self.last_outcome.unwrap_or(AttemptOutcome::Abandoned),
            attempts: self.attempts,
            api_equiv_usd: (!self.cost_unknown).then_some(self.cost_usd),
            cost_source_mix: self.mix,
            wall_ms: self
                .last_settled
                .zip(self.first_started)
                .and_then(|(end, start)| end.checked_sub(start))
                .and_then(|ms| u64::try_from(ms).ok()),
            provider_errors: self.provider_errors,
            conductor_restarts: 0,
            params_digest: None,
            arm: None,
            audited: false,
            audit_false_green: None,
            pre_instrumentation: false,
            resolved_at: self.last_settled,
        }
    }
}

// ── Logs written before S01 ───────────────────────────────────────────

/// Rows of one plan task further apart than this belong to different
/// chains: a later run of the same task.
pub const HISTORICAL_CHAIN_GAP_MS: i64 = 60 * 60 * 1000;

/// How far outside a chain's rows a `costs.jsonl` row may fall and still
/// join the chain.
const COST_JOIN_SLACK_MS: i64 = 5 * 60 * 1000;

/// What [`fold_historical`] read and left out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct HistoricalFoldReport {
    /// Non-blank lines of `efficiency.jsonl`.
    pub efficiency_rows: usize,
    /// Rows that summarize no attempt: unparseable, without a plan, task or
    /// timestamp, or not a final turn.
    pub skipped_rows: usize,
    /// Exact repeats of a row already read.
    pub duplicate_rows: usize,
    /// Non-blank lines of `costs.jsonl`.
    pub cost_rows: usize,
    /// Cost rows that fall in no chain's time span.
    pub unmatched_cost_rows: usize,
}

/// One `efficiency.jsonl` line, read leniently: rows of several writers and
/// schema versions share the file.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct EfficiencyRow {
    plan_id: Option<String>,
    task_id: Option<String>,
    attempt_id: Option<String>,
    turn_number: Option<u32>,
    is_final_turn: Option<bool>,
    gate_passed: Option<bool>,
    outcome: Option<String>,
    cost_usd: Option<f64>,
    /// The attempt's cost priced at API rates, which a subscription-billed
    /// row's `cost_usd` (about $0) is not; read first (gap-e73a26).
    api_equiv_usd: Option<f64>,
    wall_time_ms: Option<u64>,
    timestamp: Option<String>,
}

/// One `costs.jsonl` line, read leniently. A row written before
/// `cost_source` existed reads `unknown`.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CostRow {
    plan_id: Option<String>,
    task_id: Option<String>,
    timestamp: Option<String>,
    cost_usd: Option<f64>,
    /// The call's cost priced at API rates; read before `cost_usd`
    /// (gap-e73a26).
    api_equiv_usd: Option<f64>,
    cost_source: Option<CostSource>,
}

/// An attempt summary, or a gate row that carries an attempt's verdict.
#[derive(Debug)]
struct HistoricalRow {
    plan_id: String,
    task_id: String,
    at_ms: i64,
    gate: bool,
    gate_passed: Option<bool>,
    cost_usd: f64,
    wall_ms: i64,
}

impl HistoricalRow {
    fn from_efficiency(row: &EfficiencyRow) -> Option<Self> {
        let plan_id = row.plan_id.as_deref().filter(|id| !id.is_empty())?;
        let task_id = row.task_id.as_deref().filter(|id| !id.is_empty())?;
        if !row.is_final_turn.unwrap_or(true) {
            return None;
        }
        let at_ms = parse_ms(row.timestamp.as_deref()?)?;
        let attempt_id = row.attempt_id.as_deref().unwrap_or_default();
        let outcome = row.outcome.as_deref().unwrap_or_default();
        let gate = matches!(outcome, "gate_pass" | "gate_failed")
            || attempt_id.ends_with("gate-pass")
            || attempt_id.ends_with("gate-fail");
        Some(Self {
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            at_ms,
            gate,
            gate_passed: row.gate_passed,
            cost_usd: row.api_equiv_usd.or(row.cost_usd).unwrap_or(0.0),
            wall_ms: row
                .wall_time_ms
                .and_then(|ms| i64::try_from(ms).ok())
                .unwrap_or(0),
        })
    }
}

/// The rows of one plan task close together in time: one historical chain.
#[derive(Debug)]
struct HistoricalChain {
    plan_id: String,
    task_id: String,
    start_ms: i64,
    first_ms: i64,
    last_ms: i64,
    attempts: u32,
    attempt_cost_usd: f64,
    passed: Option<bool>,
    costs: Vec<(f64, CostSource)>,
}

impl HistoricalChain {
    fn new(row: HistoricalRow) -> Self {
        let mut chain = Self {
            plan_id: row.plan_id.clone(),
            task_id: row.task_id.clone(),
            start_ms: row.at_ms.saturating_sub(row.wall_ms),
            first_ms: row.at_ms,
            last_ms: row.at_ms,
            attempts: 0,
            attempt_cost_usd: 0.0,
            passed: None,
            costs: Vec::new(),
        };
        chain.add(&row);
        chain
    }

    fn takes(&self, row: &HistoricalRow) -> bool {
        self.plan_id == row.plan_id
            && self.task_id == row.task_id
            && row.at_ms.saturating_sub(self.last_ms) <= HISTORICAL_CHAIN_GAP_MS
    }

    fn add(&mut self, row: &HistoricalRow) {
        self.last_ms = self.last_ms.max(row.at_ms);
        if !row.gate {
            self.attempts += 1;
            self.attempt_cost_usd += row.cost_usd;
        }
        // A gate pass anywhere in the chain is its verified success.
        self.passed = if self.passed == Some(true) || row.gate_passed == Some(true) {
            Some(true)
        } else {
            self.passed.or(row.gate_passed)
        };
    }

    fn covers(&self, at_ms: i64) -> bool {
        let from = self.start_ms.saturating_sub(COST_JOIN_SLACK_MS);
        let to = self.last_ms.saturating_add(COST_JOIN_SLACK_MS);
        (from..=to).contains(&at_ms)
    }

    fn resolve(self) -> TaskResolution {
        let final_verdict = match self.passed {
            Some(true) => AttemptOutcome::Passed,
            Some(false) => AttemptOutcome::GateFailed,
            None => AttemptOutcome::Unverified,
        };
        let attempts = self.attempts.max(1);
        let mut cost_source_mix = CostSourceMix::default();
        let api_equiv_usd = if self.costs.is_empty() {
            cost_source_mix.unknown = attempts;
            self.attempt_cost_usd
        } else {
            for &(_, source) in &self.costs {
                cost_source_mix.add(source);
            }
            self.costs.iter().map(|&(usd, _)| usd).sum()
        };
        TaskResolution {
            chain_key: format!(
                "historical:{}:{}@{}",
                self.plan_id, self.task_id, self.first_ms
            ),
            final_verdict,
            attempts,
            api_equiv_usd: Some(api_equiv_usd),
            cost_source_mix,
            wall_ms: u64::try_from(self.last_ms.saturating_sub(self.start_ms)).ok(),
            provider_errors: 0,
            conductor_restarts: 0,
            params_digest: None,
            arm: None,
            audited: false,
            audit_false_green: None,
            pre_instrumentation: true,
            resolved_at: Some(self.last_ms),
        }
    }
}

/// Fold the pre-S01 logs into resolutions, oldest first, every one
/// `pre_instrumentation`.
///
/// An `efficiency.jsonl` row is one attempt when it is a final turn with a
/// plan, a task and a timestamp; exact repeats are dropped. Gate rows
/// (`gate_pass`, `gate_failed`) carry an attempt's verdict and are not
/// attempts. A plan task's rows less than [`HISTORICAL_CHAIN_GAP_MS`] apart
/// form one chain, which passed when any of its rows reports a gate pass,
/// failed when one reports a gate failure, and is unverified otherwise.
/// Each `costs.jsonl` row joins the chain of its plan task whose time span
/// holds it; a chain with none is priced from its efficiency rows, as
/// `unknown`.
pub fn fold_historical<E, C>(
    efficiency_lines: E,
    cost_lines: C,
) -> (Vec<TaskResolution>, HistoricalFoldReport)
where
    E: IntoIterator,
    E::Item: AsRef<str>,
    C: IntoIterator,
    C::Item: AsRef<str>,
{
    let mut report = HistoricalFoldReport::default();
    let rows = historical_rows(efficiency_lines, &mut report);
    let mut chains = group_chains(rows);
    join_costs(&mut chains, cost_lines, &mut report);
    let mut resolutions: Vec<TaskResolution> =
        chains.into_iter().map(HistoricalChain::resolve).collect();
    resolutions.sort_by_key(|resolution| resolution.resolved_at);
    (resolutions, report)
}

/// [`fold_historical`] over `<learn_dir>/efficiency.jsonl` and
/// `<learn_dir>/costs.jsonl`. A missing file holds no rows.
///
/// # Errors
///
/// Returns an error when a file exists but cannot be read.
pub fn fold_historical_dir(
    learn_dir: &Path,
) -> std::io::Result<(Vec<TaskResolution>, HistoricalFoldReport)> {
    let efficiency = read_optional(&learn_dir.join("efficiency.jsonl"))?;
    let costs = read_optional(&learn_dir.join("costs.jsonl"))?;
    Ok(fold_historical(efficiency.lines(), costs.lines()))
}

fn read_optional(path: &Path) -> std::io::Result<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error),
    }
}

fn parse_ms(timestamp: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|at| at.timestamp_millis())
}

fn historical_rows<E>(lines: E, report: &mut HistoricalFoldReport) -> Vec<HistoricalRow>
where
    E: IntoIterator,
    E::Item: AsRef<str>,
{
    let mut seen = HashSet::new();
    let mut rows = Vec::new();
    for line in lines {
        let line = line.as_ref().trim();
        if line.is_empty() {
            continue;
        }
        report.efficiency_rows += 1;
        let parsed = serde_json::from_str::<EfficiencyRow>(line)
            .ok()
            .and_then(|row| HistoricalRow::from_efficiency(&row).map(|parsed| (row, parsed)));
        let Some((row, parsed)) = parsed else {
            report.skipped_rows += 1;
            continue;
        };
        let key = (
            row.plan_id,
            row.task_id,
            row.attempt_id,
            row.turn_number,
            row.timestamp,
        );
        if !seen.insert(key) {
            report.duplicate_rows += 1;
            continue;
        }
        rows.push(parsed);
    }
    rows
}

fn group_chains(mut rows: Vec<HistoricalRow>) -> Vec<HistoricalChain> {
    rows.sort_by(|a, b| (&a.plan_id, &a.task_id, a.at_ms).cmp(&(&b.plan_id, &b.task_id, b.at_ms)));
    let mut chains: Vec<HistoricalChain> = Vec::new();
    for row in rows {
        match chains.last_mut() {
            Some(chain) if chain.takes(&row) => chain.add(&row),
            _ => chains.push(HistoricalChain::new(row)),
        }
    }
    chains
}

fn join_costs<C>(chains: &mut [HistoricalChain], lines: C, report: &mut HistoricalFoldReport)
where
    C: IntoIterator,
    C::Item: AsRef<str>,
{
    let mut by_task: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (index, chain) in chains.iter().enumerate() {
        by_task
            .entry((chain.plan_id.clone(), chain.task_id.clone()))
            .or_default()
            .push(index);
    }
    for line in lines {
        let line = line.as_ref().trim();
        if line.is_empty() {
            continue;
        }
        report.cost_rows += 1;
        let target = serde_json::from_str::<CostRow>(line).ok().and_then(|row| {
            let at_ms = parse_ms(row.timestamp.as_deref()?)?;
            let key = (row.plan_id?, row.task_id?);
            let index = by_task
                .get(&key)?
                .iter()
                .copied()
                .find(|&index| chains[index].covers(at_ms))?;
            Some((
                index,
                row.api_equiv_usd.or(row.cost_usd).unwrap_or(0.0),
                row.cost_source.unwrap_or_default(),
            ))
        });
        match target {
            Some((index, usd, source)) => chains[index].costs.push((usd, source)),
            None => report.unmatched_cost_rows += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{AttemptIdentity, AttemptKey, GateVerdictTag};

    /// A settled verdict of `run-1:plan-a:<task>`, attempt `attempt`, which
    /// started at `attempt × 10 s` and settled 4 s later.
    fn verdict(
        task: &str,
        attempt: u32,
        outcome: AttemptOutcome,
        usd: Option<f64>,
    ) -> AttemptVerdictRecord {
        let key = AttemptKey::new("run-1", "plan-a", task, attempt);
        let mut record = AttemptVerdictRecord::settle(AttemptIdentity::new(&key), outcome, false);
        record.cost.api_equiv_usd = usd;
        record.cost.source = if usd.is_some() {
            CostSource::ProviderUsage
        } else {
            CostSource::Unknown
        };
        let started = i64::from(attempt) * 10_000;
        record.timing.attempt_started_at = Some(started);
        record.timing.settled_at = Some(started + 4_000);
        record
    }

    #[test]
    fn ev_fold_dedupes_and_counts_only_passed() {
        let retried_fail = verdict("T1", 1, AttemptOutcome::GateFailed, Some(0.10));
        let retried_pass = verdict("T1", 2, AttemptOutcome::Passed, Some(0.20));
        let unverified = AttemptVerdictRecord::from_gate_verdict(
            AttemptIdentity::new(&AttemptKey::new("run-1", "plan-a", "T2", 1)),
            GateVerdictTag::Unverified,
        );
        let provider_error = verdict("T3", 1, AttemptOutcome::ProviderError, None);
        let mut preexisting = verdict("T3", 2, AttemptOutcome::Passed, Some(0.05));
        preexisting.gate_verdict = Some(GateVerdictTag::PassedWithPreexistingFailures);
        let satisfied = verdict("T4", 1, AttemptOutcome::AlreadySatisfied, Some(0.01));
        let lines = [
            retried_fail,
            unverified.clone(),
            retried_pass.clone(),
            // The same settlements again.
            retried_pass,
            unverified,
            provider_error,
            preexisting,
            satisfied,
        ];

        let mut fold = ResolutionFold::new();
        let mut resolutions: Vec<TaskResolution> =
            lines.iter().filter_map(|line| fold.push(line)).collect();
        assert_eq!(fold.duplicates(), 2);
        assert_eq!(resolutions.len(), 2, "T1 and T3 resolve at their passes");
        assert_eq!(fold.open_chains(), 2);
        resolutions.extend(fold.close());
        assert_eq!(fold.open_chains(), 0);
        let keys: Vec<&str> = resolutions
            .iter()
            .map(|resolution| resolution.chain_key.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "run-1:plan-a:T1",
                "run-1:plan-a:T3",
                "run-1:plan-a:T2",
                "run-1:plan-a:T4"
            ]
        );

        // The retried task resolves once, with both attempts and both costs.
        let retried = &resolutions[0];
        assert_eq!(retried.final_verdict, AttemptOutcome::Passed);
        assert_eq!(retried.attempts, 2);
        let usd = retried.api_equiv_usd.expect("both attempts are priced");
        assert!((usd - 0.30).abs() < 1e-12, "{usd}");
        assert_eq!(retried.cost_source_mix.provider_usage, 2);
        assert!(retried.cost_source_mix.all_reported());
        assert_eq!(retried.wall_ms, Some(14_000));
        assert_eq!(retried.resolved_at, Some(24_000));
        assert!(!retried.pre_instrumentation && !retried.holdout());

        // A pass over preexisting failures is a pass; an unpriced attempt
        // makes the chain's cost unknown, never 0.
        let after_error = &resolutions[1];
        assert!(after_error.verified_success());
        assert_eq!(after_error.provider_errors, 1);
        assert_eq!(after_error.api_equiv_usd, None);
        assert!(!after_error.cost_source_mix.all_reported());

        // Unverified and already-satisfied chains resolve at close, not
        // verified.
        assert_eq!(resolutions[2].final_verdict, AttemptOutcome::Unverified);
        assert_eq!(
            resolutions[3].final_verdict,
            AttemptOutcome::AlreadySatisfied
        );
        assert!(!resolutions[2].verified_success() && !resolutions[3].verified_success());

        // Only `passed` counts: two verified successes in four resolutions.
        let passed = resolutions
            .iter()
            .filter(|resolution| resolution.verified_success())
            .count();
        assert_eq!((passed, resolutions.len()), (2, 4));

        // A verdict for a resolved chain opens nothing.
        let late = verdict("T1", 3, AttemptOutcome::GateFailed, Some(0.1));
        assert_eq!(fold.push(&late), None);
        assert_eq!(fold.late(), 1);
        assert_eq!(fold.open_chains(), 0);

        // The one-call form folds the same.
        assert_eq!(fold_verdicts(lines.iter()), resolutions);
    }

    #[test]
    fn historical_fold_marks_pre_instrumentation_and_joins_costs() {
        let efficiency = [
            // Another writer's row, a turn that is not final, a blank line.
            r#"{"schema":"model_call/v1","kind":"call","run_id":"r","cost_usd":0.1}"#,
            r#"{"plan_id":"p","task_id":"T1","is_final_turn":false,"timestamp":"2026-09-01T10:00:00Z"}"#,
            "",
            // T1 fails, then passes its gate ten minutes later: one chain.
            r#"{"plan_id":"p","task_id":"T1","attempt_id":"T1:1:3","turn_number":3,"timestamp":"2026-09-01T10:05:00Z","cost_usd":0.2,"wall_time_ms":60000,"outcome":"failure"}"#,
            r#"{"plan_id":"p","task_id":"T1","attempt_id":"T1:1:3","turn_number":3,"timestamp":"2026-09-01T10:05:00Z","cost_usd":0.2,"wall_time_ms":60000,"outcome":"failure"}"#,
            r#"{"plan_id":"p","task_id":"T1","attempt_id":"T1:2:4","turn_number":4,"timestamp":"2026-09-01T10:15:00Z","cost_usd":0.3,"wall_time_ms":120000,"outcome":"success"}"#,
            r#"{"plan_id":"p","task_id":"T1","attempt_id":"gate-pass","turn_number":4,"timestamp":"2026-09-01T10:15:01Z","cost_usd":0.0,"wall_time_ms":0,"outcome":"gate_pass","gate_passed":true}"#,
            // T2 finishes with no gate result: not verified.
            r#"{"plan_id":"p","task_id":"T2","timestamp":"2026-09-01T11:00:00Z","cost_usd":0.5,"wall_time_ms":30000,"outcome":"success","gate_passed":null}"#,
            // T1 again three hours later: a second chain, which fails.
            r#"{"plan_id":"p","task_id":"T1","timestamp":"2026-09-01T13:20:00Z","cost_usd":0.4,"wall_time_ms":1000,"outcome":"failure","gate_passed":false}"#,
        ];
        let costs = [
            r#"{"timestamp":"2026-09-01T10:04:00Z","model":"m","provider":"x","role":"implementer","plan_id":"p","task_id":"T1","complexity_band":"focused","input_tokens":1,"output_tokens":1,"cached_tokens":0,"cost_usd":0.25,"duration_ms":1,"success":true,"session_id":""}"#,
            r#"{"timestamp":"2026-09-01T10:14:00Z","plan_id":"p","task_id":"T1","cost_usd":0.35,"cost_source":"cli_usage"}"#,
            r#"{"timestamp":"2026-09-02T00:00:00Z","plan_id":"p","task_id":"T1","cost_usd":9.0}"#,
        ];
        let (resolutions, report) = fold_historical(efficiency, costs);
        assert_eq!(
            report,
            HistoricalFoldReport {
                efficiency_rows: 8,
                skipped_rows: 2,
                duplicate_rows: 1,
                cost_rows: 3,
                unmatched_cost_rows: 1,
            }
        );
        assert!(
            resolutions
                .iter()
                .all(|resolution| resolution.pre_instrumentation)
        );
        let verdicts: Vec<AttemptOutcome> = resolutions
            .iter()
            .map(|resolution| resolution.final_verdict)
            .collect();
        assert_eq!(
            verdicts,
            [
                AttemptOutcome::Passed,
                AttemptOutcome::Unverified,
                AttemptOutcome::GateFailed
            ]
        );

        // The first chain: two attempts (the gate row is a verdict, not an
        // attempt), priced from its two cost rows, one of them unsourced.
        let first = &resolutions[0];
        assert!(
            first.chain_key.starts_with("historical:p:T1@"),
            "{}",
            first.chain_key
        );
        assert_eq!(first.attempts, 2);
        let usd = first.api_equiv_usd.expect("joined cost rows");
        assert!((usd - 0.60).abs() < 1e-12, "{usd}");
        assert_eq!(first.cost_source_mix.cli_usage, 1);
        assert_eq!(first.cost_source_mix.unknown, 1);
        assert!(!first.cost_source_mix.all_reported());
        // From 10:04:00 (the first attempt's start) to 10:15:01.
        assert_eq!(first.wall_ms, Some(661_000));

        // A chain with no cost row is priced from its efficiency rows, as
        // unknown.
        let unverified = &resolutions[1];
        assert_eq!(unverified.api_equiv_usd, Some(0.5));
        assert_eq!(unverified.cost_source_mix.unknown, 1);
        assert_eq!(resolutions[2].attempts, 1);
    }

    #[test]
    fn historical_fold_reads_a_learn_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (resolutions, report) = fold_historical_dir(dir.path()).expect("missing files");
        assert!(resolutions.is_empty());
        assert_eq!(report, HistoricalFoldReport::default());
        std::fs::write(
            dir.path().join("efficiency.jsonl"),
            "{\"plan_id\":\"p\",\"task_id\":\"T\",\"timestamp\":\"2026-09-01T10:00:00Z\",\
             \"gate_passed\":true}\n",
        )
        .expect("write efficiency.jsonl");
        let (resolutions, _) = fold_historical_dir(dir.path()).expect("one row");
        assert_eq!(resolutions.len(), 1);
        assert!(resolutions[0].verified_success() && resolutions[0].pre_instrumentation);
    }

    /// gap-e73a26: the fold reads a historical row's `api_equiv_usd`, the
    /// API-rate price of a subscription-billed attempt whose `cost_usd` is $0,
    /// from efficiency and cost rows alike; a row with `cost_usd` alone still
    /// folds from that.
    #[test]
    fn historical_fold_prefers_api_equiv_usd_over_cost_usd() {
        let efficiency = [
            // T1: subscription-billed, with no cost row.
            r#"{"plan_id":"p","task_id":"T1","timestamp":"2026-09-01T10:00:00Z","cost_usd":0.0,"api_equiv_usd":0.42,"wall_time_ms":1000,"gate_passed":true}"#,
            // T2: an older row that has `cost_usd` alone.
            r#"{"plan_id":"p","task_id":"T2","timestamp":"2026-09-01T11:00:00Z","cost_usd":0.3,"wall_time_ms":1000,"gate_passed":true}"#,
            // T3: priced by its cost row, which has both.
            r#"{"plan_id":"p","task_id":"T3","timestamp":"2026-09-01T12:00:00Z","cost_usd":0.0,"wall_time_ms":60000,"gate_passed":true}"#,
        ];
        let costs = [
            r#"{"timestamp":"2026-09-01T11:59:30Z","plan_id":"p","task_id":"T3","cost_usd":0.0,"api_equiv_usd":0.17,"cost_source":"cli_usage"}"#,
        ];
        let (resolutions, report) = fold_historical(efficiency, costs);
        assert_eq!(report.unmatched_cost_rows, 0);
        let usd: Vec<Option<f64>> = resolutions
            .iter()
            .map(|resolution| resolution.api_equiv_usd)
            .collect();
        assert_eq!(usd, [Some(0.42), Some(0.3), Some(0.17)]);
    }
}
