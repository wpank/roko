+++
id = "gap-5a6e01"
kind = "gap"
title = "Turn caps and timeouts set from each tier's p95 over successful tasks"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-learn/tier_limits", "roko-cli/graph_task_dispatch", "roko-core/config"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "8a88c6267"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e10"
discovered_from = "tmp/cybernetic-harness/evidence/field/CASES.md (CASE-004: turn caps and timeouts set by guesswork); tldr/05 P1 #14"
anchors = ["crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::task_turn_limit", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::base_attempt_timeout_ms", "crates/roko-core/src/config/gates.rs::PipelineConfig", "crates/roko-learn/src/tier_limits.rs"]
lane = "rust-hot"
parent = "spec-edda86"
links = { depends_on = ["gap-96f7ed"], blocks = [], related = ["gap-a791b4", "find-43768e", "gap-8c0a20"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tier_limits_follow_p95_of_passed_attempts' crates/roko-learn/src/ && cargo test -p roko-learn --lib tier_limits_follow_p95_of_passed_attempts"

[[verify]]
command = "grep -rqw 'fn learned_tier_limits_reach_turn_cap_and_timeout' crates/roko-cli/src/ && cargo test -p roko-cli --lib learned_tier_limits_reach_turn_cap_and_timeout"
+++

## Problem

Turn caps (`[pipeline.<tier>] max_turns`) and the attempt timeout (a task's `timeout_secs`, else
`timeouts.agent_dispatch_secs`, 600 s) are set by hand. CASE-004 (`evidence/field/CASES.md`) records the cost:

- on 09-25 turns were unbounded, and one task ran 312 s and read 304,769 cached tokens;
- on 09-28 caps of 10/10/20/30 made 3 of plan 05's first 8 attempts exit, and were raised by hand to 40/60/90/120;
- the 600 s timeout killed 01-T10, and killed 08f-T05 twice; operators raised timeouts by hand.

## Why it matters

A cap that is too low fails good attempts; one that is too high lets a looping cheap model burn money. Each tier has
its own natural length, and the ladder (epic spec-98f76d) changes it by changing the models. CASE-004 names the fix:
caps and timeouts from the p95 of successful tasks in each tier. This is step 4 of epic spec-edda86.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`: `task_turn_limit` (:712) and `base_attempt_timeout_ms` (:760).
- `crates/roko-core/src/config/gates.rs::PipelineConfig`: the mode switch.
- **New file:** `crates/roko-learn/src/tier_limits.rs`, the pure computation.
- Input: gap-96f7ed's settled attempt records (`.roko/runs/<run_id>/attempts.jsonl`, with the verdict, the turns used
  and the timings; S01 §5.5).

## Current state

Checked at `41c7ffbd6`. Limits are static per tier; only retries adapt. Since `d4be4e872` a timed-out attempt is
retried with 1.5× the timeout (at most 4× the base), and a turn-cap stop with a raised cap. `efficiency.jsonl` rows
carry turns and wall time but not the tier.

## Plan

1. `tier_limits::suggest(records, tier)`: over the tier's last 100 passed attempts (unverified ones excluded), the p95
   of turns used and of agent wall time, times 1.25, rounded up. With fewer than 20 passed attempts, `None`.
2. Guard against a downward ratchet: passed attempts bias the p95 low, since attempts that needed more turns failed at
   the cap. Keep each suggestion within [0.5×, 2×] of the configured value, and never lower a cap that more than 10%
   of the tier's recent attempts reached.
3. `[pipeline] learned_limits = "off" | "shadow" | "on"`, default `shadow`. Shadow logs each tier's configured and
   suggested limits at plan start; `on` makes `task_turn_limit` and `base_attempt_timeout_ms` use the suggestion. An
   authored `timeout_secs` always wins, and retry escalation still applies on top.
4. If the attempt records lack the tier, add it to the attempt-open record.
5. Tests: `tier_limits_follow_p95_of_passed_attempts` (roko-learn: p95, minimum sample, bounds, the cap rule) and
   `learned_tier_limits_reach_turn_cap_and_timeout` (roko-cli).

## Done when

- [ ] With `learned_limits = "on"` and enough history, a tier's cap and timeout follow its p95, within the bounds.
- [ ] With `shadow`, only the log changes.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Waits for gap-96f7ed (E4.2): only the attempt records join turns, time, tier and verdict per attempt. Uses
  gap-8c0a20's `TaskTier` if it has landed.
- Turn caps bind non-Claude providers only once gap-a791b4 lands; timeouts bind every provider.
- Shadow first, as D8 (DECISIONS.md) sets for controllers.
- **Hot file:** `graph_task_dispatch.rs` (two small functions).
- 2026-09-30 (wk-tiers): implemented on `work/gap-1d1fa6` at `a9661848c`; cargo verification deferred to the batch check (no cargo was allowed for this item).
- Not wired yet, so the first Done-when box holds only at the function level: dispatch builds no `LearnedTierLimits`. The calls to `task_turn_limit` and `base_attempt_timeout_ms` are in `graph_task_dispatch.rs` and `streaming.rs`, and `attempt.rs` does not fill `AttemptOpenRecord.tier`; batch 12 owns all three. Until then, tiers come from `learn/costs.jsonl` by attempt key, and config doctor lists `pipeline.learned_limits` as inert. The wiring is planned with gap-460230, after batch 12 merges.
- 2026-09-30 (wk-tiers): wired on `work/gap-460230` at `483190bc7`: batch and streaming dispatch read `LearnedTierLimits` once per dispatcher from `feedback.runs_dir` and `costs_path`, attempt-open lines carry the tier, and `pipeline.learned_limits` is no longer listed as inert. `dispatch_reads_learned_tier_limits_from_its_runs` covers the wiring. Both `[[verify]]` tests pass in the worktree; workspace verification is deferred to the batch check.
