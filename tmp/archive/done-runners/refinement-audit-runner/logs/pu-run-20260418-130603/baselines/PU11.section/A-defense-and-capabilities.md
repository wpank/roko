# A — Defense-in-Depth, Capability Tokens, Permits (Docs 00, 01, 04)

Parity of the three foundational safety chapters: defense-in-depth
architecture, capability tokens (`Capability<K>` with PhantomData),
and permits/allowlists.

**Major finding**: Safety is NOT a scaffold — it ships across
**TWO crates totalling ~7,183 LOC**. `crates/roko-agent/src/safety/`
(3,870 LOC, 9 modules) provides runtime guards. `crates/roko-orchestrator/
src/safety/` (3,313 LOC, 7 modules) provides the higher-level
"advanced" concepts (capability tokens, taint propagation, audit
chain, loop guard, permits, sandboxing).

Generated: 2026-04-16.

---

## A.01 — SafetyLayer composite with 6 runtime guards ships (Doc 00 §"Six Runtime Guards")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 specifies a `SafetyLayer` composite wiring six guards: BashPolicy, GitPolicy, NetworkPolicy, PathPolicy, ScrubPolicy, RateLimiter.
**Reality**: `crates/roko-agent/src/safety/mod.rs:77-95` declares `SafetyLayer { bash_policy: BashPolicy, git_policy: GitPolicy, network_policy: NetworkPolicy, path_policy: PathPolicy, scrub_policy: ScrubPolicy, rate_limiter: Option<Arc<RateLimiter>>, role: String, warrant: Option<AgentWarrant> }` — exactly the 6 guards plus role + optional warrant. `with_defaults()` at `:104-116` constructs conservative defaults. Module sizes: `bash.rs` 397 LOC, `git.rs` 719 LOC, `network.rs` 464 LOC, `path.rs` 487 LOC, `scrub.rs` 472 LOC, `rate_limit.rs` 508 LOC. Doc 16 confirms 50+ tests across the guards.

---

## A.02 — Three defense categories (structural / behavioral / cognitive) framing (Doc 00 §"Three Defense Categories")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 frames defense as structural (compile-time types) + behavioral (runtime guards) + cognitive (conductor / adaptive).
**Reality**: Structural defenses ship at the type level via `Capability<K>` with PhantomData (see A.04). Behavioral defenses ship via the 6-guard SafetyLayer (A.01). Cognitive defenses cross-reference `roko-conductor` (batch 07) + `roko-daimon` (batch 09 B.06 affect-aware routing). Framing holds.

---

## A.03 — Synapse-trait integration: safety as `Gate` + `Policy` (Doc 00 §"Integration with Synapse Loop")

**Status**: DONE
**Severity**: —
**Doc claim**: Safety uses `Gate` for verification + `Policy` for enforcement. Every Engram carries provenance by construction.
**Reality**: `crates/roko-chain/src/gate/{tx_sim_gate,wallet_gate}.rs` implement `roko_core::traits::Gate` (batch 08 F.09-F.10). `roko-agent/src/safety/` enforces via policy composition. Engram provenance tracked in `roko-core/src/provenance.rs` + `engram.rs` (see B.02).

---

## A.04 — Capability<K> with PhantomData type safety SHIPS (Doc 01 §"Target Capability<T> Design", Doc 00 §"Structural Defense")

**Status**: DONE (PRD undercount)
**Severity**: HIGH (for doc honesty)
**Doc claim**: Doc 01 §"Current ToolPermission system and target Capability<T> design" frames `Capability<T>` as a **target** design not yet implemented. Tables an advanced type-safe capability pattern with PhantomData + three tool tiers + compile-time enforcement.
**Reality**: `crates/roko-orchestrator/src/safety/capability_tokens.rs:1-860` ships the **full advanced design**:
- `CapabilityKind` trait at `:58-61` with `fn name() -> &'static str`
- Six zero-sized marker kinds at `:65-80`: `FileWrite`, `FileRead`, `NetworkEgress`, `SubprocessSpawn`, `GitMutate`, `SignalEmit` (each implementing `CapabilityKind`)
- `Capability<K: CapabilityKind>` struct at `:130-137`: `{ id: Uuid, target: String, issued_at_ms, ttl_ms, signature: [u8; 32], _kind: PhantomData<fn() -> K> }` — **not `Clone`, not `Copy`**, `#[must_use]` at `:129`
- `BurnedCapability` receipt at `:205+`
- `CapabilityError` enum at `:222+`
- `CapabilityIssuer` at `:261+` — only way to construct `Capability<K>`: `issue(target, ttl)` → `verify_and_burn(cap)` → `BurnedCapability` receipt
- Unforgeability: per-process secret + keyed-hash signature + type-system non-constructibility (no public constructor)
- Module doc at `:1-40` explicitly cites "parity §28.2"

Doc 01 claims this is a "target design" — it is a **shipping design** with 860 LOC of real implementation. The lower-bound `ToolPermission` (flags in `roko-agent/src/safety/capabilities.rs`) also ships — so both tiers exist.
**Fix sketch**: Update Doc 01 — move `Capability<T>` from "target design" to "shipping; see `roko-orchestrator/src/safety/capability_tokens.rs`". Clarify tier split: ToolPermission flags live in agent-layer for simple gating; full `Capability<K>` lives in orchestrator-layer for fine-grained authorization.

---

## A.05 — AgentWarrant (OCaps-style warrant) in SafetyLayer (Doc 01 §"OCaps Authorization")

**Status**: DONE
**Severity**: —
**Doc claim**: Object-capability (OCaps) warrants authorize tool execution.
**Reality**: `SafetyLayer.warrant: Option<AgentWarrant>` at `safety/mod.rs:94`. `AgentWarrant` + `Capability` + `CapabilityError` + `check_capability` + `delegate` re-exported from `capabilities.rs:51`. This is the agent-side OCaps surface; the richer `Capability<K>` with PhantomData lives in orchestrator (A.04).

---

## A.06 — Three tool tiers (Doc 01 §"Three Tool Tiers")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Three tool tiers: T1 (read-only), T2 (single-file write), T3 (multi-file / network / exec).
**Reality**: `crates/roko-agent/src/safety/mod.rs:55-67` declares tool groupings: `BASH_TOOLS = ["bash", "run_tests"]`, `NETWORK_TOOLS = ["web_fetch", "web_search"]`, `FILE_TOOLS = [9 file ops]`. These are **runtime policy groupings**, not the documented T1/T2/T3 tier system. A full tier system with type-level enforcement would use `Capability<FileRead>` vs `Capability<FileWrite>` vs `Capability<SubprocessSpawn>` — which does ship (A.04) but is not connected to tool-tier dispatch yet.
**Fix sketch**: Doc 01 §"Three Tool Tiers" should either (a) map T1/T2/T3 onto the shipping `Capability<K>` kinds (FileRead = T1, FileWrite = T2, SubprocessSpawn/NetworkEgress = T3) or (b) mark as `Design — Phase 2+`.

---

## A.07 — ToolPermission flags (Read / Write / Execute / Network) ship (Doc 04 §"ToolPermission Flags")

**Status**: DONE
**Severity**: —
**Doc claim**: `ToolPermission { Read, Write, Execute, Network }` flags on tool calls for coarse-grained gating.
**Reality**: Doc 16 §"Stage 3 permission" confirms ToolDispatcher runs `permission` check as stage 3 of its 7-stage pipeline. `roko-core/src/tool/` defines the permission types (per Doc 16 reference). Role-based permission matrix ships via SafetyLayer role + rate-limit key composition.

---

## A.08 — Role-based permission matrix (Doc 04 §"Role-Based Permission Matrix")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 tables a role × permission matrix (e.g., coder role has R/W/E, researcher has R/Network only).
**Reality**: `SafetyLayer.role: String` at `safety/mod.rs:92`, `with_role(role)` builder at `:119+`. `RateLimitKey` at `safety/rate_limit.rs` keys by `(role, tool)`. Role-based filtering is wired; specific role matrices are the plan/config-level concern, not code-level.

---

## A.09 — Task-level tool filters (Doc 04 §"Task-Level Tool Filters")

**Status**: DONE
**Severity**: —
**Doc claim**: Per-task tool allowlists filter which tools a task can invoke.
**Reality**: Doc 16 confirms stage 2 of the 7-stage dispatcher pipeline is `tool_filter → is this tool allowed for this role?`. Wired.

---

## A.10 — CSA MAESTRO 7-layer mapping (Doc 00 §"CSA MAESTRO 7-layer mapping")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 00's 2025-04 enhancement adds mapping to CSA's MAESTRO 7-layer threat framework.
**Reality**: `Grep 'MAESTRO|CSA' crates/ --include=*.rs` returns zero matches. Design-only academic mapping.

---

## A.11 — 9 attack categories adversarial testing framework (Doc 00 §"Adversarial Safety Testing Framework")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 00 §"Adversarial Safety Testing Framework (9 attack categories)" describes a test harness.
**Reality**: Doc 16 claims 50+ tests across the 6 guards. Whether they cover the specific 9 attack categories (prompt injection, sandbox escape, credential exfiltration, etc.) is unverified. The test infrastructure ships; categorical coverage claim needs a test audit.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 8 (A.01 6-guard SafetyLayer, A.02 defense categories, A.03 Synapse integration, A.04 Capability<K> with PhantomData, A.05 AgentWarrant, A.07 ToolPermission flags, A.08 role matrix, A.09 task-level filter) |
| PARTIAL | 2 (A.06 three tool tiers vs runtime groupings, A.11 9-category adversarial test coverage) |
| NOT DONE | 1 (A.10 CSA MAESTRO mapping) |

Section A shows topic 11 is **mostly shipping**. The biggest Doc
drift is **A.04**: Doc 01 frames `Capability<T>` as "target design"
while 860 LOC of the full advanced design ships at
`roko-orchestrator/src/safety/capability_tokens.rs`.

## Agent Execution Notes

### A.04 — Reframe Doc 01 from "target" to "shipping"

Doc 01 should be updated to cite `capability_tokens.rs:1-860` as the
shipping reference implementation of `Capability<T>`, distinct from
the simpler `ToolPermission` flag surface at the agent layer.

Acceptance criteria:

- Doc 01 "target Capability<T>" section marks it shipping,
- Doc 04 role matrix cites `safety/mod.rs:92` role field,
- Doc 00 CSA MAESTRO mapping explicitly marked design-only.
