//! How each rung of the model ladder did, from the runs' own records
//! (gap-3217a0): what a run's summary, `GET /api/learn/rungs`, `roko learn
//! rungs` and the end of `roko plan run` show.
//!
//! One row per role, rung and model, over the runs' settled tries
//! (`roko.verdict/1`, with the role from each try's open line). A row counts
//! every try: its verified passes (learning label 1), its agent failures (0),
//! and the rest, a provider, harness, budget or cancelled outcome, which no
//! learner reads.
//!
//! - A try's wall time runs from its open to its settlement.
//! - A verified pass's time is the time its task spent on the rung to reach it:
//!   the pass and the tries of the same chain on the rung before it.
//! - A try's cost is what its plan's budget counted, as its `attempt_settled`
//!   event says it: a CLI agent's own figure, else the provider's charge, else
//!   the price snapshot's.
//! - A call a rung's model made that its provider refused before failover
//!   moved the try on counts on that rung as `refused_usd`, from its
//!   `failover_refused` cost row in `learn/costs.jsonl` (gap-5cfa9c), so the
//!   rungs and the other side calls add up to a run's cost.
//!
//! [`misordered`] compares the rungs of one role. A higher rung that is both
//! cheaper and faster per verified pass than a lower one, each with at least
//! [`MIN_PASSES`] verified passes, comes out as one line with the numbers.
//!
//! [`task_times`] reads the same records per task: how long each task that
//! reached a verified pass took, under the rung it started on and its tier,
//! which a plan run's time-left estimate learns from (gap-c1045f).

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use roko_core::dashboard_snapshot::FAILOVER_REFUSED_SIDE_CALL;
use serde::{Deserialize, Serialize};

use super::records::{AttemptOutcome, AttemptVerdictRecord, BOUND_TIME};
use super::report::RunRecords;

/// The verified passes each rung needs before [`misordered`] compares it.
///
/// - Below three, a median is one try: a single quick or cheap pass would
///   decide the comparison.
/// - At three, two passes must agree before the median moves.
/// - It stays reachable: a task climbs a rung only after two failed tries, so
///   a higher rung gets few tries in one run, and a plan of five tasks can
///   give it three passes.
pub const MIN_PASSES: u32 = 3;

/// A row's role when a try's open line names none.
const UNKNOWN_ROLE: &str = "unknown";

/// How one rung did, for one role and one model.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RungRow {
    /// The role whose ladder the rung is on.
    pub role: String,
    /// The rung's name; `None` for tries the ladder did not route, such as a
    /// pinned model's.
    pub rung: Option<String>,
    /// The rung's place on the role's ladder, cheapest first.
    pub index: Option<u32>,
    /// The model that ran.
    pub model: String,
    /// Settled tries.
    pub tries: u32,
    /// Tries whose checks passed (learning label 1).
    pub verified_passes: u32,
    /// Tries the agent failed (learning label 0).
    pub agent_failures: u32,
    /// Tries that ended otherwise: a provider, harness, budget or cancelled
    /// outcome.
    pub other: u32,
    /// Tries that ended at the rung's time bound.
    pub time_bound_stops: u32,
    /// Wall time per try, from its open to its settlement, in seconds.
    pub try_s: Spread,
    /// Time per verified pass, in seconds: the pass and the tries of its
    /// chain on the rung before it.
    pub pass_s: Spread,
    /// The share of the tries' time spent on the model, over the tries whose
    /// phases say (gap-6a319c); `None` when none do.
    pub model_share: Option<f64>,
    /// What the tries cost together, in USD; a try whose cost is unknown adds
    /// nothing.
    pub cost_usd: f64,
    /// The cost per try; `None` when no try's cost is known.
    pub cost_per_try_usd: Option<f64>,
    /// Every try's cost over the verified passes; `None` without a pass.
    pub cost_per_pass_usd: Option<f64>,
    /// Tries whose cost is unknown.
    pub unpriced: u32,
    /// What the calls the model made and its provider refused before
    /// failover moved their tries on cost, in USD (gap-5cfa9c). They are no
    /// try's, so `cost_usd` leaves them out.
    pub refused_usd: f64,
}

/// The middle and the 90th percentile of some durations, in seconds; `None`
/// without any.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Spread {
    /// The median.
    pub median: Option<f64>,
    /// The 90th percentile (nearest rank).
    pub p90: Option<f64>,
}

impl Spread {
    fn of(mut values: Vec<f64>) -> Self {
        if values.is_empty() {
            return Self::default();
        }
        values.sort_by(f64::total_cmp);
        let n = values.len();
        let median = if n % 2 == 1 {
            values[n / 2]
        } else {
            f64::midpoint(values[n / 2 - 1], values[n / 2])
        };
        // Nearest rank: the least value with at least 90% of them at or
        // below it.
        let p90 = values[((0.9 * n as f64).ceil() as usize).clamp(1, n) - 1];
        Self {
            median: Some(median),
            p90: Some(p90),
        }
    }
}

/// A higher rung that is cheaper and faster per verified pass than a lower
/// one of the same role.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Misorder {
    /// The role.
    pub role: String,
    /// The lower rung.
    pub lower: RungRef,
    /// The higher rung, ahead on both counts.
    pub higher: RungRef,
    /// The finding in one line, with the numbers.
    pub line: String,
}

/// A rung, as [`Misorder`] names it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RungRef {
    /// The rung's name.
    pub rung: String,
    /// Its model.
    pub model: String,
    /// Its verified passes.
    pub verified_passes: u32,
    /// Every try's cost over its verified passes, in USD.
    pub cost_per_pass_usd: f64,
    /// Its median time per verified pass, in seconds.
    pub pass_s: f64,
}

/// What a readout shows: the runs it read, each rung's row, and the line
/// that says a higher rung beat a lower one.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RungReport {
    /// The runs read.
    pub runs: Vec<String>,
    /// Each rung's row.
    pub rows: Vec<RungRow>,
    /// A higher rung ahead of a lower one on both counts; `None` when none is.
    pub misordered: Option<Misorder>,
}

impl RungReport {
    /// The readout of `runs`.
    #[must_use]
    pub fn of(runs: &[RunRecords]) -> Self {
        Self::of_with_refused(runs, &[])
    }

    /// The readout of `runs`, with the calls their providers refused before
    /// failover in the cost rows at `costs_path` (`learn/costs.jsonl`)
    /// counted on the rungs of the models that made them (gap-5cfa9c).
    #[must_use]
    pub fn of_with_costs(runs: &[RunRecords], costs_path: &Path) -> Self {
        Self::of_with_refused(runs, &refused_calls(costs_path, runs))
    }

    /// The readout of `runs`, with `refused` counted on the rungs of the
    /// models that made them.
    #[must_use]
    pub fn of_with_refused(runs: &[RunRecords], refused: &[RefusedCall]) -> Self {
        let mut rows = rows(runs);
        add_refused(&mut rows, runs, refused);
        let misordered = misordered(&rows);
        Self {
            runs: runs.iter().map(|run| run.run_id.clone()).collect(),
            rows,
            misordered,
        }
    }

    /// The readout in lines a person reads: one per rung, then the
    /// misordered line when there is one.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .rows
            .iter()
            .map(|row| format!("Rung {}", row.line()))
            .collect();
        if let Some(misorder) = &self.misordered {
            lines.push(format!("Ladder: {}", misorder.line));
        }
        lines
    }
}

impl RungRow {
    /// The row in one line a person reads.
    #[must_use]
    pub fn line(&self) -> String {
        let rung = self.rung.as_deref().unwrap_or("(no rung)");
        let mut line = format!(
            "{rung} ({model}, {role}): {tries} {tries_word}, {passes} verified {passes_word}",
            model = self.model,
            role = self.role,
            tries = self.tries,
            tries_word = if self.tries == 1 { "try" } else { "tries" },
            passes = self.verified_passes,
            passes_word = if self.verified_passes == 1 {
                "pass"
            } else {
                "passes"
            },
        );
        if let Some(median) = self.try_s.median {
            line.push_str(&format!(", median {} a try", duration(median)));
        }
        if let Some(median) = self.pass_s.median {
            line.push_str(&format!(", {} per verified pass", duration(median)));
        }
        if let Some(share) = self.model_share {
            line.push_str(&format!(", {:.0}% on the model", share * 100.0));
        }
        if let Some(per_try) = self.cost_per_try_usd {
            line.push_str(&format!(", {} a try", usd(per_try)));
        }
        if let Some(per_pass) = self.cost_per_pass_usd {
            line.push_str(&format!(", {} per verified pass", usd(per_pass)));
        }
        if self.time_bound_stops > 0 {
            line.push_str(&format!(
                ", {} stopped at the time bound",
                self.time_bound_stops
            ));
        }
        if self.refused_usd > 0.0 {
            line.push_str(&format!(
                ", {} in calls refused before failover",
                usd(self.refused_usd)
            ));
        }
        line
    }
}

/// A call a rung's model made that its provider refused before failover
/// moved the try on (gap-5cfa9c), from its cost row, role `failover_refused`.
#[derive(Debug, Clone, PartialEq)]
pub struct RefusedCall {
    /// The try's attempt key.
    pub attempt_key: String,
    /// The model that made the call, as its tries' rows name it.
    pub model: String,
    /// What the call cost, in USD.
    pub cost_usd: f64,
}

/// The refused calls of `runs` among the cost rows at `costs_path`
/// (`learn/costs.jsonl`); a missing file gives none.
#[must_use]
pub fn refused_calls(costs_path: &Path, runs: &[RunRecords]) -> Vec<RefusedCall> {
    #[derive(Deserialize)]
    struct CostRow {
        #[serde(default)]
        attempt_key: Option<String>,
        #[serde(default)]
        role: Option<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        cost_usd: Option<f64>,
    }

    let Ok(file) = std::fs::File::open(costs_path) else {
        return Vec::new();
    };
    let prefixes: Vec<String> = runs.iter().map(|run| format!("{}:", run.run_id)).collect();
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|line| line.contains(FAILOVER_REFUSED_SIDE_CALL))
        .filter_map(|line| serde_json::from_str::<CostRow>(&line).ok())
        .filter(|row| row.role.as_deref() == Some(FAILOVER_REFUSED_SIDE_CALL))
        .filter_map(|row| {
            let attempt_key = row.attempt_key?;
            if !prefixes
                .iter()
                .any(|prefix| attempt_key.starts_with(prefix.as_str()))
            {
                return None;
            }
            Some(RefusedCall {
                attempt_key,
                model: row.model?,
                cost_usd: row.cost_usd.filter(|usd| usd.is_finite() && *usd >= 0.0)?,
            })
        })
        .collect()
}

/// Count each of `refused` on the row of its try's role and its model: the
/// row with a rung and the most tries, else a new row with no rung.
fn add_refused(rows: &mut Vec<RungRow>, runs: &[RunRecords], refused: &[RefusedCall]) {
    if refused.is_empty() {
        return;
    }
    let roles: HashMap<&str, &str> = runs
        .iter()
        .flat_map(|run| &run.opens)
        .filter_map(|line| {
            let open = &line.record;
            Some((open.identity.attempt_key.as_str(), open.role.as_deref()?))
        })
        .collect();
    for call in refused {
        let role = roles
            .get(call.attempt_key.as_str())
            .copied()
            .unwrap_or(UNKNOWN_ROLE);
        let found = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.role == role && row.model == call.model)
            .max_by_key(|(_, row)| (row.rung.is_some(), row.tries))
            .map(|(at, _)| at);
        let at = found.unwrap_or_else(|| {
            let key = (role.to_string(), None, call.model.clone());
            rows.push(row_of((key, Tally::default())));
            rows.len() - 1
        });
        rows[at].refused_usd += call.cost_usd;
    }
    sort_rows(rows);
}

/// The runs under `runs_dir` that hold attempt records, newest first, at
/// most `last` of them; one that cannot be read is left out.
#[must_use]
pub fn load_runs(runs_dir: &Path, last: Option<usize>) -> Vec<RunRecords> {
    let Ok(entries) = std::fs::read_dir(runs_dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let dir = entry.path();
            let modified = std::fs::metadata(dir.join("attempts.jsonl"))
                .and_then(|meta| meta.modified())
                .ok()?;
            Some((modified, dir))
        })
        .collect();
    dirs.sort_by_key(|dir| std::cmp::Reverse(dir.0));
    dirs.into_iter()
        .take(last.unwrap_or(usize::MAX))
        .filter_map(|(_, dir)| RunRecords::load(&dir).ok())
        .collect()
}

/// How long one task took to reach a verified pass, from its first try's
/// start to the settlement of the try that passed (gap-c1045f).
#[derive(Debug, Clone, PartialEq)]
pub struct TaskTime {
    /// The run it ran in.
    pub run_id: String,
    /// The rung its first try ran on, when the ladder routed it.
    pub rung: Option<String>,
    /// Its tier, from its first try's open line.
    pub tier: Option<String>,
    /// The seconds it took.
    pub secs: f64,
}

/// The time each task of `runs` took to reach a verified pass
/// ([`TaskTime`]); a task that never passed gives none.
#[must_use]
pub fn task_times<'a>(runs: impl IntoIterator<Item = &'a RunRecords>) -> Vec<TaskTime> {
    let mut times = Vec::new();
    for run in runs {
        let tiers: HashMap<&str, &str> = run
            .opens
            .iter()
            .filter_map(|line| {
                let open = &line.record;
                Some((open.identity.attempt_key.as_str(), open.tier.as_deref()?))
            })
            .collect();
        let mut chains: BTreeMap<&str, Vec<&AttemptVerdictRecord>> = BTreeMap::new();
        for line in &run.verdicts {
            let verdict = &line.record;
            chains
                .entry(verdict.identity.chain_key.as_str())
                .or_default()
                .push(verdict);
        }
        for tries in chains.values_mut() {
            tries.sort_by_key(|verdict| verdict.identity.attempt);
            let Some(pass) = tries
                .iter()
                .find(|verdict| verdict.learning_label == Some(1))
            else {
                continue;
            };
            let first = tries[0];
            let started = first.timing.attempt_started_at;
            let Some(millis) = started
                .zip(pass.timing.settled_at)
                .and_then(|(start, end)| end.checked_sub(start))
                .filter(|millis| *millis > 0)
            else {
                continue;
            };
            times.push(TaskTime {
                run_id: run.run_id.clone(),
                rung: first.ladder.as_ref().and_then(|ladder| ladder.rung.clone()),
                tier: tiers
                    .get(first.identity.attempt_key.as_str())
                    .map(|tier| (*tier).to_string()),
                secs: millis as f64 / 1_000.0,
            });
        }
    }
    times
}

/// What a row adds up as its tries are read.
#[derive(Default)]
struct Tally {
    tries: u32,
    verified_passes: u32,
    agent_failures: u32,
    other: u32,
    time_bound_stops: u32,
    try_s: Vec<f64>,
    pass_s: Vec<f64>,
    model_ms: u64,
    phased_ms: u64,
    cost_usd: f64,
    priced: u32,
    index: Option<u32>,
}

/// One row's key: role, rung and model.
type RowKey = (String, Option<String>, String);

/// The rows of `runs`' settled tries, by role, then ladder order, then model.
#[must_use]
pub fn rows<'a>(runs: impl IntoIterator<Item = &'a RunRecords>) -> Vec<RungRow> {
    let mut tallies: BTreeMap<RowKey, Tally> = BTreeMap::new();
    for run in runs {
        tally_run(run, &mut tallies);
    }
    let mut rows: Vec<RungRow> = tallies.into_iter().map(row_of).collect();
    sort_rows(&mut rows);
    rows
}

/// Sort `rows` by role, then ladder order, then model.
fn sort_rows(rows: &mut [RungRow]) {
    rows.sort_by(|a, b| {
        (&a.role, a.index.is_none(), a.index, &a.rung, &a.model).cmp(&(
            &b.role,
            b.index.is_none(),
            b.index,
            &b.rung,
            &b.model,
        ))
    });
}

/// Add `run`'s settled tries to `tallies`.
fn tally_run<'a>(run: &'a RunRecords, tallies: &mut BTreeMap<RowKey, Tally>) {
    let roles: HashMap<&str, &str> = run
        .opens
        .iter()
        .filter_map(|line| {
            let open = &line.record;
            Some((open.identity.attempt_key.as_str(), open.role.as_deref()?))
        })
        .collect();
    // A chain's tries in order, so that a pass takes the time of the tries on
    // its rung before it.
    let mut verdicts: Vec<&AttemptVerdictRecord> =
        run.verdicts.iter().map(|line| &line.record).collect();
    verdicts.sort_by(|a, b| {
        (&a.identity.chain_key, a.identity.attempt)
            .cmp(&(&b.identity.chain_key, b.identity.attempt))
    });
    // The time a chain has spent on a row since its last pass there.
    let mut toward_pass: HashMap<(&'a str, RowKey), f64> = HashMap::new();
    for verdict in verdicts {
        // A try refused because every rung failed its probe ran on no rung
        // and no model (gap-b856d3, gap-d91691).
        if verdict.outcome == AttemptOutcome::AllRungsSkipped {
            continue;
        }
        let role = roles
            .get(verdict.identity.attempt_key.as_str())
            .copied()
            .unwrap_or(UNKNOWN_ROLE);
        let ladder = verdict.ladder.as_ref();
        let key = (
            role.to_string(),
            ladder.and_then(|ladder| ladder.rung.clone()),
            model_of(verdict),
        );
        let tally = tallies.entry(key.clone()).or_default();
        tally.index = tally.index.or(ladder.and_then(|ladder| ladder.index));
        tally.tries += 1;
        match verdict.learning_label {
            Some(1) => tally.verified_passes += 1,
            Some(_) => tally.agent_failures += 1,
            None => tally.other += 1,
        }
        if verdict.bound.as_deref() == Some(BOUND_TIME) {
            tally.time_bound_stops += 1;
        }
        let wall_s = wall_s(verdict);
        tally.try_s.extend(wall_s);
        let spent = toward_pass
            .entry((verdict.identity.chain_key.as_str(), key))
            .or_default();
        *spent += wall_s.unwrap_or(0.0);
        if verdict.learning_label == Some(1) {
            tally.pass_s.push(std::mem::take(spent));
        }
        if let Some((model_ms, duration_ms)) = model_time(verdict) {
            tally.model_ms += model_ms;
            tally.phased_ms += duration_ms;
        }
        if let Some(usd) = cost_usd(verdict) {
            tally.cost_usd += usd;
            tally.priced += 1;
        }
    }
}

/// The row of `key`'s `tally`.
fn row_of((key, tally): (RowKey, Tally)) -> RungRow {
    let (role, rung, model) = key;
    let per = |count: u32| (count > 0).then(|| tally.cost_usd / f64::from(count));
    RungRow {
        role,
        rung,
        index: tally.index,
        model,
        tries: tally.tries,
        verified_passes: tally.verified_passes,
        agent_failures: tally.agent_failures,
        other: tally.other,
        time_bound_stops: tally.time_bound_stops,
        try_s: Spread::of(tally.try_s),
        pass_s: Spread::of(tally.pass_s),
        model_share: (tally.phased_ms > 0).then(|| tally.model_ms as f64 / tally.phased_ms as f64),
        cost_usd: tally.cost_usd,
        cost_per_try_usd: per(tally.priced),
        cost_per_pass_usd: per(tally.verified_passes),
        unpriced: tally.tries - tally.priced,
        refused_usd: 0.0,
    }
}

/// The first rung, lowest first, that a higher rung of the same role beats
/// on both cost and median time per verified pass, each with at least
/// [`MIN_PASSES`] verified passes; `None` when no rung does.
#[must_use]
pub fn misordered(rows: &[RungRow]) -> Option<Misorder> {
    let comparable = |row: &&RungRow| {
        row.verified_passes >= MIN_PASSES && row.rung.is_some() && row.index.is_some()
    };
    let rungs: Vec<&RungRow> = rows.iter().filter(comparable).collect();
    for lower_row in &rungs {
        for higher_row in &rungs {
            if higher_row.role != lower_row.role || higher_row.index <= lower_row.index {
                continue;
            }
            let (Some(lower), Some(higher)) = (rung_ref(lower_row), rung_ref(higher_row)) else {
                continue;
            };
            if higher.cost_per_pass_usd < lower.cost_per_pass_usd && higher.pass_s < lower.pass_s {
                let role = lower_row.role.clone();
                let line = format!(
                    "{higher_rung} ({higher_model}) is cheaper and faster per verified pass than \
                     {lower_rung} ({lower_model}), the rung below it on the {role} ladder: \
                     {higher_cost} vs {lower_cost} and {higher_time} vs {lower_time} (median), over \
                     {higher_n} and {lower_n} verified passes",
                    higher_rung = higher.rung,
                    higher_model = higher.model,
                    lower_rung = lower.rung,
                    lower_model = lower.model,
                    higher_cost = usd(higher.cost_per_pass_usd),
                    lower_cost = usd(lower.cost_per_pass_usd),
                    higher_time = duration(higher.pass_s),
                    lower_time = duration(lower.pass_s),
                    higher_n = higher.verified_passes,
                    lower_n = lower.verified_passes,
                );
                return Some(Misorder {
                    role,
                    lower,
                    higher,
                    line,
                });
            }
        }
    }
    None
}

/// `row` as [`Misorder`] names it, when it has a cost and a time per pass.
fn rung_ref(row: &RungRow) -> Option<RungRef> {
    Some(RungRef {
        rung: row.rung.clone()?,
        model: row.model.clone(),
        verified_passes: row.verified_passes,
        cost_per_pass_usd: row.cost_per_pass_usd?,
        pass_s: row.pass_s.median?,
    })
}

/// The model a try ran on: the one dispatched, else the one asked for.
fn model_of(verdict: &AttemptVerdictRecord) -> String {
    let executed = &verdict.executed;
    executed
        .model_dispatched
        .clone()
        .or_else(|| executed.model_requested.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

/// A try's wall time in seconds: open to settle, else its provider call.
fn wall_s(verdict: &AttemptVerdictRecord) -> Option<f64> {
    let timing = &verdict.timing;
    let (from, to) = timing
        .attempt_started_at
        .zip(timing.settled_at)
        .or_else(|| timing.dispatch_started_at.zip(timing.dispatch_ended_at))?;
    let millis = to.checked_sub(from).filter(|millis| *millis >= 0)?;
    Some(millis as f64 / 1_000.0)
}

/// A try's time on the model and its whole duration, in ms, from its phases
/// (gap-6a319c); `None` for a try recorded without them.
fn model_time(verdict: &AttemptVerdictRecord) -> Option<(u64, u64)> {
    let phases = verdict.timing.phases.as_ref()?;
    let model_ms = phases.model_ms?;
    let duration_ms = [
        phases.prompt_ms,
        phases.model_ms,
        phases.tools_ms,
        phases.checks_ms,
        phases.other_ms,
    ]
    .into_iter()
    .flatten()
    .sum();
    Some((model_ms, duration_ms))
}

/// A try's cost in USD, as its plan's budget counted it: a CLI agent's own
/// figure, else the provider's charge, else the price snapshot's; `None`
/// when none is known. Its refused calls (`cost.refused_usd`) are left out:
/// they count on the row of the model the provider refused, from the run's
/// cost rows ([`refused_calls`]).
fn cost_usd(verdict: &AttemptVerdictRecord) -> Option<f64> {
    let cost = &verdict.cost;
    cost.vendor_usd
        .or(cost.billed_usd)
        .or(cost.api_equiv_usd)
        .filter(|usd| usd.is_finite() && *usd >= 0.0)
}

/// `usd` as a person reads it: `$0.12`, or `$0.0040` below a cent.
fn usd(usd: f64) -> String {
    if usd < 0.01 {
        format!("${usd:.4}")
    } else {
        format!("${usd:.2}")
    }
}

/// `secs` as a person reads it: `44 s`, `16.9 min` or `1.2 h`.
fn duration(secs: f64) -> String {
    if secs < 120.0 {
        format!("{secs:.0} s")
    } else if secs < 7_200.0 {
        format!("{:.1} min", secs / 60.0)
    } else {
        format!("{:.1} h", secs / 3_600.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::records::{
        AttemptKey, AttemptLadder, AttemptOpenRecord, AttemptOutcome, LadderReason, Stamped,
    };
    use crate::telemetry::{AttemptIdentity, CostSource};
    use roko_core::dashboard_snapshot::AttemptPhases;

    const RUN: &str = "run-rungs";

    fn stamped<T>(record: T) -> Stamped<T> {
        Stamped {
            schema_version: String::new(),
            record_id: String::new(),
            seq: 0,
            ts: String::new(),
            record,
        }
    }

    /// One try of `task` on `rung` (`index` on the implementer ladder) and
    /// `model`, that took `secs` and cost `usd`, with how it ended.
    struct Try<'a> {
        task: &'a str,
        attempt: u32,
        rung: &'a str,
        index: u32,
        model: &'a str,
        outcome: AttemptOutcome,
        secs: i64,
        usd: f64,
    }

    fn run_of(tries: &[Try<'_>]) -> RunRecords {
        let mut run = RunRecords {
            run_id: RUN.to_string(),
            ..RunRecords::default()
        };
        for t in tries {
            let identity = AttemptIdentity::new(&AttemptKey::new(RUN, "plan", t.task, t.attempt));
            let mut open = AttemptOpenRecord::new(identity.clone(), 0);
            open.role = Some("implementer".to_string());
            run.opens.push(stamped(open));
            let mut verdict = AttemptVerdictRecord::settle(identity, t.outcome, true);
            verdict.ladder = Some(AttemptLadder {
                rung: Some(t.rung.to_string()),
                index: Some(t.index),
                step: 0,
                reason: LadderReason::Start,
                exhausted: false,
                router_pick: None,
            });
            verdict.executed.model_dispatched = Some(t.model.to_string());
            verdict.timing.attempt_started_at = Some(1_000);
            verdict.timing.settled_at = Some(1_000 + t.secs * 1_000);
            verdict.cost.api_equiv_usd = Some(t.usd);
            verdict.cost.source = CostSource::ProviderUsage;
            run.verdicts.push(stamped(verdict));
        }
        run
    }

    /// gap-d91691: a try refused because every rung failed its probe ran on no
    /// rung and no model, so it is no rung's try; the tries that ran keep their
    /// rows.
    #[test]
    fn a_try_refused_for_no_rung_is_no_rung_try() {
        let tried = |task, outcome| Try {
            task,
            attempt: 1,
            rung: "mid",
            index: 1,
            model: "gpt-5.4-mini",
            outcome,
            secs: 60,
            usd: 0.02,
        };
        let run = run_of(&[
            tried("T1", AttemptOutcome::Passed),
            tried("T2", AttemptOutcome::AllRungsSkipped),
        ]);
        let rows = rows([&run]);
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!((rows[0].tries, rows[0].verified_passes), (1, 1), "{rows:?}");
    }

    /// gap-c1045f: a task's time runs from its first try's start to the
    /// settlement of its verified pass, under the rung its first try ran on and
    /// its tier; a task that never passed gives none.
    #[test]
    fn task_times_run_from_the_first_try_to_the_pass() {
        use AttemptOutcome::{GateFailed, Passed};
        let mut run = RunRecords {
            run_id: RUN.to_string(),
            ..RunRecords::default()
        };
        let tries = [
            ("T1", 1, "mid", "integrative", GateFailed, 0, 600),
            ("T1", 2, "strong", "integrative", Passed, 600, 660),
            ("T2", 1, "mid", "architectural", Passed, 0, 1_800),
            ("T3", 1, "cheap", "focused", GateFailed, 0, 300),
        ];
        for (task, attempt, rung, tier, outcome, start, end) in tries {
            let identity = AttemptIdentity::new(&AttemptKey::new(RUN, "plan", task, attempt));
            let mut open = AttemptOpenRecord::new(identity.clone(), 0);
            open.tier = Some(tier.to_string());
            run.opens.push(stamped(open));
            let mut verdict = AttemptVerdictRecord::settle(identity, outcome, true);
            verdict.ladder = Some(AttemptLadder {
                rung: Some(rung.to_string()),
                index: Some(0),
                step: 0,
                reason: LadderReason::Start,
                exhausted: false,
                router_pick: None,
            });
            verdict.timing.attempt_started_at = Some(start * 1_000);
            verdict.timing.settled_at = Some(end * 1_000);
            run.verdicts.push(stamped(verdict));
        }
        let times = task_times(std::iter::once(&run));
        let got: Vec<(Option<&str>, Option<&str>, f64)> = times
            .iter()
            .map(|time| (time.rung.as_deref(), time.tier.as_deref(), time.secs))
            .collect();
        assert_eq!(
            got,
            [
                (Some("mid"), Some("integrative"), 660.0),
                (Some("mid"), Some("architectural"), 1_800.0),
            ]
        );
        assert!(times.iter().all(|time| time.run_id == RUN));
    }

    /// gap-5cfa9c: run 7b8cb121's rungs came to $0.7009 against a run cost
    /// of $0.9085. The $0.2076 its mid model, glm-4.7, spent in calls its
    /// provider refused before T2's failover was on no rung. A refused call's
    /// cost row now counts on the row of its try's role and its model, as
    /// `refused_usd`, so the rungs add up to the run; a try's own cost stays
    /// its own. A model with no try gets a row of its own. Another run's rows
    /// and other roles' rows are not counted.
    #[test]
    fn a_rungs_spend_counts_its_models_refused_calls() {
        use AttemptOutcome::{GateFailed, Passed};
        let run = run_of(&[
            Try {
                task: "T1",
                attempt: 1,
                rung: "mid",
                index: 1,
                model: "glm-4.7",
                outcome: GateFailed,
                secs: 600,
                usd: 0.4155,
            },
            Try {
                task: "T1",
                attempt: 2,
                rung: "mid",
                index: 1,
                model: "glm-4.7",
                outcome: Passed,
                secs: 600,
                usd: 0.25,
            },
            Try {
                task: "T2",
                attempt: 1,
                rung: "strong",
                index: 2,
                model: "gpt-5.4-mini",
                outcome: Passed,
                secs: 300,
                usd: 0.0354,
            },
        ]);
        let temp = tempfile::tempdir().expect("tempdir");
        let costs = temp.path().join("costs.jsonl");
        let t2 = AttemptKey::new(RUN, "plan", "T2", 1).attempt_key();
        let row = |key: &str, role: &str, model: &str, usd: f64| {
            serde_json::json!({
                "attempt_key": key, "role": role, "model": model, "cost_usd": usd,
                "provider": "zai", "plan_id": "plan", "task_id": "T2",
            })
            .to_string()
        };
        let lines = [
            row(&t2, "failover_refused", "glm-4.7", 0.2076),
            row(&t2, "failover_refused", "kimi-k2", 0.01),
            row(&t2, "helper", "glm-4.7", 0.5),
            row("other-run:plan:T2:1", "failover_refused", "glm-4.7", 9.0),
        ];
        std::fs::write(&costs, lines.join("\n") + "\n").expect("costs.jsonl");

        let report = RungReport::of_with_costs(std::slice::from_ref(&run), &costs);
        let rows: Vec<(Option<&str>, &str, u32, f64, f64)> = report
            .rows
            .iter()
            .map(|row| {
                (
                    row.rung.as_deref(),
                    row.model.as_str(),
                    row.tries,
                    (row.cost_usd * 10_000.0).round() / 10_000.0,
                    (row.refused_usd * 10_000.0).round() / 10_000.0,
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                (Some("mid"), "glm-4.7", 2, 0.6655, 0.2076),
                (Some("strong"), "gpt-5.4-mini", 1, 0.0354, 0.0),
                (None, "kimi-k2", 0, 0.0, 0.01),
            ]
        );
        let total: f64 = report
            .rows
            .iter()
            .map(|row| row.cost_usd + row.refused_usd)
            .sum();
        assert!((total - 0.9185).abs() < 1e-9, "{total}");
        assert!(
            report.rows[0]
                .line()
                .ends_with(", $0.21 in calls refused before failover"),
            "{}",
            report.rows[0].line()
        );
        // Without the cost rows, as before.
        let plain = RungReport::of(std::slice::from_ref(&run));
        assert_eq!(plain.rows.len(), 2);
        assert!(plain.rows.iter().all(|row| row.refused_usd == 0.0));
        assert!(refused_calls(&temp.path().join("missing.jsonl"), &[run]).is_empty());
    }

    /// gap-3217a0: each rung's row counts its tries by how they ended, takes
    /// each pass's time with the failed tries of its chain on the rung before
    /// it, and prices every try over the verified passes. A higher rung that
    /// is cheaper and faster per verified pass than a lower one, each with
    /// three passes, comes out as one line with the numbers; with two passes
    /// on the higher rung, nothing does.
    #[test]
    fn rung_rows_measure_each_rung_and_flag_a_misordered_ladder() {
        use AttemptOutcome::{GateFailed, Passed, ProviderError};
        let mid = |task, attempt, outcome, secs, usd| Try {
            task,
            attempt,
            rung: "mid",
            index: 1,
            model: "glm-4.7",
            outcome,
            secs,
            usd,
        };
        let strong = |task, attempt, outcome, secs, usd| Try {
            task,
            attempt,
            rung: "strong",
            index: 2,
            model: "gpt-5.4-mini",
            outcome,
            secs,
            usd,
        };
        let mut tries = vec![
            mid("T1", 1, GateFailed, 600, 0.10),
            mid("T1", 2, Passed, 900, 0.15),
            mid("T2", 1, Passed, 1_200, 0.20),
            mid("T3", 1, Passed, 300, 0.05),
            mid("T4", 1, ProviderError, 2, 0.0),
            strong("T5", 1, Passed, 40, 0.04),
            strong("T6", 1, Passed, 60, 0.05),
        ];
        let run = run_of(&tries);
        let two_passes = rows(std::iter::once(&run));
        let [mid_row, strong_row] = two_passes.as_slice() else {
            panic!("two rows: {two_passes:#?}");
        };
        assert_eq!(
            (
                mid_row.rung.as_deref(),
                mid_row.model.as_str(),
                mid_row.index
            ),
            (Some("mid"), "glm-4.7", Some(1))
        );
        assert_eq!(
            (
                mid_row.tries,
                mid_row.verified_passes,
                mid_row.agent_failures,
                mid_row.other
            ),
            (5, 3, 1, 1)
        );
        // T1's pass took its failed try's 600 s with its own 900 s.
        assert_eq!(mid_row.pass_s.median, Some(1_200.0));
        assert_eq!(mid_row.pass_s.p90, Some(1_500.0));
        assert_eq!(mid_row.try_s.median, Some(600.0));
        assert!((mid_row.cost_usd - 0.50).abs() < 1e-9, "{mid_row:?}");
        assert!((mid_row.cost_per_pass_usd.expect("passes") - 0.50 / 3.0).abs() < 1e-9);
        assert!((mid_row.cost_per_try_usd.expect("priced") - 0.10).abs() < 1e-9);
        assert_eq!(mid_row.model_share, None, "no try has phases");
        assert_eq!(strong_row.verified_passes, 2);
        assert_eq!(
            misordered(&two_passes),
            None,
            "two passes on strong are too few to compare"
        );

        tries.push(strong("T7", 1, Passed, 50, 0.03));
        let mut run = run_of(&tries);
        if let Some(line) = run.verdicts.last_mut() {
            line.record.timing.phases = Some(AttemptPhases {
                prompt_ms: Some(1_000),
                model_ms: Some(45_000),
                tools_ms: Some(3_000),
                checks_ms: Some(1_000),
                other_ms: Some(0),
                ..AttemptPhases::default()
            });
        }
        let three_passes = rows(std::iter::once(&run));
        assert_eq!(three_passes[1].model_share, Some(0.9));
        let misorder = misordered(&three_passes).expect("strong beats mid on both counts");
        assert_eq!(
            (misorder.lower.rung.as_str(), misorder.higher.rung.as_str()),
            ("mid", "strong")
        );
        assert_eq!(misorder.role, "implementer");
        assert_eq!(
            misorder.line,
            "strong (gpt-5.4-mini) is cheaper and faster per verified pass than mid (glm-4.7), \
             the rung below it on the implementer ladder: $0.04 vs $0.17 and 50 s vs 20.0 min \
             (median), over 3 and 3 verified passes"
        );
    }
}
