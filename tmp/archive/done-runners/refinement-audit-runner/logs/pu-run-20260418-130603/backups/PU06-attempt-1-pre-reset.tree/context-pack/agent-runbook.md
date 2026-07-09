# Agent Runbook — Batch 06

Use this when updating the `06-neuro` parity materials after the 2026-04-17 audit.

## Mission

Refresh the neuro docs so they match the audited repo state. This is a parity and scoping pass, not a runtime activation plan.

## Workflow

1. Read the current context-pack file before rewriting it.
2. Read the local PU06 instructions and the 2026-04-17 neuro audit inputs.
3. Verify claims against code when a status statement could drift.
4. Prefer short, concrete status language over speculative roadmap prose.
5. Leave explicit deferred or target-state labels for anything outside today’s neuro runtime.

## Default Decision Rules

- State plainly that `roko-neuro` is 7 source files and wired.
- State plainly that `HdcVector` exists in `crates/roko-primitives/src/hdc.rs` at 345 LOC.
- Put HDC-on-Engram at the top of the priority list.
- Treat Substrate `query_similar`, cross-domain transfer, Library of Babel, mesh sync, publish/economics, and demurrage as not current-runtime features here.
- Treat Pulse / Datum / Worldview / Custody as target-state vocabulary unless a source file proves otherwise.
- Do not rewrite the pack as instructions to modify `orchestrate.rs`, Substrate, or chain code.

## Required Completion Evidence

Every PU06 completion note should include:

- which context-pack files changed,
- that the rewrite follows the 2026-04-17 audit and PU06 prompt,
- that HDC-on-Engram is listed as top priority,
- and which concepts were explicitly marked `deferred` or `target-state`.

## Failure Modes To Avoid

- turning a docs refresh into a runtime implementation roadmap,
- implying `roko-neuro` is mostly unbuilt,
- reviving the need for a separate `roko-hdc` crate,
- describing demurrage as partially real,
- presenting Pulse / Datum / Worldview / Custody as active neuro-runtime surfaces,
- suggesting Library of Babel, mesh sync, cross-domain transfer, or publish/economics are already wired.
