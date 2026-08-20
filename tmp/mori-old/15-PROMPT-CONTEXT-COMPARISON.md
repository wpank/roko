# Prompt Assembly & Context Engineering: Mori vs Roko

## Executive Summary

Mori and Roko share the same fundamental design philosophy -- role-specific system
prompts, per-section character budgets, cache-layer ordering for prefix-cache
alignment, and a learning-feedback loop that adjusts prompt content based on
gate outcomes. The transition from Mori to Roko involved refactoring Mori's
monolithic 2000-line `prompts.rs` into a clean, layered architecture with typed
inputs, separated concerns, and a bidder-auction composition model.

---

## 1. Architecture Comparison

### Mori: Monolithic Prompt Assembly

**Core files:**
- `apps/mori/src/orchestrator/prompts.rs` (~2000 lines) -- budget tables, section
  assembly, all role-specific prompt builders, shared prefix, context pack caching,
  truncation, and prompt logging in one module.
- `apps/mori/src/orchestrator/context.rs` (~810 lines) -- filesystem artifact
  management: workspace maps, preflight snapshots, PRD extraction, context
  bundles, summaries, iteration archival.
- `apps/mori/src/orchestrator/prompt_log.rs` (~220 lines) -- prompt capture to
  `.mori/memory/prompt-logs/` with tiktoken cl100k token counting.
- `apps/mori/src/orchestrator/skills.rs` (~130 lines) -- skill injection from
  `.claude/skills/` directories.
- `apps/mori/src/orchestrator/memory.rs` -- learning pack, research pack,
  playbook hints, episode statistics.
- `apps/mori/src/support_enrich/prompts.rs` (~650 lines) -- enrichment step
  prompts (briefs, tasks, verify, review, decompose, research, etc.).

The pattern: Mori's prompt assembly is **procedural and filesystem-coupled**.
Each role has a dedicated function (`implementer_prompt()`,
`implementer_prompt_with_brief()`, `implementer_fix_prompt()`, etc.) that reads
files, truncates them, and concatenates strings with format macros.

### Roko: Layered Composition Architecture

**Core files:**
- `crates/roko-compose/src/system_prompt_builder.rs` (~2300 lines) -- the 9-layer
  `SystemPromptBuilder` and `RoleSystemPromptSpec` wrapper. No filesystem I/O --
  all content arrives via builder methods.
- `crates/roko-compose/src/templates/` (11 role templates) -- `ImplementerTemplate`,
  `StrategistTemplate`, `ReviewerTemplate`, `ScribeTemplate`, etc. Each implements
  `RolePromptTemplate` trait with typed `*Input` structs.
- `crates/roko-compose/src/prompt.rs` -- `PromptComposer`, `PromptSection`,
  `AttentionBidder`, `CacheLayer`, `Placement` types.
- `crates/roko-compose/src/auction.rs` -- `LearningBidder` with Thompson-sampling
  bids for budget allocation per subsystem.
- `crates/roko-compose/src/templates/common.rs` -- shared budget tables,
  `adaptive_budget_for()`, stanza constants.
- `crates/roko-cli/src/dispatch/prompt_builder.rs` (~500 lines) -- `PromptAssembler`
  bridge between the runner and compose layer.
- `crates/roko-cli/src/prompting.rs` -- `build_role_system_prompt()` entry point.

The pattern: Roko's prompt assembly is **trait-based, I/O-free at the compose
layer, and auction-driven**. The compose crate owns typed sections; the CLI
dispatch layer owns filesystem reads and runner integration.

---

## 2. System Prompt Layers

### Mori: Informal Layering

Mori's prompt sections have an implicit ordering enforced by `cache_layer`
values (1=role, 2=workspace, 3=plan, 0=volatile) and a priority (1-5) ranking.
The `assemble_prompt()` function sorts sections by priority for inclusion, then
by cache_layer for output ordering:

```
Layer 1 (cache_layer=1): AGENTS.md conventions -- stable across all agents
Layer 2 (cache_layer=2): workspace-map.md, preflight-snapshot.md
Layer 3 (cache_layer=3): plan content, PRD2 extract, strategist brief
Layer 0 (cache_layer=0): tasks, reviews, context packs, registry -- volatile
```

The `SharedPlanContext` struct captures the shared prefix once per plan and
`format_shared_prefix()` renders it in a fixed order for cache hits.

### Roko: Explicit 9-Layer System

Roko formalizes this into 9 named layers with distinct stability tiers:

```
Layer 1: Role identity          -- "You are the Implementer. Your job is..."
Layer 2: Conventions            -- Project coding standards, naming rules
Layer 3: Domain context         -- Project-specific knowledge, workspace map
Layer 3b: Assembled context     -- Context chunks from bidders
Layer 3c: Active signals        -- Pheromone/stigmergic guidance
Layer 4: Task context           -- Current task details, acceptance criteria
Layer 4b: Gate feedback         -- Prior verification failure digest (retry)
Layer 5: Tool instructions      -- Available tools and usage guidance
Layer 6: Relevant techniques    -- Learned playbooks, skills, tool hints
Layer 7: Anti-patterns          -- "Never call unwrap() in library crates"
Layer 8: Affect guidance        -- Emotional tone and focus from Daimon
```

Each layer maps to a `CacheLayer` enum (`Role`, `Workspace`, `Plan`, `Volatile`)
and a `Placement` enum (`Start`, `Middle`, `End`) for U-shaped attention
optimization. Cache alignment markers (`<!-- cache:TIER -->`) are inserted
between stability tiers when enabled.

**Key difference**: Mori conflates role identity and conventions into the
AGENTS.md blob. Roko separates them into distinct layers (1 and 2) with
different stability characteristics -- role identity is system-stable while
conventions are semi-stable and can be per-project.

---

## 3. Per-Role Budget Tables

Both systems use identical budget fields:

| Field | Description |
|-------|-------------|
| `plan` | Plan markdown content cap |
| `workspace_map` | Crate file tree cap |
| `prd2` | PRD specification extract cap |
| `context` | Cross-plan context cap |
| `brief` | Strategist brief cap |
| `reviews` | Prior review feedback cap |
| `instructions` | Instruction block cap |
| `file_context` | Inline file context cap |
| `skills` | Playbook/skill library cap |

### Mori Budget Table

```
Role          plan    map    prd2   ctx   brief  reviews  inst   files  skills
Implementer   50,000  20,000 12,000 4,000 8,000  3,000    4,000  8,000  8,000
Strategist    50,000  20,000 12,000 4,000 6,000  3,000    4,000  0      4,000
Arch/Auditor  50,000  6,000  6,000  2,000 4,000  3,000    4,000  6,000  4,000
Scribe        50,000  6,000  16,000 4,000 6,000  3,000    4,000  6,000  6,000
Critic        50,000  6,000  6,000  4,000 6,000  3,000    4,000  6,000  4,000
Default       50,000  8,000  6,000  4,000 4,000  2,000    4,000  6,000  4,000
```

Total budget per role: ~115k chars (Implementer) down to ~88k chars (Default).

### Roko Budget Table

Roko carries the same values forward with two additions:

- `QuickReviewer` role with minimal caps (prd2=0, context=0, file_context=0)
- `AutoFixer` role with bare-minimum caps (only instructions=2000; all else 0)

**Adaptive budgets**: Roko adds `adaptive_budget_for(role, model_context_tokens)`
which scales each field proportionally to the model's context window relative to
a 200k-token baseline. The scale factor is clamped between 0.25x and 2.0x to
prevent extreme shrinking or expansion.

Mori's `budget_for()` accepts a model name parameter but ignores it -- budgets
are fixed regardless of model.

---

## 4. Context Strategies (Mori-specific)

Mori has three `ContextStrategy` modes that control how much context is embedded
in the prompt vs. deferred to MCP tool calls:

| Strategy | Description |
|----------|-------------|
| `McpFirst` | Minimal inline context; agents fetch via `workspace_map()`, `get_plan_context()` MCP tools |
| `Hybrid` | Moderate inline context with MCP available for deep dives |
| `InlineHeavy` | Maximum inline context; all artifacts embedded in prompt |

The strategy is configurable per-plan, per-task (via `context_weight`), and
adaptively promoted when tasks fail:

```
research_before_edit → promote to Hybrid
quality=hardened → promote to InlineHeavy
category=Integration → promote to Hybrid
speed=throughput → cap at McpFirst
```

**Roko equivalent**: Roko does not have an explicit ContextStrategy enum. Instead,
the attention bidder auction (`AttentionBidder` + `LearningBidder`) dynamically
decides what gets included based on Thompson-sampling bids. The VCG allocation
mechanism replaces the discrete strategy modes with a continuous budget market
where subsystems compete for prompt tokens.

---

## 5. AST Index Integration

### Mori: `mori-index` Crate

`mori-index` is a SQLite-backed AST index with HDC fingerprinting:

- **Parser**: `parser.rs` uses `syn` to parse Rust source files and extract
  symbols (structs, enums, functions, traits, impls, constants).
- **Database**: `db.rs` stores symbols in SQLite with keyword search (LIKE),
  structural search (by kind/visibility), and HDC similarity search.
- **Graph**: `graph.rs` builds a `SymbolGraph` with PageRank scoring for
  symbol importance.
- **Context overlay**: `context_overlay.rs` provides per-worktree and per-agent
  transient overlays that shadow search results without mutating the base index.
- **Fingerprinting**: `fingerprint.rs` generates HDC vectors for structural
  similarity matching.
- **Privacy**: `privacy.rs` implements `RedactionPolicy` and `RetrievalSurface`
  to control what the index exposes to different callers.

The index is exposed to agents via MCP tools:
```
search_code(query: "Name")         -- keyword symbol search
get_symbol_context(symbol_name: T) -- signature, file, docs
get_file_ast(file_path: path)      -- list symbols in a file
find_similar_patterns(symbol: X)   -- HDC similarity search
find_references(symbol_name: X)    -- callers/importers via graph
```

Mori's `MCP_TOOLS_STANZA` is injected into every implementer prompt to steer
agents toward MCP tool calls over `rg`:

> "Use MCP tools instead of `rg` for symbol lookup"

### Roko: `roko-index` + `roko-mcp-code`

Roko has a similar architecture split across:
- `roko-index` -- parser, graph, HDC indexing
- `roko-mcp-code` -- MCP server exposing code intelligence
- `roko-primitives` -- HDC vectors and tier routing

The `MCP_TOOLS_STANZA` in Roko's common template is more generic:

> "You have MCP server tools. Use them for file reading, searching, and navigation
> instead of shelling out."

**Key difference**: Roko's context bidding system (`AttentionBidder::CodeIntelligence`)
can inject code-index results directly into the system prompt as context chunks,
rather than relying solely on the agent to call MCP tools. This is more proactive
-- the system pre-fetches relevant symbols and includes them in the prompt, while
still making MCP available for deeper exploration.

---

## 6. Context Management Per Agent

### Mori: ~150k Token Budget, Explicit Gauges

Mori's TUI (F7: Context tab) shows real-time context utilization:

- **Context gauge widget**: `tui/widgets/context_gauge.rs` renders a horizontal
  bar with threshold markers at 80% and 90% fill. Color gradients from green
  through yellow to red indicate pressure.
- **Index panel**: Shows file count, symbol count, and MCP call count from the
  `mori-index` database.
- **Backend indicators**: Shows which agent backends (Codex, Claude, Cursor) are
  enabled with MCP support.

The prompt stats from F7:inspect (based on episode data):
- **35.7 kB average prompt size** -- total system prompt bytes
- **1.3 kB average inline context** -- context embedded in the prompt
- **8.6 kB average context pack** -- learning/research/playbook pack bytes

These come from the `Episode` struct which records `inline_context_bytes` and
`context_pack_bytes` per agent invocation. The `memory.rs` module computes
rolling averages across episodes.

### Roko: Auction-Based, Observable

Roko's context management differs:

1. **PromptAssembler** (`dispatch/prompt_builder.rs`) constructs a `PromptContext`
   from the task and dispatch context, including workspace_map (20k chars max),
   tasks.toml (10k chars max), PRD excerpt (2k chars max), workspace context
   (4k chars max), and c-factor context.

2. **Diagnostics**: Each assembled prompt produces `PromptDiagnostics` that track:
   - Which sections were included/dropped
   - Total token estimate
   - Playbook IDs and knowledge IDs used
   - Scored signal list
   - Section-level audit trail (`PromptSectionAudit`)

3. **Attention bidders**: Nine subsystems compete for prompt budget:
   ```
   Neuro           -- durable knowledge from the neuro store
   Daimon          -- affect/somatic guidance
   IterationMemory -- recent turns, retries, prior outputs
   CodeIntelligence-- symbols, files, workspace structure
   PlaybookRules   -- skills, playbooks, distilled rules
   Research        -- research memos, external domain context
   TaskContext     -- task brief, plan brief, verification, PRD
   Oracles         -- predictions, warnings, forecasts
   GroupContext     -- group knowledge and pheromone signals
   ```

4. **VCG allocation**: When composition strategy is `Auto` and sufficient
   observations exist, the `vcg_allocate()` mechanism runs a Vickrey-Clarke-Groves
   auction across subsystem bids. Each bidder uses a Thompson-style posterior
   mean + exploration offset, so sections that historically correlate with gate
   passes receive higher bids.

5. **Observable via telemetry**: The E33 Lens runtime provides 39 production
   event variants; prompt composition outcomes are part of the observation
   pipeline. Section diagnostics are stored per attempt and linked to gate
   outcomes for the learning loop.

---

## 7. Research Artifact Injection

### Mori

Research artifacts flow into prompts through the context pack system:

1. **Research pre-pass**: When `task.research_before_edit = true` or
   `task.category = Research|Integration|Verification` or
   `task.quality_profile = Hardened`, the `build_learning_context_pack()` function
   loads `render_plan_research_md()`.

2. **Content sources**: The research pack includes:
   - `research.md` from the plan directory (truncated to 5000 chars)
   - `integration.md` (truncated to 3600 chars)
   - `dependency-manifest.toml` (truncated to 2800 chars)
   - `fixture-manifest.toml` (truncated to 2800 chars)

3. **Caching**: Context packs are SHA-256 keyed and cached both in memory
   (`CONTEXT_PACK_CACHE`) and on disk (`.mori/memory/context-packs/*.json`).
   The cache key includes file mtimes and episode/playbook signatures to
   invalidate when upstream artifacts change.

4. **Enrichment pipeline**: `support_enrich/prompts.rs` defines system prompts
   for 9 enrichment steps (briefs, tasks, verify, review, decompose, research,
   dependencies, fixtures, integration) that pre-generate these artifacts before
   the implementer runs.

### Roko

Research injection uses the bidder system:

1. **AttentionBidder::Research**: The research subsystem bids for prompt budget
   through the auction mechanism.

2. **PRD excerpt**: Loaded from `.roko/prd/published/{plan_id}.md` or
   `.roko/prd/draft/{plan_id}.md`, truncated to 2000 chars, and injected as
   part of `PromptContext`.

3. **Context chunks**: The `ContextChunk` type carries labeled, scored context
   fragments from any source (neuro store, research, code index). These are
   injected via `SystemPromptBuilder::with_context()` or
   `with_pheromones()`.

4. **Plan generation**: `roko prd plan <slug>` generates implementation plans
   from PRDs using an agent. Research can be enhanced via
   `roko research enhance-prd <slug>`.

**Key difference**: Mori's research injection is pre-computed and cached on disk;
Roko's is dynamically bid for at prompt-assembly time. Mori has a richer
enrichment pipeline (9 distinct steps) because it was built for a specific
large-scale Rust project; Roko generalizes the pattern into configurable bidders.

---

## 8. Plan/Task Context Assembly

### Mori

Plan context assembly follows a fixed pipeline:

1. **Preflight**: `write_preflight_files()` generates `workspace-map.md` and
   `preflight-snapshot.md` from git log, git status, and crate directory scans.

2. **Shared prefix**: `build_shared_context()` reads plan content, PRD2 extract,
   workspace map, cross-plan context, and brief once per plan. All agents in the
   same plan get the same byte-identical prefix for cache hits.

3. **Per-role dispatch**: Role-specific functions add task-specific content:
   - `implementer_prompt()` -- plan + PRD2 + verify tasks + verify chain
   - `implementer_prompt_with_brief()` -- adds brief + task checklist + reviews +
     iteration notes + skill section + agent messages
   - `implementer_fix_prompt()` -- stripped context, focused on errors/issues

4. **Context injection**: When worktrees are used, `context/in/` receives mirrored
   artifacts (`execution-pack.md`, role-specific packs, brief, etc.) with
   per-role guidance on which artifacts to open first.

5. **Prior task outputs**: `compress_prior_task_outputs()` extracts error/warning
   lines from prior task outputs (max 3 outputs, 8 relevant lines each) for
   retry context.

6. **Completion summaries**: `read_completion_summaries()` aggregates summaries
   from all completed plans into a registry snapshot.

### Roko

Plan/task context assembly uses the typed template system:

1. **PromptContext**: `PromptContext::from_task()` constructs context from the
   runner's `TaskDef` and `DispatchContext`:
   ```
   plan_id, role, workdir, files_in_scope, acceptance_criteria,
   verify_commands, gate_feedback, attempt, prompt_experiment,
   workspace_map, tasks_toml, prd_excerpt, dependency_outputs,
   workspace_context, cfactor_context
   ```

2. **TaskContext**: The compose layer receives `TaskContext` (which includes
   task description, files, acceptance criteria, verify steps, domain notes,
   and plan brief) and passes it through `RoleSystemPromptSpec`.

3. **Template dispatch**: Each role template has a typed `*Input` struct:
   - `ImplementerInput` -- agents_md, plan, brief, tasks, workspace_map,
     preflight, registry_snapshot, prev_reviews, verify_chain, invariants,
     task_enhancements
   - `StrategistInput` -- plan, workspace_map, completed_plans, reviews
   - `ReviewerInput` -- plan, workspace_map, brief, diff, acceptance, variant
   - `TaskImplInput` -- task definition, sibling tasks, gate feedback, plan

4. **Section building**: Templates emit `Vec<PromptSection>` with typed
   priority, cache layer, placement, bidder, and hard cap. The `PromptComposer`
   assembles these under the token budget.

**Key difference**: Mori builds prompts by concatenating strings in role-specific
functions. Roko builds prompts by composing typed sections through a trait-based
template system. This means Roko prompts are testable without filesystem access
and can be validated statically.

---

## 9. HDC Fingerprint Integration

### Mori

Mori uses HDC fingerprints for two purposes:
- **Index similarity search**: `search_similar()` in `mori-index` computes
  cosine similarity between symbol fingerprints.
- **Episode recording**: Episodes record fingerprints for similarity queries,
  but there is no evidence of HDC fingerprints being used for prompt-time
  context retrieval in the codebase.

### Roko

Roko integrates HDC fingerprints into the prompt assembly pipeline:

1. **Per-episode fingerprint**: The `EpisodeLogger` records an `hdc_fingerprint`
   field computed from the task title and content.

2. **Similar-episode injection**: At dispatch time, the event loop:
   - Computes the task's HDC fingerprint from its title
   - Queries past episodes for similar work via
     `EpisodeLogger::query_similar_episodes()`
   - Appends matching episodes as supplementary context in the system prompt
   - Respects per-role limits: strategist-class roles get 0 episodes,
     reviewer-class get up to 5, implementer-class get up to 3

3. **Tier routing**: `roko-primitives` provides HDC-based tier routing that
   maps tasks to complexity bands, influencing model selection.

---

## 10. Playbook / When-Then Rule Injection

### Mori

Playbook injection works through `playbook_hints_for_scope()`:

1. **Matching**: The playbook TOML file (`.mori/memory/playbook.toml`) contains
   `PlaybookRule` entries with `when` conditions (file patterns, tags) and
   `then` actions (instructions, commands).

2. **Scope derivation**: `derive_prompt_scope()` extracts files and tags from the
   task (category, reasoning level, speed priority, quality profile, context
   weight, complexity band, provider, plan section, skills, dependency tags,
   fixture keys, sidecar requirements, integration surfaces, crate/app tags,
   surface tags).

3. **Injection**: Matched rules are included in the context pack with a
   `playbook_hits` count. The pack is cached and included in the prompt as a
   `## Learning Pack` section.

4. **Feedback loop**: Episode outcomes (gate pass/fail) and iteration memories
   are used to auto-generate `learned-*` and `success-*` playbook rules via
   `build_reflection_playbook_rules()`.

### Roko

Roko's playbook injection is more explicit:

1. **PlaybookStore**: Loaded from `.roko/learn/playbooks/` directory, the store
   provides `match_playbooks(task_description, limit)` for fuzzy matching.

2. **Dispatch-time injection**: The event loop at line ~9815:
   ```rust
   let matched_playbooks = ctx.playbook_store
       .match_playbooks(&task_description, 3)
       .await;
   ```

3. **Prompt append**: `format_when_then_playbooks()` renders matched playbooks
   as a `## Relevant Techniques` section appended to the system prompt.

4. **Feedback recording**: Gate outcomes (pass/fail) are recorded against every
   playbook that was used for the attempt, closing the learning loop:
   ```rust
   if let Some(pb_ids) = task_playbook_ids.remove(&attempt_key) {
       store.record_outcome(pb_id, passed).await;
   }
   ```

5. **SystemPromptBuilder integration**: The builder accepts playbooks via
   `with_playbooks()` and renders them in layer 6 (Relevant techniques).

6. **Daimon hook**: Additionally, Daimon affect guidance is rendered and appended
   to the system prompt via `render_daimon_prompt_context()`.

---

## 11. Cache Alignment

Both systems optimize for LLM prefix-cache hits (Anthropic's 90% token discount
for repeated prefixes):

### Mori
- Sections carry `cache_layer: u8` (1=role, 2=workspace, 3=plan, 0=volatile)
- `assemble_prompt()` sorts by cache_layer before concatenation
- `<!-- mori:layer:N -->` markers inserted at layer transitions
- `SharedPlanContext` ensures byte-identical prefixes across agents in the same plan
- `IMPLEMENTER_PREFIX_CACHE` caches rendered role prefixes in memory

### Roko
- Sections carry `cache_layer: CacheLayer` enum (Role, Workspace, Plan, Volatile)
- `SystemPromptBuilder::with_cache_markers()` inserts `<!-- cache:TIER -->` markers
- `normalize_for_caching()` canonicalizes whitespace so identical content yields
  identical bytes (CRLF normalization, trailing whitespace trimming, tab-to-space)
- `canonical_tool_order()` sorts tool definitions alphabetically so tool payloads
  are deterministic across calls
- Section sorting by `(cache_layer, insertion_order)` preserves stable layers
  before volatile ones

**Key improvement in Roko**: The `normalize_for_caching()` function eliminates
spurious cache misses from whitespace differences. Mori does not normalize,
so subtle whitespace differences between runs could defeat prefix caching.

---

## 12. Prompt Experiments

### Mori

Mori has no formal prompt experiment infrastructure. Learning is implicit through
episode statistics and playbook rules.

### Roko

Roko has a full prompt experiment pipeline:

1. **Assignment**: Runner attempts durably assign experiment variants and replace
   canonical prompt sections with experimental alternatives.

2. **Settlement**: Idempotent settlement from archived/live terminal facts records
   which variant produced better gate outcomes.

3. **Section effectiveness**: `SectionEffectivenessRegistry` tracks per-role
   section-level lift/harm, enabling `build_with_section_effectiveness()` to
   promote positive-lift sections and demote negative-lift ones.

4. **Diagnostics**: Each prompt section can carry an `experiment_id` linking it
   to the experiment that caused its inclusion.

---

## 13. Summary of Differences

| Dimension | Mori | Roko |
|-----------|------|------|
| **Architecture** | Monolithic `prompts.rs` with procedural per-role functions | 9-layer `SystemPromptBuilder` + typed `RolePromptTemplate` trait |
| **I/O coupling** | Prompt builders read filesystem directly | Compose layer is I/O-free; CLI dispatch layer reads files |
| **Budget model** | Fixed per-role character caps | Fixed + adaptive scaling to model context window |
| **Context strategy** | Explicit `McpFirst/Hybrid/InlineHeavy` enum | Auction-based (`LearningBidder` + VCG allocation) |
| **Section prioritization** | Priority 1-5 integer | `SectionPriority` enum (Low/Normal/High/Critical) + bidder competition |
| **Placement** | Implicit via cache_layer ordering | Explicit `Placement` enum (Start/Middle/End) for U-shaped attention |
| **Subsystem bidding** | None -- sections included by fixed priority | 9 `AttentionBidder` variants with Thompson-sampling bids |
| **Cache normalization** | None | `normalize_for_caching()` eliminates whitespace variance |
| **Prompt experiments** | None | Durable assignment, settlement, section-effectiveness tracking |
| **HDC integration** | Index similarity search only | Per-episode fingerprint + similar-episode injection into prompts |
| **Playbook feedback** | Implicit via learned-* rules | Explicit `record_outcome()` per playbook per gate |
| **Affect guidance** | None | Layer 8 via Daimon/PadState somatic markers |
| **Temperament** | None | `Temperament` enum (Conservative/Balanced/Aggressive/Exploratory) |
| **Enrichment steps** | 9 pre-computed steps (brief, tasks, verify, review, etc.) | On-demand via bidder auction + template composition |
| **Observability** | Prompt log files + TUI F7:Context gauge | Telemetry Lens + section audit trail + prompt diagnostics |

---

## 14. What Roko Inherited Directly

The following elements transferred essentially unchanged:

1. **Budget table values** -- identical character caps per role for the core roles
2. **Truncation strategy** -- head-truncate at newline boundaries with markers
3. **Workspace map generation** -- recursive `crates/*/src/` walk with indentation
4. **PRD context extraction** -- plan-referenced PRD sections extracted to markdown
5. **Context layout stanza** -- tells agents where to find plan artifacts
6. **MCP tools stanza** -- steers agents toward MCP over shell commands
7. **Reviewer checklists** -- "What Reviewers Will Check" prompt sections
8. **Self-validation instructions** -- "Run cargo check, cargo test before done"
9. **Gate feedback injection** -- compile errors and review issues in retry prompts
10. **Skill section format** -- `<skill name="X">` XML blocks with per-skill budget

## 15. What Roko Added

1. **Formal 9-layer architecture** with named stability tiers
2. **Trait-based template system** (`RolePromptTemplate`) for testability
3. **Auction-based composition** (`LearningBidder` + VCG) replacing fixed priorities
4. **Adaptive budgets** that scale with model context window
5. **U-shaped attention placement** (Start/Middle/End) for large context windows
6. **Cross-cut functors** (Memory/Daimon/Dreams/Safety) for composition transforms
7. **Prompt experiments** with durable assignment and settlement
8. **Section effectiveness learning** with per-role positive/negative lift tracking
9. **HDC-based similar-episode injection** at dispatch time
10. **Affect-aware prompting** via Daimon somatic markers and temperament dials
11. **Group context** via pheromone signals and membership-scoped knowledge
12. **Cost attribution** per section for budget efficiency analysis
