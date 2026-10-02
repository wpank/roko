# R4: Root causes of the live-run defects (code at a43288b5f)

Static reading of the code at `a43288b5f` (the frozen binary `roko-a43288b5f`), checked against the live-run evidence. No builds, runs or model calls. `$L` is `<session-scratchpad>/runtime/live`.

**Line numbers.** `crates/roko-agent/`, `crates/roko-graph/src/cells/`, `roko-learn/src/{provider_health,model_call_feedback}.rs` and `roko-cli/src/graph_task_dispatch/{attempt,budget,ladder}.rs` are identical at `a43288b5f` and the working tree (`2f82da96a`), so their line numbers hold for both. `dispatch_v2.rs`, `graph_task_dispatch.rs`, `graph_task_dispatch/failover.rs` and `graph_execution/plan_runner.rs` have changed since then. Their anchors come from `git show a43288b5f:<path>` and are marked **(a43)**.

**Confidence.** *Confirmed*: the code path and the log agree. *Inferred*: the only code path that fits the evidence, though nothing logged it. *Open*: settling it needs a capture.

## TL;DR

- **GLM blank result (roko bug, triggered by a provider quirk).** The OpenAI-compatible SSE parser turns each chunk into one event, in a fixed priority: `reasoning_content`, then `content`, `tool_calls` (first element only), `usage`, `finish_reason`. The rest of the chunk is dropped. When a stream ends without a finish reason, `stream_turn` makes one up as `stop`. The tool loop then returns an empty answer with `stop` as a normal success. So zai's usage and real finish reason are always lost. Which GLM payload was lost (a tool call, a `length`/`sensitive` finish, or reasoning only) stays *open* until someone captures the raw stream.
- **One blank answer blocks the task for good (roko bug).** The immune boundary rates a blank answer High and writes a durable isolation control. The control is keyed by `"{plan_id}/{task_id}"`, which every attempt, provider, model and resumed run of that task shares. Its decay is `none`, and nothing (CLI, API or decay) ever releases it. After 512 controls the ledger fails closed for *every* new agent. Each later attempt is denied in preflight about 1 ms after creation. The denial is a failed `AgentResult`, so **two** recorders count it as a provider failure (`Unknown`): `dispatch_v2::record_provider_outcome`, and a second loader, `record_provider_health_at`, that hard-codes `Unknown`. That opens zai's circuit and then the substitute cerebras's. `TaskExecutorCell` retries at once, with no backoff (about 130 ms apart).
- **r2's non-blank denial was the stream cap (inferred by elimination).** The cap is 4,096 events / 4 MiB, and it counts the **whole 23-turn tool loop**, tool results included. Taint plays no part. This path writes no receipt, vault entry or control, and logs nothing. Its reason, `provider_stream_limit_exceeded`, exists only as a tag on the output that gets discarded.
- **Concurrency is set by the plan budget, not the provider.** When `max_turn_usd` is unset (0.0), every call reserves the plan's whole remaining budget, so calls run one at a time and the wait is silent. Smoke: `max_plan_usd=5`, no `max_turn_usd`, so serial. Live: `max_plan_usd=2.4` with `max_turn_usd=0.5`, so exactly 4 in flight, which is the 4 overlapping tasks. C6 with `--no-budget` has no reservation, so it overlaps. The provider difference was a confound.
- **Accounting (roko bugs, plus one harness artefact).** No request ever sets `stream_options.include_usage`, so OpenAI never streams usage and its spend reaches the budget as $0. Cerebras sends usage by default. `AttemptRun::settle` fills only `cost.source`, never the token or cost amounts. The ladder counts a substitute model's failure against the rung that was routed. `dispatch_v2::classify_provider_error` is a narrower copy of `error_classify` with no auth branch, so "Not logged in" (caused by the harness's missing `USER`) is classed `Unknown`, retried, opens the circuit and fails over.

## 1. GLM-4.7 returns a blank agent result

**Evidence.** `$L/evidence/r1-live-a/stderr.clean.txt`: at 08:32:16.706 the tool loop logs `iterations=0 final_text_len=0 finish_reason=Some("stop") input_tokens=0 output_tokens=0`, 8.5 s after `creating agent ... provider=openai_compat base_url=Some("https://api.z.ai/api/paas/v4")` (`costs.jsonl` `duration_ms=8589`). All 18 `zai` rows in `repo/.roko/learn/costs.jsonl` have 0 tokens and `cost_source=unknown`. The other 17 rows are preflight denials (section 2).

**Mechanism.**
1. The request is built in `crates/roko-agent/src/openai_compat_backend.rs:411-449`: `model`, `messages`, `tools`, optional `max_tokens`, `"stream": true` (line 447) and `extra_body_params`. There is no `stream_options`, and no thinking control unless `extra_body` sets one.
2. `crates/roko-agent/src/streaming.rs:243-326` (`parse_sse_chunk`) returns **one** event per chunk, tested in this order: `delta.reasoning_content` (247), `delta.content` (252), `delta.tool_calls` (257), `usage` (312), `finish_reason` (315). Inside the tool-call loop, `return` (lines 299 and 306) keeps only the first element of a multi-call array. A chunk that carries `content: ""` together with `finish_reason` and `usage` therefore becomes `TextDelta("")`, and its usage and finish reason are lost. Z.ai documents its final chunk in that shape, but this run doesn't prove it. A chunk that carries a `content` or `reasoning_content` string next to `tool_calls` loses the tool call. An SSE error object (`data: {"error":...}`) has no `choices`, so it yields `None` and is silently ignored.
3. `[DONE]` maps to `finish_reason: "stop"` (`streaming.rs:218-221`), and `stream_turn` emits a made-up `Done{finish_reason:"stop"}` whenever the stream ends without one (`openai_compat_backend.rs:823-832`). So `finish_reason=Some("stop")` in the log does not mean the model said `stop`: a `length`, `sensitive` or `tool_calls` finish would look the same.
4. `crates/roko-agent/src/tool_loop/mod.rs:1366-1408`: with no tool calls, an empty `final_text` and a finish reason other than `length`, the loop logs the "final text is empty" warning (1386-1390) and returns `StopReason::Stop`, i.e. a normal result with a blank body. Only `length` becomes a `BackendError`. The blank result then reaches the immune boundary (section 2).
5. The 8.5 s means GLM generated tokens, most likely reasoning first. `roko config providers test zai` (200, `content_empty`, 10 output tokens) fits a model that spends its first tokens in `reasoning_content`. *Open:* whether the lost payload was (a) a tool call sharing a chunk with a `content`/`reasoning_content` string, (b) a `length` or content-filter finish disguised as `stop`, or (c) reasoning with no answer. Usage is lost in every case.

Side observation: `events-by-run/*.jsonl` labels the glm-4.7 attempt's `agent_spawned` event `provider: "codex_cli"`. The pre-dispatch label is derived from `dispatch_plan.model.backend` (`graph_task_dispatch.rs:1391-1400` (a43)). This only affects display.

**Classification.** A roko bug: the single-event parser, the made-up `stop`, and an empty answer accepted as a success. A provider quirk triggers it: GLM streams reasoning first and packs several fields into one chunk.

**Existing item.** None. `items-leads.md` has no hits for glm, zai, reasoning_content or blank, and `work/items/` has none either.

**Fix.** Make `parse_sse_chunk` return every event in a chunk: reasoning, content, each tool call, usage and finish reason. Turn an `error` object into an `LlmError`. In the tool loop, treat "no tool calls and empty text" as a provider error (`empty_response`, retryable, may fail over), not as `Stop`.

**Verify.** Add `streaming.rs` unit tests: (1) a Z.ai-style final chunk `{"choices":[{"delta":{"role":"assistant","content":""},"finish_reason":"tool_calls"}],"usage":{...}}` must yield Usage and Done(ToolCalls); (2) a chunk with both `reasoning_content` and `tool_calls` must yield both; (3) a two-element `tool_calls` array must yield two starts. Add a fake openai_compat server test that streams reasoning only and asserts the attempt fails as `empty_response` with no immune control written. Live: capture one raw glm-4.7 SSE stream (raw-line trace) and keep it as a replay fixture.

## 2. One blank result blocks the task for good and poisons provider health

**Mechanism.**
1. **Identity.** Graph dispatch sets `agent_id = format!("{}/{}", spec.plan_id, ctx.cell_id.unwrap_or(&task.id))` (`crates/roko-cli/src/graph_task_dispatch.rs:1351-1355` (a43)), giving `live-a/cli`. The factory wraps every provider agent in `wrap_provider_agent` (`crates/roko-agent/src/immune_boundary.rs:901`) under that id. The id is therefore shared by every attempt, provider, model and resumed run of the task.
2. **Screening.** `ImmuneScreenedAgent::run` (`immune_boundary.rs:776-794`) calls `screen_result` (571-627). Inside it, `detect_provider_output_evidence_anomaly` scores blank text as `blank_primary_output_text` 0.9 (151-153). Any decision other than Accept goes to `persist_containment` (478-569). For High or Critical severity (487-489) it writes the isolation control **before** any other evidence (495-498), via `persist_agent_control` (`crates/roko-agent/src/immune_evidence.rs:315-342`). The control is built by `isolation_marker_for` (`immune_boundary.rs:630-647`): `state: "isolated"`, `reason: provider_output_immune_containment`, and the Signal's default decay `none` (as `$L/evidence/agent-controls.after-r1c.json` shows). The vault entry is marked `escalated`.
3. **Denial on every later attempt.** `run()` and `run_streaming()` call `preflight` first (`immune_boundary.rs:458-476`). That calls `is_isolated` (328-334), then `get_agent_control` (`immune_evidence.rs:344-365`), then `denied_result(.., "agent_isolated", ..)` (`immune_boundary.rs:336-369`), which is an `AgentResult::fail` returned without calling the provider. That is why the denial lands about 1 ms after `creating agent` (r1: 08:32:16.880073 to 08:32:16.881420).
4. **Nothing releases it.** `immune_evidence.rs` has only `persist_agent_control` and `get_agent_control`. Persisting rejects a conflicting control (323-329), and no CLI command or serve route reads or writes `agent-controls.json` (all references are inside `roko-agent`). With decay `none`, it never expires. Worse, once the ledger holds `MAX_AGENT_CONTROLS = 512` entries (`immune_evidence.rs:25`), `get_agent_control` fails closed for **every unknown agent** (356-360), so every new task in the workspace is denied with `isolation_state_unavailable`. The only way out today is to move the file by hand, as the harness did (`agent-controls.moved.json`).
5. **A denial counts as a provider failure, twice.**
   - Recorder 1: `AgentDispatcherV2::record_provider_outcome` (`crates/roko-cli/src/dispatch_v2.rs:1906-1927` (a43)), called at 1668 and 1870 (a43). It counts every unsuccessful result that is not a turn cap or an attempt timeout as a provider failure, classified by `classify_provider_error` (2069-2098 (a43)). That function finds nothing it knows in "provider result denied by immune boundary", so it returns `unknown`, which becomes `ErrorClass::Unknown` (`crates/roko-learn/src/provider_health.rs:1294-1308`).
   - Recorder 2: `record_agent_dispatch_feedback` (`dispatch_v2.rs:2108` (a43), called at 1670, 1775 and 1872) goes through `ModelCallFeedbackRecorder` to `record_provider_health_at` (`crates/roko-learn/src/model_call_feedback.rs:240-256`). That function loads a **fresh** registry from `provider-health.json`, records the failure with `ErrorClass::Unknown` hard-coded (252), and saves it synchronously. It keys on `provider_success.unwrap_or(success)` (74-76), so it ignores the turn-cap and timeout exemptions. That is why r1 line 166 records a cerebras failure for a turn-cap stop that recorder 1 counts as a success. The two registries explain why each attempt logs two `provider failure recorded` lines and why the circuit opens twice (r1 lines 296 and 298, `consecutive_failures=3` each). The persisted file is reloaded by the next run (`plan_runner.rs:1231-1248` (a43)), so the poisoned health carries into resumes.
   - Effect: the denial is a decision of the host's own policy, yet it is charged to whichever provider the attempt was routed to. The zai circuit opened after 3 attempts. Failover then substituted cerebras, which was denied as well (same `agent_id`), and its circuit opened too (r1 lines 307-336). That hurt the other tasks in the run.
6. **No backoff, no stop for permanent errors.** The live retry loop in `crates/roko-graph/src/cells/task_executor.rs:826-875` retries every error except `Gateway{retryable:false}`, `Rejected` and `Cancelled`, with no sleep and no jitter. The denial is a retryable `RokoError::Agent`, and the ladder had raised `max_retries` to 5. Attempts went out at 16.79, 16.93, 17.06, 17.18 and 17.40 s, so all 5 retries were used up in about 0.6 s.

**r2: a finished gpt-5.4-mini result denied with no vault or control entry (inferred).** In `$L/evidence/r2-live-b/stderr.clean.txt` lines 1546-1549, the tool loop returns `iterations=23 final_text_len=459` at 08:40:45.127271, and the provider failure follows 0.3 ms later. **No `security.immune.*` Graph node runs in between**, though that Graph runs on every screened result (316 `immune-perception` lines in this run), and no error is logged. The deny paths compare as follows:
- `invalid_agent_identity` and `agent_isolated`: preflight, before the call. Rules them out.
- `immune_graph_failed`, `immune_stage_order_invalid` and `containment_persistence_failed`: each logs `tracing::error!`. None was logged.
- A non-Accept decision: runs the Graph and writes a vault entry. Neither happened.
- **`provider_stream_limit_exceeded`** (`immune_boundary.rs:780-790` and `830-840`): skips `screen_result` and logs nothing. **This is the only path that fits.**

`drive_streaming_inner` (395-456) counts every stream event of the whole wrapped agent run, which here was a 23-turn tool loop. Every text, reasoning and tool-argument delta counts, and so does each `ToolResult` output (`stream_event_bytes`, 883-894). The limits are `MAX_PROVIDER_STREAM_CHUNKS = 4_096` and `MAX_PROVIDER_STREAM_BYTES = 4 MiB` (55-56). A token-streamed 23-turn session goes past 4,096 events. Taint is not involved: the `taint propagated ... resulting_level=external` lines come from `safety/taint_propagation.rs:229` and appear throughout the run, and the provider anomaly detector (136-179) never reads taint. The reason is not logged anywhere. It exists only as the `immune_reason` tag on the denied output signal (349-350), which the dispatcher reduces to the generic text.

**Classification.** Roko bugs, all of them:
- A quality failure (a blank answer) is treated as a security event.
- The isolation key is too broad, and there is no release.
- A denial counts as a provider failure, through a duplicate recorder.
- Retries have no backoff.
- The stream cap applies to the wrong scope and denies silently.

**Existing items.** None for the isolation key, release, denial accounting, the duplicate recorder, backoff or the stream cap. `gap-2f69e9` (open, p3: the quarantine vault caps at 50 entries and has no review surface) is adjacent.

**Fixes.**
- Score blank or empty output as a provider error before the boundary (section 1), or at least quarantine it without isolating.
- Key isolation by attempt and provider (or give controls a TTL), and add a release path: `roko safety release <agent-id>` plus a serve route, both audited.
- Return a typed immune-denial error that `record_provider_outcome` skips and `TaskExecutorCell` does not retry.
- Delete `record_provider_health_at` from the bridge feedback path, leaving one registry.
- Add exponential backoff with jitter to `TaskExecutorCell`.
- Reset the stream counters for each model call (or leave `ToolResult` out of them), and log `immune_reason` at warn on every denial.

**Verify.**
- `immune_boundary` test: a fake inner agent returns blank text, then a second `run` with the same id must not be denied (and no control may be written once blank output is a provider error).
- A fake stream of 5,000 one-token deltas across 3 turns must be accepted, and a forced denial must log its reason.
- A `dispatch_v2` test feeding in a denied result must leave the registry with no failure and `provider-health.json` with at most one record per attempt.
- A `task_executor` test must see the retry gaps grow.

## 3. Concurrency differs by provider

**Mechanism (confirmed).** The limiter is the plan budget reservation: not a per-provider cap, not `claude_cli`, not `[conductor] max_agents`.
- `GraphPlanBudgetPolicy::from_limits` (`crates/roko-cli/src/graph_task_dispatch/budget.rs:31-50`): when `max_turn_usd` is 0 or unset, `reservation_micro_usd = ceiling`, so each provider call reserves the whole remaining plan budget. The doc comment says this is deliberate: "only one unknown-cost call can be in flight at a time" (bug-0bc2b4).
- `reserve_waiting` (`budget.rs:218-241`) keeps waiting until capacity frees up, and logs nothing while it waits.
- Smoke (`smoke/repo/roko.toml`): `[budget] max_plan_usd = 5.0` and no `max_turn_usd`. The default is 0.0, which `crates/roko-core/src/config/budget.rs:5` documents as "unlimited". So one call at a time, despite `width=5`.
- Live (`live/repo/roko.toml:141-145`): `max_plan_usd = 2.4` and `max_turn_usd = 0.5`. That gives a 0.5 reservation and ⌊2.4/0.5⌋ = **4** calls in flight, matching the 4 overlapping tasks.
- C6 runs with `--no-budget`: there is no ceiling, so `reservation_micro_usd` is `None` (`budget.rs:32-34`), nothing is reserved and tasks overlap. The mapping from flag to policy was not traced.

**Ruled out.**
- `[conductor] max_agents`: unset (default 8), and no `waiting for an agent slot` line appears.
- `max_parallel`: omitted in every plan, smoke and live. Both runs logged `width=5`, and the live tasks overlapped. That rules out R2's static lead (`converted_max_parallel` = 1).
- Per-provider `max_concurrent` (bug-eba31d): not configured.
- The startup warning "estimated parallel-agent cost (40.00 USD) exceeds budget.max_plan_usd" is computed from `max_agents` and has nothing to do with the reservation.

**Classification.** A roko design bug: the documented "0.0 = unlimited" turns into "reserve everything" and quietly serialises the plan. The harness added a confound, because the provider type changed together with the budget config.

**Existing item.** `bug-0bc2b4` (closed 2026-10-01) introduced this behaviour on purpose. R2's suggested item 1 ("Plan tasks never run concurrently despite width") should be re-anchored here.

**Fix.** With no `max_turn_usd`, reserve an estimate (model price × expected tokens, or `max_plan_usd / width`), not the whole ceiling. Log "waiting for budget reservation (in flight N)" when a call waits. Make the `width=` log report the width the budget actually allows.

**Verify.** Add a variant of `crates/roko-cli/tests/scheduler_canary.rs` C6 with `max_plan_usd = 5` and no `max_turn_usd`. Today it shows no overlap; after the fix its two disjoint 3 s tasks must overlap.

## 4. Accounting gaps

### 4a. OpenAI usage is zero, so the budget never sees OpenAI spend
**Mechanism (confirmed).**
- `git grep` at `a43288b5f` finds no `include_usage` or `stream_options` anywhere in `crates/`. The streaming request only sets `"stream": true` (`openai_compat_backend.rs:446-448`).
- OpenAI streams usage only when `stream_options.include_usage` is true. So the stream has no `usage` chunk and the tool loop logs 0/0 (r2 line 1546).
- Then `usage_obs` is `None`, `cost_source` is `Unknown` (`crates/roko-cli/src/graph_task_dispatch/attempt.rs:702-710`), and `costs.jsonl` shows `cost_usd = 0.0` on all 6 openai rows.
- Cerebras puts `usage` in its last chunk by default, next to an empty `delta`, so `streaming.rs:312` reads it (`provider_usage` on 14 of 20 cerebras rows). Zai's usage is hidden behind `content: ""` (section 1), giving 18 of 18 unknown.
- The budget settles `result.usage.cost_usd` (e.g. `graph_task_dispatch/failover.rs:241-243` (a43)), so OpenAI and zai spend never counts against `max_plan_usd`.

**Classification.** A roko bug: a missing request option plus the parser bug. The difference between providers is a quirk. **Existing item:** none. `gap-ad0d39` (open, p2) covers Claude per-model usage and pricing, not OpenAI streaming usage.

**Fix.** Send `stream_options: {"include_usage": true}` on streaming requests, with a per-provider opt-out for servers that reject it. Read `usage` from any chunk (section 1 fix).

**Verify.** A `build_body(stream = true)` unit test asserting `stream_options.include_usage`. Live: one gpt-5.4-mini call must produce a `costs.jsonl` row with `cost_source = provider_usage`.

### 4b. Verdicts in `attempts.jsonl` never carry usage or cost amounts
**Mechanism (confirmed).** `AttemptRun::settle` (`attempt.rs:305-346`) sets `verdict.cost.source` (329) and nothing else on the cost. `verdict.usage.tokens_*` and `verdict.cost.{billed_usd, api_equiv_usd, without_cache_usd, vendor_usd, price_snapshot_id}` keep the `None` defaults set by `AttemptVerdictRecord::settle` (the structs are in `crates/roko-learn/src/telemetry/records.rs:642-720`). Nothing else writes them: the only other reference, `graph_task_dispatch/feedback.rs:472`, reads `cost.source`. The live verdict rows are all null, even for Cerebras and Claude CLI attempts whose `costs.jsonl` rows have amounts.

**Classification.** A roko bug: built but never connected. **Existing item:** none directly. `gap-ad0d39` (pricing) and `gap-7a8474` (efficiency fields always zero) are adjacent.

**Fix.** In `settle()`, map `dispatch.result.usage` / `usage_obs` into `AttemptUsage` (uncached input = input − cached, etc.) and price it into `AttemptCost` with the same function that writes `costs.jsonl`.

**Verify.** An `attempt.rs` unit test with the existing fake claude_cli fixture (`attempt.rs:891`: `total_cost_usd 0.01`, `output_tokens 10`) asserting `verdict.usage.tokens_out == Some(10)` and `verdict.cost.billed_usd == Some(0.01)`.

### 4c. Ladder escalation counts a substitute's failure
**Mechanism (confirmed).** The settle hook in `crates/roko-cli/src/graph_task_dispatch/ladder.rs:150-197` increments `failures_on_rung` for any `Blame::Agent` failure that has a ladder record (157-175). It escalates at `FAILURES_PER_RUNG = 2`, up to `MAX_ESCALATIONS = 2` (27, 30), and never looks at `verdict.executed.failover_chain` or at `model_dispatched` vs `model_requested`. In r2b, gpt-5.4-mini's circuit was open, so the `strong` attempt ran `cerebras-gptoss` (08:43:41 `model substitution`). Its failure counted as the second failure on `strong`, giving `escalations=2` (08:44:00) and a climb to `top`.

**Classification.** A roko bug. **Existing item:** none. The closed `bug-b2dd44` concerns the streaming path ignoring a substituted pinned model, which is a different defect.

**Fix.** Do not count attempts with a non-empty `failover_chain` against the routed rung, or count them against the substitute's rung.

**Verify.** A ladder unit test: a settled verdict with `failover_chain = ["gpt-5.4-mini"]`, `Blame::Agent` and rung `strong` must leave `failures_on_rung` unchanged.

### 4d. "Not logged in" is classified `Unknown`, retried, opens the circuit and fails over
**Mechanism (confirmed).**
- `crates/roko-agent/src/provider/error_classify.rs:52` (`classify_cli_error`) already maps "not logged in" (84) to `ProviderError::AuthFailure` (86).
- But host-side health uses `dispatch_v2::classify_provider_error` (`dispatch_v2.rs:2069-2098` (a43)). That is a separate, narrower copy covering exhaustion, billing, rate limit, timeout and 5xx, with **no auth branch**. So the result is `unknown`. `provider_health.rs:1299` would map `auth_failure` to `AuthFailure`, but nothing ever passes it.
- Recorder 2 hard-codes `Unknown` regardless (section 2).
- `TaskExecutorCell` retries the error, because it is not `Gateway{retryable:false}`.
- Failover counts only `Exhausted` and `Billing` as definitive (`graph_task_dispatch/failover.rs:374-390` (a43)), so the open circuit is read as `circuit_open` and the task is substituted onto a cheaper model.

**Classification.** The trigger is a harness artefact: per the run notes, the environment had no `USER`, so the Claude CLI could not find its login. The classification, retries and failover are a roko bug, and the duplicate classifier breaks CLAUDE.md rule 1. **Existing item:** none.

**Fix.** Replace `classify_provider_error` with the shared `error_classify` code so it returns `auth_failure`. Treat `AuthFailure` as definitive and non-retryable (a `no_credentials`-style refusal), not as a transient circuit failure.

**Verify.** `classify_provider_error("exit 1: not logged in · please run /login") == "auth_failure"`. A dispatch test must show no retry and health class `AuthFailure`.

## Suggested new work items

| # | Title | Sev | Anchor | Verify idea |
|---|---|---|---|---|
| 1 | Immune isolation is keyed by plan/task, never decays and has no release path, so one blank answer blocks the task in every later run; past 512 controls every new agent fails closed | p1 | `crates/roko-cli/src/graph_task_dispatch.rs:1351` (a43), `crates/roko-agent/src/immune_boundary.rs:478-498`, `crates/roko-agent/src/immune_evidence.rs:315-365` | Blank-then-retry `ImmuneScreenedAgent` test: the second attempt runs; a release CLI clears a control |
| 2 | A blank provider answer passes the tool loop as `Stop` and is treated as a security event | p1 | `crates/roko-agent/src/tool_loop/mod.rs:1366-1408`, `immune_boundary.rs:151-153` | Fake reasoning-only stream fails as `empty_response`; no control written |
| 3 | The OpenAI-compatible SSE parser keeps one field per chunk and only the first tool call; `error` objects are ignored and a missing finish reason becomes `stop` | p1 | `crates/roko-agent/src/streaming.rs:243-326`, `crates/roko-agent/src/openai_compat_backend.rs:823-832` | Z.ai-style chunk tests (usage + finish + `content:""`; reasoning + tool_calls; two tool calls) |
| 4 | Streaming requests never set `stream_options.include_usage`, so OpenAI spend is $0 to the plan budget | p1 | `openai_compat_backend.rs:446-448` | `build_body` test; one live gpt-5.4-mini call gives `cost_source = provider_usage` |
| 5 | Provider stream cap counts the whole multi-turn tool loop and denies a finished result without a log, receipt or reason | p1 | `immune_boundary.rs:55-56, 395-456, 780-790, 830-840` | 5,000-delta, 3-turn fake stream is accepted; a forced denial logs `immune_reason` |
| 6 | Immune denials and other non-provider failures count as provider failures, and a second recorder (`record_provider_health_at`) hard-codes `Unknown` and ignores the turn-cap and timeout exemptions | p1 | `crates/roko-cli/src/dispatch_v2.rs:1906-1927, 2108` (a43), `crates/roko-learn/src/model_call_feedback.rs:240-256` | A denied result leaves health unchanged; exactly one record per attempt in `provider-health.json` |
| 7 | `TaskExecutorCell` retries at once with no backoff and retries permanent denials | p2 | `crates/roko-graph/src/cells/task_executor.rs:826-875` | Retry gaps grow; an `agent_isolated`-type error is not retried |
| 8 | With no `max_turn_usd`, every call reserves the whole plan budget, which silently serialises the plan despite `width=N` | p2 | `crates/roko-cli/src/graph_task_dispatch/budget.rs:31-50, 218-241` | C6 variant with a budget and no `max_turn_usd` overlaps; the wait is logged |
| 9 | `AttemptRun::settle` never fills `usage.tokens_*` or the cost amounts in `attempts.jsonl` | p2 | `crates/roko-cli/src/graph_task_dispatch/attempt.rs:305-346` | Fake claude_cli fixture gives `tokens_out = 10`, `billed_usd = 0.01` |
| 10 | Ladder escalation counts a substitute model's failure against the routed rung | p2 | `crates/roko-cli/src/graph_task_dispatch/ladder.rs:157-191` | A verdict with `failover_chain` leaves `failures_on_rung` unchanged |
| 11 | `dispatch_v2::classify_provider_error` duplicates `error_classify` without an auth branch, so a Claude CLI "Not logged in" is retried, opens the circuit and fails over | p2 | `dispatch_v2.rs:2069-2098` (a43), `crates/roko-agent/src/provider/error_classify.rs:52-86`, `graph_task_dispatch/failover.rs:374-390` (a43) | Classifier unit test; no retry; health class `AuthFailure` |
| 12 | The pre-dispatch `agent_spawned` event labels an openai_compat attempt as `codex_cli` | p3 | `graph_task_dispatch.rs:1391-1400` (a43) | A glm-4.7 attempt's `agent_spawned` carries `provider = zai` |
