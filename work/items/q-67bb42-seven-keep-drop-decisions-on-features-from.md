+++
id = "q-67bb42"
kind = "question"
title = "Seven keep/drop decisions on features from the 2026-09-21 stashes (budget defaults, per-task effort, prompt compression, ...)"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["workspace/hygiene"]
created = 2026-10-08
updated = 2026-10-08
last_verified = 2026-10-08
source = "stash-triage-2026-10-08 (roko-worktree-archive/2026-10-08-stash-triage/TRIAGE.md)"
discovered_from = "archive/stash-2026-09-21-main-1, archive/stash-2026-09-21-main-2"
anchors = ["crates/roko-core/src/config/budget.rs"]
links = { depends_on = [], blocks = [], related = ["gap-4665ac", "gap-d60281", "gap-73d760", "gap-ff95f5", "gap-9e09f6", "gap-61ea48"], supersedes = [], duplicate_of = "" }
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
