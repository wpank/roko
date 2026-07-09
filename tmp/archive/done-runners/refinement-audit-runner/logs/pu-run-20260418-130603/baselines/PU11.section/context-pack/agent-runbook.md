# Agent Runbook — 11 Safety

Batch `11` is a **status-calibration and scope-discipline pass**.

## Default posture

- Trust the shipping safety crates before trusting the PRDs.
- Treat doc drift as the main problem, not missing safety code.
- Prefer status correction, scope boundaries, and better acceptance criteria over speculative expansion.

## What good work looks like

- make the two-crate safety stack impossible to miss,
- reframe `Capability<K>`, `AuditChain`, `TaintTracker`, `LoopGuard`, and `SandboxEnforcer` as shipping surfaces where appropriate,
- narrow Doc 16 from a vague "critical gap" story to a concrete coverage story,
- mark compliance, chain-safety, kernel, and advanced-risk chapters as frontier or informational where they do not map to shipping code,
- leave later agents with one clear source index, one clear deferral map, and batch scopes that can run unattended.

## What to avoid

- do not invent missing formal-methods, chain, or compliance implementations,
- do not widen the batch into general security architecture redesign,
- do not blur "shipping primitive" and "fully integrated everywhere",
- do not force every theoretical section into this batch if it belongs to a later runtime pass.

## Deliverable standard

Each batch should leave:

- explicit scope and out-of-scope language,
- verification commands that match the claimed change,
- acceptance criteria that can fail concretely,
- clear notes for any deferred seams.
