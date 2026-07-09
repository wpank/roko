# C — Query, Context, And Cross-Domain Transfer

Refresh of docs `08` and `10` against current code.

## What Ships

- `NeuroStore` and `KnowledgeStore` are real in `roko-neuro`.
- the knowledge store already supports string-driven query paths and
  feature-gated HDC indexing inside neuro.
- `ContextAssembler` is real in `crates/roko-neuro/src/context.rs:221-287` and
  already contains ranking, compression, PAD biasing, and contrarian retrieval
  logic.

## What Is Still Missing

### `query_similar()` On `Substrate`

This remains **not yet implemented**.

`roko-core/src/traits.rs:34-63` defines `Substrate` with `put`, `get`, `query`,
and `prune`. There is no `query_similar()` today, and parity docs must stop
writing as if HDC similarity search already exists at the kernel trait level.

### Cross-Domain Transfer

Cross-domain resonance, analogy, and transfer APIs from doc `08` remain
**deferred**. The audit found no production `Resonance`, `TransferRisk`,
`DomainProfile`, or `AnalogyResult` substrate in the codebase.

Important nuance:

- dreams-side cross-domain hypothesis generation is real
- that does **not** mean doc-08 resonance transfer is implemented

## Current Reading Of `ContextAssembler`

For PU06, the honest description is:

- the assembler exists and is substantial
- it is a shipping library primitive
- whether it is on the main production path is a separate code-execution
  follow-up, not something these parity docs should overclaim

## Recommended Wording For Source Docs

- use present tense for `KnowledgeStore`, `NeuroStore`, and `ContextAssembler`
- use `not yet on Substrate` for `query_similar()`
- use `deferred` or `target-state` for cross-domain transfer and analogy
