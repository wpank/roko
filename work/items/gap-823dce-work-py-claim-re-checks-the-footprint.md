+++
id = "gap-823dce"
kind = "gap"
title = "work.py claim re-checks the footprint under a lock, can be renewed, and expires by size"
status = "done"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "S"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "166e5841f"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e14"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W1-work-graph-audit.md (R3); W7-orchestration-model.md (B4)"
anchors = ["tools/work.py::cmd_claim", "tools/work.py::load_claims", "tools/work.py::prune_claims", "tools/work.py::cmd_claims", "tools/test_work.py"]
lane = "tracker"
parent = "spec-1e1b45"
links = { depends_on = [], blocks = [], related = ["gap-d1f787"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_claim_refuses_an_overlapping_footprint' tools/test_work.py && grep -qw 'def test_claim_renew_extends_the_ttl_by_size' tools/test_work.py && python3 tools/test_work.py -k test_claim_refuses_an_overlapping_footprint -k test_claim_renew_extends_the_ttl_by_size"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T08:46:11Z"
commit = "166e5841f"
by = "wk-gates"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-01T08:36:20Z"
model = "claude-opus-5-5"
forced = false
evidence = "Premise re-checked at bdeaff586: claim only checked the id (O_EXCL), claims expired after a fixed 24 h with no renewal, and next and claims called prune_claims. 166e5841f: claim holds an flock on .roko/work-claims/.lock, reloads live claims and refuses an overlapping footprint (same branch excepted), a file another worktree is changing (own branch's worktree excepted) or an open dependency not claimed on the same branch; --force overrides and prints what it overrode; release, close and sync's pruning take the lock. TTL by size from the last renewal (S 8 h, M 24 h, L 72 h, unsized 24 h); 'claim <id> --renew --by <who>' (claimant only) sets renewed_at and logs a claim event with renew=true. next and claims are read-only; 'claims --prune' drops closed items' claims; next ignores claims on closed items. README claim text updated. tools/test_work.py: overlap refused, same-branch allowed, --force reports; open dependency refused unless claimed together; a renewed M claim made 30 h ago is live, an unrenewed S claim is stale after 8 h, only the claimant renews; next and claims delete nothing; the [[verify]] command passes."
+++

## Problem

`cmd_claim` checks only that the item id is unclaimed (an `O_EXCL` file). It never re-checks the footprint, so two
sessions that ran `next` a moment apart can claim two items anchored on the same file. Claims expire after a fixed
24 hours (`CLAIM_TTL_HOURS`) and cannot be renewed: an L item loses its claim mid-work, while an abandoned S item
blocks its files for a day. And `next` and `claims`, which look read-only, call `prune_claims()` and delete claim
files.

## Why it matters

Goal `tooling`, epic spec-1e1b45. With 6–10 agents and several sessions sharing one claims directory, `next` and
`claim` race (W1 R3, W7 B4), and a claim that silently expires frees its files to another worker.

## Where

`tools/work.py`: `cmd_claim`, `load_claims` (TTL), `prune_claims`, `cmd_claims` and `cmd_next`. Tests in
`tools/test_work.py`.

## Current state

Checked at `41c7ffbd6`: all three problems are present. Claims live in the main checkout's `.roko/work-claims/`,
shared by every worktree.

## Plan

1. Hold `fcntl.flock` on `.roko/work-claims/.lock` for the whole claim: load live claims, compute the busy footprint
   (with gap-d1f787's worktree scan once it lands), refuse an item whose footprint overlaps or whose `depends_on` are
   open, then write. `--force` still overrides and prints what it overrode.
2. Store `size` in the claim. TTL: S 8 h, M 24 h, L 72 h, counted from `renewed_at` (default `at`).
3. `claim <id> --renew --by <who>` refreshes `renewed_at`; only the claimant may renew.
4. Move pruning to an explicit `claims --prune`; `next` and `claims` become read-only.
5. `close` and `release` take the same lock.

## Done when

- [ ] A claim on an item whose anchors overlap a live claim is refused.
- [ ] A renewed M claim is still live 30 h after it was made; an unrenewed S claim is stale after 8 h.
- [ ] `next` and `claims` delete nothing.
- [ ] The `[[verify]]` command passes.

## Notes

- Claim files written before the change must still load.
- Update the README's claim text ("Claims older than 24 hours count as stale") in the same commit.
- W7 suggests renewing from the commit hook; leave that for later.
