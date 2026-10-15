+++
id = "q-67bb42"
kind = "question"
title = "Seven keep/drop decisions on features from the 2026-09-21 stashes (budget defaults, per-task effort, prompt compression, ...)"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["workspace/hygiene"]
created = 2026-10-08
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "b883db4e6"
source = "stash-triage-2026-10-08 (roko-worktree-archive/2026-10-08-stash-triage/TRIAGE.md)"
discovered_from = "archive/stash-2026-09-21-main-1, archive/stash-2026-09-21-main-2"
anchors = ["crates/roko-core/src/config/budget.rs"]
links = { depends_on = [], blocks = [], related = ["gap-4665ac", "gap-d60281", "gap-73d760", "gap-ff95f5", "gap-9e09f6", "gap-61ea48"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-15
at_ts = "2026-10-15T16:30:15Z"
commit = "b883db4e6"
by = "status-quo-2026-10-15"
executor = "claude-session"
via = "manual"
forced = true
evidence = "2026-10-15 decision (Will delegated it to the coordinator's recommendation, tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md, decision 9; he can veto it): (1) Budget ceilings: keep 0.0 (unlimited) as the code default. No $50 or $200 default, and roko init does not ask. Re-scope gap-4665ac to warn once at plan start when max_plan_usd is 0, and replace its verify (today it requires non-zero defaults) with `grep -rqw 'fn plan_start_warns_without_a_plan_ceiling' crates/roko-cli/src && cargo test -p roko-cli plan_start_warns_without_a_plan_ceiling`. (2) Per-task max_turns/effort: not now. The [pipeline.<tier>] max_turns caps stay, TaskHints.reasoning_level does not drive provider effort, and plan run --effort stays rejected. (3) Compress prompt sections before dropping them: not now. gap-73d760 stays parked, because landing it needs a prompt-quality comparison with model calls (spend, Will's). (4) Safety denials as dashboard events: not as a separate change; take it up only inside gap-ff95f5 if that item is worked. (5) wasm_sha256: implement the check (refuse to load on a mismatch) in the change that brings WASM extension loading back, and hold gap-f36019 until then. Its verify passes vacuously today and must be replaced. (6) read_files budget: keep main's rule (the first 100 lines when no range is given, plus the per-task file count and snippet byte caps). No 32 KB or token budget. (7) Plans tab detail pane: not now. gap-9e09f6 and gap-61ea48 stay parked; reviving either first needs char-boundary-safe truncation. Why: (1) budget.rs documents that the earlier non-zero defaults (25/3) were removed because they silently capped runs nobody had configured (crates/roko-core/src/config/budget.rs:13-14), and max_task_retry_usd already defaults to $5 per task (budget.rs:39-45, :126). A warning protects the operator without stopping a run. (2) Nothing in the tracker shows tier caps failing tasks. (3) The quality check costs model calls. (4) gap-ff95f5 is open and owns denial provenance. (5) WASM extension loading fails closed today (crates/roko-cli/src/runner/extension_loader.rs:42-45), so the unchecked field can't be exploited yet; implementing the check with the loader keeps one change honest. gap-f36019's verify passes now only because crates/roko-plugin/src/dependency.rs:279 writes `wasm_sha256: None` in a struct literal. (6) crates/roko-compose/src/context_provider.rs:1650-1651 takes 100 lines; crates/roko-cli/src/plan_policy.rs:19,27,945 caps the file count (16, FAST 3) and the declared snippet bytes. (7) The stash code panics on non-ASCII text (item text). If vetoed: Each sub-answer can be reversed on its own. The likeliest veto is (1): a non-zero default, for example max_plan_usd = 50, makes gap-4665ac keep its current verify, and runs over $50 then stop. (5): dropping the field instead closes gap-f36019 as wontfix."
+++

## Problem

Two `git stash` entries made in the main checkout on 2026-09-21 (`archive/stash-2026-09-21-main-1` and
`archive/stash-2026-09-21-main-2`, pinned as branches) held uncommitted work that never reached main. A 2026-10-08
triage sorted their 63 features: 17 superseded, 32 obsolete, 8 worth landing (6 landed in batch 24, see below), and
6 that need a decision. This item asks for those decisions, plus one value the landing of a seventh needs.

The triage, with one patch per feature, is outside the repo at
`/Users/will/dev/nunchi/roko/roko-worktree-archive/2026-10-08-stash-triage/` (`TRIAGE.md`, `unsure-*.patch`,
`salvage-01-budget-default-ceilings.patch`).

## Decisions needed

1. **Default budget ceilings (gap-4665ac).** The stash sets non-zero defaults for `max_plan_usd` and
   `max_turn_usd`; its doc says 50, its code 200. Pick the defaults (or "require one at `roko init`"). A non-zero
   default stops runs that exceed it, and a turn cap also changes how much the scheduler runs in parallel.
2. **Per-task `max_turns` / `effort` overrides** (`unsure-02`). Should `TaskHints.reasoning_level` drive provider
   effort, and is a per-task turn cap wanted beside the `[pipeline.<tier>] max_turns` caps? Main rejects
   `plan run --effort` today (gap-d60281).
3. **Compress Normal/High prompt sections before dropping them** (`unsure-04`, parked gap-73d760). Applies cleanly
   but needs a prompt-quality check before it lands.
4. **Safety denials as dashboard events** (`unsure-01`). Decide together with gap-ff95f5 (denial provenance);
   `SafetyDenialCallback` has no caller until then.
5. **WASM `wasm_sha256` verification** (`unsure-05`): implement it (gap filed in batch 24) or drop the field.
6. **`read_files` context budget** (`unsure-03`): first 100 lines vs 32 KB per file; the triage suggests capping by
   token budget instead.
7. **Plans tab task-detail pane and duration timeline** (`unsure-06`, parked gap-9e09f6 / gap-61ea48). The stash
   code slices strings by byte offset and panics on non-ASCII text; it would need that fixed.

## Already landed from the stashes (batch 24)

`roko show --json`, `[runner] log_prompts`, the Agents view's real token split, the ProcessSupervisor watcher
handles (gap-7b065c), the $5 cost-log warning threshold (bug-d986b2), MCP `ping`, and the removal of crate-wide
clippy allows in six crates.

## Done when

- [ ] Will has answered 1–7; each answer is recorded here, and each "yes" becomes or revives an item.
