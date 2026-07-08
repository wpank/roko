# Plugin & Extension Architecture

> **TL;DR**: Roko's value as a platform — rather than a product —
> depends on how easy it is for a third-party to contribute a
> Substrate, a Gate, a Scorer axis, a Composer template, a tool, an
> MCP server, or an entire Role. This doc proposes a layered SPI
> with five distinct extension points, each with a stable trait
> contract, a discovery mechanism, and a sandbox story. The goal:
> someone outside the project can ship a valuable extension in an
> afternoon, and we can trust it without auditing it line-by-line.

> **For first-time readers**: This doc defines how outsiders contribute
> to Roko without forking it. The five tiers range from "pure data"
> (markdown prompt template) to "arbitrary code in a WASM sandbox."
> Each tier has a risk/power trade-off and a matching sandbox. The key
> insight: the plugin tier should be chosen by the plugin's *power
> needs*, and Roko's safety layer should match the tier automatically.
> Read 22 (dev UX) and 25 (domain profiles) alongside — they are the
> two biggest consumers of this SPI.

## 1. Current state

Extension points today are implicit:

- Traits exist in `roko-core` but don't have companion examples in
  `examples/` showing third-party implementations.
- MCP servers are discoverable but invoking an arbitrary MCP means
  changing `roko.toml`.
- Tools live in `roko-std` and are hard-coded; adding one means a
  PR against core.
- Roles are in `roko-compose/src/templates/` — template files
  mixed with builder code.

Nothing is *wrong* with any of these. But nothing is *designed for
external consumption* either. Every extension touches core.

## 2. The five extension tiers

### Tier 1 — **Templates and prompts** (lowest barrier)

Role prompts, system-prompt layers, tool descriptions. Should be
pure data — TOML or Markdown with front-matter. Discoverable by a
glob under `plugins/prompts/**`. Safe by construction; no code
execution.

### Tier 2 — **Configuration profiles**

Bundles of settings: a "Python shop" profile, a "Rust OSS" profile,
a "frontend team" profile. Layer on top of `roko.toml`.
Discoverable by `plugins/profiles/**`.

### Tier 3 — **Declarative tools and MCPs**

Tools described by JSON schema + invocation spec (subprocess, HTTP,
MCP). No Rust needed. The agent sees them exactly like core tools.
Discoverable by `plugins/tools/**`. Safe inside the existing tool
sandbox.

### Tier 4 — **Native trait implementations**

Rust crates that implement one of the six kernel traits (Substrate,
Bus, Scorer, Gate, Router, Composer, Policy). Compiled into the
binary or loaded as a cdylib with the ABI stable. For advanced
extensions — custom vector stores, custom gate types, custom
routing strategies.

### Tier 5 — **WASM sandboxed extensions**

Extensions that want to execute logic but that we don't trust to
run in-process. Targets wasm32-wasi, imports a host SPI, gets
memory-limited and deterministic. Slightly slower, much safer.

Each tier is a different risk/power trade-off. A plugin author
picks the lowest tier they can get away with.

## 3. Extension SPI at a glance

```rust
pub trait Extension {
    /// Stable identifier: "org.example.my_gate"
    fn id(&self) -> &str;

    /// Semver of this extension.
    fn version(&self) -> Version;

    /// Declared capabilities: which trait(s) this implements.
    fn capabilities(&self) -> &[Capability];

    /// Declared permissions required (network, files, tools).
    fn permissions(&self) -> &Permissions;

    /// Health check — called on load and periodically.
    fn health(&self) -> Health;
}

pub enum Capability {
    Substrate(SubstrateKind),  // content-addressed, ephemeral, cold, ...
    Bus(BusKind),              // in-memory, persistent, cluster
    Scorer { axes: Vec<ScoreAxis> },
    Gate { rungs: Vec<GateRung> },
    Router { strategy: RouterStrategy },
    Composer { template_classes: Vec<TemplateClass> },
    Policy { kind: PolicyKind },
    Tool { schema: JsonSchema },
    Role { name: String, description: String },
}
```

An extension registers by placing a manifest at a known path; the
loader resolves and dispatches.

## 4. Declarative tool manifest (Tier 3)

The most important tier for ecosystem growth. Tool plugins should
be *purely declarative*:

```toml
# plugins/tools/cargo-udeps.toml
[tool]
id          = "cargo.udeps"
version     = "0.1.0"
description = "Find unused dependencies in a Rust project"

[tool.schema]
name        = "cargo_udeps"
description = "Detect unused deps in Cargo.toml"
parameters  = { workspace_root = { type = "string" } }

[tool.invoke.subprocess]
cmd         = "cargo"
args        = ["+nightly", "udeps", "--workspace"]
cwd         = "{{workspace_root}}"

[tool.safety]
role_allow  = ["researcher", "implementer"]
network     = false
files       = ["{{workspace_root}}/**"]
timeout_ms  = 300000
```

The Roko loader sees this, validates, and exposes `cargo_udeps` to
any role in `role_allow`. No PR against core. Publishable to a
registry. **This is the single biggest ergonomics win.**

## 5. Versioning and ABI stability

Tier 4 (native traits) has a hard versioning problem — Rust doesn't
have a stable ABI. Two solutions, used together:

1. **cdylib bridge**: a small `roko-extension-abi` crate defines a
   narrow C-FFI layer (version struct, vtable, opaque pointers).
   Native extensions compile against this. Semver bumps require
   recompilation but don't break data.
2. **In-tree extensions** as a secondary default: users can drop a
   crate into `./plugins/native/` and the Roko binary rebuilds
   itself with it included. Cargo handles the rest. This is the
   ergonomics-friendly path for project-local extensions.

Tier 5 (WASM) sidesteps all of this. wasm-bindgen + wasi-preview2
give a stable ABI by construction. This is likely where we push
serious third-party development.

## 6. Discovery, not configuration

Plugin configuration should be minimal. The loader walks
`plugins/**`, reads manifests, validates. A new plugin should be
usable without editing `roko.toml`. Disabling a plugin is
`plugins/<id>/disabled` or `roko plugin disable <id>`.

The `roko plugin` CLI:

```bash
roko plugin list                    # installed
roko plugin search <query>          # from registry
roko plugin install <id>            # into ./plugins
roko plugin uninstall <id>
roko plugin enable <id>
roko plugin disable <id>
roko plugin info <id>
roko plugin audit                   # permission review
```

## 7. The registry

A Roko Plugin Registry — `plugins.roko.dev` or similar — modeled on
crates.io but narrower:

- Plugins publish with a signed manifest.
- Each plugin lists required tier, permissions, and a health script.
- Reviews come from actual deployments: a plugin that has been
  active in 50 deployments for 30 days gets a verified badge.
- Security issues surface through a CVE channel.

This is a Phase-2 move. Phase-1 is a github-based mechanism:
plugins in public repos, `roko plugin install <github-url>`, trust
based on signatures.

## 8. Sandboxing model

For each tier:

| Tier | Sandbox | Notes |
|---|---|---|
| 1 | None needed | pure data |
| 2 | None needed | pure data |
| 3 | Existing tool safety layer | subprocess / MCP respects role_allow, files, network |
| 4 | Rust process isolation | honor system's Linux namespaces / macOS seatbelt |
| 5 | wasm capability sandbox | imports only what host SPI exposes |

The safety layer in `crates/roko-agent/src/safety/` already handles
tier 3. Tier 5 needs a new subcrate — `roko-wasm-host` — that
implements the host interface and enforces limits.

## 9. Extension invariants the Roko core must honor

For the plugin story to work, the core must:

1. **Never break trait contracts without a semver bump.**
2. **Never change persistent data formats without a migration.**
3. **Always emit events plugins can subscribe to** (Bus).
4. **Always expose a read-only Substrate view** (no hidden state
   that plugins can't see).
5. **Always report its own version and capabilities** so plugins
   can feature-detect.

These are cheap to commit to now, expensive to retrofit later.

## 10. Example flows

### 10.1 Adding a company-specific gate

A team wants a gate that checks "PRs always include a `Closes #N`
reference." They write a tier-4 Rust crate implementing `Gate`,
drop it in `./plugins/native/gates/`, `roko plugin enable`. The
next run, the gate is part of the pipeline. One afternoon of work.

### 10.2 Adding a domain-specific tool

A team using Kubernetes wants `kubectl_apply` with policy
enforcement. They write a tier-3 TOML manifest wrapping a shell
script. Drop into `./plugins/tools/`. Roles that need it list it
in `role_allow`. Done.

### 10.3 Adding a new Role

A team wants a "Compliance Reviewer" role with specific templates
and a custom scorer axis. They write:

- `plugins/prompts/compliance_reviewer.md` (tier 1)
- `plugins/tools/compliance_check.toml` (tier 3)
- `plugins/native/scorers/compliance_axis` (tier 4)

All three get discovered and wired. No core PR.

## 11. Why this is a moat

An agent framework's long-term value is measured in its
ecosystem, not its core code. OpenAI's API is valuable because of
what people build on it. Rust is valuable because of its crates.
The Linux kernel is valuable because of its drivers.

Roko's moat, five years out, is not the Substrate or the Bus; it's
the *thousand company-specific gates, tools, and roles* that got
written because the SPI was stable and the risk/power tiers were
well-chosen. Investing now in a clean extension story compounds
(see `15-exponential-scaling.md` §2.7).

## 12. Implementation staging

- **Stage A** (weeks 1–2): Tier 3 tool manifests + discovery.
  Biggest immediate value.
- **Stage B** (weeks 2–3): Tier 1 + Tier 2 prompt/profile plugins.
  Docs-heavy but mostly data.
- **Stage C** (weeks 3–5): Tier 4 ABI bridge + `roko plugin` CLI.
- **Stage D** (month 2+): Tier 5 WASM host.
- **Stage E** (month 3+): Registry.

After Stage A, "install a plugin" is a real user action. After
Stage D, serious third-party development is safe. After Stage E,
we have a real ecosystem.

## 13. WASM host surface (Tier 5 in depth)

The tier-5 WASM sandbox is the tier most likely to unlock third-party
development at scale. A few specifics on what the host imports look
like:

```rust
// roko-wasm-host/src/abi.rs (new crate)
// All function signatures WASM-stable. Extensions compile against these.

pub mod host {
    /// Read an Engram by hash. Returns length; caller pre-allocates buf.
    pub fn engram_get(hash_ptr: u32, hash_len: u32, buf_ptr: u32, buf_cap: u32) -> i64;

    /// Publish a Pulse. Returns the sequence number or an error code.
    pub fn bus_publish(pulse_ptr: u32, pulse_len: u32) -> i64;

    /// Subscribe to a topic filter. Returns a subscription handle.
    pub fn bus_subscribe(filter_ptr: u32, filter_len: u32) -> i32;

    /// Receive the next Pulse for a subscription. Returns length;
    /// zero = no Pulse ready; negative = error.
    pub fn bus_recv(sub_handle: i32, buf_ptr: u32, buf_cap: u32) -> i64;

    /// HDC similarity query against Substrate.
    pub fn substrate_query_similar(fp_ptr: u32, radius_bits: u32, limit: u32,
                                   out_ptr: u32, out_cap: u32) -> i64;

    /// Typed logging back to the host.
    pub fn log(level: u32, msg_ptr: u32, msg_len: u32);

    /// Request a wall-clock timestamp (milliseconds since epoch).
    pub fn now_ms() -> i64;
}
```

Everything the extension does goes through these imports. No file
system, no network, no arbitrary syscalls. The host enforces:

- CPU budget per call (default 100 ms wall clock).
- Memory limit (default 64 MB per instance).
- Pulse publish rate limit (100 pulses/sec default; tunable).
- Substrate query limit (100/sec default).
- Pulse body size limit (64 KB default).

Violations kill the instance. Host publishes `plugin.violation` Pulses
so operators see what happened.

## 14. Permission manifests (all tiers)

Every plugin declares its required permissions in its manifest. The
host honors declared permissions and refuses to grant anything
outside them:

```toml
# plugins/native/my_gate/manifest.toml
id = "org.example.my_gate"
version = "0.2.1"
tier = "native"
capabilities = [ "Gate{rungs=[\"style\"]}" ]

[permissions]
network     = false
files_read  = ["**/*.rs"]
files_write = []
bus_topics_subscribe = ["gate.verdict.emitted"]
bus_topics_publish = ["gate.failed.org.example.my_gate"]
substrate_kinds_read  = ["GateVerdict"]
substrate_kinds_write = ["GateVerdict"]
hdc = false
env_vars = []
```

Manifests enable static analysis: `roko plugin audit` can report
any plugin that requests network, file-write, or broad bus access,
and the operator can decide before installing. This supports the
safety story in `32-safety-sandbox-provenance.md`.

## 15. Dogfooding the SPI

A good test of the SPI: can Roko's own built-in tools, gates, and
scorers be *rewritten* against the same SPI the plugin ecosystem
uses? If not, the SPI is hiding functionality that third parties
will inevitably demand.

Recommendation: after Stage D, port three built-ins (one tool, one
gate, one scorer) to the plugin SPI and dogfood. This catches SPI
gaps while the team still remembers why the internal APIs exist.

## 16. Cross-references

- Domain profiles from `25-domain-specific-agents.md` are the
  largest consumer of Tier-2 profile bundles.
- Developer UX from `22-developer-ux-rust.md` §2.3 covers what
  Tier-4 native extensions see from a Rust author's side.
- Web UI custom tiles (`29-web-ui-architecture.md` §11.1) use the
  plugin mechanism for front-end extensions.
- Custom projections in StateHub (`26-statehub-rearchitecture.md`
  §12) are Tier-4 extensions registering against the projection
  trait.
- The safety story that Tier-3 and Tier-5 rely on lives in
  `32-safety-sandbox-provenance.md`.
- Marketplace / registry items will also appear in
  `33-observability-telemetry.md` — install-count, version uptake,
  security-issue reports surface as dashboard metrics.
