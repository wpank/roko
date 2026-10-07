//! `roko learn sizing` (3226, decision 3203): each tier's first-try verified
//! pass rate by task size, and the size limits it supports.
//!
//! Each task's first attempt in each run under `.roko/runs/` is joined, by
//! plan and task id, to the task in today's `tasks.toml`. Only verified
//! outcomes count (S01 §4.1): a first try with no learning label is left out,
//! and so is one whose `task_spec_hash` differs from today's task, since its
//! size may have changed after it ran ("spec changed"). The rest are grouped
//! by tier and size bucket (files 1, 2-3, 4-6, 7+; `max_loc` by quarters of
//! the tier's limit), each with its count, first-try pass rate and a Wilson
//! 95% interval. A tier's recommended limit on each axis is the largest
//! bucket whose rate stays at or above the target (0.80) with at least the
//! minimum sample (20), the rule decision 3203 set; a bucket with fewer first
//! tries is passed over. `TierSizeLimits` takes a recommendation only by a
//! reviewed commit (3227).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::io::{BufRead as _, BufReader};
use std::path::Path;

use roko_cli::task_parser::TaskDef;
use roko_core::task::TaskTier;
use roko_fs::RokoLayout;
use roko_learn::homeostasis::ev::{Z95, wilson_interval_at};
use roko_learn::telemetry::records::{
    ATTEMPTS_FILE, AttemptVerdictRecord, Stamped, VERDICT_SCHEMA, b3_digest,
};
use serde::Serialize;

/// The first-try pass rate a size bucket must reach (decision 3203).
pub(crate) const TARGET: f64 = 0.80;

/// The fewest first tries a bucket needs before it counts (decision 3203).
pub(crate) const MIN_SAMPLE: usize = 20;

/// The `files` buckets: label, lowest and highest count, the last one open.
const FILE_BUCKETS: [(&str, usize, Option<usize>); 4] = [
    ("1", 0, Some(1)),
    ("2-3", 2, Some(3)),
    ("4-6", 4, Some(6)),
    ("7+", 7, None),
];

/// One task's first try in one run, joined to today's task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SizingRow {
    tier: TaskTier,
    files: usize,
    max_loc: Option<u32>,
    /// Whether the first try passed verification (`learning_label = 1`).
    passed: bool,
}

/// First tries left out of the report, by why.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub(crate) struct Excluded {
    /// The task changed after it ran: its spec hash differs from today's.
    spec_changed: usize,
    /// No plan under `plans/` has the task now.
    no_task: usize,
    /// The first try was not verified: it has no learning label.
    unlabelled: usize,
}

/// One tier and size bucket.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct SizingCell {
    tier: &'static str,
    /// `files` or `max_loc`.
    axis: &'static str,
    bucket: String,
    n: usize,
    passed: usize,
    rate: f64,
    /// The Wilson 95% interval of the rate.
    low: f64,
    high: f64,
}

/// The largest bucket on one axis whose rate holds the target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Recommended {
    bucket: String,
    /// Its upper bound; `None` for the open bucket, which measures no cap.
    max: Option<usize>,
}

/// A tier's recommended limits; `None` on an axis where no bucket qualifies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct TierRecommendation {
    tier: &'static str,
    files: Option<Recommended>,
    max_loc: Option<Recommended>,
}

/// What `roko learn sizing` reports.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct SizingReport {
    target: f64,
    min_sample: usize,
    /// First tries counted.
    rows: usize,
    excluded: Excluded,
    /// Every tier and bucket with a first try, in tier and size order.
    cells: Vec<SizingCell>,
    recommendations: Vec<TierRecommendation>,
}

/// Each task's first try in every run of `workdir`, joined to today's tasks
/// in its plans directory, with what was left out.
pub(crate) fn load_rows(workdir: &Path) -> (Vec<SizingRow>, Excluded) {
    let tasks = current_tasks(workdir);
    let mut excluded = Excluded::default();
    let mut rows = Vec::new();
    for verdict in first_tries(&RokoLayout::for_project(workdir).runs_dir()) {
        let Some(label) = verdict.learning_label else {
            excluded.unlabelled += 1;
            continue;
        };
        let identity = &verdict.identity;
        let key = (identity.plan_id.clone(), identity.task_id.clone());
        let Some((task, hash)) = tasks.get(&key) else {
            excluded.no_task += 1;
            continue;
        };
        if verdict.task_spec_hash.as_deref() != Some(hash.as_str()) {
            excluded.spec_changed += 1;
            continue;
        }
        rows.push(SizingRow {
            tier: task.tier_class(),
            files: task.files.len(),
            max_loc: task.max_loc,
            passed: label == 1,
        });
    }
    (rows, excluded)
}

/// Today's tasks by (plan id, task id), each with the spec hash dispatch
/// gives it: `b3` of its definition JSON, as the Graph run stores it.
fn current_tasks(workdir: &Path) -> HashMap<(String, String), (TaskDef, String)> {
    let plans_dir = roko_cli::workspace_paths::plans_dir(workdir);
    let plans = roko_cli::runner::plan_loader::load_plans(&plans_dir).unwrap_or_default();
    let mut tasks = HashMap::new();
    for plan in plans {
        for task in plan.tasks.tasks {
            let json = serde_json::to_value(&task).unwrap_or_default().to_string();
            let hash = b3_digest(json.as_bytes());
            tasks.insert((plan.id.clone(), task.id.clone()), (task, hash));
        }
    }
    tasks
}

/// The verdict of each task's first attempt in each run under `runs_dir`:
/// the lowest attempt number per run, plan and task.
fn first_tries(runs_dir: &Path) -> Vec<AttemptVerdictRecord> {
    let mut first: BTreeMap<(String, String, String), AttemptVerdictRecord> = BTreeMap::new();
    let Ok(runs) = std::fs::read_dir(runs_dir) else {
        return Vec::new();
    };
    for run in runs.flatten() {
        let Ok(file) = std::fs::File::open(run.path().join(ATTEMPTS_FILE)) else {
            continue;
        };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(stamped) = serde_json::from_str::<Stamped<AttemptVerdictRecord>>(&line) else {
                continue;
            };
            if stamped.schema_version != VERDICT_SCHEMA {
                continue;
            }
            let verdict = stamped.record;
            let identity = &verdict.identity;
            let key = (
                identity.run_id.clone(),
                identity.plan_id.clone(),
                identity.task_id.clone(),
            );
            match first.get(&key) {
                Some(kept) if kept.identity.attempt <= identity.attempt => {}
                _ => {
                    first.insert(key, verdict);
                }
            }
        }
    }
    first.into_values().collect()
}

/// The report over `rows`: each tier's cells and its recommendation.
pub(crate) fn sizing_report(
    rows: &[SizingRow],
    excluded: Excluded,
    target: f64,
    min_sample: usize,
) -> SizingReport {
    let file_buckets: Vec<(String, usize, Option<usize>)> = FILE_BUCKETS
        .iter()
        .map(|(label, low, high)| ((*label).to_string(), *low, *high))
        .collect();
    let mut cells = Vec::new();
    let mut recommendations = Vec::new();
    for tier in TaskTier::ALL {
        let tier_rows: Vec<SizingRow> = rows
            .iter()
            .filter(|row| row.tier == tier)
            .copied()
            .collect();
        if tier_rows.is_empty() {
            continue;
        }
        let files = axis_cells(tier, "files", &file_buckets, &tier_rows, |row| {
            Some(row.files)
        });
        let locs = axis_cells(tier, "max_loc", &loc_buckets(tier), &tier_rows, |row| {
            row.max_loc.map(|loc| loc as usize)
        });
        recommendations.push(TierRecommendation {
            tier: tier.label(),
            files: recommend(&files, target, min_sample),
            max_loc: recommend(&locs, target, min_sample),
        });
        cells.extend(files.into_iter().chain(locs).map(|(cell, _)| cell));
    }
    SizingReport {
        target,
        min_sample,
        rows: rows.len(),
        excluded,
        cells,
        recommendations,
    }
}

/// A tier's `max_loc` buckets: the four quarters of its limit, then over it.
fn loc_buckets(tier: TaskTier) -> Vec<(String, usize, Option<usize>)> {
    let limit = tier.max_loc() as usize;
    let mut buckets = Vec::new();
    let mut low = 0;
    for quarter in 1..=4 {
        let high = (limit * quarter / 4).max(low);
        buckets.push((format!("{low}-{high}"), low, Some(high)));
        low = high + 1;
    }
    buckets.push((format!("{low}+"), low, None));
    buckets
}

/// The non-empty cells of one axis, in bucket order, each with its bucket's
/// upper bound.
fn axis_cells(
    tier: TaskTier,
    axis: &'static str,
    buckets: &[(String, usize, Option<usize>)],
    rows: &[SizingRow],
    size: impl Fn(&SizingRow) -> Option<usize>,
) -> Vec<(SizingCell, Option<usize>)> {
    let mut cells = Vec::new();
    for (bucket, low, high) in buckets {
        let (mut n, mut passed) = (0, 0);
        for row in rows {
            let inside = size(row)
                .is_some_and(|value| value >= *low && high.is_none_or(|high| value <= high));
            if inside {
                n += 1;
                passed += usize::from(row.passed);
            }
        }
        if n == 0 {
            continue;
        }
        let (interval_low, interval_high) = wilson_interval_at(passed, n, Z95);
        let cell = SizingCell {
            tier: tier.label(),
            axis,
            bucket: bucket.clone(),
            n,
            passed,
            rate: passed as f64 / n as f64,
            low: interval_low,
            high: interval_high,
        };
        cells.push((cell, *high));
    }
    cells
}

/// The largest bucket whose rate holds `target`, walking up from the
/// smallest: a bucket with fewer than `min_sample` first tries is passed
/// over, and the first one below the target ends the walk.
fn recommend(
    cells: &[(SizingCell, Option<usize>)],
    target: f64,
    min_sample: usize,
) -> Option<Recommended> {
    let mut best = None;
    for (cell, high) in cells {
        if cell.n < min_sample {
            continue;
        }
        if cell.rate < target {
            break;
        }
        best = Some(Recommended {
            bucket: cell.bucket.clone(),
            max: *high,
        });
    }
    best
}

/// The report as `roko learn sizing` prints it.
pub(crate) fn render_text(report: &SizingReport) -> String {
    let excluded = report.excluded;
    let mut text = format!(
        "learn sizing: {} first tries ({} spec changed, {} without a task in plans/, {} \
         unverified left out); target {:.2}, at least {} per bucket\n",
        report.rows,
        excluded.spec_changed,
        excluded.no_task,
        excluded.unlabelled,
        report.target,
        report.min_sample
    );
    if report.cells.is_empty() {
        text.push_str("  no verified first tries to report\n");
        return text;
    }
    let _ = writeln!(
        text,
        "  {:<14}{:<9}{:<9}{:>5}{:>7}  95% interval",
        "tier", "axis", "bucket", "n", "pass"
    );
    for cell in &report.cells {
        let _ = writeln!(
            text,
            "  {:<14}{:<9}{:<9}{:>5}{:>7.2}  {:.2}-{:.2}",
            cell.tier, cell.axis, cell.bucket, cell.n, cell.rate, cell.low, cell.high
        );
    }
    text.push_str("recommended limits (decision 3203):\n");
    let describe = |limit: Option<&Recommended>, axis: &str| match limit {
        Some(Recommended {
            bucket,
            max: Some(max),
        }) => format!("{axis} <= {max} (bucket {bucket})"),
        Some(Recommended { bucket, max: None }) => {
            format!("{axis}: no cap measured (bucket {bucket})")
        }
        None => format!("{axis}: not enough data"),
    };
    for recommendation in &report.recommendations {
        let _ = writeln!(
            text,
            "  {}: {}; {}",
            recommendation.tier,
            describe(recommendation.files.as_ref(), "files"),
            describe(recommendation.max_loc.as_ref(), "max_loc")
        );
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_learn::telemetry::records::{AttemptIdentity, AttemptKey, AttemptOutcome};

    const PLAN: &str = "[meta]\nplan = \"p\"\n\n\
        [[task]]\nid = \"T1\"\ntitle = \"One file\"\ndescription = \"Add a.rs.\"\n\
        role = \"implementer\"\ntier = \"mechanical\"\nfiles = [\"a.rs\"]\nmax_loc = 5\n\
        depends_on = []\nverify = [{ phase = \"test\", command = \"test -f a.rs\" }]\n\n\
        [[task]]\nid = \"T2\"\ntitle = \"Three files\"\ndescription = \"Add b.rs to d.rs.\"\n\
        role = \"implementer\"\ntier = \"mechanical\"\nfiles = [\"b.rs\", \"c.rs\", \"d.rs\"]\n\
        max_loc = 20\ndepends_on = []\n\
        verify = [{ phase = \"test\", command = \"test -f d.rs\" }]\n";

    /// A verdict line for `task`'s attempt in `run` of plan `p`.
    fn verdict(run: &str, task: &str, attempt: u32, label: Option<u8>, hash: &str) -> String {
        let identity = AttemptIdentity::new(&AttemptKey::new(run, "p", task, attempt));
        let outcome = match label {
            Some(1) => AttemptOutcome::Passed,
            Some(_) => AttemptOutcome::GateFailed,
            None => AttemptOutcome::Unverified,
        };
        let mut record = AttemptVerdictRecord::settle(identity, outcome, true);
        record.learning_label = label;
        record.task_spec_hash = Some(hash.to_string());
        serde_json::to_string(&Stamped {
            schema_version: VERDICT_SCHEMA.to_string(),
            record_id: String::new(),
            seq: 0,
            ts: String::new(),
            record,
        })
        .expect("serialize the verdict")
    }

    /// 3226: first tries join today's tasks; a changed spec, a task no plan
    /// has and an unverified first try are left out; the cells and the
    /// recommendation match values computed by hand (Wilson 95%).
    #[test]
    fn learn_sizing_reports_pass_rate_by_tier_and_size() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        std::fs::create_dir_all(workdir.join("plans/p")).expect("plan dir");
        std::fs::write(workdir.join("plans/p/tasks.toml"), PLAN).expect("write the plan");
        let tasks = current_tasks(workdir);
        let hash = |task: &str| tasks[&("p".to_string(), task.to_string())].1.clone();
        let (t1, t2) = (hash("T1"), hash("T2"));
        // T1 passes its first try in runs 1-3 and fails it in run 4, where its
        // retry passes; T2 passes in runs 1 and 4. Run 5's rows are left out:
        // T1 ran an older spec, T9 is in no plan now, T2 was not verified.
        let runs = [
            vec![
                verdict("run-1", "T1", 1, Some(1), &t1),
                verdict("run-1", "T2", 1, Some(1), &t2),
            ],
            vec![
                verdict("run-2", "T1", 1, Some(1), &t1),
                verdict("run-2", "T2", 1, Some(0), &t2),
            ],
            vec![
                verdict("run-3", "T1", 1, Some(1), &t1),
                verdict("run-3", "T2", 1, Some(0), &t2),
            ],
            vec![
                verdict("run-4", "T1", 2, Some(1), &t1),
                verdict("run-4", "T1", 1, Some(0), &t1),
                verdict("run-4", "T2", 1, Some(1), &t2),
            ],
            vec![
                verdict("run-5", "T1", 1, Some(1), "b3:older"),
                verdict("run-5", "T9", 1, Some(1), &t1),
                verdict("run-5", "T2", 1, None, &t2),
            ],
        ];
        for (index, lines) in runs.iter().enumerate() {
            let dir = workdir.join(format!(".roko/runs/run-{}", index + 1));
            std::fs::create_dir_all(&dir).expect("run dir");
            std::fs::write(dir.join(ATTEMPTS_FILE), lines.join("\n") + "\n").expect("attempts");
        }

        let (rows, excluded) = load_rows(workdir);
        let expected = Excluded {
            spec_changed: 1,
            no_task: 1,
            unlabelled: 1,
        };
        assert_eq!(excluded, expected);
        assert_eq!(rows.len(), 8, "{rows:?}");

        let report = sizing_report(&rows, excluded, 0.70, 4);
        let cell = |axis: &str, bucket: &str| {
            let found = report
                .cells
                .iter()
                .find(|cell| cell.axis == axis && cell.bucket == bucket);
            found.expect("the cell").clone()
        };
        let one = cell("files", "1");
        assert_eq!((one.tier, one.n, one.passed), ("mechanical", 4, 3));
        assert!((one.rate - 0.75).abs() < 1e-9, "{one:?}");
        assert!((one.low - 0.3006).abs() < 1e-4, "{one:?}");
        assert!((one.high - 0.9544).abs() < 1e-4, "{one:?}");
        let three = cell("files", "2-3");
        assert_eq!((three.n, three.passed), (4, 2));
        assert!((three.low - 0.1500).abs() < 1e-4, "{three:?}");
        assert!((three.high - 0.8500).abs() < 1e-4, "{three:?}");
        assert_eq!(cell("max_loc", "0-5").n, 4);
        assert_eq!(cell("max_loc", "16-20").passed, 2);
        assert_eq!(report.cells.len(), 4, "{:?}", report.cells);

        let recommended = |bucket: &str, max| {
            Some(Recommended {
                bucket: bucket.to_string(),
                max: Some(max),
            })
        };
        assert_eq!(
            report.recommendations,
            [TierRecommendation {
                tier: "mechanical",
                files: recommended("1", 1),
                max_loc: recommended("0-5", 5),
            }]
        );
        let text = render_text(&report);
        assert!(text.contains("1 spec changed"), "{text}");
        assert!(text.contains("files <= 1 (bucket 1)"), "{text}");

        // With the default sample, no bucket has enough first tries yet.
        let strict = sizing_report(&rows, excluded, TARGET, MIN_SAMPLE);
        assert_eq!(strict.recommendations[0].files, None);
    }
}
