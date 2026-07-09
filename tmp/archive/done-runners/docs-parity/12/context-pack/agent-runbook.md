# Agent Runbook — 12 Interfaces

Batch `12` is a documentation truth pass.

## Default Posture

- trust checked-in code over ambitious interface prose
- split shipping from planned
- narrow overscoped docs instead of trying to rescue them with more invention

## What Good Work Looks Like

- shipping CLI/TUI/HTTP/sidecar surfaces are stated plainly
- `roko new` and standalone `roko explain` are marked non-shipping
- the `9090` vs `6677` drift is called out directly
- Spectre, web UI, A2UI, sonification, and IDE work are clearly deferred
- REF28 and REF26 are carried forward as near-term hardening work

## What To Avoid

- do not invent a browser app, renderer, audio layer, or IDE runtime
- do not use README text to override source code when they conflict
- do not treat proposal docs as evidence that a feature ships
- do not widen this batch into SDK, domain-profile, or roadmap design

## Verification Habit

Before finalizing a rewrite, check:

- `main.rs` for command truth
- `tui/tabs.rs` for tab truth
- `routes/mod.rs`, `providers.rs`, and `messaging.rs` for transport truth
- `README` files only for documenting drift, not for defining the ground truth
