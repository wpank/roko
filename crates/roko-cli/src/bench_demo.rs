//! `roko bench demo` — naive vs roko-optimized comparison.
//!
//! Runs a set of tasks twice:
//! 1. **Naive mode**: single model (opus), no caching, no routing, no knowledge
//! 2. **Optimized mode**: the configured model routing
//!
//! Without `--real` no model is called: [`simulate_task`] makes up every
//! figure from fixed per-difficulty constants, and the output says
//! "simulated" wherever such a figure is shown. Simulated figures are only
//! printed; the demo writes no result files.
//!
//! With `--real` each task is one model call. Tokens and latency are
//! measured; cost and cache hits are not, and no gate checks the output, so
//! no task is reported as passed or failed. A failed call is reported as an
//! error, never replaced with simulated figures.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::config::load_resolved_config;
use crate::inline::styled;
use crate::inline::symbols;
use crate::inline::terminal::{InlineTerminal, should_use_inline};
use crate::serve_runtime::BenchDispatchResult;
use crate::tui::Theme;
use anyhow::Result;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

/// Model the naive mode forces in real dispatch.
const NAIVE_MODEL: &str = "claude-opus-4-5";

/// A single benchmark task.
#[derive(Debug, Clone)]
pub struct BenchTask {
    /// Task ID.
    pub id: String,
    /// Task description.
    pub description: String,
    /// Difficulty level.
    pub difficulty: String,
}

/// Result of running one task in one mode.
#[derive(Debug, Clone)]
pub struct BenchResult {
    /// Task ID.
    pub task_id: String,
    /// Mode label ("naive" or "optimized").
    pub mode: String,
    /// Whether every figure was made up by [`simulate_task`].
    pub simulated: bool,
    /// Whether the task passed all gates, or `None` when no gate ran.
    pub passed: Option<bool>,
    /// Cost in USD, or `None` when it was not measured.
    pub cost_usd: Option<f64>,
    /// Input tokens.
    pub input_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
    /// Cache hit rate, or `None` when it was not measured.
    pub cache_hit_rate: Option<f64>,
    /// Wall time in seconds.
    pub duration_s: f64,
    /// Model used.
    pub model: String,
    /// Why the model call failed, when it did.
    pub error: Option<String>,
}

/// Aggregated benchmark summary for one mode.
#[derive(Debug, Clone)]
pub struct ModeSummary {
    pub mode: String,
    /// Whether every result in the mode was simulated.
    pub simulated: bool,
    /// Total cost, or `None` when any task's cost was not measured.
    pub total_cost: Option<f64>,
    pub total_tokens: u64,
    /// Share of verified tasks that passed, or `None` when no task was verified.
    pub pass_rate: Option<f64>,
    /// Mean cache hit rate, or `None` when any task's was not measured.
    pub avg_cache_hit: Option<f64>,
    pub avg_duration_s: f64,
    pub tasks_run: u32,
    /// Tasks whose gates ran, so they passed or failed.
    pub tasks_verified: u32,
    pub tasks_passed: u32,
    /// Tasks whose model call failed.
    pub tasks_errored: u32,
    pub primary_model: String,
}

/// Default benchmark task set.
pub fn default_tasks() -> Vec<BenchTask> {
    vec![
        BenchTask {
            id: "B01".into(),
            description: "Add --dry-run flag to plan command".into(),
            difficulty: "easy".into(),
        },
        BenchTask {
            id: "B02".into(),
            description: "Fix typo in error message".into(),
            difficulty: "easy".into(),
        },
        BenchTask {
            id: "B03".into(),
            description: "Add unit test for CascadeRouter".into(),
            difficulty: "medium".into(),
        },
        BenchTask {
            id: "B04".into(),
            description: "Refactor gate pipeline error handling".into(),
            difficulty: "medium".into(),
        },
        BenchTask {
            id: "B05".into(),
            description: "Implement cost waterfall decomposition".into(),
            difficulty: "hard".into(),
        },
    ]
}

/// Run the benchmark demo.
///
/// This runs tasks in two modes and displays a comparison. When `real_dispatch`
/// is false, every figure is simulated and labeled as simulated.
pub async fn run_bench_demo(workdir: &Path, real_dispatch: bool) -> Result<()> {
    let tasks = default_tasks();

    if should_use_inline() {
        run_bench_inline(workdir, &tasks, real_dispatch).await
    } else {
        run_bench_plain(workdir, &tasks, real_dispatch).await
    }
}

async fn run_bench_inline(workdir: &Path, tasks: &[BenchTask], real_dispatch: bool) -> Result<()> {
    let mut term = InlineTerminal::new().map_err(|e| anyhow::anyhow!("init terminal: {e}"))?;
    let theme = *term.theme();

    // Header
    term.push_lines_revealed(
        &[
            styled::section_start(
                &theme,
                "bench",
                &format!("{} tasks", tasks.len()),
                Some(source_banner(real_dispatch)),
            ),
            styled::continuation(
                &theme,
                "mode 1",
                &mode_description("naive", real_dispatch),
                None,
            ),
            styled::continuation(
                &theme,
                "mode 2",
                &mode_description("optimized", real_dispatch),
                None,
            ),
        ],
        Duration::from_millis(30),
    )?;
    term.push_blank()?;

    let naive = run_mode_inline(&mut term, &theme, workdir, tasks, "naive", real_dispatch).await?;
    let optimized = run_mode_inline(
        &mut term,
        &theme,
        workdir,
        tasks,
        "optimized",
        real_dispatch,
    )
    .await?;

    // Comparison table
    term.push_separator()?;
    let figures = if real_dispatch {
        "measured tokens and latency"
    } else {
        "simulated figures"
    };
    let comparison_lines = vec![
        styled::section_start(&theme, "comparison", "naive vs optimized", Some(figures)),
        Line::from(vec![
            Span::styled(symbols::BAR.to_string(), theme.muted()),
            Span::raw("  "),
            Span::styled(format!("{:<18}", ""), Style::default().fg(Theme::TEXT_DIM)),
            Span::styled(
                format!("{:<14}", "NAIVE"),
                Style::default()
                    .fg(Theme::EMBER)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<14}", "OPTIMIZED"),
                Style::default()
                    .fg(Theme::SAGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "IMPROVEMENT".to_string(),
                Style::default()
                    .fg(Theme::BONE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        comparison_row(
            &theme,
            "total cost",
            &fmt_usd(naive.total_cost),
            &fmt_usd(optimized.total_cost),
            &cost_ratio(naive.total_cost, optimized.total_cost),
        ),
        comparison_row(
            &theme,
            "total tokens",
            &naive.total_tokens.to_string(),
            &optimized.total_tokens.to_string(),
            &pct_change(naive.total_tokens as f64, optimized.total_tokens as f64),
        ),
        comparison_row(
            &theme,
            "pass rate",
            &fmt_pct(naive.pass_rate),
            &fmt_pct(optimized.pass_rate),
            &pp_change(naive.pass_rate, optimized.pass_rate),
        ),
        comparison_row(
            &theme,
            "avg latency",
            &format!("{:.1}s", naive.avg_duration_s),
            &format!("{:.1}s", optimized.avg_duration_s),
            &pct_change(naive.avg_duration_s, optimized.avg_duration_s),
        ),
        comparison_row(
            &theme,
            "cache hit",
            &fmt_pct(naive.avg_cache_hit),
            &fmt_pct(optimized.avg_cache_hit),
            &pp_change(naive.avg_cache_hit, optimized.avg_cache_hit),
        ),
        comparison_row(
            &theme,
            "primary model",
            &naive.primary_model,
            &optimized.primary_model,
            "-",
        ),
    ];
    term.push_lines_revealed(&comparison_lines, Duration::from_millis(40))?;

    // Final result
    let (icon, style) = if real_dispatch {
        (symbols::INFO, theme.text())
    } else {
        (symbols::WARN, theme.warning())
    };
    term.push_blank()?;
    term.push_lines(&[Line::from(vec![
        Span::styled(format!("{icon} "), style),
        Span::styled(closing_line(&naive, &optimized, real_dispatch), style),
    ])])?;
    term.push_blank()?;

    drop(term);
    Ok(())
}

/// Run every task in one mode, printing a line per task and the mode's total.
async fn run_mode_inline(
    term: &mut InlineTerminal,
    theme: &Theme,
    workdir: &Path,
    tasks: &[BenchTask],
    mode: &str,
    real_dispatch: bool,
) -> Result<ModeSummary> {
    term.push_separator()?;
    term.push_lines(&[styled::section_start(
        theme,
        mode,
        &mode_description(mode, real_dispatch),
        None,
    )])?;

    let mut results = Vec::with_capacity(tasks.len());
    for task in tasks {
        let result = run_task(workdir, task, mode, real_dispatch).await;

        term.push_lines(&[Line::from(vec![
            Span::styled(symbols::BAR.to_string(), theme.muted()),
            Span::raw(" "),
            Span::styled(
                verdict_icon(&result).to_string(),
                verdict_style(theme, &result),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:<4}", task.id),
                Style::default().fg(Theme::TEXT_DIM),
            ),
            Span::styled(format!("{:<40}", task.description), theme.text()),
            Span::styled(fmt_usd(result.cost_usd), Style::default().fg(Theme::SAGE)),
            Span::raw("  "),
            Span::styled(
                format!("{}tok", result.input_tokens + result.output_tokens),
                Style::default().fg(Theme::TEXT_DIM),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{:.1}s", result.duration_s),
                Style::default().fg(Theme::TEXT_DIM),
            ),
            Span::raw("  "),
            Span::styled(result.model.clone(), Style::default().fg(Theme::DREAM)),
            Span::raw("  "),
            Span::styled(
                format!("cache:{}", fmt_pct(result.cache_hit_rate)),
                Style::default().fg(Theme::TEXT_DIM),
            ),
            Span::raw("  "),
            Span::styled(verdict_note(&result), theme.muted()),
        ])])?;

        results.push(result);
        if !real_dispatch {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    let summary = summarize(&results);
    term.push_lines(&[styled::section_end(theme, "total", &mode_total(&summary))])?;
    term.push_blank()?;
    Ok(summary)
}

fn comparison_row(
    theme: &Theme,
    label: &str,
    naive: &str,
    optimized: &str,
    improvement: &str,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(symbols::BAR.to_string(), theme.muted()),
        Span::raw("  "),
        Span::styled(
            format!("{:<18}", label),
            Style::default().fg(Theme::TEXT_DIM),
        ),
        Span::styled(format!("{:<14}", naive), theme.text()),
        Span::styled(format!("{:<14}", optimized), theme.text()),
        Span::styled(improvement.to_string(), Style::default().fg(Theme::SAGE)),
    ])
}

fn summarize(results: &[BenchResult]) -> ModeSummary {
    let n = results.len() as u32;
    let verified = results.iter().filter(|r| r.passed.is_some()).count() as u32;
    let passed = results.iter().filter(|r| r.passed == Some(true)).count() as u32;
    ModeSummary {
        mode: results.first().map(|r| r.mode.clone()).unwrap_or_default(),
        simulated: !results.is_empty() && results.iter().all(|r| r.simulated),
        total_cost: results.iter().map(|r| r.cost_usd).sum::<Option<f64>>(),
        total_tokens: results
            .iter()
            .map(|r| r.input_tokens + r.output_tokens)
            .sum(),
        pass_rate: if verified > 0 {
            Some(passed as f64 / verified as f64)
        } else {
            None
        },
        avg_cache_hit: results
            .iter()
            .map(|r| r.cache_hit_rate)
            .sum::<Option<f64>>()
            .map(|total| total / n.max(1) as f64),
        avg_duration_s: results.iter().map(|r| r.duration_s).sum::<f64>() / n.max(1) as f64,
        tasks_run: n,
        tasks_verified: verified,
        tasks_passed: passed,
        tasks_errored: results.iter().filter(|r| r.error.is_some()).count() as u32,
        primary_model: results.first().map(|r| r.model.clone()).unwrap_or_default(),
    }
}

/// Where the figures come from, shown at the top of every run.
fn source_banner(real_dispatch: bool) -> &'static str {
    if real_dispatch {
        "real dispatch: tokens and latency are measured; cost is not, and no gate runs"
    } else {
        "SIMULATED: no model is called; every figure comes from fixed constants"
    }
}

/// What a mode stands for. A simulated mode names the premise of its
/// constants; a real mode names only what the dispatch does.
fn mode_description(mode: &str, real_dispatch: bool) -> String {
    match (mode, real_dispatch) {
        ("naive", false) => "simulated: opus, no cache, no routing".to_string(),
        (_, false) => "simulated: cascade router, cache, gates, knowledge".to_string(),
        ("naive", true) => format!("{NAIVE_MODEL}, one model call per task"),
        (_, true) => "configured model routing, one model call per task".to_string(),
    }
}

/// The icon for a result's verdict. A task no gate checked gets a neutral
/// icon, never the pass mark.
fn verdict_icon(result: &BenchResult) -> &'static str {
    match (result.passed, result.error.is_some()) {
        (Some(true), _) => symbols::PASS,
        (Some(false), _) => symbols::FAIL,
        (None, true) => symbols::WARN,
        (None, false) => symbols::PENDING,
    }
}

/// The style for a result's verdict icon.
fn verdict_style(theme: &Theme, result: &BenchResult) -> Style {
    match (result.passed, result.error.is_some()) {
        (Some(true), _) => theme.success(),
        (Some(false), _) => theme.danger(),
        (None, true) => theme.warning(),
        (None, false) => theme.muted(),
    }
}

/// A result's verdict in words; a simulated verdict says so.
fn verdict_note(result: &BenchResult) -> String {
    if let Some(error) = &result.error {
        return format!("error: {error}");
    }
    let verdict = match result.passed {
        Some(true) => "pass",
        Some(false) => "fail",
        None => "not verified: no gate ran",
    };
    if result.simulated {
        format!("{verdict} (simulated)")
    } else {
        verdict.to_string()
    }
}

/// A mode's totals: cost, tokens, and verdicts; simulated totals say so.
fn mode_total(summary: &ModeSummary) -> String {
    let sep = symbols::SEP;
    let cost = fmt_usd(summary.total_cost);
    let tokens = summary.total_tokens;
    let mut total = if summary.tasks_verified > 0 {
        let (passed, verified) = (summary.tasks_passed, summary.tasks_verified);
        format!("{cost}  {sep}  {tokens}tok  {sep}  {passed}/{verified} passed")
    } else {
        let run = summary.tasks_run;
        format!("{cost}  {sep}  {tokens}tok  {sep}  {run} not verified")
    };
    if summary.tasks_errored > 0 {
        let errored = summary.tasks_errored;
        total.push_str(&format!("  {sep}  {errored} failed calls"));
    }
    if summary.simulated {
        total.push_str("  (simulated)");
    }
    total
}

/// The closing line. A simulated run says it is a simulation and never calls
/// itself a benchmark; a real run says what it did not measure.
fn closing_line(naive: &ModeSummary, optimized: &ModeSummary, real_dispatch: bool) -> String {
    let sep = symbols::SEP;
    if real_dispatch {
        format!(
            "real dispatch complete  {sep}  {} model calls, {} failed  {sep}  \
             tokens and latency measured; cost not measured; \
             no gate ran, so no task passed or failed",
            naive.tasks_run + optimized.tasks_run,
            naive.tasks_errored + optimized.tasks_errored,
        )
    } else {
        format!(
            "simulation complete  {sep}  {} simulated cost reduction  {sep}  \
             {}/{} simulated passes  {sep}  no model was called; not a benchmark result",
            cost_ratio(naive.total_cost, optimized.total_cost),
            optimized.tasks_passed,
            optimized.tasks_run,
        )
    }
}

/// Dollars, or "cost n/a" when the cost was not measured.
fn fmt_usd(cost: Option<f64>) -> String {
    match cost {
        Some(cost) => format!("${cost:.4}"),
        None => "cost n/a".to_string(),
    }
}

/// A 0-1 rate as a percentage, or "n/a" when it was not measured.
fn fmt_pct(rate: Option<f64>) -> String {
    match rate {
        Some(rate) => format!("{:.0}%", rate * 100.0),
        None => "n/a".to_string(),
    }
}

/// How many times cheaper optimized was, or "n/a" without both costs.
fn cost_ratio(naive: Option<f64>, optimized: Option<f64>) -> String {
    match (naive, optimized) {
        (Some(naive), Some(optimized)) if optimized > 0.0 => format!("{:.1}x", naive / optimized),
        _ => "n/a".to_string(),
    }
}

/// Relative change from naive to optimized, such as "-57%".
fn pct_change(naive: f64, optimized: f64) -> String {
    if naive > 0.0 {
        format!("{:+.0}%", (optimized / naive - 1.0) * 100.0)
    } else {
        "n/a".to_string()
    }
}

/// Change in percentage points, or "n/a" without both rates.
fn pp_change(naive: Option<f64>, optimized: Option<f64>) -> String {
    match (naive, optimized) {
        (Some(naive), Some(optimized)) => format!("{:+.0}pp", (optimized - naive) * 100.0),
        _ => "n/a".to_string(),
    }
}

/// Run one task: simulated, or one real model call.
async fn run_task(
    workdir: &Path,
    task: &BenchTask,
    mode: &str,
    real_dispatch: bool,
) -> BenchResult {
    if real_dispatch {
        run_task_real(workdir, task, mode).await
    } else {
        simulate_task(task, mode)
    }
}

/// Simulate a result from fixed per-difficulty constants.
///
/// Every figure is made up. The result is marked `simulated`, so the output
/// labels it wherever it is shown.
fn simulate_task(task: &BenchTask, mode: &str) -> BenchResult {
    let is_naive = mode == "naive";
    let difficulty_factor = match task.difficulty.as_str() {
        "easy" => 1.0,
        "medium" => 2.0,
        "hard" => 3.5,
        _ => 1.0,
    };

    let (cost, tokens_in, tokens_out, cache_hit, duration, model, pass_rate) = if is_naive {
        (
            0.85 * difficulty_factor,
            (4000.0 * difficulty_factor) as u64,
            (1200.0 * difficulty_factor) as u64,
            0.0,
            12.0 * difficulty_factor,
            "opus".to_string(),
            0.75,
        )
    } else {
        (
            0.028 * difficulty_factor,
            (1800.0 * difficulty_factor) as u64,
            (500.0 * difficulty_factor) as u64,
            0.82 + (0.10 * (1.0 / difficulty_factor)),
            3.2 * difficulty_factor,
            if difficulty_factor > 2.0 {
                "sonnet".to_string()
            } else {
                "haiku".to_string()
            },
            0.90,
        )
    };

    // Deterministic "randomness" based on task ID
    let seed: u64 = task.id.bytes().map(|b| b as u64).sum();
    let passed = (seed % 100) as f64 / 100.0 < pass_rate;

    BenchResult {
        task_id: task.id.clone(),
        mode: mode.to_string(),
        simulated: true,
        passed: Some(passed),
        cost_usd: Some(cost),
        input_tokens: tokens_in,
        output_tokens: tokens_out,
        cache_hit_rate: Some(cache_hit),
        duration_s: duration,
        model,
        error: None,
    }
}

/// Run a task for real via `dispatch_bench_prompt`.
///
/// Naive mode forces [`NAIVE_MODEL`]. Optimized mode uses configured routing.
/// See [`real_result`] for what the result reports.
async fn run_task_real(workdir: &Path, task: &BenchTask, mode: &str) -> BenchResult {
    let started = Instant::now();

    let mut config = load_resolved_config(workdir)
        .map(|resolved| resolved.config)
        .unwrap_or_else(|_| {
            let mut c = crate::config::Config::default();
            c.agent.command = "claude".into();
            c
        });

    let model_label = if mode == "naive" {
        config.agent.model = Some(NAIVE_MODEL.to_string());
        "opus".to_string()
    } else {
        config
            .agent
            .model
            .clone()
            .unwrap_or_else(|| "routed".to_string())
    };

    let prompt = format!(
        "Task: {}\nDifficulty: {}\n\nImplement the described task. Provide a complete, working solution.",
        task.description, task.difficulty
    );

    let model_override = config.agent.model.clone();
    let dispatch = crate::serve_runtime::dispatch_bench_prompt(
        workdir,
        &config,
        &prompt,
        model_override.as_deref(),
    )
    .await;
    real_result(
        task,
        mode,
        model_label,
        started.elapsed().as_secs_f64(),
        dispatch,
    )
}

/// The result of one real model call.
///
/// It reports only what the call measured: tokens and wall time. No gate
/// checks these tasks, so the verdict is "not verified", never a pass, and
/// cost and cache hits stay unmeasured. A failed call is an error; it never
/// falls back to simulated figures.
fn real_result(
    task: &BenchTask,
    mode: &str,
    model: String,
    duration_s: f64,
    dispatch: Result<BenchDispatchResult>,
) -> BenchResult {
    let (input_tokens, output_tokens, error) = match dispatch {
        Ok(dispatch) => (dispatch.input_tokens, dispatch.output_tokens, None),
        Err(err) => {
            tracing::warn!(task = %task.id, mode, error = %err, "real dispatch failed");
            (0, 0, Some(format!("{err:#}")))
        }
    };

    BenchResult {
        task_id: task.id.clone(),
        mode: mode.to_string(),
        simulated: false,
        passed: None,
        cost_usd: None,
        input_tokens,
        output_tokens,
        cache_hit_rate: None,
        duration_s,
        model,
        error,
    }
}

async fn run_bench_plain(workdir: &Path, tasks: &[BenchTask], real_dispatch: bool) -> Result<()> {
    println!(
        "roko bench demo — {} tasks, naive vs optimized",
        tasks.len()
    );
    println!("{}", source_banner(real_dispatch));
    println!();

    let naive = run_mode_plain(workdir, tasks, "naive", real_dispatch).await;
    let optimized = run_mode_plain(workdir, tasks, "optimized", real_dispatch).await;
    println!("{}", closing_line(&naive, &optimized, real_dispatch));
    Ok(())
}

async fn run_mode_plain(
    workdir: &Path,
    tasks: &[BenchTask],
    mode: &str,
    real_dispatch: bool,
) -> ModeSummary {
    println!("--- {mode}: {} ---", mode_description(mode, real_dispatch));
    let mut results = Vec::with_capacity(tasks.len());
    for task in tasks {
        let result = run_task(workdir, task, mode, real_dispatch).await;
        println!("{}", plain_task_line(&result));
        results.push(result);
    }
    let summary = summarize(&results);
    println!("  total: {}", mode_total(&summary));
    println!();
    summary
}

/// One task's line in plain output.
fn plain_task_line(result: &BenchResult) -> String {
    format!(
        "  {} {}  {}  {}tok  {:.1}s  {}  cache:{}  {}",
        verdict_icon(result),
        result.task_id,
        fmt_usd(result.cost_usd),
        result.input_tokens + result.output_tokens,
        result.duration_s,
        result.model,
        fmt_pct(result.cache_hit_rate),
        verdict_note(result),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulate_produces_results() {
        let tasks = default_tasks();
        for task in &tasks {
            let naive = simulate_task(task, "naive");
            let opt = simulate_task(task, "optimized");
            assert!(naive.simulated && opt.simulated);
            assert!(
                naive.cost_usd.expect("simulated cost") > opt.cost_usd.expect("simulated cost"),
                "optimized should be cheaper"
            );
            assert!(
                naive.duration_s > opt.duration_s,
                "optimized should be faster"
            );
            assert_eq!(
                naive.cache_hit_rate,
                Some(0.0),
                "naive should have no cache"
            );
            assert!(
                opt.cache_hit_rate.is_some_and(|rate| rate > 0.0),
                "optimized should have cache hits"
            );
        }
    }

    #[test]
    fn summarize_works() {
        let result = |task_id: &str, passed: bool, cost_usd: f64| BenchResult {
            task_id: task_id.into(),
            mode: "naive".into(),
            simulated: true,
            passed: Some(passed),
            cost_usd: Some(cost_usd),
            input_tokens: 1000,
            output_tokens: 500,
            cache_hit_rate: Some(0.0),
            duration_s: 10.0,
            model: "opus".into(),
            error: None,
        };
        let summary = summarize(&[result("T01", true, 1.0), result("T02", false, 2.0)]);
        assert_eq!(summary.tasks_run, 2);
        assert_eq!(summary.tasks_verified, 2);
        assert_eq!(summary.tasks_passed, 1);
        assert!(summary.simulated);
        assert!((summary.pass_rate.expect("verified") - 0.5).abs() < f64::EPSILON);
        assert!((summary.total_cost.expect("costed") - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn real_mode_does_not_fabricate_passes() {
        let task = &default_tasks()[0];

        // A successful call ran no gate: it neither passed nor failed, and its
        // cost was not measured.
        let answered = real_result(
            task,
            "optimized",
            "routed".to_string(),
            1.5,
            Ok(BenchDispatchResult {
                text: "done".to_string(),
                input_tokens: 120,
                output_tokens: 40,
            }),
        );
        assert!(!answered.simulated);
        assert_eq!(
            answered.passed, None,
            "no gate ran, so the task did not pass"
        );
        assert_eq!(answered.cost_usd, None, "cost was not measured");
        assert_eq!(answered.cache_hit_rate, None);
        assert_eq!((answered.input_tokens, answered.output_tokens), (120, 40));
        assert!(answered.error.is_none());
        assert_eq!(verdict_icon(&answered), symbols::PENDING);
        assert_eq!(verdict_note(&answered), "not verified: no gate ran");

        // A failed call is an error, never replaced with simulated figures.
        let failed = real_result(
            task,
            "naive",
            "opus".to_string(),
            0.2,
            Err(anyhow::anyhow!("provider down")),
        );
        assert!(!failed.simulated);
        assert_eq!(failed.passed, None);
        assert_eq!(failed.cost_usd, None);
        assert_eq!((failed.input_tokens, failed.output_tokens), (0, 0));
        assert!(
            failed
                .error
                .as_deref()
                .is_some_and(|error| error.contains("provider down"))
        );

        // Together they claim no pass rate and no cost.
        let summary = summarize(&[answered, failed]);
        assert!(!summary.simulated);
        assert_eq!(summary.tasks_verified, 0);
        assert_eq!(summary.tasks_passed, 0);
        assert_eq!(summary.tasks_errored, 1);
        assert_eq!(summary.pass_rate, None);
        assert_eq!(summary.total_cost, None);
        assert!(closing_line(&summary, &summary, true).contains("no gate ran"));
    }

    #[test]
    fn simulated_output_is_labeled_as_simulated() {
        let tasks = default_tasks();
        let naive: Vec<_> = tasks
            .iter()
            .map(|task| simulate_task(task, "naive"))
            .collect();
        let optimized: Vec<_> = tasks
            .iter()
            .map(|task| simulate_task(task, "optimized"))
            .collect();
        for result in naive.iter().chain(&optimized) {
            let line = plain_task_line(result);
            assert!(line.contains("(simulated)"), "unlabeled line: {line}");
        }

        let naive = summarize(&naive);
        let optimized = summarize(&optimized);
        assert!(mode_total(&naive).contains("(simulated)"));
        let closing = closing_line(&naive, &optimized, false);
        assert!(closing.contains("simulated cost reduction"), "{closing}");
        assert!(!closing.contains("benchmark complete"), "{closing}");
        assert!(source_banner(false).contains("SIMULATED"));
        assert!(mode_description("optimized", false).starts_with("simulated"));
        assert!(!mode_description("optimized", true).contains("gates"));
    }
}
