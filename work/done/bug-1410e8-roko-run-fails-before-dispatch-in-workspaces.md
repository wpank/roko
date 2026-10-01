+++
id = "bug-1410e8"
kind = "bug"
title = "`roko run` fails before dispatch in workspaces with no Cargo.toml or go.mod and no configured gate"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/run"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "aec267cac"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3d-roko-run"
anchors = ["crates/roko-cli/src/run.rs::prompt_verify_steps", "crates/roko-cli/src/run.rs:404"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -q "fn prompt_plan_without_build_manifest" crates/roko-cli/src/run.rs && cargo test -p roko-cli --lib prompt_plan_without_build_manifest'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T13:44:21Z"
by = "coordinator (session 7622b882)"
claimed_at = "2026-10-01T09:07:57Z"
forced = false
evidence = "Batch 20e gate on 1a8aad603, re-checked with the compile fixes (cf722c1be, bed29287d), tiers' rustfmt (8a6c932ce) and the run-index scrub fix (d972959bd) on 32fe02384; MAIN aec267cac has the same crates: check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/fs/gateway/graph/learn/neuro/serve/std; lib tests roko-cli 3305, roko-agent 2294, roko-core 1962, roko-learn 1213, roko-serve 991, roko-graph 478, roko-fs 260, roko-neuro 239, roko-std 229, roko-acp 199, roko-dreams 100, roko-gateway 42 all pass; extras: golden_path_suite 2/2, all eight canaries pass (secret_canary 11/11 and C2 2/2 after the scrub fix), worktree_task_diff 2/2, plan_run_config_flag 1/1, default_engine 1, bin 429, routing crash loop 10/10, bench driver 18, including prompt_plan_without_build_manifest. Merged d7cbcfc1f."
+++

`roko run "<prompt>"` builds a one-task implementer plan and executes it with `run_graph_plan`. Its verify steps come from `prompt_verify_steps`: the configured required gate rungs; else `cargo check --workspace` when `Cargo.toml` exists; else `go build ./...` when `go.mod` exists; else nothing (run.rs:527-532). Implementer tasks must have a verify step, so in any other workspace (JavaScript, Python or docs, without a configured gate) the plan is rejected before an agent is dispatched (reported by w3d).

Fix: fall back to a language-neutral check, or allow verify-less prompt runs with the result marked unverified. Add a test named `prompt_plan_without_build_manifest`.

## Notes

- 2026-10-01 (wk-childenv): Premise re-checked at `32938ad4b`: `run_prompt` bailed ("no gate can verify this
  change") before dispatch when `prompt_verify_steps` found no rung, Cargo.toml or go.mod, and the plan loader's
  schema check (`TasksFile::validate_against_schema`) rejects an implementer task with no verify step.
  Implemented on `work/bug-4ed3c2`; cargo verification deferred to the batch check.
  - No language-neutral check verifies an arbitrary change honestly, so the run goes ahead without one and its
    task ends unverified, which is not a success (gap-29a84b). `roko run` prints a note naming the
    `[[gates.rungs]]` fix, and the workflow outcome reads "unverified: no gate could verify the change".
  - New `[meta] allow_unverified` (default false, written only when true) lets a plan's implementer tasks have no
    verify step; `roko run` sets it only when it found none. Authored plans keep the requirement.
  - Test: `run.rs::prompt_plan_without_build_manifest` runs `run_prompt` in a docs-only workspace with a fake
    provider: the agent edits the README, the verdict record says `unverified`, and the report is not a success.
