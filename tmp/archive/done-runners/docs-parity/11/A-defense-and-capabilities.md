# A — Defense, Capabilities, and Permits

Parity review for Docs 00, 01, and 04.

Generated: 2026-04-18

---

## Core Read

The safety docs should start from the shipped authorization stack, not from a greenfield design:

- `roko-agent` already ships `AgentContract`, `GovernanceRule`, and `Invariant`
- `roko-agent` already ships `AgentWarrant` and the agent-layer `Capability` enum
- `roko-orchestrator` already ships typed `Capability<K>` tokens at **860 LOC**

This topic's job is to document that stack accurately, then clearly label anything beyond it as planned.

---

## Shipping Now

### A.01 — `SafetyLayer` is the live entry point

`SafetyLayer` is already the runtime safety composite for the dispatcher path. It is not a placeholder API.

### A.02 — `AgentContract` already ships

REF32 undercounted the current system by not starting from `AgentContract`.

The parity docs should treat these as existing:

- role-scoped contracts
- invariants
- governance rules

This is the current contract surface, not future work.

### A.03 — `AgentWarrant` and agent-layer `Capability` already ship

The agent crate already has an operational warrant/capability layer for tool execution. Any doc that presents warrants as a proposed direction is stale.

### A.04 — `Capability<K>` ships and is the main doc-honesty fix

`Capability<K>` with typed marker kinds is live in `crates/roko-orchestrator/src/safety/capability_tokens.rs` at **860 LOC**.

This means Doc 01 should no longer say "target capability design." The correct framing is:

- coarse capability checks already exist in `roko-agent`
- typed one-shot capability tokens already exist in `roko-orchestrator`
- future work is about adoption and integration breadth, not inventing the type

### A.05 — permit-style scoping also ships

The orchestrator safety layer already includes `Permit`. That supports a narrow, factual story for Docs 01 and 04: authorization and scoped grants already exist in code.

---

## Narrow, Don’t Inflate

### A.06 — tool-tier language should be treated as explanatory, not as missing implementation

If Docs 01 or 04 use T1/T2/T3-style tier language, keep it clearly secondary to the shipped code. The real source of truth is the current contract, warrant, capability, and permit system.

### A.07 — role matrices should be described as live policy wiring

The docs can say role-scoped gating ships. They should not imply a brand-new authz framework is required before the current system counts.

---

## Explicitly Deferred

These do not belong in this parity refresh except as planned extensions:

- replacement authz abstractions
- stronger theoretical tier taxonomies
- speculative custody-style safety primitives

---

## Recommended Doc Posture

For Docs 00, 01, and 04:

1. start with the current `SafetyLayer` + `AgentContract` + `AgentWarrant` system
2. present `Capability<K>` as shipping
3. treat extra taxonomy or stricter modeling as follow-on work

That is the smallest truthful rewrite that fixes the section.
