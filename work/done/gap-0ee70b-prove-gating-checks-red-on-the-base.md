+++
id = "gap-0ee70b"
kind = "gap"
title = "Prove gating checks red on the base by default, except cargo checks (D14, Will 2026-10-02)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "509e3e807"
source = "backlog wave 1, PK17 report; Will's decision 2026-10-02"
anchors = ["crates/roko-core/src/config/spec_quality.rs", "crates/roko-cli/src/spec_gate.rs", "crates/roko-cli/src/commands/plan.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn red_on_base_runs_shell_checks_and_skips_cargo' crates/roko-cli/ && cargo test -p roko-cli red_on_base_runs_shell_checks_and_skips_cargo"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T18:37:12Z"
commit = "509e3e807"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-02T16:03:27Z"
forced = false
evidence = "Gate 3a on work/backlog-batch-3 (merged into main as 509e3e807; main differs from the gated tree only in work/ files): cargo check --workspace --tests, clippy -D warnings, nextest --lib 11,514 passed over 10 crates, golden-path canaries 13/13 (7 targets), roko-agent sse_replay 1/1; every [[verify]] passes (lib tests named in each verify passed; integration tests run by target; static parts rc=0)."
+++

## Problem

Decision D14 says every gating check is proven red on the base before dispatch. PK17 (backlog 3201–3213, merged in e53136640) added the spec gate with `[spec_quality] red_on_base`, defaulting to `false` (`crates/roko-core/src/config/spec_quality.rs`), and `roko plan validate` passes an empty red-on-base map (`commands/plan.rs`), so no check is ever run on the base and a vacuous (green-on-base) check reaches a paid agent.

## Why it matters

Planner-written checks are the gate cheap models are judged by (tldr design rule 3). Will decided on 2026-10-02: red-on-base on by default, except cargo checks, which are proven red at the batch gate instead.

## Where

`crates/roko-core/src/config/spec_quality.rs::SpecQualityConfig` (`red_on_base`), `crates/roko-cli/src/spec_gate.rs` (`RedOnBase`, `lint_files_with`), `crates/roko-cli/src/commands/plan.rs` (the empty map at validation).

## Current state

The map type and HF3 handling exist; nothing fills the map.

## Plan

1. Make `red_on_base` default to on, with a mode that skips checks whose command runs cargo (`cargo ` at a command boundary), recorded as `skipped: cargo`.
2. At `plan validate` and before `plan run`, run each remaining verify command on the plan's base tree (a scratch checkout or the base worktree), with the gate environment and a short timeout, and fill the map: a check that passes on the base is HF3.
3. Tests: `red_on_base_runs_shell_checks_and_skips_cargo` (a fixture plan with one vacuous shell check, one real shell check and one cargo check).
4. Amend D14's record (backlog 3201) to say cargo checks are proven at the gate.

## Done when

The verify passes; a fixture with a vacuous shell check is refused before dispatch.

## Notes

Decided by Will, 2026-10-02 (coordinator question round). Backlog context: tmp/backlog/2026-10-02-complete-and-wire (3201, 3211).

## Progress

- Implemented at e53f21502 on `work/gap-e4bfbf` (worker claude-agent, 2026-10-02); cargo verification deferred to the batch
  gate. `[spec_quality] red_on_base` defaults to true and `red_on_base_cargo` (false) decides whether cargo steps run;
  `spec_red_on_base::gate_results` fills the map before `plan run` (a check that cannot start blocks nothing).
  `plan validate --spec-quality --dynamic` follows the same cargo policy; plain `--spec-quality` stays static for speclint
  parity. Git-backed test workspaces opt out explicitly: `tests/common/mod.rs` (scripted workspace and
  `setup_sample_plan_workspace`) and `tests/plan_branch_integration.rs`. D14's record (backlog 3201) is amended.
