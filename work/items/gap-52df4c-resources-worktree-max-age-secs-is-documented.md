+++
id = "gap-52df4c"
kind = "gap"
title = "resources.worktree_max_age_secs is documented as worktree eviction, but nothing reads it"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-10-08
updated = 2026-10-08
last_verified = 2026-10-08
source = "stash-triage-2026-10-08 (roko-worktree-archive/2026-10-08-stash-triage/TRIAGE.md)"
discovered_from = "stash triage side finding"
anchors = ["crates/roko-core/src/config/schema.rs::ResourcesConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rn 'worktree_max_age_secs' crates/ --include='*.rs' | grep -v 'roko-core/src/config/schema.rs' | grep -q ."
+++

## Problem

`ResourcesConfig::worktree_max_age_secs` (`crates/roko-core/src/config/schema.rs`) is documented as the age after
which an idle worktree is evicted "at startup (orphan detection) and during periodic TTL-based eviction", default
86400, but no code reads it: setting it changes nothing.

## Why it matters

Worktrees pile up (285 were found on 2026-10-02, 147 more removed by hand on 2026-10-07). An operator who sets this
key expects eviction that never happens.

## Where

- `crates/roko-core/src/config/schema.rs::ResourcesConfig::worktree_max_age_secs`
- the orphan-worktree cleanup the doc names (post-plan cleanup and startup orphan detection in `roko-cli`).

## Current state

Found by the 2026-10-08 stash triage (`roko-worktree-archive/2026-10-08-stash-triage/TRIAGE.md`, side findings).
Nothing reads the field: `grep -rn worktree_max_age_secs crates/` hits only the schema.

## Plan

Either wire it into the orphan-worktree cleanup (evict clean, merged worktrees older than the age) or remove the key
and its documentation. Wiring it is preferable; removal needs a config migration note.

## Done when

- [ ] The key is read by the cleanup, or it is gone from the schema and docs.
- [ ] `[[verify]]` passes.
