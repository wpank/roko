# Current State, Gaps, and Structural Analysis

> Every claim in this document is verified against source code on branch `wp-arch2`.
> File paths are absolute. Line numbers reference the working tree at the time of audit.
> "Wired" means the code is called from a live CLI or HTTP path. "Built" means
> the code compiles and has tests but is not reachable from any user-facing surface.

---

## 1. Aggregate Parity Numbers

**Audit v3 scope**: 130 tasks audited of 147 committed (88%).

| Category | Count | Notes |
|---|---|---|
| SOLID | 90 | Works as described, wired, tested |
| PARTIAL | 37 | Implemented with meaningful gaps |
| HOLLOW | 3 | Claimed done but fundamentally broken |
| Runtime crashes | 3 | panic! or unreachable! on live paths |
| Security findings | 4 | 1 critical, 1 high, 1 medium, 1 low |
| Structural anti-patterns | 8 | Dual paths, dead code, config splits |

The parity numbers overstate readiness. 37 PARTIAL tasks means 28% of audited work
has gaps significant enough to affect users. The 3 HOLLOW tasks are particularly
concerning because they represent features that appear in documentation and help text
but crash or do nothing when invoked.

---

## 2. Three HOLLOW Tasks (Verified)

### HOLLOW 1: `roko config mcp` -- Runtime Panic

**File**: `crates/roko-cli/src/commands/config_cmd.rs`
**Symptom**: `ConfigCmd::Mcp` arm falls through to `unreachable!()`
**Impact**: Running `roko config mcp list` crashes the process

The MCP config subcommand was added to the CLI parser (clap derives the subcommand
variant) but never wired to a handler. The `match` arm for `ConfigCmd::Mcp` hits
`unreachable!()` which panics. This is the most basic form of "built but never wired."

**Fix complexity**: Low. The MCP config is already in `roko.toml` and parseable.
The handler just needs to read `config.agent.mcp_config` and `config.agent.mcp_servers`
and format them for display.

### HOLLOW 2: API Provider Stub -- No `send_turn_api`

**File**: `crates/roko-agent/src/claude_cli_agent.rs` (approximate location)
**Symptom**: Guard clause returns `ApiProviderNotImplemented` error
**Impact**: The `api_history` and `http_client` fields are constructed but never used

Roko's agent dispatch has two paths: Claude CLI (subprocess) and API (HTTP).
The API path was scaffolded with fields for HTTP history tracking but the actual
`send_turn_api()` function body is a guard clause that returns an error immediately.
All real dispatch goes through the CLI subprocess path.

This is not a user-facing crash but it means the `--backend api` flag (or any config
that selects the API provider directly) silently fails.

### HOLLOW 3: Plan Regenerate Diagnostics -- R4_C05

**File**: `crates/roko-cli/src/commands/plan.rs`
**Symptom**: `plan regenerate` validates after generation but does NOT inject
diagnostics into the regeneration prompt
**Impact**: Regeneration ignores validation errors, producing the same bad plan

The `plan regenerate` command:
1. Runs an agent to regenerate the plan
2. Validates the output (checks for valid TOML, references existing files, etc.)
3. If validation fails, reports the error to the user

What it should do but does not:
1. Run agent to regenerate
2. Validate output
3. If validation fails, inject validation errors into a new prompt
4. Re-run agent with error context
5. Repeat up to 2 iterations

The validation-feedback loop is missing. The agent generates blindly without
knowing what went wrong.

---

## 3. Eight Structural Anti-Patterns (Verified)

### AP-1: Two Parallel Execution Engines

**Paths**: `crates/roko-cli/src/runner/` (v2, default) vs `crates/roko-cli/src/orchestrate.rs` (legacy)

Runner v2 is the default for `plan run`. orchestrate.rs has the full learning
stack (CascadeRouter, AdaptiveThresholds, episodes, replanning, C-factor, playbooks,
section effectiveness, knowledge queries, crate familiarity, Daimon affect, enrichment).

This is not merely "two implementations." It means the system's most valuable features --
the ones that make it learn from experience -- are unreachable from the default path.

**Evidence**: Runner v2's `event_loop.rs` imports `LearningRuntime`, `Episode`,
`AgentEfficiencyEvent`, `RoutingContext` and constructs a `LearningRuntime` from
`LearningPaths`. But the actual recording calls are either absent or partial.
Specifically:
- `CascadeRouter::observe()` is not called after dispatch
- `AdaptiveThresholds::observe()` is not called after gate execution
- No `build_gate_failure_plan_revision()` call exists
- `SectionEffectivenessRegistry` is not updated

### AP-2: Two Parallel Model Selection Paths

**Paths**: `EffectiveModelSelection` (CLI `run` / `chat`) vs inline 8-step pipeline
(orchestrate.rs `dispatch_agent_with()`)

The CLI's model selection resolves through `auth_detect.rs` which scans environment
variables in fixed priority, ignoring `roko.toml` configuration. The orchestrate.rs
path uses a multi-step pipeline: task requirements -> CascadeRouter -> tier models ->
domain override -> role override -> fallback.

These two paths can produce different model selections for the same configuration.
Setting `default_model = "glm51"` in roko.toml has no effect when running via
`roko run` because `auth_detect.rs` picks up `ANTHROPIC_API_KEY` and routes to
Claude regardless.

The ServiceFactory (`service_factory.rs`) resolves this correctly via `resolve_model()`,
but not all entry points use ServiceFactory.

### AP-3: Two Parallel Gate Execution Paths

**Paths**: Runner v2 uses `[gates]` table from roko.toml. orchestrate.rs uses
`[[gate]]` array with per-task verification, adaptive thresholds, and episode logging.

`roko init` writes `[[gate]]` arrays. `RokoConfig::from_toml()` silently discards
`[[gate]]` arrays. So gates generated by `roko init` are invisible to `roko plan run`.

The config schema split means:
- `roko init` generates valid-looking configuration
- The runtime ignores it
- Users see no error message
- Gate behavior falls back to defaults

### AP-4: Two Parallel Playbook Mechanisms

**Paths**: Bench uses real agent tool-call extraction to build playbooks.
Plan executor uses static TOML metadata from tasks.toml.

The PlaybookStore in `.roko/learn/playbooks/` is populated by orchestrate.rs
from actual agent tool sequences. But runner v2 reads task metadata from TOML
(static, never updated from real executions). The two systems produce different
playbook data that never converges.

### AP-5: Legacy-Gated ACP Features

**Environment variable**: `ROKO_ACP_LEGACY`

File changes, phase badges, narrative text, and forensic analysis in the ACP
pipeline require the `ROKO_ACP_LEGACY` environment variable to be set. Without it,
these features are compiled but gated behind `cfg` checks or runtime env reads.

This means ACP sessions running without the env var produce less informative output
with no indication that features are being suppressed.

### AP-6: Config Schema Split (`[[gate]]` vs `[gates]`)

**Impact**: Gates generated by `roko init` are silently ignored.

The root cause is in `RokoConfig::from_toml()`. The TOML parser handles `[gates]`
(a table with `enabled = [...]`) but not `[[gate]]` (an array of gate objects).
The `roko init` command writes `[[gate]]` because it was originally designed for
the orchestrate.rs path which expected that format.

Fix options:
1. Make the parser accept both formats (normalize to single internal repr)
2. Change `roko init` to write the format the runtime reads
3. Both

### AP-7: Streaming Events Silently Drained

**File**: `crates/roko-cli/src/chat_inline.rs`

The chat inline handler creates a streaming channel, spawns the agent, then
drains events with:
```rust
while let Some(_event) = event_rx.recv().await {}
```

The underscore-prefixed binding means every streaming event is received and
immediately discarded. The TUI stays in a spinner animation showing no agent
output until the entire response is complete.

This is not a bug in the sense that it crashes -- it is architecturally correct
but functionally useless. The streaming infrastructure works; the consumer ignores it.

### AP-8: Context Pack Not Wired Into Plan Generation

**File**: `crates/roko-cli/src/repo_context.rs` (`build_repo_context()`)

`build_repo_context()` is called from `prd draft new` to give the drafting agent
awareness of the repository structure. It is NOT called from:
- `plan generate`
- `plan regenerate`
- `prd plan`

This means generated plans are created without repository awareness. Agents
propose greenfield crates that duplicate existing functionality, reference
non-existent modules, and create file structures that conflict with the
established layout.

---

## 4. Security Findings (Verified)

### CRITICAL: Share Routes Outside Auth Middleware

**File**: `crates/roko-serve/src/routes/shared_runs.rs`

`POST /api/runs/{id}/share` is mounted OUTSIDE the auth middleware layer.
Any caller -- authenticated or not -- can create share links for any run.

This exposes agent transcripts (which may contain repository code, API keys,
internal documentation) to anyone who can reach the server.

### HIGH: Auth Opt-In and Off By Default

The HTTP control plane (`roko serve`) binds to `0.0.0.0:6677` with auth disabled
by default. Cloud deployments (Railway, Fly, Docker) expose this publicly.

`roko deploy railway` does not generate or require an API key. The deployed
server is immediately accessible with no authentication.

### MEDIUM: `acknowledge_public_risk` Bypass

Setting `acknowledge_public_risk = true` in config bypasses the terminal-displayed
auth warning without checking whether `api_auth.enabled` is actually true. A user
can acknowledge the risk warning and still run without auth, believing they have
addressed the security concern.

### LOW: CLI Gist Path Sends Unscrubbed Transcripts

`roko run --share` creates a GitHub Gist with the raw agent transcript. The HTTP
share path (`/api/runs/{id}/share`) applies secret scrubbing before sharing.
The CLI path does not. API keys, tokens, and other secrets embedded in agent
output are uploaded to GitHub as-is.

---

## 5. Runner v2 Status and Migration Path

### Phase Status

| Phase | Scope | Status |
|---|---|---|
| A: Build | 15 files, ~2,181 lines | Complete |
| B: Wire for `--approval` mode | Event loop + TUI bridge | Complete |
| C: Make default for all `plan run` | CLI dispatch routing | Complete |
| D: Deprecate orchestrate.rs | Rename, feature-gate | Open |
| E: Align with unified spec | Type renames, event schema | Open |

### Runner v2 Feature Gaps (Detailed)

Each gap listed with the specific orchestrate.rs code that needs porting and the
runner v2 location where it should be wired.

| Gap | orchestrate.rs Source | Runner v2 Target | Complexity |
|---|---|---|---|
| CascadeRouter observations | `record_task_success()` -> `cascade_router.observe()` | `event_loop.rs` after task completion | Low |
| AdaptiveThreshold updates | `run_gate_pipeline()` -> `thresholds.observe()` | `gate_dispatch.rs` after gate verdict | Low |
| Episode logging | `record_episode()` writes to `episodes.jsonl` | `event_loop.rs` on task completion | Low |
| Efficiency events | `emit_efficiency_event()` with 30+ fields | `event_loop.rs` on agent completion | Medium |
| Replan-on-gate-failure | `maybe_emit_gate_failure_plan_revision()` | `event_loop.rs` on gate failure | Medium |
| Section effectiveness | `update_section_effectiveness()` | `event_loop.rs` on task completion | Low |
| C-factor computation | `CFactorSummary::compute()` | `event_loop.rs` at run end | Low |
| Knowledge store queries | `knowledge_store.query()` -> system prompt | `dispatch` module | Medium |
| Playbook injection | `playbook_store.find_matching()` -> Layer 6 | `dispatch` module | Medium |
| Crate familiarity | `familiarity_tracker.record()` | `event_loop.rs` on task completion | Low |
| Daimon affect | `daimon_state.modulate()` -> dispatch params | `dispatch` module | Medium |
| Enrichment pipeline | `EnrichmentPipeline::run()` -> context sections | `dispatch` module | High |
| Model name in events | `model` field in TUI events | `tui_bridge.rs` | Low |

The "Low" complexity items are wiring calls -- the code exists, the types are
imported, the construction happens. The gap is calling the method at the right point.

The "Medium" complexity items require constructing additional state or passing
additional context through the dispatch pipeline.

The "High" complexity item (enrichment pipeline) requires significant refactoring
because the enrichment pipeline is deeply coupled to orchestrate.rs's PlanRunner struct.

---

## 6. Dogfood Learnings (April 26, 2026)

### Six Critical Fixes

These were discovered during the first real self-hosting run and represent the
class of bugs that only appear under real workload.

**Fix 1: `force_shutdown()` Self-Kill**

Process group signal (`kill -TERM -$$`) killed the roko process itself along with
child agents. Fixed by masking SIGTERM before sending the group signal.

**Fix 2: No State Persistence**

`executor.json` was never written during runs. The auto-save-every-5-actions
logic existed in orchestrate.rs but the save function was never called. The runner
ran to completion or crashed without any recoverable state.

Fixed by calling `save_state()` after every phase transition in the executor.

**Fix 3: Efficiency Events Not Flushed**

`append_efficiency_event()` appended to an in-memory Vec but never called
`flush()` on the underlying writer. Events accumulated in memory and were lost
on process exit.

Fixed by calling `flush()` after each append. Runner v2 also flushes after
every task completion.

**Fix 4: Model Routing Fallback to Haiku**

CascadeRouter had a hardcoded fallback to `claude-3-haiku` when no observations
existed. This ignored the user's configured models entirely. A user configuring
`default_model = "claude-sonnet-4-20250514"` would get Haiku for the first 50 tasks.

Fixed by merging configured models into the router's candidate set before
applying the static routing table.

**Fix 5: Implementation Phase Never Dispatching**

Missing `ensure_task_tracker()` call at the start of `handle_implementing()`.
Plans were loaded and the DAG was built, but the task tracker was not initialized,
so tasks were never dispatched. The executor sat in "Implementing" phase forever.

Fixed by calling `ensure_task_tracker()` before the dispatch loop.

**Fix 6: TOML Parse Failure on Markdown Fences**

LLMs wrap TOML output in markdown code fences (` ```toml ... ``` `). The TOML
parser failed on the fence markers. Plans generated by agents were unparseable.

Fixed by adding `extract_toml_payload()` which strips code fences, and
`TasksFile::parse_agent_output()` which tries extraction before raw parse.

### Key Patterns Discovered

| Pattern | Description |
|---|---|
| Built but never wired | Dominant failure mode. Code exists, compiles, has tests, but is never called from a live path |
| Batch, not streaming | orchestrate.rs waited for agent exit and read all output at once. Runner v2 fixed this with line-by-line parsing |
| Two event systems | ServerEvent (HTTP SSE) and DashboardEvent (TUI) overlap with lossy conversion between them |
| Plans dir ambiguity | `plans/` vs `.roko/plans/` -- code checks only one |
| God object | orchestrate.rs at 22K+ lines, every dogfood fix touched it |
| Config schema split | `roko init` writes one TOML schema; `roko plan run` reads another |
| Memory leaks | 9.5-11.5GB RSS after 17 min. Unbounded vectors, enrichment artifacts held with no GC |

---

## 7. Master Task Priorities

### Section 1: Demo and Pitch (Deadline: Past)

These were P0 for a May 6 deadline and are presumably complete or deferred.
Keeping for historical context.

- `nunchi` CLI wrapper, agent management, audit, resume, replay
- Pre-warm LLM cache, demo backups
- TUI streaming, TOML fence stripping, memory investigation
- Slide deck, pre-read memo, landing page

### Section 2: Runtime Bugs (Active)

| Bug | File/Location | Impact | Status |
|---|---|---|---|
| Enrichment timeout hardcode | 120s in gate judge config | Judge timeouts on large diffs | Open |
| Model shows "-" in TUI | Runner v2 passes empty string for model | User sees no model info | Open |
| Memory leak (efficiency_events Vec) | `event_loop.rs` unbounded Vec | RSS grows without bound | Fixed in v2 (per-task flush) |
| `signals.jsonl` dead path | Writes to `engrams.jsonl` instead | Signal log never populated | Open |
| Learn files stale in runner v2 | `cascade-router.json` etc. never updated | Learning never persists | Open |

### Section 3: Runner v2 Completion (Active)

Phase D (deprecate orchestrate.rs) and Phase E (unified spec alignment) are open.
The detailed gap list in Section 5 above enumerates every feature that needs porting.

### Section 4: UX/Wiring (40 Open of 112)

- CognitiveWorkspace VCG auction: `vcg_allocate()` built but greedy path dominates
- ExtensionChain: Extension loading exists but no extensions are registered
- Hardcoded paths: Various paths assume specific directory layouts
- TUI event parity: DashboardEvent does not cover all RuntimeEvent variants

### Section 5: Spec Migration (95 Batches)

0 done, 78 pending, 17 blocked. These are type renames and structural changes
to align with the unified specification documents.

### Section 6: Gap-Fix PRDs (46 Tasks Across 6 PRDs)

| PRD | Scope | Tasks |
|---|---|---|
| Chain integration | Phase 2+ blockchain witness anchoring | 8 |
| Config unification | `[[gate]]`/`[gates]` split, model selection unification | 7 |
| Event bridge | ServerEvent/DashboardEvent consolidation | 6 |
| Gates/safety | Judge gate, safety contracts enforcement | 8 |
| Learning/neuro | Runner v2 learning wiring | 10 |
| Dead code cleanup | Remove orchestrate.rs, unused imports, dead paths | 7 |

### Section 7: Deferred/Blocked

| Item | Reason |
|---|---|
| Chain runtime integration | Needs blockchain backend (Phase 2+) |
| Dreams cron trigger | Built but no runtime scheduling mechanism |
| Cold substrate archival | Built but not instantiated at runtime |
| Knowledge-informed routing | Neuro store not consulted in CascadeRouter |
| `force_backend` override learning | CascadeRouter doesn't learn from manual overrides |

---

## 8. Prioritized Fix List

### P0 -- Must Fix (Blocks Users or Is a Security Vulnerability)

| # | Issue | File | Fix Description |
|---|---|---|---|
| 1 | Wire `ConfigCmd::Mcp` dispatch | `commands/config_cmd.rs` | Replace `unreachable!()` with handler reading `config.agent.mcp_config` |
| 2 | Move share routes inside auth | `routes/shared_runs.rs` | Move `POST /api/runs/{id}/share` from public router to protected router |
| 3 | Auto-provision auth on cloud deploy | `commands/deploy.rs` | Generate random API key, set `api_auth.enabled = true`, print key |

### P1 -- Feature Does Not Work As Claimed

| # | Issue | File | Fix Description |
|---|---|---|---|
| 4 | Forward streaming events to TUI | `chat_inline.rs` | Replace `while let Some(_event)` with actual event mapping to DashboardEvent |
| 5 | Wire `build_repo_context` into plan generate | `commands/plan.rs` | Call `build_repo_context()` before agent dispatch in generate/regenerate/prd plan |
| 6 | Inject validation diagnostics into regeneration | `commands/plan.rs` | On validation failure, inject errors into prompt, retry up to 2x |
| 7 | Read pipeline template from config | `workflow_engine.rs` | Read `[workflow]` table from roko.toml instead of hardcoded "standard" |
| 8 | Add scrubbing to CLI Gist path | `share.rs` | Apply `scrub_secrets()` before GitHub Gist upload |

### P2 -- Correctness and Quality

| # | Issue | File | Fix Description |
|---|---|---|---|
| 9 | Unify model selection paths | `auth_detect.rs`, `service_factory.rs` | Make all entry points use `ServiceFactory::build()` for model resolution |
| 10 | Normalize model aliases | `service_factory.rs` | Resolve `glm-5-1`/`glm51` to canonical slug at load time |
| 11 | Thread provider usage through ACP streaming | `roko-acp/src/bridge_events.rs` | Accumulate tokens during stream, emit efficiency event on completion |
| 12 | Make grounding validation blocking | `commands/plan.rs` | Reject plans that create new crates when existing crates cover the domain |
| 13 | Make `roko init` emit `[gates]` format | `commands/init.rs` | Change init template to match runtime parser expectations |
| 14 | Wire runner v2 learning persistence | `runner/event_loop.rs` | Add CascadeRouter/threshold/episode recording calls (see Section 5 gap list) |

---

## 9. Config Analysis

### 12 Providers Configured (from roko.toml)

anthropic, openai, perplexity, moonshot, zai, zhipu, cerebras, ollama, gemini,
openrouter, claude_cli

### 31 Models Configured

Spanning: Anthropic (sonnet, haiku, opus), OpenAI (o4-mini, gpt-5.x), ZhiPu
(glm-5.x), Moonshot (kimi-k2.x), Perplexity (sonar-x), Cerebras (llama 8b/70b/scout),
Gemini (flash/pro), Ollama (gemma4, llama32)

### Notable Duplicate Model Entries

- `glm-5-1` on provider "zai" vs `glm51` on provider "zhipu" -- both resolve to `glm-5.1`
- Multiple Claude aliases that resolve to the same model ID

### Critical Config-Runtime Disconnects

| Config Field | Expected Behavior | Actual Behavior |
|---|---|---|
| `default_model = "glm51"` | All dispatch uses GLM-5.1 | Thrown away by most dispatch paths |
| `auth_detect.rs` | Respects config providers | Ignores config, scans env vars in fixed priority |
| `[[gate]]` arrays | Parsed and used for gate execution | Silently discarded by `RokoConfig::from_toml()` |
| `agent.mcp_config` | MCP servers available to agents | Passed through in orchestrate.rs only |
| `workflow.template` | Selects express/standard/full | WorkflowEngine reads it; runner v2 does not |
| `budget.prompt_token_budget` | Limits prompt assembly tokens | Read by ServiceFactory, but PromptAssemblyService ignores 0 |
| `learning.replan_on_gate_failure` | Triggers replan loop | Read by orchestrate.rs only; runner v2 ignores |
| `agent.tier_models` | Per-tier model mapping | Loaded into CascadeRouter but tier routing is not called at dispatch |
| `workflow.max_iterations` | Caps implementation loops | WorkflowEngine respects; runner v2 uses its own `max_retries` |
| `tools.profiles` | Per-role tool restrictions | Assembled into instructions but never enforced at gate level |

### Config Field Load Paths (Traced Through Source)

The following shows where each config field is loaded and whether it reaches
the execution engine:

```
roko.toml
  |
  +-- [agent]
  |     +-- default_model -----> ServiceFactory (YES) -----> WorkflowEngine (YES)
  |     |                   +--> auth_detect.rs (NO, ignored)
  |     |                   +--> runner v2 dispatch (PARTIAL, via DispatchContext)
  |     +-- default_backend ---> auth_detect.rs (NO, not read)
  |     +-- mcp_config -------> orchestrate.rs (YES) ----> runner v2 (PARTIAL)
  |     +-- tier_models -------> CascadeRouter (LOADED) --> dispatch (NOT CALLED)
  |     +-- fallback_model ----> ServiceFactory (YES) ----> CascadeRouter candidates
  |
  +-- [gates] / [[gate]]
  |     +-- enabled -----------> runner v2 gate_dispatch (YES, from [gates])
  |     +-- [[gate]] array ----> RokoConfig::from_toml (DISCARDED)
  |     +-- max_rung ----------> GateService (YES)
  |     +-- shell_gates -------> GateService (YES, from ShellGateCommand)
  |
  +-- [workflow]
  |     +-- template ----------> WorkflowEngine (YES)
  |     |                   +--> runner v2 (NO, uses its own config)
  |     +-- max_iterations ----> WorkflowEngine (YES)
  |     +-- has_review --------> WorkflowEngine (YES)
  |
  +-- [learning]
  |     +-- replan_on_gate_failure -> orchestrate.rs (YES)
  |     |                        +--> runner v2 (NO)
  |     +-- cascade_router_enabled -> CascadeRouter (LOADED)
  |                              +--> runner v2 (NOT WIRED)
  |
  +-- [budget]
        +-- prompt_token_budget -> PromptAssemblyService (CONDITIONAL, 0=ignored)
        +-- max_cost_per_run ----> orchestrate.rs budget check (YES)
                              +--> runner v2 (NO)
```

### 9+ Dispatch Paths with Inconsistent Model Selection

| Entry Point | Model Resolution Path | Uses Config? |
|---|---|---|
| `roko run` | ServiceFactory -> resolve_model -> EffectDriver | Yes |
| `roko chat` | ServiceFactory -> resolve_model -> EffectDriver | Yes |
| `roko plan run` (v2) | runner dispatch -> auth_detect scan | Partial |
| `roko plan run` (legacy) | dispatch_agent_with -> CascadeRouter | Yes (full) |
| `roko agent chat` | Direct provider construction | Partial |
| `roko research` | Perplexity hardcoded | No |
| `roko prd draft` | Agent dispatch with config | Partial |
| `roko prd plan` | Agent dispatch with config | Partial |
| HTTP `/message` | ModelCallService | Yes |

---

## 10. What "Done" Looks Like

The system reaches feature completeness when the following concrete criteria are met.
Each criterion is testable with a specific verification procedure.

### Criterion 1: Single Execution Path

Runner v2 (for multi-task plans) and WorkflowEngine (for single-task workflows)
are the only execution paths. orchestrate.rs is deleted from the codebase.

**Verification**: `find crates/ -name 'orchestrate*.rs' | wc -l` returns 0.
All `roko plan run` invocations route through `runner::run()`.

### Criterion 2: Learning Persists on Every Run

Episodes, efficiency events, routing observations, gate threshold observations,
and section effectiveness measurements are all written to disk after every task,
regardless of which entry point was used.

**Verification**: Run `roko plan run` on a 3-task plan. After completion:
- `.roko/episodes.jsonl` has 3+ entries (one per task attempt, more if autofix triggered)
- `.roko/learn/cascade-router.json` has `observations > 0`
- `.roko/learn/gate-thresholds.json` has per-rung stats with `total_observations > 0`
- `.roko/learn/efficiency.jsonl` has entries with non-zero token counts
- `.roko/learn/section-effects.json` has section entries

Run `roko run "hello"` (single-task path). After completion:
- `.roko/episodes.jsonl` has a new entry
- `.roko/learn/cascade-router.json` observation count increased

### Criterion 3: Config Is Respected

`default_model` in roko.toml determines the model for all dispatch paths.
Gate configuration is honored regardless of format.

**Verification**:
- Set `default_model = "cerebras-70b"` in roko.toml
- Run `roko run "hello"` -- agent log shows Cerebras, not Claude
- Run `roko plan run` -- same result
- Run `roko chat` -- same result
- `roko init` generates gates, `roko plan run` uses those gates

### Criterion 4: No Silent Failures

Every configured feature either works or produces a clear error message.

**Verification**:
- `roko config mcp list` -- no crash, shows MCP servers or "none configured"
- Set `enabled_gates = ["judge"]` -- message says "LLM judge running" not "skipped: not implemented"
- `roko chat` -- streaming text appears in TUI, not spinner-only
- Set an invalid model -- clear error "model X not found in config"

### Criterion 5: Security by Default

Auth enabled on cloud deployments. Share routes behind auth. Secret scrubbing
on all share paths.

**Verification**:
- `roko deploy railway` -- output includes API key
- `curl -X POST deployed-url/api/runs/test/share` without auth -- 401
- `roko run --share` with API key in output -- Gist shows `[REDACTED]`

### Criterion 6: Replan Loop Complete

Gate failures that exhaust the autofix budget trigger replanning.

**Verification**:
- Create a task that fails compile gate consistently
- Set `max_auto_fix_iterations = 2`
- Run the task. After 2 autofix attempts fail:
  - A strategist agent is spawned with the gate errors
  - The strategist produces a revised approach
  - The task retries with the revised approach
  - If the revision also fails, the task reports failure with full context

### Criterion 7: Judge Gate Functional

LLM judge at rung 6 performs real code review.

**Verification**:
- Set `enabled_gates = ["compile", "clippy", "test", "judge"]`
- Run a task that passes compile/lint/test
- Judge gate runs an LLM call with the git diff
- Gate verdict includes the judge's reasoning
- Cost is non-zero (confirms real LLM call, not stub)

### Effort Estimate

The gap between current state and "done" by these criteria:

| Criterion | Estimated Effort | Blocking Dependencies |
|---|---|---|
| 1. Single path | 5-7 days | Criteria 2 (learning must be ported first) |
| 2. Learning persistence | 2-3 days | None |
| 3. Config respected | 3-4 days | None |
| 4. No silent failures | 2-3 days | Criteria 3 (config must work) |
| 5. Security | 1 day | None |
| 6. Replan loop | 2 days | Criteria 2 (learning must persist) |
| 7. Judge gate | 2 days | None |

**Total**: 17-22 days of focused work. Critical path is Criteria 2 -> 1 -> 4,
which is ~10-14 days. Other criteria can be parallelized.
