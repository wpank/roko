//! The report-only loop census (S03 §7 A1; backlog 5106): one row per
//! registered loop, from a workspace's learning logs and the loop registry,
//! with $0 spent and no transitions.
//!
//! ```text
//! cargo run -p roko-learn --example loop_census -- [--workdir DIR] [--json] [--out FILE]
//! ```
//!
//! Declared findings are judged against `ROKO_HARNESS_SHA`, else the
//! workdir's `git rev-parse HEAD`. Nothing is written but `--out`, which gets
//! the `roko.loop_census/1` JSON.

use std::path::PathBuf;
use std::process::ExitCode;

use roko_learn::loop_audit::Registry;
use roko_learn::loop_audit::census;

fn main() -> ExitCode {
    let mut workdir = PathBuf::from(".");
    let mut json = false;
    let mut out = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--workdir" => match args.next() {
                Some(dir) => workdir = PathBuf::from(dir),
                None => return usage("--workdir needs a directory"),
            },
            "--json" => json = true,
            "--out" => match args.next() {
                Some(path) => out = Some(PathBuf::from(path)),
                None => return usage("--out needs a file"),
            },
            other => return usage(&format!("unknown argument `{other}`")),
        }
    }

    let registry = match Registry::load(&workdir) {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("loop_census: {error}");
            return ExitCode::FAILURE;
        }
    };
    let sha = census::harness_sha(&workdir);
    let report = census::run(&workdir, &registry, sha.as_deref());
    let rendered = match census::render_json(&report) {
        Ok(rendered) => rendered,
        Err(error) => {
            eprintln!("loop_census: render: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(path) = &out
        && let Err(error) = std::fs::write(path, &rendered)
    {
        eprintln!("loop_census: write {}: {error}", path.display());
        return ExitCode::FAILURE;
    }
    if json {
        print!("{rendered}");
    } else {
        print!("{}", census::render_text(&report));
    }
    ExitCode::SUCCESS
}

/// Print `problem` and the usage line; exit code 2.
fn usage(problem: &str) -> ExitCode {
    eprintln!("loop_census: {problem}");
    eprintln!("usage: loop_census [--workdir DIR] [--json] [--out FILE]");
    ExitCode::from(2)
}
