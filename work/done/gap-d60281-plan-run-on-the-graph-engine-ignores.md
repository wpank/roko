+++
id = "gap-d60281"
kind = "gap"
title = "plan run on the Graph engine ignores --resume, --effort, --skip-preflight, --force and screenshot flags"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/plan-run"]
created = 2026-09-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "gaps-md#from-engine-audit-tmpengine-auditsummarymd/engine-graph-drops-flags"
anchors = ["crates/roko-cli/src/commands/plan.rs::warn_graph_unsupported_flags"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'will be ignored' crates/roko-cli/src/commands/plan.rs && cargo test -p roko-cli --bin roko graph_plan_run_rejects_or_honours_legacy_flags"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:46:01Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:51Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Already fixed at BASE ebdc0f5d5; the item's 2026-10-01 note gives the evidence."
+++

The 2026-09-01 audit found that `--engine graph` silently dropped 8 flags. They now log a warning but are still ignored (`crates/roko-cli/src/commands/plan.rs:2398-2429`): `--resume`, `--effort`, `--skip-preflight`, `--force`, `--screenshots` and `--screenshot-interval`. Graph is now the only engine, so these flags do nothing on any `plan run`.

Fix: implement each flag on the Graph path (`--resume` maps onto Graph checkpoints), or remove it from the CLI and return a deprecation error.

Rechecked 2026-09-29 at d9e79e9d8. The warnings moved to warn_graph_unsupported_flags (commands/plan.rs:2439-2509). The ignored set is eight flags; the body lists six, and --screenshot-dir and --batch-size are also ignored. Graph checkpoint resume already exists as --resume-plan (main.rs:2036-2040), so --resume <session> can be rejected with a pointer to --resume-plan instead of being reimplemented.

## Notes

- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- At BASE `--force` (the disk-space pre-check) and `--log-file` were already implemented. `plan run` now stops before any
  work on `--resume <session>`, `--effort`, `--skip-preflight`, `--screenshots`, `--screenshot-interval`,
  `--screenshot-dir` and `--batch-size`, with an error naming what to use instead (`--resume-plan` or `roko resume`,
  `[agent] default_effort`, `roko screenshot`). The flags still parse, so main.rs is unchanged; its help text for them
  is left for the main.rs split. `./dev.sh fast --screenshots` no longer passes `--screenshots` to `plan run` (it only
  asks the evidence wrapper to import screenshots), and docs/v3/28-CLI.md lists the rejected flags.
