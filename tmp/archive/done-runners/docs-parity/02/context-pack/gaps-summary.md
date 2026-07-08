# Gap Inventory — 02 Agents

Concise post-audit gap list for the narrowed batch scope.

## Focus Now

These are the highest-value gaps to document with evidence. They are not automatic instructions to start multi-crate implementation work.

### 1. Provider And Backend Docs Still Lag The Live Surface — HIGH

- the checked-in provider/runtime surface is broader than the old parity copy
- Gemini and Perplexity are first-class provider paths now
- provider docs should distinguish provider kinds, runtime families, and concrete agent types

### 2. Tool Runtime Coverage Needs Precise Wording — HIGH

- `ToolLoop`, `ToolDispatcher`, and `SafetyLayer` are clearly shipped
- several provider families reach the tool runtime today
- the shared backend helper is not the whole story, so docs should stop flattening provider-specific and shared-path coverage together

### 3. MCP, Sidecar, And Lifecycle Docs Understate What Is Live — HIGH

- `agent.mcp_config` passthrough is working
- `roko-agent-server` has real dispatcher-backed routes and tests
- `PlanRunner` owns `ProcessSupervisor`
- parity docs should present those as shipped infrastructure

### 4. `AgentEvent` Duplication Is The Main Concrete Cross-Crate Gap — MEDIUM

- a narrower enum lives in `roko-agent`
- a wider runtime/learning enum lives in `roko-learn`
- this is a real cleanup target and worth naming directly

### 5. Domain Profiles And Plugin SPI Tiers 4-5 Must Be Deferred — MEDIUM

- these were described too far ahead of code and usage
- keep them only as explicit future work
- do not imply they are partial parity or lightly wired

## Secondary Gaps

- response-type ownership remains uneven
- temperament is descriptive metadata, not a typed shared runtime policy
- plan-path dispatcher reachability should be described precisely, not globally

## Working Rule

If a finding requires:

- executor-state redesign,
- learning-policy redesign,
- verification-policy redesign,
- or a broad cross-crate migration,

then batch `02` should record the current evidence, the owning follow-on batch, and the minimum seam left behind.
