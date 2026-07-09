# Existing Parity Analysis Summary

Key findings from the prior `tmp/docs-parity/` audit (sections 00-12).
This is a digest — see the full analysis files for details.

## Section 00


## Section 01


## Section 02


## Section 03


## Section 04

## Highest-Value Corrections

### 1. The old pack understated shipped verification

- `A-D` should no longer read as if the verification runtime is mostly absent.
- The real task is documentation correction, not feature-program expansion.

### 2. Runtime path and canonical docs drift

- the live path is `orchestrate.rs -> rung_dispatch.rs`
- `rung_selector.rs` and `GatePipeline` remain real abstractions, but they are not the main production entrypoint today

### 3. Artifact/ratchet scope needs narrowing

- artifact store is real but in-memory
- ratchet is real but should not be described as a fully active persisted runtime guardrail

### 4. Thresholds are wired; advanced analytics are not

- EMA updates and persistence are real

## Section 05


## Section 06


## Section 07


## Section 08


## Section 09


## Section 10

## Highest-priority doc gaps

### 1. Doc 16 is materially stale

- it undercounts current runtime modules,
- it still carries `roko-golem` ownership history as if it were current,
- it mislabels several shipped Phase 3 surfaces as absent.

### 2. Runtime is ahead of docs on triggers and entry points

- scheduled trigger ships,
- manual trigger ships,
- CLI, daemon, and orchestrator entry points ship.

### 3. Runtime is ahead of docs on imagination and liminal systems

- Mattar-Daw-style replay scoring ships,
- REM counterfactual imagination ships,
- creativity modes ship,
- hypnagogia ships,

## Section 11


## Section 12

## Highest-Value Corrections

- doc status framing still undersells the shipped CLI/TUI/HTTP core
- `roko new` and standalone `roko explain` need explicit non-shipping language
- `9090` vs `6677` is a real inconsistency, not a wording preference

## Medium-Value Corrections

