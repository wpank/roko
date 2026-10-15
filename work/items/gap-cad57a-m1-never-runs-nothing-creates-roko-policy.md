+++
id = "gap-cad57a"
kind = "gap"
title = "M1 never runs: nothing creates .roko/policy/viability.toml, so ship a default policy and run M1 in shadow on a default plan run"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/runtime_feedback", "roko-learn/homeostasis", "roko-cli/init"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-11"
anchors = ["crates/roko-cli/src/runtime_feedback/homeostasis.rs::for_workdir", "crates/roko-learn/src/homeostasis/policy.rs::load_optional", "crates/roko-learn/src/homeostasis/policy.rs::ViabilityPolicy", "crates/roko-cli/src/commands/util.rs::cmd_init", "crates/roko-learn/src/homeostasis/streams.rs::calibrated_policy"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = ["gap-6bb6d8"], blocks = [], related = ["spec-c4fd9f", "gap-799698"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn default_plan_run_registers_m1_in_shadow' crates/roko-cli/ && cargo test -p roko-cli default_plan_run_registers_m1_in_shadow"

[[verify]]
command = "grep -rqw 'fn init_writes_the_default_viability_policy' crates/roko-cli/ && cargo test -p roko-cli init_writes_the_default_viability_policy"
+++

## Problem

M1 never runs. `[homeostasis] mode` defaults to `shadow` (`crates/roko-core/src/config/homeostasis.rs:24`) and every
non-frozen Graph run tries to build the sink (`graph_execution/plan_runner.rs::build_graph_feedback_context`), but
`HomeostasisSink::for_workdir` (`crates/roko-cli/src/runtime_feedback/homeostasis.rs:320-325`) returns `None` when
`.roko/policy/viability.toml` is missing, logging only at debug level. Nothing writes that file: not `roko init`, not
`roko setup`, not a template; only the test `crates/roko-cli/tests/homeostat_on_mode.rs:102` does.

So no controller state, θ commit or `harness_policy` row exists in this workspace, `~/.roko`, the worktrees or the nine
live/dogfood workspaces (A09 §4); with no sink there is no θ handle, no live retry budget and no slot resizing.

## Why it matters

Goal `cybernetic` (S06; decision 8102 meant "shadow on every run"). Will's decision 2 (2026-10-15): ship a default
policy so M1 runs in shadow on a default plan run. NEW-FINDINGS finding 11. The papers say the controller starts in
shadow (D5 9-30; D7 A.11, A-23). Stage 2 of the epic.

## Where

- `crates/roko-cli/src/runtime_feedback/homeostasis.rs::for_workdir`: the early return.
- `crates/roko-learn/src/homeostasis/policy.rs::ViabilityPolicy` (`load_optional`, `parse`; read-only by design).
- `crates/roko-cli/src/commands/util.rs::cmd_init`: what `roko init` writes.
- The S06 §5 example policy (`tmp/cybernetic-harness/specs/S06-ultrastable-controller.md:331-342`) and
  `crates/roko-learn/src/homeostasis/streams.rs::calibrated_policy` (the §4.2 bound rules).

## Current state

The controller, SafetyBox, holdout, ledger and routes are done and tested (A09: 35/37 tasks). Only the policy source
is missing.

## Plan

1. Add a built-in default policy (`ViabilityPolicy::shipped_default()` from a `DEFAULT_VIABILITY_POLICY` TOML text): the
   S06 §5 bounds (pass rate lo 0.70 / inner 0.75; $/verified success hi 0.12 / inner 0.108 / abs cap 0.50; false green
   hi 0.10; p90 latency hi 900 s / inner 810 s), drive weights 1, 1, 2, 0.5 (decision 8102), `verify.min_rungs =
   ["authored"]`, and no model names (derive tiers from the workspace's ladder, or leave B1's tier map empty if the
   parser allows).
2. `for_workdir`: with no file and `mode = shadow`, use the built-in policy and say so at info level, once per run.
   With `mode = on`, keep requiring a file a person wrote (warn and run in shadow without one).
3. `roko init` writes the same text to `.roko/policy/viability.toml`, with a header saying these are shipped defaults, M1
   only reads the file, and a person edits it. Never overwrite an existing file.
4. Tests `default_plan_run_registers_m1_in_shadow` (a fake-provider plan run with no policy file leaves
   `learn/homeostat-state.json` and harness decision rows, with every applied change absent: shadow keeps θ₀) and
   `init_writes_the_default_viability_policy`.

## Done when

Both `[[verify]]` commands pass, and `roko learn homeostasis status` on that workspace shows state instead of "no
controller state".

## Notes

- Shadow never changes dispatch (θ₀ unless `on`), so this is safe to default. `on` stays a human action (8102).
- Depends on gap-6bb6d8 so that M1's shadow log never credits a move on a knob nothing reads. E3 stays `unmeasured` until
  gap-618548 and audits (gap-d76b1b).
- spec-c4fd9f (S06 names `gates.max_rung` as B3's ceiling) touches the same spec text.
