+++
id = "bug-a22228"
kind = "bug"
title = "Serve bench counts a task as passed when its output merely contains the expected text"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-serve/bench"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "39cd18049"
source = "tmp/cybernetic-harness/assessment-2026-09-28/s07-s09-bench.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/s07-s09-bench.md"
anchors = ["crates/roko-serve/src/routes/bench.rs:306", "crates/roko-serve/src/routes/bench.rs::scaffold_bench_workdir", "crates/roko-serve/src/routes/bench.rs::learnable_rust_lib_contents", "crates/roko-serve/src/bench.rs::builtin_learnable_rust_suite", "crates/roko-serve/src/bench.rs::estimate_cost_usd"]
links = { depends_on = [], blocks = [], related = ["bug-3a037b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'text.contains(expected.as_str())' crates/roko-serve/src/routes/bench.rs && cargo test -p roko-serve routes::bench::tests::unedited_learnable_scaffold_fails_grading"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in d25f9f2da. Serve bench grades only from an executed check that first fails on an untouched scaffold; suites without checks grade skipped. Batch 13 gate (MAIN 39cd18049 has the same crates and Cargo.lock as gated 172f3683a/d76f9faf8): cargo check --workspace --tests clean; nightly fmt clean after the coordinator's rustfmt commits on 7 branches; clippy -p (10 crates) --keep-going -D warnings clean after two doc-paragraph fixes (8b8ec4f25, e3deb0c37); lib tests pass: roko-cli 3160, roko-agent 2262, roko-core 1945, roko-learn 1199, roko-serve 977, roko-gate 689, roko-graph 472, roko-execution 252, roko-std 226, roko-acp 199. Three load flakes (turn_policy escalated-timeout, roko-gate tautology, verification efficiency-record wait) pass alone and are noted on bug-779ae7. Verify: its test passes in that run and its static checks pass on MAIN."
+++
- A serve bench task passes when the agent succeeded and its output contains `expected_output` as a substring (`roko-serve/src/routes/bench.rs:307-312`).
- The built-in `learnable-rust` suite ships `todo!()` stubs with no tests (`routes/bench.rs:1388`). Its expected outputs are strings like "Finished" and "test result: ok", which a build or an empty test run prints anyway, so it can pass without any edit. This is inferred from the scaffold; it was not run.
- All tasks share one temp dir.
- `estimate_cost_usd` prices models by substring and is out of date.

Fix: grade with executed checks (tests that fail before the change), not output substrings, and give each task its own workspace.

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-28becc` at `de59c1d1d`; cargo verification deferred to the
  batch check. `unedited_learnable_scaffold_fails_grading` and `learnable_reference_solutions_pass_grading` run
  `cargo test` on the scaffold (5 tasks at once, each with its own target dir) and take a few seconds each.
- Only learnable-rust has checks. Every other built-in or uploaded suite now grades as `skipped`. Letting uploaded
  suites define their own check commands would run commands from an API request on the host, which needs an authz
  decision first.
- Not done here: `estimate_cost_usd` (substring pricing) has no callers; the bench prices with `CostTable`. The
  `Demo` strategy still writes simulated tokens and cost into `.roko/bench` runs and the index.
