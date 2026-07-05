# 100 · TRACE — ACP session end-to-end

> **Verification** · HEAD `5852c93c05` · branch `main` · 2026-07-08 · deeper second pass over `51-ACP.md`.
> Sources read line-by-line this pass: `crates/roko-acp/src/handler.rs` (663), `bridge_events.rs` (5598), `builtin_tools.rs` (810); cross-refs `session.rs`, `transport.rs`, `types.rs`, `roko-cli/src/main.rs`.
> Status tags: ✅ wired · ⚠️ partial/branch-gated · ❌ built-not-called · 🔌 orphaned (built + tested, zero prod callers) · 🕰️ legacy.
>
> **Headline (deeper than 51-ACP):** the orphaned permission gate is not merely *unwired* — it is on the **wrong side of a task boundary**. `handle_session_prompt_inner` spawns the whole dispatch as a detached `tokio::spawn` (bridge_events.rs:1320) into which **only the mpsc `event_sender` is moved**; the `transport` + `&mut AcpSession` stay in the *parent* stream loop. Tool execution happens 4 frames deep inside `AcpBuiltinToolHandler::execute(&self, call, ctx)` (bridge_events.rs:2921) — a trait method with **no transport and no session handle**. `request_permission` (bridge_events.rs:768) *requires* both. So the gate is architecturally unreachable from the execution site; wiring it needs a new request/reply channel (CognitiveEvent variant + oneshot), not a one-line call. The code comment at bridge_events.rs:**1175** ("Permission is requested per-tool-call by the agent, not preemptively") describes a mechanism **that does not exist**.

---

## 1 · The hops (file:line at every step)

### Boot / transport / handshake
| # | Hop | File:line | Note |
|---|---|---|---|
| 0 | CLI arm `roko acp` forks a Tokio runtime **before** any stdout tracing subscriber | `roko-cli/src/main.rs:681-698,2016-2049` | stdout is the protocol channel; tracing → `.roko/acp.log` (handler.rs:617-646). Duplicate unreachable arm at main.rs:2573-2589. |
| 1 | `run_acp_server_inner` → mkdir `.roko/`, install file logger, `StdioTransport::new()` | `handler.rs:49-73` | Falls back to `/tmp/roko-acp-<pid>.log` if `.roko/` unwritable (56-68). |
| 2 | `run_acp_server_with_transport`: load `roko.toml` (+warning), provider-readiness check, `SessionManager::new`, `ConfigWatcher::start`, GC sessions >7d | `handler.rs:76-140` | `check_provider_readiness` (235-267): CLI providers need no key; API providers need `api_key_env`. |
| 3 | **Read loop**: `transport.read_message()` → match Request / Response / Notification | `handler.rs:142-231` | On each Request, checks `config_watcher.changed()` and hot-reloads + pushes `server/config_sources_update` (166-220). Response → `handle_incoming_response` (224). Single-transport, single-client. |

### initialize (capability advertisement)
| # | Hop | File:line | Note |
|---|---|---|---|
| 4 | `"initialize"` → `InitializeResult` | `handler.rs:280-309` | `protocol_version=ACP_PROTOCOL_VERSION`; `load_session:true`. |
| 4a | **Capabilities advertised** | `handler.rs:287-298` | `prompt_capabilities{ image:false, audio:false, embedded_context:true }`; `mcp_capabilities{ http:true, sse:true }`; `auth_methods:[]` (299). |
| 4b | `agent_info{name:"roko", version:CARGO_PKG_VERSION, title:"Roko"}`, plus `config_sources` + `config_warnings` from startup | `handler.rs:300-306` | Startup warnings surfaced to the editor here. |

### session/new (are mcpServers accepted?)
| # | Hop | File:line | Note |
|---|---|---|---|
| 5 | `"session/new"` → `sessions.create_session(params)` | `handler.rs:310-327` | Then auto-detects bare-mode and pushes `available_commands_update` (send_slash_commands_notification, 524-541). |
| 5a | **`mcpServers` accepted unconditionally** at session creation regardless of provider | `session.rs` `create_session` / validation ~904-926 (per 51-ACP) | The stored `session.mcp_servers` is later honored **only** if the resolved provider is openai-compat (see hop 11). Accepting-then-silently-ignoring for Anthropic/Claude-CLI is drift D2. |
| 5b | `session/list`, `session/load`, `session/close`, `session/resume`, `session/set_mode`, `session/config/update` | `handler.rs:328-459` | `close`/`resume` exist beyond the ACP guide TOC. |

### prompt turn
| # | Hop | File:line | Note |
|---|---|---|---|
| 6 | `"session/prompt"` → clone workdir+config, `get_session_mut`, `handle_session_prompt(...)`, then `persist_session` + `send_success` | `handler.rs:350-377` | The **only** entry into the dispatch engine. |
| 7 | `handle_session_prompt`: busy-guard → `begin_prompt` → `handle_session_prompt_inner` → `finish_prompt` | `bridge_events.rs:1111-1132` | Busy sessions rejected with `SessionBusy` (1122). |
| 8 | Inner: `extract_prompt_text`, `resolve_model`, detect leading `/` (slash), pick `WorkflowTemplate` (`auto` → `auto_select`) | `bridge_events.rs:1145-1169` | Slash commands bypass knowledge/context/history entirely. |
| 9 | `push_user_turn` (non-slash) — **comment claims per-tool-call permission here (1175) but no such code exists** | `bridge_events.rs:1172-1177` | 🔌 The load-bearing lie. |
| 10 | Knowledge+playbook query, @-mention/resource context, system prompt, `build_messages_array`, **image content-parts built** for Anthropic (1223-1224) / OpenAI (1226) | `bridge_events.rs:1179-1237` | Image parts built despite `image:false` advertised (D6). |
| 11 | Channel `mpsc::channel(64)`; emit knowledge/provenance cards; snapshot `session_mcp_servers`, `tools_enabled`, `effort`, cancel token | `bridge_events.rs:1239-1278` | Everything the detached task needs is **copied out** of `session` here. |
| 12 | **SP-1 pre-dispatch safety**: `SafetyLayer::with_defaults().with_role(mode)` → `pre_dispatch_check`; `Block` short-circuits | `bridge_events.rs:1280-1310` | Falls back to permissive default when the role YAML contract is missing (partial per CLAUDE.md). |

### provider dispatch (the fork)
| # | Hop | File:line | Note |
|---|---|---|---|
| 13 | **`cognitive_task = tokio::spawn(async move { … })`** — `event_sender` moved in; **transport + session stay behind** | `bridge_events.rs:1320` | ← The task boundary that strands the permission gate. |
| 14 | Inside task: safety-block branch → slash branch (`run_slash_command`, subprocess) → pipeline branch (`run_with_workflow_engine`, or legacy behind `ROKO_ACP_LEGACY`) → **default single-agent** | `bridge_events.rs:1321-1428` | |
| 15 | **Provider match** on `resolved.provider_kind` | `bridge_events.rs:1440-1476` | `AnthropicApi` → `run_anthropic_cognitive_task` (1445); **all others incl. `ClaudeCli`** → `run_openai_compat_cognitive_task` (1462). |
| 16 | Parent (holding transport) runs `stream_events_to_editor` concurrently, draining the mpsc | `bridge_events.rs:1479-1485` | `AcpEventForwarder::from_env` optionally mirrors events to `ROKO_SERVE_URL` (965,1010). |

### streaming + tool execution + (missing) permission
See §2 for the per-loop split. Common execution sink:
| # | Hop | File:line | Note |
|---|---|---|---|
| 17 | `ToolLoop::run_messages_streaming` → `ToolDispatcher` → `AcpBuiltinToolHandler::execute(&self, call, ctx)` | `bridge_events.rs:2921-2936` | Trait method: **no transport, no session**. |
| 18 | → `execute_acp_builtin_tool(name, args, workdir, event_sender)` | `builtin_tools.rs:269-322` | Emits `ToolCallStart` (280) → **unconditional match** (291-301) → `ToolCallComplete` (309). |
| 19 | `write_file`/`edit_file`/`bash` **run with no consent** | `builtin_tools.rs:293-297,390-435,678-747` | Only guard is the `resolve_path` workdir jail (217-234, write variant 237-255). `needs_permission` (173) / `tool_needs_permission` (326) have **zero callers**. |
| 19b | Tool capabilities are **hard-forced all-true** just before the loop runs | `bridge_events.rs:1829-1835, 2360-2366` | `ToolPermission{read,write,exec,git,network: all true}` — the ctx-level cap check is a no-op. |
| — | 🔌 `request_permission` (fail-closed, AlwaysAllow-persisting round-trip) | `bridge_events.rs:768-950` | Callers: **only** unit tests 5154 / 5181 / 5218. Needs `&mut StdioTransport` + `&mut AcpSession` (768-775) — both stranded in the parent. |

### streaming — subprocess line-reading (slash + Claude-CLI opacity)
| # | Hop | File:line | Note |
|---|---|---|---|
| 20 | `run_slash_command` spawns `roko <cmd>` as a child, pipes stdout/stderr | `bridge_events.rs:3352,3926-3956` | Not buffered — line-by-line `next_line()` loop. |
| 21 | Parse `ROKO_PROGRESS: {json}` → `task_started`→`ToolCallStart(Terminal)`, `task_completed`→`ToolCallComplete`, `agent_started`→`TokenChunk` | `bridge_events.rs:3972-4038` | Non-JSON / unknown-type lines pass through as raw text (4024-4038). |
| 22 | Cancel → `child.kill()`; stderr loop mirrors stdout | `bridge_events.rs:3966-3971,4047-4160` | Tool calls **inside** a Claude-CLI subprocess remain opaque unless it emits `ROKO_PROGRESS` (D8-C). |

### learning telemetry (observe-only) + session end
| # | Hop | File:line | Note |
|---|---|---|---|
| 23 | After stream completes: `usage` update pushed; post-dispatch safety (log-only, response already sent) | `bridge_events.rs:1487-1566` | |
| 24 | `append_acp_episode` → `.roko/episodes.jsonl` (kind `acp-dispatch`/`acp-pipeline-*`, cost from pricing table or workflow override) | `bridge_events.rs:298-525` (called 1573) | ✅ write. |
| 25 | `emit_acp_efficiency_event` → `.roko/learn/efficiency.jsonl` | `bridge_events.rs:527-608` (called 1589) | ✅ write. |
| 26 | `record_cascade_observation` → **`observe` only**, `select_model` never called | `bridge_events.rs:685-722` (called 1614); ctx `acp_routing_context` 609-641; reward `compute_acp_reward` 663-683 | ❌ decisioning. `daimon_policy: DaimonPolicy::default()` hardcoded (634). Model always came from `session.model` (D3/D4). |
| 27 | Persist session (handler.rs:375), auto-title (1632-1648), push assistant turn / pop dangling user turn (1650-1661), return `SessionPromptResult` | `bridge_events.rs:1632-1663` | Dream consolidation may spawn if ≥10 episodes (`maybe_spawn_dream_consolidation`, 447-525). |
| 28 | `session/cancel` (notification) → `session.cancel()` cooperative token | `handler.rs:470-498` | EOF on stdin → clean shutdown (handler.rs:145-148). |

---

## 2 · Three-"tool-loop" comparison

Reality: there are **two** in-process ACP tool loops (Anthropic-native, openai-compat) plus a **degenerate third** — the Claude-CLI / plain-streaming path that runs **no ACP-side loop** (any tool use happens inside the `claude` subprocess, invisible to ACP).

| Aspect | **Anthropic native** | **openai-compat** | **Claude-CLI / plain** |
|---|---|---|---|
| Entry | `run_anthropic_cognitive_task` 1671 → `run_anthropic_builtin_tool_loop` 1746 | `run_openai_compat_cognitive_task` 2159 → MCP loop 2272 **or** builtin loop 2448 | `run_openai_compat_cognitive_task` 2159 → falls through to `stream_model_call_to_cognitive_events` 2005 |
| Gate to enter loop | `tools_enabled` (1708) + Anthropic API key present (1757-1769) | `openai_compat_tool_loop_supported` = `OpenAiCompat\|Perplexity\|Cerebras` (2264-2269) | `ClaudeCli` is **not** in that set → loop **skipped**, plain stream only |
| Streaming | `forward_tool_loop_stream_chunks` via `chunk_sender` (1820-1824) | same forwarder (2352-2356) | `stream_model_call_to_cognitive_events` (2005) — text chunks only |
| Tool exec sink | `AcpBuiltinToolHandler`→`execute_acp_builtin_tool` (2921) | builtin: same handler; MCP: `AcpMcpHandlerResolver` (2343) | none in ACP — subprocess-internal |
| **Permission gate** | ❌ none | ❌ none | ⚠️ external (the `claude` binary's own perms; ACP has zero visibility) |
| **Session MCP servers** | ❌ never registered (only `acp_builtin_tools`, 1779) | ✅ `setup_session_mcp_tools` (2313) — **only place MCP is wired** | ❌ not passed in-band |
| Builtin 8 tools | ✅ | ✅ (when no MCP servers or MCP empty, 2323-2333) | n/a |
| **Learning decisioning** | ❌ observe-only | ❌ observe-only | ❌ observe-only |
| Cap enforcement | forced all-true (1829-1835) | forced all-true (2360-2366) | n/a |
| Max iterations | `DEFAULT_MAX_TOOL_ITERATIONS` (25) 1817 | 25 (2350) | n/a |

Note: openai-compat with a session that declared `mcpServers` prefers the MCP loop (2188); if MCP discovery yields zero tools it emits a notice and returns `false` (2323-2333) so control falls to the builtin loop (2207).

---

## 3 · Capability-truth table (advertised vs real)

| Capability (handler.rs:287-306) | Advertised | Reality | Verdict |
|---|---|---|---|
| `prompt.image` | **false** | Anthropic image parts built at 1223-1224; OpenAI `image_url` data-URI at 1226 (build_openai_content_parts) | ⚠️ **under-advertised** — build path is dead because editors won't send images |
| `prompt.audio` | false | no audio path | ✅ honest |
| `prompt.embedded_context` | true | `resolve_context_items` / resource blocks / @-mentions (1189-1202) | ✅ |
| `mcp.http` / `mcp.sse` | true / true | MCP tools reach the model **only** via openai-compat loop (2313); Anthropic + Claude-CLI ignore `mcpServers` | ⚠️ **over-advertised** for 2 of 3 providers (D2) |
| `load_session` | true | `session/load` + `session/resume` implemented (handler.rs:332-348,413-444) | ✅ |
| `auth_methods` | `[]` | none | ✅ (nothing to authenticate) |
| Per-action permission (implied by 1175 comment + `request_permission` machinery) | *implied* | 🔌 gate exists (768) but zero prod callers; tools run unconditionally (builtin_tools.rs:291) | ❌ **advertised-by-implication, absent in fact** |
| `terminal/*` client capability | parsed only (`ClientCapabilities.terminal`) | no terminal calls made | ❌ inert |
| Cascade routing / Daimon / experiments (CLI parity) | *implied by CLAUDE.md "learning wired e2e"* | telemetry-only in ACP (D3/D4/D5) | ❌ CLI-only |

---

## 4 · Session state diagram

```
                       stdin EOF ──────────────► [SHUTDOWN] (handler.rs:145)
                          ▲
   ┌──────────┐  initialize   ┌───────────┐  session/new   ┌───────────┐
   │  BOOTED  │ ────────────► │ HANDSHOOK │ ─────────────► │   IDLE    │◄──────┐
   │ (h:76)   │  caps=image:F │ (h:280)   │  +mcpServers   │ (session  │       │
   └──────────┘               └───────────┘  stored (5a)   │  created) │       │
                                                            └────┬──────┘       │
                                      session/prompt (h:350)     │              │
                                                                 ▼              │
                                                          ┌─────────────┐       │
                                     busy? ──yes──► reject │  PROMPTING  │       │ persist +
                                                          │ begin_prompt│       │ push turn
                                                          │ (be:1126)   │       │ (h:375,
                                                          └──────┬──────┘       │  be:1650)
                                                                 │              │
                       ┌── SP-1 pre-dispatch safety (be:1280) ───┤              │
                       │        Block ─► TokenChunk + Complete    │             │
                       ▼                                          ▼             │
              ╔══════════════════ tokio::spawn (be:1320) ══════════════════╗    │
              ║  event_sender ONLY crosses here; transport+session stay out ║    │
              ║                                                             ║    │
              ║  slash? ─► run_slash_command (subprocess, ROKO_PROGRESS)    ║    │
              ║  pipeline? ─► run_with_workflow_engine                      ║    │
              ║  else provider match (be:1440):                            ║    │
              ║    AnthropicApi ─► anthropic tool loop (be:1746)           ║    │
              ║    _ (incl ClaudeCli) ─► openai_compat task (be:2159)      ║    │
              ║        ├ mcpServers+supported ─► MCP loop (be:2272)        ║    │
              ║        ├ tools_enabled+supported ─► builtin loop (be:2448) ║    │
              ║        └ else ─► plain stream (be:2005)                    ║    │
              ║                     │                                       ║    │
              ║      tool exec ─► AcpBuiltinToolHandler::execute (be:2921) ║    │
              ║                     │  ✗ NO permission (gate stranded)     ║    │
              ║                     ▼  write/edit/bash unconditionally     ║    │
              ║              builtin_tools.rs:291 (jail-only)              ║    │
              ╚═════════════════════════════════════════════════════════════╝  │
                       │  events ─► (mpsc) ─► stream_events_to_editor (be:954)  │
                       ▼                         held by PARENT (has transport) │
              telemetry: episode(be:298)+efficiency(be:527)+cascade OBSERVE ────┘
              (be:685, select_model never called; daimon=default)
                       │
             session/cancel (notification) ─► cooperative token (h:470)
```

---

## 5 · Wiring checklist

### A · Permission gate (P0 — the structural fix)
- [ ] **Add a reply channel across the task boundary.** Introduce `CognitiveEvent::PermissionRequest { action, title, detail, reply: oneshot::Sender<PermissionDecision> }` so the tool handler can ask and block. Tool sink: `AcpBuiltinToolHandler::execute` (bridge_events.rs:2921) sends it via `event_sender`; awaits the oneshot.
- [ ] **Answer it in the parent loop** that owns `transport` + `&mut session`: extend `stream_events_to_editor` (bridge_events.rs:954-1013) to match `PermissionRequest` and call the existing `request_permission` (768) then `reply.send(decision)`.
- [ ] **Gate the execution** in `execute_acp_builtin_tool` (builtin_tools.rs:291): if `needs_permission(name)` (173) and decision != `Allow`/`AlwaysAllow`, return a `Rejected` result instead of running the tool.
- [ ] Delete/repurpose the misleading comment at bridge_events.rs:**1175**.
- [ ] Conformance test: outbound `session/request_permission` precedes any `write_file`; a `Reject` blocks the write and `AlwaysAllow` persists to `.roko/trust/permissions.json` (grant_always_allow, 860-861).
- [ ] Per-slash-command tool denylist (D7) once the gate exists.

### B · Learning decisioning (P1 — make telemetry two-way)
- [ ] Call `CascadeRouter::select_model` **before** dispatch (currently only `observe`, 685-712) when `routing_mode != manual`; feed `acp_routing_context` (609) as features.
- [ ] Load real `DaimonState` from `.roko/state/daimon.json` (or recent episodes) into `acp_routing_context` instead of `DaimonPolicy::default()` (634).
- [ ] Consult `ExperimentStore` for a prompt variant per role before building the system prompt (1205).
- [ ] Emit a distinct manual-override signal for UX34 (CLAUDE.md item 15).

### C · MCP parity (P1)
- [ ] Register `setup_session_mcp_tools` (2313) in `run_anthropic_builtin_tool_loop` (1779 builds only `acp_builtin_tools`), **or** reject `mcpServers` at `session/new` for non-openai providers with a clear error instead of silently storing them.
- [ ] Add MCP to the Claude-CLI path (or document its opacity).

### D · Capability honesty (P1/P2)
- [ ] Reconcile `image:false` (handler.rs:290): advertise `image:true` + test, or delete the image content-part builders (bridge_events.rs:1223-1233, `build_openai_content_parts`).
- [ ] Advertise MCP only for providers that honor it, or fix C first.
- [ ] Wire or drop the inert `terminal/*` client capability.

### E · Re-verification (P0 hygiene — last live log 2026-05-09)
- [ ] `printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1}}\n' | cargo run -p roko-cli -- acp --workdir . | head -1`
- [ ] `cargo test -p roko-acp`
- [ ] Add `roko-acp` row to CLAUDE.md crate table; remove unreachable `Command::Acp` arm (main.rs:2573-2589) + date-stamp `ROKO_ACP_LEGACY` path.

---

## 6 · Corrections / deltas vs 51-ACP.md
51-ACP is accurate; this pass **deepens** rather than corrects it:
- **New (structural):** the permission gate is stranded by the `tokio::spawn` task boundary at bridge_events.rs:1320 — the tool sink (`AcpBuiltinToolHandler::execute`, 2921) has neither `transport` nor `session`, so the fix is a reply-channel refactor, not a call insertion. 51-ACP framed D1 as "not wired"; it is more precisely "**unwireable without a channel**."
- **Clarified:** the "third tool loop" (Claude-CLI) is **not a loop** — `ClaudeCli` is absent from `openai_compat_tool_loop_supported` (2264), so it degrades to plain text streaming (`stream_model_call_to_cognitive_events`, 2005); any tool use is subprocess-internal and opaque.
- **Confirmed:** capabilities on `ToolContext` are hard-forced all-true (1829-1835, 2360-2366), so even the ctx capability layer is a no-op — a detail 51-ACP did not call out.
</content>
</invoke>
