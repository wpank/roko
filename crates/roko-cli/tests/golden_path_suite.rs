//! The golden-path suite (gap-3aa9cb): canaries C1–C8 of assessment W8, the
//! regulator's regression suite. CI's `golden-path` job runs each canary's
//! test target on every pull request. This test keeps the table below, the
//! canaries' shared scripted provider (`tests/common/scripted_provider.rs`)
//! and that job in step.

use std::fs;
use std::path::Path;

/// Each canary and its test target. Closed items' verify commands name these
/// targets, so they keep their names.
const CANARIES: [(&str, &str); 8] = [
    ("C1", "honest_verdicts_canary"),
    ("C2", "secrets_and_git_guard_canary"),
    ("C3", "plan_branch_integration"),
    ("C4", "plan_branch_integration"),
    ("C5", "attempt_diff_canary"),
    ("C6", "scheduler_canary"),
    ("C7", "supervision_canary"),
    ("C8", "tier_ladder_canary"),
];

/// The `--test` targets the `golden-path` job of `workflow` names.
fn golden_path_targets(workflow: &str) -> Vec<String> {
    let mut lines = workflow
        .lines()
        .skip_while(|line| line.trim_end() != "  golden-path:");
    assert!(lines.next().is_some(), "ci.yml has no golden-path job");
    // The job's own lines are indented deeper than the next job's name.
    let words: Vec<&str> = lines
        .take_while(|line| line.trim().is_empty() || line.starts_with("    "))
        .flat_map(str::split_whitespace)
        .collect();
    words
        .windows(2)
        .filter(|pair| pair[0] == "--test")
        .map(|pair| pair[1].to_string())
        .collect()
}

#[test]
fn golden_path_suite_covers_c1_to_c8() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = crate_dir.join("../../.github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow)
        .unwrap_or_else(|error| panic!("read {}: {error}", workflow.display()));
    let in_ci = golden_path_targets(&workflow);

    let mut gaps = Vec::new();
    for (canary, target) in CANARIES {
        let file = crate_dir.join("tests").join(format!("{target}.rs"));
        match fs::read_to_string(&file) {
            Err(_) => gaps.push(format!("{canary}: tests/{target}.rs does not exist")),
            Ok(source) if !source.contains("common::scripted_provider") => gaps.push(format!(
                "{canary}: tests/{target}.rs does not use common::scripted_provider"
            )),
            Ok(_) => {}
        }
        if !in_ci.iter().any(|ci_target| ci_target == target) {
            gaps.push(format!(
                "{canary}: the golden-path job does not run --test {target}"
            ));
        }
    }
    assert!(
        gaps.is_empty(),
        "the golden-path suite is incomplete:\n{}",
        gaps.join("\n")
    );
}

#[test]
fn golden_path_targets_are_read_from_the_job_alone() {
    let workflow = "jobs:\n  test:\n    steps:\n      - run: cargo test --test elsewhere\n  \
                    golden-path:\n    env:\n      CANARIES: >-\n        --test a\n        \
                    --test b\n\n    steps:\n      - run: cargo test $CANARIES\n  next:\n    \
                    steps:\n      - run: cargo test --test after\n";
    assert_eq!(golden_path_targets(workflow), ["a", "b"]);
}
