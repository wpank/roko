+++
id = "gap-9681d2"
kind = "gap"
title = "177 eprintln! call sites remain outside tests; tracing migration incomplete"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["workspace/logging"]
created = 2026-08-13
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "825d45f97"
source = "gaps-md#batch-2026-08-1213/eprintln-expect"
anchors = ["crates/roko-cli/src/prd.rs", "crates/roko-cli/src/main.rs", "crates/roko-cli/src/runner/output_sink.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -Eq 'print_stderr *= *\"(warn|deny)\"' Cargo.toml && test \"$(git grep -n 'eprintln!' -- 'crates/*.rs' ':!*/tests/*' ':!*tests.rs' ':!*/examples/*' ':!*/benches/*' | wc -l)\" -le 60"
+++

The 2026-08-12/13 conversions (`eprintln!` to `tracing`, `.expect()` to `Result`) were only partial. On 2026-09-28 there are 177 `eprintln!` calls in non-test Rust files. The largest counts are `crates/roko-cli/src/prd.rs` (39), `main.rs` (22) and `runner/output_sink.rs` (13). Some of these are legitimate user-facing stderr; the rest should use `tracing`. Backlog #381 is archived without a status. `.expect()` sites were not re-counted.

Fix: classify each call site as user-facing output or diagnostics, and convert the diagnostics. Add a lint or grep check that blocks new ones.

Re-checked 2026-09-29: unchanged; 174 non-test calls by a stricter count than the original 177, with the same top files. The parked, unverified gap-556ffe (created 2026-09-21, backlog #381, 'Complete eprintln! to tracing Migration') describes the same migration. Mark it duplicate_of this verified item, or fold it in, so only one item stays live.

## Remaining eprintln! sites (2026-10-01, work/gap-cd51b7)

156 sites remain by the verify's count. Each one stays for the reason in its row. Without `--verbose`, `ROKO_LOG` or `RUST_LOG`, the CLI writes tracing only to `.roko/roko.log` (main.rs, the `show_stderr` stderr layer), so converting user-facing output to tracing would hide it from the user.

| File | Lines | Why it stays |
|---|---|---|
| crates/roko-cli/src/main.rs | 3550, 3604, 3613, 3744, 3759, 3763, 4054, 4088, 4122, 4170, 4228, 4232, 4420, 4426, 4490, 4538, 4571, 4620-4623, 4626, 4627 | Not classified: skipped while wk-climain splits main.rs (gap-0d0e81) |
| crates/roko-cli/src/commands/ (develop.rs 10, util.rs 13, plan.rs 4, job.rs 3, history.rs 3, show.rs 1) | develop 160-211; util 14-103, 1520-1731; plan 1345, 1646, 1647, 1670; job 26, 270, 500; history 63-69; show 208 | Not classified: skipped with main.rs (gap-0d0e81) |
| crates/roko-cli/src/prd.rs | 1011, 1016, 1023 | User-facing: the auto-generated plan's outcome after `roko prd draft promote` |
| crates/roko-cli/src/prd.rs | 1032, 1033 | User-facing: error and recovery hint before giving up |
| crates/roko-cli/src/prd.rs | 1651, 1660 | User-facing: the generated plan may reference code that does not exist |
| crates/roko-cli/src/prd.rs | 1697, 1715, 1717, 1777, 1801, 2013, 2029, 2096 | User-facing: `roko prd plan` progress, retries and model escalation |
| crates/roko-cli/src/prd.rs | 2210-2212 | User-facing: raw model output shown before the command fails |
| crates/roko-cli/src/prd.rs | 2237, 2242, 2258, 2273, 2285 | User-facing: dry-run and artifact validation results for the generated plan |
| crates/roko-cli/src/prd.rs | 2303 | Diagnostic, deferred: duplicates the `tracing::info!` phase timing just above it; delete it once bug-2d06bf (wk-taskdef), which changes `generate_plan`, has merged |
| crates/roko-cli/src/prd.rs | 3405 | User-facing: PRD validation issues |
| crates/roko-cli/src/runner/output_sink.rs | all 13 | User-facing: the stderr sinks' plan summary, agent lines and run-complete report |
| crates/roko-cli/src/auth_detect.rs | all 13 | User-facing: `print_setup_instructions`, shown when no provider is configured |
| crates/roko-cli/src/unified.rs | all 11 | User-facing: banner, tips, tool summaries, usage footer, and errors before exit 1 |
| crates/roko-cli/src/chat.rs | all 8 | User-facing: which backend chat connected to, and what to start when it is unreachable |
| crates/roko-cli/src/agent_exec.rs | all 6 | User-facing: agent progress ticker and completion lines |
| crates/roko-cli/src/prd/dry_run_fs.rs | all 5 | User-facing: dry-run plan validation issues and quality warnings |
| crates/roko-cli/src/graph_execution/plan_runner.rs | 45, 55, 59, 63 | User-facing: per-node progress, only with `show_progress` |
| crates/roko-cli/src/graph_execution/plan_runner.rs | 401 | User-facing: the forced-exit message and resume hint, printed beside its `tracing` record |
| crates/roko-cli/src/runner/preflight.rs | 245, 266 | User-facing: preflight check results |
| crates/roko-cli/src/serve_client.rs | 798, 836 | User-facing: "run cancelled" on Ctrl-C |
| crates/roko-cli/src/run.rs | 519 | User-facing: `roko run` says the change will end unverified (skipped with `--quiet`) |
| crates/roko-cli/src/model_selection.rs | 88 | User-facing: printed only with `ROKO_VERBOSE=1`, and also logged with `tracing::debug!` |
| crates/roko-cli/src/hints.rs | 38 | User-facing: renamed-command hint |
| crates/roko-cli/src/chat_inline/event_loop.rs | 178 | User-facing: hint when stdin is not a TTY |
| crates/roko-cli/src/custody.rs | 81 | User-facing: "No custody records found" |
| crates/roko-demo/src/main.rs | 196 | User-facing: the demo benchmark's result line |
| crates/roko-cli/src/cli_reporter.rs | 6 | Not a call: a doc comment that mentions `eprintln!` |
| crates/roko-demo/src/deploy.rs | 478 | Test code: a skip message inside `mod tests` |
| crates/roko-agent/src/openai_compat_backend.rs | 1682 | Test code: a skip message in a live-API `#[tokio::test]` |

## Notes

- 2026-10-01 (wk-filer4): implemented the first step on work/gap-cd51b7; cargo verification deferred to the batch check. The item stays open, and its verify still fails (156 sites against at most 60, and no `print_stderr` lint).
- 2026-10-01 (wk-filer4): What changed: 22 diagnostic sites now use tracing. In roko-learn, runtime_feedback/mod.rs has 5 save and transaction failures as `warn` and the experiment-concluded record as `info`, and bandits.rs logs an unknown arm as `warn`. In roko-core, env_registry.rs `warn_if_deprecated` uses `warn`; it has no caller. In roko-cli, prd.rs `validate_and_fix_generated_plan` logs its 14 automatic plan corrections as `warn`, or `info` for the model_hint removal, the auto-added verify and placeholder replacement. Every other site is user-facing output, skipped, deferred or test code (table above).
- 2026-10-01 (wk-filer4): Next: after gap-0d0e81 splits main.rs, classify main.rs and commands/. A count of at most 60 looks out of reach: 95 of the 99 sites outside main.rs and commands/ are user-facing output, which should stay on stderr until it moves to `CliReporter` (cli_reporter.rs). A better verify: `print_stderr = "warn"` in [workspace.lints.clippy], with `#[allow(clippy::print_stderr)]` on each module that prints user-facing output, so new diagnostics cannot use eprintln!. gap-556ffe (parked) is still the duplicate the notes above describe.
