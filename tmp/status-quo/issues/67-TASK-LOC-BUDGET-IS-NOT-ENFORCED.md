# Task LOC budget is not enforced

- Severity: high
- Area: task supervision / scope control
- Reproduced: 2026-07-11, task `SH01-T03`

`SH01-T03` declares `max_loc = 400`. Its live worktree reached 429 insertions and 94 deletions, or 523 changed lines, while the agent remained active. The runner emitted no budget warning, scope event, cancellation, or terminal failure.

The task budget currently behaves as prompt text rather than a runtime constraint. Measure the owned diff during execution and before gating, define whether the budget counts additions plus deletions or net additions, and reject or require explicit replanning when exceeded. The terminal event must record the measured value, configured limit, and disposition.
