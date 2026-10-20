//! A plan run's time left (gap-c1045f): the longest path through its
//! unfinished tasks, from how long past tasks took.
//!
//! A task's duration on the rung it starts on is log-normal, fitted to how
//! long past tasks took to reach a verified pass
//! ([`roko_learn::telemetry::rung_stats::task_times`]), this run's own
//! included as they finish:
//! - three or more past tasks of the same start rung and tier: those
//!   (`history`);
//! - else the rung's past tasks of any tier, and with too few of those, every
//!   rung's, each scaled by the ratio of the tiers' size bounds
//!   ([`TaskTier::max_loc`]) (`scaled`);
//! - else half the attempt timeout (`guess`).
//!
//! A running task counts what it likely still needs, given how long it has
//! run: the median, and the 90th percentile, of D − a given D > a. A task that
//! runs long so raises its own estimate instead of reading as nearly done. A
//! waiting task counts D. The plan's estimate is the longest path through its
//! unfinished tasks along their dependencies, and at least their total over
//! the run's agent slots, at the median and at the 90th percentile.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use roko_core::dashboard_snapshot::EtaBasis;
use roko_core::task::TaskTier;
use roko_learn::self_model::prior::{normal_cdf, normal_quantile};
use roko_learn::telemetry::rung_stats::TaskTime;

/// The runs whose past tasks a plan run's estimate learns from, newest first.
pub const HISTORY_RUNS: usize = 20;

/// How often the estimate is published while a plan's tasks run.
pub const REFRESH: Duration = Duration::from_secs(60);

/// Fewest past tasks a duration is fitted from.
const MIN_SAMPLES: usize = 3;

/// The narrowest spread of a fitted duration, in log seconds: past tasks that
/// all took about as long still leave room either side.
const MIN_SPREAD: f64 = 0.25;

/// The spread of a guess, in log seconds: its 90th percentile is about 2.6
/// times its median.
const GUESS_SPREAD: f64 = 0.75;

/// How far past its fit, in standard deviations of log time, a running task
/// must be before its remaining time comes from the tail's own approximation.
const FAR_TAIL: f64 = 6.0;

/// The label of this run's own finished tasks among the past ones.
const THIS_RUN: &str = "this run";

/// One past task.
#[derive(Debug, Clone)]
struct PastTask {
    rung: Option<String>,
    tier: Option<TaskTier>,
    secs: f64,
    run: String,
}

/// How long past tasks took, by start rung and tier, and the guess for a task
/// none of them can speak for.
#[derive(Debug, Clone, Default)]
pub struct TaskDurations {
    past: Vec<PastTask>,
    guess_secs: f64,
}

impl TaskDurations {
    /// The durations of `times`, with `guess_secs` for a task they don't cover.
    #[must_use]
    pub fn new(times: Vec<TaskTime>, guess_secs: f64) -> Self {
        let past = times
            .into_iter()
            .filter(|time| time.secs > 0.0)
            .map(|time| PastTask {
                rung: time.rung,
                tier: time.tier.as_deref().and_then(TaskTier::parse),
                secs: time.secs,
                run: time.run_id,
            })
            .collect();
        Self {
            past,
            guess_secs: guess_secs.max(1.0),
        }
    }

    /// Learn one of this run's tasks: it started on `rung`, is of `tier`, and
    /// took `secs` to its verified pass.
    pub fn add(&mut self, rung: Option<&str>, tier: TaskTier, secs: f64) {
        if secs > 0.0 {
            self.past.push(PastTask {
                rung: rung.map(str::to_string),
                tier: Some(tier),
                secs,
                run: THIS_RUN.to_string(),
            });
        }
    }

    /// The duration of a task of `tier` that starts on `rung`.
    fn fit(&self, rung: Option<&str>, tier: TaskTier) -> Fit {
        let on_rung = |(_, task): &(usize, &PastTask)| task.rung.as_deref() == rung;
        let every: Vec<(usize, &PastTask)> = self.past.iter().enumerate().collect();
        let same: Vec<(usize, &PastTask)> = every
            .iter()
            .copied()
            .filter(on_rung)
            .filter(|(_, task)| task.tier == Some(tier))
            .collect();
        if same.len() >= MIN_SAMPLES {
            return Fit::of(&same, |task| task.secs, Source::History);
        }
        let scaled = |task: &PastTask| {
            let from = task.tier.unwrap_or(tier);
            task.secs * f64::from(tier.max_loc()) / f64::from(from.max_loc())
        };
        let rung_tasks: Vec<(usize, &PastTask)> = every.iter().copied().filter(on_rung).collect();
        if rung_tasks.len() >= MIN_SAMPLES {
            return Fit::of(&rung_tasks, scaled, Source::Scaled);
        }
        if every.len() >= MIN_SAMPLES {
            return Fit::of(&every, scaled, Source::Scaled);
        }
        Fit {
            mu: self.guess_secs.ln(),
            sigma: GUESS_SPREAD,
            source: Source::Guess,
            past: Vec::new(),
        }
    }
}

/// Where a task's duration came from, weakest last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    History,
    Scaled,
    Guess,
}

impl Source {
    const fn label(self) -> &'static str {
        match self {
            Self::History => "history",
            Self::Scaled => "scaled",
            Self::Guess => "guess",
        }
    }
}

/// A task's log-normal duration, in seconds, and the past tasks behind it, by
/// their place in [`TaskDurations`].
#[derive(Debug, Clone)]
struct Fit {
    mu: f64,
    sigma: f64,
    source: Source,
    past: Vec<usize>,
}

impl Fit {
    /// The fit of `tasks`' durations, each read by `secs`.
    fn of(tasks: &[(usize, &PastTask)], secs: impl Fn(&PastTask) -> f64, source: Source) -> Self {
        let logs: Vec<f64> = tasks.iter().map(|(_, task)| secs(task).ln()).collect();
        let n = logs.len() as f64;
        let mu = logs.iter().sum::<f64>() / n;
        let variance = logs.iter().map(|log| (log - mu).powi(2)).sum::<f64>() / (n - 1.0);
        Self {
            mu,
            sigma: variance.sqrt().max(MIN_SPREAD),
            source,
            past: tasks.iter().map(|&(index, _)| index).collect(),
        }
    }

    /// The `q` quantile of the time a task still needs after running
    /// `elapsed` seconds.
    fn remaining(&self, elapsed: f64, q: f64) -> f64 {
        if elapsed <= 0.0 {
            return self.sigma.mul_add(normal_quantile(q), self.mu).exp();
        }
        let z = (elapsed.ln() - self.mu) / self.sigma;
        if z > FAR_TAIL {
            // Far past the fit, where Φ's tail underflows: cutting the tail to
            // (1 − q) of itself moves z by about ln(1/(1 − q))/z.
            return elapsed * ((self.sigma * (1.0 / (1.0 - q)).ln() / z).exp() - 1.0);
        }
        let tail = (1.0 - q) * (1.0 - normal_cdf(z));
        let end = self
            .sigma
            .mul_add(normal_quantile(1.0 - tail), self.mu)
            .exp();
        (end - elapsed).max(0.0)
    }
}

/// One task of a plan, as the estimate sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtaTask {
    /// The task's id, which is its graph node's.
    pub id: String,
    /// The plan's tasks it waits for.
    pub depends_on: Vec<String>,
    /// The rung the ladder starts it on; `None` for a pinned task.
    pub rung: Option<String>,
    /// Its tier.
    pub tier: TaskTier,
}

/// Where a task stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Waiting,
    Running(Instant),
    Done,
}

/// A plan's tasks and where each stands.
#[derive(Debug, Clone, Default)]
pub struct PlanEta {
    tasks: BTreeMap<String, (EtaTask, State)>,
}

/// A plan's time left, at the median and the 90th percentile.
#[derive(Debug, Clone, PartialEq)]
pub struct Estimate {
    /// The median seconds left.
    pub p50_secs: f64,
    /// The 90th percentile.
    pub p90_secs: f64,
    /// What it rests on.
    pub basis: EtaBasis,
}

impl PlanEta {
    /// A plan of `tasks`, none of them started.
    #[must_use]
    pub fn new(tasks: Vec<EtaTask>) -> Self {
        Self {
            tasks: tasks
                .into_iter()
                .map(|task| (task.id.clone(), (task, State::Waiting)))
                .collect(),
        }
    }

    /// Task `id` started at `at`.
    pub fn started(&mut self, id: &str, at: Instant) {
        if let Some((_, state)) = self.tasks.get_mut(id)
            && *state == State::Waiting
        {
            *state = State::Running(at);
        }
    }

    /// Task `id` finished at `at`: its start rung, its tier and the seconds it
    /// ran, when it was seen to start.
    pub fn finished(&mut self, id: &str, at: Instant) -> Option<(Option<String>, TaskTier, f64)> {
        let (task, state) = self.tasks.get_mut(id)?;
        let ran = match *state {
            State::Running(since) => Some(at.saturating_duration_since(since).as_secs_f64()),
            State::Waiting | State::Done => None,
        };
        *state = State::Done;
        ran.map(|secs| (task.rung.clone(), task.tier, secs))
    }

    /// Whether a task of the plan is running.
    #[must_use]
    pub fn running(&self) -> bool {
        self.tasks
            .values()
            .any(|(_, state)| matches!(state, State::Running(_)))
    }

    /// The plan's time left at `now`, from `durations`, with `slots` tasks able
    /// to run at once; `None` once every task finished.
    #[must_use]
    pub fn estimate(
        &self,
        durations: &TaskDurations,
        slots: usize,
        now: Instant,
    ) -> Option<Estimate> {
        let open: BTreeMap<&str, (&EtaTask, State, Fit)> = self
            .tasks
            .iter()
            .filter(|(_, (_, state))| *state != State::Done)
            .map(|(id, (task, state))| {
                let fit = durations.fit(task.rung.as_deref(), task.tier);
                (id.as_str(), (task, *state, fit))
            })
            .collect();
        if open.is_empty() {
            return None;
        }
        let at = |q: f64| -> f64 {
            let left: BTreeMap<&str, f64> = open
                .iter()
                .map(|(&id, (_, state, fit))| {
                    let elapsed = match state {
                        State::Running(since) => {
                            now.saturating_duration_since(*since).as_secs_f64()
                        }
                        State::Waiting | State::Done => 0.0,
                    };
                    (id, fit.remaining(elapsed, q))
                })
                .collect();
            let mut memo = BTreeMap::new();
            let path = open
                .keys()
                .map(|&id| longest(id, &open, &left, &mut memo, &mut BTreeSet::new()))
                .fold(0.0, f64::max);
            let work = left.values().sum::<f64>() / slots.max(1) as f64;
            path.max(work)
        };
        let source = open
            .values()
            .map(|(_, _, fit)| fit.source)
            .max()
            .unwrap_or(Source::Guess);
        let past: BTreeSet<usize> = open
            .values()
            .flat_map(|(_, _, fit)| fit.past.iter().copied())
            .collect();
        let runs: BTreeSet<&str> = past
            .iter()
            .filter_map(|&index| durations.past.get(index))
            .map(|task| task.run.as_str())
            .collect();
        Some(Estimate {
            p50_secs: at(0.5),
            p90_secs: at(0.9),
            basis: EtaBasis {
                source: source.label().to_string(),
                tasks: u32::try_from(past.len()).unwrap_or(u32::MAX),
                runs: u32::try_from(runs.len()).unwrap_or(u32::MAX),
            },
        })
    }
}

/// The seconds from now until task `id` and the unfinished tasks it waits for
/// are done, each taking `left`; a dependency cycle counts nothing.
fn longest<'a>(
    id: &'a str,
    open: &BTreeMap<&'a str, (&'a EtaTask, State, Fit)>,
    left: &BTreeMap<&'a str, f64>,
    memo: &mut BTreeMap<&'a str, f64>,
    visiting: &mut BTreeSet<&'a str>,
) -> f64 {
    if let Some(&done) = memo.get(id) {
        return done;
    }
    let Some((task, _, _)) = open.get(id) else {
        return 0.0;
    };
    if !visiting.insert(id) {
        return 0.0;
    }
    let before = task
        .depends_on
        .iter()
        .filter_map(|dep| open.get_key_value(dep.as_str()).map(|(&key, _)| key))
        .map(|dep| longest(dep, open, left, memo, visiting))
        .fold(0.0, f64::max);
    visiting.remove(id);
    let total = before + left.get(id).copied().unwrap_or(0.0);
    memo.insert(id, total);
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A past task of `rung` and `tier` that took `minutes`.
    fn past(rung: &str, tier: &str, minutes: f64) -> TaskTime {
        TaskTime {
            run_id: "run-1".to_string(),
            rung: Some(rung.to_string()),
            tier: Some(tier.to_string()),
            secs: minutes * 60.0,
        }
    }

    fn task(id: &str, depends_on: &[&str], tier: TaskTier) -> EtaTask {
        EtaTask {
            id: id.to_string(),
            depends_on: depends_on.iter().map(|dep| (*dep).to_string()).collect(),
            rung: Some("mid".to_string()),
            tier,
        }
    }

    fn minutes(secs: f64) -> f64 {
        (secs / 60.0 * 10.0).round() / 10.0
    }

    /// gap-c1045f, as in Undertow phase 2's run c91b63ea: a short T1, then T2
    /// and T3 side by side, then T4, with integrative tasks on mid taking 5 to
    /// 20 minutes before. When T1 is done in 4 minutes, today's rule says
    /// elapsed / done × remaining = 12; the estimate counts the parallel tasks
    /// (architectural, scaled from integrative history by 300/150 lines) and
    /// T4 after them. When T2 is done at 24 minutes, it counts T3's 20
    /// minutes of running, so T3 is no longer forecast as a fresh task.
    #[test]
    fn eta_counts_running_parallel_tasks_and_the_task_after_them() {
        use TaskTier::{Architectural, Integrative};
        let history = [5.0, 8.0, 10.0, 12.0, 20.0].map(|m| past("mid", "integrative", m));
        let durations = TaskDurations::new(history.to_vec(), 900.0);
        let mut plan = PlanEta::new(vec![
            task("T1", &[], Integrative),
            task("T2", &["T1"], Architectural),
            task("T3", &["T1"], Architectural),
            task("T4", &["T2", "T3"], Integrative),
        ]);
        let start = Instant::now();
        let at = |m: u64| start + Duration::from_secs(m * 60);
        plan.started("T1", at(0));
        let finished = plan.finished("T1", at(4)).expect("T1 ran");
        assert_eq!(finished.0.as_deref(), Some("mid"));
        assert!((finished.2 - 240.0).abs() < 1e-9);
        let after_t1 = plan
            .estimate(&durations, 4, at(4))
            .expect("three tasks left");
        // The median integrative task takes about 10 minutes, an architectural
        // one about 20: the path is the slower of T2 and T3, then T4.
        let p50 = minutes(after_t1.p50_secs);
        assert!((25.0..35.0).contains(&p50), "{after_t1:?}");
        assert!(after_t1.p90_secs > 1.5 * after_t1.p50_secs, "{after_t1:?}");
        assert!(p50 > 12.0, "more than today's elapsed / done × remaining");
        assert_eq!(
            after_t1.basis.source, "scaled",
            "T2 and T3 have no architectural history"
        );
        assert_eq!((after_t1.basis.tasks, after_t1.basis.runs), (5, 1));

        plan.started("T2", at(4));
        plan.started("T3", at(4));
        plan.finished("T2", at(24));
        let after_t2 = plan
            .estimate(&durations, 4, at(24))
            .expect("two tasks left");
        // T3 has run 20 minutes: it counts what an architectural task that ran
        // that long still needs, then T4's 10 minutes.
        let fresh = durations
            .fit(Some("mid"), Architectural)
            .remaining(0.0, 0.5);
        let aged = durations
            .fit(Some("mid"), Architectural)
            .remaining(1_200.0, 0.5);
        assert!(aged > 0.0 && aged < fresh, "{aged} vs {fresh}");
        let t4 = durations.fit(Some("mid"), Integrative).remaining(0.0, 0.5);
        assert!(
            (after_t2.p50_secs - (aged + t4)).abs() < 1e-6,
            "{after_t2:?}"
        );

        // Running past the median keeps the estimate above zero.
        let late = plan
            .estimate(&durations, 4, at(4 + 120))
            .expect("still open");
        assert!(late.p50_secs > t4, "{late:?}");
        plan.finished("T3", at(30));
        plan.started("T4", at(30));
        plan.finished("T4", at(40));
        assert_eq!(plan.estimate(&durations, 4, at(40)), None);
    }

    /// gap-c1045f: a task's duration comes from its rung's tasks of its tier,
    /// else from the rung's other tiers scaled by size, else from every rung,
    /// else it is a guess at half the attempt timeout; this run's finished
    /// tasks join the history; a run's agent slots bound the estimate from
    /// below.
    #[test]
    fn durations_fall_back_from_history_to_a_guess() {
        use TaskTier::{Architectural, Focused, Integrative};
        // The caller's guess is half the attempt timeout.
        let guess = TaskDurations::new(Vec::new(), 900.0);
        let fit = guess.fit(Some("mid"), Integrative);
        assert_eq!(fit.source, Source::Guess);
        assert!((fit.remaining(0.0, 0.5) - 900.0).abs() < 1e-6);

        let mut durations = TaskDurations::new(
            [10.0, 10.0, 10.0]
                .map(|m| past("strong", "focused", m))
                .to_vec(),
            1_800.0,
        );
        // No mid history: every rung's tasks, scaled from focused (50 lines)
        // to integrative (150).
        let scaled = durations.fit(Some("mid"), Integrative);
        assert_eq!(scaled.source, Source::Scaled);
        assert!((scaled.remaining(0.0, 0.5) - 1_800.0).abs() < 1e-6);
        for minutes in [4.0, 5.0, 6.0] {
            durations.add(Some("mid"), Integrative, minutes * 60.0);
        }
        let own = durations.fit(Some("mid"), Integrative);
        assert_eq!(own.source, Source::History);
        assert!((own.remaining(0.0, 0.5) / 60.0 - 4.93).abs() < 0.01);
        assert!(
            own.past
                .iter()
                .all(|&index| durations.past[index].run == THIS_RUN)
        );
        assert_eq!(
            durations.fit(Some("mid"), Architectural).source,
            Source::Scaled
        );
        assert_eq!(
            durations.fit(Some("strong"), Focused).source,
            Source::History
        );

        // Three ten-minute tasks with nothing between them, on one slot, take
        // at least thirty minutes.
        let plan = PlanEta::new(
            ["A", "B", "C"]
                .map(|id| EtaTask {
                    id: id.to_string(),
                    depends_on: Vec::new(),
                    rung: Some("strong".to_string()),
                    tier: Focused,
                })
                .to_vec(),
        );
        let now = Instant::now();
        let one_slot = plan.estimate(&durations, 1, now).expect("open");
        let three_slots = plan.estimate(&durations, 3, now).expect("open");
        assert!(
            (minutes(one_slot.p50_secs) - 30.0).abs() < 0.2,
            "{one_slot:?}"
        );
        assert!(
            (minutes(three_slots.p50_secs) - 10.0).abs() < 0.2,
            "{three_slots:?}"
        );
    }
}
