# Configuration, Tools, Signals & Events Audit

roko.toml config system, 30 builtin tools (16 std + 14 chain-domain) with role profiles, 32-kind signal taxonomy, plugin SDK — the foundational plumbing that everything else depends on.

## The Problem

These subsystems are foundational and mostly well-designed. The issues: config hot-reload is implemented for 7 sections but not wired to a file-watcher trigger in the daemon, builtin tools have no versioning or metrics collection, the signal system has 32 kinds but emotional_tag and decay functions are never tuned, and the plugin system has no isolation (one panicking plugin takes down the bus).

---

## 1. Configuration (roko-cli)

### File Sizes

| File | LOC | Purpose |
|---|---|---|
| config.rs | 3,803 | Schema, loading, validation, wizard |
| config_cmd.rs | 1,823 | CLI command handlers |
| config_helpers.rs | 295 | Helper utilities |
| **Total** | **5,921** | |

Note: Previously reported as 61K — **actual is ~6K LOC**. The 61K figure was file size in bytes, not lines.

### roko.toml Structure

The actual `RokoConfig` struct (in `roko-core/src/config/schema.rs`) maps to these top-level TOML sections:

```toml
[project]       # Project name, workspace root, language, tags
[prd]           # PRD auto-plan trigger settings
[agent]         # Backend selection, model, effort, temperament, per-role overrides
[providers.*]   # Per-provider config (api_key, base_url, timeout_ms, models)
[models.*]      # Per-model profiles (context_window, pricing, capabilities)
[gates]         # Gate pipeline (compile, test, clippy, shell_true, custom)
[routing]       # CascadeRouter algorithm, weights, overrides
[pipeline]      # Multi-band pipeline configuration
[budget]        # max_plan_usd, max_turn_usd, cost_overrun degradation
[conductor]     # Meta-orchestrator thresholds and watcher settings
[watcher]       # Filesystem watcher paths and filters
[learning]      # Episode logging, playbook extraction, replan_on_gate_failure
[demurrage]     # Knowledge decay rate and balance floor
[attention]     # Attention bidder configuration
[immune]        # Immune system alert thresholds
[temporal]      # Temporal reasoning config
[goals]         # Goal tracking settings
[energy]        # Energy model config
[tui]           # TUI display preferences
[serve]         # HTTP server: port, TLS, auth, deploy settings
[scheduler]     # Cron schedule config
[webhooks]      # Webhook ingestion settings
[subscriptions] # Event subscription configs (array)
[server]        # Agent server sidecar settings
[deploy]        # Cloud deployment (Railway, Fly, Docker)
[perplexity]    # Perplexity API settings
[gemini]        # Gemini API settings
[tools]         # Tool allowlist/denylist and per-domain profiles
[oneirography]  # Dream/consolidation cycle settings (replaces [dreams])
[chain]         # Chain/DeFi integration settings
[relay]         # Relay configuration
[[agents]]      # Per-agent definition blocks (name, role, domain, mcp_config)
```

**Sections that do NOT exist** (previously listed incorrectly): `[executor]`, `[runtime]`, `[prompt]`, `[dreams]`, `[daimon]`, `[[repos]]`. These concepts map to `[learning]`, `[pipeline]`/`[conductor]`, `[agent]`, `[oneirography]`, and `[[agents]]` respectively.

### Config Load Path

The actual config load chain in `roko-cli/src/config.rs`:

1. **Global** (`~/.config/roko/config.toml` — resolved via `config_paths().global`)
2. **Project** (`./roko.toml` — resolved via `load_config(workdir)` in `roko-core`)
3. **CLI flags** (override individual fields, merged via `apply_layer_value` pattern)
4. **Environment variables** (`${VAR}` interpolation run after parse, also `*_file` secret resolution)
5. **Detection** (auto-detect installed LLM CLIs on `roko init`)

Layers 1-2 are partial configs (`ConfigLayer` with all fields `Option<T>`), merged together before being materialized into a full `RokoConfig`. The `apply_layer_value` function handles individual key overrides from `--set` flags or env.

### Config Wizard (`roko init`)

- Detects installed LLM CLIs (Claude, Cursor, Codex, OpenAI, Ollama, Gemini, Perplexity)
- Prompts for token budget, role persona, gate enablement
- Non-interactive mode via `WizardInputs` (for CI)

### Anti-Patterns

| Issue | Details |
|---|---|
| **Hot-reload not triggered** | `hot_reload.rs` implements `apply_hot_reload` for 7 sections (budget/tools/learning/demurrage/gates/conductor/routing) but no file-watcher calls it; a process restart is still required in practice |
| **Provider routing not learned** | `force_backend` overrides don't feed back to CascadeRouter |
| **Sparse wizard coverage** | `oneirography`, `chain`, `temporal`, `goals`, and `energy` sections not covered by `roko init` wizard |
| **Schema version at 2** | `CURRENT_SCHEMA_VERSION = 2` / `CURRENT_CONFIG_VERSION = 2`; `compat.rs` reads legacy Mori format but migration between Roko schema versions is not stress-tested |

---

## 2. Builtin Tools (roko-std)

### 30 Builtin Tools (16 std + 14 chain-domain)

`TOOL_COUNT = 30` per `roko-std/src/tool/builtin/mod.rs`. The registry is `ROKO_BUILTIN_TOOLS: LazyLock<Vec<ToolDef>>`, materializing 16 std tools then extending with `CHAIN_DOMAIN_TOOLS` (14 entries from `roko-chain`).

**16 std tools** (registered in `roko-std/src/tool/builtin/mod.rs`):

| Tool | Purpose |
|---|---|
| read_file | Read file contents with line range |
| write_file | Create/overwrite files |
| edit_file | Single-file edit with line replacement |
| multi_edit | Batch edits across multiple files |
| glob | File pattern matching |
| grep | Regex search over files |
| bash | Execute shell commands with timeout |
| ls | Directory listing |
| web_fetch | Fetch and summarize web pages |
| web_search | Perplexity search integration |
| notebook_edit | Jupyter notebook editing |
| todo_write | Write task/note entries |
| task (task_agent) | Spawn nested agent tasks |
| exit_plan_mode | Signal plan completion |
| apply_patch | Apply unified diffs |
| run_tests | Run test suites (language-agnostic) |

**14 chain-domain tools** (from `roko-chain/src/tools.rs`, prefixed `chain.`):

`chain.balance`, `chain.transfer`, `chain.approve`, `chain.swap`, `chain.add_liquidity`, `chain.remove_liquidity`, `chain.get_pool_info`, `chain.get_position`, `chain.simulate_tx`, `chain.gas_estimate`, `chain.wallet_create`, `chain.wallet_list`, `chain.wallet_info`, `chain.wallet_export_address`

### Role-Based Tool Profiles (5 archetypes)

Defined in `roko-std/src/roles.rs` as `RoleToolProfile` constants:

| Role | Allowed (allowlist) | Denied |
|---|---|---|
| Implementer | No allowlist — all registry tools | None |
| Researcher | read_file, grep, glob, web_search, web_fetch | write_file, edit_file, bash |
| Reviewer | read_file, grep, glob, web_search, web_fetch, todo_write | write_file, edit_file |
| Strategist | read_file, grep, glob, web_search, web_fetch, todo_write, exit_plan_mode, task_agent | write_file, edit_file, multi_edit, apply_patch, notebook_edit, bash, run_tests |
| Scribe | read_file, grep, glob, web_search, web_fetch, write_file, edit_file, multi_edit, apply_patch, todo_write | bash, run_tests |

Note: `Reviewer` denies `write_file, edit_file` (not `multi_edit` or `apply_patch`). Strategist denies all 7 `DESTRUCTIVE_TOOLS`.

### Domain Profiles (4 domains)

Defined in `roko-std/src/roles.rs` as `DomainToolProfile` constants:

| Domain | Extra | Excluded |
|---|---|---|
| Coding | All 16 std tools | None |
| Chain | read_file, grep, glob, bash, web_fetch, web_search | write_file, edit_file, multi_edit, apply_patch, notebook_edit |
| Research | read_file, grep, glob, web_search, web_fetch, todo_write | write_file, edit_file, multi_edit, apply_patch, notebook_edit, bash, run_tests |
| General | None (passthrough) | None |

**Composition:** `effective = (role_allowed ∪ domain_extra) \ (role_denied ∪ domain_excluded ∪ override_deny)`

The `compose_profile` function in `roko-std/src/roles.rs` implements this exactly. Config-level `ToolsConfig.allow`/`.deny` act as an additional override layer on top.

### Tool Registration

```rust
// roko-std/src/tool/builtin/mod.rs
pub const TOOL_COUNT: usize = 30;  // 16 std + 14 chain

pub static ROKO_BUILTIN_TOOLS: LazyLock<Vec<ToolDef>> = LazyLock::new(|| {
    let mut tools = vec![ /* 16 std tool_def() calls */ ];
    tools.extend(CHAIN_DOMAIN_TOOLS.iter().cloned());  // 14 chain tools
    tools
});

// roko-std/src/tool/registry.rs
pub struct StaticToolRegistry;  // zero-sized; lookups are linear scan over ROKO_BUILTIN_TOOLS
```

`ROKO_BUILTIN_TOOLS` is a `Vec<ToolDef>` (not `&[ToolDef]`), materialized once via `LazyLock` on first access.

### Anti-Patterns

| Issue | Details |
|---|---|
| **No tool versioning** | ToolDef schema is frozen; no compatibility shims |
| **No tool metrics** | Call count, latency, error rate not auto-collected |
| **No tool quota** | No max calls per agent per task |
| **Hard-coded domain profiles** | Only 4 fixed domains (`coding`, `chain`, `research`, `general`); `ToolsConfig.profiles` map allows config-driven overrides per domain but the canonical 4 are compile-time constants in `roko-std` |
| **sandbox.rs exists but is not in TOOL_COUNT** | `roko-std/src/tool/builtin/sandbox.rs` exists but its tool is not in the `ROKO_BUILTIN_TOOLS` vec nor counted in `TOOL_COUNT = 30` |
| **chain.* tools always present** | 14 chain tools are included in every agent's registry even for non-chain tasks; no flag to strip them for non-chain agents |

---

## 3. Signal/Event System (roko-core)

### Engram (Signal Wrapper)

```rust
Engram {
    id: ContentHash,                    // BLAKE3(kind|body|author|tags|lineage)
    fingerprint: Option<HdcFingerprint>,// Semantic vector
    kind: Kind,                         // 32 variants
    body: Body,                         // Payload (text, JSON, binary)
    created_at_ms: i64,
    decay: Decay,                       // Weight decay function
    provenance: Provenance,             // Author + taint flag
    score: Score,                       // Multi-dimensional confidence
    lineage: Vec<ContentHash>,          // DAG of parent engrams
    tags: BTreeMap<String, String>,
    attestation: Option<Attestation>,   // Cryptographic proof
    emotional_tag: Option<EmotionalTag>,// Somatic marker (rarely populated)
}
```

### 32 Signal Kinds

**Agent Runtime:** ProcessSpawn, ProcessExit, AgentMessage, AgentOutput, TokenUsage, ApprovalRequested

**Verification:** GateVerdict, TestResult, CompileDiagnostic

**Tasks & Plans:** Task, Plan, PlanPhase

**Context:** PromptSection, ContextPack, Prompt

**Routing & Learning:** RouterChoice, RouterFeedback

**Memory:** Episode, PlaybookRule, Skill, Compound(Vec<Kind>)

**Observability:** Metric, ExperimentResult, ToolInvocation, ToolHealthDegraded

**Chain:** Insight, Pheromone, Bounty, Transaction, Service, Prediction

**Extension:** Custom(String)

### The 6 Universal Traits

| Trait | Verb | What It Does |
|---|---|---|
| Store (Substrate) | Persist | put/get/query/prune engrams |
| Score (Scorer) | Rate | score engram along dimensions |
| Verify (Gate) | Check | async verify correctness |
| Route (Router) | Select | pick one from candidates + feedback |
| Compose (Composer) | Combine | merge engrams within budget |
| React (Policy) | Watch | stream → interventions |

### Pulse Bus (Event Transport)

```rust
// roko-core/src/pulse_bus.rs
PulseBus {
    inner: Arc<EventBus<Pulse>>,  // Ring buffer + broadcast
}

// roko-core/src/pulse.rs
Pulse {
    seq: u64,               // Monotonic sequence number assigned by the Bus
    topic: Topic,           // "gate.compile", "agent.turn", etc.
    kind: Kind,
    body: Body,
    created_at_ms: i64,
    tags: BTreeMap<String, String>,
}
```

**Features:**
- Topic-based filtering via `TopicFilter::Prefix("gate.")`, `TopicFilter::Exact(...)`, or `TopicFilter::All`
- Sequence numbers (`seq: u64`) per pulse for gap detection
- `PulseBusReceiver` filters silently; lagged receivers log a warning
- `replay_from(after_seq, filter)` for catch-up on reconnect
- Pulse → Engram promotion via `Engram::from_pulse_synthetic` / `Engram::from_pulses` (one-way; no Engram → Pulse path exists)

### Design Strengths

- Content-addressed identity (immutable, hash-based)
- Decay contract (weight fades over time per Kind)
- Lineage DAG (every derived signal tracks parents)
- HDC fingerprinting for similarity queries

### Anti-Patterns

| Issue | Details |
|---|---|
| **Pulse → Engram is lossy** | `from_pulse_synthetic` loses the `seq` field; `from_pulses` concatenates bodies and merges tags (last writer wins). There is no Engram → Pulse path. |
| **No signal versioning** | `Kind` enum is `#[non_exhaustive]`; `Custom(String)` is the only extension point. No version field on `Engram`. |
| **emotional_tag rarely populated** | Field exists in `Engram` builder but nothing in the orchestration path sets it |
| **Decay not learned** | Default is `Decay::None`; `Decay::GATE_VERDICT` is the one named constant. Decay values are hardcoded per call site, not tuned per-task |
| **Lineage explosion** | `derive_verdict` deduplicates but lineage grows with every derivation; no depth limit in `EngramBuilder.lineage` |

---

## 4. Plugin System (roko-plugin)

### Size: 1,663 LOC (lib.rs: 1,080 + manifest.rs: 583)

### Event Source Trait

```rust
trait EventSource: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> EventSourceKind;  // Webhook, Cron, FileWatch, Custom
    async fn start(&self, sender: SignalSender, cancel: CancellationToken);
}
```

### Implementations

| Source | What It Does | Implemented |
|---|---|---|
| CronEventSource | Scheduled tasks via cron expressions | Yes — `CronEventSource` struct in lib.rs |
| FileWatchEventSource | Directory monitoring via `notify` crate | Yes — `FileWatchEventSource` struct in lib.rs |
| WebhookEventSource | HTTP POST ingestion | No — `EventSourceKind::Webhook` variant exists but no `WebhookEventSource` struct is implemented |

### Feedback System

```rust
enum FeedbackOutcome {
    Approved, Rejected, Commented, Ignored, Merged
}
```

### Anti-Patterns

| Issue | Details |
|---|---|
| **No rate limiting** | FileWatch and Cron can emit unbounded signals |
| **No plugin isolation** | Plugins run in same process; panic = crash |
| **Feedback mapping lossy** | GitHub events map to only 5 outcomes |
| **No plugin versioning** | No API compatibility check |
| **No manifest security** | Any TOML in .roko/plugins/ loaded without verification |

---

## 5. MCP Integrations

| Crate | LOC | Tools | Status |
|---|---|---|---|
| roko-mcp-code | 1,935 (lib.rs=1930, main.rs=5) | 14 code intelligence tools | Wired |
| roko-mcp-github | 3,192 | 19 GitHub tools (list_prs, get_pr, create_pr, comment_pr, review_pr, merge_pr, list_issues, create_issue, comment_issue, close_issue, add_labels, create_label, get_file, search_code, list_commits, create_branch, get_branch, compare_branches, get_actions_status) | Partial |
| roko-mcp-slack | 1,114 | **4** Slack tools (post_message, list_channels, get_channel_history, update_message) | Partial |
| roko-mcp-scripts | 758 | 2 script tools (run_script, list_scripts) | Wired |
| roko-mcp-stdio | 251 | JSON-RPC framing (infrastructure) | Active |

**roko-mcp-code tools (14):** `search_code`, `get_symbol_context`, `get_file_ast`, `find_similar_patterns`, `get_index_stats`, `find_references`, `find_implementations`, `get_callers`, `workspace_map`, `get_context`, `symbol_lookup`, `call_graph`, `imports`, `semantic_search`

### MCP Anti-Patterns

| Issue | Details |
|---|---|
| **Naming inconsistency** | GitHub: `github.snake_case`; Slack: `slack.snake_case`; scripts use bare names (`run_script`, not `scripts.run_script`) |
| **No rate limiting** | GitHub tools can hammer API with no backoff |
| **No version negotiation** | All hardcode MCP protocol version in initialize response |
| **No retry logic** | Failed API calls return generic error |
| **Unidirectional feedback** | Write to external systems but don't pull signals back |
| **roko-mcp-slack under-implemented** | Audit previously claimed 9 tools; actual dispatch has only 4 (post, list_channels, get_history, update) |

---

## 6. File Inventory

### Configuration
| File | LOC |
|---|---|
| `roko-cli/src/config.rs` | 3,803 |
| `roko-cli/src/config_cmd.rs` | 1,823 |
| `roko-cli/src/config_helpers.rs` | 295 |
| `roko-core/src/config/` (17 modules) | (part of roko-core total) |
| **Key schema file** | `roko-core/src/config/schema.rs` — `RokoConfig` with all section structs |
| **Hot-reload** | `roko-core/src/config/hot_reload.rs` — `apply_hot_reload`, `config_diff` |

### Builtin Tools
| File | LOC |
|---|---|
| `roko-std/src/lib.rs` | 37 |
| `roko-std/src/tool/builtin/mod.rs` | 94 |
| `roko-std/src/tool/builtin/*.rs` (16 tool files + sandbox) | 3,211 |
| `roko-std/src/tool/registry.rs` | 245 |
| `roko-std/src/roles.rs` | 603 |
| `roko-chain/src/tools.rs` | 758 (14 chain tools) |
| **roko-std total** | **5,869** |

### Signal System
| File | LOC |
|---|---|
| `roko-core/src/` (53 modules) | 24,376 |
| Key: `engram.rs`, `signal.rs` (alias), `kind.rs`, `pulse.rs`, `pulse_bus.rs`, `body.rs`, `decay.rs` | |

### Plugin System
| File | LOC |
|---|---|
| `roko-plugin/src/lib.rs` | 1,080 |
| `roko-plugin/src/manifest.rs` | 583 |

### MCP Integrations
| Crate | LOC |
|---|---|
| roko-mcp-code | 1,935 (lib.rs=1930 + main.rs=5) |
| roko-mcp-github | 3,192 |
| roko-mcp-slack | 1,114 |
| roko-mcp-scripts | 758 |
| roko-mcp-stdio | 251 |

---

## Sources

Every claim above was verified against the following source files:

| Topic | Source file |
|---|---|
| Config schema (`RokoConfig`) | `crates/roko-core/src/config/schema.rs` |
| Config sections (all structs) | `crates/roko-core/src/config/{agent,budget,gates,learning,tools,serve,routing,project,subscriptions}.rs` |
| Hot-reload implementation | `crates/roko-core/src/config/hot_reload.rs` |
| Config load + layers | `crates/roko-cli/src/config.rs` |
| Tool count (`TOOL_COUNT = 30`) | `crates/roko-std/src/tool/builtin/mod.rs` |
| Tool registry (`StaticToolRegistry`) | `crates/roko-std/src/tool/registry.rs` |
| Role tool profiles (5 archetypes) | `crates/roko-std/src/roles.rs` |
| Domain tool profiles (4 domains) | `crates/roko-std/src/roles.rs` |
| Chain tools (14 tools) | `crates/roko-chain/src/tools.rs` |
| `Kind` enum (32 variants) | `crates/roko-core/src/kind.rs` |
| `Engram` struct | `crates/roko-core/src/engram.rs` |
| `Signal` alias | `crates/roko-core/src/signal.rs` |
| `Pulse` / `PulseBus` / `TopicFilter` | `crates/roko-core/src/pulse.rs`, `crates/roko-core/src/pulse_bus.rs` |
| Plugin SDK | `crates/roko-plugin/src/lib.rs`, `crates/roko-plugin/src/manifest.rs` |
| roko-mcp-code tools (14) | `crates/roko-mcp-code/src/lib.rs` |
| roko-mcp-github tools (19) | `crates/roko-mcp-github/src/main.rs` |
| roko-mcp-slack tools (4) | `crates/roko-mcp-slack/src/main.rs` |
| roko-mcp-scripts tools (2) | `crates/roko-mcp-scripts/src/main.rs` |
| AgentRole enum (28 variants) | `crates/roko-core/src/agent.rs` |
