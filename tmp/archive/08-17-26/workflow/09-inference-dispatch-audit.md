# Inference & Provider Dispatch Subsystem Audit

Every path in roko that calls an LLM — who calls what, how responses are parsed, what feedback is recorded, and where the duplication lives.

## The Problem

There are **13+ separate call sites** that invoke LLMs across 8 crates. They use 4 different spawning mechanisms, 3 different response parsers, and record feedback inconsistently. The recent tool output visibility fix added a 4th copy of the stream-json parsing logic instead of consolidating into one.

---

## 1. Invocation Paths (Every Way We Call An LLM)

### 1A. Claude CLI Subprocess Spawning

Four separate `Command::new("claude")` (or equivalent) paths:

| # | File | Function | Args | Model? | System Prompt? | Stream? | Feedback? |
|---|---|---|---|---|---|---|---|
| 1 | `roko-acp/src/runner.rs` | `run_claude_cli()` | `--print --dangerously-skip-permissions` | No | No | No | None |
| 2 | `roko-acp/src/bridge_events.rs` | `run_claude_cognitive_task()` | `--print --output-format stream-json --model <m> --system-prompt <sp>` | Yes | Yes | stream-json | None |
| 3 | `roko-cli/src/runner/agent_stream.rs` | `spawn_agent()` | Via `CliProviderConfig::build_invocation()` | Yes | Yes | stream-json | Episode via event_loop |
| 4 | `roko-cli/src/dispatch_direct.rs` | `dispatch_claude_cli()` | `--print --output-format stream-json` | No (default) | No | stream-json | CostMeter only |

**What's wrong:** Same subprocess, 4 different arg sets, 4 different response handling paths. #1 gets no model control or streaming. #4 gets no system prompt. None go through provider resolution or CascadeRouter.

### 1B. HTTP API Calls (Anthropic Messages API)

| # | File | Class/Function | Endpoint | Tools? | Streaming? | Feedback? |
|---|---|---|---|---|---|---|
| 5 | `roko-agent/src/claude_agent.rs` | `ClaudeAgent::run()` | `/v1/messages` | Yes | No | Via TaskRunner events |
| 6 | `roko-agent/src/tool_loop/mod.rs` | `ToolLoop::run_task()` | Via LlmBackend trait | Yes | Optional | Via TaskRunner events |
| 7 | `roko-cli/src/dispatch_direct.rs` | `dispatch_anthropic_api()` | `/v1/messages` | No | No | CostMeter only |
| 8 | `roko-gate/src/llm_judge_gate.rs` | `JudgeOracle::judge()` | `/v1/messages` (inferred) | No | No | Gate verdict only |

**What's wrong:** #5 and #6 go through the provider system. #7 bypasses it entirely (reads `ANTHROPIC_API_KEY` from env, builds request manually). #8 is completely isolated — the gate's LLM call doesn't participate in routing, cost tracking, or episode logging.

### 1C. OpenAI-Compatible API Calls

| # | File | Class/Function | Used For | Feedback? |
|---|---|---|---|---|
| 9 | `roko-agent/src/openai_compat_backend.rs` | `OpenAiCompatLlmBackend::send_turn()` | OpenAI, Ollama, Perplexity chat, Gemini compat | Via TaskRunner |
| 10 | `roko-cli/src/dispatch_direct.rs` | `dispatch_openai_compat()` | One-shot `roko "prompt"` with OpenAI key | CostMeter only |

**What's wrong:** #9 is the proper provider path. #10 bypasses provider system, reads `OPENAI_API_KEY` from env directly.

### 1D. Specialized Provider Calls (Bypassing Provider System)

| # | File | What | Reads Env Directly? |
|---|---|---|---|
| 11 | `roko-cli/src/commands/research.rs` | Perplexity search | Yes (`PERPLEXITY_API_KEY`) |
| 12 | `roko-neuro/src/episode_completion.rs` | Neuro distillation | Yes (`ANTHROPIC_API_KEY`) |
| 13 | `roko-std/src/tool/builtin/web_search.rs` | Web search tool | Yes (`PERPLEXITY_API_KEY`) |
| 14 | `roko-dreams/src/runner.rs` | Dream consolidation | Yes (via `create_agent_for_model`) |

**What's wrong:** Every one of these reads API keys from environment variables directly instead of going through credential resolution. None participate in CascadeRouter, cost tracking, or episode logging.

### 1E. Dead Code Path (orchestrate.rs)

| # | File | Function | Status |
|---|---|---|---|
| 15 | `roko-cli/src/orchestrate.rs` | `dispatch_agent_with()` | DEAD — never called from any CLI path |

This is the most sophisticated dispatch path (CascadeRouter, 9-layer prompts, episode logging, HDC fingerprints, daimon affect, conductor intervention, budget guardrails, anomaly detection). 21,577 lines. Zero callers.

---

## 2. Response Parsing (How We Read LLM Output)

### 2A. Claude CLI stream-json Parsing

The tool output fix created **4 copies** of the same parsing logic:

| # | File:Line | Function | What It Does | Truncation? |
|---|---|---|---|---|
| 1 | `dispatch_direct.rs:94-160` | `dispatch_claude_cli()` | Parses `tool`/`result`/`assistant` event types, builds `ToolOutput` structs | 4096 + char_boundary |
| 2 | `translate/mod.rs:172-211` | `BackendResponse::extract_text()` | Parses same events, embeds tool output inline as `[toolname] content` | 4096 + char_boundary |
| 3 | `translate/mod.rs:190-219` | `BackendResponse::extract_tool_outputs()` | Parses same events, returns `Vec<(Option<String>, String)>` | 4096 + char_boundary |
| 4 | `chat.rs:464-489` | `extract_clean_text()` | Parses same events in JSONL multi-line mode, formats `[toolname] content` | 4096 + char_boundary |

**Inconsistencies across copies:**
- #1 stores `ToolOutput { tool_name: Option<String>, content: String }` — struct
- #2 embeds `"\n[{tool_name}]\n{content}\n"` into text buffer — inline string
- #3 returns `(Option<String>, String)` tuple — different type from #1
- #4 formats `"[{tool_name}] {content}"` or `"[{tool_name}] {content}...[truncated]"` — yet another format

**What should exist:** ONE `ClaudeStreamParser` that all paths use:
```rust
struct ParsedStreamEvent {
    text_deltas: Vec<String>,
    tool_outputs: Vec<ToolOutput>,
    usage: Option<Usage>,
    session_id: Option<String>,
    model: Option<String>,
}

fn parse_claude_stream_json(lines: &[String]) -> ParsedStreamEvent;
```

### 2B. Anthropic API Response Parsing

| Path | Parser | Returns |
|---|---|---|
| `ClaudeAgent::run()` | Deserializes `MessagesResponse` | `ContentBlock` (text, tool_use) |
| `ToolLoop::send_turn()` | Via `ClaudeTranslator::parse_calls()` | `Vec<ToolCall>` |
| `dispatch_anthropic_api()` | Manual JSON extraction | `String` (text only) |

**What's wrong:** #3 ignores tool_use blocks entirely — if the model returns tool calls via API, they're dropped.

### 2C. OpenAI-Compatible Response Parsing

| Path | Parser | Returns |
|---|---|---|
| `OpenAiCompatBackend::send_turn()` | Via `OpenAiTranslator::parse_calls()` | `Vec<ToolCall>` |
| `dispatch_openai_compat()` | Manual JSON extraction | `String` (text only) |

Same problem: the direct dispatch path drops tool calls.

### 2D. `extract_clean_text()` — The 246-Line Monster

`chat.rs:extract_clean_text()` handles 13 different response formats:
1. Claude CLI `stream-json` JSONL (multi-line, per-event)
2. Claude CLI `{"type":"result","result":"text"}` wrapper
3. Sidecar `{"content":"text"}` wrapper
4. Sidecar `{"content":[{"type":"text","text":"..."}]}` array
5. Anthropic API `{"content":[{"type":"text","text":"..."}]}` format
6. OpenAI `{"choices":[{"message":{"content":"..."}}]}` format
7. Gemini `{"candidates":[{"content":{"parts":[{"text":"..."}]}}]}` format
8. Run status `{"status":"...", "output":"..."}` wrapper
9. Plain text (passthrough)
10. Empty/whitespace (passthrough)
11. `tool` events (new — 4096 truncation)
12. `system` events (skipped)
13. Generic `result`/`content` string fields (fallback)

This function is doing the work of 5+ separate response parsers. It should be replaced by typed deserialization per backend, not a giant match cascade.

---

## 3. Model Routing & Selection

### 3A. CascadeRouter (LinUCB Bandit)

**File:** `roko-learn/src/cascade_router.rs`
**Status:** Built, persisted to `.roko/learn/cascade-router.json`, but only called from dead `orchestrate.rs`

**Features:**
- LinUCB contextual bandit (17 features in orchestrate.rs)
- Per-model reward tracking (mean, variance, trial count)
- Task requirement scoring (complexity, domain, category)
- Conductor load pressure integration
- Knowledge-informed routing boost (neuro store query)
- Cost spike detection and model filtering
- Routing decision explanation

**Live callers:** Zero. `dispatch_direct.rs` uses hardcoded model defaults.

### 3B. Model Resolution (What's Actually Used)

| Entry Point | How Model Is Selected |
|---|---|
| `roko "prompt"` | `AuthMethod::ClaudeCli` → whatever `claude` defaults to |
| `roko chat` | Agent sidecar config (roko.toml `agent.model`) |
| `roko run` | `resolved_model()` → config default or task tier |
| `roko plan run` | `CliProviderConfig` from task definition |
| `roko acp` | Hardcoded or bridge_events model param |
| orchestrate.rs (dead) | CascadeRouter + role override + conductor bias |

---

## 4. Feedback & Learning Recording

### 4A. What Gets Recorded Where

| Signal | Written To | Written By | Live Callers |
|---|---|---|---|
| Episode (full turn record) | `.roko/episodes.jsonl` | `EpisodeLogger::append()` | `orchestrate.rs` only (dead) |
| Efficiency event | `.roko/learn/efficiency.jsonl` | `publish_turn_learning_feedback()` | `orchestrate.rs` only (dead) |
| Cost record | `.roko/learn/costs.jsonl` | `CostsLog` | `orchestrate.rs` only (dead) |
| Routing decision | `.roko/learn/routing-log.jsonl` | `RoutingDecisionLogStore` | `orchestrate.rs` only (dead) |
| CostMeter (session) | In-memory only | `chat_inline.rs` | `roko` / `roko "prompt"` |
| Gate verdict | Episode attachment | `run_gate_pipeline()` | `roko run`, `roko plan run` |
| TaskRunner events | EventBus (in-memory) | `TaskRunner` | `orchestrate.rs` only (dead) |

**The problem:** ALL persistent learning (episodes, efficiency, costs, routing) is only written from the dead orchestrate.rs path. The live paths (`roko`, `roko chat`, `roko run`) record nothing durable. The system cannot learn from its runs.

### 4B. What a Unified FeedbackService Should Record

Every model call, regardless of entry point, should emit:
1. **Episode** — who called, what model, what prompt sections, token usage, cost, duration
2. **Routing observation** — what model was chosen, why, what happened (for CascadeRouter)
3. **Cost record** — model, tokens, cost_usd, caller context
4. **Gate attachment** — if gates ran, attach verdicts to the episode

---

## 5. Prompt Assembly

### 5A. Where Prompts Are Built

| Path | How | SystemPromptBuilder? | Templates? | Knowledge? |
|---|---|---|---|---|
| `dispatch_direct.rs` | No system prompt at all | No | No | No |
| `chat_inline.rs` | No system prompt | No | No | No |
| `roko run` | `dispatch_helpers::build_system_prompt_with_context_validated()` | Yes (9-layer) | Yes | Playbooks + skills |
| `roko plan run` | Via `CliProviderConfig` + task definition | Partial | Partial | No |
| `roko acp runner.rs` | Inline format strings | No | No | No |
| `roko acp bridge_events.rs` | Inline format strings | No | No | No |
| orchestrate.rs (dead) | Full 9-layer builder + VCG + bidders + affect | Yes | Yes | Full |

**The problem:** Only `roko run` and the dead `orchestrate.rs` use the prompt builder. The most common paths (`roko`, `roko chat`) send bare prompts with zero system context.

### 5B. Template System (Unused by Most Paths)

`roko-compose/src/templates/` has proper templates for:
- Strategist, Implementer, Reviewer, Architect, Auditor, Scribe, Critic
- Quick reviewer, researcher, dream reviewer

`roko-compose/src/system_prompt_builder.rs` has the 9-layer assembly:
1. Role definition (from template)
2. Convention rules
3. Domain context
4. Task description
5. Gate feedback (prior failures)
6. Tool descriptions
7. Skill library
8. Anti-patterns
9. Affect state

Only 2 of 6+ entry points use any of this.

---

## 6. Anti-Pattern Assessment

The recent tool output visibility fix violates:

| Anti-Pattern | How |
|---|---|
| **#7 Copy-Paste Between Runtimes** | 4 copies of 4096-byte truncation + char_boundary + tool event parsing |
| **#3 Build Another Runtime** | Adds parsing to multiple existing paths instead of building toward ModelCallService |
| **#1 Just Shell Out** | Fixes are layered on top of the bare `claude --print` path instead of replacing it |

**What Phase 0.1 should replace:**
- All 4 stream-json parsers → ONE `ClaudeCliAdapter` with `ClaudeStreamParser`
- All 3 API dispatch functions → ONE `ModelCallService::complete()` / `::stream()`
- All env-var key reads → ONE credential resolver in `InferenceGateway`
- CostMeter-only tracking → `FeedbackService` that records episodes, costs, routing

---

## 7. File Inventory

### Files That Touch LLM Invocation

| File | LOC | What | Lives? | Anti-Pattern? |
|---|---|---|---|---|
| `roko-cli/src/orchestrate.rs` | 21,577 | Full dispatch + routing + feedback + gates + replan | Dead | #10 God file |
| `roko-cli/src/dispatch_direct.rs` | 405 | 3 dispatch functions (CLI, API, OpenAI) | Live | #1 bare spawns, #7 duplicated parsing |
| `roko-cli/src/chat.rs` | 659 | REPL + `extract_clean_text()` | Live | #7 duplicated parsing |
| `roko-cli/src/run.rs` | 1,555 | Universal loop (best live path) | Live | Closest to correct |
| `roko-cli/src/dispatch/mod.rs` | ~200 | CliProviderConfig for plan runner | Live | Partial provider use |
| `roko-agent/src/claude_agent.rs` | 970 | Anthropic API agent | Live | Proper (via provider system) |
| `roko-agent/src/tool_loop/mod.rs` | 1,837 | Multi-turn tool loop | Live | Proper |
| `roko-agent/src/translate/mod.rs` | 700+ | Response parsing + BackendResponse | Live | #7 duplicated tool parsing |
| `roko-agent/src/provider/mod.rs` | 700+ | Provider adapter registry | Live | Proper (this is the right layer) |
| `roko-agent/src/openai_compat_backend.rs` | ~500 | OpenAI-compat backend | Live | Proper |
| `roko-agent/src/gemini/native.rs` | ~400 | Gemini native API | Live | Proper |
| `roko-agent/src/perplexity/mod.rs` | ~600 | Perplexity search + chat | Live | Proper but bypassed by CLI |
| `roko-agent/src/task_runner.rs` | 929 | TaskRunner (budget, anomaly, conductor) | Live | Proper but only used from dead path |
| `roko-acp/src/runner.rs` | ~800 | ACP pipeline driver | Live | #1 bare spawns, #2 inline prompts |
| `roko-acp/src/bridge_events.rs` | ~2,500 | ACP event bridge | Live | #1 different bare spawn |
| `roko-serve/src/routes/gateway.rs` | ~47K | HTTP gateway routes | Live | Some bypass provider |
| `roko-gate/src/llm_judge_gate.rs` | ~300 | LLM judge for gate rung 3 | Live | Isolated LLM call |
| `roko-dreams/src/runner.rs` | ~300 | Dream consolidation | Live | Direct `create_agent_for_model` |
| `roko-neuro/src/episode_completion.rs` | ~200 | Neuro distillation | Live | Direct env key read |
| `roko-std/src/tool/builtin/web_search.rs` | ~200 | Web search tool | Live | Direct env key read |

### What ModelCallService Replaces

Every row marked "Anti-Pattern" in the table above. The proper provider system (`roko-agent/src/provider/`) is the right foundation — ModelCallService wraps it with:
- Credential resolution (no more env reads)
- CascadeRouter integration (model selection)
- Automatic episode + cost recording
- Prompt assembly via SystemPromptBuilder
- Stream-json parsing via one ClaudeStreamParser

---

## 8. Grep Gates (Acceptance Criteria)

After ModelCallService is implemented, these should return zero results outside tests:

```bash
# No more bare claude spawns
rg 'Command::new\("claude"\)' crates/ --type rust | grep -v test | grep -v orchestrate

# No more direct env key reads for providers
rg 'std::env::var.*API_KEY' crates/ --type rust | grep -v test | grep -v orchestrate

# No more extract_clean_text (replaced by typed parsing)
rg 'extract_clean_text' crates/ --type rust | grep -v test

# No more inline 4096 truncation (one utility function)
rg '4096' crates/ --type rust | grep -v test | wc -l  # should be ≤ 1

# All dispatch paths go through ModelCallService
rg 'dispatch_claude_cli|dispatch_anthropic_api|dispatch_openai_compat' crates/ --type rust | grep -v test  # should be 0
```
