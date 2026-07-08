# Developer UX: Building Agents in Rust

> **TL;DR**: The single largest obstacle to Roko's adoption by Rust
> developers is not performance, correctness, or features — it's
> *time to first working agent*. Today that number is measured in
> hours. This doc proposes a layered Rust SDK with four ergonomic
> entry points (one-liner, builder, trait-impl, runtime-impl), a
> deliberate error vocabulary, a plugin-aware examples/ directory,
> and documentation discipline that turns Roko into something a
> Rust dev can reach for the same way they reach for tokio. Target
> metric: "hello, agent" in under 60 seconds for any Rust dev who
> has cargo installed.

### For first-time readers

A handful of terms recur throughout this doc. Brief definitions:

- **Engram** — durable, BLAKE3-addressed record (episode, heuristic,
  plan, claim). Lives in a `Substrate`. See `02-engram-vs-pulse.md`.
- **Pulse** — ephemeral, sequenced in-flight message on the `Bus`.
  See `03-bus-as-first-class.md`.
- **Substrate / Bus** — the two kernel fabrics: durable store vs
  ephemeral stream. Both are traits a developer can implement.
- **Operator** — one of Scorer / Gate / Router / Composer / Policy.
  Operators consume either medium and are the units of extension.

## 1. The audiences

Four developer audiences, each with a different relationship to the
kernel. The SDK needs a pleasant surface for each.

1. **Application author**: wants to embed an agent in their Rust
   program. Doesn't want to know about Substrate traits.
2. **Agent author**: wants to build a role-specific agent with
   custom tools, templates, and maybe a gate. Needs the builder
   surface.
3. **Trait implementor**: wants to swap out a Substrate, Bus,
   Scorer, or Router with their own. Needs stable trait contracts.
4. **Runtime implementor**: wants to build a new execution mode
   (e.g., browser-based, edge, distributed). Needs access to the
   kernel types directly.

The four layers should be *visually* different. A one-line example
shouldn't look like a trait impl. A trait impl shouldn't require
writing boilerplate to reach the kernel.

## 2. The four entry points

### 2.1 One-liner (for demos and scripts)

```rust
use roko::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let out = roko::run("Summarize README.md").await?;
    println!("{out}");
    Ok(())
}
```

`roko::run` should pick sensible defaults: local model if available,
Claude if not, memory in `.roko/`, no plugins. This is the
"anybody can paste this in and see it work" hook.

### 2.2 Builder (for configured agents)

```rust
use roko::{Agent, Role, Model};

#[tokio::main]
async fn main() -> Result<()> {
    let agent = Agent::builder()
        .role(Role::Researcher)
        .model(Model::claude_opus())
        .tool(tools::web_search())
        .tool(tools::fs_read("."))
        .memory_dir("./.roko")
        .build().await?;

    let response = agent.send("What's new in Rust 1.91?").await?;
    println!("{response}");
    Ok(())
}
```

The builder is the *daily driver*. 90% of application authors
never leave this layer. It should:

- Return typed errors, not strings.
- Fail at `.build()` for misconfiguration, not at `.send()`.
- Have a `dry_run()` that prints the system prompt it would use.
- Support `clone()` for spawning worker agents from a template.

### 2.3 Trait impl (for custom kernel parts)

```rust
use roko_core::{Substrate, Engram, EngramHash, Result};

pub struct MySubstrate { /* ... */ }

#[async_trait]
impl Substrate for MySubstrate {
    async fn put(&self, e: Engram) -> Result<EngramHash> { ... }
    async fn get(&self, h: &EngramHash) -> Result<Option<Engram>> { ... }
    async fn query(&self, p: Predicate) -> Result<Vec<Engram>> { ... }
    async fn scan(&self, range: HashRange) -> Result<Stream<Engram>> { ... }
    async fn freeze(&self, h: &EngramHash) -> Result<()> { ... }
    async fn thaw(&self, h: &EngramHash) -> Result<Engram> { ... }
}

// Use it:
let agent = Agent::builder()
    .substrate(MySubstrate::new(...))
    .build().await?;
```

Trait impls should be *self-contained*: the trait should require
only what's in `roko-core`. No type leakage from the runtime tier
into the trait contract.

### 2.4 Runtime impl (for new execution modes)

```rust
use roko_runtime::{Runtime, Supervisor, EventBus};

pub struct BrowserRuntime { /* WASM-specific state */ }

impl Runtime for BrowserRuntime {
    type Supervisor = BrowserSupervisor;
    type Bus = InMemoryBus;
    fn supervisor(&self) -> &BrowserSupervisor { &self.supervisor }
    fn bus(&self) -> &InMemoryBus { &self.bus }
}

// ...
let agent = Agent::builder()
    .runtime(BrowserRuntime::new())
    .build().await?;
```

This is rarely used but *must exist* — otherwise Roko is locked to
tokio-on-Linux and can't target browsers, edge, or embedded.

## 3. Error vocabulary

Errors are UX. Rust makes them visible; Roko should make them
*useful*. The canonical shape:

```rust
#[derive(Debug, thiserror::Error)]
pub enum RokoError {
    #[error("substrate error: {0}")]
    Substrate(#[from] SubstrateError),

    #[error("bus error: {0}")]
    Bus(#[from] BusError),

    #[error("tool {tool} failed: {reason}")]
    Tool { tool: String, reason: String },

    #[error("gate {gate} rejected: {reason}")]
    Gate { gate: String, reason: String },

    #[error("agent timed out after {}ms", .0.as_millis())]
    Timeout(Duration),

    #[error("configuration invalid: {0}")]
    Config(String),

    // ... more
}
```

Rules:

- **Never** bubble up `anyhow::Error` at the public API surface.
- **Every** error variant must be actionable (tell the user what
  to try).
- **`#[non_exhaustive]`** on every enum so adding variants is
  non-breaking.
- **Backtrace** attached in debug builds; elided in release.

## 4. Docs discipline

Four levels of documentation, each targeting a different audience:

### 4.1 `README.md` (for discovery)

60-second pitch. Copy-pasteable one-liner. Link to docs site.

### 4.2 `docs/tutorials/` (for first-hour users)

Guided walkthroughs: "Build a research agent in 10 minutes,"
"Add a custom tool," "Swap the Substrate." Linear, with every
code block runnable as-is.

### 4.3 `docs/cookbook/` (for returning users)

Recipe format: "I want to do X, show me the minimum code."
Searchable by goal, not by API shape. Examples:

- "Run an agent against a codebase"
- "Stream responses to a web UI"
- "Swap memory backends"
- "Add a compliance gate"

### 4.4 `docs/reference/` (for API hunters)

Auto-generated from rustdoc, but curated: hand-written overviews
per crate explaining its role in the larger architecture.

### 4.5 Rustdoc on every public item

Not optional. Every `pub fn`, `pub struct`, `pub trait` has:

- One-line summary.
- Example code block.
- Cross-link to related items.
- `# Errors` section if it returns Result.
- `# Panics` section if it can panic.

Enforce with `#![warn(missing_docs)]` in each crate.

## 5. The `examples/` directory

Worth taking seriously. Proposed structure:

```
examples/
├── 01-hello-agent/         — the one-liner
├── 02-builder-basics/      — builder with one tool
├── 03-custom-tool/         — implementing a tool
├── 04-custom-gate/         — implementing a gate
├── 05-swap-substrate/      — using a custom Substrate
├── 06-multi-agent/         — coordinating two agents
├── 07-streaming/           — streaming responses via Bus
├── 08-plugin-manifest/     — declarative tool plugin
├── 09-research-workflow/   — paper → claim → heuristic
├── 10-web-ui/              — serving to a minimal web UI
```

Every example is a working cargo project. Every example's README
links to the relevant docs. `cargo test --examples` runs them all
in CI.

## 6. Builder patterns Rust developers already know

Borrow from the best:

- **`tokio::runtime::Builder`**: explicit build step, typed errors,
  sensible defaults.
- **`reqwest::ClientBuilder`**: fluent methods, no-magic semantics.
- **`serde::Serializer` trait**: narrow, stable, implementable.
- **`sqlx`**: compile-time query validation, derive macros for
  types.
- **`bevy_ecs`**: plugin pattern via App::add_plugin(...).

Roko's APIs should feel like they belong in this family. The
gesture is consistent fluent `.x().y().build()` with typed output.

## 7. Macros where they earn their keep

Macros are UX but abused are anti-UX. Proposed macros:

### 7.1 `#[tool]`

```rust
#[tool(
    description = "Read a file from disk",
    role_allow = ["researcher", "implementer"],
    files = "{path}",
)]
async fn read_file(path: String) -> Result<String> {
    tokio::fs::read_to_string(&path).await.map_err(Into::into)
}
```

Expands to a `Tool` impl with schema derived from the signature.

### 7.2 `#[gate]`

```rust
#[gate(rung = Rung::Unit)]
async fn my_gate(ctx: &GateCtx) -> GateResult { ... }
```

### 7.3 `claim!`

From `16-research-to-runtime.md` — resolves a Claim at build time
and produces a tracked parameter at runtime.

### 7.4 Prompt template DSL

```rust
let prompt = prompt! {
    role: Role::Researcher,
    context: episode.recent(5),
    situation: current_task,
    heuristics: hdc_match(0.7),
};
```

A declarative prompt builder that the Composer compiles. Beats
string concatenation.

## 8. `cargo roko` — dev workflow subcommand

A cargo plugin for agent dev workflow:

```bash
cargo roko new my-agent           # scaffold a new agent project
cargo roko play                    # REPL for iterating on prompts
cargo roko replay <episode>        # re-run an episode with new code
cargo roko bench                   # benchmark against a task suite
cargo roko explain <hash>          # trace a decision through operators
cargo roko heuristics              # browse current beliefs
```

These are the Rust-dev versions of `roko` CLI commands, scoped to
the current crate. They compose with existing cargo workflows and
make Roko feel like a first-class cargo citizen.

## 9. Type signatures as documentation

Rust's type system is expressive enough that *signatures alone*
can carry intent. Some opinions:

### 9.1 `Engram` vs `&Engram` vs `EngramHash`

- Take `&Engram` when you read.
- Take `Engram` when you'll mutate and return.
- Take `EngramHash` when you want to defer resolution.

Don't mix these casually. The right choice saves allocations and
tells the reader what will happen.

### 9.2 `async fn` vs `Stream`

- `async fn -> T` for single-shot operations.
- `async fn -> Stream<T>` for fan-out.
- Never `async fn` that returns a future of a stream of futures —
  flatten it.

### 9.3 `Box<dyn Trait>` vs generic parameter

- Generic `T: Trait` when the type is known at composition time.
- `Box<dyn Trait>` only when heterogeneity is required (e.g.,
  plugin dispatch).

### 9.4 `Result` typing

- `Result<T>` with crate's `Error` type at top of the module.
- Full type `Result<T, SpecificError>` only at crate boundaries.

## 10. Testing ergonomics

Agent testing is hard. Roko should ship testing helpers.

```rust
use roko::testing::{MockAgent, AssertingBus};

#[tokio::test]
async fn my_flow_hits_gate() {
    let agent = MockAgent::builder()
        .expect_tool("read_file", "fake contents")
        .build();
    let bus = AssertingBus::expect("gate.passed.unit");

    agent.run(bus, "read README.md").await.unwrap();

    bus.assert_expectations();
}
```

- **MockAgent**: scripted responses without hitting a model.
- **AssertingBus**: declarative expectations on Bus activity.
- **RecordingSubstrate**: captures all writes for snapshot testing.
- **TimeTravel**: advance the `demurrage` clock explicitly to test
  decay behavior.

All under `roko::testing::*`. None in `#[cfg(test)]`-only code —
these are for downstream tests too.

### 10.1 A second, end-to-end example

A heuristic-sensitive test that wires StateHub projections (see
`26-statehub-rearchitecture.md` §3) into the harness:

```rust
use roko::testing::{AgentHarness, RecordingSubstrate, FakeClock};
use roko::heuristics::HeuristicId;

#[tokio::test]
async fn retries_after_flaky_test_heuristic_fires() {
    let clock     = FakeClock::at("2026-04-16T09:00:00Z");
    let substrate = RecordingSubstrate::new();
    let mut h     = AgentHarness::new()
        .with_clock(clock.clone())
        .with_substrate(substrate.clone())
        .seed_heuristic(HeuristicId::from("flaky-test-log-first"))
        .build().await.unwrap();

    h.enqueue("run the failing test and diagnose").await;
    h.tick_until_idle().await;

    assert!(substrate.writes_matching("episode.tool.read_file").len() >= 1);
    assert!(h.projection::<GateHistory>().any(|g| g.rung == Rung::Unit));
    assert_eq!(h.applied_heuristics(), ["flaky-test-log-first"]);
}
```

### 10.2 `cargo test` and `cargo bench` integration

Roko ships first-class cargo integration so tests and benches don't
need a special runner:

- **`roko::testing::harness_main!()`** — expands at the top of a
  test file; rewires the tokio runtime, installs a `FakeClock`, and
  enables `#[roko_test]` attributes with fixtures.
- **`criterion` adapters** — `roko::bench::agent_bench(b, |h| ...)`
  returns a `Criterion` benchmark over a pre-seeded `AgentHarness`
  so you can measure plan latency, compose time, and gate cost as
  normal Criterion benches.
- **`cargo roko bench`** (see §8) wraps `cargo bench` and emits a
  run record into `.roko/learn/efficiency.jsonl` for regression
  tracking across commits.
- **Snapshot tests** — `RecordingSubstrate::assert_snapshot("name")`
  writes under `tests/snapshots/`; updates via `UPDATE_SNAPSHOTS=1`.

Net effect: the same `cargo test`/`cargo bench` muscle memory works,
but each test gets a reproducible substrate + bus + clock.

## 11. Release cadence and compatibility

Developers need *predictable* API evolution. Proposed rules:

- **SemVer strict**: major-bump for any trait signature change in
  `roko-core`, `roko-spi`, `roko-bus`, `roko-hdc`.
- **6-week release train**: minor versions every 6 weeks, patch
  versions on demand.
- **MSRV policy**: always stable Rust, upgrade MSRV only in minor
  releases, never in patch.
- **CHANGELOG.md**: every PR updates it. Keep-a-changelog format.
- **Deprecation**: two minor versions of deprecation before removal,
  with a migration guide entry.

This is mundane and load-bearing. Projects that skip it bleed
developers.

## 11.5 Debug builds, logging, tracing

Observability for a developer's own custom agents. The same
spans/metrics that `24-deployment-ux.md` §5 exposes in production
should be available in local dev, with less ceremony.

- **`tracing` everywhere**: `roko-core` emits spans at operator
  boundaries. Downstream code inherits them — `tracing::instrument`
  on a custom `Scorer` nests correctly under the kernel span.
- **`roko::log::init()`** — one call at `main` wires `tracing_subscriber`
  with sensible filters (`info` for your crate, `warn` for deps),
  respects `RUST_LOG`, and routes structured logs to stderr JSON
  when `--format json` is set on the runtime.
- **Span conventions**: every operator emits a span named
  `op.<kind>` with fields `{operator_id, pulse_seq?, engram_hash?}`
  so traces correlate cleanly to `27-realtime-event-surface.md`
  cursors.
- **Pulse inspector**: `roko::dev::tap_bus()` returns a
  `tracing::Subscriber` that also forwards every Pulse as a
  structured log — useful when your custom `Composer` appears to
  drop events.
- **`#[roko::instrument]`** — macro sugar over `tracing::instrument`
  that adds `operator_kind`, `agent_id`, and an auto-generated
  correlation id so multi-agent flows stay readable.
- **Debug-build assertions**: `debug_assertions` enables extra
  invariants in `roko-runtime` (e.g. Bus back-pressure never
  silently drops, Substrate writes always round-trip). Release
  builds elide them.

A pragmatic pattern: wire a `RecordingSubstrate` + a `tracing` layer
exporting to a local file, then `cargo roko explain <hash>` (see
§8) replays the trace with operator boundaries highlighted. See
also `26-statehub-rearchitecture.md` §5 on projecting bus traffic
into a live-updating debug view.

## 12. What "world-class dev UX" actually means

A Rust developer should be able to:

1. See Roko on GitHub, read the README, believe in it — in under
   a minute.
2. Clone the repo or `cargo add roko` and get a working one-liner
   — in under 5 minutes.
3. Build a custom tool and run it — in under 30 minutes.
4. Swap a Substrate or Bus impl with their own — in under half a
   day.
5. File an issue that gets a substantive response, not a robot
   reply — within a couple of days.

All five are achievable with the practices in this doc. Most of
them are *not* achievable with the current state of the repo —
not because the code is bad but because the SDK surface and
examples/docs haven't been designed as a product. They need to be.

### Related refinements

- `23-user-ux-running-agents.md` §2 — the verb set a developer's
  custom agent inherits for free at runtime.
- `26-statehub-rearchitecture.md` §3 — typed projections the harness
  in §10 exposes to downstream tests.
- `27-realtime-event-surface.md` §4 — WebSocket/SSE wire format
  that custom `Bus` impls must speak to interop with official
  clients.
- `17-plugin-extension-architecture.md` §2 — tier boundaries that
  govern where a given extension (macro-generated tool, native
  crate, WASM module) lives.
- `25-domain-specific-agents.md` §9 — how a developer packages their
  tools, heuristics, and gates as an installable profile bundle.
