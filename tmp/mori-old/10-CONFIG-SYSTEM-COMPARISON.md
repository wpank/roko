# Mori Configuration System -- Detailed Analysis and Roko Comparison

Source files examined:

| File | System |
|---|---|
| `/Users/will/dev/uniswap/bardo/apps/mori/src/state/config.rs` | Mori `ConfigState` (~1800 LOC) |
| `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/roles.rs` | Mori `AgentRole` + `AgentBackend` |
| `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/views/config.rs` | Mori F6:cfg TUI renderer |
| `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/views/context.rs` | Mori MCP/Context TUI renderer |
| `/Users/will/dev/uniswap/bardo/.mori/config.toml` | Active Mori repo config |
| `/Users/will/dev/uniswap/bardo/.mori/config.toml.example` | Mori config example |
| `/Users/will/dev/uniswap/bardo/.mori/mcp-config.json` | Mori MCP server config |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` | Roko `RokoConfig` schema |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/loader.rs` | Roko unified loader |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/provider.rs` | Roko `ProviderConfig` + `ModelProfile` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config.rs` | Roko CLI config (`Config`) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/model_routing.rs` | Roko model routing pipeline |
| `/Users/will/dev/nunchi/roko/roko/roko.toml` | Active Roko project config |

---

## 1. Backend Defaults

### Mori

Mori has six dedicated "Backend Defaults" fields displayed in F6:cfg Section 0:

```
 --- Backend Defaults ---
 Codex default:    < claude-sonnet-4-6 >
 Cursor default:   < composer-2-fast >
 Claude default:   < claude-sonnet-4-6 >
 Conductor model:  < claude-sonnet-4-6 > [cl]
 Fallback model:   < claude-sonnet-4-6 > [cl]
 Force one model:  [x] / [ ]
 Forced model:     < claude-sonnet-4-6 >
 Disabled providers: (none)
```

Each backend (Codex, Cursor, Claude) has its own default model field. The conductor has
a dedicated model separate from the three backend defaults. Implementation:

```rust
// ConfigState fields
pub codex_default_model: String,        // "claude-sonnet-4-6" (active config)
pub cursor_default_model: String,       // "composer-2-fast"
pub claude_default_model: String,       // "claude-sonnet-4-6"
pub conductor_model: String,            // "claude-sonnet-4-6"
pub fallback_model: Option<String>,     // fallback on spawn failure
pub global_model_override_enabled: bool, // "Force one model" toggle
pub global_model_override_model: String, // slug used when override is on
pub disabled_providers: Vec<String>,    // e.g. ["codex"]
```

Hard-coded defaults (in `Default` impl):
- `codex_default_model`: read from `~/.codex/config.toml`, else `"gpt-5.4-mini"`
- `cursor_default_model`: `"composer-2-fast"`
- `claude_default_model`: `"claude-haiku-4-5"`
- `conductor_model`: `"claude-sonnet-4-6"`

The active config (`.mori/config.toml`) overrides all four to `claude-sonnet-4-6`.

### Roko

Roko uses a single default model + provider rather than three backend-specific defaults:

```toml
[agent]
default_model = "claude-sonnet"
default_backend = "claude"
default_effort = "medium"
```

The model is resolved through the `[models]` and `[providers]` registries:

```toml
[providers.claude_cli]
kind = "claude_cli"
command = "claude"

[models.claude-sonnet]
provider = "claude_cli"
slug = "claude-sonnet-4-6"
context_window = 200000
```

Roko's provider system is extensible (11 provider kinds), while Mori's three-backend
system was hard-coded to Codex/Cursor/Claude.

**Key difference**: Mori had one model field per backend (3 backends). Roko has one
default model + an open-ended provider/model registry. The Mori approach was simpler to
reason about in the TUI but required code changes to add a new backend; Roko's approach
is declarative and extensible but does not surface a compact "defaults per backend" view.

---

## 2. Per-Role Overrides

### Mori

Mori has 28 agent roles (1 Conductor + 27 in `ALL_AGENTS`), each displayed in the TUI:

```
 --- Per-Role Overrides ---
 conductor:  (cl: cl-sonnet-4-6)      <- dim, inherited from conductor_model
 strat:      (cl: cl-sonnet-4-6)      <- dim, inherited from claude default
 impl:       (cl: cl-sonnet-4-6)      <- dim, inherited
 arch:       (cd: cl-sonnet-4-6)      <- dim, inherited from codex default
 audit:      (cl: cl-sonnet-4-6)
 scribe:     < cl-sonnet-4-6 > [cl]   <- BRIGHT, explicit role_models override
 critic:     (cl: cl-sonnet-4-6)
 refac:      (cd: cl-sonnet-4-6)
 prepl:      (cd: cl-sonnet-4-6)
 docvf:      (cd: cl-sonnet-4-6)
 itest:      (cd: cl-sonnet-4-6)
 merge:      (cd: cl-sonnet-4-6)
 tval:       (cd: cl-sonnet-4-6)
 glct:       (cd: cl-sonnet-4-6)
 sdrf:       (cd: cl-sonnet-4-6)
 regd:       (cd: cl-sonnet-4-6)
 perf:       (cd: cl-sonnet-4-6)
 covr:       (cd: cl-sonnet-4-6)
 plcm:       (cd: cl-sonnet-4-6)
 xsys:       (cd: cl-sonnet-4-6)
 errdx:      (cd: cl-sonnet-4-6)
 rsrch:      (cl: cl-sonnet-4-6)
 depv:       (cd: cl-sonnet-4-6)
 patrn:      (cd: cl-sonnet-4-6)
 snapc:      (cd: cl-sonnet-4-6)
 afix:       (cl: cl-sonnet-4-6)
 qrev:       (cl: cl-sonnet-4-6)
 FLV:        (cl: cl-sonnet-4-6)
```

The full role list with abbreviations:

| Short | Full label | Default backend |
|---|---|---|
| `cond` | conductor | Claude |
| `strat` | strategist | Claude |
| `impl` | implementer | Claude |
| `arch` | architect | Codex |
| `audit` | auditor | Claude |
| `scribe` | scribe | Claude |
| `critic` | critic | Claude |
| `refac` | refactorer | Codex |
| `prepl` | pre-planner | Codex |
| `docvf` | doc-verifier | Codex |
| `itest` | integration-tester | Codex |
| `merge` | merge-resolver | Codex |
| `tval` | terminal-validator | Codex |
| `glct` | golem-lifecycle-tester | Codex |
| `sdrf` | spec-drift-detector | Codex |
| `regd` | regression-detector | Codex |
| `perf` | performance-sentinel | Codex |
| `covr` | coverage-tracker | Codex |
| `plcm` | plan-lifecycle-mgr | Codex |
| `xsys` | cross-system-tester | Codex |
| `errdx` | error-diagnoser | Codex |
| `rsrch` | researcher | Claude |
| `depv` | dep-validator | Codex |
| `patrn` | pattern-extractor | Codex |
| `snapc` | snapshot-comparator | Codex |
| `afix` | auto-fixer | Claude |
| `qrev` | quick-reviewer | Claude |
| `FLV` | full-loop-validator | Claude |

The resolution logic in `model_for()`:

1. If `global_model_override_enabled` and its provider is not disabled, return the override model
2. If role is `Conductor`, return `conductor_model` (dedicated field)
3. If `role_models` HashMap has an explicit override for `role.label()`, return it
4. Otherwise, get the role's native backend (`role.backend()`), reroute if disabled,
   return `default_model_for_backend(effective_backend)`

Stored in config.toml as:

```toml
[role_models]
scribe = "claude-sonnet-4-6"
# (other overrides omitted = inherit from backend default)
```

### Roko

Roko does not have a fixed enum of 28 roles. It has a more generic approach:

```toml
[agent.roles]    # currently empty in roko.toml
[agent.defaults] # currently empty
```

Domain profiles provide per-use-case overrides:

```toml
[profiles.coding]
name = "coding"
effort = "high"
max_iterations = 3
tool_profile = "full"

[profiles.research]
name = "research"
effort = "medium"
context_limit_k = 200
```

The routing system uses task complexity bands instead of named roles:

```toml
[routing]
fast_task_model = "claude-haiku-4-5"
standard_task_model = "claude-sonnet-4-6"
complex_task_model = "claude-opus-4-6"
```

**Key difference**: Mori mapped each of 28 concrete roles to a model by name in a HashMap.
Roko routes by task-tier/complexity-band/domain-profile rather than by named role. Mori's
approach gave fine-grained per-role control visible in the TUI; Roko's approach is more
general but less directly mappable to "which model does the architect use?"

---

## 3. Provider/Backend Notation

### Mori

The TUI displays `(cl: cl-sonnet-4-6)` and `(cd: cl-sonnet-4-6)` with specific semantics:

```
(cl: cl-sonnet-4-6)   <- inherited default (dim text)
< cl-sonnet-4-6 > [cl] <- explicit override (bright text)
```

The two-letter prefix is the **backend abbreviation**:

| Prefix | Backend | Full name |
|---|---|---|
| `cd` | Codex | OpenAI Codex CLI |
| `cx` | Cursor | Cursor ACP / Cursor CLI |
| `cl` | Claude | Claude Code CLI (Anthropic) |

Implemented by `AgentBackend::short()`:

```rust
pub fn short(&self) -> &'static str {
    match self {
        Self::Codex => "cd",
        Self::Cursor => "cx",
        Self::Claude => "cl",
    }
}
```

The model slug is shortened for display by `shorten_model()`:

```rust
fn shorten_model(slug: &str) -> String {
    slug.replace("gpt-", "")
        .replace("-codex", "c")
        .replace("-mini", "m")
        .replace("composer-", "cx-")
        .replace("claude-", "cl-")
}
```

So `claude-sonnet-4-6` becomes `cl-sonnet-4-6`. The backend is inferred from the model
slug by `AgentBackend::from_model()`:

- `claude-*` -> Claude
- `composer-*`, `cursor-*`, `auto`, `sonnet-*`, `opus-*`, `gemini-*`, `kimi-*` -> Cursor
- Everything else (gpt-*, o3, o4-mini) -> Codex

When a role is **inherited** (no explicit override), the TUI shows the value in dim/ghost
text with parentheses: `(cd: cl-sonnet-4-6)`. When **overridden**, it shows in bright text
with angle brackets and a badge: `< cl-sonnet-4-6 > [cl]`.

### Roko

Roko uses `ProviderKind` enum with 11 variants:

```rust
pub enum ProviderKind {
    AnthropicApi,
    ClaudeCli,
    OpenAiCompat,
    CursorAcp,
    CursorCli,
    PerplexityApi,
    GeminiApi,
    GeminiCli,
    CerebrasApi,
    Hermes,
    OpenClaw,
}
```

Provider/model binding is declarative in config rather than inferred from slug:

```toml
[providers.claude_cli]
kind = "claude_cli"
command = "claude"

[models.claude-sonnet]
provider = "claude_cli"        # explicit binding, not inferred
slug = "claude-sonnet-4-6"
```

---

## 4. Agent Status Display and Priority Levels

### Mori

The F6:cfg tab shows an **Agent Status** panel on the right side. For each of the 27
`ALL_AGENTS` roles, it shows:

```
 ● implementer  cl-sonnet-4-6  45k/200k  22%  t3  high
 ○ architect     cl-sonnet-4-6   0/200k   0%  t0  medium
 · scribe        cl-sonnet-4-6   0/200k   0%  t0  low
```

The columns are:

| Column | Source |
|---|---|
| Status icon | `●` active, `○` has tokens, `·` idle |
| Role name | `role.label()` (12 chars padded) |
| Short model | `shorten_model(cfg_model)` |
| Token usage | `input_tokens/context_limit_k` |
| Context pct | percentage of context window used |
| Turn count | `t{turns}` |
| Effort | `effort_for(role).label()` |

**Priority levels** are derived from the **effort** configuration, not a separate "priority"
field. The `effort_for()` function assigns default effort per role:

```rust
pub fn effort_for(&self, role: AgentRole) -> ReasoningEffort {
    // Per-role effort override from config
    if let Some(&effort) = self.role_effort.get(role.label()) {
        return effort;
    }
    // Role-specific defaults:
    match role {
        // HIGH: core creation -- must one-shot code/strategy/architecture
        Implementer | Strategist | Architect => High,
        PrePlanner | DependencyValidator => High,
        PatternExtractor => High,
        // MEDIUM: review & validation
        Auditor | Critic | Researcher | Conductor | AutoFixer
        | FullLoopValidator | DocVerifier => Medium,
        // LOW: lightweight bookkeeping
        Scribe | QuickReviewer => Low,
        // DEFAULT: everything else uses global default_effort
        _ => self.default_effort,
    }
}
```

The four levels are `Low`, `Medium`, `High`, `Max`. These map to reasoning effort
parameters passed to the LLM providers (Codex `model_reasoning_effort`, Claude `--effort`).
Cursor ignores effort entirely.

The TUI shows the effort in the rightmost column of each agent row. When the role's backend
supports effort (`effort_configurable()` returns true for Codex and Claude), it shows
colored text; otherwise it shows "N/A" in ghost text.

### Roko

Roko does not have an equivalent "agent status" panel in its TUI. The runner dispatches
with effort from config:

```toml
[agent]
default_effort = "medium"
```

Effort is set globally rather than per-role. Per-role effort overrides are not part of
the current Roko config schema.

---

## 5. Context & Effort Section

### Mori

F6:cfg Section 2 ("Context & Effort") manages:

```
 --- Context & Effort ---
 Global context limit:  < 150k >
 strat ctx:   (default: 150k)
 impl ctx:    (default: 150k)
 arch ctx:    (default: 150k)
 ... (27 per-role context limits)
 Reasoning effort:  < Medium >
```

Implementation:

```rust
pub context_limit_k: u32,                        // global, in thousands
pub role_context_k: HashMap<String, u32>,         // per-role overrides
pub default_effort: ReasoningEffort,              // Low/Medium/High/Max
pub role_effort: HashMap<String, ReasoningEffort>, // per-role effort overrides
pub context_pressure_pct: u32,                    // threshold for pressure detection
```

The `context_limit_for()` function returns tokens (role override * 1000, or global * 1000):

```rust
pub fn context_limit_for(&self, role: AgentRole) -> u64 {
    self.role_context_k
        .get(role.label())
        .copied()
        .unwrap_or(self.context_limit_k) as u64
        * 1000
}
```

### Roko

Roko has a single global context limit:

```toml
[agent]
context_limit_k = 200
```

Per-role context overrides are not in the schema. Domain profiles can override
`context_limit_k` for specific use cases.

---

## 6. Config Hierarchy

### Mori

Two-tier hierarchy (repo override vs global defaults):

```
Precedence (high to low):
1. .mori/config.toml         (repo-local)
2. $XDG_CONFIG_HOME/mori/config.toml  (global, or ~/.config/mori/config.toml)
3. dirs::config_dir()/mori/config.toml (legacy platform path)
4. ConfigState::default()     (hard-coded defaults)
```

Important: this is **not a merge** -- the first file found wins entirely. If the repo
config exists, the global config is ignored. No field-level merge.

The TUI shows the active source:

```
Source: repo override                              (green)
Repo config is active and overrides your global Mori defaults.

Source: global default                             (green)
Using global Mori defaults. Saving here creates a repo-only override.

Source: not configured                             (yellow warning)
No Mori config found yet. Run `mori setup` or save from this screen.
```

Additionally, CLI flags can override config:

```rust
pub fn from_full_app_config(config: &AppConfig) -> Self {
    // --model, --skip-tests, --max-iterations, --no-docs, --no-review, --fast
    // --max-agents, --max-parallel-plans, --parallel, --pre-plan, --express
    // --fallback-model, --preset
}
```

And execution presets adjust multiple fields at once:

- `"quality"`: opus model, 5 iterations, clippy on
- `"balanced"`: sonnet model, 3 iterations
- `"cost"`: sonnet default, haiku for critic/scribe/auditor, 2 iterations, tokens profile
- `"speed"`: sonnet default, express mode, 2 auto-fix attempts, latency profile

Queue-level overrides (`queue.toml`) can also adjust routing/model/strategy settings
per milestone run.

### Roko

Five-tier hierarchy with true field-level merge:

```
Precedence (highest wins):
1. Named env var overrides (ROKO_MODEL, ROKO_BACKEND, ROKO_EFFORT, etc.)
2. Hierarchical env overrides (ROKO__SECTION__FIELD=value)
3. ROKO_CONFIG env var (explicit file path)
4. roko.toml (ancestor walk from workdir)
5. ~/.roko/config.toml (global -- providers/models merged, not overridden)
6. RokoConfig::default() (built-in defaults)
```

Important differences from Mori:

- **Field-level merge**: global `~/.roko/config.toml` providers and models are merged
  into the project config, not replaced. This means a user can define providers globally
  and use them in any project.
- **Environment variable overrides**: both named (`ROKO_MODEL`) and hierarchical
  (`ROKO__AGENT__DEFAULT_MODEL`) patterns are supported.
- **Schema versioning**: `config_version` and `schema_version` fields with migration support.
- **Provenance tracking**: the loader records which source each field came from.
- **Validation**: seven invariants checked, with configurable strict mode.
- **Secret interpolation**: `ROKO__*` patterns support secret references.

```rust
pub struct LoadOptions {
    pub merge_global: bool,
    pub apply_env_overrides: bool,
    pub apply_hierarchical_env: bool,
    pub strict_validation: bool,
}
```

---

## 7. MCP/Context Section

### Mori

The F6:cfg tab embeds a compact "MCP / Context" panel in the bottom-right. The full MCP
Context tab (F7) has three sub-panels:

**Compact view (bottom of F6:cfg):**

```
 MCP  READY  mcp_first  refresh 12s ago
 codex:on  claude:on  cursor:on
 1.2k files  8.3k syms  42 mcp calls
```

The `backend_span()` function renders each backend's on/off status:

```rust
fn backend_span<'a>(label: &str, enabled: bool) -> Span<'a> {
    let color = if enabled { Theme::SAGE } else { Theme::WARNING };
    let mark = if enabled { "on" } else { "off" };
    Span::styled(format!("{label}:{mark}"), ...)
}
```

**Full context view (F7) has three columns:**

1. **Servers / Roots**: claude repo/wt config paths, codex repo/wt config paths, cursor
   repo/wt config paths, launch command, root path, index db path -- each with ok/miss status
2. **AST Index**: files, symbols, references, resolved %, density, routing coverage gauge
3. **Tool / Learning**: episodes, playbook rules, routing coverage, rich routing, hints,
   prompt stats, artifacts, registries, knowledge utilization, tool call counts per backend

The MCP config itself is simple JSON:

```json
{
  "mcpServers": {
    "mori": {
      "command": "mori-mcp",
      "args": ["context-server", "--root", "."]
    }
  }
}
```

### Roko

Roko's MCP config is part of the provider system:

```toml
[providers.claude_cli]
kind = "claude_cli"
command = "claude"
```

MCP tools are configured through:

```toml
[tools]
allow = []
deny = []
```

Roko does not have the equivalent of the three-backend on/off toggle display. It supports
MCP server configuration through `.mcp.json` files (auto-discovered or explicit path in
`agent.mcp_config`).

---

## 8. Config File Format

### Mori

- **Format**: TOML
- **Location**: `.mori/config.toml` (repo), `~/.config/mori/config.toml` (global)
- **Serialization**: `toml::from_str` / `toml::to_string_pretty`
- **Backward compat**: `#[serde(alias = "default_model")]` for old field names
- **No schema version** -- implicit migration through serde defaults
- **Flat structure** with `[role_models]`, `[role_effort]`, `[role_context_k]`,
  `[plan_overrides]` sections

Active config example:

```toml
codex_default_model = "claude-sonnet-4-6"
cursor_default_model = "composer-2-fast"
claude_default_model = "claude-sonnet-4-6"
conductor_model = "claude-sonnet-4-6"
context_limit_k = 150
default_effort = "Medium"
auto_advance_batch = true
auto_merge_on_complete = true
architect_enabled = true
auditor_enabled = true
scribe_enabled = true
critic_enabled = true
skip_tests = false
max_iterations = 5
clippy_enabled = false
context_pressure_pct = 70
max_agents = 20
max_parallel_plans = 5
fresh_base_branch = "main-fresh"
allow_main_merge_from_tui = false
parallel_enabled = true
pre_plan = false
fast_mode = false
express_mode = true
agent_bare_mode = false
max_auto_fix_attempts = 2
auto_fix_model = "claude-sonnet-4-6"
fallback_model = "claude-sonnet-4-6"
global_model_override_enabled = false
global_model_override_model = "claude-sonnet-4-6"
optimization_profile = "balanced"
context_strategy = "mcp_first"
routing_mode = "auto_override"
warm_implementers_per_plan = 1
fast_task_model = "claude-haiku-4-5-20251001"
standard_task_model = "claude-sonnet-4-6"
complex_task_model = "claude-opus-4-6"
fast_task_provider = "claude"
standard_task_provider = "claude"
complex_task_provider = "claude"
auto_playbook_refresh = true
explicit_routing_backfill = false
auto_verify_artifact_refresh = false
auto_research_prepass = true
auto_generate_research = true
auto_generate_dependencies = true
auto_generate_fixtures = true
auto_generate_integration = true
auto_refresh_downstream = true
auto_start_fixtures = true
enabled_fixture_kinds = ["headless-terminal", "mirage-evm", "mock-http"]
max_fixture_concurrency = 3
learning_min_occurrences = 2
disabled_providers = []

[role_models]
# conductor downgraded to sonnet -- read-only role, doesn't need opus

[role_effort]
[role_context_k]
[plan_overrides]
```

### Roko

- **Format**: TOML
- **Location**: `roko.toml` (project root, ancestor walk), `~/.roko/config.toml` (global)
- **Schema version**: explicit `config_version = 2`, `schema_version = 2`
- **Migration**: `ConfigMigrator` with registered v1->v2 edge
- **Deeply nested** sections: `[agent]`, `[providers.*]`, `[models.*]`, `[routing]`,
  `[pipeline.*]`, `[budget]`, `[conductor]`, `[learning]`, `[timeouts]`, `[serve]`,
  `[gates]`, `[tools]`, etc.
- **Provenance tracking**: each field knows its source (file, env, default, migration)
- **Validation invariants**: seven checks with configurable severity

---

## Summary Comparison Table

| Feature | Mori | Roko |
|---|---|---|
| **Config format** | TOML flat | TOML nested sections |
| **Config locations** | `.mori/config.toml`, `~/.config/mori/config.toml` | `roko.toml` (ancestor walk), `~/.roko/config.toml` |
| **Merge strategy** | First file wins (no merge) | Field-level merge (global providers/models into project) |
| **Schema version** | None | `schema_version = 2` with migration pipeline |
| **Env overrides** | CLI flags only | Named (`ROKO_MODEL`) + hierarchical (`ROKO__*`) |
| **Provenance** | `loaded_from_repo` bool | Full per-field provenance tracking |
| **Backend count** | 3 hard-coded (Codex, Cursor, Claude) | 11 provider kinds, declarative |
| **Backend notation** | `cd`/`cx`/`cl` shortcodes | Provider name from registry |
| **Default models** | 3 per-backend + conductor + fallback | 1 default + registry lookup |
| **Per-role overrides** | 28 named roles, HashMap | Task-tier/domain-profile based |
| **Per-role context** | HashMap per role (27 entries) | Global only (profiles can override) |
| **Per-role effort** | 4-level enum per role | Global effort string |
| **Agent toggles** | 4 booleans (architect/auditor/scribe/critic) | No direct equivalent |
| **Force override** | Toggle + model slug | `force_backend` CLI/config |
| **Disabled providers** | `Vec<String>` with fallback rerouting | Provider health registry (learned) |
| **Complexity bands** | fast/standard/complex models + providers | Same concept in `[routing]` |
| **Presets** | quality/balanced/cost/speed | Domain profiles (coding/research/review) |
| **Pipeline tiers** | Express mode toggle | mechanical/focused/integrative/architectural |
| **Execution controls** | ~30 toggle/value rows in TUI | Separate sections in roko.toml |
| **MCP config** | Separate `.mori/mcp-config.json` | `agent.mcp_config` field or auto-discover |
| **MCP backend toggle** | codex:on/off, claude:on/off, cursor:on/off | Not applicable (provider-based) |
| **TUI config view** | F6:cfg with 5 navigable sections + Apply | F6 tab exists but no interactive config editing |
| **Hot reload** | Save from TUI, model snapshot diff | Transactional reload with freshness tracking |
| **Queue overrides** | `.mori/queue.toml` with per-milestone settings | Not applicable |
| **Cost model routing** | `model_cost_rank()` (opus=3/sonnet=2/haiku=1) | CascadeRouter with LinUCB bandit |
| **Knowledge injection** | 6 toggles + 5 tuning params | Same toggles in `[learning]` |
| **Fixture management** | 3 fixture kinds, concurrency, auto-start | Not applicable |
| **Bare mode** | `agent_bare_mode` toggle | `agent.bare_mode` |
| **Auto-advance** | batch + plan auto-advance toggles | Runner handles plan advancement |

### What Mori Had That Roko Could Adopt

1. **Per-role model overrides in a flat HashMap** -- simple and visible in TUI
2. **Per-role context limits** -- the `role_context_k` HashMap gave fine-grained control
3. **Per-role effort defaults** -- the hardcoded effort tiers per role were well-tuned
4. **Disabled-provider rerouting** -- explicit list + deterministic fallback order
5. **Execution presets** -- "quality"/"balanced"/"cost"/"speed" as one-command config changes
6. **Interactive config editing in TUI** -- j/k navigation, h/l cycle values, Enter to toggle
7. **Queue.toml milestone definitions** -- per-milestone model/routing/strategy overrides

### What Roko Has That Mori Lacked

1. **Declarative provider registry** -- 11 provider kinds vs 3 hard-coded
2. **Field-level global merge** -- global providers/models carry into any project
3. **Environment variable overrides** -- both named and hierarchical
4. **Schema versioning and migration** -- forward-compatible config evolution
5. **Provenance tracking** -- know exactly where each config value came from
6. **Domain profile inheritance** -- profiles can inherit from other profiles
7. **Validation invariants** -- seven checks at load time
8. **Transactional config reload** -- atomic swap with freshness tracking
9. **Budget enforcement** -- per-plan, per-turn, per-task, per-session USD limits
10. **Timeout configuration** -- 9 separate timeout categories with defaults
