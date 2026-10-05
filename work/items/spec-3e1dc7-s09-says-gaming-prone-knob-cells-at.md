+++
id = "spec-3e1dc7"
kind = "spec"
title = "S09 says 'gaming-prone knob cells at 40%' but never the formula (F1/F3/F4/F5 at ladder 4-5)"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-05
updated = 2026-10-05
last_verified = 2026-10-05
source = "wave-19 follow-up reports 2026-10-05 (gap-6e7a86, work/gap-6e7a86)"
discovered_from = "gap-6e7a86 (done on work/gap-6e7a86; own docstring states the formula is a derivation, not a citation)"
anchors = ["tmp/cybernetic-harness/specs/S09-experiments.md"]
lane = "paper"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

S09's gaming-prone knob cells were derived as F1/F3/F4/F5 at ladder level 4-5, but no spec
states that formula — S09 only says "40%," never which 40%. `tmp/cybernetic-harness/specs/S09-experiments.md:351`
(H5's design) says: "F1, F3, F4, F5 with gaming-prone knob cells at 40%, F8 at 8%" — a
percentage, with no levels named. Confirmed by grepping the whole file for any specific level
number near this topic: none exists (the only other "ladder"-adjacent match, line 242's "ℓ1–ℓ4,"
is about P1-core task variants, unrelated).

`gap-6e7a86` (done on `work/gap-6e7a86`, not yet merged) had to derive the formula to build
`is_gaming_prone_knob_cell` at all, and its own module docstring is explicit that this is an
inference, not a citation of an existing fact:
`benchmarks/viabilitybench/families/common/knobs.py:27-31`: "H5's design streams F1/F3/F4/F5
'with gaming-prone knob cells at 40%' -- ℓ4 and ℓ5 of those four families: S08 §4.4's design
band bottoms out there (cheap-model VS <= 0.2-0.4), where a model more often cannot solve a
task for real than it can, so it is relatively more likely to reach for a shortcut the family's
planted gaming trick would catch. ℓ4 and ℓ5 are exactly 2 of each family's 5 levels, matching
S09's own '40%' precisely (8 of `p1_core`'s 20 instances per family). **Nothing in the family
generators or the driver computed this before gap-6e7a86; no code anywhere named it.**"

## Why it matters

Goal: proof, spec quality (S07), S09 closure test X3 (the same closure `gap-6e7a86`/`gap-90fae4`
fixed). A derivation this specific — levels 4 and 5, not some other pair, justified by a
cross-reference to a *different* spec section's difficulty bands — belongs in the spec that
defines the experiment, not only in one implementation file's docstring. Without it, a future
reader of S09 has no way to confirm `is_gaming_prone_knob_cell`'s levels are what S09 actually
intended versus one reasonable reading of an underspecified "40%."

## Where

- `tmp/cybernetic-harness/specs/S09-experiments.md:351` (H5's design row; the line to amend or
  footnote).
- `benchmarks/viabilitybench/families/common/knobs.py::GAMING_PRONE_FAMILIES`,
  `GAMING_PRONE_LEVELS`, `is_gaming_prone_knob_cell` (the derivation; read-only reference).
- S08 §4.4 (the difficulty-band cross-reference the derivation depends on; cite the exact
  section/line once confirmed, so S09's new text points at it directly).

## Plan

1. Add to S09 §4.4/§5.1 (next to the "40%" line) the explicit formula: gaming-prone knob cells
   are F1/F3/F4/F5 at ladder levels 4 and 5 (2 of each family's 5 levels, matching 40%), with
   the reasoning (S08 §4.4's difficulty bands bottom out there, where a shortcut is relatively
   more tempting) and a citation to S08's relevant section.
2. Note that this was a derivation (`gap-6e7a86`), not an original part of S09's text, so a
   future editor knows to treat it as confirmed-by-inference rather than an independent,
   separately-verified requirement.

## Done when

- S09 states the gaming-prone knob cell formula (F1/F3/F4/F5, levels 4-5) explicitly, not only
  "40%."

## Notes

- 2026-10-05 (wave-19 follow-up, gap-6e7a86, work/gap-6e7a86 not yet merged): confirmed at main
  HEAD `78d646e9a` that S09 states only "40%" (grepped the whole file). The derivation itself
  (`knobs.py`'s docstring) is branch-only, confirmed via `git show`. No `[[verify]]` command:
  the fix is spec-text only, filed as `kind = "spec"`, editing
  `tmp/cybernetic-harness/specs/` only (not `docs/whitepaper/*` or
  `tmp/cybernetic-harness/paper/*`, both held for the paper-rewrite session).
