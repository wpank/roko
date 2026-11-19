+++
id = "bug-5b04d3"
kind = "bug"
title = "A helper call that names no tools (error enrichment, gate reflections, the judge via CheapFactoryAgent; the distiller and chat via ModelCallService) is sent the whole local tool catalog: 16 schemas, 2,316 tokens against a 79-token prompt (97% of the request), about 2.25 helper calls a run plus one distillation per plan, ~$0.004-0.012 a run"
status = "open"
triage = "verified"
severity = "p3"
size = "M"
subsystem = ["roko-cli/graph-dispatch", "roko-agent/model-call"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round v7, roko-effort's survey of which calls bug-29014c reaches (2026-10-26, $0 capture), the lead's ask"
discovered_from = "agent:roko-effort (bug-29014c, which calls are tool-less)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/routing_context.rs::CheapFactoryAgent", "crates/roko-agent/src/model_call_service.rs::build_agent_options", "crates/roko-agent/src/provider/openai_compat.rs::tool_registry_for_options", "crates/roko-cli/src/dispatch_v2.rs::agent_options"]
links = { depends_on = [], blocks = [], related = ["bug-29014c", "gap-1ba0ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_helper_that_names_no_tools_sends_none' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_helper_that_names_no_tools_sends_none"
+++

## Problem

roko's one-shot helper calls name no tools, and "no tools named" means the whole local tool catalog. So every helper
request carries 16 tool schemas that the helper never needs.

- **The helpers:** error enrichment, gate reflections and the judge all go through `CheapFactoryAgent`, whose
  `AgentDispatchRequest` has `tools: None` and `agent_contract: None`. The episode distiller and the chat go through
  `ModelCallService`, which sets no tools either.
- **Why none means all:** `tool_registry_for_options` reads no allowlist as unrestricted. An empty CSV is no
  allowlist too: `parse_allowed_tools_csv(Some(""))` is `None`. Only a contract's `allowed_tools: Some([])` keeps an
  empty set.
- **Measured** on 2026-10-26, at $0: roko-effort-e4bd6ad74 with Undertow's roko.toml, a stand-in upstream that saves
  each body, a failing verify step and one retry. The error-enrichment helper's request on gpt-oss-120b carried:
  - 16 tools: read_file, write_file, edit_file, multi_edit, glob, grep, bash, ls, web_fetch, web_search,
    notebook_edit, todo_write, task, exit_plan_mode, apply_patch and run_tests;
  - 10,417 bytes, **2,316 tokens** (o200k, the tools' raw JSON), against 79 tokens of messages ("Diagnose this
    compilation/test error in 2 sentences…"). That is 97% of the request.
  - `max_tokens: 32768` for a two-sentence diagnosis.
- **OpenAI's count:** it counts tool schemas at about 0.82 of that raw figure. An implementer request with 14 tools
  measured 3,325 raw tokens, and OpenAI reported 2,724 prompt tokens. So the catalog is about 1,900 prompt tokens on
  OpenAI.
- **Records:** every helper and distiller row in `.roko/learn/efficiency.jsonl` says `tools_available: 0`, though
  the request carried 16.

## Why it matters

The dollars are small; the waste per request is not.

- **Calls per run** (Undertow's efficiency ledger, phases 0–4, 12 plan runs with tries):
  - 27 helper calls, in 7 runs: 8, 6, 5, 3, 3, 1, 1. That is 2.25 a run on average and 3.9 a run with failing tries.
    24 ran on gpt-oss-120b and 3 on gpt-5.4-mini.
  - The ones that went through recorded 300–1,063 uncached plus 1,280–3,456 cached input tokens.
  - Plus one episode distillation per plan written: 16 calls, each 12,373–12,875 input tokens for 10–14 output
    tokens, on gpt-5.4-mini (14) and gpt-5.4 (2), $0.2007 in all. By the same code path these carry the catalog too;
    no distiller body was captured.
- **Cost per run**, at 2,300 catalog tokens a call:
  - per call: gpt-oss-120b ($0.35/M; Cerebras bills cached input at that rate too) $0.0008, gpt-5.4-mini ($0.75/M)
    $0.0017, gpt-5.4 ($2.50/M) $0.0058;
  - a mean run (2.25 helper calls plus one distillation): about $0.004–0.008, roughly 0.5% of a $0.9–3 run;
  - the worst run (8 calls plus a gpt-5.4 distillation): about $0.012, and 20,000+ catalog tokens sent for nothing.
- **Latency and turns:** the catalog adds prefill time to the helper calls a retry waits for. A helper that does call
  a tool burns a turn on it: `CheapFactoryAgent` has no contract, so its tool loop runs under the default safety
  layer, which denies every tool.
- **bug-29014c:** a request without function tools is the only kind that can carry `reasoning_effort` on OpenAI's
  chat completions. Sending the helpers none would let them reason on a GPT-5.x model too.

## Where

- `crates/roko-cli/src/graph_task_dispatch/routing_context.rs::CheapFactoryAgent`: the helpers' request
  (`tools: None`, no contract).
- `crates/roko-agent/src/model_call_service.rs::build_agent_options`: `ModelCallService`'s options, which set no
  tools. This is the distiller's path (`learning_helpers.rs::distillation_model_caller_on`) and the chat's.
- `crates/roko-agent/src/provider/openai_compat.rs::tool_registry_for_options`: no allowlist means every local tool.
- `crates/roko-cli/src/dispatch_v2.rs::agent_options`: an `AgentDispatchRequest` becomes the agent's options here.

## Plan

1. A helper that wants no tools says so. `CheapFactoryAgent` gives its request an empty allowlist: a contract with
   `allowed_tools: Some([])`, or a dedicated no-tools option, since an empty CSV means unrestricted. The distiller's
   `ModelCallService` call does the same.
2. With no tools left, the backend omits `tools`: `build_body` already drops an empty array. bug-29014c's gate then
   lets these calls carry `reasoning_effort` on a GPT-5.x model.
3. Cap the helpers' output. A two-sentence diagnosis needs a few hundred tokens, not 32,768.
4. Record `tools_available` as what the request carried.

## Done when

A test captures a helper's request body and a distillation's, and finds no `tools` in either. A $0 capture like the
one above shows the error-enrichment request at about 100 tokens. The `[[verify]]` test passes.

## Notes

- Found while checking which calls bug-29014c's narrowed fix reaches. In Undertow's setup none of the named helpers
  is tool-less, and this is why.
- The measurement's records are in `tmp/hermes-roko-v7/roko-effort/helper-catalog/`:
  `helper-error-enrichment-request.json` (the helper's body), `implementer-turn-request.json`, the scratch config and
  the stand-in upstream (`capture_up2.py`).
