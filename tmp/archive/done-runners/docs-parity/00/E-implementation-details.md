# E — Implementation Details (Docs 18-22)

Post-audit parity notes for `docs/00-architecture/18-decay-tier-matrix.md` through
`22-error-handling-recovery.md`.

This arc needed two hard corrections: the repo scale was understated, and speculative mechanisms
were written too close to present tense. The audit fix is to anchor the section in the actual
workspace size and then split shipped behavior from guidance-only and future-state mechanics.

---

## Baseline Facts

- 36 workspace members
- 322,088 Rust LOC
- `roko-learn`: 42 modules, 35,847 LOC
- audit baseline: 3,761 test functions

Use the exact `36 workspace members / 322,088 Rust LOC` phrasing when correcting stale scale
claims in this arc.

## Current Runtime Truth

| Doc | Status | Current truth |
|-----|--------|---------------|
| `18-decay-tier-matrix.md` | `rewrite` | current code uses decay plus knowledge tiers, but the polished unified matrix reads ahead of enforcement |
| `19-compositional-kinds.md` | `defer` | compound kinds are a plausible extension, not a present-tense runtime dependency |
| `20-configuration-schema.md` | `keep` | configuration is real and substantial |
| `21-performance-and-numerical-stability.md` | `narrow` | useful guidance exists, but broad guarantees are stronger than the code evidence |
| `22-error-handling-recovery.md` | `keep` + `narrow` | retry and recovery machinery are real, while the full graceful-degradation ladder is still more contract than proof |

## Deferred Concepts To Keep Explicit

- `Demurrage` remains a documentation concept with 0 production code.
- `Kind::Compound` remains planned.
- target-state unification language in docs `18-22` should stay labeled as guidance or future work.

## Rewrite Bias For Docs 18-22

Prefer:

- `current code uses decay plus knowledge tiers`
- `target-state demurrage model`
- `planned compound kind support`
- `guidance is ahead of enforcement`

Avoid:

- `demurrage is the current memory model`
- `compound kinds are already assumed by the runtime`
- `full numerical guarantees` unless code enforces them

## Batch-00 Boundary

For docs `18-22`, the parity refresh should:

1. correct the repo baseline to 36 workspace members and 322,088 Rust LOC,
2. keep live decay/config/recovery surfaces in present tense,
3. move demurrage and compound-kind language back into planned or deferred posture.
