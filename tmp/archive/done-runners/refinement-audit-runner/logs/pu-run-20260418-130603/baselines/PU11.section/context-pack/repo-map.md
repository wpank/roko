# Repo Map — 11 Safety

High-value paths for batch `11`.

## Primary code anchors

- `crates/roko-agent/src/safety/mod.rs`
- `crates/roko-agent/src/safety/bash.rs`
- `crates/roko-agent/src/safety/git.rs`
- `crates/roko-agent/src/safety/network.rs`
- `crates/roko-agent/src/safety/path.rs`
- `crates/roko-agent/src/safety/rate_limit.rs`
- `crates/roko-agent/src/safety/scrub.rs`
- `crates/roko-agent/src/safety/capabilities.rs`
- `crates/roko-agent/src/safety/contract.rs`
- `crates/roko-agent/src/dispatcher/mod.rs`
- `crates/roko-orchestrator/src/safety/capability_tokens.rs`
- `crates/roko-orchestrator/src/safety/audit_chain.rs`
- `crates/roko-orchestrator/src/safety/taint_propagation.rs`
- `crates/roko-orchestrator/src/safety/loop_guard.rs`
- `crates/roko-orchestrator/src/safety/permit.rs`
- `crates/roko-orchestrator/src/safety/sandboxing.rs`

## Primary docs

- `docs/11-safety/01-capability-tokens.md`
- `docs/11-safety/02-audit-chain.md`
- `docs/11-safety/03-taint-tracking.md`
- `docs/11-safety/05-loop-detection.md`
- `docs/11-safety/06-sandboxing.md`
- `docs/11-safety/07-prompt-security.md`
- `docs/11-safety/14-cognitive-kernel.md`
- `docs/11-safety/15-forensic-ai.md`
- `docs/11-safety/16-critical-integration-gap.md`
- `docs/11-safety/INDEX.md`

## Fastest verification searches

```bash
rg -n "Capability<|CapabilityKind|CapabilityIssuer|AgentWarrant" crates docs/11-safety --include=*.rs --include=*.md
rg -n "AuditChain|AuditEntry|content_hash|ContentHash" crates docs/11-safety --include=*.rs --include=*.md
rg -n "TaintTracker|TaintReason|is_tainted|mark_tainted" crates docs/11-safety --include=*.rs --include=*.md
rg -n "LoopGuard|SandboxEnforcer|SandboxPolicy|Permit" crates docs/11-safety --include=*.rs --include=*.md
rg -n "ToolDispatcher|with_safety|Critical Integration Gap|MCP Avoidance" crates docs/11-safety --include=*.rs --include=*.md
```
