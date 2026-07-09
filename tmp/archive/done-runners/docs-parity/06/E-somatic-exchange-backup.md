# E — Somatic Retrieval, Exchange, And Backup

Refresh of docs `13`, `14`, and `15` against current code.

## What Ships

- somatic and PAD-aware retrieval bias are real
- `ContextAssembler` already consumes affect state and contrarian retrieval
  policy
- the local neuro store is real and durable

## What Is Deferred

### Somatic Exchange And Library Of Babel

These remain **deferred**. The broader exchange story in docs `14` and `15`
should not be described as current runtime.

That includes:

- Library of Babel mesh exchange
- Korai / Lethe channel flows
- multi-agent publishing policies
- cross-collective knowledge traffic

### Backup / Restore / Publish Flows

These also remain **deferred**.

`crates/roko-cli/src/main.rs:569-591` shows that `NeuroCmd` currently exposes
only `Query`, `Stats`, and `Gc`. There is no shipped `backup`, `restore`, or
`publish` neuro CLI surface.

## Required Doc Tone

- use present tense for somatic retrieval bias
- use deferred or target-state language for exchange, Library of Babel, and
  backup / publish systems
- keep local durable storage separate from any future mesh-sharing story
