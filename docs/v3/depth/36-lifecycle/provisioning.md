# Provisioning

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/provisioning.md`
> Canonical source: v1 `docs/v1/17-lifecycle/02-provisioning.md`
> Status: **Current** (`resolve_manifest` pipeline in `roko-agent/src/lifecycle.rs`;
> `ProcessSupervisor` in `roko-runtime` provides restart backoff)

---

## 1. Purpose

Provisioning transforms an `AgentExtendedManifest` into a running agent
process with initialized knowledge store, configured model routing, loaded
tool profile, and active supervision. The pipeline uses Rust type-state
markers to enforce correct ordering at compile time -- you cannot start the
cognitive loop on an uninitialized agent because the method does not exist
on that type.

---

## 2. Three Deployment Paths

### Path A: Managed Compute (Hosted)

Zero infrastructure management. The managed service provisions VMs, injects
configuration, monitors health, and destroys machines on deletion.

| VM tier | Config | Price/hr | Typical use |
|---------|--------|----------|-------------|
| `micro` | 1 shared CPU / 256MB | $0.025 | Simple monitors |
| `small` | 1 shared CPU / 512MB | $0.05 | Standard agent (default) |
| `medium` | 2 CPU / 1GB | $0.10 | Multi-tool, knowledge-heavy |
| `large` | 4 CPU / 2GB | $0.20 | Full cognitive loop |

### Path B: Self-Deploy Helper (Automated)

```bash
roko deploy --provider fly --app-name my-agent --region iad
roko deploy --provider docker --name my-agent
roko deploy --provider ssh --host 203.0.113.42 --user root
```

### Path C: Manual / Bare Metal

```bash
roko init
roko start --config roko.toml
```

A Raspberry Pi behind double-NAT works identically to a cloud VM because
connectivity is outbound-only (WebSocket to relay, no inbound ports needed).

---

## 3. Type-State Pipeline

Each provisioning stage transitions the agent to a new type. The compiler
prevents calling methods that belong to a later stage:

```rust
// crates/roko-agent/src/lifecycle.rs (conceptual)

pub struct Unvalidated;
pub struct Validated;
pub struct ResourcesAllocated;
pub struct NeuroInitialized;
pub struct RoutingConfigured;
pub struct ToolsLoaded;
pub struct MeshRegistered;
pub struct Ready;

pub struct Agent<S> {
    manifest: AgentExtendedManifest,
    state: AgentState,
    _stage: PhantomData<S>,
}

impl Agent<Unvalidated> {
    pub fn new(manifest: AgentExtendedManifest) -> Self { ... }
    pub fn validate(self) -> Result<Agent<Validated>, ProvisioningError> { ... }
}

impl Agent<Validated> {
    pub async fn allocate_resources(self) -> Result<Agent<ResourcesAllocated>, _> { ... }
}

// Each impl block gates its methods to the correct stage.
// Agent<Ready>::start_cognitive_loop() is the only entry to the running loop.
```

The PhantomData marker carries zero runtime cost. The type system enforces
the seven-stage invariant without any dynamic checks.

---

## 4. Pipeline Stages

| # | Stage | Layer | What happens | Duration |
|---|-------|-------|-------------|----------|
| 1 | Validate | -- | Check manifest against domain features, resource limits | ~instant |
| 2 | Allocate Resources | L0 | Claim VM (hosted) or verify local resources (self-hosted) | 300ms-30s |
| 3 | Initialize Neuro | L1 | Create Signal storage, set Ebbinghaus decay, configure tiers | ~1-3s |
| 4 | Configure Routing | L1 | Set up cascade routing, provider preferences, cost limits | ~instant |
| 5 | Load Tools | L1 | Register tools based on domain plugin and profile | ~500ms |
| 6 | Register Mesh | L4 | Connect outbound WebSocket to relay | ~500ms-2s |
| 7 | Ready | -- | Start cognitive loop | ~instant |

**Total provisioning time (warm pool)**: 3-8 seconds from manifest to first
cognitive loop iteration.

**Total provisioning time (cold)**: 15-30 seconds (image pull + boot +
initialization).

---

## 5. Agent Startup Sequence

```
1. Parse roko.toml                                  ~instant
2. Initialize knowledge store (Signal storage)       ~1-3s
   +-- Or restore from backup if available
3. Load tool profile, register tools                 ~500ms
4. Initialize model routing (cascade)                ~instant
5. Connect to coordination layer (outbound WS)       ~500ms-2s
6. Register on-chain identity (chain domain only)    ~2-5s
7. Start cognitive loop (first iteration)            ~instant
8. Health server reports 'ready'                     ~instant
```

**Total boot time**: The Rust binary starts in ~100ms. Full initialization
including knowledge store and connectivity: 3-8 seconds.

---

## 6. Process Supervision

`ProcessSupervisor` in `roko-runtime` manages agent processes with restart
backoff:

- Maximum 5 restarts per hour (configurable).
- Clean exit (code 0) means graceful shutdown -- no restart.
- Non-zero exit triggers restart with exponential backoff.
- On max restarts exceeded, agent enters `Crashed` state awaiting operator
  intervention.

### Restart Backoff

```rust
// crates/roko-runtime (conceptual)

pub struct RestartBackoff {
    pub failure_count: u32,
    pub base_delay: Duration,      // Default: 100ms
    pub max_delay: Duration,       // Default: 300s
    pub reset_after: Duration,     // Default: 300s
}

// delay = min(base_delay * 10^failure_count, max_delay)
```

| Failure Count | Delay | Cumulative Wait |
|---------------|-------|-----------------|
| 0 | 100ms | 100ms |
| 1 | 1s | 1.1s |
| 2 | 10s | 11.1s |
| 3 | 100s | 111.1s |
| 4+ | 300s (capped) | 411.1s+ |

After 5 consecutive failures, the agent enters `Crashed` state.

---

## 7. Health Probes

Three-probe model adapted from Kubernetes:

| Probe | Checks | Failure action |
|-------|--------|----------------|
| **Liveness** | Event loop tick within timeout, no OOM | Supervisor restarts process |
| **Readiness** | Knowledge store initialized, routing configured, tools loaded | Remove from task distribution |
| **Startup** | First cognitive loop iteration completed | Kill and restart (boot failure) |

**Critical invariant**: While the startup probe has not yet passed, liveness
and readiness probes are not evaluated. This prevents the supervisor from
killing an agent that is legitimately slow to initialize (e.g., loading a
large knowledge backup during restore).

Default startup timeout: 300 seconds (30 failure checks at 10-second
intervals).

---

## 8. Configuration Injection

At provisioning time, configuration is injected via the platform's native
mechanism:

- **Hosted (Fly.io)**: File injection at machine creation.
- **Docker**: Environment variables and mounted config files.
- **Bare metal**: Config file at `--config` path.

Secrets (API keys, wallet keys) are accepted only from environment variables,
keystore files, or interactive stdin prompts -- never from CLI flags or config
files.

---

## 9. Init/Sidecar Pattern

Provisioning decomposes into sequential init tasks and persistent sidecar
processes:

**Init tasks** (must complete before agent starts):
- Schema migration -- upgrade `.roko/` directory layout.
- Secret injection -- load API keys from keystore/env.
- Knowledge restore -- if `--restore` flag, load backup.
- Model validation -- verify configured providers are reachable.

**Sidecar processes** (run alongside the agent):
- Metrics exporter (telemetry sampling).
- Log forwarder (episode streaming).
- Connectivity keepalive (reconnect on failure).

**Termination order**: Main agent terminates first, then sidecars in reverse
manifest order. This ensures metrics/logs capture the agent's final state.

---

## 10. Domain-Specific Provisioning

### Chain Domain (`roko-chain`)

1. Wallet provisioning (Delegation, Embedded, or LocalKey).
2. ERC-8004 identity registration on Korai chain.
3. Token setup (balance tracking, demurrage configuration).
4. DeFi tool loading (chain-specific tool set).

### Coding Domain

1. Sandboxed file system access configuration.
2. Compiler and test runner integration.
3. VCS integration (git access for PR workflows).

### General Domain

No domain-specific provisioning. Standard tool profile and knowledge
configuration only.

---

## 11. Implementation Sources

| Surface | File | What |
|---------|------|------|
| Type-state markers | `crates/roko-agent/src/lifecycle.rs` | `Agent<S>` pipeline |
| `ProcessSupervisor` | `crates/roko-runtime/src/` | Restart backoff, crash detection |
| Agent startup | `crates/roko-cli/src/agent_serve.rs` | `roko agent start` |
| Health probes | `crates/roko-agent-server/src/` | Sidecar health endpoints |

---

## Cross-References

- [agent-creation.md](agent-creation.md) -- Manifest generation
- [configuration-operator-model.md](configuration-operator-model.md) -- Config layers
- [agent-deletion-8-step.md](agent-deletion-8-step.md) -- Shutdown (provisioning in reverse)
