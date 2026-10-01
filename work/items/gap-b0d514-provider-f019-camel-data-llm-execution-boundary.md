+++
id = "gap-b0d514"
kind = "gap"
title = "CaMeL Data-LLM execution boundary not enforced"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "features"
subsystem = ["roko-agent/safety"]
created = 2026-09-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "52f00fb94"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F019"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F019"
anchors = ["crates/roko-agent/src/safety/data_llm.rs::DataLlmRouter", "crates/roko-agent/src/tool_loop/mod.rs::ToolLoop", "crates/roko-agent/src/dispatcher/mod.rs::tool_result_taint", "crates/roko-core/src/config/agent.rs::DataLlmConfig", "crates/roko-cli/src/graph_task_dispatch/inert_settings.rs::graph_engine_inert_settings"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q '\"agent.data_llm\"' crates/roko-cli/src/graph_task_dispatch/inert_settings.rs && grep -rqw 'fn untrusted_tool_result_never_reaches_main_model_raw' crates/roko-agent/src/ && cargo test -p roko-agent untrusted_tool_result_never_reaches_main_model_raw"
+++

## Problem

roko has a CaMeL-style "Data LLM" router, but no dispatch path uses it. Untrusted content (web fetch and search
results, MCP and plugin tool output) goes straight back into the main model's context. The idea is that tainted
content is first processed by a separate, tool-less model, and only its schema-validated result reaches the
privileged model. `DataLlmRouter` can decide, sanitize and validate, but nothing calls it.

How to see it:
- `grep -rln 'DataLlmRouter\|sanitize_input(' crates/ --include='*.rs'` finds only `safety/data_llm.rs` itself,
  its re-export in `safety/mod.rs:86-88`, and a doc comment in `crates/roko-core/src/config/agent.rs`.
- Setting `[agent.data_llm]` in `roko.toml` does nothing. `graph_engine_inert_settings` lists it as a key that
  "no production code reads" (`crates/roko-cli/src/graph_task_dispatch.rs:792`, entry at `:899`); that list is
  reported when a Graph dispatcher is built and by `roko config doctor`.

Expected (when `[agent.data_llm]` is set): tool results from untrusted sources pass through the Data LLM before
they are appended to the main model's messages. The raw tainted text never reaches the main model, and any Data
LLM failure blocks the content instead of passing it through.

## Why it matters

- Goal `features`. This is a missing prompt-injection defence, not a regression: the design exists in code and
  config but has no effect.
- The config key and module docs advertise a protection that is not there. Until this is wired, docs and
  `roko config` output should not imply otherwise.
- Related: `gap-ff95f5` (persist taint, witness and custody provenance across restart) is where routing decisions
  would be recorded durably. That is a soft dependency: useful, not required first.

## Where

- `crates/roko-agent/src/safety/data_llm.rs` (439 lines, last changed in `310860465`):
  - `DataLlmDecision` (`:36`): `Passthrough` or `RouteToDataLlm { reason }`. The variant
    `SanitizationRequired` named in the original notes does not exist.
  - `sanitize_input` (`:82`): strips a fixed list of injection phrases, case-insensitively.
  - `DataLlmRouter` (`:155-251`): `route(&Taint)` (`:169`; `None`/`UserInput` pass through, and
    `ExternalFetch`, `ThirdPartyPlugin` and `LegacyImport` route), accessors for model, max tokens,
    temperature and `strip_tool_calls`, `maybe_sanitize` (`:210`), and `validate_output` (`:225`). The latter
    only checks JSON syntax and the schema's top-level `required` keys.
  - `DataLlmAuditEntry` (`:255`).
- `crates/roko-core/src/config/agent.rs`: `AgentConfig.data_llm: Option<DataLlmConfig>` (`:76-81`, documented
  as "reserved"), and `DataLlmConfig` (`:289-330`: `model` defaults to `MODEL_FAST`, plus `max_tokens`,
  `temperature`, `strip_tool_calls` (default true), `output_schema`, `sanitize_input` (default true)).
- `crates/roko-core/src/provenance.rs::Taint` (`:123`): the action-centric taint the router takes.
- The boundary where tool output enters the model context:
  `crates/roko-agent/src/tool_loop/mod.rs` (`~:1284-1289`): `dispatcher.dispatch_batch(calls, ctx)`, then
  `translator.render_results(&results)`, then `result_msg::append_results(&mut messages, ...)`.
- Per-result trust: `crates/roko-agent/src/dispatcher/mod.rs::tool_result_taint` (`:1175-1183`) maps
  `ToolSource::Mcp`/`WebSearch`/`Retrieval`/`Plugin` and network builtins to `CamelTaintLevel::Untrusted`,
  and other builtins to `Local`. `ctx.raise_taint(result_taint)` is at `:874`.
- Roko-owned tool loops that would get the boundary (`ToolLoop::new` sites):
  `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs`, `provider/openai_compat.rs`,
  `provider/cerebras.rs`, `gemini/adapter.rs`, `perplexity/`, `openai_compat_backend.rs`,
  `tool_loop/agent_wrapper.rs`, and the ACP loops in `crates/roko-acp/src/bridge_events/dispatch.rs`.
- The inert-config entry to remove once wired: `crates/roko-cli/src/graph_task_dispatch.rs::graph_engine_inert_settings` (`:899`).

## Current state

- Infrastructure only. The router, sanitizer, output check and audit struct have unit tests in `data_llm.rs`.
  No production caller exists (re-checked 2026-09-29 at `d9e79e9d8`).
- Taint is tracked, but separately: the dispatcher raises the context's `CamelTaintLevel` after untrusted tools.
  Contracts can use that level to refuse later privileged calls, but the content itself is not transformed.
- CLI providers (Claude CLI, Codex, Gemini CLI, Cursor) run their own tool loops inside the subprocess. roko never
  sees their tool results before the model does, so this boundary cannot apply to them.
- No JSON Schema validator crate is in the workspace (`jsonschema` is not a dependency).
- The archived backlog spec (#352, "Blocked on shared runtime services") assumed a six-field `RuntimeServices`
  with a `DispatchServices` bundle. That shape no longer exists: `crates/roko-execution/src/builder.rs:118`
  has `dispatch: Arc<DispatchFactory>` among about ten fields. Its blocker (#243, shared runtime services)
  and #350 (single production safety chain; see `ProductionSafetyChain` in the dispatcher) no longer block
  this work.

## Plan

1. Config (`DataLlmConfig`, `crates/roko-core/src/config/agent.rs`): keep `None` = disabled. Add only what the
   live boundary needs: `timeout_ms`, `max_input_bytes`, and optionally a taint floor. Validate that `model`
   resolves and that `strip_tool_calls` is true. Do not add a second `[safety.data_llm]` section.
2. Data-only caller: a small type (for example `DataLlmBoundary { router: DataLlmRouter, backend: Arc<dyn
   LlmBackend> }`) built from the configured model through the existing provider factory. Send it only a fixed
   system prompt, the bounded tainted text and the output schema. Pass an empty tool list, and no MCP, plugin,
   secrets, workspace paths or main system prompt.
3. Wire it into `ToolLoop` once: add an optional `with_data_llm(boundary)` builder. In the loop, after
   `dispatch_batch` and before `render_results`/`append_results`, map each result's source to `Taint`
   (MCP/Plugin -> `ThirdPartyPlugin(server or plugin)`, WebSearch/Retrieval/network builtins ->
   `ExternalFetch(label)`, local builtins -> `None`). For routed results: `maybe_sanitize`, bound the size, call
   the data model, validate, and replace the result text with the validated JSON, tagged as data. The loop needs
   the source per result: return it alongside each result from `dispatch_batch`, or re-derive it from the
   registry with `tool_result_taint`.
4. Output validation. Options:
   - (a) Recommended: a fixed, narrow Rust output type (for example `{ summary: String, facts: Vec<String> }`)
     deserialized with serde and checked for size and depth. No new dependency, and it is hard to smuggle
     instructions through.
   - (b) Full JSON Schema validation of `output_schema` with a new dependency (`jsonschema`). More flexible, but
     it is a dependency decision to raise with the owner first.
5. Failure policy: when enabled, a timeout, provider error or invalid output replaces the result with a typed
   "untrusted content withheld: <reason>" tool error. Never fall back to the raw text. When disabled, keep current
   behaviour.
6. Construct the boundary where agents are built from config (the shared agent factory used by
   `create_agent_for_model`, and the ACP loops), so every roko-owned tool loop gets it from
   `config.agent.data_llm`. Then remove the `agent.data_llm` `NO_READER` entry at
   `graph_task_dispatch.rs:899` and update the "reserved" doc comment in `config/agent.rs`.
7. Record each decision as a bounded audit event (`DataLlmAuditEntry`: hashes, taint, model, validity; never raw
   content). Persist it through `gap-ff95f5` if that has landed.
8. Tests with a fake backend, no live API: untrusted result routed and validated; raw text absent from the main
   model's next request; schema rejection, timeout and cancellation block the content; local tool results pass
   through; the Data LLM request has no tools.

## Done when

- With `[agent.data_llm]` set, an MCP, plugin, web-search or network tool result in a roko-owned tool loop
  reaches the main model only as validated Data-LLM output. On any Data-LLM failure the main model gets a withheld
  notice, never the raw text.
- The Data-LLM request has no tools and no main system prompt.
- Local builtin results and runs with `data_llm` unset behave as today.
- A Graph run no longer reports `agent.data_llm` as unread.
- Verify:
  `! grep -q '"agent.data_llm", NO_READER' crates/roko-cli/src/graph_task_dispatch.rs && grep -rqw 'fn untrusted_tool_result_never_reaches_main_model_raw' crates/roko-agent/src/ && cargo test -p roko-agent untrusted_tool_result_never_reaches_main_model_raw`

## Notes

- Safety-sensitive, and on the hot path of every API-provider tool loop. The feature must be opt-in (`None` =
  disabled). With it on, a failure must fail closed.
- It cannot cover CLI-subprocess providers; say so in docs and in the config comment.
- Cost and latency: every untrusted tool result adds a model call. Keep the data model small (default
  `MODEL_FAST`) and bound the input size.
- The old verify command only grepped for any `DataLlmRouter::` or `sanitize_input(` reference in a few
  directories. A trivial call would have passed without any boundary. The command above requires the config to
  be read and a behaviour test.
- Touches `crates/roko-agent/src/tool_loop/mod.rs`, which many providers share. Do not run in parallel with other
  tool-loop work.
- 2026-10-01 (wk-childenv): Plan step 1 (config) on work/gap-1555ac; cargo verification deferred to the batch
  check. `[agent.data_llm]` is back as `AgentConfig::data_llm: Option<DataLlmConfig>` (`None`, the default, turns
  it off), and `DataLlmConfig` gains `timeout_ms` (30 s) and `max_input_bytes` (32 KiB). gap-7a3527 had removed the
  key, so its `REMOVED_CONFIG_KEYS` entry is gone and its schema sentinel is back in `build_schema_tree`. Loading
  fails (invariant 8) when `strip_tool_calls` is false or either bound is 0, and `validate_references` warns when
  `model` is neither a `[models.*]` key nor a builtin. Until step 6 builds the boundary from config,
  `graph_engine_inert_settings` reports the key, and the config docs say it protects nothing yet. Tests:
  `validate_invariants_rejects_a_data_llm_with_tools_or_no_bounds`, `validate_references_warns_on_unknown_data_llm_model`,
  the data_llm rows of `every_optional_config_key_survives_a_load` and `inert_settings_list_only_changed_keys_the_graph_engine_ignores`.
- 2026-10-01 (wk-childenv): Plan step 2 (data-only caller) on work/gap-1555ac; cargo verification deferred to the
  batch check. `DataLlmBoundary { router, backend }` in `safety/data_llm.rs` (re-exported from `safety`) is built
  with `DataLlmBoundary::new(config, backend)`, which refuses `strip_tool_calls = false` or a zero bound. `process`
  sanitizes, cuts the text to `max_input_bytes` at a character boundary, and sends only the fixed
  `DATA_LLM_SYSTEM_PROMPT` (plus the configured output schema) and that text, with an empty tool list, through
  `stream_turn` under `timeout_ms`, `max_tokens` and `temperature`. It never dispatches a tool call. A timeout, a
  backend error, a response that asks for a tool, or output that fails `validate_output` is a `DataLlmWithheld`
  that names no content. Building the backend from the provider factory belongs to step 6. Tests:
  `data_llm_boundary_sends_only_the_fixed_prompt_and_the_text`, `data_llm_boundary_withholds_what_it_cannot_validate`,
  `data_llm_boundary_bounds_its_input`, `data_llm_boundary_refuses_tools_and_unbounded_calls`.
- 2026-10-01 (wk-childenv): Plan step 3 (tool loop) on work/gap-1555ac; cargo verification deferred to the batch
  check. `ToolLoop::with_data_llm(Arc<DataLlmBoundary>)` (default: none) makes `run_inner` pass each
  `dispatch_batch` result through `DataLlmBoundary::screen_result` before previews and `render_results`, by the
  taint `tool_source_taint` gives the tool's registry source (MCP and plugin: third party; web search, retrieval
  and network builtins: external fetch; other builtins: none). A routed result reaches the model only as
  `[untrusted output (<reason>), read by the data model: ...]` plus the validated JSON, or as the tool error
  `untrusted content withheld: <why>`. Images and artifacts are withheld, and a `ToolError::Other` message is
  screened like text. Tests: `a_routed_tool_result_reaches_the_model_only_as_data_output`,
  `a_withheld_tool_result_never_reaches_the_model_raw` (tool_loop/mod.rs).
- Left (steps 4-8): the narrow output type (step 4; validation is still JSON plus the configured `required` keys);
  a typed withheld error instead of `ToolError::Other` (step 5); building the boundary from `config.agent.data_llm`
  through the provider factory for every roko-owned loop, including ACP's, and dropping the inert entry (step 6);
  audit records (step 7); cancellation coverage (step 8). The verify now greps `inert_settings.rs`, where the inert
  list moved. Its test, `untrusted_tool_result_never_reaches_main_model_raw`, is left for step 6, so that it covers
  the boundary built from config; the step-3 tests use other names, so the verify does not pass early.

## Original notes

The `data_llm.rs` module contains a `DataLlmDecision::SanitizationRequired` routing decision, but no code actually enforces routing through a separate sanitizing LLM. The decision is produced but callers are not required to act on it; there is no production code path that actually routes through...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F019`
- `tmp/backlog/archive/352-data-llm-tainted-content-routing.md#352 — Wire the CaMeL Data-LLM Boundary for Tainted Content`

How to verify: E34 claimed complete; check CaMeL Data-LLM boundary enforcement. Confirm in crates/roko-agent/src/safety/data_llm.rs whether still true: CaMeL Data-LLM execution boundary not enforced / Check: When enabled, External/Untrusted content never enters the main-model request before successful Data-LLM schema validation.; The Data LLM has no tool/MCP/plugin capability and cannot access workspace secrets.; Failure/timeout/malformed output… [evidence: own status: Blocked on shared runtime services]

Merged 2 mined candidates: m2-010, m1-093.

Verified 2026-09-28: still true. `DataLlmRouter`, `DataLlmDecision::SanitizationRequired` and the rest of the Data-LLM API have no callers outside crates/roko-agent/src/safety/data_llm.rs; they are only re-exported (safety/mod.rs:86-87). No dispatch path routes External/Untrusted content through a separate Data-LLM. The cited config file is crates/roko-core/src/config/agent.rs (see :77).
