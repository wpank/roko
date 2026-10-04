+++
id = "gap-dadd56"
kind = "gap"
title = "L-route's dormant:mask registry finding is stale since 4113, not re-verified"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/loop-audit"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "e37dd2c5e"
source = "wave-6 follow-up reports 2026-10-03 (PK40 gap-1f4bec)"
discovered_from = "gap-1f4bec"
anchors = ["crates/roko-learn/src/loop_audit/loops.toml", "crates/roko-cli/src/dispatch/model_routing.rs::ModelRouter"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'verified_at = \"976220c3ebb91d3879e9d7701fa4aec5d9723d1f\"' crates/roko-learn/src/loop_audit/loops.toml"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T14:16:00Z"
commit = "e37dd2c5e"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T12:50:35Z"
forced = false
evidence = "Gate 18 (merged e37dd2c5e): its verify passes; census canaries pass. L-route's mask claim was false at 00a28ea0c (guards mask before the argmax; fallbacks are labelled), so it was rewritten and its dormant:mask reason dropped; all 20 findings pinned at 976220c3e were re-checked and re-pinned."
+++

## Problem

`crates/roko-learn/src/loop_audit/loops.toml`'s `L-route` registry entry carries a `dormant:mask` static finding
(lines 32-36) whose claim is: "The three `default_slug` fallbacks (unconfigured model, disabled provider, no tool
support) return `ModelChoiceSource::Router`, so a guard's pick carries the router's label (G55)," pinned at
`verified_at = "976220c3ebb91d3879e9d7701fa4aec5d9723d1f"`. Backlog task 4113 ("mask ineligible models before
cascade argmax") changed the masking behavior this claim describes, so the claim's premise no longer matches
current code — exactly the situation `Qualifier::DeclaredStale` / `declared_stale`
(`crates/roko-learn/src/loop_audit/spec.rs:290-296,332`) exists to catch (a finding pinned at a commit the harness
has since moved past). The loop census (see `gap-1f4bec`, done) already flags entries like this; nobody has gone
back and either updated the claim/`verified_at` or confirmed it's genuinely obsolete.

## Why it matters

A stale static finding in the registry misrepresents the current wiring of `L-route` (the cascade model router) to
anyone reading the loop census report, and blocks the registry from being a trustworthy, current record of what's
actually wired vs. dormant — the whole point of `tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md`'s
census. Related: task 2207 ("A guard's fallback is labelled fallback, not router," still open per an earlier
batch's research) would, once implemented, independently invalidate the same claim by making the three guards
return `Fallback` instead of `Router` — so this entry needs re-verification regardless of which change (4113's
masking or 2207's label fix) is what actually moved it.

## Where

`crates/roko-learn/src/loop_audit/loops.toml` lines 19-36 (`L-route`'s `static_findings`); the claim's own pointer,
`crates/roko-cli/src/dispatch/model_routing.rs::ModelRouter::route`.

## Current state

Unverified since `976220c3e`. Whether the claim still holds, holds partially, or is fully superseded has not been
re-checked against HEAD.

## Plan

1. Read `ModelRouter::route`'s three `default_slug` guard returns at current HEAD and determine whether they still
   return `ModelChoiceSource::Router`, now return something else (e.g. after 4113's masking change), or the
   scenario they describe can no longer occur at all (fully masked out before reaching that code).
2. Update the finding's `claim` and `verified_at` to match, or remove it if the masking it describes makes the
   scenario unreachable, with a note explaining why.
3. Re-run the census to confirm it no longer reports this entry as `declared_stale`.

## Done when

- `L-route`'s `dormant:mask` finding in `loops.toml` matches current code, with an updated `verified_at`.
- The `[[verify]]` command passes.

## Notes

- This is a registry-content fix, not a code-behavior fix — `[[verify]]` below is a static check on the TOML file
  itself plus the census tool, not a new Rust test.
- The experiment.seed sub-finding of this same PK40 report was already covered by a dated Notes line added to
  backlog task 4114 in an earlier batch today (2026-10-02, filer-w5); not repeated here.
