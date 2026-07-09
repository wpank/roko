# F — Theory And Learning Frontier

Refresh of parity for the theory-heavy conductor chapters.

Generated: 2026-04-18

---

## Required Reframe

The audit outcome for this file is simple:

- OODA, Good Regulator, and Yerkes-Dodson are useful framing,
- they are not current implementation specs,
- and conductor-learning federation belongs in Phase 2+.

This file exists to keep that line sharp.

---

## Informational, Not Spec

### OODA

OODA is a valid explanation for the current `Conductor::evaluate()` loop, but
it is still framing. The implementation contract lives in:

- `Conductor::evaluate()` at `crates/roko-conductor/src/conductor.rs:156-186`
- `InterventionPolicy` at `crates/roko-conductor/src/interventions.rs:99-121`

Do not turn OODA language into a requirement for additional types or control
layers that are not in code.

### Good Regulator

The self-model material is not the current engineering contract. Keep it as
informational background until the named surfaces actually exist.

Examples that should stay marked planned/informational:

- Brier-style self-model scoring
- Kalman or forward-prediction machinery
- threshold-learning or posterior-update systems

### Yerkes-Dodson

The pressure chapter should be treated as theory and future tuning guidance,
not as a shipped subsystem.

That includes:

- pressure dial / pressure envelope
- flow detection
- model pressure profiles
- pressure-performance curve fitting

---

## What Is Actually Live On The Learning Side

### `ConductorBandit`

`ConductorBandit` is real and wired, but only in the retry path.

Anchors:

- definition: `crates/roko-learn/src/conductor.rs:108-223`
- load/save: `crates/roko-learn/src/conductor.rs:143-204`
- orchestrator integration: `crates/roko-cli/src/orchestrate.rs:3797-3799`,
  `7182-7183`, `7294-7295`, `7311-7331`, `8058-8060`

Parity wording should therefore say:

- retry-path bandit is shipped,
- learned conductor as the default `InterventionPolicy` is not.

### Conductor-learning federation

Federated conductor layers, self-healing conductor, and triple-loop learning
remain Phase 2+ material. Keep them in the docs, but clearly as future work.

---

## Carry-Forward Wording

Use language like:

- "informational framing"
- "planned extension"
- "Phase 2+"
- "retry-path only"

Avoid language like:

- "implemented" for pressure/federation/self-healing chapters
- "current spec" for theory sections
- "wired" for a learned-policy replacement that does not exist yet
