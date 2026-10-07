+++
id = "gap-ad93ce"
kind = "gap"
title = "demo_seed's efficiency and episode rows carry no attempt_key, so telemetry check reports false coverage gaps"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-learn"]
created = 2026-10-04
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "19f76451c"
source = "wave-17 follow-up reports 2026-10-04 (gap-e411e5, gate 17a)"
discovered_from = "gap-e411e5 (closed; fixed the attempt ledger, not the efficiency/episode/cost rows)"
anchors = ["crates/roko-cli/src/demo_seed.rs::build_efficiency_events", "crates/roko-learn/src/telemetry/report.rs::LegacyRows"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn demo_workspace_telemetry_check_reports_no_coverage_gaps' crates/roko-cli/ && cargo test -p roko-cli demo_workspace_telemetry_check_reports_no_coverage_gaps"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T15:52:42Z"
commit = "19f76451c"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-05T09:03:18Z"
forced = false
evidence = "Gate 19 (merged 19f76451c): verify passes (demo_workspace_telemetry_check_reports_no_coverage_gaps). The demo seeder's efficiency, episode and cost rows carry the ledger's attempt keys."
+++

## Problem

`demo_seed` (`gap-e411e5`, closed at gate 17a) now feeds the real attempt ledger
(`runs/seed-run-20260429/attempts.jsonl`, with proper `AttemptKey::new(DEMO_RUN_ID, plan_id,
task_id, attempt)` keys — `crates/roko-cli/src/demo_seed.rs::build_attempt_ledger`, line 690),
but its efficiency and episode rows use an unrelated, seeder-local naming scheme, and it writes
no `costs.jsonl` rows at all. So `LegacyRows::load`
(`crates/roko-learn/src/telemetry/report.rs:218-245`) — which `roko learn telemetry check`
uses to confirm every real attempt has a matching legacy-log row — would find none of the
demo's attempts covered, and report every one as a coverage gap.

Specifically:
- `build_efficiency_events` (`demo_seed.rs:637-...`) builds `AgentEfficiencyEvent`s whose
  `attempt_id` is `"seed-{role}-{task_id}-{primary/followup}-{event_index}"` (line ~1088), not
  the real `AttemptKey`. It's written with `write_jsonl_if_absent(&learn_paths.efficiency_jsonl,
  &efficiency_events, ...)` (line 241) — the bare struct, with no top-level `attempt_key`
  field. Production efficiency rows get that field from a separate `AttemptKeyed<T>` wrapper
  (`crates/roko-learn/src/telemetry/records.rs:1947-1953`, `#[serde(flatten)]`) applied at the
  real write site (`graph_task_dispatch/feedback.rs`); demo_seed never uses it.
- `build_episodes` (`demo_seed.rs:552-...`) sets `episode.id`/`episode.agent_id` to
  `"seed-{role}-{task_id}"` and never inserts `attempt_key` into `episode.extra`, which is the
  nested field (`["extra", "attempt_key"]`) `LegacyRows::load` reads episodes by.
- `costs.jsonl` isn't written by the seeder at all — confirmed by listing every
  `write_jsonl_if_absent` call in `demo_seed.rs` (6 total: episodes, the attempt ledger,
  efficiency, `cfactor.jsonl`, knowledge seeds, `neuro/knowledge.jsonl`; no `costs.jsonl`
  target). So `LegacyRows.costs` isn't merely mismatched for the demo run, it's empty outright
  — the same visible symptom (a reported coverage gap), from "no row exists" rather than "a row
  exists with the wrong key."

## Why it matters

Severity p3 (demo-only). `roko learn telemetry check` is meant to catch *real* coverage
regressions in a live workspace. A freshly-seeded demo workspace reporting dozens of false
coverage gaps, from data that is deliberately synthetic, either trains an operator to ignore
the check's output on a demo workspace (masking a real gap later) or produces confusing noise
the first time someone runs the check right after `roko init`'s demo seeding.

## Where

- `crates/roko-cli/src/demo_seed.rs::build_efficiency_events`, `::build_episodes`,
  `::build_attempt_ledger` (the three generators to align).
- `crates/roko-learn/src/telemetry/records.rs::AttemptKeyed` (the wrapper efficiency rows need;
  episodes need `extra.attempt_key` set directly, matching how real episodes carry it).
- `crates/roko-learn/src/telemetry/report.rs::LegacyRows::load` (the reader; read-only
  reference, confirms exactly which fields/paths must match).

## Current state

The attempt ledger alone uses real `AttemptKey`s (`gap-e411e5`'s fix). Efficiency and episode
rows use an independent, seeder-local naming scheme with no `attempt_key` anywhere; costs rows
don't exist.

## Plan

1. Compute each seeded attempt's real `AttemptKey` once (the same one `build_attempt_ledger`
   already builds) and thread it into `build_efficiency_events` and `build_episodes`.
2. Wrap each efficiency event in `AttemptKeyed` (or otherwise add a top-level `attempt_key`
   field) before writing `learn/efficiency.jsonl`.
3. Set `episode.extra.insert("attempt_key", ...)` in `build_episodes`.
4. Decide whether to also add seeded `costs.jsonl` rows (keyed the same way) for a fully
   coverage-clean demo workspace, or accept that `costs.jsonl` coverage for the demo run stays
   absent as out of scope (document the choice either way).
5. Regression test: `roko learn telemetry check` against a freshly-seeded demo workspace
   reports no legacy-log coverage gaps for the seeded attempts.

## Done when

- `roko learn telemetry check` on a freshly-seeded demo workspace reports no false coverage
  gaps for the demo's own attempts.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-17 follow-up, gap-e411e5, gate 17a): confirmed at main HEAD `c796f09c1`.
  Costs.jsonl has no seeded rows at all (not rows with a mismatched key) — a more precise
  restatement of the source report's framing, with the same practical effect on
  `roko learn telemetry check`'s output.

## Progress

- gap-ad93ce: implemented at 0cbe9eb4f. The demo's rows carry the attempt keys the ledger uses (`demo_attempt_key`, `AttemptKey::new`). Efficiency rows go through `AttemptKeyed`: a task's primary turn is keyed to its first attempt, and its follow-up to its last one. For a task that passed, that is another turn of the same attempt. For a task that failed, it is the retry, which now records a failed gate as the ledger does, where before it claimed a recovery. Episodes name their attempt in `extra.attempt_key`. Each retry gets an episode of its own (7 episodes, not 5), and the seeded knowledge still cites only first attempts. Plan step 4's choice: the seeder writes one `learn/costs.jsonl` row per settled attempt, costed as the attempt's verdict is, and only beside the ledger, so a workspace with real runs gets no demo spend. The verify's grep passes; cargo verification is deferred to the batch gate (`demo_workspace_telemetry_check_reports_no_coverage_gaps`, which also asserts that the whole check passes).
