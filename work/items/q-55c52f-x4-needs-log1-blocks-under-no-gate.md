+++
id = "q-55c52f"
kind = "question"
title = "X4 needs LOG1 blocks under no_gate/always_refine/always_escalate: add them, skip X4, or simulate?"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-15 follow-up reports 2026-10-04 (gap-1a8ee7, gate 16a)"
discovered_from = "gap-1a8ee7 (open, work/gap-1a8ee7; X4's own more vaguely stated data gap traced precisely)"
anchors = ["benchmarks/viabilitybench/experiments/log1.toml", "benchmarks/viabilitybench/analysis/replay_closure.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

X4 needs a LOG1 block run under the `no_gate`, `always_refine` and `always_escalate` policies,
and `experiments/log1.toml` defines none. The closure-replay module's own doc comment already
says so: "**X4** `usd_per_vs(gate_policy) vs {no_gate, always_refine, always_escalate}`...
Scoring the three counterfactual policies needs the gate's own refine/escalate decision model
to simulate what each would have done with a unit the real gate did not refine or escalate;
LOG1 has no block run under those policies directly (grepped `experiments/log1.toml`: its one
gate-off block, Block C, is H3's vague/precise/refined spec-wording design, not a spec-handling
policy sweep). So this adapter reports X4 not evaluated too."
(`benchmarks/viabilitybench/analysis/replay_closure.py:33-37`). Confirmed directly: grepping
`experiments/log1.toml` for `no_gate`, `always_refine`, `always_escalate` and `refine_spec`
returns nothing; Block C's four sub-blocks (`c-vague-oss`, `c-precise-oss`, `c-refined-oss`,
`c-vague-glm`) vary spec wording (vague/precise/refined), not the gate policy applied to it.

Adding the missing blocks is not a pure code fix: LOG1 is a billed, pre-registered experiment
plan (S09 §4.3; `requires_lock = true`, `requires_lock` referencing the pre-registration lock,
task 3345). Any new block changes LOG1's cell count and its planned spend
(`experiments/log1.toml`'s header: "billed blocks are streams/log1.toml's cells..., 2,156 in
all on budget line BL1, planned $151.80"), which is exactly the kind of change the
pre-registration lock exists to freeze before LOG1 runs for real money.

## Why it matters

Goal: proof, S09 closure test X4 (cross-loop closure: S07 → M3 → routing). Like X3, X4 can never
evaluate without this data regardless of code changes elsewhere — the gap is in the experiment
plan itself, not the analysis code. But unlike a code gap, deciding to add these blocks has
real, billed consequences (more cells, more planned spend) that only Will can authorize, and it
must happen *before* LOG1's pre-registration lock (`gap-394f28`) is taken, since the lock is
meant to freeze the plan LOG1 actually runs.

## Where

- `benchmarks/viabilitybench/experiments/log1.toml` (the plan; needs new blocks if Will
  approves).
- `benchmarks/viabilitybench/analysis/replay_closure.py:33-37` (the X4 adapter, already written
  and waiting for data).
- `gap-394f28` (the pre-registration lock; this decision must land before it's taken).

## Plan (decision needed)

- **Option A — add the three policy blocks to LOG1** (`no_gate`, `always_refine`,
  `always_escalate`, each over the same block-C-style tasks), accepting the added cells and
  spend this implies, and lock LOG1 only after they're in.
- **Option B — skip X4** (accept it stays "not evaluated" for this LOG1 run), and either
  pre-register a smaller, separate $0 or low-cost follow-up experiment for it later, outside
  LOG1's locked plan, or drop it from this round's closure tests entirely.
- **Option C — simulate the counterfactual policies analytically** from existing block C data
  (per the adapter's own framing, "needs the gate's own refine/escalate decision model to
  simulate what each would have done") instead of running new blocks at all — cheaper, but
  only as trustworthy as that simulation model, and the adapter's current design assumes real
  blocks, not a simulated counterfactual.

## Done when

Will picks an option before `gap-394f28`'s lock is taken; if A, `experiments/log1.toml` gets
the new blocks and the plan's billed totals are re-verified against BL1's cap before locking.

## Notes

- 2026-10-04 (wave-15 follow-up, gap-1a8ee7, gate 16a not yet merged): confirmed at main HEAD
  `7d82b944a`. Filed as `kind = "question"` per the instruction, since adding LOG1 blocks
  changes billed spend and needs Will's sign-off, not just a code change. A matching note has
  been added to `gap-394f28` flagging this as a pre-lock dependency.
