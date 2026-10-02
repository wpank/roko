//! bench command handlers.

use crate::*;

#[derive(Debug, Subcommand)]
pub(crate) enum BenchCmd {
    /// Run a comparative benchmark: naive vs roko-optimized.
    #[command(after_help = "\
Examples:
  roko bench demo                     Run with simulated data
  roko bench demo --real              Run with real LLM dispatch")]
    Demo {
        /// Use real LLM dispatch instead of simulated results.
        #[arg(long)]
        real: bool,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Run a native SWE-bench-style proxy batch.
    #[command(after_help = "\
Examples:
  roko bench swe --batch-size 2 --agent-mode gold      (control: checks the harness)
  roko bench swe --dataset ./swe-smoke.jsonl --predictions ./predictions.jsonl --agent-mode prediction-file
  roko bench swe --agent-mode command --agent-command './my-agent.sh'")]
    Swe {
        /// Local JSONL dataset. If omitted, a built-in two-task smoke dataset is generated.
        #[arg(long)]
        dataset: Option<PathBuf>,
        /// Number of instances to run.
        #[arg(long, default_value_t = 2)]
        batch_size: usize,
        /// Offset into the dataset.
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// Agent adapter to use (required). `gold` and `empty` are controls: they check the
        /// harness, not a model, and are never recorded as learning.
        #[arg(long, value_enum)]
        agent_mode: roko_cli::bench::SweAgentMode,
        /// Predictions JSONL path for --agent-mode prediction-file.
        #[arg(long)]
        predictions: Option<PathBuf>,
        /// Command for --agent-mode command. Receives instance JSON on stdin, prints a unified diff.
        #[arg(long)]
        agent_command: Option<String>,
        /// Scores JSONL output path.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Write SWE-bench-style predictions JSONL.
        #[arg(long)]
        export_predictions: Option<PathBuf>,
        /// Disable learning episode, efficiency, and C-factor writes.
        #[arg(long)]
        no_learning: bool,
        /// Keep per-instance benchmark workdirs for debugging.
        #[arg(long)]
        keep_workdirs: bool,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

pub(crate) async fn cmd_bench(cli: &Cli, cmd: BenchCmd) -> Result<i32> {
    match cmd {
        BenchCmd::Demo { real, workdir } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            roko_cli::bench_demo::run_bench_demo(&workdir, real).await?;
            Ok(EXIT_SUCCESS)
        }
        BenchCmd::Swe {
            dataset,
            batch_size,
            offset,
            agent_mode,
            predictions,
            agent_command,
            report,
            export_predictions,
            no_learning,
            keep_workdirs,
            workdir,
        } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let report = roko_cli::bench::run_swe_bench(roko_cli::bench::SweBenchOptions {
                workdir,
                dataset,
                batch_size,
                offset,
                agent_mode,
                predictions,
                agent_command,
                report,
                export_predictions,
                record_learning: !no_learning,
                keep_workdirs,
            })
            .await?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", report.render_text());
                println!();
                println!(
                    "note: this is fast proxy scoring, not official SWE-bench Docker scoring."
                );
            }
            Ok(EXIT_SUCCESS)
        }
    }
}
