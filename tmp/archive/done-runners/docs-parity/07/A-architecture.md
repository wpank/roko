# A — Architecture

Refresh of the conductor architecture parity brief after the audit.

Generated: 2026-04-18

---

## Bottom Line

The conductor architecture is already shipped. The main parity problem here was
not missing architecture; it was that the previous writeup treated section `07`
like a code-execution roadmap instead of a docs-truth pass.

The right posture is:

- keep the live conductor core in present tense,
- keep the `roko-conductor -> roko-learn` dependency visible as a real seam,
- defer larger architectural remedies to a later code pass.

---

## What Exists Today

### Composite policy

- `roko-core` still defines the live `Policy` trait at
  `crates/roko-core/src/traits.rs:166-171`.
- `Conductor` implements that trait and holds the watcher ensemble, the
  intervention policy, the circuit breaker, and `RoutingBias` at
  `crates/roko-conductor/src/conductor.rs:53-61`.

### Ten-watcher ensemble

- `crates/roko-conductor/src/watchers/mod.rs:8-28` declares and re-exports all
  10 watcher modules.
- `Conductor::new()` wires those 10 watchers at
  `crates/roko-conductor/src/conductor.rs:82-99`.

### Circuit breaker and decision path

- `Conductor::evaluate()` runs the breaker check, watcher pass, and policy
  evaluation at `crates/roko-conductor/src/conductor.rs:156-186`.
- The default intervention policy is `WorstSeverityPolicy` at
  `crates/roko-conductor/src/conductor.rs:96-100` and
  `crates/roko-conductor/src/interventions.rs:107-121`.

### Diagnosis and support modules

- `DiagnosisEngine` ships in `crates/roko-conductor/src/diagnosis.rs`.
- `HealthMonitor` ships in `crates/roko-conductor/src/health.rs`.
- `StuckDetector` ships in `crates/roko-conductor/src/stuck_detection.rs`.
- `PhaseTransition` ships in `crates/roko-conductor/src/state_machine.rs`.

Those should be described as implemented modules, not speculative design.

---

## Corrections This File Carries Forward

### `RoutingBias` is part of the real architecture

The old doc posture understated the conductor by omitting `RoutingBias`.

- `RoutingBias` is defined at `crates/roko-conductor/src/conductor.rs:27-35`.
- It is stored on `Conductor` at `crates/roko-conductor/src/conductor.rs:60-61`.
- It is derived from live watcher output at
  `crates/roko-conductor/src/conductor.rs:258-315`.

That is a real public surface, not a side note.

### Live signal names matter

The emitted kinds are:

- `conductor:alert:<watcher>` from
  `crates/roko-conductor/src/interventions.rs:123-144`
- `conductor.decision` from
  `crates/roko-conductor/src/conductor.rs:226-249`
- `conductor.circuit_breaker` from
  `crates/roko-cli/src/orchestrate.rs:4706-4708`

Older wording around a generic `conductor.intervention` surface should be
removed from parity materials.

### The layer violation is real

The conductor still depends directly on learning:

- `crates/roko-conductor/Cargo.toml:13-21` includes
  `roko-learn = { path = "../roko-learn" }`.
- The audit identifies this as the load-bearing architectural issue to keep,
  not to inflate into a full rewrite.

The modest follow-up remains the same:

- add a generic `Bus<E>` trait to `roko-core`,
- converge on shared event transport,
- break the direct conductor/learn dependency without inventing new nouns.

---

## What Not To Claim Here

Do not use the architecture file to imply that these are current runtime
contracts:

- typed `CognitiveSignal`
- federated conductor layers
- self-healing conductor
- Yerkes-Dodson pressure primitives
- Good Regulator calibration machinery

Those belong in informational or Phase 2+ sections, not the core
architecture summary.

---

## Carry-Forward

Keep the architecture story simple:

1. the conductor core exists,
2. the 10-watcher architecture exists,
3. the remaining architectural seam is the conductor/learn dependency,
4. the fix is small compared with the older overscoped rewrite proposals.
