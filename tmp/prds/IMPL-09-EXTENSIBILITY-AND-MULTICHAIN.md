# IMPL-09: Extensibility, multi-chain agents, and predictive foraging

**Implements:** PRD-09 (Extensibility and multi-chain)
**Status:** Draft
**Date:** 2026-04-21
**Estimated effort:** 14-18 weeks across 9 phases

---

## Context

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/` with 18 crates. This document specifies every task required to build the package ecosystem (`roko install`), Pi-compatibility layer (QuickJS bridge), multi-domain agent composition, multi-chain ingestion architecture, contract discovery pipeline, predictive foraging model, and dynamic worldview building from PRD-09.

These six capabilities form one connected system. The package ecosystem lets anyone add chain connectors and domain profiles. Multi-domain composition lets an agent use several profiles at once. Multi-chain ingestion feeds the agent data from many sources. Foraging decides which sources deserve attention. The worldview accumulates what the agent discovers.

### Workspace layout

| Crate | Path | Role in this plan |
|-------|------|-------------------|
| `roko-core` | `crates/roko-core/` | Extend `DomainProfile`, add `ChainConnector` and `PackageManifest` types |
| `roko-plugin` | `crates/roko-plugin/` | Existing plugin SDK; extend with package registry types |
| `roko-chain` | `crates/roko-chain/` | Existing chain primitives; extend with multi-chain actor model |
| `roko-cli` | `crates/roko-cli/` | New `install`, `remove`, `list`, `publish`, `search` subcommands |
| `roko-compose` | `crates/roko-compose/` | WorldGraph context injection via VCG bidding |
| `roko-learn` | `crates/roko-learn/` | Foraging bandit integration |
| `roko-runtime` | `crates/roko-runtime/` | Multi-actor chain subscription management |
| `roko-agent` | `crates/roko-agent/` | Composed profile dispatch |
| `roko-orchestrator` | `crates/roko-orchestrator/` | Domain-routed gate selection |

### New crates introduced by this plan

| Crate | Path | Purpose |
|-------|------|---------|
| `roko-ext-registry` | `crates/roko-ext-registry/` | Package manifest, lockfile, download, install/remove lifecycle |
| `roko-quickjs` | `crates/roko-quickjs/` | Embedded QuickJS runtime, Pi API bridge, sandbox enforcement |
| `roko-chain-ingest` | `crates/roko-chain-ingest/` | Multi-chain actor model, canonical event bus, finality tracking |
| `roko-foraging` | `crates/roko-foraging/` | Gittins index computation, attention budget, habituation masking |
| `roko-worldgraph` | `crates/roko-worldgraph/` | Entity graph, relationship extraction, HDC fingerprinting, WG context |

### What already exists

Read these files before writing anything. Duplicate implementations are the number one failure mode in this codebase.

**Package system foundations:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-plugin/src/manifest.rs` -- `PluginManifestFile` TOML schema, `PluginMeta`, `DeclarativeTool`, `TriggerDef`, `load_manifest()`, `discover_plugins()`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-plugin/src/lib.rs` -- `EventSource` trait, `FeedbackCollector` trait, `PluginManifest`, `PluginBuilder`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/tool/registry.rs` -- `ToolRegistry` trait, `VecToolRegistry`, `for_role()`, `for_call()` progressive discovery
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/tool/def.rs` -- `ToolDef` struct, `ToolCategory`, `ToolPermission`

**Domain profiles:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/domain_profile.rs` -- `DomainProfile` enum (Coding/Research/Chain/DataMl/Ops/Writing), `TypedContext`, `default_gate_rungs()`, `tool_categories()`, `context_fraction()`

**Chain primitives:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/client.rs` -- `ChainClient` trait (block_number, get_block_header, get_receipt, get_logs, get_storage_at, eth_call, chain_id)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/wallet.rs` -- `ChainWallet` trait
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/types.rs` -- `BlockNumber`, `TxHash`, `ChainHeader`, `LogEntry`, `Receipt`, `ChainError`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/observer.rs` -- `BlockObserver`, `AddressFilter`, `BlockTracker`, `ObservedEvent`, gap detection
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/alloy_impl.rs` -- Alloy-backed `ChainClient` (feature-gated)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/triage.rs` -- `TriagePipeline`, `MidasRScorer`, `TriageConfig`

**Attention and context:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/lib.rs` -- `AttentionBidder` variants (Neuro/Task/Research), `PromptComposer`, VCG allocation
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` -- `RokoConfig`, `AttentionConfig`, `AgentConfig`, all TOML sections

**CLI entry point:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` -- `Cli` struct, `Subcommand` enum, 35+ existing subcommands

**Orchestration:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` -- `dispatch_agent_with()`, `enrich_rung_config()`, domain-aware orchestration

**Learning subsystem:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/` -- episodes, playbooks, bandits, efficiency logging

---

## Phase 1: Package registry foundation

Goal: define the package manifest types, storage layout, lockfile format, and the core install/remove lifecycle. No CLI yet -- this phase builds the library that the CLI will call.

### Task 1.1: Create `roko-ext-registry` crate skeleton

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (workspace members list, workspace dependencies)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-plugin/Cargo.toml` (dependency pattern for plugin crates)

**Implementation:**
1. Create directory `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/`
2. Create `Cargo.toml` with dependencies: `roko-core`, `serde`, `serde_json`, `toml`, `anyhow`, `thiserror`, `reqwest` (for downloads), `flate2` + `tar` (for archive extraction), `sha2` (for integrity verification), `semver` (for version resolution)
3. Create `src/lib.rs` with module declarations: `manifest`, `lockfile`, `storage`, `resolver`, `installer`, `registry`
4. Add `"crates/roko-ext-registry"` to the `[workspace].members` array in the root `Cargo.toml`
5. Run `cargo check -p roko-ext-registry` to verify the skeleton compiles

**Test:** `cargo check -p roko-ext-registry` exits 0.

### Task 1.2: Define `PackageManifest` struct

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-plugin/src/manifest.rs` (existing `PluginManifestFile`, `PluginMeta` -- do NOT duplicate; extend)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/domain_profile.rs` (domain profile enum)

**Implementation:**
1. In `crates/roko-ext-registry/src/manifest.rs`, define:
   ```rust
   /// Source type for a package installation.
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(tag = "type", rename_all = "snake_case")]
   pub enum PackageSource {
       /// Rust crate from crates.io or a git repository.
       Crate { name: String, version: Option<String>, git: Option<String> },
       /// npm package (Pi-compatible or Roko-native JS/TS).
       Npm { name: String, version: Option<String> },
       /// Git repository (auto-detect type from contents).
       Git { url: String, rev: Option<String>, branch: Option<String> },
       /// Local path (symlinked, not copied).
       Local { path: PathBuf },
   }
   ```
2. Define `PackageKind` enum: `Extension`, `Skill`, `Prompt`, `Theme`, `DomainProfile`, `ChainConnector`, `Arena`, `Composite`
3. Define `PackageManifest` struct:
   ```rust
   pub struct PackageManifest {
       pub name: String,
       pub version: semver::Version,
       pub kind: PackageKind,
       pub source: PackageSource,
       pub description: Option<String>,
       pub author: Option<String>,
       pub license: Option<String>,
       pub capabilities: Vec<Capability>,    // declared permissions
       pub dependencies: Vec<PackageDep>,
       pub roko_version: Option<semver::VersionReq>, // minimum roko version
   }
   ```
4. Define `Capability` enum: `Network`, `FileSystem`, `Exec`, `ChainRpc`, `ToolRegister`, `ContextInject`
5. Define `PackageDep` struct: `name: String`, `version: semver::VersionReq`
6. Implement `PackageManifest::from_plugin_manifest(plugin: &PluginManifestFile) -> Self` to convert existing plugin manifests
7. Implement `PackageManifest::from_cargo_toml(content: &str) -> Result<Self>` to parse `[package.metadata.roko]` sections
8. Implement `PackageManifest::from_package_json(content: &str) -> Result<Self>` to parse npm packages with a `"roko"` or `"pi"` key
9. Write unit tests: roundtrip serde for each source type, conversion from each input format

**Test:** `cargo test -p roko-ext-registry -- manifest` passes.

### Task 1.3: Implement lockfile format

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/manifest.rs` (the manifest you just wrote)

**Implementation:**
1. In `crates/roko-ext-registry/src/lockfile.rs`, define:
   ```rust
   pub struct Lockfile {
       pub version: u32,
       pub packages: Vec<LockedPackage>,
   }

   pub struct LockedPackage {
       pub name: String,
       pub version: semver::Version,
       pub kind: PackageKind,
       pub source: PackageSource,
       pub integrity: String,        // sha256 hash of installed content
       pub installed_at: DateTime<Utc>,
       pub scope: PackageScope,      // Global or Project
   }

   pub enum PackageScope {
       Global,
       Project,
   }
   ```
2. Implement `Lockfile::load(path: &Path) -> Result<Self>` (reads TOML from `.roko/packages.lock`)
3. Implement `Lockfile::save(&self, path: &Path) -> Result<()>`
4. Implement `Lockfile::add(&mut self, pkg: LockedPackage)` -- replaces existing entry with same name
5. Implement `Lockfile::remove(&mut self, name: &str) -> Option<LockedPackage>`
6. Implement `Lockfile::find(&self, name: &str) -> Option<&LockedPackage>`
7. Write tests: save/load roundtrip, add replaces existing, remove returns removed entry

**Test:** `cargo test -p roko-ext-registry -- lockfile` passes.

### Task 1.4: Implement package storage layout

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-fs/src/layout.rs` (existing `RokoLayout` for `.roko/` directory structure)

**Implementation:**
1. In `crates/roko-ext-registry/src/storage.rs`, define:
   ```rust
   pub struct PackageStorage {
       global_root: PathBuf,   // ~/.roko/packages/
       project_root: PathBuf,  // .roko/packages/
   }
   ```
2. Implement `PackageStorage::new(global: PathBuf, project: PathBuf) -> Self`
3. Implement `PackageStorage::from_defaults() -> Self` using `dirs::home_dir()` for global
4. Implement `PackageStorage::package_dir(&self, name: &str, scope: PackageScope) -> PathBuf`
5. Implement `PackageStorage::ensure_dirs(&self) -> Result<()>` -- creates both roots if missing
6. Implement `PackageStorage::list_installed(&self, scope: PackageScope) -> Result<Vec<String>>` -- reads subdirectory names
7. Implement `PackageStorage::is_installed(&self, name: &str) -> bool` -- checks both scopes, project overrides global
8. Implement `PackageStorage::resolve(&self, name: &str) -> Option<(PathBuf, PackageScope)>` -- project-first lookup
9. Write tests: directory creation, list empty, install detection across scopes

**Test:** `cargo test -p roko-ext-registry -- storage` passes.

### Task 1.5: Implement package resolver

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/manifest.rs` (PackageSource, PackageDep)

**Implementation:**
1. In `crates/roko-ext-registry/src/resolver.rs`, define:
   ```rust
   pub struct ResolvedPackage {
       pub manifest: PackageManifest,
       pub source: ResolvedSource,
   }

   pub enum ResolvedSource {
       CratesIo { name: String, version: semver::Version, download_url: String },
       Npm { name: String, version: semver::Version, tarball_url: String },
       Git { url: String, rev: String },
       Local { path: PathBuf },
   }
   ```
2. Implement `resolve_crate(name: &str, version: Option<&str>) -> Result<ResolvedSource>` -- queries crates.io API for latest compatible version, returns download URL
3. Implement `resolve_npm(name: &str, version: Option<&str>) -> Result<ResolvedSource>` -- queries npm registry API, returns tarball URL
4. Implement `resolve_git(url: &str, rev: Option<&str>, branch: Option<&str>) -> Result<ResolvedSource>` -- resolves branch/tag to a specific commit SHA
5. Implement `resolve_local(path: &Path) -> Result<ResolvedSource>` -- verifies path exists, resolves to absolute
6. Implement top-level `resolve(source: &PackageSource) -> Result<ResolvedSource>`
7. Write tests: local resolution with existing path, local resolution with missing path returns error

**Test:** `cargo test -p roko-ext-registry -- resolver` passes. (Network tests gated behind `#[ignore]` or mock HTTP.)

### Task 1.6: Implement package installer

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/storage.rs` (PackageStorage)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/resolver.rs` (ResolvedSource)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/lockfile.rs` (Lockfile)

**Implementation:**
1. In `crates/roko-ext-registry/src/installer.rs`, define:
   ```rust
   pub struct Installer {
       storage: PackageStorage,
       lockfile_path: PathBuf,
   }
   ```
2. Implement `Installer::new(storage: PackageStorage, lockfile_path: PathBuf) -> Self`
3. Implement `Installer::install(&self, source: &PackageSource, scope: PackageScope) -> Result<InstalledPackage>`:
   - Call `resolve(source)` to get `ResolvedSource`
   - Match on `ResolvedSource`:
     - `CratesIo` -> download tarball via reqwest, extract to `storage.package_dir()`, run `cargo build --release` if Rust crate
     - `Npm` -> download tarball via reqwest, extract to storage dir
     - `Git` -> `git clone --depth 1` into storage dir, checkout rev
     - `Local` -> create symlink from storage dir to local path
   - Parse manifest from extracted contents (try `plugin.toml`, then `Cargo.toml`, then `package.json`)
   - Compute sha256 integrity hash of installed content
   - Update lockfile: load, add entry, save
   - Return `InstalledPackage { manifest, path, scope, integrity }`
4. Implement `Installer::remove(&self, name: &str) -> Result<()>`:
   - Find package in lockfile
   - Delete directory (or remove symlink for local)
   - Update lockfile: remove entry, save
5. Implement `Installer::list(&self) -> Result<Vec<InstalledPackage>>`:
   - Load lockfile
   - For each entry, verify directory still exists
   - Return list with resolved paths
6. Write tests: install from local path creates symlink, remove cleans up, list reflects state

**Test:** `cargo test -p roko-ext-registry -- installer` passes.

### Task 1.7: Implement package registry API client

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/resolver.rs` (resolver structure)

**Implementation:**
1. In `crates/roko-ext-registry/src/registry.rs`, define:
   ```rust
   pub struct RegistryClient {
       base_url: String,
       http: reqwest::Client,
   }

   pub struct SearchResult {
       pub name: String,
       pub version: String,
       pub description: Option<String>,
       pub kind: PackageKind,
       pub downloads: u64,
   }
   ```
2. Implement `RegistryClient::new(base_url: &str) -> Self`
3. Implement `RegistryClient::search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>`
4. Implement `RegistryClient::publish(&self, manifest: &PackageManifest, tarball: &[u8], token: &str) -> Result<()>`
5. Implement `RegistryClient::info(&self, name: &str) -> Result<PackageManifest>`
6. For the initial implementation, the registry is optional. Fallback to crates.io and npm directly.
7. Write tests: mock HTTP responses for search and info

**Test:** `cargo test -p roko-ext-registry -- registry` passes.

---

## Phase 2: CLI package commands

Goal: wire the `roko-ext-registry` library into the CLI with `roko install`, `roko remove`, `roko list`, `roko search`, and `roko publish` subcommands.

### Task 2.1: Add `PackageCmd` subcommand enum

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (existing `Subcommand` enum, clap derive pattern)

**Implementation:**
1. In `crates/roko-cli/src/main.rs`, add new variants to the `Subcommand` enum:
   ```rust
   /// Install a package (extension, skill, prompt, chain connector, etc.)
   Install {
       /// Package specifier: crate:<name>, npm:<name>, git:<url>, or <local-path>
       specifier: String,
       /// Install globally instead of project-local
       #[arg(long)]
       global: bool,
   },
   /// Remove an installed package
   Remove {
       /// Package name to remove
       name: String,
   },
   /// List installed packages
   #[command(name = "ls")]
   List {
       /// Show only global packages
       #[arg(long)]
       global: bool,
       /// Show only project-local packages
       #[arg(long)]
       local: bool,
   },
   /// Search the package registry
   Search {
       /// Search query
       query: String,
       /// Maximum results to show
       #[arg(long, default_value = "20")]
       limit: usize,
   },
   /// Publish a package to the roko registry
   Publish {
       /// Path to the package directory (default: current directory)
       #[arg(default_value = ".")]
       path: PathBuf,
   },
   ```
2. Add `roko-ext-registry` to `crates/roko-cli/Cargo.toml` dependencies
3. Stub match arms for each command that call into the registry library

**Test:** `cargo build -p roko-cli` succeeds. `cargo run -p roko-cli -- install --help` prints usage.

### Task 2.2: Implement `roko install` specifier parser

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/manifest.rs` (PackageSource enum)

**Implementation:**
1. In `crates/roko-cli/src/package.rs` (new file), implement:
   ```rust
   pub fn parse_specifier(specifier: &str) -> Result<PackageSource> {
       if let Some(name) = specifier.strip_prefix("crate:") {
           // Parse "crate:serde" or "crate:serde@1.0"
           let (name, version) = split_at_version(name);
           Ok(PackageSource::Crate { name, version, git: None })
       } else if let Some(name) = specifier.strip_prefix("npm:") {
           let (name, version) = split_at_version(name);
           Ok(PackageSource::Npm { name, version })
       } else if let Some(url) = specifier.strip_prefix("git:") {
           Ok(PackageSource::Git { url, rev: None, branch: None })
       } else if Path::new(specifier).exists() {
           Ok(PackageSource::Local { path: PathBuf::from(specifier) })
       } else {
           // Try as crate name first, fall back to npm
           Ok(PackageSource::Crate { name: specifier.to_string(), version: None, git: None })
       }
   }
   ```
2. Implement `split_at_version(s: &str) -> (String, Option<String>)` -- splits on `@` if present
3. Write unit tests: each prefix type, version parsing, local path detection, bare name fallback

**Test:** `cargo test -p roko-cli -- package::parse_specifier` passes.

### Task 2.3: Wire `roko install` execution path

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/package.rs` (specifier parser from 2.2)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/installer.rs` (Installer)

**Implementation:**
1. In the `Install` match arm of `main.rs`:
   - Parse `specifier` via `parse_specifier()`
   - Determine scope from `--global` flag
   - Construct `PackageStorage::from_defaults()` or project-rooted storage
   - Construct `Installer`
   - Call `installer.install(&source, scope).await?`
   - Print success message with package name, version, kind, install path
2. Handle error cases: network failures (retry once), integrity mismatches (delete and retry), existing package (update or skip)
3. Print progress: "Resolving...", "Downloading...", "Installing...", "Done."

**Test:** `cargo run -p roko-cli -- install ./path/to/local/extension` installs a local extension. Verify with `roko ls`.

### Task 2.4: Wire `roko remove` execution path

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/installer.rs` (Installer::remove)

**Implementation:**
1. In the `Remove` match arm:
   - Construct `Installer`
   - Call `installer.remove(&name)?`
   - Print success: "Removed package `{name}`"
   - Handle not-found: "Package `{name}` is not installed"

**Test:** Install a local package, then `roko remove <name>`. Verify the directory and lockfile entry are gone.

### Task 2.5: Wire `roko ls` execution path

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/installer.rs` (Installer::list)

**Implementation:**
1. In the `List` match arm:
   - Construct `Installer`
   - Call `installer.list()?`
   - Filter by `--global` / `--local` flags if set
   - Print table: name, version, kind, scope, path
   - If no packages: "No packages installed."

**Test:** Install two packages (one global, one local). `roko ls` shows both. `roko ls --global` shows one. `roko ls --local` shows one.

### Task 2.6: Wire `roko search` and `roko publish` execution paths

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/registry.rs` (RegistryClient)

**Implementation:**
1. In the `Search` match arm:
   - Construct `RegistryClient`
   - Call `client.search(&query, limit).await?`
   - Print results table: name, version, kind, description (truncated to 60 chars), downloads
   - If no results: "No packages found for `{query}`"
2. In the `Publish` match arm:
   - Read manifest from path
   - Create tarball of package contents
   - Read auth token from `~/.roko/auth-token` or `ROKO_REGISTRY_TOKEN` env
   - Call `client.publish(&manifest, &tarball, &token).await?`
   - Print: "Published `{name}@{version}`"

**Test:** `roko search <query>` returns results (or gracefully handles no registry). `roko publish .` reads manifest.

---

## Phase 3: QuickJS bridge for Pi compatibility

Goal: embed a QuickJS runtime that executes JavaScript/TypeScript extensions and bridges the Pi API surface into Roko's tool registry and event system.

### Task 3.1: Create `roko-quickjs` crate skeleton

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (workspace members)

**Implementation:**
1. Create directory `/Users/will/dev/nunchi/roko/roko/crates/roko-quickjs/`
2. Create `Cargo.toml` with dependencies: `roko-core`, `roko-plugin`, `roko_quickjs` (the `rquickjs` crate for embedding QuickJS), `serde`, `serde_json`, `anyhow`, `tokio`
3. Create `src/lib.rs` with module declarations: `runtime`, `bridge`, `sandbox`, `pi_api`
4. Add to workspace members in root `Cargo.toml`

**Test:** `cargo check -p roko-quickjs` exits 0.

### Task 3.2: Implement QuickJS runtime wrapper

**Read first:**
- `rquickjs` crate docs (https://docs.rs/rquickjs)

**Implementation:**
1. In `crates/roko-quickjs/src/runtime.rs`, define:
   ```rust
   pub struct JsRuntime {
       runtime: rquickjs::Runtime,
       context: rquickjs::Context,
   }
   ```
2. Implement `JsRuntime::new(config: SandboxConfig) -> Result<Self>`:
   - Create QuickJS runtime with memory limit from config (default: 64MB)
   - Create context with console, setTimeout stubs
   - Set max stack size from config (default: 1MB)
3. Implement `JsRuntime::eval(&self, code: &str) -> Result<serde_json::Value>`:
   - Evaluate JS code in the context
   - Convert result to JSON
   - Handle exceptions by mapping to `RokoError`
4. Implement `JsRuntime::call(&self, fn_name: &str, args: &[serde_json::Value]) -> Result<serde_json::Value>`
5. Define `SandboxConfig`:
   ```rust
   pub struct SandboxConfig {
       pub memory_limit_bytes: usize,  // default 64MB
       pub max_stack_bytes: usize,     // default 1MB
       pub timeout_ms: u64,            // default 5000
       pub allow_network: bool,        // default false
       pub allow_fs: bool,             // default false
   }
   ```
6. Write tests: eval simple expression, eval throws error, memory limit enforcement

**Test:** `cargo test -p roko-quickjs -- runtime` passes.

### Task 3.3: Implement Pi API bridge

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/tool/def.rs` (ToolDef, ToolCategory, ToolPermission)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/tool/registry.rs` (ToolRegistry, VecToolRegistry)
- PRD-09 section 3 (Pi-compatible API surface)

**Implementation:**
1. In `crates/roko-quickjs/src/pi_api.rs`, define the Rust-side state for a JS extension:
   ```rust
   pub struct PiApiState {
       pub registered_tools: Vec<ToolDef>,
       pub event_handlers: HashMap<String, Vec<JsFunction>>,
       pub config: HashMap<String, serde_json::Value>,
       pub logs: Vec<LogEntry>,
   }
   ```
2. Implement JS-callable functions that manipulate `PiApiState`:
   - `pi_register_tool(name, description, schema, handler)` -> pushes to `registered_tools`
   - `pi_on(event, handler)` -> pushes handler to `event_handlers[event]`
   - `pi_get_config()` -> returns config as JSON
   - `pi_set_config(key, value)` -> updates config
   - `pi_log(level, message)` -> appends to logs
3. In `crates/roko-quickjs/src/bridge.rs`, define:
   ```rust
   pub struct PiBridge {
       runtime: JsRuntime,
       state: Arc<Mutex<PiApiState>>,
   }
   ```
4. Implement `PiBridge::load_extension(path: &Path) -> Result<Self>`:
   - Create new `JsRuntime`
   - Inject `pi` global object with all `pi_*` functions bound
   - Read and eval the extension's entry point (`index.js` or `main.js`)
   - Return bridge with populated state
5. Implement `PiBridge::registered_tools(&self) -> Vec<ToolDef>`:
   - Convert JS tool registrations to `ToolDef` structs
   - Set `ToolCategory::Custom` and `ToolPermission` from declared capabilities
6. Implement `PiBridge::fire_event(&self, event: &str, payload: serde_json::Value) -> Result<()>`:
   - Look up handlers for event
   - Call each handler with payload
7. Write tests: load a minimal JS extension that registers one tool, verify tool appears in registered_tools

**Test:** `cargo test -p roko-quickjs -- bridge` passes.

### Task 3.4: Implement Pi context API

**Read first:**
- PRD-09 section 7 (QuickJS bridge context API: `ctx.cwd`, `ctx.ui`, `ctx.model`)

**Implementation:**
1. In `crates/roko-quickjs/src/pi_api.rs`, extend the `pi` global object with a `ctx` sub-object:
   - `ctx.cwd` -> returns current working directory as string
   - `ctx.projectRoot` -> returns project root (where `.roko/` lives)
   - `ctx.model` -> returns current model name string
   - `ctx.role` -> returns current agent role string
   - `ctx.env(key)` -> returns environment variable (filtered by sandbox config)
2. These values are set when the bridge is constructed and do not change during execution
3. Write tests: ctx.cwd returns a valid path, ctx.model returns the configured model

**Test:** `cargo test -p roko-quickjs -- pi_api::ctx` passes.

### Task 3.5: Implement skill loading from markdown

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/` (existing template format)
- PRD-09 section 3 (SKILL.md format)

**Implementation:**
1. In `crates/roko-ext-registry/src/skill_loader.rs` (new file), define:
   ```rust
   pub struct LoadedSkill {
       pub name: String,
       pub description: Option<String>,
       pub trigger: Option<String>,    // regex or keyword that activates the skill
       pub content: String,
   }
   ```
2. Implement `load_skill(path: &Path) -> Result<LoadedSkill>`:
   - Read file content
   - Parse YAML frontmatter between `---` delimiters
   - Extract `name`, `description`, `trigger` from frontmatter
   - Remaining content is the skill body
3. Implement `load_skills_dir(dir: &Path) -> Result<Vec<LoadedSkill>>`:
   - Scan for `*.md` files
   - Load each, skip files without valid frontmatter (log warning)
4. Write tests: parse valid SKILL.md, handle missing frontmatter, handle empty file

**Test:** `cargo test -p roko-ext-registry -- skill_loader` passes.

### Task 3.6: Implement prompt template loading

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-plugin/src/manifest.rs` (existing `PromptTemplate` struct)

**Implementation:**
1. In `crates/roko-ext-registry/src/prompt_loader.rs` (new file), define:
   ```rust
   pub struct LoadedPrompt {
       pub name: String,
       pub role: Option<String>,
       pub template: String,
       pub variables: Vec<String>,   // extracted from {{variable}} placeholders
   }
   ```
2. Implement `load_prompt(path: &Path) -> Result<LoadedPrompt>`:
   - Read file content
   - Parse YAML frontmatter for `name`, `role`
   - Extract `{{variable}}` placeholders from body via regex
   - Return `LoadedPrompt`
3. Implement `render_prompt(template: &LoadedPrompt, vars: &HashMap<String, String>) -> Result<String>`:
   - Replace each `{{key}}` with the corresponding value
   - Return error for unresolved variables
4. Write tests: extraction of variables, rendering with all variables provided, error on missing variable

**Test:** `cargo test -p roko-ext-registry -- prompt_loader` passes.

### Task 3.7: Implement theme loading

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/theme.rs` (existing TUI theme system)

**Implementation:**
1. In `crates/roko-ext-registry/src/theme_loader.rs` (new file), define:
   ```rust
   pub struct LoadedTheme {
       pub name: String,
       pub colors: HashMap<String, String>,  // key -> hex color
   }
   ```
2. Implement `load_theme(path: &Path) -> Result<LoadedTheme>`:
   - Parse JSON file
   - Validate color values are valid hex
3. Implement `LoadedTheme::to_tui_theme(&self) -> Result<roko_cli::tui::Theme>`:
   - Map color keys to the existing theme struct fields
   - Unknown keys are ignored (forward compatibility)
4. Write tests: parse valid theme, reject invalid hex colors

**Test:** `cargo test -p roko-ext-registry -- theme_loader` passes.

---

## Phase 4: Roko-native extension types

Goal: implement loading for cognitive extensions (Rust crates), domain profiles (TOML), arena definitions, and chain connectors -- the package types that go beyond Pi compatibility.

### Task 4.1: Implement cognitive extension loading via Rust dynamic libraries

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-plugin/src/lib.rs` (EventSource trait, PluginManifest)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/traits.rs` (Gate, Scorer, Policy traits)

**Implementation:**
1. In `crates/roko-ext-registry/src/extension_loader.rs` (new file), define:
   ```rust
   /// A Rust extension loaded from a compiled crate.
   pub struct LoadedExtension {
       pub manifest: PackageManifest,
       pub tools: Vec<ToolDef>,
       pub event_sources: Vec<Box<dyn EventSource>>,
       pub feedback_collectors: Vec<Box<dyn FeedbackCollector>>,
   }
   ```
2. For the initial implementation, cognitive extensions are compiled as part of the workspace. They expose a `pub fn roko_extension() -> PluginManifest` entry point.
3. Implement `load_rust_extension(crate_dir: &Path) -> Result<LoadedExtension>`:
   - Run `cargo build --release` in the crate directory
   - Load the resulting `cdylib` via `libloading`
   - Call the `roko_extension` symbol to get the manifest
   - Extract tools, event sources, feedback collectors
4. For safety: cdylib loading is feature-gated behind `#[cfg(feature = "dynamic-extensions")]`. Without the feature, only statically linked extensions are supported.
5. Write tests: load a test extension crate from `tests/fixtures/dummy-extension/`

**Test:** `cargo test -p roko-ext-registry --features dynamic-extensions -- extension_loader` passes.

### Task 4.2: Implement domain profile loading from TOML

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/domain_profile.rs` (existing `DomainProfile` enum, `TypedContext`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` (RokoConfig)

**Implementation:**
1. In `crates/roko-ext-registry/src/profile_loader.rs` (new file), define:
   ```rust
   /// A domain profile loaded from a TOML package.
   pub struct LoadedDomainProfile {
       pub name: String,
       pub base_domain: Option<DomainProfile>,  // extend existing domain, or standalone
       pub gate_rungs: Vec<String>,
       pub tool_categories: Vec<String>,
       pub context_fraction: f64,
       pub extensions: Vec<String>,      // names of extensions to load
       pub event_subscriptions: Vec<String>,
       pub metadata: HashMap<String, String>,
   }
   ```
2. Implement `load_domain_profile(path: &Path) -> Result<LoadedDomainProfile>`:
   - Parse TOML file
   - Validate gate rung names against known gates
   - Validate tool categories against known categories
   - Return `LoadedDomainProfile`
3. Implement `LoadedDomainProfile::to_typed_context(&self) -> TypedContext`:
   - Convert to existing `TypedContext` struct for integration with orchestrate.rs
4. Write tests: parse valid profile TOML, roundtrip, validate gate rungs

**Test:** `cargo test -p roko-ext-registry -- profile_loader` passes.

### Task 4.3: Implement arena definition loading

**Read first:**
- PRD-09 section 3 (arena definitions)

**Implementation:**
1. In `crates/roko-ext-registry/src/arena_loader.rs` (new file), define:
   ```rust
   pub struct LoadedArena {
       pub name: String,
       pub description: String,
       pub tasks: Vec<ArenaTask>,
       pub scoring: ScoringConfig,
       pub time_limit_secs: u64,
   }

   pub struct ArenaTask {
       pub prompt: String,
       pub expected_artifacts: Vec<String>,  // files that should exist after
       pub validation_command: Option<String>,
       pub max_turns: usize,
   }

   pub struct ScoringConfig {
       pub correctness_weight: f64,
       pub efficiency_weight: f64,
       pub style_weight: f64,
   }
   ```
2. Implement `load_arena(path: &Path) -> Result<LoadedArena>`:
   - Parse TOML file
   - Validate scoring weights sum to 1.0 (or normalize)
3. Write tests: parse valid arena, reject invalid weights

**Test:** `cargo test -p roko-ext-registry -- arena_loader` passes.

### Task 4.4: Implement chain connector loading

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/client.rs` (ChainClient trait)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/types.rs` (ChainHeader, LogEntry, BlockNumber)

**Implementation:**
1. In `crates/roko-ext-registry/src/chain_loader.rs` (new file), define:
   ```rust
   /// Configuration for a loaded chain connector.
   pub struct LoadedChainConnector {
       pub chain_id: String,           // e.g., "ethereum", "korai", "hyperliquid"
       pub chain_type: ChainType,
       pub rpc_url: String,
       pub ws_url: Option<String>,
       pub block_time_ms: u64,
       pub finality: FinalityMode,
       pub connector_crate: Option<String>,  // for custom connectors
   }

   pub enum ChainType {
       Evm,
       Hyperliquid,
       Custom(String),
   }

   pub enum FinalityMode {
       Confirmations(u32),
       Deterministic,
       Probabilistic { threshold: f64 },
   }
   ```
2. Implement `load_chain_config(path: &Path) -> Result<Vec<LoadedChainConnector>>`:
   - Parse TOML file with `[[chains]]` array
   - Validate RPC URLs
   - Return list of chain connectors
3. Implement `LoadedChainConnector::to_chain_client(&self) -> Result<Box<dyn ChainClient>>`:
   - For `Evm`: construct AlloyChainClient with the RPC URL
   - For `Hyperliquid`: construct HyperliquidClient (new)
   - For `Custom`: attempt dynamic loading
4. Write tests: parse chain config, validate URL formats

**Test:** `cargo test -p roko-ext-registry -- chain_loader` passes.

---

## Phase 5: Multi-domain agent composition

Goal: enable agents that span multiple domain profiles simultaneously, with merged tool sets, gate pipelines, and context budgets.

### Task 5.1: Implement `ComposedProfile` struct

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/domain_profile.rs` (DomainProfile, TypedContext)

**Implementation:**
1. In `crates/roko-core/src/domain_profile.rs`, add:
   ```rust
   /// A composition of multiple domain profiles for multi-domain agents.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ComposedProfile {
       pub domains: Vec<DomainProfile>,
       pub merge_strategy: MergeStrategy,
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum MergeStrategy {
       /// Union all gate rungs and tool categories. Max context fraction.
       Union,
       /// Use primary domain's defaults, add secondary tools.
       PrimaryWithExtensions,
   }
   ```
2. Implement `ComposedProfile::new(domains: Vec<DomainProfile>, strategy: MergeStrategy) -> Self`
3. Implement `ComposedProfile::effective_gate_rungs(&self) -> Vec<String>`:
   - `Union`: collect all gate rungs from all domains, deduplicate, preserve order
   - `PrimaryWithExtensions`: use first domain's rungs
4. Implement `ComposedProfile::effective_tool_categories(&self) -> Vec<String>`:
   - `Union`: collect all categories, deduplicate
   - `PrimaryWithExtensions`: union of primary + secondary categories
5. Implement `ComposedProfile::effective_context_fraction(&self) -> f64`:
   - `Union`: max of all domains' fractions
   - `PrimaryWithExtensions`: primary domain's fraction
6. Implement `ComposedProfile::to_typed_context(&self) -> TypedContext`:
   - Convert to TypedContext using effective values
   - Set metadata `"composed_from"` to comma-separated domain labels
7. Write tests: union of Coding+Research includes all rungs from both, dedup works, max fraction

**Test:** `cargo test -p roko-core -- domain_profile::composed` passes.

### Task 5.2: Implement `--profile` CLI parsing for multi-domain

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (how `roko run` and `roko plan run` parse arguments)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_serve.rs` (existing `AgentCmd` with `--profile`)

**Implementation:**
1. In `crates/roko-cli/src/main.rs`, update any `--profile` argument from a single string to accept comma-separated values:
   ```rust
   #[arg(long, value_delimiter = ',')]
   profile: Vec<String>,
   ```
2. In the dispatch path, parse profile strings:
   ```rust
   fn parse_profiles(labels: &[String]) -> Result<ComposedProfile> {
       let domains: Vec<DomainProfile> = labels
           .iter()
           .map(|l| DomainProfile::from_label(l)
               .ok_or_else(|| anyhow!("unknown domain profile: {l}")))
           .collect::<Result<Vec<_>>>()?;
       Ok(ComposedProfile::new(domains, MergeStrategy::Union))
   }
   ```
3. Pass the `ComposedProfile` through to the orchestration path where `TypedContext` is consumed
4. Write tests: parse "blockchain,research", reject unknown profiles

**Test:** `cargo run -p roko-cli -- run "test" --profile coding,research` does not error on profile parsing.

### Task 5.3: Implement extension deduplication in composed profiles

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/profile_loader.rs` (LoadedDomainProfile.extensions)

**Implementation:**
1. In `crates/roko-core/src/domain_profile.rs`, add to `ComposedProfile`:
   ```rust
   impl ComposedProfile {
       pub fn deduplicated_extensions(&self, profiles: &[LoadedDomainProfile]) -> Vec<String> {
           let mut seen = HashSet::new();
           let mut result = Vec::new();
           for profile in profiles {
               for ext in &profile.extensions {
                   if seen.insert(ext.clone()) {
                       result.push(ext.clone());
                   }
               }
           }
           result
       }
   }
   ```
2. Write tests: extensions from two profiles are merged without duplicates, order preserves first occurrence

**Test:** `cargo test -p roko-core -- domain_profile::dedup` passes.

### Task 5.4: Implement per-task domain routing in orchestration

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (first 100 lines, `enrich_rung_config()`, `dispatch_agent_with()`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/task.rs` (TaskDomain, TaskCategory)

**Implementation:**
1. In `crates/roko-cli/src/orchestrate.rs`, modify the gate selection logic to route based on task domain:
   ```rust
   fn select_gates_for_task(
       task: &Task,
       composed: &ComposedProfile,
   ) -> Vec<String> {
       match task.domain {
           Some(TaskDomain::Coding) => DomainProfile::Coding.default_gate_rungs()
               .iter().map(|s| s.to_string()).collect(),
           Some(TaskDomain::Chain) => DomainProfile::Chain.default_gate_rungs()
               .iter().map(|s| s.to_string()).collect(),
           Some(TaskDomain::Research) => DomainProfile::Research.default_gate_rungs()
               .iter().map(|s| s.to_string()).collect(),
           _ => composed.effective_gate_rungs(),
       }
   }
   ```
2. Wire this into the existing `enrich_rung_config()` call path so per-task gate rungs are selected before gate execution
3. Write test: coding task gets compile+test+clippy gates, research task gets content_review+citation_check

**Test:** `cargo test -p roko-cli -- orchestrate::domain_routing` passes (unit test with mock tasks).

---

## Phase 6: Multi-chain ingestion architecture

Goal: build the actor-per-chain model that subscribes to multiple chains simultaneously, normalizes events into a canonical format, and feeds a unified event bus.

### Task 6.1: Create `roko-chain-ingest` crate skeleton

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (workspace members, workspace deps)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/Cargo.toml` (chain crate dependencies)

**Implementation:**
1. Create directory `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/`
2. Create `Cargo.toml` with dependencies: `roko-core`, `roko-chain`, `roko-runtime`, `tokio`, `serde`, `serde_json`, `anyhow`, `tracing`, `uuid`, `chrono`, `dashmap`
3. Create `src/lib.rs` with module declarations: `connector`, `actor`, `canonical`, `bus`, `finality`, `temporal`, `config`
4. Add to workspace members

**Test:** `cargo check -p roko-chain-ingest` exits 0.

### Task 6.2: Define `ChainConnector` trait

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/client.rs` (existing ChainClient trait)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/observer.rs` (BlockObserver, ObservedEvent)

**Implementation:**
1. In `crates/roko-chain-ingest/src/connector.rs`, define:
   ```rust
   /// Chain-agnostic connector that normalizes chain events into CanonicalEvents.
   #[async_trait]
   pub trait ChainConnector: Send + Sync {
       /// Unique chain identifier (e.g., "ethereum", "korai").
       fn chain_id(&self) -> &str;

       /// Subscribe to new block events. Returns a stream of raw block data.
       async fn subscribe_blocks(&self) -> Result<Pin<Box<dyn Stream<Item = RawBlock> + Send>>>;

       /// Subscribe to log events matching the given filters.
       async fn subscribe_logs(
           &self,
           addresses: &[String],
           topics: &[String],
       ) -> Result<Pin<Box<dyn Stream<Item = RawLog> + Send>>>;

       /// Fetch historical blocks in a range (for backfill).
       async fn fetch_block_range(
           &self,
           from: u64,
           to: u64,
       ) -> Result<Vec<RawBlock>>;

       /// Current chain tip block number.
       async fn tip(&self) -> Result<u64>;

       /// Expected block time in milliseconds (for timeout calculation).
       fn block_time_ms(&self) -> u64;

       /// Finality configuration for this chain.
       fn finality_mode(&self) -> FinalityMode;

       /// Human-readable name.
       fn name(&self) -> &str;
   }

   pub struct RawBlock {
       pub number: u64,
       pub hash: String,
       pub parent_hash: String,
       pub timestamp: u64,
       pub logs: Vec<RawLog>,
   }

   pub struct RawLog {
       pub address: String,
       pub topics: Vec<String>,
       pub data: Vec<u8>,
       pub block_number: u64,
       pub tx_hash: String,
       pub log_index: u32,
   }
   ```
2. Write a basic test verifying the trait is object-safe: `Box<dyn ChainConnector>`

**Test:** `cargo test -p roko-chain-ingest -- connector` passes.

### Task 6.3: Implement `EvmConnector`

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/alloy_impl.rs` (existing Alloy-backed ChainClient)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/connector.rs` (ChainConnector trait)

**Implementation:**
1. In `crates/roko-chain-ingest/src/connector/evm.rs`, implement `EvmConnector`:
   ```rust
   pub struct EvmConnector {
       chain_id: String,
       rpc_url: String,
       ws_url: Option<String>,
       block_time_ms: u64,
       finality: FinalityMode,
       client: Arc<dyn ChainClient>,
   }
   ```
2. Implement `ChainConnector` for `EvmConnector`:
   - `subscribe_blocks()`: use WebSocket subscription to `eth_subscribe("newHeads")` if ws_url is set, otherwise poll via `block_number()` + `get_block_header()`
   - `subscribe_logs()`: use WebSocket `eth_subscribe("logs", {address, topics})` if ws_url, otherwise poll via `get_logs()`
   - `fetch_block_range()`: batch `get_block_header()` + `get_logs()` calls
   - `tip()`: delegate to `client.block_number()`
3. Implement `EvmConnector::new(config: &ChainConfig) -> Result<Self>`:
   - Construct the underlying `ChainClient` (AlloyChainClient if feature enabled, otherwise error)
4. Write tests: create with mock client, verify subscription poll loop runs

**Test:** `cargo test -p roko-chain-ingest -- connector::evm` passes.

### Task 6.4: Implement `HyperliquidConnector`

**Read first:**
- Hyperliquid API docs (WebSocket subscriptions for trades, fills, order book)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/connector.rs` (ChainConnector trait)

**Implementation:**
1. In `crates/roko-chain-ingest/src/connector/hyperliquid.rs`, implement:
   ```rust
   pub struct HyperliquidConnector {
       chain_id: String,
       api_url: String,
       ws_url: String,
   }
   ```
2. Implement `ChainConnector` for `HyperliquidConnector`:
   - `subscribe_blocks()`: Hyperliquid does not have traditional blocks. Map fill events to pseudo-blocks grouped by timestamp windows.
   - `subscribe_logs()`: Map trade events, fill events, and funding rate events to `RawLog` format
   - `fetch_block_range()`: Query historical trades via REST API, group by timestamp
   - `tip()`: Return latest event sequence number
   - `block_time_ms()`: Return 50 (Hyperliquid's approximate interval)
   - `finality_mode()`: Return `FinalityMode::Deterministic`
3. Write tests: parse mock Hyperliquid WebSocket messages into RawBlock/RawLog

**Test:** `cargo test -p roko-chain-ingest -- connector::hyperliquid` passes.

### Task 6.5: Implement `CanonicalEvent` schema and `DeterministicId`

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/engram.rs` (Engram, ContentHash)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/hash.rs` (ContentHash, blake3)

**Implementation:**
1. In `crates/roko-chain-ingest/src/canonical.rs`, define:
   ```rust
   /// Chain-agnostic event that all connectors normalize into.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct CanonicalEvent {
       pub id: DeterministicId,
       pub chain_id: String,
       pub block_number: u64,
       pub block_hash: String,
       pub timestamp: u64,
       pub event_type: CanonicalEventType,
       pub source_address: String,
       pub topics: Vec<String>,
       pub data: Vec<u8>,
       pub tx_hash: String,
       pub log_index: u32,
       pub finality: FinalityStatus,
   }

   /// Deterministic identifier derived from chain_id + block + tx + log_index.
   #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
   pub struct DeterministicId(pub String);

   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub enum CanonicalEventType {
       Transfer,
       Swap,
       LiquidityChange,
       GovernanceVote,
       ContractDeploy,
       ContractCall,
       OracleUpdate,
       Unknown,
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum FinalityStatus {
       Pending,
       Confirmed(u32),     // confirmations count
       Finalized,
       Reorged,
   }
   ```
2. Implement `DeterministicId::compute(chain_id: &str, block: u64, tx_hash: &str, log_index: u32) -> Self`:
   - Blake3 hash of `"{chain_id}:{block}:{tx_hash}:{log_index}"`
   - Return hex-encoded hash
3. Implement `CanonicalEvent::from_raw(chain_id: &str, raw: &RawLog, block: &RawBlock) -> Self`:
   - Classify event type from topic[0] signatures (Transfer, Swap, etc.)
   - Compute deterministic ID
   - Set finality status to `Pending`
4. Write tests: deterministic ID is stable for same inputs, different for different inputs. Event classification for known topic hashes.

**Test:** `cargo test -p roko-chain-ingest -- canonical` passes.

### Task 6.6: Implement `ChainActor` struct

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs` (ProcessSupervisor, cancellation tokens)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/connector.rs` (ChainConnector trait)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/canonical.rs` (CanonicalEvent)

**Implementation:**
1. In `crates/roko-chain-ingest/src/actor.rs`, define:
   ```rust
   /// Async actor that runs a single chain subscription and normalizes events.
   pub struct ChainActor {
       connector: Box<dyn ChainConnector>,
       event_tx: mpsc::Sender<CanonicalEvent>,
       cancel: CancellationToken,
       metrics: ChainActorMetrics,
   }

   pub struct ChainActorMetrics {
       pub blocks_processed: AtomicU64,
       pub events_emitted: AtomicU64,
       pub errors: AtomicU64,
       pub last_block: AtomicU64,
   }
   ```
2. Implement `ChainActor::new(connector, event_tx, cancel) -> Self`
3. Implement `ChainActor::run(&self) -> Result<()>`:
   - Subscribe to blocks via connector
   - For each block: normalize all logs to CanonicalEvents, send via event_tx
   - Track metrics
   - Handle connector errors: log, increment error counter, reconnect with backoff
   - Exit on cancel signal
4. Implement `ChainActor::backfill(&self, from: u64, to: u64) -> Result<usize>`:
   - Fetch block range from connector
   - Normalize and send events
   - Return count of events sent
5. Write tests: actor processes mock blocks, emits correct number of events, exits on cancel

**Test:** `cargo test -p roko-chain-ingest -- actor` passes.

### Task 6.7: Implement `CanonicalEventBus`

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/pulse_bus.rs` (existing PulseBus pattern)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/bus_backends.rs` (BroadcastBus, MemoryBus)

**Implementation:**
1. In `crates/roko-chain-ingest/src/bus.rs`, define:
   ```rust
   /// Unified event bus that merges CanonicalEvents from multiple ChainActors.
   pub struct CanonicalEventBus {
       tx: broadcast::Sender<CanonicalEvent>,
       actors: Vec<JoinHandle<Result<()>>>,
   }
   ```
2. Implement `CanonicalEventBus::new(capacity: usize) -> Self`
3. Implement `CanonicalEventBus::add_chain(&mut self, connector: Box<dyn ChainConnector>, cancel: CancellationToken)`:
   - Create mpsc channel for the actor
   - Spawn a task that reads from mpsc and forwards to broadcast
   - Spawn `ChainActor::run()` task
   - Store join handle
4. Implement `CanonicalEventBus::subscribe(&self) -> broadcast::Receiver<CanonicalEvent>`
5. Implement `CanonicalEventBus::shutdown(&mut self)`:
   - Cancel all actors
   - Await all join handles
6. Write tests: add two mock chains, verify events from both arrive on subscriber

**Test:** `cargo test -p roko-chain-ingest -- bus` passes.

### Task 6.8: Implement `FinalityTracker`

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/observer.rs` (BlockTracker, gap detection)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/canonical.rs` (FinalityStatus, CanonicalEvent)

**Implementation:**
1. In `crates/roko-chain-ingest/src/finality.rs`, define:
   ```rust
   pub struct FinalityTracker {
       chains: HashMap<String, ChainFinalityState>,
   }

   struct ChainFinalityState {
       finality_mode: FinalityMode,
       pending_blocks: BTreeMap<u64, BlockState>,  // block_number -> state
       finalized_tip: u64,
   }

   struct BlockState {
       hash: String,
       parent_hash: String,
       events: Vec<DeterministicId>,
       confirmations: u32,
   }
   ```
2. Implement `FinalityTracker::new() -> Self`
3. Implement `FinalityTracker::track_block(&mut self, chain_id: &str, block: &RawBlock, finality: FinalityMode)`:
   - Add block to pending state
   - Check if parent hash matches previous block's hash (reorg detection)
   - If reorg detected: return list of affected event IDs
4. Implement `FinalityTracker::advance_finality(&mut self, chain_id: &str, current_tip: u64) -> Vec<FinalityUpdate>`:
   - For `Confirmations(n)`: blocks with `tip - block_number >= n` become finalized
   - For `Deterministic`: all blocks are immediately final
   - Return list of events that changed finality status
5. Implement `FinalityTracker::detect_reorg(&mut self, chain_id: &str, block: &RawBlock) -> Option<ReorgEvent>`:
   - Compare incoming block's parent_hash with stored hash for `block.number - 1`
   - If mismatch: walk back to find fork point, return affected blocks
   ```rust
   pub struct ReorgEvent {
       pub chain_id: String,
       pub fork_block: u64,
       pub orphaned_blocks: Vec<u64>,
       pub affected_events: Vec<DeterministicId>,
   }
   ```
6. Write tests: normal finality progression, reorg detection with known fork point, deterministic finality

**Test:** `cargo test -p roko-chain-ingest -- finality` passes.

### Task 6.9: Implement `TemporalAggregator`

**Read first:**
- PRD-09 section 10 (hierarchical temporal resolution: R0/R1/R2/R3)

**Implementation:**
1. In `crates/roko-chain-ingest/src/temporal.rs`, define:
   ```rust
   /// Hierarchical temporal aggregation of chain events.
   pub struct TemporalAggregator {
       windows: [AggregationWindow; 4],
   }

   pub struct AggregationWindow {
       pub resolution: TemporalResolution,
       pub bucket_duration: Duration,
       pub buckets: VecDeque<EventBucket>,
       pub max_buckets: usize,
   }

   #[derive(Debug, Clone, Copy)]
   pub enum TemporalResolution {
       R0,  // Per-block (real-time)
       R1,  // 1-minute windows
       R2,  // 1-hour windows
       R3,  // 1-day windows
   }

   pub struct EventBucket {
       pub start: u64,       // unix timestamp
       pub end: u64,
       pub event_count: u64,
       pub unique_addresses: HashSet<String>,
       pub total_value: u128,
       pub event_types: HashMap<CanonicalEventType, u64>,
   }
   ```
2. Implement `TemporalAggregator::new() -> Self`:
   - R0: per-block, keep last 256 blocks
   - R1: 1-minute, keep last 1440 (24 hours)
   - R2: 1-hour, keep last 720 (30 days)
   - R3: 1-day, keep last 365
3. Implement `TemporalAggregator::ingest(&mut self, event: &CanonicalEvent)`:
   - Add to R0 bucket for the event's block
   - Promote aggregates up to R1/R2/R3 when time boundaries cross
4. Implement `TemporalAggregator::query(&self, resolution: TemporalResolution, lookback: usize) -> Vec<&EventBucket>`:
   - Return the last `lookback` buckets at the given resolution
5. Write tests: ingest 100 events, verify R0 has per-block granularity, R1 aggregates correctly

**Test:** `cargo test -p roko-chain-ingest -- temporal` passes.

### Task 6.10: Implement chain config in roko.toml

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` (RokoConfig sections)

**Implementation:**
1. In `crates/roko-core/src/config/schema.rs`, add a new section:
   ```rust
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct ChainConfig {
       pub id: String,
       pub rpc: String,
       #[serde(default)]
       pub ws: Option<String>,
       #[serde(default = "default_block_time")]
       pub block_time_ms: u64,
       #[serde(default)]
       pub finality_confirmations: Option<u32>,
       #[serde(default)]
       pub finality: Option<String>,  // "deterministic" or "probabilistic"
       #[serde(default)]
       pub watched_addresses: Vec<String>,
       #[serde(default)]
       pub watched_topics: Vec<String>,
   }
   ```
2. Add to `RokoConfig`:
   ```rust
   #[serde(default, skip_serializing_if = "Vec::is_empty")]
   pub chains: Vec<ChainConfig>,
   ```
3. Write tests: parse TOML with `[[chains]]` array, verify deserialization

**Test:** `cargo test -p roko-core -- config::chains` passes.

---

## Phase 7: Contract discovery pipeline

Goal: build a multi-layer classification system that identifies and categorizes smart contracts automatically from on-chain data.

### Task 7.1: Implement Layer 0 -- ERC-165 interface detection

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/client.rs` (ChainClient::eth_call)
- ERC-165 standard (interface ID = first 4 bytes of `supportsInterface(bytes4)`)

**Implementation:**
1. In `crates/roko-chain-ingest/src/discovery/erc165.rs` (new module), define:
   ```rust
   pub struct Erc165Detector;

   pub struct InterfaceResult {
       pub address: String,
       pub supported_interfaces: Vec<String>,
       pub known_standards: Vec<KnownStandard>,
   }

   pub enum KnownStandard {
       Erc20,
       Erc721,
       Erc1155,
       Erc4626,
       Erc2981,
       Unknown(String),
   }
   ```
2. Implement `Erc165Detector::detect(client: &dyn ChainClient, address: &str) -> Result<InterfaceResult>`:
   - Call `eth_call` with `supportsInterface(0x01ffc9a7)` (ERC-165 itself)
   - If supported, check known interface IDs: ERC-20 (0x36372b07), ERC-721 (0x80ac58cd), ERC-1155 (0xd9b67a26), ERC-4626 (varies), ERC-2981 (0x2a55205a)
   - Return detected interfaces
3. Write tests: mock client that returns true for ERC-20 interface, verify detection

**Test:** `cargo test -p roko-chain-ingest -- discovery::erc165` passes.

### Task 7.2: Implement Layer 1 -- Function selector fingerprinting

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/client.rs` (ChainClient::get_storage_at)

**Implementation:**
1. In `crates/roko-chain-ingest/src/discovery/selectors.rs`, define:
   ```rust
   pub struct SelectorFingerprint {
       pub address: String,
       pub selectors: Vec<[u8; 4]>,
       pub matched_functions: Vec<MatchedFunction>,
   }

   pub struct MatchedFunction {
       pub selector: [u8; 4],
       pub signature: String,       // e.g., "transfer(address,uint256)"
       pub confidence: f32,         // 1.0 for exact match, lower for collision
   }
   ```
2. Build a lookup table of ~200 common DeFi function selectors (transfer, approve, swap, addLiquidity, etc.)
3. Implement `fingerprint_contract(client: &dyn ChainClient, address: &str) -> Result<SelectorFingerprint>`:
   - Fetch contract bytecode via `eth_call` with EXTCODECOPY or get_storage_at
   - Extract PUSH4 opcodes (0x63 XX XX XX XX) to find function selectors
   - Match against known selector database
4. Write tests: extract selectors from known bytecode patterns, match against database

**Test:** `cargo test -p roko-chain-ingest -- discovery::selectors` passes.

### Task 7.3: Implement Layer 2 -- Bytecode similarity matching

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/lib.rs` (HDC vectors)

**Implementation:**
1. In `crates/roko-chain-ingest/src/discovery/similarity.rs`, define:
   ```rust
   pub struct BytecodeSimilarity {
       known_families: Vec<ContractFamily>,
   }

   pub struct ContractFamily {
       pub name: String,           // e.g., "Uniswap V2 Pair"
       pub code_hash: String,      // blake3 of init code or runtime code
       pub selectors: Vec<[u8; 4]>,
   }

   pub struct SimilarityMatch {
       pub family: String,
       pub similarity: f32,        // 0.0-1.0
       pub matching_selectors: usize,
       pub total_selectors: usize,
   }
   ```
2. Seed the database with known contract families: Uniswap V2/V3, Aave V2/V3, Compound V2/V3, Curve, Maker, OpenZeppelin ERC-20/721/1155
3. Implement `BytecodeSimilarity::match_contract(selectors: &SelectorFingerprint) -> Vec<SimilarityMatch>`:
   - Jaccard similarity of selector sets
   - Return families above 0.6 threshold
4. Write tests: Uniswap V2 Pair selectors match the known family

**Test:** `cargo test -p roko-chain-ingest -- discovery::similarity` passes.

### Task 7.4: Implement Layer 3 -- Transaction pattern classifier

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/canonical.rs` (CanonicalEvent, CanonicalEventType)

**Implementation:**
1. In `crates/roko-chain-ingest/src/discovery/patterns.rs`, define:
   ```rust
   pub struct PatternClassifier {
       thresholds: PatternThresholds,
   }

   pub struct PatternThresholds {
       pub min_events: usize,          // minimum events to classify (default: 10)
       pub dex_swap_ratio: f64,        // fraction of Swap events for DEX classification
       pub lending_ratio: f64,         // fraction of Deposit/Borrow for lending
   }

   pub struct PatternClassification {
       pub contract_type: ContractType,
       pub confidence: f64,
       pub evidence: Vec<String>,
   }

   pub enum ContractType {
       Dex,
       Lending,
       Bridge,
       Oracle,
       Nft,
       Governance,
       Yield,
       Unknown,
   }
   ```
2. Implement `PatternClassifier::classify(events: &[CanonicalEvent]) -> PatternClassification`:
   - Count event types
   - If >50% are Swap events -> Dex
   - If >30% are Deposit + Borrow -> Lending
   - If events span multiple chain_ids -> Bridge
   - If >80% are OracleUpdate -> Oracle
   - Otherwise -> Unknown
3. Write tests: 20 Swap events classify as Dex, mixed events classify as Unknown

**Test:** `cargo test -p roko-chain-ingest -- discovery::patterns` passes.

### Task 7.5: Implement Layer 4 -- Factory contract tracker

**Read first:**
- Known factory event signatures: `PairCreated(address,address,address,uint256)`, `PoolCreated(address,address,uint24,int24,address)`

**Implementation:**
1. In `crates/roko-chain-ingest/src/discovery/factories.rs`, define:
   ```rust
   pub struct FactoryTracker {
       known_factories: HashMap<String, FactoryType>,  // topic[0] -> type
       discovered_children: Vec<FactoryChild>,
   }

   pub struct FactoryChild {
       pub factory_address: String,
       pub child_address: String,
       pub factory_type: FactoryType,
       pub creation_block: u64,
       pub creation_args: Vec<String>,  // decoded event args
   }

   pub enum FactoryType {
       UniswapV2,
       UniswapV3,
       CurveFactory,
       AavePool,
       Custom(String),
   }
   ```
2. Seed `known_factories` with topic hashes for PairCreated, PoolCreated, etc.
3. Implement `FactoryTracker::process_event(&mut self, event: &CanonicalEvent) -> Option<FactoryChild>`:
   - Check if event's topic[0] matches a known factory event
   - Decode the event data to extract child contract address
   - Return FactoryChild
4. Write tests: PairCreated event with known topic produces correct FactoryChild

**Test:** `cargo test -p roko-chain-ingest -- discovery::factories` passes.

### Task 7.6: Implement Layer 5 -- Cross-agent InsightStore query

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/lib.rs` (knowledge store)

**Implementation:**
1. In `crates/roko-chain-ingest/src/discovery/insights.rs`, define:
   ```rust
   pub struct InsightClassifier {
       insight_store: Arc<dyn InsightQuery>,
   }

   #[async_trait]
   pub trait InsightQuery: Send + Sync {
       async fn query_contract(&self, address: &str) -> Result<Vec<InsightEntry>>;
   }

   pub struct InsightEntry {
       pub source_agent: String,
       pub classification: String,
       pub confidence: f64,
       pub timestamp: u64,
   }
   ```
2. Implement `InsightClassifier::classify(address: &str) -> Result<Option<PatternClassification>>`:
   - Query insight store for any previous classifications of this address
   - If multiple agents agree with high confidence -> return consensus classification
   - If disagreement -> return None (let other layers decide)
3. Write tests: mock insight store with consensus, verify classification

**Test:** `cargo test -p roko-chain-ingest -- discovery::insights` passes.

### Task 7.7: Implement `ContractRegistry` that composes all layers

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/discovery/` (all previous layers)

**Implementation:**
1. In `crates/roko-chain-ingest/src/discovery/registry.rs`, define:
   ```rust
   pub struct ContractRegistry {
       erc165: Erc165Detector,
       selectors: SelectorFingerprint,
       similarity: BytecodeSimilarity,
       patterns: PatternClassifier,
       factories: FactoryTracker,
       insights: InsightClassifier,
       classifications: DashMap<String, ContractClassification>,
   }

   pub struct ContractClassification {
       pub address: String,
       pub contract_type: ContractType,
       pub known_standard: Option<KnownStandard>,
       pub family: Option<String>,
       pub confidence: f64,
       pub layers_used: Vec<u8>,  // which layers contributed
       pub first_seen: u64,
       pub last_updated: u64,
   }
   ```
2. Implement `ContractRegistry::classify(&self, client: &dyn ChainClient, address: &str, events: &[CanonicalEvent]) -> Result<ContractClassification>`:
   - Run layers in order (0 through 5)
   - Combine results: highest-confidence classification wins
   - Cache result in `classifications`
3. Implement `ContractRegistry::process_event(&self, event: &CanonicalEvent)`:
   - If source address is unknown, trigger classification
   - Update event counts for pattern classifier
   - Check for factory events
4. Write tests: classify a known Uniswap V2 pair (ERC-165 returns ERC-20, selectors match, patterns show Swap)

**Test:** `cargo test -p roko-chain-ingest -- discovery::registry` passes.

---

## Phase 8: Predictive foraging and dynamic worldview

Goal: implement the attention allocation model (Gittins indices, habituation, MVT patch switching) and the dynamic WorldGraph that accumulates discovered entities and relationships.

### Task 8.1: Create `roko-foraging` crate skeleton

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (workspace pattern)

**Implementation:**
1. Create `/Users/will/dev/nunchi/roko/roko/crates/roko-foraging/`
2. Create `Cargo.toml`: `roko-core`, `serde`, `serde_json`, `rand`
3. Create `src/lib.rs`: modules `gittins`, `budget`, `habituation`, `mvt`
4. Add to workspace

**Test:** `cargo check -p roko-foraging` exits 0.

### Task 8.2: Implement Gittins index computation

**Read first:**
- Gittins & Jones 1974 (multi-armed bandit allocation indices)
- PRD-09 section 13 (predictive foraging)

**Implementation:**
1. In `crates/roko-foraging/src/gittins.rs`, define:
   ```rust
   pub struct ForagingEntity {
       pub id: String,                    // e.g., chain_id:address
       pub observations: usize,
       pub total_reward: f64,             // sum of information values
       pub last_observation: u64,         // timestamp
       pub gittins_index: f64,            // computed allocation index
   }

   pub struct GittinsComputer {
       discount_factor: f64,              // gamma, typically 0.99
       exploration_bonus: f64,            // UCB-style bonus for under-observed entities
   }
   ```
2. Implement `GittinsComputer::new(discount: f64, bonus: f64) -> Self`
3. Implement `GittinsComputer::compute_index(&self, entity: &ForagingEntity) -> f64`:
   - Mean reward = total_reward / observations
   - UCB bonus = exploration_bonus * sqrt(ln(total_observations) / entity.observations)
   - Gittins approximation = mean_reward + UCB bonus
   - Apply discount factor for recency: multiply by discount^(now - last_observation)
4. Implement `GittinsComputer::rank_entities(&self, entities: &mut [ForagingEntity])`:
   - Compute index for each
   - Sort descending by index
5. Write tests: entity with high reward ranks above entity with low reward, under-observed entity gets bonus

**Test:** `cargo test -p roko-foraging -- gittins` passes.

### Task 8.3: Implement `AttentionBudget` allocator

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/lib.rs` (existing AttentionBidder, VCG allocation)

**Implementation:**
1. In `crates/roko-foraging/src/budget.rs`, define:
   ```rust
   pub struct AttentionBudget {
       total_budget: f64,               // total units of attention per tick (e.g., 100.0)
       allocations: HashMap<String, f64>, // entity_id -> allocated budget
       min_allocation: f64,              // floor per entity (e.g., 0.1)
   }
   ```
2. Implement `AttentionBudget::allocate(&mut self, entities: &[ForagingEntity]) -> HashMap<String, f64>`:
   - Compute Gittins indices for all entities
   - Allocate proportional to index: `budget_i = total_budget * (G_i / sum(G))`
   - Enforce minimum allocation floor
   - Re-normalize so total equals budget
3. Implement `AttentionBudget::should_monitor(&self, entity_id: &str) -> bool`:
   - Returns true if allocation > min_allocation
4. Implement `AttentionBudget::monitoring_interval(&self, entity_id: &str) -> Duration`:
   - Higher allocation = shorter interval
   - `interval = base_interval / (allocation / mean_allocation)`
5. Write tests: three entities, verify proportional allocation, verify minimum floor

**Test:** `cargo test -p roko-foraging -- budget` passes.

### Task 8.4: Implement `HabituationMask`

**Read first:**
- PRD-09 section 13 (habituation for suppressing frequent benign events)

**Implementation:**
1. In `crates/roko-foraging/src/habituation.rs`, define:
   ```rust
   pub struct HabituationMask {
       decay_rate: f64,                     // how fast novelty decays (0.0-1.0)
       suppression_threshold: f64,          // below this novelty, suppress the event
       entity_state: HashMap<String, HabituationState>,
   }

   struct HabituationState {
       event_count: u64,
       novelty: f64,                        // starts at 1.0, decays toward 0.0
       last_novel_event: u64,               // timestamp of last non-habituated event
   }
   ```
2. Implement `HabituationMask::new(decay_rate: f64, threshold: f64) -> Self`
3. Implement `HabituationMask::observe(&mut self, entity_id: &str, event: &CanonicalEvent) -> bool`:
   - Get or create state for entity
   - Decrease novelty: `novelty *= (1.0 - decay_rate)`
   - If event is "surprising" (different type, large value, new topic): reset novelty to 1.0
   - Return `novelty > suppression_threshold` (true = pass through, false = suppress)
4. Implement `HabituationMask::reset(&mut self, entity_id: &str)`:
   - Reset novelty to 1.0 (used when foraging model reallocates attention to this entity)
5. Write tests: 100 identical events, verify suppression kicks in after threshold, surprise event resets novelty

**Test:** `cargo test -p roko-foraging -- habituation` passes.

### Task 8.5: Implement Marginal Value Theorem patch switching

**Read first:**
- Charnov 1976 (Marginal Value Theorem)
- PRD-09 section 16 (active inference for attention allocation)

**Implementation:**
1. In `crates/roko-foraging/src/mvt.rs`, define:
   ```rust
   pub struct MvtDecider {
       travel_cost: f64,                    // cost to switch attention to a new entity
       diminishing_rate: f64,               // how fast returns diminish in a patch
   }

   pub struct PatchState {
       pub entity_id: String,
       pub cumulative_return: f64,
       pub time_in_patch: u64,              // ticks spent monitoring this entity
       pub marginal_return: f64,            // current rate of return
   }
   ```
2. Implement `MvtDecider::should_switch(&self, current: &PatchState, alternatives: &[ForagingEntity]) -> Option<String>`:
   - Compute current marginal return: `dR/dt = cumulative_return / time_in_patch`
   - Compute average return of alternatives (mean Gittins index)
   - If marginal return < average return - travel_cost -> switch to highest-indexed alternative
   - Return entity_id to switch to, or None to stay
3. Implement `MvtDecider::update_patch(&self, patch: &mut PatchState, reward: f64)`:
   - Add reward to cumulative
   - Increment time
   - Recompute marginal return with diminishing factor
4. Write tests: entity with diminishing returns triggers switch, entity with constant returns stays

**Test:** `cargo test -p roko-foraging -- mvt` passes.

### Task 8.6: Create `roko-worldgraph` crate skeleton

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (workspace pattern)

**Implementation:**
1. Create `/Users/will/dev/nunchi/roko/roko/crates/roko-worldgraph/`
2. Create `Cargo.toml`: `roko-core`, `roko-chain-ingest`, `roko-primitives`, `serde`, `serde_json`, `dashmap`, `petgraph`
3. Create `src/lib.rs`: modules `graph`, `entity`, `relationship`, `fingerprint`, `context`
4. Add to workspace

**Test:** `cargo check -p roko-worldgraph` exits 0.

### Task 8.7: Implement `WorldGraph` struct

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/canonical.rs` (CanonicalEvent)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/discovery/registry.rs` (ContractClassification)

**Implementation:**
1. In `crates/roko-worldgraph/src/graph.rs`, define:
   ```rust
   pub struct WorldGraph {
       entities: DashMap<String, Entity>,
       relationships: DashMap<String, Relationship>,
       graph: RwLock<petgraph::Graph<String, RelationType>>,
   }
   ```
2. In `crates/roko-worldgraph/src/entity.rs`, define:
   ```rust
   pub struct Entity {
       pub id: String,                      // chain_id:address
       pub chain_id: String,
       pub address: String,
       pub classification: Option<ContractClassification>,
       pub first_seen: u64,
       pub last_seen: u64,
       pub event_count: u64,
       pub tags: HashSet<String>,
   }
   ```
3. In `crates/roko-worldgraph/src/relationship.rs`, define:
   ```rust
   pub struct Relationship {
       pub id: String,                      // deterministic from source+target+type
       pub source: String,
       pub target: String,
       pub rel_type: RelationType,
       pub strength: f64,                   // 0.0-1.0, based on interaction frequency
       pub first_seen: u64,
       pub last_seen: u64,
       pub interaction_count: u64,
   }

   pub enum RelationType {
       Transfer,           // token transfers between addresses
       LiquidityProvider,  // address provides liquidity to pool
       Factory,            // factory created child contract
       OracleConsumer,     // contract reads from oracle
       Governance,         // address votes in governance
       Bridge,             // cross-chain bridge relationship
       Unknown,
   }
   ```
4. Implement `WorldGraph::new() -> Self`
5. Implement `WorldGraph::add_entity(&self, entity: Entity)`
6. Implement `WorldGraph::add_relationship(&self, rel: Relationship)`
7. Implement `WorldGraph::get_entity(&self, id: &str) -> Option<Entity>`
8. Implement `WorldGraph::neighbors(&self, id: &str) -> Vec<(String, RelationType)>`
9. Implement `WorldGraph::entity_count(&self) -> usize`
10. Implement `WorldGraph::relationship_count(&self) -> usize`
11. Write tests: add entities and relationships, verify graph structure, neighbor lookup

**Test:** `cargo test -p roko-worldgraph -- graph` passes.

### Task 8.8: Implement entity discovery from CanonicalEvents

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-worldgraph/src/graph.rs` (WorldGraph)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/canonical.rs` (CanonicalEvent)

**Implementation:**
1. In `crates/roko-worldgraph/src/graph.rs`, implement:
   ```rust
   impl WorldGraph {
       pub fn process_event(&self, event: &CanonicalEvent) {
           // Add source address as entity if new
           self.ensure_entity(&event.chain_id, &event.source_address, event.timestamp);

           // Decode event to extract target addresses
           // (Transfer events have a recipient, Swap events have a pool, etc.)
           if let Some(targets) = extract_targets(event) {
               for target in targets {
                   self.ensure_entity(&event.chain_id, &target, event.timestamp);
                   self.ensure_relationship(
                       &event.source_address,
                       &target,
                       classify_relationship(&event.event_type),
                       event.timestamp,
                   );
               }
           }
       }

       fn ensure_entity(&self, chain_id: &str, address: &str, timestamp: u64) {
           let id = format!("{chain_id}:{address}");
           self.entities.entry(id.clone())
               .and_modify(|e| { e.last_seen = timestamp; e.event_count += 1; })
               .or_insert_with(|| Entity {
                   id, chain_id: chain_id.to_string(), address: address.to_string(),
                   classification: None, first_seen: timestamp, last_seen: timestamp,
                   event_count: 1, tags: HashSet::new(),
               });
       }
   }
   ```
2. Implement `extract_targets(event: &CanonicalEvent) -> Option<Vec<String>>`:
   - For Transfer: decode topic[2] as recipient address
   - For Swap: contract address is the pool
   - For ContractDeploy: the deployed address
   - For Unknown: None
3. Implement `classify_relationship(event_type: &CanonicalEventType) -> RelationType`
4. Write tests: Transfer event creates two entities and a Transfer relationship

**Test:** `cargo test -p roko-worldgraph -- graph::process_event` passes.

### Task 8.9: Implement HDC fingerprint for WorldGraph state

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/lib.rs` (HdcVector)

**Implementation:**
1. In `crates/roko-worldgraph/src/fingerprint.rs`, define:
   ```rust
   pub fn fingerprint_worldgraph(graph: &WorldGraph) -> HdcVector {
       let mut combined = HdcVector::zero(HDC_DIM);
       // Encode entity set
       for entity in graph.entities.iter() {
           let entity_vec = encode_entity(&entity);
           combined = combined.bundle(&entity_vec);
       }
       // Encode relationship set
       for rel in graph.relationships.iter() {
           let rel_vec = encode_relationship(&rel);
           combined = combined.bundle(&rel_vec);
       }
       combined.normalize()
   }
   ```
2. Implement `encode_entity(entity: &Entity) -> HdcVector`:
   - Bind address hash vector with classification vector
3. Implement `encode_relationship(rel: &Relationship) -> HdcVector`:
   - Bind source vector, target vector, and type vector
4. Write tests: two identical graphs produce the same fingerprint, different graphs produce different fingerprints

**Test:** `cargo test -p roko-worldgraph -- fingerprint` passes.

### Task 8.10: Implement WorldGraph context injection via VCG bidding

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/lib.rs` (AttentionBidder, PromptComposer, VCG)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (how AttentionBidders are registered)

**Implementation:**
1. In `crates/roko-worldgraph/src/context.rs`, define:
   ```rust
   pub struct WorldGraphBidder {
       graph: Arc<WorldGraph>,
   }
   ```
2. Implement the `AttentionBidder` interface (or adapt to whatever interface `roko-compose` expects):
   - `bid()`: compute bid based on how relevant the WorldGraph state is to the current task
   - `render()`: produce a text section summarizing relevant entities and relationships for injection into the system prompt
   ```rust
   impl WorldGraphBidder {
       pub fn render_context(&self, task_domain: &str, max_tokens: usize) -> String {
           // Find entities most relevant to task domain
           let relevant = self.graph.entities.iter()
               .filter(|e| is_relevant_to_domain(e.value(), task_domain))
               .take(20)
               .collect::<Vec<_>>();
           // Format as context section
           format_entity_context(&relevant, max_tokens)
       }
   }
   ```
3. Wire `WorldGraphBidder` into the existing `AttentionBidder` variants in `orchestrate.rs`:
   - Add a new variant or register as a custom bidder
4. Write tests: WorldGraph with 10 DeFi entities produces a context section when task domain is "chain"

**Test:** `cargo test -p roko-worldgraph -- context` passes.

### Task 8.11: Implement strategy evolution through dream consolidation

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/lib.rs` (DreamRunner, dream cycle)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-worldgraph/src/fingerprint.rs` (WorldGraph fingerprint)

**Implementation:**
1. In `crates/roko-worldgraph/src/graph.rs`, implement:
   ```rust
   impl WorldGraph {
       /// Take a snapshot of the current graph state for dream replay.
       pub fn snapshot(&self) -> WorldGraphSnapshot {
           WorldGraphSnapshot {
               entities: self.entities.iter()
                   .map(|e| e.value().clone())
                   .collect(),
               relationships: self.relationships.iter()
                   .map(|r| r.value().clone())
                   .collect(),
               fingerprint: fingerprint_worldgraph(self),
               timestamp: current_timestamp(),
           }
       }

       /// Compute the delta between two snapshots.
       pub fn diff(before: &WorldGraphSnapshot, after: &WorldGraphSnapshot) -> WorldGraphDelta {
           WorldGraphDelta {
               new_entities: /* entities in after but not before */,
               removed_entities: /* entities in before but not after */,
               new_relationships: /* relationships in after but not before */,
               changed_classifications: /* entities whose classification changed */,
               fingerprint_distance: before.fingerprint.cosine_distance(&after.fingerprint),
           }
       }
   }
   ```
2. Implement `WorldGraphSnapshot` and `WorldGraphDelta` structs
3. Write tests: create two snapshots with a new entity added, verify delta detects it

**Test:** `cargo test -p roko-worldgraph -- graph::snapshot` passes.

---

## Phase 9: Integration and end-to-end testing

Goal: wire all components together and verify the full pipeline works end-to-end.

### Task 9.1: Wire package installation into agent startup

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (agent dispatch)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-ext-registry/src/installer.rs` (Installer::list)

**Implementation:**
1. In the agent startup path (orchestrate.rs or agent_serve.rs), add a step that:
   - Lists installed packages via `Installer::list()`
   - For each JS extension: loads via `PiBridge::load_extension()`, adds tools to the dynamic registry
   - For each Rust extension: loads via `load_rust_extension()` (if dynamic-extensions feature)
   - For each skill: loads via `load_skills_dir()`, makes available for agent context
   - For each domain profile: loads via `load_domain_profile()`, merges into ComposedProfile
2. Verify that installed extensions' tools appear in the agent's tool set
3. Write integration test in `tests/` crate

**Test:** `cargo test -p tests -- package_loading_integration` passes.

### Task 9.2: Wire multi-chain subscription into agent runtime

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` (ChainConfig)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain-ingest/src/bus.rs` (CanonicalEventBus)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (agent dispatch)

**Implementation:**
1. In orchestrate.rs, when a blockchain-domain agent starts:
   - Read `[[chains]]` config from roko.toml
   - For each chain config: instantiate the appropriate `ChainConnector` (EvmConnector or HyperliquidConnector)
   - Create `CanonicalEventBus`, add all connectors
   - Pass the bus subscriber to the agent's event processing loop
2. In the heartbeat/tick loop:
   - Drain events from the bus subscriber
   - Process through `FinalityTracker`
   - Process through `ContractRegistry`
   - Process through `ForagingModel` (attention allocation)
   - Process through `WorldGraph` (entity discovery)
3. Write integration test with mock chains

**Test:** `cargo test -p tests -- multichain_integration` passes.

### Task 9.3: Wire foraging model into the attention allocation loop

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-foraging/src/budget.rs` (AttentionBudget)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-foraging/src/habituation.rs` (HabituationMask)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-foraging/src/mvt.rs` (MvtDecider)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (the theta tick loop)

**Implementation:**
1. In orchestrate.rs, add a `ForagingModel` struct that composes:
   ```rust
   struct ForagingModel {
       entities: Vec<ForagingEntity>,
       budget: AttentionBudget,
       habituation: HabituationMask,
       mvt: MvtDecider,
       gittins: GittinsComputer,
   }
   ```
2. On each theta tick:
   - Recompute Gittins indices for all entities
   - Reallocate attention budget
   - For each incoming event: check habituation mask
   - Check MVT: should the agent switch attention patches?
3. Log attention allocations to `.roko/learn/foraging.jsonl` for analysis
4. Write integration test: 100 ticks with mock entities, verify attention concentrates on high-value entities

**Test:** `cargo test -p tests -- foraging_integration` passes.

### Task 9.4: Wire WorldGraph into dream consolidation

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/lib.rs` (DreamRunner)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-worldgraph/src/graph.rs` (WorldGraph::snapshot, WorldGraph::diff)

**Implementation:**
1. In the dream consolidation loop (triggered by DreamRunner):
   - Take a WorldGraph snapshot before consolidation
   - Run dream replay: re-process recent events, update entity classifications
   - Take a WorldGraph snapshot after
   - Compute delta: new entities, changed classifications, fingerprint distance
   - If fingerprint distance > threshold: emit a "worldview_shift" signal for the agent
2. Persist WorldGraph snapshots to `.roko/state/worldgraph/`
3. Write integration test: dream consolidation updates entity classifications

**Test:** `cargo test -p tests -- worldgraph_dream_integration` passes.

### Task 9.5: End-to-end -- Pi package install and execution

**Read first:**
- All of Phase 2 (CLI commands) and Phase 3 (QuickJS bridge)

**Implementation:**
1. Create a test fixture: `tests/fixtures/pi-hello-world/`:
   - `package.json` with `"pi": { "name": "hello-world" }`
   - `index.js` that calls `pi.registerTool({ name: "hello", description: "says hello" })`
2. Write integration test:
   - Install the fixture via `Installer::install(&PackageSource::Local { path })`
   - Load via `PiBridge::load_extension()`
   - Verify "hello" tool appears in registered tools
   - Call the tool via the bridge
3. This test proves the Pi-compatibility claim

**Test:** `cargo test -p tests -- pi_compat_e2e` passes.

### Task 9.6: End-to-end -- multi-chain agent with 3 chains

**Read first:**
- Phase 6 (multi-chain architecture)

**Implementation:**
1. Write integration test:
   - Create 3 mock ChainConnectors (Ethereum, Base, Hyperliquid) with different block times
   - Create CanonicalEventBus, add all 3
   - Subscribe to the bus
   - Inject 10 events per chain via the mock connectors
   - Verify 30 events arrive on the subscriber
   - Verify FinalityTracker handles confirmations correctly for EVM chains
   - Verify FinalityTracker handles deterministic finality for Hyperliquid
2. Test reorg scenario: inject a block, then inject a conflicting block at the same height
   - Verify FinalityTracker emits ReorgEvent
   - Verify affected events get FinalityStatus::Reorged

**Test:** `cargo test -p tests -- multichain_e2e` passes.

### Task 9.7: End-to-end -- foraging model reduces monitoring cost

**Read first:**
- Phase 8 (foraging model)

**Implementation:**
1. Write integration test:
   - Create 50 mock entities with varying reward rates (10 high, 10 medium, 30 low)
   - Run 200 ticks of the foraging model
   - Measure: what fraction of attention goes to the top 10 entities?
   - Assert: top 10 entities receive > 60% of total attention budget
   - Measure: how many low-value entities are suppressed by habituation?
   - Assert: at least 20 of 30 low-value entities are suppressed after 200 ticks

**Test:** `cargo test -p tests -- foraging_efficiency_e2e` passes.

### Task 9.8: End-to-end -- WorldGraph discovers and classifies contracts

**Read first:**
- Phase 7 (contract discovery) and Phase 8 (WorldGraph)

**Implementation:**
1. Write integration test:
   - Create mock chain with 20 contracts:
     - 5 Uniswap V2 pairs (PairCreated events + Swap events)
     - 5 ERC-20 tokens (Transfer events)
     - 5 Aave pools (Deposit/Borrow events)
     - 5 unknown contracts
   - Process 100 events through ContractRegistry + WorldGraph
   - Verify: 15 of 20 contracts correctly classified (75%+ accuracy)
   - Verify: WorldGraph has all 20 entities
   - Verify: WorldGraph has Transfer and LiquidityProvider relationships
   - Verify: factory tracker detected the Uniswap pairs

**Test:** `cargo test -p tests -- worldgraph_discovery_e2e` passes.

### Task 9.9: End-to-end -- multi-domain agent handles mixed tasks

**Read first:**
- Phase 5 (multi-domain composition)

**Implementation:**
1. Write integration test:
   - Create a ComposedProfile with Coding + Chain domains
   - Create two mock tasks: one coding task (implement a function), one chain task (analyze a swap)
   - Dispatch both through the orchestration path
   - Verify: coding task uses compile+test+clippy gates
   - Verify: chain task uses compile+simulation+audit gates
   - Verify: both tasks have access to the full union of tool categories

**Test:** `cargo test -p tests -- multidomain_e2e` passes.

---

## Phase 10: Fine-tuning loop (Stream C)

**Goal**: Close the loop from successful agent episodes to fine-tuned models that re-enter the CascadeRouter as new routing arms. This is the self-improvement engine -- roko gets better at tasks it has done before.

### Task 10.1: Implement episode-to-training-data extraction

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/training_data.rs` (new file)

**Read first:**
- `crates/roko-learn/src/episode_logger.rs` -- `Episode` struct, `EpisodeLogger`
- `crates/roko-learn/src/hdc_fingerprint.rs` -- `fingerprint_episode()` for deduplication
- `.roko/episodes.jsonl` -- episode format on disk

**What to do:**

1. Create `crates/roko-learn/src/training_data.rs`.
2. Define the extraction pipeline:

```rust
/// Extracts training data from successful episodes for fine-tuning.
pub struct TrainingDataExtractor {
    /// Minimum gate pass rate for an episode to qualify.
    min_pass_rate: f64,
    /// Maximum episodes per extraction batch.
    batch_size: usize,
    /// Deduplicate by HDC fingerprint similarity threshold.
    dedup_threshold: f64,
}

/// A single training example in JSONL format compatible with
/// HuggingFace AutoTrain and OpenAI fine-tuning APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    /// System prompt used for this episode.
    pub system: String,
    /// User message (task description + context).
    pub user: String,
    /// Assistant response (the agent's successful output).
    pub assistant: String,
    /// Metadata for filtering and analysis.
    pub metadata: TrainingMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetadata {
    pub episode_id: String,
    pub domain: String,
    pub gate_pass_rate: f64,
    pub model_used: String,
    pub cost_usd: f64,
    pub turns: usize,
    pub extracted_at: String,
}
```

3. Implement `TrainingDataExtractor::extract(episodes: &[Episode]) -> Vec<TrainingExample>`:
   - Filter: only episodes where all gates passed (`gate_pass_rate == 1.0` by default)
   - Filter: only episodes with `cost_usd > 0` (skip T0 suppressions)
   - Deduplicate: compute HDC fingerprint for each episode, skip if similarity > `dedup_threshold` with an already-included episode
   - Map: extract system prompt, user message (task description), and assistant response from the episode's turn data
   - Cap at `batch_size`

4. Implement `TrainingDataExtractor::to_jsonl(examples: &[TrainingExample]) -> String`:
   - One JSON object per line, compatible with HuggingFace and OpenAI formats.

5. Register in `crates/roko-learn/src/lib.rs`: `pub mod training_data;`

**Test:**
- 10 episodes: 7 pass all gates, 3 fail. Assert 7 training examples extracted.
- 2 near-duplicate episodes (HDC similarity > threshold). Assert only 1 included.
- JSONL output parses as valid JSON-per-line.
- T0-suppressed episode (cost = 0) excluded.

- [ ] `TrainingDataExtractor` filters successful, non-duplicate episodes
- [ ] `TrainingExample` struct compatible with HF/OpenAI fine-tuning formats
- [ ] JSONL serialization
- [ ] HDC-based deduplication
- [ ] Module registered in `lib.rs`

---

### Task 10.2: Implement HuggingFace dataset push

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-hf/src/datasets.rs`

**Read first:**
- Task 10.1 output
- `crates/roko-hf/src/hub.rs` -- `HubClient`
- HuggingFace Hub upload API documentation

**What to do:**

1. Add upload capability to `HubClient`:

```rust
impl HubClient {
    /// Push a training data JSONL file to a HuggingFace dataset repository.
    pub async fn push_training_data(
        &self,
        dataset_id: &str,       // e.g., "my-org/roko-training-data"
        split: &str,            // e.g., "train"
        jsonl_content: &str,    // JSONL string
        commit_message: &str,
    ) -> Result<PushResult> {
        // Use the HF Hub API to:
        // 1. Create the dataset repo if it does not exist
        // 2. Upload the JSONL file as `data/{split}.jsonl`
        // 3. Create or update the dataset card
    }
}

pub struct PushResult {
    pub dataset_id: String,
    pub commit_sha: String,
    pub num_examples: usize,
    pub file_size_bytes: usize,
}
```

2. Implement automatic dataset card generation with training data statistics.
3. Support incremental push: append to existing split file, do not overwrite.

**Test:**
- Mock HTTP: push 100 training examples. Assert commit created.
- Mock HTTP: push to non-existent dataset. Assert dataset created first.
- JSONL content matches expected format.

- [ ] `push_training_data()` uploads JSONL to HF Hub
- [ ] Dataset repo created if missing
- [ ] Incremental append to existing splits
- [ ] Dataset card auto-generated

---

### Task 10.3: Implement AutoTrain trigger (push -> fine-tune -> model -> Hub)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/fine_tune_loop.rs` (new file)

**Read first:**
- Task 10.1, 10.2 output
- `crates/roko-hf/src/autotrain.rs` -- `AutoTrainClient`
- `crates/roko-learn/src/cascade_router.rs` -- `CascadeRouter`

**What to do:**

1. Create `crates/roko-learn/src/fine_tune_loop.rs`.
2. Define the orchestration:

```rust
/// The fine-tuning loop: episodes -> training data -> Hub -> AutoTrain -> new model -> CascadeRouter.
pub struct FineTuneLoop {
    extractor: TrainingDataExtractor,
    hub: HubClient,
    autotrain: AutoTrainClient,
    config: FineTuneConfig,
}

pub struct FineTuneConfig {
    /// Minimum training examples before triggering a fine-tune.
    pub min_examples: usize,         // default 500
    /// HuggingFace dataset ID for training data.
    pub dataset_id: String,
    /// Base model to fine-tune from.
    pub base_model: String,          // e.g., "claude-haiku-4-5" (or equivalent HF model)
    /// HuggingFace model ID for the output.
    pub output_model_id: String,
    /// Whether to auto-trigger training when enough data accumulates.
    pub auto_trigger: bool,
}
```

3. Implement `FineTuneLoop::check_and_trigger(&mut self, episodes: &[Episode]) -> Result<Option<TrainingJob>>`:
   - Extract training data from episodes
   - If count >= `min_examples`:
     - Push to HuggingFace dataset
     - Trigger AutoTrain job
     - Return the job handle
   - Otherwise return None

4. Implement `FineTuneLoop::poll_and_integrate(&mut self, job: &TrainingJob, router: &mut CascadeRouter) -> Result<bool>`:
   - Poll job status
   - If completed: discover the new model on Hub, add as CascadeRouter arm
   - Return true if integrated

5. Wire into the orchestrator: after plan completion, call `check_and_trigger()`. Periodically poll active jobs.

6. Register in `crates/roko-learn/src/lib.rs`.

**Files to modify:**
- `crates/roko-learn/src/fine_tune_loop.rs` (new)
- `crates/roko-learn/src/lib.rs`
- `crates/roko-cli/src/orchestrate.rs` (wire trigger)

**Test:**
- 600 episodes, 500 pass gates. Assert training triggered (>= 500 examples).
- 300 episodes, all pass. Assert training not triggered (< 500 examples).
- Mock AutoTrain returns completed. Assert new model added to CascadeRouter.

- [ ] `FineTuneLoop` orchestrates extract -> push -> train -> integrate
- [ ] Triggers when enough training data accumulates
- [ ] Polls training job and integrates result into CascadeRouter
- [ ] Wired into orchestrator post-plan hook

---

### Task 10.4: Implement CascadeRouter model discovery (scan Hub for new models)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`

**Read first:**
- `crates/roko-learn/src/cascade_router.rs` -- existing `CascadeRouter`
- `crates/roko-hf/src/hub.rs` -- `HubClient::discover_fine_tuned()`
- Task 10.3 output

**What to do:**

1. Add periodic model discovery to `CascadeRouter`:

```rust
impl CascadeRouter {
    /// Scan HuggingFace Hub for new fine-tuned models and add them
    /// as new routing arms with neutral priors.
    pub async fn discover_and_integrate(
        &mut self,
        hub: &HubClient,
        org: &str,
    ) -> Result<Vec<String>> {
        let models = hub.discover_fine_tuned(&self.base_model).await?;
        let mut added = Vec::new();
        for model in models {
            if !self.has_arm(&model.model_id) {
                let arm = RouterArm {
                    model: model.model_id.clone(),
                    stage: RoutingStage::Confidence,
                    alpha: 1.0,  // neutral prior
                    beta: 1.0,
                    cost_per_1k: estimate_cost(&model),
                };
                self.add_arm(arm);
                added.push(model.model_id);
            }
        }
        if !added.is_empty() {
            self.save()?;
            tracing::info!(count = added.len(), models = ?added, "discovered new fine-tuned models");
        }
        Ok(added)
    }
}
```

2. Schedule discovery: run after every plan completion and on a timer (every 6 hours).
3. Persist the router state after adding new arms.

**Test:**
- Hub returns 2 new models. Assert CascadeRouter gains 2 arms with neutral priors.
- Hub returns models already known. Assert no duplicates.
- Router state persisted to `.roko/learn/cascade-router.json` after discovery.

- [ ] `discover_and_integrate()` adds new models as routing arms
- [ ] Neutral priors (alpha=1, beta=1) for new arms
- [ ] No duplicate arms
- [ ] State persisted after discovery

---

### Task 10.5: Integration test -- fine-tuned model appears as CascadeRouter arm

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/tests/fine_tune_integration.rs` (new file)

**Read first:**
- Tasks 10.1 through 10.4

**Do:**

1. **End-to-end scenario:**
   - Create 600 mock episodes (500 pass all gates, 100 fail)
   - Run `FineTuneLoop::check_and_trigger()` -> assert training triggered with 500 examples
   - Mock AutoTrain returns completed with model "my-org/roko-haiku-ft-v1"
   - Run `poll_and_integrate()` -> assert returns true
   - Assert CascadeRouter now has a new arm for "my-org/roko-haiku-ft-v1"
   - Assert the new arm has neutral priors (alpha=1, beta=1)
   - Run 10 tasks through CascadeRouter -> assert the new arm receives some traffic (UCB exploration)
   - Assert `.roko/learn/cascade-router.json` includes the new arm

2. **Incremental scenario:**
   - Run with 300 episodes -> no training triggered
   - Add 200 more episodes (total 500 passing) -> training triggered
   - Assert second run builds on first (cumulative training data)

3. Run: `cargo test -p roko-learn --test fine_tune_integration`

- [ ] End-to-end: episodes -> training data -> Hub -> AutoTrain -> CascadeRouter arm
- [ ] New arm receives exploration traffic via UCB
- [ ] Incremental training data accumulation
- [ ] State persistence verified
- [ ] All integration tests pass

---

## Acceptance criteria

- [ ] `roko install crate:<name>` downloads, builds, and registers a Rust extension
- [ ] `roko install npm:<name>` downloads and loads a Pi-compatible JS extension
- [ ] `roko install git:<url>` clones and auto-detects package type
- [ ] `roko install <local-path>` symlinks and loads a local package
- [ ] `roko remove <name>` unlinks and cleans up
- [ ] `roko ls` shows installed packages with type, version, scope
- [ ] `.roko/packages.lock` records exact versions for reproducibility
- [ ] Pi npm packages install and run in roko: tools register, events fire
- [ ] Multi-domain agent with `--profile blockchain,research` gets merged tool sets and gates
- [ ] Per-task domain routing selects correct gate pipeline
- [ ] `[[chains]]` config in roko.toml parsed and used to spawn chain actors
- [ ] Multi-chain agent subscribes to 3+ chains simultaneously
- [ ] CanonicalEventBus merges events from all chains into one stream
- [ ] FinalityTracker detects reorgs for EVM chains
- [ ] FinalityTracker handles deterministic finality for Hyperliquid
- [ ] TemporalAggregator provides R0/R1/R2/R3 hierarchical views
- [ ] Contract discovery pipeline classifies 75%+ of known DeFi protocols
- [ ] ERC-165 detection works for standard interfaces
- [ ] Factory tracker detects PairCreated and PoolCreated events
- [ ] Gittins index computation ranks high-reward entities above low-reward
- [ ] AttentionBudget allocates proportional to Gittins index
- [ ] HabituationMask suppresses repetitive benign events
- [ ] MVT patch switching triggers when marginal returns diminish
- [ ] Foraging model reduces monitoring cost by 50%+ while maintaining coverage
- [ ] WorldGraph discovers entities and relationships from CanonicalEvents
- [ ] WorldGraph HDC fingerprint changes when graph topology changes
- [ ] WorldGraph context injection produces relevant context sections via VCG bidding
- [ ] Dream consolidation updates WorldGraph classifications
- [ ] All new crates pass `cargo clippy --no-deps -- -D warnings`
- [ ] All new crates pass `cargo +nightly fmt --all`

---

## Dependency graph

```
Phase 1 (registry lib)
  |
  v
Phase 2 (CLI commands) ----> Phase 3 (QuickJS) ----> Phase 4 (native extensions)
                                                        |
                                                        v
                                                  Phase 5 (multi-domain)
                                                        |
Phase 6 (multi-chain) ---------------------------> Phase 7 (contract discovery)
        |                                               |
        v                                               v
Phase 8 (foraging + worldgraph) -----------------> Phase 9 (integration tests)
```

Phases 1-4 can proceed in parallel with Phases 6-7. Phase 5 depends on Phase 4. Phase 8 depends on Phases 6-7. Phase 9 depends on everything.

---

## New files created by this plan

| Phase | Crate | Files |
|-------|-------|-------|
| 1 | roko-ext-registry | `Cargo.toml`, `src/lib.rs`, `src/manifest.rs`, `src/lockfile.rs`, `src/storage.rs`, `src/resolver.rs`, `src/installer.rs`, `src/registry.rs` |
| 2 | roko-cli | `src/package.rs` (new) |
| 3 | roko-quickjs | `Cargo.toml`, `src/lib.rs`, `src/runtime.rs`, `src/bridge.rs`, `src/sandbox.rs`, `src/pi_api.rs` |
| 3 | roko-ext-registry | `src/skill_loader.rs`, `src/prompt_loader.rs`, `src/theme_loader.rs` |
| 4 | roko-ext-registry | `src/extension_loader.rs`, `src/profile_loader.rs`, `src/arena_loader.rs`, `src/chain_loader.rs` |
| 6 | roko-chain-ingest | `Cargo.toml`, `src/lib.rs`, `src/connector.rs`, `src/connector/evm.rs`, `src/connector/hyperliquid.rs`, `src/canonical.rs`, `src/actor.rs`, `src/bus.rs`, `src/finality.rs`, `src/temporal.rs`, `src/config.rs` |
| 7 | roko-chain-ingest | `src/discovery/mod.rs`, `src/discovery/erc165.rs`, `src/discovery/selectors.rs`, `src/discovery/similarity.rs`, `src/discovery/patterns.rs`, `src/discovery/factories.rs`, `src/discovery/insights.rs`, `src/discovery/registry.rs` |
| 8 | roko-foraging | `Cargo.toml`, `src/lib.rs`, `src/gittins.rs`, `src/budget.rs`, `src/habituation.rs`, `src/mvt.rs` |
| 8 | roko-worldgraph | `Cargo.toml`, `src/lib.rs`, `src/graph.rs`, `src/entity.rs`, `src/relationship.rs`, `src/fingerprint.rs`, `src/context.rs` |

## Existing files modified by this plan

| Phase | File | Change |
|-------|------|--------|
| 1 | `/Users/will/dev/nunchi/roko/roko/Cargo.toml` | Add 5 new crate members |
| 2 | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` | Add Install/Remove/List/Search/Publish subcommands |
| 2 | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/Cargo.toml` | Add roko-ext-registry dependency |
| 5 | `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/domain_profile.rs` | Add ComposedProfile, MergeStrategy |
| 6 | `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` | Add ChainConfig, `chains: Vec<ChainConfig>` to RokoConfig |
| 9 | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` | Wire package loading, multi-chain bus, foraging model, WorldGraph bidder |
