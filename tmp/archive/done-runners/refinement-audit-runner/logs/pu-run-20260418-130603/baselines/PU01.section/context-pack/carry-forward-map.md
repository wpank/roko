# Carry-Forward Map — Batch 01

Use this when a real finding appears during batch `01`, but the fix belongs somewhere else.

| Item | Better Home | Keep In 01 As | Why |
|------|-------------|---------------|-----|
| event-enum unification / generic bus work | foundation cleanup | boundary note | real issue, but not owned by orchestration runtime |
| domain-specific gate suites | `tmp/docs-parity/04` | routing note | verification owns gate semantics |
| domain-specialized agent behavior | `tmp/docs-parity/02` | handoff note | orchestration can route, but agents own behavior |
| adaptive routing / reward by domain | `tmp/docs-parity/05` | learning note | learning owns policy adaptation |
| conductor/learn boundary cleanup | later architecture/runtime cleanup | layering note | real issue, but too broad for one batch-01 runtime patch |
| local event-log vs shared bus mismatch | foundation cleanup | boundary note | important to describe accurately, but not a batch-01 runtime feature gap |
| formal stigmergy model | Phase 2+ roadmap | deferral note | doc `12` is mostly conceptual framing |
| cross-domain orchestration | Phase 2+ roadmap | deferral note | doc `13` is not current code-domain runtime |
| semantic merge strategies | later orchestration or VCS pass | queue note | broader than worktree/merge hygiene |
| saga coordinator / plan repair / templates | later orchestration roadmap | deferral note | too large for batch `01` |
| distributed recovery / CRDT / HLC | post-parity roadmap | roadmap note | not required for current self-hosting runtime |

When deferring, record:

1. the exact gap or file,
2. why it is outside batch `01`,
3. which later topic should own it,
4. and the smallest runtime contract batch `01` still needs to leave behind.

If the finding only changes wording or scope boundaries, keep it in this pack and do not promote it to a future code batch.
