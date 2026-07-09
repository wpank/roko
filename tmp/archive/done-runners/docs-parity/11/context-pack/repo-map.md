# Repo Map — 11 Safety

High-value paths for the audit-constrained `tmp/docs-parity/11` refresh.

## Shipping code anchors

- `crates/roko-agent/src/safety/mod.rs` — `SafetyLayer`, role/warrant wiring, guard composition
- `crates/roko-agent/src/safety/{bash,git,network,path,rate_limit,scrub,capabilities,contract}.rs`
- `crates/roko-agent/src/dispatcher/mod.rs` — `ToolDispatcher::with_safety(SafetyLayer)` coverage seam
- `crates/roko-orchestrator/src/safety/mod.rs`
- `crates/roko-orchestrator/src/safety/capability_tokens.rs` — `Capability<K>`
- `crates/roko-orchestrator/src/safety/audit_chain.rs` — `AuditChain`
- `crates/roko-orchestrator/src/safety/taint_propagation.rs` — `TaintTracker`
- `crates/roko-orchestrator/src/safety/loop_guard.rs` — `LoopGuard`
- `crates/roko-orchestrator/src/safety/permit.rs` — `Permit`
- `crates/roko-orchestrator/src/safety/sandboxing.rs` — `SandboxEnforcer`

## Owned parity files by batch

- `M1`: `00-INDEX.md`, `SOURCE-INDEX.md`, `A-defense-and-capabilities.md`, `B-audit-taint-provenance.md`, `C-runtime-guards.md`
- `M2`: `D-threat-risk-adaptive.md`, `F-kernel-forensics-gap.md`
- `M3`: `E-chain-safety.md`
- `M4`: `context-pack/*.md`
- `M5`: `run-docs-parity.sh` plus final consistency checks across `tmp/docs-parity/11`

## Fast verification

```bash
rg -n '7,183|two crates|SafetyLayer|Capability<K>|AuditChain|TaintTracker|LoopGuard|SandboxEnforcer' \
  tmp/docs-parity/11/00-INDEX.md \
  tmp/docs-parity/11/SOURCE-INDEX.md \
  tmp/docs-parity/11/A-defense-and-capabilities.md \
  tmp/docs-parity/11/B-audit-taint-provenance.md \
  tmp/docs-parity/11/C-runtime-guards.md

rg -n 'coverage status|status refresh|defer|frontier|Phase 2|Tier 6' \
  tmp/docs-parity/11/D-threat-risk-adaptive.md \
  tmp/docs-parity/11/E-chain-safety.md \
  tmp/docs-parity/11/F-kernel-forensics-gap.md

rg -n 'M1|M2|M3|M4|M5|7,183|two crates|doc/status refresh|defer' \
  tmp/docs-parity/11/context-pack/*.md \
  tmp/docs-parity/11/run-docs-parity.sh
```
