# IDE Integration — Task Checklist

Status: `[ ]` not started, `[~]` in progress, `[x]` done

Each item references a batch file in `batches/` with full implementation details.
Items within the same "Parallel Group" can be assigned to separate agents simultaneously.

## Branch Audit Update - 2026-05-05

On `wp-arch2`, the core IDE/ACP code changes are mostly wired. This checklist remains useful
as the original work breakdown, but unchecked boxes below should not be read as proof that the
code is absent. Re-run the IDE harness before changing items to `[x]`.

Known implemented areas:
- IndexMap migration
- `session/new` model/provider/effort params
- MCP status types, notifications, and `discoveryTimeoutMs`
- command categories / bare-mode filtering
- effective `max_output` surfacing
- provider `ready`
- config option validation
- `--global-config`, `configSources`, request-driven ConfigWatcher reload

Known remaining gaps:
- `configSources` order/shape and missing implicit-global watch
- proactive config reload notifications
- stale `SessionManager.config_sources` after reload
- clearer missing-workdir warning for Zed users

---

## Phase 1: Code Changes (defer ALL compilation to Phase 2)

### Parallel Group 0 — Foundation (1 agent, ~10 min)

- [ ] **0.1** Add `indexmap = { version = "2", features = ["serde"] }` to workspace `Cargo.toml` `[workspace.dependencies]`
- [ ] **0.2** Add `indexmap = { workspace = true }` to `crates/roko-core/Cargo.toml`
- [ ] **0.3** Add `indexmap = { workspace = true }` to `crates/roko-agent/Cargo.toml`
- [ ] **0.4** Add `indexmap = { workspace = true }` to `crates/roko-cli/Cargo.toml`
- [ ] **0.5** Add `indexmap = { workspace = true }` to `crates/roko-learn/Cargo.toml`
- [ ] **0.6** Add `indexmap = { workspace = true }` to `crates/roko-serve/Cargo.toml`

Batch: `W0-A-indexmap-migration.md` → W0-A-1

---

### Parallel Group 1 — Core Type Change (1 agent, ~15 min, after Group 0)

- [ ] **1.1** schema.rs: add `use indexmap::IndexMap;` import (line 10)
- [ ] **1.2** schema.rs: `providers` field `HashMap` → `IndexMap` (line 71)
- [ ] **1.3** schema.rs: `models` field `HashMap` → `IndexMap` (line 73)
- [ ] **1.4** schema.rs: Default impl `HashMap::new()` → `IndexMap::new()` (lines 133-134)
- [ ] **1.5** schema.rs: `effective_providers()` return type (line 204) + empty return (line 222)
- [ ] **1.6** schema.rs: `effective_models()` return type (line 232)
- [ ] **1.7** schema.rs: `interpolate_env_vars_with()` param type (line 499)
- [ ] **1.8** registry.rs: `profiles` field (line 21)

Batch: `W0-A-indexmap-migration.md` → W0-A-2

---

### Parallel Group 2 — Type Propagation (up to 6 agents in parallel, after Group 1)

**Agent A: roko-agent references (~10 min)**
- [ ] **2.1** dispatch_resolver.rs: add import, change line 156
- [ ] **2.2** provider/mod.rs: add import, change lines 392, 416

Batch: `W0-A-indexmap-migration.md` → W0-A-3

**Agent B: roko-cli references (~15 min)**
- [ ] **2.3** config.rs: add import, change lines 65, 68
- [ ] **2.4** model_selection.rs: add import, change lines 339-340
- [ ] **2.5** plan_validate.rs: add import, change lines 102, 114, 122, 269, 941
- [ ] **2.6** commands/config_cmd.rs: change lines 1043, 1065, 2027, 2049, 2080
- [ ] **2.7** tests/plan_validation.rs: add import, change lines 38, 44

Batch: `W0-A-indexmap-migration.md` → W0-A-4

**Agent C: roko-learn + roko-serve references (~5 min)**
- [ ] **2.8** cost_table.rs: add import, change line 111
- [ ] **2.9** routes/providers.rs: add import, change line 571

Batch: `W0-A-indexmap-migration.md` → W0-A-5

**Agent D: SessionNewParams extension (~30 min)**
- [ ] **2.10** types.rs: Add model/provider/effort to SessionNewParams (lines 240-253)
- [ ] **2.11** types.rs: Add warnings to SessionNewResult (lines 283-295)
- [ ] **2.12** session.rs: Rewrite create_session with param overrides (lines 753-761)
- [ ] **2.13** session.rs: Add `warnings: Vec::new()` to new_result() constructor

Batch: `W1-A-session-new-params.md`

**Agent E: Default fallback logic (~20 min)**
- [ ] **2.14** session.rs: Rewrite from_roko_config fallback (lines 175-204) — prefer first ready provider
- [ ] **2.15** session.rs: Add tracing::warn when agent.default_model not found

Batch: `W1-B-default-fallback-logic.md`

**Agent F: MCP status types (~10 min)**
- [ ] **2.16** types.rs: Add McpServerStatus struct (after line 263)
- [ ] **2.17** types.rs: Add McpInitStatus enum
- [ ] **2.18** types.rs: Add McpServerStatus::ready() and ::failed() constructors

Batch: `W2-A-mcp-status-types.md`

---

### Parallel Group 3 — MCP + Commands + Config (up to 4 agents, after Group 2)

**Agent G: MCP error accumulation (~30 min, needs Agent F done)**
- [ ] **3.1** bridge_events.rs: Change setup_session_mcp_tools signature → return tuple
- [ ] **3.2** bridge_events.rs: Add `let mut statuses` accumulator
- [ ] **3.3** bridge_events.rs: Site 1 — HTTP transport → push TransportUnsupported
- [ ] **3.4** bridge_events.rs: Site 2 — spawn failure → push SpawnFailed
- [ ] **3.5** bridge_events.rs: Site 3 — initialize failure → push InitializeFailed
- [ ] **3.6** bridge_events.rs: Site 4 — initialize timeout → push InitializeTimeout
- [ ] **3.7** bridge_events.rs: Site 5 — tools/list failure → push ToolsListFailed
- [ ] **3.8** bridge_events.rs: Site 6 — tools/list timeout → push ToolsListTimeout
- [ ] **3.9** bridge_events.rs: Add success status push after tool collection
- [ ] **3.10** bridge_events.rs: Change return to tuple
- [ ] **3.11** bridge_events.rs: Fix caller in run_openai_compat_mcp_tool_loop

Batch: `W2-B-mcp-error-accumulation.md`

**Agent H: Command categories (~30 min)**
- [ ] **3.12** types.rs: Add `category: Option<String>` to SlashCommand (line 635-646)
- [ ] **3.13** session.rs: Add category to each of ~47 commands in build_slash_commands (lines 1116-1427)
- [ ] **3.14** session.rs: Change build_slash_commands signature to accept `bare_mode: bool`
- [ ] **3.15** session.rs: Add filtering logic at end of build_slash_commands
- [ ] **3.16** handler.rs: Change send_slash_commands_notification to accept bare_mode param (lines 363-379)
- [ ] **3.17** handler.rs: Pass `sessions.roko_config.agent.bare_mode` at call sites (lines 176, 276)

Batch: `W3-A-command-categories.md`

**Agent I: max_output surfacing (~15 min)**
- [ ] **3.18** provider.rs: Add `effective_max_output()` method to ModelProfile impl
- [ ] **3.19** session.rs: Include effective max_output in model option descriptions (build_config_options, line 961-971)
- [ ] **3.20** loader.rs: Add diagnostic warning for max_output < 1000 (in collect_diagnostics)

Batch: `W4-A-max-output-surfacing.md`

**Agent J: Provider readiness (~15 min)**
- [ ] **3.21** types.rs: Add `ready: bool` field to ConfigOptionValue (lines 586-597)
- [ ] **3.22** session.rs: Set `ready` from `is_provider_available` in provider options (lines 949-958)
- [ ] **3.23** session.rs: Set `ready` from provider availability in model options (lines 960-971)
- [ ] **3.24** session.rs: Add `ready: true` to all other ConfigOptionValue construction sites (effort, temperament, etc.)

Batch: `W4-B-provider-readiness.md`

---

### Parallel Group 4 — MCP Notification (1 agent, after Agent G, ~30 min)

- [ ] **4.1** bridge_events.rs: Add McpStatus variant to CognitiveEvent enum (line 207-236)
- [ ] **4.2** bridge_events.rs: Emit mcp_statuses via CognitiveEvent in run_openai_compat_mcp_tool_loop
- [ ] **4.3** bridge_events.rs: Handle McpStatus in map_event_to_update (lines 3315-3354)
- [ ] **4.4** types.rs: Add McpStatus variant to SessionUpdate enum (find it near AgentMessageChunk etc.)
- [ ] **4.5** bridge_events.rs: Add discovery_timeout_ms to per-server timeout selection (W2-D)
- [ ] **4.6** types.rs: Add discovery_timeout_ms to McpServerConfig (W2-D)

Batches: `W2-C-mcp-notification.md` + `W2-D-mcp-config-options.md`

---

## Phase 2: Compilation + Fix (1 agent, after ALL Phase 1)

- [ ] **5.1** `cargo +nightly fmt --all`
- [ ] **5.2** `cargo build --workspace 2>&1 | head -200` — fix errors iteratively
- [ ] **5.3** `cargo clippy --workspace --no-deps -- -D warnings` — fix warnings
- [ ] **5.4** `cargo test --workspace` — fix any broken tests

---

## Phase 3: Integration Test (1 agent, after Phase 2)

- [ ] **6.1** Run `cd tmp/solutions/ide/tests && bash run-all.sh`
- [ ] **6.2** Verify "session/new respects model param" → PASS (was FAIL)
- [ ] **6.3** Verify "nonexistent MCP binary → structured error" → PASS (was FAIL)
- [ ] **6.4** Verify "MCP binary that exits → structured error" → PASS (was FAIL)
- [ ] **6.5** Verify "nonexistent model → error or warning" → PASS (was WARN)

---

## Summary: Maximum Parallelism

```
Group 0:  1 agent   (~10 min)
Group 1:  1 agent   (~15 min)
Group 2:  6 agents  (~30 min, the longest sub-task)
Group 3:  4 agents  (~30 min)
Group 4:  1 agent   (~30 min)
Phase 2:  1 agent   (~30 min)
Phase 3:  1 agent   (~15 min)
────────────────────────────────
Total wall clock: ~2.5 hours (vs ~6 hours sequential)
Total agent-time: ~14 agents × ~20 min avg
```
