# SOURCE-INDEX — 11 Safety

Primary code anchors for the batch-11 parity refresh.

Generated: 2026-04-18

---

## Baseline

Use this source index to keep the parity pack anchored to the shipped system:

- topic baseline: **7,183 LOC across 2 safety crates**
- doc focus: shipped safety first, ship-soon next, deferred frontier last
- module presence and runtime coverage are tracked separately

---

## Agent-Layer Safety Anchors

### `crates/roko-agent/src/safety/mod.rs`

| Anchor | Why it matters |
|---|---|
| `:79-100` | `SafetyLayer` fields, including `role`, `contract`, and `warrant` |
| `:121-136` | `with_defaults()` live default posture |
| `:146-176` | `with_role`, `with_contract`, `with_warrant` |
| `:183-320` | `check_pre_execution()` and the current enforcement chain |
| `:320-333` | role-based contract loading |

### `crates/roko-agent/src/safety/contract.rs`

| Anchor | Why it matters |
|---|---|
| `:26` | `AgentContract` |
| `:41` | main `AgentContract` implementation |
| `:169` | `Invariant` |

### `crates/roko-agent/src/safety/capabilities.rs`

| Anchor | Why it matters |
|---|---|
| `:27` | `AgentWarrant` |
| `:78` | `check_capability` |
| `:87` | `delegate` |

### `crates/roko-agent/src/dispatcher/mod.rs`

| Anchor | Why it matters |
|---|---|
| `:80` | `ToolDispatcher` |
| `:110` | `with_safety()` |
| `:123-200+` | dispatch pipeline entry and early stages |

### `crates/roko-agent/src/provider/mod.rs`

| Anchor | Why it matters |
|---|---|
| `:253-261` | provider-side dispatcher construction with optional safety layer |

---

## Orchestrator Safety Anchors

### `crates/roko-orchestrator/src/safety/capability_tokens.rs`

| Anchor | Why it matters |
|---|---|
| `:4-23` | module-level statement that `Capability<K>` is already the parity design in code |
| `:58-61` | `CapabilityKind` |
| `:65-107` | marker kinds such as `FileWrite`, `NetworkEgress`, `SignalEmit` |
| `:130-137` | `Capability<K>` |
| `:205` | `BurnedCapability` |
| `:222` | `CapabilityError` |
| `:314` | token issuing |
| `:354` | verify-and-burn path |

### `crates/roko-orchestrator/src/safety/audit_chain.rs`

| Anchor | Why it matters |
|---|---|
| `:37-53` | `AuditEntry` |
| `:56-76` | entry construction |
| `:82-85` | optional signature attachment |
| `:91-134` | canonical content-hash encoding |
| `:136-188` | `AuditChain` core state and append path |

### `crates/roko-orchestrator/src/safety/taint_propagation.rs`

| Anchor | Why it matters |
|---|---|
| `:3-9` | module-level shipped taint story |
| `:39-45` | `TaintReason` |
| `:56-70` | standard taint constructors |
| `:92-117` | `TaintTracker`, `mark_tainted`, `is_tainted`, `reason` |
| `:146-162` | propagation and signal-marking logic |

### `crates/roko-orchestrator/src/safety/loop_guard.rs`

| Anchor | Why it matters |
|---|---|
| `:33-52` | `LoopGuardConfig` |
| `:55-69` | `LoopVerdict` |
| `:141-203` | `LoopGuard` |

### `crates/roko-orchestrator/src/safety/permit.rs`

| Anchor | Why it matters |
|---|---|
| `:3-7` | permit model summary |
| `:100-114` | `Permit` and use-time validation framing |

### `crates/roko-orchestrator/src/safety/sandboxing.rs`

| Anchor | Why it matters |
|---|---|
| `:59-103` | `SandboxError` and core constraints |
| `:107-136` | `SandboxPolicy` |
| `:138-215` | builder surface |
| `:217-383` | `SandboxEnforcer` |

### `crates/roko-orchestrator/src/executor/mod.rs`

| Anchor | Why it matters |
|---|---|
| `:28` | executor imports `AuditChain` |
| `:156` | executor carries optional audit chain state |
| `:286-293` | executor wiring helpers for audit-chain integration |

---

## Supporting Cross-References

| Path | Why it matters |
|---|---|
| `docs/11-safety/01-capability-tokens.md` | currently the main capability-framing hotspot |
| `docs/11-safety/02-audit-chain.md` | should acknowledge shipping `AuditChain` |
| `docs/11-safety/03-taint-tracking.md` | should acknowledge shipping `TaintTracker` |
| `docs/11-safety/05-loop-detection.md` | should cite `LoopGuard` |
| `docs/11-safety/06-sandboxing.md` | should cite `SandboxEnforcer` |
| `docs/11-safety/16-critical-integration-gap.md` | stale headline; coverage-status rewrite target |

---

## Fast Verification

```bash
rg -n "AgentContract|AgentWarrant|Capability<K>|CapabilityKind|AuditChain|TaintTracker|LoopGuard|SandboxEnforcer|Permit\\b" crates/roko-agent crates/roko-orchestrator --glob '*.rs'
rg -n "with_safety|ToolDispatcher|subprocess|coverage" crates/roko-agent docs/11-safety --glob '*.rs' --glob '*.md'
```
