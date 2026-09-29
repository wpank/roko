# 05-18 -- Safety Layer

> **Implementation status (2026-09):** Complete (E34 8/8 strict). Six
> core policy families, AgentContract enforcement, trust-origin IFC,
> five-layer immune Graph, five-head corrigibility, sandbox policy,
> temporal monitoring, taint propagation, and mandatory audited hooks
> are all live.

---

## Overview

The `SafetyLayer` at `crates/roko-agent/src/safety/mod.rs` is the
central safety enforcement boundary for tool dispatch. It composes
multiple policy families into a single check that the dispatcher calls
before every tool execution.

### Design Properties

1. **Pure validators** -- policies take a call and context, return a
   verdict. No side effects, no mutation of the caller's state.
2. **First failure short-circuits** -- the dispatcher chains policies
   in order; the first failure is returned verbatim.
3. **Fail-closed** -- unknown roles deny all tools by default.
   Unknown tools are denied. Missing safety contracts are denied.

---

## SafetyLayer Structure

```rust
// crates/roko-agent/src/safety/mod.rs

pub struct SafetyLayer {
    pub bash_policy: BashPolicy,
    pub git_policy: GitPolicy,
    pub network_policy: NetworkPolicy,
    pub path_policy: PathPolicy,
    pub scrub_policy: ScrubPolicy,
    pub rate_limiter: Option<Arc<RateLimiter>>,
    pub safety_budget: Option<Arc<Mutex<SafetyBudgetTracker>>>,
    pub role: String,
    pub contract: AgentContract,
    pub warrant: Option<AgentWarrant>,
    pub temporal_monitor: Option<Arc<Mutex<TemporalMonitor>>>,
    pub tool_permission_policy: ToolPermissionPolicy,
    pub tool_permission_list: Vec<String>,
    pub sandbox_level: SandboxLevel,
    // ... internal fields
}
```

---

## Six Core Policy Families

### 1. BashPolicy (`safety/bash.rs`)

Controls which shell commands the `bash` tool is allowed to execute.

- **Denylist** -- regex patterns for commands that are always blocked
  (e.g., `rm -rf /`, `chmod 777`, direct API key export).
- **Allowlist** -- optional regex patterns for explicitly permitted
  commands.
- **Default posture** -- when no allowlist is configured, commands not
  matching the denylist are permitted.

```rust
pub struct BashPolicy {
    deny_patterns: Vec<Regex>,
    allow_patterns: Option<Vec<Regex>>,
}
```

The tool names matched: `bash`, `run_tests`.

### 2. GitPolicy (`safety/git.rs`)

Branch protection rules preventing agents from modifying protected
branches:

- **Protected branches** -- configurable list of branch patterns
  (default: `main`, `master`, `release/*`).
- **Allowed operations** -- read-only operations are always permitted.
- **Force push blocking** -- force pushes to any branch are denied by
  default.

### 3. NetworkPolicy (`safety/network.rs`)

Outbound destination allowlist for network-capable tools:

- **Allowed destinations** -- configurable list of hostname patterns.
- **Default posture** -- all outbound connections are denied unless
  explicitly permitted.
- **Tool scope** -- applied to `web_fetch`, `web_search`, and any
  tool with `ToolConcurrency::Serial` that makes network calls.

### 4. PathPolicy (`safety/path.rs`)

Worktree-relative path canonicalization and escape prevention:

- **Worktree root** -- all file operations are constrained to the
  worktree boundary.
- **Canonicalization** -- symlinks and `..` traversals are resolved
  and checked against the boundary.
- **Escape detection** -- any path that resolves outside the worktree
  is denied.

The tool names matched: `read_file`, `write_file`, `edit_file`,
`multi_edit`, `apply_patch`, `notebook_edit`, `ls`, `glob`, `grep`.

### 5. ScrubPolicy (`safety/scrub.rs`)

Secret scrubbing from tool outputs before they enter conversation
history:

- **Pattern matching** -- regex patterns for API keys, tokens, and
  other secrets (e.g., `sk-...`, `ghp_...`, `AKIA...`).
- **Redaction** -- matched strings are replaced with `[REDACTED]`.
- **Applied post-execution** -- scrubbing happens in the dispatcher's
  BOUND step (Step 5), after the handler has returned results.

### 6. Rate Limiter (`safety/rate_limit.rs`)

Per-tool and per-role rate limits:

```rust
pub struct RateLimiter {
    limits: HashMap<RateLimitKey, TokenBucket>,
    default_rate: f64,
    default_burst: u32,
}

pub enum RateLimitKey {
    Tool(String),
    Role(String),
    ToolRole(String, String),
}
```

Token-bucket algorithm with configurable rate and burst parameters.
The rate limiter is shared via `Arc` across all calls for a given
agent instance.

---

## AgentContract Enforcement

The `AgentContract` struct (`safety/contract.rs`) declares the
complete set of restrictions for a role:

```rust
pub struct AgentContract {
    pub role: String,
    pub allowed_tools: Option<Vec<String>>,
    pub invariants: Vec<Invariant>,
    pub governance: Vec<GovernanceRule>,
}
```

### Invariants

Invariants are always-true properties that the contract enforces:

```rust
pub enum Invariant {
    NeverWriteOutsideWorktree,
    NeverDeleteProtectedBranch,
    NeverExposeSecrets,
    NeverBypassGates,
    NeverModifyAuditLog,
    Custom(String),
}
```

### Governance Rules

Governance rules define conditional restrictions:

```rust
pub enum GovernanceRule {
    RequireApprovalFor(Vec<String>),
    DenyToolsFor(Vec<String>),
    MaxConcurrentCalls(usize),
    Custom(String),
}
```

### Hardened Default

`AgentContract::hardened_default("role_name")` applies the same
invariants and governance as `AgentContract::restricted` but leaves
`allowed_tools = None` so the TOML role-tools whitelist remains the
binding tool-access constraint.

---

## Trust-Origin IFC (Information Flow Control)

The trust-origin IFC lattice tracks the provenance of data flowing
through the system:

```rust
pub enum CamelTaintLevel {
    Trusted,   // Operator-authored, verified
    Local,     // Local user/task action
    Remote,    // External API or network source
    Untrusted, // Provider output, unverified
}
```

### TaintTracker (`safety/taint_propagation.rs`)

```rust
pub struct TaintTracker {
    // Tracks per-signal taint levels
    // Propagates taint through data flow
    // Enforces that higher-taint data cannot flow to
    // lower-taint sinks without explicit declassification
}
```

Taint propagation is transitive: if a tool result is derived from
untrusted provider output, the result inherits the `Untrusted` taint
level regardless of the tool's own trust level.

---

## Five-Head Corrigibility

Corrigibility is enforced through five ordered heads, each of which
can veto an action:

```rust
pub struct ActionContext {
    pub autonomy_level: Option<String>,    // "auto" | "observe"
    pub reversible: Option<bool>,          // Can the action be undone?
    pub modifies_audit: Option<bool>,      // Does it touch audit logs?
    pub outputs_verifiable: Option<bool>,  // Are outputs checkable?
    pub on_task: Option<bool>,             // Is it relevant to the task?
}
```

The `classify_corrigibility_action` function analyzes action
descriptions for concerning patterns:

- **Defiance** -- "ignore the user", "override human"
- **Oversight weakening** -- "disable audit", "bypass verification"
- **Deception** -- "fabricate results", "suppress errors"
- **Irreversibility** -- "force push", "delete without backup"
- **Off-task** -- "unrelated to the assigned task"

Protective patterns (starting with "prevent", "block", "detect",
"test", "never") are exempted from triggering these detections.

---

## Sandbox Policy

```rust
pub enum SandboxLevel {
    None,      // No sandboxing (test only)
    Restrict,  // Default: filesystem and network restrictions
    Isolate,   // Process-level isolation
    Container, // Container-level isolation
}
```

The `sandbox_level` field on `SafetyLayer` is applied at tool,
subprocess, and agent dispatch boundaries.

---

## Safety Violations

```rust
pub struct SafetyViolation {
    pub plan_id: String,
    pub task_id: String,
    pub violation_type: ViolationType,
    pub message: String,
    pub severity: ViolationSeverity,
}

pub enum ViolationType {
    PathEscape,
    SecretLeak,
    ContractViolation,
    BudgetExhausted,
    ForbiddenTool,
    TaintViolation,
    CorrigibilityViolation,
    SandboxViolation,
}

pub enum ViolationSeverity {
    Block,  // Execution must stop
    Warn,   // Log warning, continue
}
```

---

## Temporal Monitoring

The `TemporalMonitor` (`safety/temporal.rs`) evaluates safety and
liveness properties on each tool call using linear temporal logic:

```rust
pub enum LtlProperty {
    Never(Predicate),     // Safety: must never be true
    Always(Predicate),    // Invariant: must always hold
    Eventually(Predicate),// Liveness: must eventually become true
}
```

When a `Never` property is violated or an `Always` property fails,
the monitor records a `Violation`.

---

## Additional Safety Submodules

| Module | Purpose |
|--------|---------|
| `allowlist` | AllowlistGuard for tool access control |
| `authz` | Authorization decisions, confirmation channels, escalation targets |
| `capabilities` | AgentWarrant, Capability, ToolPermissionPolicy, plugin tier checks |
| `data_llm` | DataLlmRouter for sanitizing inputs to data-processing tools |
| `hallucination` | HallucinationDetector for detecting fabricated tool outputs |
| `hooks` | SafetyHook trait, CorrigibilityHook, TaintLevelHook, TaintedString |
| `normalize` | Input normalization for classification |
| `provenance` | Attestation levels, Custody records, Taint tracking |
| `recursive` | Recursive safety for meta-agents; SpawnAuthority, delegation validation |
| `result_filter` | ResultFilter for post-execution output filtering |
| `risk` | SafetyBudget, OperationalConfidenceTracker, Kelly fraction, irreversibility scoring |
| `spending` | SpendingLimiter, ToolCostEstimate |
| `witness` | WitnessDag, WitnessLogger for integrity verification |

---

## DispatchSafetyContext

For non-tool dispatch (provider invocations), the
`DispatchSafetyContext` carries the action facts:

```rust
pub struct DispatchSafetyContext {
    pub action_description: String,
    pub input_taint: CamelTaintLevel,
    pub corrigibility: ActionContext,
    pub requires_network: bool,
}
```

Factory methods:
- `DispatchSafetyContext::trusted()` -- for internal callers.
- `DispatchSafetyContext::for_local_action(desc)` -- classifies
  corrigibility from the action description text.

---

## Safety Budget

The `SafetyBudgetTracker` (`safety/risk.rs`) tracks cumulative risk
across an agent's lifetime:

```rust
pub struct SafetyBudgetTracker {
    pub budgets: HashMap<BudgetDimension, SafetyBudget>,
}

pub enum BudgetDimension {
    FileWrites,
    NetworkCalls,
    BashExecutions,
    GitOperations,
    DangerousTools,
}
```

Each dimension has a configurable limit. When a budget is exhausted,
subsequent calls in that dimension are denied with
`ViolationType::BudgetExhausted`.

---

## Implementation Sources

| File | Purpose |
|------|---------|
| `crates/roko-agent/src/safety/mod.rs` | SafetyLayer, SafetyViolation, ViolationType |
| `crates/roko-agent/src/safety/bash.rs` | BashPolicy |
| `crates/roko-agent/src/safety/git.rs` | GitPolicy |
| `crates/roko-agent/src/safety/network.rs` | NetworkPolicy |
| `crates/roko-agent/src/safety/path.rs` | PathPolicy |
| `crates/roko-agent/src/safety/scrub.rs` | ScrubPolicy |
| `crates/roko-agent/src/safety/rate_limit.rs` | RateLimiter |
| `crates/roko-agent/src/safety/contract.rs` | AgentContract, Invariant, GovernanceRule |
| `crates/roko-agent/src/safety/taint_propagation.rs` | TaintTracker |
| `crates/roko-agent/src/safety/temporal.rs` | TemporalMonitor, LtlProperty |
| `crates/roko-agent/src/safety/risk.rs` | SafetyBudget, OperationalConfidenceTracker |
| `crates/roko-agent/src/safety/capabilities.rs` | AgentWarrant, Capability |
| `crates/roko-agent/src/safety/recursive.rs` | Recursive safety, SpawnAuthority |

---

## Citations

1. `crates/roko-agent/src/safety/mod.rs` -- SafetyLayer, six policy
   families, violation types.
2. `crates/roko-agent/src/safety/contract.rs` -- AgentContract,
   hardened defaults.
3. `crates/roko-core/src/corrigibility.rs` -- ActionContext,
   five-head corrigibility.
4. `crates/roko-core/src/extension.rs` -- CamelTaintLevel,
   trust-origin lattice.
