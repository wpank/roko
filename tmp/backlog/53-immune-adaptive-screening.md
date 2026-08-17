# Immune System Adaptive Screening

**Priority**: P2 — enforcement is live but visibility/adaptation gaps remain
**Size**: L (5-7d)
**Crates**: `crates/roko-agent/`, `crates/roko-graph/`, `crates/roko-core/`

---

## Problem

The five-stage `ImmunePipelineGraph` (Perception → Assessment → Containment → Validation →
Escalation) runs automatically for all canonical provider primary outputs and every
host-visible `ToolDispatcher` result. Suspicious output is withheld and durably quarantined.
This enforcement layer is production-complete.

Three categories of gap remain:

1. **Provider-internal opacity**: Provider-owned internal tool calls/results and provider
   trace Signals are outside the primary-output boundary. The host sees only the final
   `AgentResult`. If a provider makes dangerous internal calls (e.g., subprocess execution,
   network requests) during reasoning, those are invisible to screening.

2. **No adaptive immune memory**: Detectors are deliberately bounded pattern matchers rather
   than general semantic classifiers. There is no feedback loop from quarantine decisions to
   detector thresholds — a false positive at the same pattern fires indefinitely, and a novel
   attack pattern that doesn't match existing detectors passes indefinitely.

3. **No external ledger authentication**: Historical receipts prove internal consistency but
   are not externally anchored. A wholesale rewrite of all local ledgers (authority, vault,
   quarantine) would be undetectable without an external digest, MAC, or key.

---

## What already exists

| Component | File | Status |
|---|---|---|
| Pure 5-stage pipeline | `roko-core/src/immune.rs:284-415` | Live |
| Runtime Graph Cells | `roko-graph/src/cells/immune.rs` | Live, fail-closed |
| Provider output wrapper | `roko-agent/src/immune_boundary.rs` | Live, wraps all providers |
| Tool result screening | `roko-agent/src/tool_immune.rs` | Live, all ToolDispatcher results |
| Durable quarantine store | `.roko/immune/quarantine/` | Live |
| Vault index | `.roko/immune/quarantine-vault.json` | Live |
| Isolation controls | `.roko/immune/` authority/evidence files | Live |
| Provider boundary record | `ProviderBoundaryRecord` struct | Live, content-addressed |

---

## What to do

### Gap 1: Provider-internal visibility (P3 — requires provider API changes)

This gap cannot be fully closed without provider cooperation. Partial mitigations:

- **Step 1a.** For providers that expose tool-use traces (Anthropic API `content_block`
  events), capture and screen intermediate tool calls through the same immune pipeline.
- **Step 1b.** Add a `provider_internal_calls_visible: bool` flag to `ProviderCapabilities`
  so routing can prefer transparent providers for high-security tasks.

### Gap 2: Adaptive immune memory (P2 — main deliverable)

- **Step 2a.** Add an `ImmuneMemory` store that records quarantine decisions with their
  anomaly patterns, decision outcomes, and optional operator feedback (confirm/dismiss).
- **Step 2b.** On each screening pass, check `ImmuneMemory` for matching patterns. If a
  pattern was previously dismissed (false positive), reduce its anomaly score. If confirmed,
  increase sensitivity for similar patterns.
- **Step 2c.** Persist `ImmuneMemory` to `.roko/immune/memory.json` with the same
  atomic-write discipline as the quarantine store.
- **Step 2d.** Add `roko knowledge immune stats/review` CLI commands for operator feedback.

### Gap 3: External ledger authentication (P3 — defense in depth)

- **Step 3a.** After each quarantine/isolation write, append a content-addressed digest to
  an append-only local chain file (`.roko/immune/chain.jsonl`).
- **Step 3b.** Optionally anchor the chain head to an external service (git commit hash,
  on-chain transaction) for tamper evidence.

---

## Acceptance criteria

- [ ] `ImmuneMemory` store persisted at `.roko/immune/memory.json`
- [ ] Quarantine decisions update memory with pattern + outcome
- [ ] Operator feedback (confirm/dismiss) adjusts future screening scores
- [ ] CLI commands for immune stats and quarantine review
- [ ] All existing tests pass (`cargo test -p roko-agent -p roko-graph -p roko-core`)
- [ ] Memory feedback does not weaken Critical-severity enforcement (hard floor)

---

**Origin**: GAPS.md "Immune system screening coverage -- PARTIAL" (2026-08-17)
