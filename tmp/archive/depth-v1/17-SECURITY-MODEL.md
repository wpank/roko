# 17 — Security Model

> Three-layer capability intersection, recursive safety monitoring, and sandboxing at every tier.

**Source**: New synthesis combining the Block capability model ([doc-02](02-BLOCK.md)), Space grants ([doc-04](04-SPECIALIZATIONS.md)), Extension safety layers ([doc-08](08-EXTENSION-SYSTEM.md)), and autonomy-level safety mapping ([doc-10](10-LEARNING-LOOPS.md)).

---

## 1. Overview

Roko's security model is built on one principle: **the system fails closed**. No Block runs unless every layer of the capability stack explicitly permits it. Capabilities can be narrowed but never widened when delegated. Every grant, usage, and denial is logged as a Signal.

Three layers:

```
Block declaration  ∩  Graph allow-list  ∩  Space grant  =  effective capabilities
```

If any layer is missing a required capability, the Block does not run. There is no override, no sudo, no escape hatch. The intersection is computed at Graph-load time and enforced at Block-run time.

---

## 2. Capability Types

Capabilities describe what system resources a Block may access. Each capability type supports granular constraints.

```rust
pub enum Capability {
    FsRead { paths: Option<Vec<PathPattern>> },
    FsWrite { paths: Option<Vec<PathPattern>> },
    Net { domains: Option<Vec<String>> },
    Shell { commands: Option<Vec<String>> },
    Llm { providers: Option<Vec<String>> },
    Chain { read: bool, write: bool, networks: Option<Vec<String>> },
    Secrets { keys: Option<Vec<String>> },
    KnowledgeRead,
    KnowledgeWrite,
    Process { kind: ProcessKind },
    Custom { name: String, params: Value },
}
```

### 2.1 Capability semantics

| Capability | Grants | Constraints |
|---|---|---|
| `FsRead` | Read files from the filesystem | `paths`: glob patterns restricting which paths are readable |
| `FsWrite` | Write files to the filesystem | `paths`: glob patterns restricting which paths are writable |
| `Net` | Make outbound network requests | `domains`: allowlisted domains. `*` for unrestricted. |
| `Shell` | Execute shell commands | `commands`: allowlisted command names (e.g., `["cargo", "git"]`). No wildcard. |
| `Llm` | Call LLM providers | `providers`: optional provider filter (e.g., `["anthropic", "openai"]`) |
| `Chain` | Interact with blockchains | `read`/`write` flags. `networks`: allowlisted chain names. |
| `Secrets` | Access stored secrets | `keys`: specific secret names the Block may read |
| `KnowledgeRead` | Query the knowledge store | No constraints (scoped by Space) |
| `KnowledgeWrite` | Write to the knowledge store | No constraints (scoped by Space) |
| `Process` | Spawn or manage system processes | `kind`: `Spawn`, `Signal`, `Kill` |
| `Custom` | Extension-defined capabilities | `name` + arbitrary `params` |

### 2.2 Constraint narrowing

When a capability appears at multiple layers, the effective capability is the **intersection** of constraints:

| Block declares | Graph allows | Space grants | Effective |
|---|---|---|---|
| `Net { domains: ["api.openai.com"] }` | `Net { domains: ["api.openai.com", "api.anthropic.com"] }` | `Net { domains: ["*"] }` | `Net { domains: ["api.openai.com"] }` |
| `FsWrite { paths: ["**"] }` | `FsWrite { paths: [".roko/**"] }` | `FsWrite { paths: [".roko/**", "tmp/**"] }` | `FsWrite { paths: [".roko/**"] }` |
| `Shell { commands: ["cargo", "git", "rm"] }` | `Shell { commands: ["cargo", "git"] }` | `Shell { commands: ["cargo", "git", "npm"] }` | `Shell { commands: ["cargo", "git"] }` |

The narrowest constraint at any layer wins.

---

## 3. Three-Layer Capability Stack

### 3.1 Layer 1: Block declaration

Every Block declares what capabilities it requires in its TOML manifest:

```toml
[block.capabilities]
required = [
  { "FsRead"  = { paths = ["src/**", "docs/**"] } },
  { "FsWrite" = { paths = [".roko/artifacts/**"] } },
  { "Shell"   = { commands = ["cargo", "rustc"] } },
  { "Llm" = {} },
]
```

A Block that does not declare a capability cannot access that resource, even if the Graph and Space both allow it. This is the Block's contract with the system.

### 3.2 Layer 2: Graph allow-list

Graphs may restrict what their constituent Blocks can do, beyond what the Blocks themselves declare:

```toml
[graph.capabilities]
allow = [
  { "FsRead"  = { paths = ["src/**"] } },    # narrower than Block's declaration
  { "FsWrite" = { paths = [".roko/**"] } },
  { "Llm" = {} },
  # Shell intentionally omitted — not allowed in this Graph
]
```

Omitting a capability from the Graph's allow-list denies it. This lets a Graph author constrain a powerful Block to a safer subset for the Graph's specific context.

### 3.3 Layer 3: Space grant

The Space (workspace) is the user's authority. The user grants capabilities in `workspace.toml`:

```toml
[space.capabilities]
fs_read       = true                          # all paths
fs_write      = { paths = [".roko/**", "tmp/**", "dist/**"] }
net           = { domains = ["api.anthropic.com", "api.openai.com", "*.perplexity.ai"] }
llm           = true
shell         = { commands = ["cargo", "git", "npm", "rustc"] }
chain_write   = false
secrets       = { keys = ["anthropic_key", "openai_key"] }
```

Space grants are the user's final word. A capability not granted by the Space is denied regardless of Block and Graph declarations.

### 3.4 Resolution algorithm

At Graph-load time:

```
for each Block in Graph:
    for each capability in Block.required:
        graph_allowed = Graph.allow.contains(capability)
        space_granted = Space.grants.contains(capability)

        if !graph_allowed:
            error("Graph does not allow {capability} for Block {block}")
        if !space_granted:
            prompt_user("Block {block} requires {capability}. Grant?")

        effective[block][capability] = intersect(
            block.declared,
            graph.allowed,
            space.granted,
        )
```

At Block-run time, every resource access is checked against the effective capability set. Violations emit a `CapabilityDenied` error and are logged as Signals.

---

## 4. Delegation Caveats

When a Block delegates work to another Block (e.g., a Graph executing a sub-Graph, or an Agent dispatching a task), capabilities are **narrowed, never widened**.

```
Parent's effective capabilities  ⊇  Child's effective capabilities
```

This is the **caveat rule**: a parent may pass a subset of its capabilities to a child, but never capabilities it does not itself hold. The delegation chain is tracked:

```rust
pub struct DelegationChain {
    pub grants: Vec<DelegationGrant>,
}

pub struct DelegationGrant {
    pub from: BlockRef,
    pub to: BlockRef,
    pub capabilities: Vec<Capability>,
    pub caveats: Vec<Caveat>,
    pub timestamp: DateTime<Utc>,
}

pub enum Caveat {
    TimeLimit(Duration),
    UsageLimit(u32),                   // max invocations
    PathRestriction(Vec<PathPattern>),
    DomainRestriction(Vec<String>),
    ReadOnly,                          // downgrades write to read
}
```

Caveats allow further narrowing at delegation time. An Agent dispatching a coding task might delegate `FsWrite` with a `PathRestriction` caveat that limits writes to the specific directory being modified.

---

## 5. Recursive Safety Monitoring

The `RecursiveSafetyMonitor` is a React-protocol Block that runs continuously during any Flow, watching for structural and behavioral anomalies.

### 5.1 Depth limits

```rust
pub struct DepthLimits {
    pub max_graph_nesting: u32,        // max sub-Graph depth (default: 8)
    pub max_delegation_chain: u32,     // max delegation depth (default: 12)
    pub max_loop_iterations: u32,      // max per Loop node (from Graph config)
    pub max_fan_out: u32,              // max parallel branches (default: 64)
}
```

Exceeding any limit halts the Flow with a structured error.

### 5.2 Rate limits

```rust
pub struct RateLimits {
    pub max_blocks_per_minute: u32,    // Block executions per minute (default: 120)
    pub max_llm_calls_per_minute: u32, // LLM API calls per minute (default: 60)
    pub max_fs_writes_per_minute: u32, // filesystem writes per minute (default: 300)
    pub max_net_requests_per_minute: u32, // outbound HTTP per minute (default: 100)
}
```

Rate limits prevent runaway Blocks from overwhelming external systems or accumulating cost. When a limit is hit, the execution engine throttles (queues) rather than fails, unless the rate is extreme (10x limit), in which case it halts.

### 5.3 Quality bounds

```rust
pub struct QualityBounds {
    pub min_gate_pass_rate: f64,       // halt if pass rate drops below (default: 0.3)
    pub max_consecutive_failures: u32, // halt after N consecutive failures (default: 5)
    pub max_cost_multiplier: f64,      // halt if cost exceeds estimate by Nx (default: 3.0)
    pub max_duration_multiplier: f64,  // halt if time exceeds estimate by Nx (default: 5.0)
}
```

Quality bounds prevent the system from spending resources on a failing path. When a bound is hit, the SafetyReactor emits a halt Signal.

### 5.4 Caveat enforcement

The monitor continuously verifies that delegation caveats are respected:
- Time-limited delegations are revoked when the time limit expires
- Usage-limited delegations are revoked after N invocations
- Path and domain restrictions are enforced on every access

Caveat violations are treated as capability denials.

---

## 6. Autonomy Level Safety

Six autonomy levels, each with explicit bounds and requirements. Levels map to what the system can do without human approval.

| Level | Name | Bounds | Human involvement |
|---|---|---|---|
| 0 | **Observe** | Read-only. No mutations. | None needed |
| 1 | **Suggest** | Proposes actions as Signals. Does not execute. | Human reviews and approves each action |
| 2 | **Act-with-review** | Executes actions. Human reviews results before they persist. | Post-action review |
| 3 | **Act-with-guardrails** | Executes and persists within declared parameter ranges. | Review on bound violations |
| 4 | **Act-autonomously** | Full execution within capability grant. Escalates novel situations. | Review on escalation only |
| 5 | **Structural** | Can modify Graphs, create Blocks, alter agent configuration. | Human approval required for every structural change |

### 6.1 Level enforcement

The Space configuration declares the maximum autonomy level:

```toml
[space.safety]
max_autonomy_level = 3
structural_changes = "require-approval"
```

Blocks and Graphs can request a lower level but never a higher one. Level 5 (structural changes) always requires human approval regardless of Space configuration.

### 6.2 Parameter ranges

At Level 3+, Blocks declare parameter ranges. The safety monitor ensures runtime values stay within declared ranges:

```toml
[block.safety]
params.temperature = { min = 0.0, max = 1.0 }
params.max_tokens = { min = 100, max = 32000 }
params.budget_usd = { min = 0.01, max = 50.0 }
```

Out-of-range values are clamped (if `clamp = true`) or rejected (if `clamp = false`).

### 6.3 Rollback

Every autonomy level above 0 maintains a rollback capability:

- **Level 1-2**: No persistent changes; rollback is trivial (discard proposed Signals)
- **Level 3-4**: Every mutation is journaled. Rollback replays the journal in reverse.
- **Level 5**: Structural changes are staged in a separate branch. Rollback discards the branch.

---

## 7. Sandboxing by Implementation Tier

Each Block implementation tier has a sandboxing strategy:

### 7.1 Rust Blocks (no sandbox)

Compiled into the roko binary. Full trust. Used only for built-in Blocks and trusted in-tree plugins. No marketplace distribution in this tier.

Security relies on:
- Code review
- Capability system (runtime checks)
- Process-level isolation (the entire roko process)

### 7.2 WASM Blocks (WASM sandbox)

Sandboxed by the WASM runtime (wasmtime).

```rust
pub struct WasmSandbox {
    pub fuel_limit: u64,               // max execution steps (default: 100_000_000)
    pub memory_limit_mb: u32,          // max memory (default: 64 MB)
    pub table_limit: u32,              // max table entries (default: 10_000)
    pub instance_limit: u32,           // max concurrent instances (default: 4)
}
```

**Fuel metering**: every WASM instruction consumes fuel. When fuel is exhausted, the Block is terminated. This prevents infinite loops and excessive computation.

**Memory limits**: the WASM linear memory is capped. Exceeding the limit traps.

**Syscall filtering**: WASM Blocks can only access capabilities through the roko-defined ABI. Direct syscalls are not available. Network access, filesystem access, and process spawning all route through capability-checked host functions.

### 7.3 Script Blocks (OS-level process isolation)

Sandboxed by subprocess isolation.

```rust
pub struct ScriptSandbox {
    pub timeout: Duration,             // max execution time
    pub working_dir: PathBuf,          // isolated temp directory
    pub env: HashMap<String, String>,  // filtered environment
    pub stdin: StdinMode,              // json | raw
    pub stdout: StdoutMode,            // json | raw
}
```

**Filesystem isolation**: the engine creates a temp directory and symlinks only the paths declared in `FsRead` / `FsWrite` capabilities. The script cannot access paths outside its declared set.

**Network isolation**: if `Net` is declared, outbound traffic is routed through a proxy that enforces the domain allowlist. If `Net` is not declared, network access is blocked at the OS level (platform-specific: `seccomp` on Linux, `sandbox-exec` on macOS).

**Process isolation**: scripts cannot spawn subprocesses unless `Shell` is explicitly declared. The subprocess runs with minimal environment variables.

### 7.4 Composition Blocks (inherited)

TOML-only Blocks that compose other Blocks. No execution of their own. Their effective sandbox is the intersection of their constituent Blocks' sandboxes.

---

## 8. Marketplace Security

### 8.1 Capability tree disclosure

On install, the marketplace computes and displays the complete capability tree — not just the top-level artifact's capabilities but the transitive closure of all dependencies:

```
@wpank/doc-ingest@1.0.0

Direct capabilities:
  FsRead              any path
  FsWrite             .roko/artifacts/**
  Llm                 any provider

Transitive capabilities (via 8 Block dependencies):
  Net                 api.perplexity.ai, arxiv.org   (via perplexity-search@1.0.0)
  Shell               (none)
  Chain               (none)
  Secrets             (none)

Total capability surface: FsRead, FsWrite, Llm, Net (2 domains)
```

The user sees the full picture before granting.

### 8.2 Hash verification

Every marketplace artifact is content-addressed:

```toml
checksum = "blake3:abc123..."
```

The CLI verifies the checksum on download. Tampering is detected before any code runs.

### 8.3 Publisher signing

Artifacts are signed by the publisher's key (Sigstore integration). The install flow verifies the signature against the publisher's registered public key. Signature verification failure aborts the install.

### 8.4 Static analysis on WASM

Marketplace CI runs static analysis on submitted WASM Blocks:
- Banned imports (no raw syscalls, no unrestricted memory access)
- Fuel limit verification (the declared fuel limit is within marketplace bounds)
- Memory limit verification (the declared memory limit is within marketplace bounds)
- Known vulnerability pattern matching

### 8.5 Tier restrictions

| Tier | Public marketplace | Private marketplace | Local |
|---|---|---|---|
| Composition (TOML) | Allowed | Allowed | Allowed |
| WASM | Allowed | Allowed | Allowed |
| Script | Verified publishers only | Allowed | Allowed |
| Rust | Not allowed | Allowed | Allowed |

---

## 9. Audit Trail

Every capability-related event is logged as a Signal on the Bus and persisted to the Store:

```rust
pub enum SecurityEvent {
    CapabilityGranted {
        capability: Capability,
        granted_to: GrantScope,
        granted_by: String,
        space: SpaceId,
    },
    CapabilityDenied {
        capability: Capability,
        requested_by: BlockRef,
        reason: DenialReason,
        run: RunId,
    },
    CapabilityUsed {
        capability: Capability,
        used_by: BlockRef,
        run: RunId,
        details: Value,              // e.g., which path was read, which domain was called
    },
    DelegationCreated {
        from: BlockRef,
        to: BlockRef,
        capabilities: Vec<Capability>,
        caveats: Vec<Caveat>,
    },
    DelegationRevoked {
        grant_id: SignalRef,
        reason: RevocationReason,
    },
    SafetyViolation {
        kind: ViolationKind,
        block: BlockRef,
        run: RunId,
        details: Value,
    },
    AutonomyEscalation {
        from_level: u8,
        to_level: u8,
        block: BlockRef,
        reason: String,
        approved_by: Option<String>,
    },
}
```

### 9.1 Queryable audit

The audit trail is queryable via the Store protocol:

```bash
# All capability denials in the last 24 hours
roko run audit-query --input kind=CapabilityDenied --input since=24h

# All Shell capability usage for a specific Flow
roko run audit-query --input kind=CapabilityUsed --input capability=Shell --input run=wf_01HGZK7B...

# All delegation chains for a specific Agent
roko run audit-query --input kind=DelegationCreated --input from=agent:coder
```

### 9.2 Anomaly detection

The AnomalyLens ([doc-09 Telemetry](09-TELEMETRY.md)) monitors the security event stream for anomalies:
- Unusual capability usage patterns (a Block that normally reads 10 files suddenly reading 1000)
- Rapid delegation chain creation (possible privilege escalation attempt)
- Repeated capability denials from the same Block (possible probing)
- Cost anomalies correlated with capability usage

Anomalies emit alert Signals consumed by the SafetyReactor.

---

## 10. Agent Contract Enforcement

Agents operate under contracts that define their behavioral bounds:

```toml
# .roko/contracts/coder.toml

[contract]
agent = "coder"
version = "1.0.0"

[contract.bounds]
max_files_modified_per_task = 20
max_lines_changed_per_file = 500
allowed_file_extensions = ["rs", "toml", "md"]
forbidden_paths = ["Cargo.lock", ".env", "secrets/**"]
max_cost_per_task_usd = 5.0
max_duration_per_task = "15m"

[contract.behavioral]
must_run_gates_before_commit = true
must_preserve_existing_tests = true
escalate_on_security_findings = true

[contract.fallback]
on_missing_contract = "permissive"   # or "deny-all"
```

The safety layer checks contracts at dispatch time. Contract violations are logged and may halt the Agent depending on configuration. When no contract YAML exists, the system falls back to the configured default (`permissive` or `deny-all`).

---

## 11. Summary

| Layer | Mechanism | Enforcement point |
|---|---|---|
| Block capabilities | `block.capabilities.required` in TOML | Graph-load time + Block-run time |
| Graph allow-list | `graph.capabilities.allow` in TOML | Graph-load time |
| Space grants | `space.capabilities` in `workspace.toml` | Graph-load time + runtime |
| Delegation caveats | `DelegationGrant.caveats` | Every delegated Block-run |
| Recursive safety | `RecursiveSafetyMonitor` React Block | Continuous during Flow |
| Autonomy levels | `space.safety.max_autonomy_level` | Before every mutation |
| Parameter ranges | `block.safety.params` | Block-run time |
| WASM sandbox | Fuel metering, memory limits, syscall filtering | Every WASM instruction |
| Script sandbox | Process isolation, path restriction, network proxy | Every script execution |
| Marketplace security | Hash verification, signing, static analysis, tier restrictions | Install time |
| Agent contracts | `.roko/contracts/<agent>.toml` | Dispatch time |
| Audit trail | SecurityEvent Signals | Every capability event |
| Anomaly detection | AnomalyLens on security event stream | Continuous |

Every layer fails closed. The only way to widen permissions is for the user to explicitly grant them in the Space configuration. The system never assumes trust — it verifies at every boundary.

---

## 12. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Block requesting undeclared capability errors at Graph-load time | Negative test: Block uses `Net` without declaring it |
| Graph allow-list narrows Block capabilities | Test: Block declares `FsWrite { paths: ["**"] }`, Graph allows `FsWrite { paths: [".roko/**"] }`, Block writes to `src/` -> denied |
| Space grant denial prevents Block execution | Test: Space grants `Shell = false`, Block requires `Shell` -> denied at load time |
| Three-layer intersection computed correctly | Combinatorial test matrix: deny at each layer, verify closed |
| Delegation caveats enforced at runtime | Test: time-limited delegation expires, subsequent call denied |
| Recursive safety halts on depth limit exceeded | Test: Graph with 9 levels of nesting (limit 8) -> halt |
| Rate limits throttle and then halt | Test: Block fires 200 LLM calls/minute (limit 60) -> throttled, then halted at 10x |
| WASM fuel metering terminates runaway Block | Test: WASM Block with infinite loop -> terminated at fuel limit |
| Script sandbox denies filesystem access outside declared paths | Test: script reads `/etc/passwd` without FsRead for that path -> denied |
| Marketplace capability tree computed transitively | Test: Graph with 3 levels of Block dependencies -> all capabilities surfaced |
| Audit trail logs every capability event | Test: run a Graph, query audit -> all grant/deny/use events present |
| AnomalyLens detects unusual capability patterns | Test: Block reads 100x normal file count -> anomaly Signal emitted |
