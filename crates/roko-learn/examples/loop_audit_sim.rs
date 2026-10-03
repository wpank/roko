//! E2, the Monte Carlo check of S03's C4 (backlog 5117): the family-wise
//! rate of a false harm-demotion over K loops with no benefit, judged by the
//! state machine at its default parameters, and the legacy experiment rule's
//! A/A false-winner rate as the contrast. It spends $0.
//!
//! ```text
//! cargo run -p roko-learn --example loop_audit_sim -- [--reps N] [--seed S]
//! ```

use std::process::ExitCode;

use roko_learn::loop_audit::sim::{E2Config, e2_run, legacy_false_winners};

fn main() -> ExitCode {
    let mut reps = 10_000_u64;
    let mut seed = 7_u64;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let target = match arg.as_str() {
            "--reps" => &mut reps,
            "--seed" => &mut seed,
            other => return usage(&format!("unknown argument `{other}`")),
        };
        match args.next().and_then(|value| value.parse::<u64>().ok()) {
            Some(value) => *target = value,
            None => return usage(&format!("{arg} needs a whole number")),
        }
    }

    let config = E2Config::default();
    let result = e2_run(&config, reps, seed);
    let (low, high) = result.interval;
    println!(
        "C4 family-wise false harm-demotion: {:.4} ({}/{}, K = {}; 95% CI {low:.4}–{high:.4})",
        result.rate, result.demoted, result.reps, config.loops
    );
    let (winners, rate) = legacy_false_winners(reps, seed);
    println!("legacy check_conclusion A/A false-winner: {rate:.4} ({winners}/{reps})");
    ExitCode::SUCCESS
}

/// Print `message` and the usage line; exit code 2.
fn usage(message: &str) -> ExitCode {
    eprintln!("loop_audit_sim: {message}");
    eprintln!("usage: loop_audit_sim [--reps N] [--seed S]");
    ExitCode::from(2)
}
