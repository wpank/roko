# D — Distillation And Tier Progression

Refresh of doc `12` against current code.

## What Ships

- `Distiller` is real in `crates/roko-neuro/src/distiller.rs:29-94`.
- the default distillation model is Claude Haiku.
- `TierProgression` is real in
  `crates/roko-neuro/src/tier_progression.rs:165-258`.
- the current pipeline already covers:
  - D1-style extraction into knowledge candidates
  - D2-style heuristic promotion thresholds
  - D3-style playbook compilation helpers
- tier feedback is also wired back into runtime code through
  `TierProgressionDecision`.

## What The Docs Should Stop Claiming

- doc `12` should not read as if the whole distillation roadmap is missing
- it should not present scheduler, quality-report, or HDC-clustering concepts as
  current runtime
- it should not imply that all promotion guards from the design doc already
  exist in code

## What Is Still Deferred

- `DistillationScheduler`
- `DistillationQualityReport`
- HDC-native D2 clustering as the default progression path
- larger cross-validation and anti-knowledge promotion gates beyond the current
  shipped thresholds

## Near-Term Framing

The audit takeaway is narrow:

- distillation is already wired
- tier progression is already wired
- the parity docs should focus on describing the real runtime and explicitly
  moving the research-heavy extensions to future work
