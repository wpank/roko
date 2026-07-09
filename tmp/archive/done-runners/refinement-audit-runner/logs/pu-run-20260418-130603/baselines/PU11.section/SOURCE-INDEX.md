# SOURCE-INDEX — Code Anchors for 11-Safety Parity

Verified code references for batch `11`, organized by crate.

Generated: 2026-04-16

---

## Important Corrections First

- Safety ships across **TWO crates totalling ~7,183 LOC**: `roko-agent/src/safety/` (3,870 LOC runtime guards) + `roko-orchestrator/src/safety/` (3,313 LOC advanced primitives).
- Doc 01 frames `Capability<T>` as "target design" — it ships as `Capability<K>` with PhantomData at `roko-orchestrator/src/safety/capability_tokens.rs:1-860`.
- Doc 03 describes a full Denning-lattice taint system — the shipping `TaintTracker` (`taint_propagation.rs:1-409`) is simpler but functional.
- Doc 16's "Critical Integration Gap" headline is stale — SafetyLayer is wired to ToolDispatcher for 5 HTTP provider paths; subprocess paths are the residual.
- `AuditChain` (`audit_chain.rs:1-565`) ships as hash-chain primitive, separate from `roko-core::Engram` provenance.

---

## crates/roko-agent/src/safety/ (3,870 LOC, 9 modules)

### `mod.rs` (462 LOC) — SafetyLayer composite

| File | What | Section |
|------|------|---------|
| `mod.rs:29-36` | Module declarations: bash, capabilities, contract, git, network, path, rate_limit, scrub | A.01 |
| `mod.rs:43-48` | Imports: BashPolicy, GitPolicy, NetworkPolicy, PathPolicy, RateLimitKey/RateLimiter, ScrubPolicy | A.01 |
| `mod.rs:50-51` | `use self::capabilities::{exec_capability_from_command, network_capability_from_url}` + `pub use capabilities::{AgentWarrant, Capability, CapabilityError, check_capability, delegate}` | A.05 |
| `mod.rs:55-67` | Tool-name constants: BASH_TOOLS, NETWORK_TOOLS, FILE_TOOLS (9 file ops) | A.06 |
| `mod.rs:77-95` | `SafetyLayer { bash_policy, git_policy, network_policy, path_policy, scrub_policy, rate_limiter: Option<Arc<RateLimiter>>, role, warrant: Option<AgentWarrant> }` | A.01 |
| `mod.rs:104-116` | `with_defaults()` conservative constructor | A.01 |
| `mod.rs:119+` | `with_role(role)` builder | A.08 |

### `bash.rs` (397 LOC)

| File | What | Section |
|------|------|---------|
| Full file | `BashPolicy` — deny-pattern matching (rm -rf, sudo, curl pipe, fork bombs, 8192 char limit) | D.10 |

### `git.rs` (719 LOC)

| File | What | Section |
|------|------|---------|
| Full file | `GitPolicy` — block force-push, hard reset, branch deletion on protected branches | D.10 |

### `network.rs` (464 LOC)

| File | What | Section |
|------|------|---------|
| Full file | `NetworkPolicy` — scheme filtering, private-network blocking (RFC1918, link-local, loopback), host allow/deny | D.10 |

### `path.rs` (487 LOC)

| File | What | Section |
|------|------|---------|
| Full file | `PathPolicy` — worktree sandbox via canonicalization, escape prevention, symlink denial | C.07, D.10 |

### `scrub.rs` (472 LOC)

| File | What | Section |
|------|------|---------|
| Full file | `ScrubPolicy` — 9 default regex patterns (API keys, JWTs, private keys, env assignments) | C.05, D.10 |

### `rate_limit.rs` (508 LOC)

| File | What | Section |
|------|------|---------|
| Full file | `RateLimiter` sliding-window counter keyed by (role, tool), default 60 calls/60s | C.01, D.10 |

### `capabilities.rs` (188 LOC)

| File | What | Section |
|------|------|---------|
| Full file | `AgentWarrant`, `Capability`, `CapabilityError`, `check_capability`, `delegate` — agent-layer OCaps surface | A.05 |

### `contract.rs` (173 LOC)

| File | What | Section |
|------|------|---------|
| Full file | Tool behavioral contract primitives (pre/post/invariant) | E.11 |

---

## crates/roko-orchestrator/src/safety/ (3,313 LOC, 7 modules)

### `mod.rs` (12 LOC) — module declarations

### `capability_tokens.rs` (860 LOC) — type-safe `Capability<K>` with PhantomData

| File | What | Section |
|------|------|---------|
| `capability_tokens.rs:1-40` | Module doc: unforgeable + non-cloneable + single-use tokens | A.04 |
| `capability_tokens.rs:58-61` | `CapabilityKind` trait with `fn name() -> &'static str` | A.04 |
| `capability_tokens.rs:65-80` | 6 marker types: `FileWrite`, `FileRead`, `NetworkEgress`, `SubprocessSpawn`, `GitMutate`, `SignalEmit` | A.04 |
| `capability_tokens.rs:82-111` | `impl CapabilityKind` for each marker | A.04 |
| `capability_tokens.rs:129` | `#[must_use = "a capability is a one-shot permission token; present it to verify_and_burn"]` | A.04 |
| `capability_tokens.rs:130-137` | `Capability<K>` struct (id, target, issued_at_ms, ttl_ms, signature: [u8; 32], PhantomData) — not Clone, not Copy | A.04 |
| `capability_tokens.rs:139-200+` | `Capability<K>` methods: id(), target(), kind_name(), issued_at_ms() | A.04 |
| `capability_tokens.rs:205+` | `BurnedCapability` receipt | A.04 |
| `capability_tokens.rs:222+` | `CapabilityError` enum | A.04 |
| `capability_tokens.rs:261+` | `CapabilityIssuer` — issues + verifies + burns tokens | A.04 |

### `taint_propagation.rs` (409 LOC) — TaintTracker

| File | What | Section |
|------|------|---------|
| `taint_propagation.rs:1-27` | Module doc + example | B.05 |
| `taint_propagation.rs:39-45` | `TaintReason { category: String, detail: String }` | B.05 |
| `taint_propagation.rs:56-70` | TaintReason constructors: `external`, `user_input`, `propagated` | B.05 |
| `taint_propagation.rs:92-94` | `TaintTracker { inner: Mutex<HashMap<ContentHash, TaintReason>> }` | B.05 |
| `taint_propagation.rs:105-107` | `mark_tainted(hash, reason)` | B.05 |
| `taint_propagation.rs:111-113` | `is_tainted(hash)` | B.05, B.09 |
| `taint_propagation.rs:117-119` | `reason(hash)` | B.05 |

### `audit_chain.rs` (565 LOC) — append-only hash-chain

| File | What | Section |
|------|------|---------|
| `audit_chain.rs:37-53` | `AuditEntry { prev_hash: [u8; 32], kind, actor, resource, ts_ms, signature: Option<String> }` | B.01 |
| `audit_chain.rs:56-76` | `AuditEntry::new(prev_hash, kind, actor, resource)` | B.01 |
| `audit_chain.rs:82-85` | `with_signature(sig)` | B.01 |
| `audit_chain.rs:91+` | `content_hash()` — canonical byte encoding with `auditv1\|` prefix + field tags + length prefixes | B.01 |
| `audit_chain.rs:136+` | `AuditChain { tip / append / verify }` | B.01 |

### `loop_guard.rs` (364 LOC) — orchestrator-layer loop detection

| File | What | Section |
|------|------|---------|
| `loop_guard.rs:33+` | `LoopGuardConfig` | C.04 |
| `loop_guard.rs:57+` | `LoopVerdict` enum | C.04 |
| `loop_guard.rs:141+` | `LoopGuard` | C.04 |

### `permit.rs` (452 LOC) — permit system

| File | What | Section |
|------|------|---------|
| `permit.rs:34+` | `PermitScope` enum | A.09 |
| `permit.rs:102+` | `Permit` struct | A.09 |

### `sandboxing.rs` (651 LOC) — orchestrator-layer sandbox

| File | What | Section |
|------|------|---------|
| `sandboxing.rs:59+` | `SandboxError` enum | C.08 |
| `sandboxing.rs:107+` | `SandboxPolicy` | C.08 |
| `sandboxing.rs:138+` | `SandboxPolicyBuilder` | C.08 |
| `sandboxing.rs:217+` | `SandboxEnforcer<'p>` | C.08 |

---

## crates/roko-agent/src/dispatcher/mod.rs (~1,070 LOC)

| File | What | Section |
|------|------|---------|
| full file | `ToolDispatcher` with 7-stage pipeline: validate → tool_filter → permission → safety pre-exec → handler → truncate → safety scrub | A.09, F.10 |
| `with_safety(SafetyLayer)` | Injection point for SafetyLayer | F.08 |

---

## crates/roko-core/src/

| File | What | Section |
|------|------|---------|
| `provenance.rs` | Engram provenance + tainted flag | B.02 |
| `engram.rs` | Engram struct + lineage | B.02 |
| `ContentHash` | Content-addressing primitive (BLAKE3-shaped) | B.01, B.02, E.07 |

---

## Cross-crate cross-references

| Concern | Where | Section |
|---------|-------|---------|
| Circuit breaker | `roko-conductor` (batch 07 A.04) | C.02 |
| Ghost turn detection | `roko-conductor/src/watchers/ghost_turn.rs` (batch 07 A.03) | C.03 |
| Stuck detection | `roko-conductor/src/stuck_detection.rs` (batch 07 A.07) | C.03 |
| Adaptive gate thresholds | `.roko/learn/gate-thresholds.json` (CLAUDE.md) | C.06 |
| Worktree isolation | `.claude/worktrees/`, `.roko/worktrees/` | C.10 |
| System prompt builder (XML delimiters) | `roko-compose` (batch 09 D.11) | C.11 |
| Daimon affect-aware risk | `roko-daimon` + DaimonPolicy (batch 09 B.06) | D.13 |
| HealthMonitor (dark) | `roko-conductor/src/health.rs` (batch 07 E.05) | D.13 |
| `ChainWitnessEngine` (attestation anchor) | `roko-chain/src/witness.rs` (batch 08 F.08) | B.04 |
| `WalletGate` / `TxSimGate` | `roko-chain/src/gate/*` (batch 08 F.09-F.10) | E.01, E.09 |
| `roko replay` CLI | `crates/roko-cli` (CLAUDE.md) | F.05 |
| Dream budget (3D cost budget) | `roko-dreams/src/runner.rs:156-216` (batch 10 A.08) | D.14 |

---

## Missing / Absent (code-search negatives)

### Academic frameworks (all informational)

| Absent Feature | Search | Section |
|----------------|--------|---------|
| NIST AI RMF alignment | `rg -n "NIST\|AI RMF" crates --include=*.rs` | D.04 |
| MITRE ATLAS techniques | `rg -n "MITRE\|ATLAS" crates --include=*.rs` | D.05 |
| STRIDE-AI classification | `rg -n "STRIDE" crates --include=*.rs` | D.06 |
| OWASP Agentic Top 10 | `rg -n "OWASP\|ASI0[0-9]\|ASI10" crates --include=*.rs` | D.07 |
| CSA MAESTRO | `rg -n "MAESTRO\|CSA" crates --include=*.rs` | A.10 |

### Chain-domain safety (Tier 6 deferred)

| Absent Feature | Search | Section |
|----------------|--------|---------|
| MEV detection | `rg -n "MEV\|sandwich_attack\|frontrun" crates --include=*.rs` | E.01 |
| LTL Büchi automata | `rg -n "LTL\|Buchi\|TemporalMonitor" crates --include=*.rs` | E.03 |
| Formal verification pipeline | `rg -n "Heimdall\|Slither\|Echidna\|hevm\|Certora\|Kontrol" crates --include=*.rs` | E.09 |

### Advanced taint / PFI

| Absent Feature | Search | Section |
|----------------|--------|---------|
| FIDES / RTBAS / PFI / PCAS | `rg -n "FIDES\|RTBAS\|PFI\|PCAS\|Datalog" crates --include=*.rs` | B.08 |
| Bloom Oracle | `rg -n "BloomOracle\|bloom_oracle" crates --include=*.rs` | B.07 |

### Advanced risk math

| Absent Feature | Search | Section |
|----------------|--------|---------|
| Kelly sizing | `rg -n "kelly\|Kelly" crates --include=*.rs` | D.11 |
| Beta-Binomial tracker | `rg -n "beta_binomial\|OperationalConfidenceTracker" crates --include=*.rs` | D.12 |
| 5D safety budget | `rg -n "SafetyBudget\|irreversibility\|blast_radius" crates --include=*.rs` | D.14 |

### Prompt security frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| CaMeL dual-LLM | `rg -n "CaMeL\|dual[_-]LLM\|quarantined_llm" crates --include=*.rs` | C.12 |
| Ventriloquist on-chain | cross-ref batch 08 B.08 | C.13 |

### Cognitive kernel frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| Cognitive namespaces with ACL | `rg -n "CognitiveNamespace\|KernelNamespace" crates --include=*.rs` | F.01 |
| Cognitive scheduling | `rg -n "cognitive_scheduler\|priority_deadline" crates --include=*.rs` | F.03 |
| Engram syscalls | `rg -n "EngramSyscall" crates --include=*.rs` | F.04 |

### Forensic-AI compliance packaging

| Absent Feature | Search | Section |
|----------------|--------|---------|
| Regulatory report generators | `rg -n "EU AI Act\|HIPAA\|SOX\|GDPR\|SEC/CFTC" crates --include=*.rs` | F.06 |

---

## Practical Search Priorities

```bash
rg -n "SafetyLayer|BashPolicy|GitPolicy|NetworkPolicy|PathPolicy|ScrubPolicy|RateLimiter" crates --include=*.rs
rg -n "Capability<|CapabilityKind|CapabilityIssuer|AgentWarrant" crates --include=*.rs
rg -n "TaintTracker|TaintReason|is_tainted|mark_tainted" crates --include=*.rs
rg -n "AuditChain|AuditEntry|content_hash" crates --include=*.rs
rg -n "LoopGuard|SandboxEnforcer|Permit\b" crates --include=*.rs
rg -n "ToolDispatcher|with_safety|dispatch_pipeline" crates --include=*.rs
```
