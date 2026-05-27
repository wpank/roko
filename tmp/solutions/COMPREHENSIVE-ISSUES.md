# Comprehensive Issue Catalog

Date: 2026-04-28 (second pass — full demo-app + CLI + API)
Source: End-to-end testing of CLI, demo-app source analysis, roko-serve API endpoint testing
Test workspace: `/tmp/roko-e2e-1777398304`

Every issue found, with severity, evidence, root cause, and fix approach.

---

## Table of Contents

1. [CLI: Provider & Model Routing](#1-cli-provider--model-routing)
2. [CLI: Gate Execution](#2-cli-gate-execution)
3. [CLI: Cost & Token Tracking](#3-cli-cost--token-tracking)
4. [CLI: Learning & Feedback Path](#4-cli-learning--feedback-path)
5. [CLI: Interactive Chat](#5-cli-interactive-chat)
6. [CLI: One-Shot Dispatch](#6-cli-one-shot-dispatch)
7. [CLI: Plan Generation & Validation](#7-cli-plan-generation--validation)
8. [CLI: Config & Init](#8-cli-config--init)
9. [API: Shape Mismatches](#9-api-shape-mismatches)
10. [API: Missing or Broken Endpoints](#10-api-missing-or-broken-endpoints)
11. [API: Route Path Mismatches](#11-api-route-path-mismatches)
12. [Demo: Scenario Execution](#12-demo-scenario-execution)
13. [Demo: Dashboard Data Quality](#13-demo-dashboard-data-quality)
14. [Demo: UI Logic Issues](#14-demo-ui-logic-issues)
15. [Security](#15-security)
16. [Architecture & Design Debt](#16-architecture--design-debt)

---

## 1. CLI: Provider & Model Routing

### 1.1 Fresh workspace defaults to broken provider routing
**Severity:** Critical
**Evidence:** `roko run "prompt"` in a fresh workspace tries `anthropic_api` (no key) → fails.
After `roko config migrate`, routes to `claude_cli` correctly.
**Root cause:** `roko init` emits schema v1 config with `[agent] command = "claude"` but no
`[providers]` table. WorkflowEngine reads `[providers]` and `[models]` — neither exist.
**Fix:** `roko init` should emit schema v2 config with `[providers.claude_cli]` section and a
`[models]` table that references it. Alternatively, `roko init` can auto-run `config migrate`.

### 1.2 `--model` flag ignored by most paths
**Severity:** High
**Evidence:** `roko "prompt" --model gpt-4o`, `--model glm-5-1` — response always comes from
glm-5.1 via zai (one-shot) or fails (run/interactive). `prd plan --model claude-haiku-4-5`
used Opus anyway.
**Root cause:** CLI arg is parsed but not propagated to the workflow engine's model resolution.
The engine has its own provider/model resolution that ignores CLI overrides.
**Fix:** Thread `--model` as a hard override through `EffectiveModelSelection`, which should
be shared by all agent-starting commands. CLI `--model` should override config unless a
command documents and prints a stronger planner policy.

### 1.3 `config models route` always returns Sonnet
**Severity:** Medium
**Evidence:** `roko config models route gpt-4o` → routes to Sonnet. `route llama32` → Sonnet.
**Root cause:** The router falls through to the default model without checking model availability.
**Fix:** The route command should actually exercise the cascade router with the requested model
slug and return the effective model + provider, or error if not configured.

### 1.4 Provider health shows 0/0 healthy
**Severity:** Medium
**Evidence:** `/api/health` returns `"providers":{"healthy":0,"total":0,"unhealthy":0}`.
**Root cause:** Provider health check not wired in default serve mode, or no providers
registered in AppState.
**Fix:** Wire provider discovery into AppState init. At minimum, enumerate configured providers
from roko.toml and probe them at startup.

### 1.5 Model slugs explosion
**Severity:** Low
**Evidence:** `/api/learn/cascade-router` returns 22 model slugs, most of which are aliases
or unavailable models (`o3`, `o4-mini`, `kimi-k2`, `glm45-flash`, etc.).
**Root cause:** The cascade router seed table includes every model slug ever configured,
with no pruning.
**Fix:** Either prune model_slugs to only configured/available models, or add an `available`
flag. The UI should only show models that can actually be routed.

---

## 2. CLI: Gate Execution

### 2.1 Shell gate always fails — `gate_for_name("shell")` has no match case
**Severity:** Critical
**Evidence:** `roko run` with `[[gate]] kind = "shell" program = "true"` always reports gate
failure, triggering infinite autofix loops.
**Root cause:** `orchestrate.rs:7522` converts `GateConfig::Shell` to the string `"shell"`.
`gate_service.rs:65-80` has no match case for `"shell"` in `gate_for_name()`. Falls through
to `_ => None`, which creates `GateVerdict { passed: false }`.
The actual `ShellGate` implementation at `gate_service.rs:119-135` is never instantiated.
**Fix:** Add `"shell" => Some(Box::new(ShellGate::new(program, args)))` to `gate_for_name()`.
One-line fix.

### 2.2 Stub gates report as passing instead of skipped
**Severity:** Medium
**Evidence:** E2E dogfood showed `symbol`, `generated_test`, `verify_chain`, `fact_check`,
`llm_judge`, `integration` gates all reporting as "passed" when they're actually stubs.
**Root cause:** Unimplemented gate variants return `GateVerdict { passed: true }` by default.
**Fix:** Return `GateVerdict { passed: true, skipped: true }` or a new `not_wired` status
so that dashboards and learning don't treat stubs as real validation.

---

## 3. CLI: Cost & Token Tracking

### 3.1 Cost always $0.00 for all episodes
**Severity:** High
**Evidence:** `prd plan` ran 471s with Claude Opus ($1.46 real billing). Roko logged
`cost_usd: 0.0, input_tokens: 0, output_tokens: 0` in efficiency.jsonl and episodes.
**Root cause:** `ClaudeCliAgent` parses stream-json for progress and final text but returns
`AgentResult.usage` with `wall_ms` only. Token/cost metadata from Claude CLI `result` event
is never extracted.
**Fix:** Parse the `result` event from Claude CLI stream-json output. Extract
`input_tokens`, `output_tokens`, `cost_usd` into `AgentResult.usage`. If usage is
unavailable, record as null/unknown rather than numeric zero.

### 3.2 Cost events double-counted
**Severity:** Medium
**Evidence:** costs.jsonl logs an attempt once as success and again as gate failure for the
same attempt. This inflates cost tracking by 2x.
**Root cause:** The efficiency logger records both the agent completion event and the
subsequent gate failure event, each carrying the same cost.
**Fix:** Emit one cost event per agent attempt. Gate failure is a separate event that should
not duplicate the cost.

### 3.3 Status shows negative cost ($-0.0000)
**Severity:** Low
**Evidence:** `roko status` shows `Total: $-0.0000`.
**Root cause:** Floating point display artifact when cost is exactly 0.0. The negative sign
comes from `-0.0` representation.
**Fix:** Use `max(0.0, cost)` or absolute value when formatting cost display.

### 3.4 Model recorded as "unknown-model" in some events
**Severity:** Low
**Evidence:** Some efficiency/cost events show `model: "unknown-model"`.
**Root cause:** When the model is not resolved before dispatch, the logger uses a fallback string.
**Fix:** Ensure model resolution happens before logging. If truly unknown, use `None` not a
string that looks like a model name.

---

## 4. CLI: Learning & Feedback Path

### 4.1 `learn all` says "empty" despite existing data
**Severity:** Medium
**Evidence:** `.roko/learn/efficiency.jsonl` has 22 entries. `roko learn all` says "empty".
**Root cause:** The `learn` command reads from a different expected path than what the
efficiency logger writes, or uses a different format parser.
**Fix:** Align read/write paths. The `learn all` command should read from the same paths
listed in `/api/learn/efficiency`'s evidence section.

### 4.2 Cascade router has 0 observations from actual runs
**Severity:** Medium
**Evidence:** cascade-router.json shows `total_observations: 0` in demo workspace.
Roko workspace shows 15, but all from seed/manual data, not from real dispatch feedback.
**Root cause:** The dispatch loop doesn't feed outcomes back to the cascade router.
**Fix:** After each agent dispatch completes, call `cascade_router.observe(model, success)`.

---

## 5. CLI: Interactive Chat

### 5.1 Interactive `roko` (no args) fails immediately
**Severity:** Critical
**Evidence:** `echo "hello" | roko` → tries `anthropic_api`, fails with "Missing API key".
**Root cause:** The no-args chat path uses `dispatch_direct.rs` which doesn't read
provider config and falls back to anthropic_api.
**Fix:** This is the ChatAgentSession work from FINAL-SOLUTION.md. Route through
`ClaudeCliAgent` instead of building raw `claude` subprocess commands.

### 5.2 No system prompt in chat dispatch
**Severity:** High
**Evidence:** `dispatch_direct.rs:140-143` spawns Claude with only `--print --output-format stream-json`.
No `--model`, `--effort`, `--append-system-prompt`, `--tools`, `--mcp-config`, `--resume`.
**Root cause:** The interactive path was a minimal stub that was never completed.
**Fix:** Use `ClaudeCliAgent` command builder which already has all these flags.

### 5.3 No session continuity (--resume)
**Severity:** High
**Evidence:** Each chat turn starts a fresh Claude session. No `session_id` is extracted
or passed as `--resume` on subsequent turns.
**Root cause:** `dispatch_direct.rs` doesn't extract or store session state.
**Fix:** `ChatAgentSession` should own session_id, extract it from response, pass it as
`--resume` on next turn.

### 5.4 No slash commands work
**Severity:** Medium
**Evidence:** `/model`, `/system`, `/effort`, `/reset` are not implemented in the chat path.
**Root cause:** Chat REPL doesn't have slash command parsing or session mutation.
**Fix:** `ChatAgentSession` should handle `/command` prefixes and mutate session state.

### 5.5 Ctrl-C doesn't cleanly kill child process
**Severity:** Medium
**Evidence:** Interrupting a chat turn may leave orphaned Claude processes.
**Root cause:** No signal forwarding or process group management in dispatch_direct.
**Fix:** `ClaudeCliAgent` already handles child process lifecycle. Route through it.

---

## 6. CLI: One-Shot Dispatch

### 6.1 Positional prompt has no tools or context
**Severity:** High
**Evidence:** `roko "What files are here?"` → Claude responds with generic knowledge,
no tool use, no workspace awareness.
**Root cause:** One-shot path through `dispatch_direct.rs` doesn't pass tools or MCP config.
**Fix:** Route one-shot prompts through the same `ClaudeCliAgent` path as `roko run`.

### 6.2 One-shot falls back to wrong provider
**Severity:** High
**Evidence:** Even with `claude` CLI available, one-shot falls back to `zai/glm-5.1` via
OpenAI-compat.
**Root cause:** One-shot path has a separate provider fallback that finds zai keys in the
environment before checking for Claude CLI.
**Fix:** Unify provider resolution. Claude CLI should be preferred when available.

---

## 7. CLI: Plan Generation & Validation

### 7.1 Generated tasks.toml missing required `role` field
**Severity:** High
**Evidence:** `plan validate` correctly identifies all tasks missing `role`.
**Root cause:** The plan generator prompt doesn't mention the required `role` field.
**Fix:** Either add `role` to the plan generator prompt/schema, or make `role` optional
in the runner with a default value.

### 7.2 Plan uses unconfigured model aliases
**Severity:** High
**Evidence:** Generated plans use `sonnet` and `haiku` as model hints. These don't match
configured model names like `claude-sonnet-4-6`.
**Root cause:** Plan generator prompt uses shorthand model names. No normalization.
**Fix:** Either normalize model aliases before execution (map `sonnet` →
`claude-sonnet-4-6`), or make the plan generator aware of configured model names.

### 7.3 Generated plans create greenfield for existing functionality
**Severity:** Medium
**Evidence:** Asked to build "config file parser" → plan creates `roko-config` crate with
22 structs in a repo that already has config loading.
**Root cause:** Plan generator has no mechanism to check existing code for overlap.
The workspace context pack doesn't verify the repo root or existing crates.
**Fix:** This is Batch 4 from FINAL-SOLUTION.md. Build a repository context pack before
generation. Validate plans against existing Cargo.toml workspace members.

### 7.4 `plan regenerate` doesn't fix known validation issues
**Severity:** Medium
**Evidence:** After `plan validate` identifies missing roles and bad model names,
`plan regenerate` produces the same invalid output.
**Root cause:** Regeneration doesn't feed validation errors back into the prompt.
**Fix:** Pass validation failures as context to the regeneration prompt.

---

## 8. CLI: Config & Init

### 8.1 `roko init` emits schema v1 (no providers table)
**Severity:** Critical (blocks all `roko run` / interactive paths)
**Evidence:** Fresh `roko init` creates config without `[providers]` or `[models]`.
`roko run` then fails on provider resolution.
**Root cause:** init template hasn't been updated for schema v2.
**Fix:** Update `roko init` to emit schema v2 with `[providers.claude_cli]` section.

### 8.2 `resume` uses wrong plans path
**Severity:** Medium
**Evidence:** `roko resume` hardcodes `./plans` but PRD-generated plans go to `.roko/plans`.
**Root cause:** Path mismatch between plan generation output and resume reader.
**Fix:** `resume` should use the same `.roko/plans` root as PRD-generated plans.

### 8.3 No config preflight/validation on first run
**Severity:** Medium
**Evidence:** Running any agent command on a v1 config gives cryptic errors instead of
"run `config migrate` first".
**Root cause:** No preflight check for required config sections.
**Fix:** Add a preflight check before agent dispatch. If `[providers]` is missing,
suggest `roko config migrate` or auto-migrate.

---

## 9. API: Shape Mismatches (server returns different shape than demo expects)

### 9.1 `/api/dashboard` returns rendered text, not JSON
**Severity:** High
**Evidence:** Server returns `{"rendered": "dashboard scaffold: 16 pages, 39 widgets..."}`.
Demo `CostDashboard.tsx` expects `{ total_cost, cache_hit_rate, routing_distribution, gate_pass_rate }`.
**Root cause:** Server endpoint returns TUI dashboard text representation instead of JSON data.
Demo works because `useApiWithFallback` catches the shape mismatch and falls back to
`DEMO_DASHBOARD` data. But this means live data is never shown on the dashboard.
**Fix:** Either:
  (a) Make `/api/dashboard` return the JSON shape the demo expects, OR
  (b) Remove the `/api/dashboard` endpoint since `CostDashboard.tsx` already assembles
  its data from `/api/health`, `/api/learn/efficiency`, `/api/metrics/c_factor`, and
  `/api/learn/cascade-router` (which it calls directly). The dashboard endpoint is redundant.

### 9.2 `/api/managed-agents` — empty capabilities, domain_tags, zero reputation
**Severity:** High
**Evidence:**
```
relay-demo: capabilities=[], domain_tags=[], reputation=0,
            performance.completed_tasks=0, costs.cumulative_usd=None
```
Demo expects: `capabilities: ['rust','systems'], domain_tags: ['systems'], reputation: 92`
**Root cause:** Registered agents are bare entries with no profile data. Agent creation
doesn't populate capabilities, domain_tags, or reputation.
**Fix:** Two approaches:
  (a) Populate agent metadata on registration (from roko.toml agent profiles), OR
  (b) Derive from dispatch history (capabilities from task domains, reputation from gate pass rate)

### 9.3 `/api/statehub/events` — wrapped shape vs flat array
**Severity:** Medium
**Evidence:** Server returns `{ after_seq, cursor, events: [{ cursor, event: {...}, seq, ts_millis }], limit }`.
Demo `Explorer.tsx:99` does `const evts = await get<StateEvent[]>('/api/statehub/events')` and then
iterates as `events.map(evt => evt.type)`.
**Root cause:** Server wraps events in envelope with cursor pagination. Demo expects flat array
of `{ type, payload, timestamp }`.
**Fix:** Either:
  (a) Unwrap in the demo: `const wrapper = await get(...); setEvents(wrapper.events.map(e => e.event))`, OR
  (b) Add a `?flat=true` query param on the server that returns just the event array.

### 9.4 `/api/learn/cascade-router` — confidence_stats has extra fields
**Severity:** Low (doesn't break, just noise)
**Evidence:** Server returns confidence_stats with 14 fields per model
(`gemini_code_execution_failures`, `perplexity_requests`, etc.). Demo expects only
`{ successes, trials }`.
**Root cause:** The projection contract dumps the full internal stats struct.
**Fix:** The demo `CascadeRouter.tsx` already only reads `successes` and `trials`, so extra
fields are harmless. But for cleanliness, the projection could include a `summary` view.

### 9.5 `/api/health` — providers shape mismatch
**Severity:** Medium
**Evidence:** Server returns `{ healthy: 0, total: 0, unhealthy: 0 }` (counts).
Demo's Explorer `getProviders()` tries to interpret as both `Record<name, {healthy: boolean}>`
AND the count shape, with a fallback that synthesizes named providers.
CostDashboard shows `${health.providers.healthy}/${health.providers.total} providers` → "0/0".
**Root cause:** Two different shapes expected. Explorer handles both but CostDashboard only
handles counts (which are always 0/0).
**Fix:** Wire provider discovery into AppState so the health endpoint reports real provider
data. Consider returning both `{ names: {...}, summary: { healthy, total } }`.

### 9.6 `/api/metrics/c_factor` — sub_metrics key mismatch
**Severity:** Medium
**Evidence:** Demo `CostDashboard.tsx` expects 5 keys:
`gate_pass_rate, cost_efficiency, speed, reuse_rate, learning_rate`.
Server returns 11 keys: `convergence_velocity, cost_efficiency, first_try_rate,
gate_pass_rate, hdc_diversity, information_flow_rate, knowledge_growth,
knowledge_integration_rate, social_perceptiveness, speed, turn_taking_equality`.
**Root cause:** Server computes the full c-factor breakdown. Demo only displays 5 specific
metrics that it hardcodes in the `METRICS` array.
**Fix:** The demo should display all available sub_metrics instead of hardcoding 5. The
missing `reuse_rate` and `learning_rate` from demo data don't exist in the real API, while
real API has metrics the demo doesn't show.

---

## 10. API: Missing or Broken Endpoints

### 10.1 `/api/share/:token` returns SPA HTML (catch-all)
**Severity:** High
**Evidence:** `curl /api/share/test-token` → returns full SPA HTML page.
Demo `Share.tsx:28` calls `GET /api/share/${token}` expecting JSON receipt.
**Root cause:** There is no `/api/share/:token` route. The SPA catch-all serves HTML.
Server has `/api/shared/:token` (note: `shared` not `share`).
**Fix:** Either:
  (a) Add `/api/share/:token` as alias for `/api/shared/:token` on the server, OR
  (b) Fix `Share.tsx` to use `/api/shared/${token}`.
The `ShareView.tsx` (dashboard) correctly uses `/api/shared/${token}`.

### 10.2 `/api/knowledge/*` always returns empty
**Severity:** Medium
**Evidence:** `/api/knowledge/entries` → `{ items: [], total: 0 }`.
`/api/knowledge/edges` → `{ items: [], total: 0 }`.
Demo `KnowledgeGraph.tsx` and `KnowledgeEntries.tsx` fall back to demo data (18 entries, 28 edges).
**Root cause:** The neuro knowledge store at `.roko/neuro/knowledge.jsonl` has 0 records.
Knowledge is never populated from agent runs.
**Fix:** After successful agent episodes, seed knowledge entries from task outcomes,
gate results, and prompt patterns. Or populate on `roko init --demo` for demo purposes.

### 10.3 `/api/bench/runs/:id` returns SPA HTML (catch-all)
**Severity:** High
**Evidence:** `curl /api/bench/runs/0a57b782...` → returns SPA HTML.
Server route is `/api/bench/run/:id` (singular), not `/api/bench/runs/:id` (plural).
**Root cause:** Route path mismatch between demo-app and server.
**Fix:** See section 11 below for full route path mismatch list.

### 10.4 SSE `/api/bench/events` returns empty on timeout
**Severity:** Medium
**Evidence:** `curl --max-time 2 /api/bench/events?bench_id=...` → empty response.
**Root cause:** SSE stream may only emit events during active bench runs. With no active
run, the stream is idle.
**Fix:** This is expected behavior for SSE. The demo handles this via polling fallback
(useBench.ts polls every 3s if SSE unavailable). No fix needed unless events are lost
during active runs.

---

## 11. API: Route Path Mismatches (demo calls X, server has Y)

### 11.1 Bench run creation
**Demo calls:** `POST /api/bench/runs`
**Server has:** `POST /api/bench/run` (singular)
**Impact:** Creating a bench run from the UI silently fails (falls back to demo data).
**Fix:** Either rename server route to `/api/bench/runs` (RESTful plural) or fix demo to
use `/api/bench/run`.

### 11.2 Bench run detail
**Demo calls:** `GET /api/bench/runs/:id`
**Server has:** `GET /api/bench/run/:id` (singular)
**Impact:** Viewing bench run details from UI returns SPA HTML, not JSON.
**Fix:** Same as 11.1 — align route names.

### 11.3 Bench run cancel
**Demo calls:** `POST /api/bench/runs/:id/cancel`
**Server has:** `DELETE /api/bench/run/:id`
**Impact:** Cancel button in UI does nothing.
**Fix:** Align route: either add `POST /api/bench/runs/:id/cancel` or fix demo to use
`DELETE /api/bench/run/:id`.

### 11.4 Share endpoint
**Demo calls (Share.tsx):** `GET /api/share/:token`
**Server has:** `GET /api/shared/:token`
**Impact:** Share page never loads real data.
**Fix:** Fix demo `Share.tsx` to use `/api/shared/:token` (ShareView.tsx already correct).

### 11.5 Health probe uses minimal endpoint
**Demo probes:** `GET ${SERVE_URL}/health` → returns `{"status":"ok"}` (minimal, no details)
**Full health:** `GET ${SERVE_URL}/api/health` → returns full health with providers, statehub, agents
**Impact:** Probe succeeds (server detected as live). But then real API calls may return
shapes the demo doesn't expect, causing individual pages to show partial or broken data
instead of clean demo fallback data.
**Root cause:** Two health endpoints with different detail levels. Probe succeeds → live
mode → real API calls → shape mismatches → broken UI.
**Fix:** Low priority for probe itself. The real fix is aligning API shapes (section 9).

---

## 12. Demo: Scenario Execution

### 12.1 Selfhost scenario will fail on fresh workspace
**Severity:** Critical
**Commands:** `prd idea`, `prd draft new`, `prd plan`, `status`, `learn all`
**Issue:** Without `config migrate`, `prd draft new` and `prd plan` may fail on provider
resolution. Even if they succeed via Claude CLI fallback, `learn all` will show "empty".
**Fix:** Scenario should include `roko config migrate -y` as step 0, or `roko init` should
emit v2 config.

### 12.2 Builder scenario depends on `roko run` which is broken pre-migrate
**Severity:** Critical
**Commands:** `roko run "<prompt>"`
**Issue:** Without config migrate, `roko run` fails. With migrate, shell gate always fails
→ infinite autofix loop.
**Fix:** Fix config init (8.1) + shell gate (2.1). Both must work for Builder demo.

### 12.3 Race scenario compares broken vs broken
**Severity:** Critical
**Commands:** `roko run --no-replan` vs `roko run` (both panes)
**Issue:** Both paths are broken (same issues as Builder). The comparison is meaningless.
**Fix:** Same prerequisites as Builder: config init + shell gate.

### 12.4 Providers scenario references unconfigured providers
**Severity:** High
**Commands:** `roko run --provider zhipu/openai/anthropic/moonshot`
**Issue:** `--provider` flag may not work with the workflow engine's provider resolution.
Even if it does, zhipu and moonshot require API keys that won't be configured.
**Fix:** The providers scenario should detect which providers are actually configured and
only run those. Or use a fallback prompt that doesn't require real LLM output.

### 12.5 Explore scenario — most commands work but with poor data
**Severity:** Medium
**Commands:** `status`, `doctor`, `prd list`, `learn all/efficiency/tune gates`,
`config providers/models/validate`, `knowledge stats/query`, `explain`
**Issue:** `learn all` → empty. `knowledge stats/query` → empty. `explain` may fail without
provider. Other commands work but show minimal data.
**Fix:** Ensure a demo workspace has pre-seeded data for all these commands.

### 12.6 Chat scenario fails immediately
**Severity:** Critical
**Commands:** `roko` (bare), then type messages
**Issue:** Interactive `roko` fails with missing API key (issue 5.1). Even if it starts,
slash commands `/status` and `/model` are not implemented (issue 5.4).
**Fix:** Requires ChatAgentSession implementation (FINAL-SOLUTION.md Batch 1-2).

### 12.7 Mirage scenario — no visible output
**Severity:** Low (intentional placeholder)
**Commands:** None (just connects WebSocket and clears terminal)
**Issue:** This is a placeholder for blockchain fork visualization (Phase 2+).
**Fix:** Either implement with demo data or mark as "Coming Soon" in the UI.

---

## 13. Demo: Dashboard Data Quality

### 13.1 CostDashboard shows all zeros/defaults when live
**Severity:** High
**Evidence:** Live server data: cost=$0.00, c-factor=0.459, episodes=2, gates_passed=0.
Demo fallback: cost=$1.42, c-factor=0.847, episodes=847, gates_passed=791.
**Root cause:** Real workspace has minimal data. All costs are $0 (issue 3.1).
When server is live, the dashboard shows real (mostly-zero) data instead of impressive demo numbers.
**Fix:** Two tracks:
  (a) Fix cost tracking (3.1) so real data is non-zero
  (b) Add a "demo mode" flag that uses demo data even when server is live (for investor demos)

### 13.2 AgentFleet shows bare agents with no metadata
**Severity:** High
**Evidence:** 3 agents with empty capabilities, 0 reputation, 0 tasks, no domain tags.
Demo expects 5 agents with rich profiles.
**Root cause:** Agent registration creates minimal entries (issue 9.2).
**Fix:** Populate agent metadata from roko.toml profiles or seed demo agents on `roko init --demo`.

### 13.3 KnowledgeGraph always empty from real API
**Severity:** Medium
**Evidence:** 0 entries, 0 edges. Falls back to 18/28 demo data.
**Root cause:** Neuro store never populated (issue 10.2).
**Fix:** Seed from episode outcomes or provide demo data seeding.

### 13.4 CascadeRouter shows raw internal model names
**Severity:** Low
**Evidence:** `glm-5.1` with 14-field stats objects. 28 roles with many pointing to
models that aren't available.
**Root cause:** Cascade router has bloated seed data and no pruning.
**Fix:** Prune to available models only. Display human-friendly names.

### 13.5 ChainView is Phase 2 placeholder
**Severity:** Low
**Evidence:** Shows fake rotating SHA-256 hash. No real chain data.
**Root cause:** Intentional — chain runtime not implemented yet.
**Fix:** Mark as "Coming in Phase 2" or hide from nav in demo mode.

---

## 14. Demo: UI Logic Issues

### 14.1 Health probe fails → all pages show demo data silently
**Severity:** High
**Evidence:** `useApiWithFallback.ts:37` probes `${SERVE_URL}/health` (wrong URL).
This always fails → `_serverLive = false` → every page uses fallback demo data.
**Root cause:** Missing `/api` prefix in probe URL.
**Fix:** Change `probeServer()` to fetch `${SERVE_URL}/api/health`.

### 14.2 Mosaic cells fall back to hardcoded demo values
**Severity:** Medium
**Evidence:** `AgentFleet.tsx:96`: `value={agents.length || 5}` — if 0 agents, shows 5.
`MosaicCell label="TASKS DONE" value={totalTasks || 827}` — if 0 tasks, shows 827.
**Root cause:** Fallback values baked into component render to make demo look good.
But this means you can't tell if you're looking at real or demo data.
**Fix:** Remove inline fallbacks. Use a dedicated `isDemoMode` state to conditionally
apply demo values. When live, show actual values (even if 0).

### 14.3 BenchRunDetail gets SPA HTML instead of JSON
**Severity:** Medium
**Evidence:** `BenchRunDetail.tsx` fetches `GET /api/bench/runs/:id`. Server route is
`/api/bench/run/:id` (singular). The request hits the SPA catch-all → HTML.
**Root cause:** Route path mismatch (issue 11.2).
**Fix:** Align bench route paths between demo and server.

### 14.4 Explorer episodes tab may crash on real data
**Severity:** Low
**Evidence:** Explorer expects episode array directly. API returns array (correct), but
episode objects have 27 fields vs demo's 11. `JSON.stringify(ep)` in search would work but
expanded detail view dumps all 27 fields with no formatting.
**Fix:** Filter displayed fields in episode detail view. Show key fields prominently,
collapse rest into "raw" section.

### 14.5 Explorer events tab crashes on real data shape
**Severity:** Medium
**Evidence:** Events are wrapped `{ cursor, event: {...}, seq, ts_millis }`.
Code does `evt.type` which is undefined (it's `evt.event.type`).
Code does `new Date(evt.timestamp).toLocaleTimeString()` — `timestamp` doesn't exist
(it's `ts_millis`).
**Root cause:** Shape mismatch (issue 9.3). Demo data has flat shape, real data has envelope.
**Fix:** Unwrap events in the handler: `events.map(e => ({ ...e.event, timestamp: new Date(e.ts_millis).toISOString() }))`.

---

## 15. Security

### 15.1 No auth on serve by default
**Severity:** High
**Evidence:** `roko-core/src/config/serve.rs:54-57` — auth disabled by default.
Terminal PTY endpoint allows arbitrary shell command execution.
**Root cause:** Development convenience. Never hardened.
**Fix:** FINAL-SOLUTION.md Batch 0: disable background serve for no-args `roko` by default.
Enable auth when binding non-localhost. Restrict terminal routes.

### 15.2 Terminal sessions allow arbitrary command execution
**Severity:** High
**Evidence:** `POST /api/terminal/sessions` creates a shell. `WebSocket /ws/terminal/:id`
gives full PTY access. No auth, no command filtering.
**Root cause:** Designed for demo/development use without security controls.
**Fix:** Add auth token requirement. Only allow terminal sessions when explicitly enabled.
Consider read-only mode for demos.

### 15.3 Background serve exposed on all interfaces
**Severity:** Medium
**Evidence:** `roko serve` binds on 0.0.0.0:6677 or localhost:6677 depending on config.
Interactive `roko` may auto-start serve in background.
**Root cause:** serve startup doesn't check if interface should be restricted.
**Fix:** Default to localhost-only. Require explicit `--bind 0.0.0.0` for external access.

---

## 16. Architecture & Design Debt

### 16.1 Two completely separate dispatch paths
**Severity:** High (design)
**Description:** `dispatch_direct.rs` (one-shot/interactive) and `WorkflowEngine` (run/plan)
have zero shared code. Provider resolution, model selection, system prompt, tools, and MCP
config are implemented independently in each.
**Fix:** The ChatAgentSession from FINAL-SOLUTION.md is the unification point.
Both paths should use `ClaudeCliAgent` (or the provider adapters) through a shared interface.

### 16.2 Duplicate state tracking
**Severity:** Medium (design)
**Description:** `status`, `plan list`, `resume`, and `.roko/state/run-state.json` all
maintain separate views of task/plan state that disagree.
**Fix:** Single source of truth for plan/task state. Other views derive from it.

### 16.3 Throwaway HTTP clients in model_call_service
**Severity:** Medium (perf)
**Description:** New `reqwest::Client` created per API call instead of reusing a pooled client.
**Fix:** Create client once in init, share via `Arc<Client>`.

### 16.4 `orchestrate.rs` is 8000+ lines
**Severity:** Low (maintainability)
**Description:** Single file handles dispatch, gates, enrichment, efficiency, cost, state,
resume, and more. Hard to navigate.
**Fix:** Extract into focused modules: dispatch.rs, gates.rs, enrichment.rs, efficiency.rs.
Not urgent but would help future development.

### 16.5 Demo data and real API shapes drift
**Severity:** Medium (ongoing)
**Description:** `demo-data.ts` defines expected shapes that don't match real API responses.
This means the demo always looks good (using fake data) but live mode is broken.
**Fix:** Generate demo data from actual API response types, or add integration tests
that verify demo data shape matches API response shape.

### 16.6 Mutex/unwrap patterns in hot paths
**Severity:** Low (reliability)
**Description:** Several `Mutex::lock().unwrap()` calls in dispatch and logging paths.
A panicked thread could poison the mutex and cascade.
**Fix:** Use `parking_lot::Mutex` (no poisoning) or handle lock errors gracefully.
Some paths already use parking_lot — make it consistent.

### 16.7 No demo-mode seeding
**Severity:** Medium (DX)
**Description:** There's no way to create a workspace with realistic demo data pre-seeded.
This means every demo starts with empty data.
**Fix:** Add `roko init --demo` that seeds episodes, knowledge, agents, and cost data
so the dashboard immediately shows something interesting.

---

## Priority Matrix

### Quick Wins (<1h each, unblocks demos)

| # | Issue | Impact |
|---|---|---|
| 2.1 | Shell gate match case | Unblocks `roko run` |
| 9.3+14.5 | Fix events shape unwrap | Unblocks Explorer events tab |
| 11.1-4 | Fix bench route paths | Unblocks bench UI |
| 10.1 | Fix share endpoint URL | Unblocks share pages |
| 3.3 | Fix negative cost display | Cosmetic but embarrassing |

### Medium (1-4h each, makes demo credible)

| # | Issue | Impact |
|---|---|---|
| 8.1 | Init emits schema v2 | Unblocks fresh workspace flows |
| 9.1 | Dashboard JSON endpoint | Live dashboard data |
| 9.2 | Agent metadata population | AgentFleet shows real data |
| 7.1 | Plan generator role field | Plans pass validation |
| 9.3 | Events shape unwrap | Explorer events tab works |
| 14.2 | Remove inline demo fallbacks | Can distinguish live vs demo data |
| 16.7 | Add `roko init --demo` | Demo seeding for presentations |

### Large (1+ days each, structural fixes)

| # | Issue | Impact |
|---|---|---|
| 5.1-5.5 | ChatAgentSession | Interactive chat works |
| 3.1 | Cost tracking from stream-json | All cost displays show real data |
| 1.2 | --model flag threading | Model selection works |
| 7.3 | Grounded plan generation | Plans reference existing code |
| 6.1-6.2 | One-shot dispatch unification | Positional prompts work |
| 15.1-15.3 | Security hardening | Safe for non-local use |
| 16.1 | Dispatch path unification | Eliminates duplicate provider resolution |
| 16.5 | Type-safe demo data generation | Prevents future shape drift |
