# Agent Runbook — 09 Daimon

Batch `09` is an audit pass over a mostly shipping subsystem.

The aim is to leave later agents with docs they can trust without forcing them
to re-derive the runtime shape from source.

## Default posture

- prefer doc calibration over new daimon code
- trust `crates/roko-core/src/affect.rs` and `crates/roko-daimon/src/lib.rs`
- use `roko-golem` only as historical provenance for stale claims
- treat `docs/09-daimon/11-coding-agent-integration.md` and
  `docs/09-daimon/12-collective-emotional-contagion.md` as frontier docs unless
  code proves otherwise
- keep future work explicit and narrow

## What good work looks like

- replace "Phase 2+" framing with "shipping core plus explicit frontier edges"
- remove stale `roko-golem` runtime claims and octant-as-contract language
- correct `EmotionalTag` references and separate derived
  `discovery_emotion` provenance from the stored tag
- refresh repo-map counts and verification searches before handoff
- leave clear deferrals for Doc 11, Doc 12, and deeper memory / strategy work

## What to avoid

- do not implement new Daimon, Neuro, Compose, or routing features
- do not widen the batch into cross-crate refactors
- do not present speculative surfaces as live because adjacent plumbing exists
- do not rewrite Doc 13 or the whole topic when a precise calibration will do

## Deliverable standard

Every batch should leave:

- changed docs with an explicit audit stance
- searches or commands that another agent can rerun
- explicit future seams
- a `PASS`, `FAIL`, or `BLOCKED` outcome
