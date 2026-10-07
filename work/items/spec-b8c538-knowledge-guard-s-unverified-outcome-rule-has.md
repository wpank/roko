+++
id = "spec-b8c538"
kind = "spec"
title = "Knowledge guard's 'unverified outcome' rule has no definition in S06/S03"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["specs"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-13 follow-up reports 2026-10-04 (PK71 gap-099513, task 8137)"
discovered_from = "gap-099513 (closed; rule shipped with 8137, never cross-referenced to a spec)"
anchors = ["tmp/cybernetic-harness/specs/S06-ultrastable-controller.md"]
lane = "paper"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

The knowledge guard's "unverified outcome" rule is the implementing worker's own ad hoc
definition, not something either S06 (ultrastable controller) or S03 (loop liveness audit)
actually specifies. `KnowledgeChecks::invariants`
(`crates/roko-neuro/src/knowledge_store/commit.rs:207-238`) rejects a batch entry as coming
"from no verified outcome" when either: its `source` is `SourceChannel::AgentOutput` or
`SourceChannel::DreamConsolidation` (lines 217-219, 232-234), or its `source_episodes` is empty
(line 235). That is a reasonable rule, and it is documented in the function's own doc comment
(lines 206-208) — but nothing in S06 or S03 defines "verified outcome" this way, or references
this rule at all. The closest spec concept is `TaskGateVerdict::{Passed, Unverified,
ForcedAccept}` (S06 line 100-103, S03 lines 104, 241-242, 520), which is a *gate*-level verdict
on a task's execution, scored in E1/E2 — a different mechanism entirely from `SourceChannel`
and `source_episodes` on a *knowledge entry*. Both are "was this actually checked, or just
claimed," but S06/S03 only formalize the gate-verdict sense; the knowledge-entry sense exists
only as a code comment with no spec anchor.

## Why it matters

Goal: cybernetic, spec quality (S07) / M1 knowledge guard (8137). When S06 or a future spec
needs to state what makes a knowledge entry's outcome "verified" (e.g. to reason about the
guard's soundness, or to compare it with the gate's `Unverified` verdict), there is no canonical
definition to cite — only this one function's doc comment, which a future editor could change
silently without anyone noticing it drifted from what any spec assumed.

## Where

- `crates/roko-neuro/src/knowledge_store/commit.rs::KnowledgeChecks::invariants` (the rule).
- `crates/roko-neuro/src/lib.rs::SourceChannel` (`AgentOutput`, `DreamConsolidation` variants).
- `tmp/cybernetic-harness/specs/S06-ultrastable-controller.md` (M1/controller spec — the
  natural home for a knowledge-guard definition, since the guard is part of M1's commit path).
- `tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md` (defines the *other* "unverified",
  `TaskGateVerdict::Unverified` — worth cross-referencing so the two aren't conflated).

## Current state

The rule is implemented and tested (`commit.rs`'s own unit tests exercise it), and is
internally documented via a doc comment. It is not mentioned anywhere under
`tmp/cybernetic-harness/specs/`.

## Plan

1. Add a short definition to S06 (the M1/controller spec) stating exactly what the knowledge
   guard treats as an "unverified outcome" (source is `AgentOutput` or `DreamConsolidation`, or
   `source_episodes` is empty), citing `commit.rs::KnowledgeChecks::invariants` as the
   implementation.
2. Note in that same definition how it relates to, and differs from, S03/S06's
   `TaskGateVerdict::Unverified` (gate-level, scored in E1/E2) so a reader doesn't conflate the
   two "unverified"s.
3. If a later change to the rule's logic lands, the spec text and the code comment should be
   updated together.

## Done when

- S06 (or S03, whichever ends up the better home) states the knowledge guard's "unverified
  outcome" definition in terms a future spec or implementer can cite, distinct from
  `TaskGateVerdict::Unverified`.

## Notes

- 2026-10-04 (wave-13 follow-up, PK71 8137): confirmed at main HEAD `b7ad508ce`. This item has
  no `[[verify]]` command: the fix is a spec-text addition under `tmp/cybernetic-harness/specs/`
  (in scope; only `docs/whitepaper/*` and `tmp/cybernetic-harness/paper/*` are held for the
  paper-rewrite session), not a code change, so there is nothing to grep/test for. Filed as
  `kind = "spec"` rather than `gap` for that reason.
