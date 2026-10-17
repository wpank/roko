+++
id = "gap-bccf58"
kind = "gap"
title = "The metering proxy forwards any model and server-side web tool, so an agent shell in the Roko and mini-swe-agent arms can have the provider search the web"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "4e22639c3"
source = "gap-0bd49a worker (claude-agent), 2026-10-16"
discovered_from = "gap-0bd49a: the network sandbox admits the metering proxy's port on purpose"
anchors = ["benchmarks/viabilitybench/driver/faultproxy.py::_serve", "benchmarks/viabilitybench/driver/vb.py::_start_proxy"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-0bd49a", "gap-f253cf", "bug-d58954"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_proxy_refuses_server_side_web_tools' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_the_proxy_refuses_server_side_web_tools -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T03:21:22Z"
commit = "4e22639c3"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-17T02:10:41Z"
forced = false
evidence = "4e22639c3: faultproxy.py's policy, checked before the caps, answers 403 before forwarding to: any path but /chat/completions, or a query string (vb_path_not_allowed: z.ai's /web_search and /reader, Moonshot's /tools/search and /tools/fetch, OpenAI's /responses); a model outside the run's (vb_model_not_allowed: OpenAI's search models, OpenRouter's :online); and a server-side tool or search option (vb_server_tool: a tools entry whose type is not function, a $-prefixed built-in such as Moonshot's $web_search, web_search_options, plugins). The names were checked against each provider's docs on 2026-10-17. Each refusal is logged under the task's key (refused, server_tools) and counted in the meter's policy_refused. vb.py gives the proxy the arm's models_allow plus any declared model_swap target, and per upstream the arm file's [providers.<name>] proxy_allow (none is set). The manifest's proxy.policy and each record's provenance.proxy_policy record the policy, the record with the task's refusals by kind, and the census flags such a task at place web (proxy:<refusal>), so the run is leak_suspected. The [[verify]] passes: the stub upstream, which records every request on any path, sees none of the refused requests, while a plain request and an allowed tool pass, and the policy comes before the caps. test_an_agent_cannot_have_the_provider_search_the_web runs vb run --proxy on roko_fixed and cheap_direct_msa with a fake harness whose shell asks through the proxy for z.ai's web_search tool, gpt-5-search-api and Moonshot's /tools/fetch. All three get 403, the provider sees only the harness's own call, and the record is leak_suspected with proxy_policy.refused at 1 of each. With 200ced75c's driver code the same tests fail: 200, 200, and /tools/fetch forwarded upstream (the stub's 404). With VB_TEST_ROKO_BIN (roko at 3e05c6f06) the real-Roko driver tests pass 47/47, and their 110 proxied requests (gpt-oss-120b, glm-4.7, gpt-5.4-mini) were all /chat/completions with no server tool and none refused. benchmarks/viabilitybench: 786 passed, 16 skipped (no roko binary in the worktree, no analysis stack, opt-in sweeps), against 782 passed and the same 16 skipped before. Filed bug-d58954."
+++

## Problem

The metering proxy forwards any request body it is given to the upstream's chat-completions endpoint. It meters the
call and checks the input-token caps, but it doesn't check the model or the tools a request names.

In the Roko and mini-swe-agent arms, the agent's shell runs inside a process tree whose network rule admits the proxy's
loopback port (gap-0bd49a: `run_roko.network_rule`, `run_msa._network_rule`). That shell can POST its own request to
the proxy and ask the provider to search or fetch the web on its behalf. Three providers can do this:

- z.ai (GLM): a `tools` entry of type `web_search`;
- Moonshot (Kimi): the built-in function `$web_search`;
- OpenAI: a search model, or `web_search_options`, on chat completions.

The public repository holds `families/*/hidden.py` and the plan-slice suites. A search that returns one of them
contaminates the run, just as the shell `curl` that gap-0bd49a closed would have.

Expected: the proxy forwards only what an arm's harness sends, which is plain chat completions with the arm's own
models and function tools. It refuses anything else before forwarding, as it already refuses unbounded input
(`vb_unbounded_input`).

## Why it matters

This is the pilot benchmark's validity (epic spec-567e52), and the same contamination risk as gap-0bd49a. The census
counts web requests only when they show in the transcript (`census._web_requests`). A provider-side search made by an
agent's own `curl` to the proxy may never appear there. The arms involved are roko_ladder, roko_full and roko_plan
(z.ai and OpenAI rungs), roko_fixed_glm and roko_fixed_kimi, and any mini-swe-agent arm pointed at those providers.

## Where

- `benchmarks/viabilitybench/driver/faultproxy.py::FaultProxy._serve`: parses the request (`_json_object`), checks
  the caps, then forwards it.
- `benchmarks/viabilitybench/driver/vb.py::_start_proxy`: builds the proxy from the plan's endpoints. The arm's
  `models_allow` is known there.

## Current state

At 47b3cdd20 plus gap-0bd49a's change, `_serve` refuses only on the caps and unbounded media. `model_requested` is
logged but never checked. Nothing looks at `tools`, `web_search_options` or provider-specific search fields.

## Plan

1. Give the proxy the arm's allowed models (`models_allow`, plus a declared `model_swap`). Refuse a request whose
   `model` is outside that set with a 403 (`vb_model_not_allowed`), logged with the task's key.
2. Refuse server-side tools with a 403 (`vb_server_tool`): any `tools` entry whose `type` isn't `function`,
   `web_search_options`, a `$`-prefixed built-in function, and OpenRouter-style `plugins`. Each upstream may keep an
   allowlist if a harness legitimately needs a field.
3. Record the refusals in the proxy row (`refused`). Have the census flag a task with such a refusal (place `web`),
   since only an agent would send one.
4. Add `test_the_proxy_refuses_server_side_web_tools` in `driver/test_faultproxy.py`. A stub upstream sees none of the
   refused requests, and a plain request still passes.

## Done when

- [x] A request that names a model outside the arm's set, or a server-side search or fetch tool, never reaches the
  upstream. Its refusal is logged with the task's key.
- [x] The `[[verify]]` command passes.

## Notes

- Found while closing gap-0bd49a. The network sandbox there admits the proxy's port on purpose, since Roko and
  mini-swe-agent call the model themselves. So the proxy is the place to hold the provider path to plain completions.
- Check each provider's current parameter names against its documentation before writing the filter. The names
  above are from memory, not checked live.
- 2026-10-17, checked against the providers' documentation:
  - z.ai: `tools` types `function`, `web_search` and `retrieval`, plus standalone `POST /paas/v4/web_search` and
    `/paas/v4/reader` endpoints.
  - Moonshot (now platform.kimi.ai): `{"type": "builtin_function", "function": {"name": "$web_search"}}`, with `$`
    marking every built-in, plus standalone `POST /v1/tools/search`, `/v1/tools/search_pro` and `/v1/tools/fetch`.
  - OpenAI: chat completions searches only with a search model (`gpt-5-search-api`, `gpt-4o(-mini)-search-preview`),
    which searches even without `web_search_options`; the Responses API has a `web_search` tool.
  - OpenRouter: `plugins: [{"id": "web"}]`, `:online` model slugs, `web_search_options` and `openrouter:web_search`.
  - Cerebras documents no server-side tool.

  Because of the standalone endpoints, the proxy also refuses every path but `/chat/completions`. A body filter
  alone would have left `/tools/fetch` open.
