# Config unification: make roko.toml values actually affect runtime behavior

## Scope

Roko has two config systems that coexist without communicating. Users edit
`roko.toml` expecting behavior changes. Most of those changes silently do
nothing because `PlanRunner` reads from a different struct.

**System A — old `Config`** (`crates/roko-cli/src/config.rs`): what
`PlanRunner` actually uses. Has `BudgetConfig`, `ExecutorConfig`,
`ToolsConfig`, and a `Vec<GateConfig>`. PlanRunner holds `config: Config` as a
field and reads it at every dispatch site.

**System B — new `RokoConfig`** (`crates/roko-core/src/config/schema.rs`, line
44): what users edit. Has 30+ sections. Updated by hot-reload into
`AppState.roko_config`, but `PlanRunner` never reads from `AppState`.

Instead of reading from `RokoConfig`, `PlanRunner` calls `load_roko_config()`
ad-hoc from disk at 16+ scattered call sites — once per dispatch, once for
gates, once for routing weights, etc. This means every config access parses
TOML from disk, and sections that have a parallel old-`Config` field are never
migrated.

**This PRD is a section-by-section migration.** Each section gets its own set
of checklist items. The goal after each section is: old field removed, new
field wired, test proves it.

### Migration strategy

1. Add `roko_config: RokoConfig` as a field on `PlanRunner` (loaded once at
   construction, alongside the old `config: Config` field).
2. For each section, replace all reads of `self.config.<old_field>` with
   `self.roko_config.<new_field>`.
3. Remove the old field from `Config` once all its reads are migrated.
4. Wire hot-reload: after migration, a signal from `AppState` (or a shared
   `ArcSwap`) lets `PlanRunner` pick up changes without restart.
5. Delete dead sections — any `RokoConfig` field that has no corresponding
   runtime reader gets either wired or explicitly documented as deferred.

### What this PRD does NOT cover

- `[attention]`, `[energy]`, `[temporal]`, `[goals]`, `[immune]`,
  `[demurrage]`, `[oneirography]` — these have no runtime consumer anywhere.
  They are deferred to a follow-up PRD after the basic wiring is proven.
- `[agent.roles.*.tools]` per-role tool filtering — this is a separate PRD
  (TOOL-03 followup).
- Chain runtime, `roko-dreams`, `roko-daimon` — Phase 2.

---

## Implementation checklist

### Phase 0: groundwork — add `roko_config` field to PlanRunner

- [ ] **0.1** Add `roko_config: RokoConfig` field to `PlanRunner` struct.

  File: `crates/roko-cli/src/orchestrate.rs`, line 3024.

  Before:
  ```rust
  pub struct PlanRunner {
      workdir: PathBuf,
      config: Config,
      // ...
  ```

  After:
  ```rust
  pub struct PlanRunner {
      workdir: PathBuf,
      config: Config,
      roko_config: RokoConfig,
      // ...
  ```

  Populate in all three constructors (`from_plans_dir`, `from_snapshot`,
  `from_snapshots` (line 4891)). Each already calls `load_roko_config(&workdir)` for
  other purposes — extract that call once at the top of the constructor and
  store the result. Anti-pattern: do not call `load_roko_config` twice.

- [ ] **0.2** Add a `fn roko_config(&self) -> &RokoConfig` accessor method on
  `PlanRunner`. This lets future callers use the cached copy without touching
  the field directly. No logic in the accessor — just `&self.roko_config`.

- [ ] **0.3** Eliminate all ad-hoc `load_roko_config(&self.workdir)` call sites
  that read fields now covered by `self.roko_config`. Replace with
  `&self.roko_config`. Keep only call sites that genuinely need a fresh read
  (none do after this migration).

  Current ad-hoc call sites (as of 2026-04-22):
  - Line 859: definition of `load_roko_config` — keep, used in constructors
  - Line 1569: `run_prepared_agent` — this is a free function, not a method;
    keep its local load
  - Line 3315: `build_routing_context` — replace with `&runner.roko_config`
  - Line 9899: `provider_id_for_model` — replace with `&self.roko_config`
  - Line 10272: reward weights in cascade observation — replace with
    `&self.roko_config`
  - Line 13094: `config_default_domain` — replace with `&self.roko_config`
  - Line 13156: `roko_config` local in `dispatch_agent_with` — replace with
    `&self.roko_config`
  - Line 14101: `roko_config` local in Claude path — replace with
    `&self.roko_config`
  - Line 15353: `runtime_gate_config` — replace with `&self.roko_config`
  - Line 15386: `current_task_domain` — replace with `&self.roko_config`
  - Line 15481: `domain_gate_steps` — replace with `&self.roko_config`
  - Lines 15598, 4626, 4784, 4944: constructor-time loads — consolidate into
    Phase 0.1 constructor initialization

  Verification: `grep -n 'load_roko_config(&self' crates/roko-cli/src/orchestrate.rs`
  must return zero matches.

---

### Phase 1: `[budget]` section

Old field: `self.config.budget` (`crates/roko-cli/src/config.rs`, line 354)
New field: `self.roko_config.budget` (`crates/roko-core/src/config/schema.rs`, line 2576)

**Schema mismatch to resolve first:**

| Old `BudgetConfig` field | New `BudgetConfig` field | Resolution |
|---|---|---|
| `max_plan_usd: f64` | `max_plan_usd: f32` | Cast `f32` to `f64` at read sites |
| `max_task_usd: f64` | `max_turn_usd: f32` | Rename: `max_turn_usd` maps to per-task cap |
| `max_session_usd: f64` | (missing) | Add `max_session_usd: f32` to new `BudgetConfig` |
| `warn_at_percent: u32` | (missing) | Add `warn_at_percent: u32` to new `BudgetConfig` |
| `fn warn_threshold_usd()` | (missing) | Move method to new `BudgetConfig` |

- [ ] **1.1** Add `max_session_usd: f32` and `warn_at_percent: u32` fields to
  `roko_core::config::schema::BudgetConfig`. Add serde defaults matching old
  defaults (50.0 and 80). Add `fn warn_threshold_usd(&self) -> f64` that
  returns `f64::from(self.max_plan_usd) * f64::from(self.warn_at_percent) / 100.0`.

  File: `crates/roko-core/src/config/schema.rs`, around line 2576.

- [ ] **1.2** Rename `max_turn_usd` to `max_task_usd` in the new `BudgetConfig`
  (or keep both and add `max_task_usd` as an alias). The old `roko.toml`
  documentation used `max_turn_usd`; the old `Config` called it
  `max_task_usd`. Resolve by keeping `max_task_usd` as the canonical name and
  adding a `#[serde(alias = "max_turn_usd")]` so existing configs continue to
  parse.

  File: `crates/roko-core/src/config/schema.rs`, line 2582.

- [ ] **1.3** Replace all `self.config.budget.*` reads in `orchestrate.rs` with
  `self.roko_config.budget.*`. Cast `f32` → `f64` as needed. Every occurrence
  must be updated; leaving one behind creates a silent split.

  Sites (as of 2026-04-22) — **25 total call sites**, not ~15 as previously
  estimated:
  - Line 10259–10260: `self.config.budget.max_task_usd` (x2) →
    `f64::from(self.roko_config.budget.max_task_usd)`
  - Line 10413: `self.config.budget.max_plan_usd` →
    `f64::from(self.roko_config.budget.max_plan_usd)`
  - Line 10416: `self.config.budget.*` → migrate
  - Line 12933: `&self.config.budget` passed to `warn_threshold_usd()` →
    update to call `self.roko_config.budget.warn_threshold_usd()`
  - Line 12963, 13391–13394, 14149–14152, 14360–14363: all four budget fields
    → map to new struct fields
  - Line 13070: `self.config.budget.max_plan_usd` → migrate
  - Line 13073: `self.config.budget.*` → migrate
  - Line 13079: `routing_budget_pressure(&self.config.budget, ...)` → update
    the function signature or add an adapter
  - Line 13400: `self.config.budget.*` → migrate
  - Line 14465: `self.config.budget.max_plan_usd` → migrate
  - Line 14468: `self.config.budget.*` → migrate
  - Line 14657: `self.config.budget.*` → migrate
  - Line 14660: `self.config.budget.*` → migrate
  - Line 14668: `self.config.budget.*` → migrate

  The original estimate of ~15 sites missed lines 10416, 13073, 13400, 14468,
  14657, 14660, and 14668. All 25 must be migrated.

- [ ] **1.4** Update `routing_budget_pressure` (line 2452) to accept
  `&roko_core::config::schema::BudgetConfig` instead of
  `&crate::config::BudgetConfig`. Add a temporary adapter if needed, then
  remove the old version once no callers remain.

- [ ] **1.5** Remove `BudgetConfig` from `crates/roko-cli/src/config.rs`
  (lines 354–403) and the `config.budget` field from `Config` (line 51). The
  compiler will catch any remaining reads.

- [ ] **1.6** Write an integration test:

  ```rust
  #[tokio::test]
  async fn budget_cap_respected_from_roko_toml() {
      let dir = test_workdir();
      // Write a roko.toml with a very small budget.
      std::fs::write(dir.join("roko.toml"), r#"
          [budget]
          max_plan_usd = 0.001
          max_task_usd = 0.001
      "#).unwrap();
      let runner = PlanRunner::from_plans_dir(
          &dir.join("plans"), &dir, Config::default(), Arc::new(MetricRegistry::new()), false,
      ).await.unwrap();
      // Assert runner read the new budget.
      assert!((f64::from(runner.roko_config.budget.max_plan_usd) - 0.001).abs() < 1e-6);
  }
  ```

  File: `crates/roko-cli/src/orchestrate.rs`, test module at bottom of file.

---

### Phase 2: `[conductor]` section → ExecutorConfig

Old fields: `self.config.executor` (`ExecutorConfig` from
`crates/roko-orchestrator/src/executor/mod.rs`, line 147).
New fields: `self.roko_config.conductor` (`ConductorConfig` from
`crates/roko-core/src/config/schema.rs`, line 2615).

**Mapping:**

| Old `ExecutorConfig` field | New `ConductorConfig` field | Note |
|---|---|---|
| `max_concurrent_tasks: usize` | `max_agents: usize` | Same concept |
| `max_concurrent_plans: usize` | `max_parallel_plans: usize` | Same concept |
| `max_auto_fix_iterations: u32` | `max_auto_fix_attempts: u32` | Same concept |
| `auto_replan: bool` | (no equivalent — lives in `LearningConfig`) | Read from `roko_config.learning.replan_on_gate_failure` |
| `use_worktrees: bool` | (no equivalent in ConductorConfig) | Add `use_worktrees: bool` to `ConductorConfig` |
| `task_timeout_secs: u64` | (missing) | Add `task_timeout_secs: u64` to `ConductorConfig` |

- [ ] **2.1** Add `use_worktrees: bool` and `task_timeout_secs: u64` to
  `ConductorConfig` in `crates/roko-core/src/config/schema.rs`. Serde
  defaults: `false` and `600`.

- [ ] **2.2** In the three `PlanRunner` constructors, build `ExecutorConfig`
  from `RokoConfig` instead of from `Config.executor`. Extract a helper:

  ```rust
  fn executor_config_from_roko(rc: &RokoConfig) -> ExecutorConfig {
      ExecutorConfig {
          max_concurrent_tasks: rc.conductor.max_agents,
          max_concurrent_plans: rc.conductor.max_parallel_plans,
          max_auto_fix_iterations: rc.conductor.max_auto_fix_attempts,
          auto_replan: rc.learning.replan_on_gate_failure,
          use_worktrees: rc.conductor.use_worktrees,
          task_timeout_secs: rc.conductor.task_timeout_secs,
          // The following 4 fields exist on ExecutorConfig but have NO
          // equivalent in ConductorConfig yet. They must be handled:
          max_merge_attempts: rc.conductor.max_merge_attempts.unwrap_or(3),
          budget_usd: rc.budget.max_plan_usd.into(),
          resource_budget: rc.conductor.resource_budget.unwrap_or_default(),
          speculative_threshold_multiplier: rc.conductor.speculative_threshold_multiplier.unwrap_or(1.5),
      }
  }
  ```

  **Note:** `ExecutorConfig` has 4 fields not mapped above that the
  `..ExecutorConfig::default()` spread would silently default. These are:
  - `max_merge_attempts` — add to `ConductorConfig` with default `3`
  - `budget_usd` — bridge from `rc.budget.max_plan_usd` (cross-section read)
  - `resource_budget` — add to `ConductorConfig` with default `ResourceBudget::default()`
  - `speculative_threshold_multiplier` — add to `ConductorConfig` with default `1.5`

  All four must have explicit mappings. Using `..ExecutorConfig::default()` silently
  discards configured values.

  File: `crates/roko-cli/src/orchestrate.rs`, near the constructors.

  Before (line 4484):
  ```rust
  let mut executor = ParallelExecutor::new(config.executor.clone());
  ```

  After:
  ```rust
  let mut executor = ParallelExecutor::new(executor_config_from_roko(&roko_config));
  ```

  Apply the same change in `from_snapshot` (line 4750) and
  `from_snapshots` (line 4891).

  **IMPORTANT:** Three additional sites mutate `config.executor.*` directly and
  must also be migrated:
  - Line 18744: mutates `config.executor.max_concurrent_tasks`
  - Line 19010: mutates `config.executor.auto_replan`
  - Line 19515: mutates `config.executor.use_worktrees`
  These are runtime overrides that must write to `roko_config.conductor.*` instead.

- [ ] **2.3** Remove `executor: ExecutorConfig` from `Config`
  (`crates/roko-cli/src/config.rs`, line 48) once all constructor sites are
  migrated.

- [ ] **2.4** Write integration test verifying `conductor.max_agents` in
  `roko.toml` controls actual parallelism in the executor. Assert
  `runner.executor.config().max_concurrent_tasks == configured_value`.

---

### Phase 3: `[gates]` section

Old fields: `self.config.gates` (`Vec<GateConfig>`) — used for gate commands.
New fields: `self.roko_config.gates` (`GatesConfig` from schema line 2125).

The old `Config.gates` is a `Vec<GateConfig>` (shell commands). The new
`GatesConfig` is a struct with `clippy_enabled`, `skip_tests`,
`max_iterations`, and `domain_gates`. The `runtime_gate_config()` method
(line 15352) already reads from disk into `GatesConfig` — the fix is to make
it read from `self.roko_config` instead.

- [ ] **3.1** Change `runtime_gate_config` to return `&GatesConfig`:

  Before (line 15352–15359):
  ```rust
  fn runtime_gate_config(&self) -> GatesConfig {
      load_roko_config(&self.workdir)
          .map(|config| config.gates)
          .unwrap_or_else(|err| { ... GatesConfig::default() })
  }
  ```

  After:
  ```rust
  fn runtime_gate_config(&self) -> &GatesConfig {
      &self.roko_config.gates
  }
  ```

  Update all callers (`gate_rung_caps`, `selected_gate_steps`, line 15404,
  15418) to use the reference.

- [ ] **3.2** Wire `gates.max_iterations` into the gate pipeline retry count.
  The gate pipeline currently uses a hardcoded limit of `1` in the
  `max_iterations` field of the dispatch call at lines 14168 and 14379.
  Change these to read `self.roko_config.gates.max_iterations`.

  Before (line 14168):
  ```rust
  max_iterations: 1,
  ```

  After:
  ```rust
  max_iterations: self.roko_config.gates.max_iterations as usize,
  ```

- [ ] **3.3** Remove `gates: Vec<GateConfig>` from `Config`
  (`crates/roko-cli/src/config.rs`, line 44). Remove `GateConfig` struct if
  nothing else uses it. The compiler will show remaining consumers.

- [ ] **3.4** Write integration test:

  ```rust
  #[test]
  fn gate_skip_tests_read_from_roko_toml() {
      let dir = test_workdir();
      std::fs::write(dir.join("roko.toml"), "[gates]\nskip_tests = true\n").unwrap();
      let rc = load_roko_config(&dir).unwrap();
      assert!(rc.gates.skip_tests);
  }
  ```

---

### Phase 4: `[routing]` section

Old: routing parameters hardcoded in `CascadeRouter` init (LinUCB algorithm,
no model map from config). New: `self.roko_config.routing` (`RoutingConfig`
from schema line 2504).

The routing config IS read by `load_roko_config` at lines 1569 (free
function), 3315 (`build_routing_context`), and 10272 (reward weights). These
are the only three sites that actually consume `routing.*` fields — but they
all re-parse TOML from disk. After Phase 0, they use `self.roko_config`
directly. The remaining gap is model arm initialization in `CascadeRouter`.

- [ ] **4.1** Pass `routing.fast_task_model`, `routing.standard_task_model`,
  `routing.complex_task_model` as the initial arm models when constructing the
  `CascadeRouter` in `PlanRunner`. Currently the router uses whatever arms are
  registered from the `[models]` section. Make the three routing tier models
  explicit inputs.

  Locate `CascadeRouter` initialization in `orchestrate.rs` (search:
  `CascadeRouter::`) and pass the three model strings from
  `self.roko_config.routing.*`.

- [ ] **4.2** Pass `routing.algorithm` to the `CascadeRouter` so the algorithm
  is configurable. The router currently hardcodes LinUCB. Pass the enum value
  from config at construction time.

- [ ] **4.3** Pass `routing.discount_factor` to the `CascadeRouter` at
  construction time. This controls Thompson sampling in non-stationary
  environments.

- [ ] **4.4** Write integration test: set `routing.algorithm = "thompson"` in
  `roko.toml`, build the router, assert the algorithm label matches.

---

### Phase 5: `[tools]` section

Old: `self.config.tools` (`crates/roko-cli/src/config.rs`, line 204) — has
`prefer_mcp: bool`, `global_denied: Vec<String>`, `mcp_timeout_secs: u64`.
New: `self.roko_config.tools` (`ToolsConfig` from schema line 1288) — has
`allow: Vec<String>`, `deny: Vec<String>`, `profiles: HashMap<String,
ToolProfileConfig>`.

**Mapping:**

| Old field | New field |
|---|---|
| `prefer_mcp` | No direct equivalent — add `prefer_mcp: bool` to new `ToolsConfig` |
| `global_denied` | `deny` (rename) |
| `mcp_timeout_secs` | Add `mcp_timeout_secs: u64` to new `ToolsConfig` |

- [ ] **5.1** Add `prefer_mcp: bool` (default `false`) and `mcp_timeout_secs:
  u64` (default `30`) to `roko_core::config::schema::ToolsConfig`.

  File: `crates/roko-core/src/config/schema.rs`, around line 1288.

- [ ] **5.2** Replace `config.tools.prefer_mcp` at line 4378 with
  `self.roko_config.tools.prefer_mcp`.

  Before:
  ```rust
  roko_agent::mcp::DynamicToolRegistry::with_preference(&base, config.tools.prefer_mcp);
  ```

  After:
  ```rust
  roko_agent::mcp::DynamicToolRegistry::with_preference(&base, self.roko_config.tools.prefer_mcp);
  ```

  Note: `setup_mcp` is called from 4 sites (lines 4615, 4773, 4933, 12999),
  not just one internal read. It is a static method taking `config: &Config`.
  After Phase 5, all 4 call sites must pass the relevant bool directly rather
  than passing the whole old `Config`.

- [ ] **5.3** Wire `roko_config.tools.deny` as the global tool denylist at
  dispatch time. The existing `SafetyLayer::from_config(&roko_config)` call at
  line 4632 already reads the `RokoConfig` for whitelist setup — verify that
  the deny list is forwarded through `SafetyLayer` and that
  `global_denied` from the old config is superseded.

  **SEQUENCING WARNING (Phase 5 + Phase 6 conflict):** `setup_mcp` reads BOTH
  `config.tools.prefer_mcp` (Phase 5 field) AND `config.agent.mcp_config`
  (Phase 6 field). These two phases cannot be done independently — migrating
  Phase 5 alone will leave `setup_mcp` with a split read (one field from
  `roko_config`, one from old `config`). Either: (a) migrate both fields in
  `setup_mcp` atomically as part of Phase 5, or (b) introduce a temporary
  adapter that reads `prefer_mcp` from `roko_config` and `mcp_config` from
  old `config` until Phase 6 completes. Option (a) is strongly preferred.

- [ ] **5.4** Remove `tools: ToolsConfig` from old `Config`
  (`crates/roko-cli/src/config.rs`, line 36). Compiler will confirm no
  remaining reads.

- [ ] **5.5** Write integration test: set `deny = ["bash"]` in `[tools]`,
  assert the tool is blocked at dispatch.

---

### Phase 6: `[agent]` section — model and effort fields

Old: `self.config.agent.*` fields (`AgentConfig` from
`crates/roko-cli/src/config.rs`, line 160). New: `self.roko_config.agent.*`
(`AgentConfig` in `crates/roko-core/src/config/schema.rs` — the new one, not
the old one with the same name).

This is the largest migration because `self.config.agent` is read at 40+
sites. Do it field by field.

**Mapping:**

| Old field (`crates/roko-cli`) | New field (`roko-core`) |
|---|---|
| `command: String` | `agent.command` (via provider registry) |
| `args: Vec<String>` | Provider-level `args` |
| `model: Option<String>` | `agent.default_model` |
| `effort: String` | `agent.default_effort` |
| `bare_mode: bool` | `agent.bare_mode` |
| `fallback_model: Option<String>` | `agent.fallback_model` |
| `timeout_ms: u64` | Provider-level `timeout_ms` |
| `env: Vec<(String, String)>` | Provider-level `env` or `agent.env` |
| `mcp_config: Option<PathBuf>` | `agent.mcp_config` (add this field) |
| `tier_models: HashMap<String, String>` | `agent.tier_models` |
| `escalation.max_retries: u32` | `conductor.max_auto_fix_attempts` |
| `escalation.escalate_model: bool` | No direct equivalent — add to `ConductorConfig` |

- [ ] **6.1** Add `mcp_config: Option<PathBuf>` to the new
  `roko_core::config::schema::AgentConfig`. This field currently only exists
  in the old `AgentConfig`.

  File: `crates/roko-core/src/config/schema.rs`.

- [ ] **6.2** Replace `self.config.agent.model` at all read sites (lines
  13116–13120, 13127–13133, 13148–13153) with
  `self.roko_config.agent.default_model`.

  Note the type difference: old is `Option<String>`, new is `String`.
  Wrap new value in `Some(...)` at call sites that expect `Option<String>`, or
  add a helper `fn effective_default_model(&self) -> &str`.

- [ ] **6.3** Replace `self.config.agent.effort` (lines 3342, 13499) with
  `self.roko_config.agent.default_effort`.

- [ ] **6.4** Replace `self.config.agent.tier_models` (lines 9335, 9731, 10712,
  12162, 13132, 17266) with `&self.roko_config.agent.tier_models`.

- [ ] **6.5** Replace `self.config.agent.mcp_config` (lines 4292, 4414, 4425,
  4434, 4443, 4451) with `self.roko_config.agent.mcp_config`.

- [ ] **6.6** Replace agent spawn fields at lines 7442–7449, 8411–8419,
  9384–9400, 14111–14138 to read from `self.roko_config` instead of
  `self.config.agent`. Specifically:
  - `command` → derive from provider registry in `self.roko_config`
  - `args` → from provider `args`
  - `bare_mode` → `self.roko_config.agent.bare_mode`
  - `fallback_model` → `self.roko_config.agent.fallback_model`
  - `timeout_ms` → from provider `timeout_ms`
  - `env` → from provider `env` or `agent.env`

  **Additional `agent.*` call sites not listed above** that must also be
  migrated:
  - Lines 5476–5477: `self.config.agent.*` reads in dispatch path
  - Line 8399: `self.config.agent.model` read
  - Line 13909: `self.config.agent.*` read in Claude dispatch
  - Line 14088: `self.config.agent.*` read
  - Line 14193: `self.config.agent.*` read
  - Lines 14300–14353: entire `ExecAgent` dispatch block reads multiple
    `self.config.agent.*` fields (`command`, `args`, `model`, `effort`,
    `env`, `bare_mode`, `timeout_ms`)

- [ ] **6.7** Replace `self.config.agent.escalation.max_retries` (line 8901)
  with `self.roko_config.conductor.max_auto_fix_attempts`.

- [ ] **6.8** Remove `agent: AgentConfig` from old `Config`. The compiler will
  confirm zero remaining reads. Remove `AgentConfig` from
  `crates/roko-cli/src/config.rs` if nothing else imports it. Watch for the
  TUI config view — it may render fields from the old struct.

- [ ] **6.9** Write integration test: set `agent.default_model = "claude-haiku-4-5"`
  in `roko.toml`, construct a `PlanRunner`, assert
  `runner.roko_config.agent.default_model == "claude-haiku-4-5"` and that the
  model is used at dispatch.

---

### Phase 7: `[server]` section — bind and port

Old: hardcoded `0.0.0.0:6677` in `roko-serve` (bypassed by `PORT` env var).
New: `self.roko_config.server.bind` and `self.roko_config.server.port`.

The `ServerBuildConfig::effective_addr` method (line 126–132 of
`crates/roko-serve/src/lib.rs`) already reads `roko_config.server.bind` and
`roko_config.server.port`. This is already wired. Verify the path is
exercised.

- [ ] **7.1** Audit the serve startup in `crates/roko-cli/src/main.rs`. Confirm
  that `ServerBuildConfig` is constructed with the loaded `RokoConfig`, not
  with a default. If the CLI passes `None` for bind/port (triggering the
  effective_addr fallback to `server.bind/port`), that is correct. If it
  passes a hardcoded string, replace with `None`.

- [ ] **7.2** Add a test in `crates/roko-serve/src/lib.rs` that constructs
  `ServerBuildConfig` with a `RokoConfig` that has `server.port = 7000` and
  asserts `effective_addr()` returns `"0.0.0.0:7000"`.

---

### Phase 8: `[tui]` section — refresh rate

Old: `Duration::from_millis(16)` hardcoded at line 574 of
`crates/roko-cli/src/tui/app.rs` and `Duration::from_millis(250)` at line 396.
New: `self.roko_config.tui.refresh_rate_ms`.

- [ ] **8.1** Pass `RokoConfig` (or just `tui.refresh_rate_ms: u64`) into the
  TUI `App` struct at startup. The dashboard startup call is in
  `crates/roko-cli/src/main.rs` (`roko dashboard` subcommand). Load the
  config there and thread the `tui.refresh_rate_ms` value through.

- [ ] **8.2** Replace `Duration::from_millis(16)` at line 574 with
  `Duration::from_millis(config.tui.refresh_rate_ms)`. **WARNING:** The
  `TuiConfig.refresh_rate_ms` field in the schema defaults to `250`, NOT `16`.
  If both the 16ms frame interval and the 250ms event poll are wired to the
  same `refresh_rate_ms` field, the TUI degrades from 60fps to 4fps. The
  default for `refresh_rate_ms` must be set to `16` (matching the current
  frame interval), OR two separate fields must be used.

- [ ] **8.3** A separate `tui.event_poll_ms` field is **REQUIRED**, not
  optional. Replace `Duration::from_millis(250)` at line 396 (the event poll
  timeout) with `Duration::from_millis(config.tui.event_poll_ms)`. Add
  `event_poll_ms: u64` to `TuiConfig` with default `250`. Do NOT compute it
  as `refresh_rate_ms * 15` — the two durations serve different purposes
  (frame rendering vs input polling) and must be independently configurable.
  Wiring both to a single field with a schema default of 250 would cause the
  frame rate to drop from 60fps to 4fps.

- [ ] **8.4** Write a test: construct `App` with `refresh_rate_ms = 100`,
  assert the `EventHandler` tick interval is 100ms.

---

### Phase 9: hot-reload wiring

The config watcher (`crates/roko-serve/src/config_watcher.rs`) already polls
`roko.toml` every 2 seconds and calls `apply_hot_reload`. **Note:**
`apply_hot_reload` does NOT update `AppState` directly — it mutates a local
`&mut RokoConfig`. The `ArcSwap` store happens in the caller, not inside
`apply_hot_reload`. This is important for Phase 9 wiring: the reload path is
`watcher detects change → loads new config → calls apply_hot_reload(&mut new_config) →
caller stores new_config into ArcSwap`. `PlanRunner` holds its own copy and
never receives updates.

- [ ] **9.1** Change `PlanRunner.roko_config` from `RokoConfig` to
  `Arc<ArcSwap<RokoConfig>>`. This lets a background watcher push updates
  into the same slot that `PlanRunner` reads from.

  Alternative: pass a `tokio::sync::watch::Receiver<Arc<RokoConfig>>` and
  snapshot it at the start of each dispatch. Choose the approach that has
  fewer lock acquisitions per dispatch. The `ArcSwap` approach (a single
  atomic load) is preferred.

- [ ] **9.2** When constructing `PlanRunner` from within `roko serve` (the
  `auto_orchestrate` path), share the same `ArcSwap` that `AppState` holds.
  When constructing `PlanRunner` standalone (the `roko plan run` path),
  construct a new `ArcSwap` backed by the initial load. The two paths are
  already separate constructor call sites.

- [ ] **9.3** Update the `roko_config()` accessor to do:
  ```rust
  fn roko_config(&self) -> Arc<RokoConfig> {
      self.roko_config.load_full()
  }
  ```
  All call sites that currently call `&self.roko_config` switch to
  `self.roko_config()` (returning `Arc<RokoConfig>`). Use `.as_ref()` at the
  call site to avoid cloning the whole struct.

- [ ] **9.4** Test hot-reload: write a `roko.toml` with `max_plan_usd = 5.0`,
  run a `PlanRunner`, then write `max_plan_usd = 10.0` to disk, trigger a
  reload signal, assert `runner.roko_config().budget.max_plan_usd == 10.0`.

---

### Phase 10: delete the old `Config` struct

**WARNING:** Phases 1-9 do NOT cover all `Config` fields. The following live
fields on `Config` are NOT addressed by any phase in this PRD and must be
migrated before `Config` can be deleted:

- `dreams` — dream runner config (runtime consumer exists)
- `daimon` — daimon state config (runtime consumer exists)
- `prompt` — prompt composition config
- `auto_plan` — automatic plan generation trigger
- `repos` — multi-repo config
- `providers` — LLM provider registry
- `models` — model name aliases
- `serve` — serve config (distinct from `server`)
- `log_format` — structured logging format
- `bind` — bind address (overlaps with `server.bind`)
- `data_dir` — data directory path

Phase 10 requires ALL of these to be migrated first. Either add phases 10a-10k
to this PRD or defer Phase 10 to a follow-up. Do not attempt to delete `Config`
until every field is accounted for.

After all fields are migrated:

- [ ] **10.1** Remove `pub struct Config` and all remaining fields/impls from
  `crates/roko-cli/src/config.rs`. Keep only non-migrated utility types that
  have no equivalent (e.g., `PromptFile`, `ContextBudgetConfig`) until a
  follow-up PRD addresses them.

- [ ] **10.2** Remove the `Config` parameter from all `PlanRunner` constructor
  signatures. Replace with `RokoConfig`. Update all call sites in
  `main.rs`.

- [ ] **10.3** Run `cargo build --workspace` and fix all remaining type errors.
  Do not suppress with `#[allow()]` — each error is a missed migration.

- [ ] **10.4** Run `cargo clippy --workspace --no-deps -- -D warnings` and fix
  all warnings.

---

## Concrete file touchpoints

| File | Changes |
|---|---|
| `crates/roko-cli/src/orchestrate.rs` | Add `roko_config` field (Phase 0); replace all `self.config.*` reads across Phases 1–8 |
| `crates/roko-cli/src/config.rs` | Remove migrated structs field by field; delete entirely in Phase 10 |
| `crates/roko-core/src/config/schema.rs` | Add missing fields: `max_session_usd`, `warn_at_percent`, `use_worktrees`, `task_timeout_secs`, `prefer_mcp`, `mcp_timeout_secs`, `mcp_config` |
| `crates/roko-orchestrator/src/executor/mod.rs` | `ExecutorConfig` construction now driven by `ConductorConfig` values (Phase 2) |
| `crates/roko-cli/src/tui/app.rs` | Accept `tui.refresh_rate_ms` at construction; replace hardcoded durations (Phase 8) |
| `crates/roko-serve/src/lib.rs` | Already reads `server.bind/port` — verify path is exercised (Phase 7) |
| `crates/roko-serve/src/state.rs` | Expose `ArcSwap<RokoConfig>` for sharing with `PlanRunner` (Phase 9) |
| `crates/roko-cli/src/main.rs` | Update `roko serve` and `roko plan run` startup to pass `RokoConfig` instead of `Config` |

---

## Verification checklist

For each phase, all three checks must pass before moving to the next phase.

- [ ] `cargo build --workspace` — zero errors
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` — zero warnings
- [ ] `cargo test --workspace` — all tests pass, including new integration tests

Additionally:

- [ ] `grep -n 'self\.config\.budget' crates/roko-cli/src/orchestrate.rs` returns
  zero matches after Phase 1
- [ ] `grep -n 'self\.config\.executor' crates/roko-cli/src/orchestrate.rs` returns
  zero matches after Phase 2
- [ ] `grep -n 'load_roko_config(&self' crates/roko-cli/src/orchestrate.rs` returns
  zero matches after Phase 0
- [ ] `grep -rn 'BudgetConfig' crates/roko-cli/src/config.rs` returns zero matches
  after Phase 1
- [ ] `grep -rn 'pub struct Config' crates/roko-cli/src/config.rs` returns zero
  matches after Phase 10

---

## Acceptance criteria

### AC-1: Budget enforcement from roko.toml

Write a `roko.toml`:
```toml
[budget]
max_plan_usd = 0.001
max_task_usd = 0.001
```

Run `PlanRunner::from_plans_dir`. The runner must reject dispatch before
spending $0.001, and the rejection error message must reference the value from
`roko_config.budget`, not a hardcoded fallback.

Verify: `ensure_task_budget_available` returns `Err` immediately when called
after recording `$0.002` of task spend.

### AC-2: Conductor concurrency from roko.toml

Write a `roko.toml`:
```toml
[conductor]
max_agents = 2
```

Run `PlanRunner::from_plans_dir`. Assert
`runner.executor.config().max_concurrent_tasks == 2`. Run a plan with 8
tasks and confirm no more than 2 run simultaneously.

### AC-3: Gate skip-tests from roko.toml

Write a `roko.toml`:
```toml
[gates]
skip_tests = true
```

Run a plan against a Rust project with a failing test. The gate pipeline must
not run `cargo test`. The task must pass. Verify via test coverage that the
`Rung::Test` rung is absent from the selected steps.

### AC-4: Tool deny list from roko.toml

Write a `roko.toml`:
```toml
[tools]
deny = ["bash"]
```

Attempt to dispatch an agent with `bash` in its allowed tools. The safety
layer must strip or block `bash`. Verify the tool list passed to the agent
subprocess does not include it.

### AC-5: Hot-reload budget change

Start a `PlanRunner`. Modify `roko.toml` on disk to double `max_plan_usd`.
Trigger a reload (or wait for the watcher interval). Assert
`runner.roko_config().budget.max_plan_usd` reflects the new value without
restarting the runner.

### AC-6: Server port from roko.toml

Write a `roko.toml`:
```toml
[server]
port = 7999
```

Start `roko serve`. Assert the server binds to port 7999. Verify with a
connection attempt (or read the log output).

### AC-7: No ad-hoc disk reads during dispatch

Instrument `load_roko_config` with a call counter. Run a full plan (3+ tasks
with gate runs). Assert the counter does not increment during the plan run
after initial construction. All reads must use the cached `roko_config` field.

### AC-8: Regression — existing behavior preserved

Run the existing integration test suite (`cargo test --workspace`) before and
after each phase. No test that passed before a phase may fail after it. All
previously-passing behavior (budget warnings, gate selection, model routing,
MCP preference) must work identically with values drawn from the new path.

---

## Anti-patterns

**Do not add a second read path alongside the old one.** Every migration must
remove the old read before merging. Two paths that return different values for
the same field are worse than one wrong path.

**Do not pass `&Config` and `&RokoConfig` to the same function.** If a helper
function needs both, it has not been migrated. Refactor the function to accept
`&RokoConfig` only.

**Do not suppress `#[allow(dead_code)]` on old Config fields.** Dead fields
mean the migration is incomplete. Let the compiler show you what remains.

**Do not call `load_roko_config` inside a hot path.** After Phase 0, the only
legitimate call sites are constructors and the hot-reload signal handler.
Every other call is a bug.

**Do not rename new fields to match old fields arbitrarily.** When old and new
names differ (e.g., `max_task_usd` vs `max_turn_usd`), pick the clearer name
and add `#[serde(alias = "...")]` for backward compatibility. Document the
choice in a comment.

**Do not delete `Config` before all phases are complete.** Delete it in Phase
10, not earlier. Premature deletion causes a cascade of compile errors that is
harder to reason about than a field-by-field migration.

---

## Errata applied

Corrections applied 2026-04-22 based on audit discrepancy report:

1. **Phase 1.3: Budget call sites expanded from ~15 to 25.** Added 7 missing
   sites at lines 10416, 13073, 13400, 14468, 14657, 14660, 14668.

2. **Phase 2: Third constructor name corrected.** Changed `from_single_plan` to
   `from_snapshots` (line 4891) to match the actual codebase.

3. **Phase 2: 4 unmapped `ExecutorConfig` fields documented.** Added explicit
   handling for `max_merge_attempts`, `budget_usd`, `resource_budget`, and
   `speculative_threshold_multiplier` which were previously hidden behind
   `..ExecutorConfig::default()`.

4. **Phase 2: 3 mutation sites added.** Lines 18744, 19010, 19515 mutate
   `config.executor.*` directly and must be migrated to write to
   `roko_config.conductor.*`.

5. **Phase 5: `setup_mcp` call site count corrected.** Now documents all 4 call
   sites at lines 4615, 4773, 4933, 12999.

6. **Phase 5+6 sequencing conflict documented.** `setup_mcp` reads both
   `config.tools.prefer_mcp` (Phase 5) and `config.agent.mcp_config` (Phase 6).
   Must migrate atomically or use temporary adapter.

7. **Phase 6: Missing `agent.*` call sites added.** Lines 5476-5477, 8399,
   13909, 14088, 14193, and the entire ExecAgent dispatch block at 14300-14353.

8. **Phase 8: TUI default mismatch corrected.** `TuiConfig.refresh_rate_ms`
   defaults to 250 in schema, not 16. Wiring both durations to this field
   degrades framerate from 60fps to 4fps. `tui.event_poll_ms` is now documented
   as REQUIRED (separate field, default 250ms).

9. **Phase 9: `apply_hot_reload` behavior clarified.** Does NOT update AppState
   directly; it mutates a local `&mut RokoConfig`. The ArcSwap store happens in
   the caller.

10. **Phase 10: Live Config fields documented.** 11 fields (`dreams`, `daimon`,
    `prompt`, `auto_plan`, `repos`, `providers`, `models`, `serve`, `log_format`,
    `bind`, `data_dir`) are live and not addressed by any phase. Phase 10 cannot
    proceed until all are migrated.
