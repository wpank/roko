# RuntimeServices Builder

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 2.
> Documents `RuntimeServicesBuilder` (#243), profiles, and bundle construction.

---

## Overview

Before any execution begins, the runtime constructs a shared service facade.
`RuntimeServicesBuilder` takes a validated `RokoConfig` and a
`RuntimeProfile`, and produces a `RuntimeServices` value containing six
bundles. This ensures provider health registries, rate limiters, cost tables,
prompt caches, and process supervisors are constructed once and shared across
all execution paths.

The builder was introduced in PR #243 specifically to prevent service
duplication during the engine convergence period (#260, #276). Without it,
both Runner-v2 and the Graph engine would have independently constructed
their own service instances, leading to divergent health tracking, doubled
rate limits, inconsistent cost accounting, and doubled memory usage from
duplicate prompt caches. Now that convergence is complete, the builder
continues to serve as the single canonical construction path for all seven
runtime profiles.

**Source:** `crates/roko-execution/src/builder.rs`,
`crates/roko-execution/src/profiles.rs`

---

## The Six Bundles

```rust
pub struct RuntimeServices {
    pub dispatch:    Arc<DispatchFactory>,    // provider factory, model resolver, rate limiter
    pub prompt:      PromptBundle,            // prompt cache + builder config
    pub feedback:    Option<FeedbackBundle>,  // learning stores (None for light profiles)
    pub extensions:  ExtensionsBundle,        // plugin chain, MCP runtime
    pub observation: ObservationBundle,       // telemetry, event publisher
    pub guards:      GuardsBundle,            // safety, budget, process supervisor
    pub profile:     RuntimeProfile,
}
```

### DispatchFactory

The dispatch bundle owns provider construction. It carries:

- A `ProviderHealthRegistry` (shared with feedback) tracking live/unhealthy
  status per provider endpoint. Health updates from dispatch outcomes flow
  back through the same registry instance.
- A model-to-provider resolver for routing cascade decisions. The resolver
  reads from the config's provider catalog and the learned cascade state.
- Per-provider rate limiters using the token bucket algorithm. Burst capacity
  and refill rate are configurable per provider.
- A cost table mapping model IDs to per-token pricing, loaded from the
  config's `[models]` section.

### PromptBundle

```rust
pub struct PromptBundle {
    pub cache: Arc<PromptCacheHandle>,
    pub build_handle: PromptBuildHandle,
}
```

The cache holds pre-loaded knowledge entries, recent episodes, playbook
matches, and section-effectiveness data. Pre-loading at build time means
cells do not need to perform I/O during graph execution -- they query the
cache instead.

The build handle carries composition strategy configuration: section
ordering, token budget allocation per section, truncation policy, and
whether to include experimental sections.

### FeedbackBundle

```rust
pub struct FeedbackBundle {
    pub learn_dir: PathBuf,
    pub health_registry: Arc<ProviderHealthRegistry>,
    pub cascade_router: Option<Arc<CascadeRouter>>,
}
```

Only constructed for profiles that require feedback (`GraphPlan`,
`FullPlan`). The `learn_dir` points to `.roko/learn/`, where persisted
learning state lives. The cascade router loads arm statistics from
`.roko/learn/cascade-router.json` and provides learned model selection
at dispatch time. The health registry is the same `Arc` instance as in the
dispatch bundle, ensuring bidirectional health updates.

### ExtensionsBundle

Plugin chain and MCP runtime state. Optional for all profiles. When
present, it carries live MCP client handles and the plugin admission
policy. Plugin startup failures are non-fatal; the bundle records them
for diagnostic reporting without blocking execution.

### ObservationBundle

Telemetry event publisher and metric registry. Required for all profiles.
Provides the `TelemetryEventSink` used by the graph engine and cells to
emit observable events. The sink fans out to registered observers (Lens
executors, StateHub, JSONL writers).

### GuardsBundle

Safety contracts, budget enforcement, and process supervision. Required
for all profiles. The process supervisor tracks spawned agent processes
and ensures coordinated shutdown. The budget tracker enforces the
configured USD limit. Safety contracts provide the fail-closed tool
admission policy.

---

## Runtime Profiles

The `RuntimeProfile` enum determines which bundles are required, optional,
or forbidden. The profile matrix is encoded as data in
`profile_bundle_manifest()` so that drift is caught by snapshot tests.

```rust
pub enum RuntimeProfile {
    FullPlan,       // Runner-v2 plan execution (deprecated)
    GraphPlan,      // Graph engine plan execution
    Workflow,       // roko run (single prompt)
    DirectLight,    // roko do, roko develop
    AgentServer,    // roko agent serve
    ChatLight,      // roko chat
    AuthoredGraph,  // roko graph run
}
```

### Profile bundle matrix

| Profile | Dispatch | Prompt | Feedback | Extensions | Observation | Guards |
|---|---|---|---|---|---|---|
| `GraphPlan` | Required | Required | Required | Optional | Required | Required |
| `FullPlan` | Required | Required | Required | Optional | Required | Required |
| `Workflow` | Required | Required | -- | Optional | Required | Required |
| `DirectLight` | Required | Required | -- | Optional | Required | Required |
| `AgentServer` | Required | Required | -- | Optional | Required | Required |
| `ChatLight` | Required | Required | -- | Optional | Required | Required |
| `AuthoredGraph` | Required | Required | -- | Optional | Required | Required |

Only plan execution profiles require the feedback bundle. All profiles
require dispatch, prompt, observation, and guards. The "required" vs
"optional" distinction is enforced at build time -- the builder returns
an error if a required bundle cannot be constructed.

### BundleRequirement

```rust
pub enum BundleRequirement {
    Required,   // must succeed or build fails
    Optional,   // failure is non-fatal, recorded as diagnostic
    Forbidden,  // must not be constructed (currently unused)
}
```

### ProfileBundleManifest

```rust
pub struct ProfileBundleManifest {
    pub profile: RuntimeProfile,
    pub dispatch_required: bool,
    pub prompt_required: bool,
    pub feedback_required: bool,
    pub extensions_optional: bool,
    pub observation_required: bool,
    pub guards_required: bool,
}
```

Snapshot tests verify this manifest against the actual builder output to
prevent silent drift. If someone adds a new profile without updating the
manifest, the snapshot test fails.

---

## Builder Construction

The builder follows a staged construction pattern:

```rust
let services = RuntimeServicesBuilder::new(config, profile)
    .with_workspace(workspace_path)
    .with_overrides(overrides)
    .build()
    .await?;
```

### Build stages

1. **Validate profile.** Confirm the config satisfies the profile's
   mandatory bundles.

2. **Build dispatch.** Construct the `DispatchFactory` with provider
   health, rate limiters, and cost table from config.

3. **Build prompt.** Load knowledge, episodes, and playbooks into the
   prompt cache. Construct the build handle from composition config.

4. **Build feedback (if required).** Load cascade router state, connect
   the shared health registry, initialize the learning directory.

5. **Build extensions (if present).** Start MCP clients, load plugin
   chain, apply admission policy.

6. **Build observation.** Construct the telemetry event publisher and
   metric registry.

7. **Build guards.** Construct safety contracts, budget tracker, and
   process supervisor.

8. **Assemble.** Combine all bundles into the final `RuntimeServices` value.

### Overrides

`ExecutionOverrides` allows callers to override defaults without modifying
the workspace config:

| Override | Effect |
|---|---|
| `force_model` | Bypass cascade routing with a specific model |
| `force_provider` | Bypass provider selection |
| `budget_usd` | Override the configured budget |
| `timeout_secs` | Override the configured timeout |
| `dry_run` | Skip actual dispatch (useful for validation) |

Overrides apply after profile validation. A `force_model` override that
names a model not in the cost table produces a warning but does not fail
the build -- the cost table falls back to a default rate.

---

## Consumption

The `RuntimeServices` value is consumed by different execution surfaces:

| Surface | Profile | Entry point |
|---|---|---|
| `roko plan run` | `GraphPlan` | `drive_controller()` in roko-cli |
| `roko run` | `Workflow` | `run_workflow()` in roko-cli |
| `roko do` | `DirectLight` | `do_task()` in roko-cli |
| `roko develop` | `DirectLight` | `develop()` in roko-cli |
| `roko agent serve` | `AgentServer` | `agent_serve()` in roko-agent-server |
| `roko chat` | `ChatLight` | `chat_repl()` in roko-cli |
| `roko graph run` | `AuthoredGraph` | `AuthoredGraphController` in roko-execution |

### CellResources injection

The graph engine does not consume `RuntimeServices` directly. Instead,
the controller extracts handles and injects them into `CellContext` via
`CellResources`:

```rust
pub struct CellResources {
    pub dispatch_factory: Arc<DispatchFactory>,
    pub prompt_cache: Arc<PromptCacheHandle>,
    pub budget_tracker: Arc<BudgetTracker>,
    pub cancellation: CancellationToken,
    pub event_sink: Arc<dyn TelemetryEventSink>,
    pub workspace_path: PathBuf,
    pub worktree_path: Option<PathBuf>,
}
```

This decouples cells from the full service facade. Cells only see the
handles they need. The dispatch factory provides provider construction,
the prompt cache provides pre-loaded context, the budget tracker provides
atomic microdollar accounting, and the cancellation token enables
cooperative shutdown from the `GuaranteedFinallyController`.

---

## Testing

The builder includes `for_test()` constructors on each bundle type for
unit testing without full config:

```rust
let prompt = PromptBundle::for_test();
let feedback = FeedbackBundle::for_test();
```

Profile manifest tests use `insta` snapshots to verify the bundle matrix:

```rust
#[test]
fn profile_manifest_snapshot() {
    let manifest = profile_bundle_manifest(RuntimeProfile::GraphPlan);
    insta::assert_yaml_snapshot!(manifest);
}
```

This ensures that adding or removing bundle requirements from any profile
produces a visible diff in the test output, rather than silently changing
behavior.

---

## Verification Commands

```bash
# Build services for the GraphPlan profile (default for plan execution)
cargo run -p roko-cli -- plan run plans/<dir>

# Build services for the Workflow profile
cargo run -p roko-cli -- run "test prompt"

# Build services for the ChatLight profile
cargo run -p roko-cli -- chat
```
