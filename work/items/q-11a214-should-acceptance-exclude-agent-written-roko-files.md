+++
id = "q-11a214"
kind = "question"
title = "Should acceptance exclude agent-written .roko/ files unless HEAD already tracks them, like .cursor?"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/orchestrator"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-9 follow-up reports, filed 2026-10-04 (gap-7f32ed)"
discovered_from = "gap-7f32ed"
anchors = ["crates/roko-cli/src/orchestrator/worktree/acceptance.rs", "crates/roko-cli/src/orchestrator/worktree/git_ops.rs::ISOLATION_DIRS"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

Acceptance's commit step (`crates/roko-cli/src/orchestrator/worktree/acceptance.rs:244-257,593-608`) runs `git
add --all -- .` with exclusions built from `ISOLATION_DIRS` (`crates/roko-cli/src/orchestrator/worktree/git_ops.rs:956`,
currently `[".cursor"]`): for each dir in that list, it checks whether HEAD already tracks it
(`cat-file -e HEAD:<dir>`) and excludes it from the add **only when HEAD does not already track it**. This is
exactly the conditional rule team-lead's question proposes for `.roko/` — but `.roko` isn't in `ISOLATION_DIRS`,
so acceptance currently stages and commits anything an agent writes under `.roko/` (the implementer prompt
itself names `.roko/plans/` as a place an agent may write).

Roko's *own* state no longer leaks in by accident — `de1b9fb48` moved the efficiency row write to the workspace
root, and `accepted_worktree_attempt_commits_no_roko_state` (`attempt_workspace.rs`) tests that roko itself
doesn't write `.roko/` into an attempt's tree. This question is about the other direction: an *agent*
deliberately writing `.roko/` files (e.g. following the implementer prompt's own mention of `.roko/plans/`),
which acceptance happily commits today.

## Why it matters

Goal: release/core (commit hygiene, matching the precedent `.cursor` already sets). `.roko/` is roko's own
runtime/state directory; an agent-written file landing there and getting committed could pollute the plan
branch's history with something that was never meant to be tracked content, the same risk `.cursor`'s existing
rule already guards against for editor state.

## Where

- `crates/roko-cli/src/orchestrator/worktree/acceptance.rs` (both `git add --all` sites, lines ~244-257 and
  ~593-608).
- `crates/roko-cli/src/orchestrator/worktree/git_ops.rs::ISOLATION_DIRS` — **note**: this constant is shared with
  the worktree-setup code that *copies* `.cursor/` into each worktree (`git_ops.rs:956-983`, so each worktree
  gets its own MCP config) — adding `.roko` here would also make the worktree-copy logic try to copy `.roko/`
  into every worktree, which may not be wanted. A dedicated accept-time exclusion list (reusing the same
  conditional-tracking pattern, not literally the same constant) may be the cleaner fix.

## Why this needs Will

The mechanism to do this already exists and is proven (`.cursor` uses exactly this "exclude unless HEAD already
tracks it" rule) — the only real decision is scope: should `.roko/` get the identical treatment, and if so,
should it share `ISOLATION_DIRS` (simpler, but couples two different concerns) or get its own list (more code,
cleaner separation)?

## Notes

- Discovered during the work that closed gap-7f32ed.
- If Will decides yes, this becomes a regular `gap`/`bug` item with its own anchors and verify; the mechanism to
  reuse is already identified above.
