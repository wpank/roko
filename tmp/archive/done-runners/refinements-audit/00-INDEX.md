# Refinements Audit

This folder is the pre-doc-edit audit pass for the refinement source set in
`tmp/refinements/` and the generated docs changes that landed from it.

Scope:
- Review the source refinements themselves.
- Compare them to the landed docs.
- Evaluate them primarily as target-state design references for what Roko
  should become.
- Use the current codebase as a sequencing and feasibility constraint, not as
  the standard by which future-state architecture is rejected.
- Separate good direction from overreach, jargon debt, sequencing mistakes,
  weak mechanism design, and ideas that should stay as research hypotheses.

Working rule:
- This folder is critique and triage first.
- It is not yet the rewritten canonical docs.

Files:
- `01-executive-summary.md`
  What is directionally right, what is overbuilt, and what should happen next.
- `02-foundation-learning.md`
  Foundation, kernel, learning, HDC, demurrage, heuristics, c-factor,
  research-to-runtime.
- `03-extensions-and-surfaces.md`
  Modularity, plugins, domain profiles, developer UX, user UX, StateHub,
  realtime, CLI, web UI, rich UX primitives.
- `04-safety-observability-roadmap.md`
  Safety spine, provenance, telemetry, glossary, synergy framing, roadmap and
  sequencing.
- `05-refinement-matrix.md`
  All 35 refinement items triaged one by one as `keep`, `narrow`, `defer`, or
  `rewrite`.
- `06-second-pass-additions.md`
  Net-new candidate refinements to add on top of the original 35.
- `07-naming-and-term-cuts.md`
  Better names, split/demote decisions, and terms that should not become canon.
- `08-simpler-target-architecture.md`
  Better mechanisms and a leaner redesign shape for the highest-risk areas.

Status labels:
- `keep`
  Strong direction. Mostly right. Continue with better wording or stronger
  evidence.
- `narrow`
  Good core idea, but too broad, too absolute, or too expensive to make
  canonical in the first redesign pass.
- `defer`
  Potentially useful later, but too speculative, too hard to validate, or too
  dependent on upstream wins to drive the near-term redesign.
- `rewrite`
  The current framing is materially misleading, internally inconsistent, or
  shaping the redesign in the wrong way even if the underlying instinct is
  valid.

Recurring audit theme:
- The refinements found several real gaps.
- As target-state design, many of them are directionally right.
- The next pass should preserve the strongest architectural moves while sharply
  reducing needless totalization, jargon inflation, and quarter-level
  optimism.
- The second pass should add missing seams and better names, not just narrow
  the existing set.
