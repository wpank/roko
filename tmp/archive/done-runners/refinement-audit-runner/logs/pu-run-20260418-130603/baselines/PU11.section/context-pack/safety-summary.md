# Safety Summary — 11

## Shipping now

- `roko-agent/src/safety/` 6-guard `SafetyLayer` runtime
- agent-layer `AgentWarrant` and lightweight capability checks
- `roko-orchestrator/src/safety/` advanced primitives:
  `Capability<K>`, `AuditChain`, `TaintTracker`, `LoopGuard`, `Permit`,
  `SandboxEnforcer`
- `ToolDispatcher` 7-stage pipeline with `with_safety(SafetyLayer)`
- conductor-backed circuit-breaker, ghost-turn, and stuck-pattern support
- content-addressed replay and provenance surfaces

## Shipping, but easy to misdescribe

- `Capability<K>` exists, but docs still frame it as "target design"
- `AuditChain` ships, but docs imply a more future-tense audit story
- `TaintTracker` ships, but it is simpler than the full Denning-style spec
- `SandboxEnforcer` ships, but some docs still treat sandboxing as future
- Doc 16 already acknowledges partial dispatcher coverage, but the title/status framing still overstates the gap

## Mostly future or informational

- compliance framework mappings
- advanced safety-budget math
- Tier 6 chain-safety pipeline
- CaMeL / Ventriloquist frontier prompt-security patterns
- cognitive-kernel namespace / syscall model
- regulator-facing forensic packaging
