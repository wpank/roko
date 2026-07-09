# Gap Inventory - 07 Conductor Docs Refresh

Concise gap list for the parity-refresh/docs-only scope.

## Focus Now

These are the gaps this refresh should actively close:

### 1. Status Honesty Drift

- update the context pack so it says the live conductor core is already
  wired,
- state clearly that 10 watchers, the plan-level breaker, and diagnosis
  call sites are in path today,
- remove the old "go implement the runtime" posture from the owned docs.

### 2. Theory And Federation Need Explicit Deferral

- mark the pressure, self-model, cognitive-signal, federation, and
  self-healing material as deferred,
- keep those topics visible as roadmap/design items,
- stop presenting them as active batch work for `07`.

### 3. Repo-Shape And Terminology Drift

- refresh the repo counts for `docs/07-conductor/` and
  `crates/roko-conductor/src/`,
- call out `RoutingBias` and live retry-path `ConductorBandit` status,
- treat older execution wording as historical context, not the current
  instruction set.

### 4. Docs-Only Scope Needs Hard Edges

- keep implementation items as carry-forward notes instead of open "do now"
  gaps,
- make the run script describe documentation batches and source-check
  verification,
- avoid implying cargo-test or Rust-edit work from this package alone.

## Carry Forward, Do Not Solve Here

Record these honestly, but do not turn them into work inside this refresh:

- `HealthMonitor` runtime activation
- `StuckDetector` / `MetaCognitionHook` runtime activation
- circuit-breaker restart persistence
- `ProcessSupervisor` ownership unification
- `PhaseTransition` / `adaptive_timeout_ms` production wiring

## Working Rule

If a task requires edits outside the owned docs files in `tmp/docs-parity/07/`,
the refresh should hand it off instead of expanding scope.
