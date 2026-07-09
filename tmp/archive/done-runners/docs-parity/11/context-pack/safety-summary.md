# Safety Summary — 11

Batch `11` is an audit-constrained doc/status refresh, not a safety-architecture buildout.

## Shipping system to acknowledge first

- Use the batch framing: the safety surface spans **two crates and 7,183 LOC**.
- `crates/roko-agent/src/safety/` is the shipping runtime-guard layer: `SafetyLayer`, bash/git/network/path/rate-limit/scrub, `AgentWarrant`, and contracts.
- `crates/roko-orchestrator/src/safety/` is the shipping advanced layer: `Capability<K>`, `AuditChain`, `TaintTracker`, `LoopGuard`, `Permit`, and `SandboxEnforcer`.
- `crates/roko-agent/src/dispatcher/mod.rs` already wires `ToolDispatcher::with_safety(SafetyLayer)`; the honest gap is coverage/status description, not absence of a safety system.

## What this batch should do

- refresh `tmp/docs-parity/11` so the shipped two-crate safety system is impossible to miss
- narrow work to realistic status corrections in the core parity docs
- refresh threat/risk and Doc 16 notes as coverage-status work
- mark chain/compliance/kernel/frontier material deferred instead of inflating it into required implementation

## Not this batch

- compliance-framework mappings or regulator packaging
- chain activation, formal methods, or frontier prompt-security work
- cognitive-kernel / namespace / syscall design
- deep taint-theory expansion beyond the shipped tracker
