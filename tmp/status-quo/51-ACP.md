# roko-acp — Agent Client Protocol

> Status-quo audit · re-verified 2026-07-08 (HEAD 5852c93c0) · sources: 15 src files (~15,851 LOC) + 3 test files (~1,247 LOC) + `docs/v2/ACP-INTEGRATION-GUIDE.md` (2,177 lines) + `tmp/acp-features/00-ACP-FEATURES.md` + `tmp/acp-runner/` + `tmp/tmp-feedback/2/{18,19,32,34}` (2026-05-16) + `.roko/acp.log` + call-site greps.
>
> **This revision corrects a P0 miss in the 2026-07-07 pass** (permission gating claimed ✅) and reconciles the four May-16 tmp-feedback tickets against current code. Feedback tickets are a mix of *fixed*, *still-valid*, and *stale* — each is adjudicated with file:line below.

## Summary

`roko-acp` is an **ACP server** (agent side) — Roko exposed as a coding agent to any ACP-compatible editor (Zed, Cursor, JetBrains, Neovim) over stdio JSON-RPC 2.0 (`src/lib.rs:1-5`). It implements the **upstream Agent Client Protocol, spec 0.12.2 / protocol version 1** (`src/types.rs:6-9`) — method names (`initialize`, `session/new|load|prompt|cancel|set_mode`, `session/request_permission`, `session/update` variants, `fs/*` flags) match the Zed ACP spec. It is **not** the v2 "Connect protocol" (a Cell connector protocol, `docs/v2/11-CONNECTIVITY.md:11-13`) and is not one of the five v2 Surfaces (`docs/v2/20-SURFACES.md:3` = CLI/TUI/Dashboard/Visual-Editor). ACP is a de-facto sixth surface, documented only in `docs/v2/ACP-INTEGRATION-GUIDE.md`.

The crate is **absent from CLAUDE.md's crate table**. It was bootstrapped by the `tmp/acp-runner/` harness — an overnight **Codex batch runner** (ACP01–ACP18) that generated the crate with per-batch scope/clippy/test gates. acp-runner *built* roko-acp via agents; it did not drive agents *through* ACP.

Wiring is real and deep: single entry point `roko acp` (`crates/roko-cli/src/main.rs:681-698`), which forks its own Tokio runtime **before** any stdout tracing subscriber (main.rs:2016-2049) because stdout is the protocol channel. Logging → `.roko/acp.log` via `tracing-appender` (`src/handler.rs:617-646`). **`.roko/acp.log` stops 2026-05-09 (493 lines, `initialize`/`session/load` from a real editor); no live use in ~2 months.** Dispatch integrates most of the stack: roko-agent providers, roko-gate (adaptive thresholds), roko-learn (episodes, efficiency), roko-neuro (knowledge/playbook), roko-dreams (consolidation trigger), roko-runtime `WorkflowEngine`, and a SafetyLayer pre-dispatch check. `.roko/GAPS.md` has zero ACP entries. ~90 test fns across the crate + 16 integration tests.

**The bottom line for this pass:** the *protocol surface* is solid and spec-faithful, but three learning/adaptation systems that are load-bearing in the CLI path are **stubbed in ACP** (cascade *selection*, DaimonState, prompt experiments), and — most importantly — the **in-process builtin tool loop executes `write_file`/`edit_file`/`bash` with no permission gate**, despite a fully-built `request_permission` machinery sitting orphaned. See Drift.

## Current state table

| Component | Code | Status | Evidence |
|---|---|---|---|
| Stdio JSON-RPC 2.0 transport (+ outbound request registry) | `src/transport.rs` (307) | ✅ | `StdioTransport` w/ `pending_requests` for server→client requests (transport.rs:47-52) |
| `initialize` handshake (capabilities, agentInfo, config warnings) | `src/handler.rs:280-309` | ✅ | protocol 1, `load_session:true`, MCP http+sse, embedded_context:true, startup warnings |
| Session methods: new/list/load/prompt/config-update/set_mode | `src/handler.rs:310-459` | ✅ | Full dispatch match |
| `session/close` + `session/resume` (beyond guide TOC) | `src/handler.rs:405-444` | ✅ | Implemented beyond guide's method reference |
| `session/cancel` → CancelToken (cooperative) | `src/handler.rs:470-498`, `src/session.rs:44-95` | ✅ | Cooperative cancel |
| `session/update` streaming (message/thought chunks, tool_call, plan, usage, session_info, commands, config_option) | `src/bridge_events.rs`, `src/types.rs:574-590` | ✅ | `map_event_to_update`; UsageUpdate; SessionInfoUpdate |
| Slash-command subprocess streaming (**not buffered**) | `src/bridge_events.rs:3926-4160` | ✅ | Pipes stdout/stderr, `next_line()` loop, parses `ROKO_PROGRESS:` → `ToolCallStart`/`ToolCallComplete`/agent events (3975-4038) |
| Session persistence + 7-day GC + cross-restart resume | `src/session.rs:1082-1186` | ✅ | `.roko/sessions/sess_*.json` |
| Conversation history (FIFO turns, messages array, CLI history) | `src/session.rs:604-688` | ✅ | `push_user_turn`/`build_messages_array` |
| 9 config options (model/effort/temperament/routing/clippy/tests/workflow/review/retries) | `src/session.rs:145-268,1222-1430` | ✅ | `SessionConfigState` + `build_config_options` |
| Config hot-reload + `server/config_sources_update` push | `src/config_watch.rs`, `src/handler.rs:166-220` | ✅ | notify watcher; per-session refresh |
| Slash commands (53) + bare-mode filtering | `src/session.rs:1490-1786`, `src/bridge_events.rs:3352-3830` | ✅ | 53 `slash_command(` defs; exec via roko CLI subprocess |
| Provider dispatch: Anthropic API / Claude CLI / OpenAI-compat (zhipu, moonshot, gemini, ollama) | `src/bridge_events.rs:1440-1470,1671-2626` | ✅ | AnthropicApi → dedicated path (1444); all others → openai-compat/ModelCallService (1461) |
| Builtin tool loop (8 tools: read/write/edit/glob/grep/bash/ls/web_fetch) w/ path jail | `src/builtin_tools.rs:18-810` | ⚠️ | `resolve_path` jail ✅ (217-268); **permission gate NOT invoked** — see Drift D1 |
| MCP server passthrough per session | `src/bridge_events.rs:2272-2350,2635` | ⚠️ | `setup_session_mcp_tools` wired **only to openai-compat** (2313); **not** Anthropic loop nor Claude-CLI — see Drift D2 |
| File/@-mention context injection (`includeContext`, resource blocks, git diff) | `src/bridge_events.rs:4410-4634` | ✅ | `resolve_context_items`, `extract_at_mentions`, branch-diff |
| Knowledge + playbook cards + provenance chain | `src/knowledge.rs:65-411`, `src/bridge_events.rs:2984-3350` | ✅ | neuro+playbook joint query per dispatch |
| Mode-specific prompts + safety layers (code/plan/research) | `src/session.rs:541`, `src/runner.rs:936-975` | ✅ | `build_system_prompt`; `safety_layer_for_mode`; SP-1 fail-closed (bridge_events.rs:1279-1283) |
| Pipeline templates express/standard/full/auto + state machine | `src/pipeline.rs:97-405`, `src/workflow.rs` | ✅ | `auto_select` keyword rules |
| WorkflowEngine path (canonical) + AcpAdapter event bridge | `src/runner.rs:444-510`, `src/acp_adapter.rs` | ✅ | `run_with_workflow_engine` → `roko_runtime::workflow_engine` |
| Legacy pipeline runner (gates, autopsy, autofix, review, commit) | `src/runner.rs:977-2210` | 🕰️ | Kept behind `ROKO_ACP_LEGACY` env; ~1.2K LOC duplicated |
| roko-gate integration + adaptive thresholds | `src/runner.rs:1866-1966` | ✅ | `.roko/learn/gate-thresholds.json` EMA (`observe` at 1905/1922/1944) |
| Multi-role review (architect + auditor for "thorough") | `src/runner.rs:1713-1807` | ✅ | Closed since 09-STATUS |
| Episode + efficiency + cascade-router **observation** | `src/bridge_events.rs:298-722,1614` | ✅ | `append_acp_episode`; `emit_acp_efficiency_event`; `record_cascade_observation` (685-722) |
| Cascade-router **selection** (`select_model`) | — | ❌ | **Never called**; dispatch always uses `session.model` — see Drift D3 |
| DaimonState modulation | `src/bridge_events.rs:634` | ❌ | `DaimonPolicy::default()` hardcoded in routing context — see Drift D4 |
| Prompt experiments (A/B) | — | ❌ | Zero production use; `experiments` only in a help string (session.rs:1513) — see Drift D5 |
| Dream consolidation trigger (≥10 episodes) | `src/bridge_events.rs:447-525` | ✅ | `maybe_spawn_dream_consolidation` |
| Event forward to control plane (`ROKO_SERVE_URL`) | `src/event_forward.rs:13-44` | ✅ | CognitiveEvent → RuntimeEvent → `HttpEventSink` |
| Outbound `fs/read_text_file`/`fs/write_text_file` to client | transport registry only | 🔌 | Registry exists (transport.rs:51) but bridge reads workdir directly |
| Client terminal capability (`terminal/*`) | capability parsed only | ❌ | `ClientCapabilities.terminal` parsed (types.rs:136); no terminal calls |
| Image/audio prompt input | see Drift D6 | ⚠️ | Advertises `image:false,audio:false` (handler.rs:290-291) yet image content parts built for both Anthropic (bridge_events.rs:1223) and OpenAI (4366-4370) — **under-advertised** |
| CLI entry `roko acp` | `crates/roko-cli/src/main.rs:681-698,2016-2049` | ✅ | Early-exit before tracing init; duplicate unreachable arm at 2573-2589 |
| Tests | `tests/` + inline | ✅ | ~90 test fns; 11 conformance (protocol_conformance.rs); 5 telemetry-integration |

## Drift catalog (verified against HEAD 5852c93c0)

### D1 — Builtin tool loop has NO permission gate [P0] (feedback #18 — VALID for in-process loop)
`execute_acp_builtin_tool` (`builtin_tools.rs:269-322`) runs `write_file`/`edit_file`/`bash` **unconditionally** (match at 291-301). It emits `ToolCallStart` with a `ToolCallKind` and the doc-comment claims "the ACP layer can gate on it" (builtin_tools.rs:266-268) — **but no layer does.**
- `tool_needs_permission` (builtin_tools.rs:326) and `needs_permission` (173) have **zero callers** outside this file.
- `request_permission` (bridge_events.rs:768 — a real, fail-closed, AlwaysAllow-persisting `session/request_permission` round-trip) has **zero production callers**: the only call sites are three unit tests (5167/5202/5236).
- All three tool-execution sites route through `execute_acp_builtin_tool` ungated: Anthropic loop (1738), openai-compat loop (2445), MCP ToolHandler wrapper (2911-2927).
**Impact:** an editor-driven Anthropic-API or openai-compat session can write/overwrite files and run bash with no confirmation. The machinery to fix this already exists; it is simply not wired into the loop. **The 2026-07-07 audit marked this ✅ — that was wrong.** (The path-jail `resolve_path` at builtin_tools.rs:217 still holds, so writes are workdir-scoped; but there is no per-action user consent.)

### D2 — MCP session tools wired to openai-compat only [P1] (wave-1 — VALID)
`setup_session_mcp_tools` (bridge_events.rs:2635) is invoked **only** from `run_openai_compat_mcp_tool_loop` (2313). The Anthropic path (`run_anthropic_builtin_tool_loop`, 1746) builds only `acp_builtin_tools()` and never registers session MCP servers; the Claude-CLI subprocess path has no in-band MCP either. So a session that declares `mcpServers` gets MCP tools **only if** the resolved provider is openai-compat and `openai_compat_tool_loop_supported` (2264) returns true. Validation at `session/new` (session.rs:904-926) accepts the servers regardless, so this fails silently for Anthropic users.

### D3 — CascadeRouter selection never runs in ACP [P1] (feedback #19A/34A — VALID)
`record_cascade_observation` (bridge_events.rs:685-722, called at 1614) records the *outcome* correctly, but `CascadeRouter::select_model` is never called — dispatch resolves the model from `session.model`/config only. So ACP *feeds* the bandit but never *reads* it: every interaction uses the fixed session model. Half the learning loop (feedback #19 "50%+ of usage gets zero adaptive routing").
- Sub-issue #19B (observation key mismatch): partially mitigated — `record_cascade_observation` checks the model is in `model_slugs` and *skips* (logs "model not in router arms", 705) rather than silently mis-recording; the arms come from `cascade_router_model_slugs` (655). Not a normalization fix, but no longer silent corruption.

### D4 — DaimonState always default() in ACP [P1] (feedback #19C/34B — VALID)
`acp_routing_context` hardcodes `daimon_policy: DaimonPolicy::default()` (bridge_events.rs:634). No load from `.roko/state/daimon.json` or recent episodes. Affect-based modulation (model strength, turn limits, tool policy) never fires in ACP.

### D5 — Prompt experiments (A/B) never consulted in ACP [P1] (feedback #34C — VALID)
`roko-learn` `ExperimentStore` is not referenced in any ACP dispatch path. `experiments` appears only in a slash-command help string (session.rs:1513). CLI path selects a variant per task; ACP does not.

### D6 — image:false advertised but image parts built [P2] (both audits agree — VALID)
`handler.rs:290-291` advertises `image:false, audio:false`, yet `bridge_events.rs:1223-1233` (Anthropic) and `4354-4408` (openai `image_url` data-URI) construct image content blocks from prompt input. Either advertise `image:true` (and test) or strip the image-building code. Editors reading the capability will never send images, so the build path is currently dead.

### D7 — Slash-command subprocess path has no permission gate [P2] (feedback #18, secondary)
Distinct from D1: slash commands spawn `roko <cmd>` as a subprocess (bridge_events.rs:3926). Destructive tools there run under the *CLI's* own safety layer, not ACP's. Feedback #18's per-command denylist idea (`/research` should not get bash) is unaddressed but lower-severity since it's the CLI's concern.

### D8 — Streaming: feedback #32 is now largely STALE [resolve/close]
#32 claimed slash commands buffer via `.output()`. Current code streams: subprocess stdout/stderr read line-by-line (3953-4160) with structured `ROKO_PROGRESS:` → `ToolCall*` translation. The only remaining `.output()` calls are git helpers for @-mentions (4520-4546) — appropriate to buffer. **Close #32 except:** tool calls *inside* a Claude-CLI subprocess are still opaque unless the subprocess emits `ROKO_PROGRESS` markers (#32 part C — partial).

## Guide coverage (ACP-INTEGRATION-GUIDE + acp-features vs code)

**Guide → code: ~97% accurate.** Methods, notification variants, config options, workflow templates/auto-select, error codes (types.rs:12-30), knowledge/episode sections, and the five Known Limitations (guide:2156-2177) all verify. Divergences: (1) code adds `session/close`/`session/resume` (guide omits); (2) the image mismatch (D6); (3) the guide does **not** flag the orphaned permission gate (D1) — the guide implies gating works.

**acp-features (2026-05-01) → code: stale-pessimistic.** Its "not started" list (conversation accumulation, persistence, knowledge prompts, MCP passthrough, file-context, mode prompts, cascade routing, file-change) is mostly ✅. Still genuinely missing: budget-limit / context-limit / auto-escalate / provider-selector config options; marketplace/trigger/visual-authoring (features §9b-9d).

**tmp-feedback/2 (2026-05-16) → code:** #18 partly VALID (D1 in-process loop is the real hole; slash path D7 minor). #19 VALID (D3/D4). #32 mostly STALE (D8). #34 VALID for router/daimon/experiments (D3/D4/D5); its "distillation/auto-dream/efficiency now wired" claims verified ✅.

## Cross-cutting drift for the navigation layer
- **ACP is the only editor surface yet lives outside the v2 Surfaces taxonomy** (`20-SURFACES.md:3`). It should be formalized as a Surface or explicitly noted as an integration guide, so nav docs don't imply CLI/TUI/Dashboard are the whole story.
- **Learning-parity is asymmetric.** CLI orchestrate.rs is the reference for cascade-selection + daimon + experiments; ACP mirrors *telemetry* (episodes/efficiency/observation/gate-thresholds/dream) but not *decisioning* (D3/D4/D5). Any nav claim that "learning is wired end-to-end" is CLI-only.
- **Permission model divergence.** CLI agents run in Claude-CLI subprocesses with their own permission system; ACP's in-process loop (D1) does not. The safety story differs by surface — nav/security docs must not treat "SafetyLayer + path-jail" as equivalent to per-action consent.
- **`CursorAcpAdapter`** in roko-agent (`provider/cursor_acp.rs:14-15`) is an HTTP-fallback *provider*, unrelated to this crate despite the name — a naming collision the nav layer should disambiguate.

## Old paradigm & tech debt
- 🕰️ Legacy pipeline runner (`run_workflow_pipeline`, runner.rs:977) behind `ROKO_ACP_LEGACY`; ~1.2K LOC duplicating WorkflowEngine phases.
- 🕰️ Stale docs: `tmp/acp-features/`, `tmp/acp-runner/09-STATUS.md` undercount; guide file-inventory LOC ~10x under actual.
- Dead duplicate `Command::Acp` match arm (main.rs:2573-2589), unreachable after early exit at 2020.
- Single-transport: `SessionManager` not `Arc<RwLock>`; no concurrent editor clients.
- Local test residue: `crates/roko-acp/.roko/sessions/*.json` (gitignored, noisy).
- CLAUDE.md crate table omits roko-acp.

## Not implemented
- Per-action permission consent in the in-process loop (D1); client-side FS bridge (`fs/read_text_file`/`fs/write_text_file`) — unsaved buffers invisible.
- Terminal bridge (`terminal/*`), auth methods (`auth_methods: []`, handler.rs:299), image/audio capability advertisement (D6).
- Cascade *selection* (D3), DaimonState load (D4), prompt experiments (D5) in ACP.
- MCP for Anthropic / Claude-CLI providers (D2).
- Budget-limit / context-limit / auto-escalate / provider-selector config options.
- Trigger system, marketplace, visual-authoring via ACP.
- UX34 override learning: cascade observations recorded for every dispatch but no distinct manual-override signal.

## Ordered roadmap
1. **[P0] Wire the permission gate into the builtin tool loop (D1).** Before `execute_acp_builtin_tool` runs a tool where `tool_needs_permission(name)`, call `request_permission` (already built) and honor `Reject`. Verify: conformance test asserting an outbound `session/request_permission` before a `write_file`, and that a `Reject` blocks execution.
2. **[P0] Re-verify boot + handshake after ~2mo of refactors** (last live use 2026-05-09). Verify: `printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1}}\n' | cargo run -p roko-cli -- acp --workdir . | head -1`
3. **[P0] Run the crate test suite.** Verify: `cargo test -p roko-acp`.
4. **[P1] Wire MCP session tools into the Anthropic loop (D2)** (mirror the openai-compat `setup_session_mcp_tools` call in `run_anthropic_builtin_tool_loop`), or reject `mcpServers` at `session/new` for non-openai providers with a clear error.
5. **[P1] Wire CascadeRouter selection (D3):** call `select_model` before dispatch when `routing_mode != manual`; keep `record_cascade_observation`. Verify: dispatch model differs from `session.model` when router prefers another arm.
6. **[P1] Load DaimonState (D4)** from `.roko/state/daimon.json` (or recent episodes) into `acp_routing_context` instead of `default()`.
7. **[P1] Consult ExperimentStore (D5)** for a prompt variant per role before dispatch.
8. **[P1] Reconcile `image:false` (D6):** advertise `image:true` + add a conformance test, or delete the image content-part builders.
9. **[P1] Add roko-acp row to CLAUDE.md crate table.** Verify: `grep roko-acp CLAUDE.md`.
10. **[P2] Wire outbound `fs/read_text_file`/`fs/write_text_file`** so unsaved editor buffers reach the agent.
11. **[P2] Per-slash-command tool denylist (D7).**
12. **[P2] Delete or date-stamp the legacy runner** (`ROKO_ACP_LEGACY`) and remove the unreachable `Command::Acp` arm (main.rs:2573-2589).
13. **[P2] Document `session/close`/`session/resume` in guide §4.**
14. **[P3] Formalize ACP in `20-SURFACES.md`;** refresh `tmp/acp-features` counts; disambiguate `CursorAcpAdapter`.

## Migration checklist
- [ ] **[P0]** Wire `request_permission` into `execute_acp_builtin_tool` for write/edit/bash — verify: outbound `session/request_permission` before a write in a conformance test.
- [ ] **[P0]** Re-verify ACP boot + handshake — verify: `initialize` roundtrip pipe (roadmap #2).
- [ ] **[P0]** `cargo test -p roko-acp`.
- [ ] **[P1]** MCP for Anthropic/Claude-CLI providers or reject non-openai `mcpServers` — verify: session/new error or a Claude-path MCP tool call.
- [ ] **[P1]** Call `CascadeRouter::select_model` in ACP dispatch — verify: selected model can differ from `session.model`.
- [ ] **[P1]** Load DaimonState (not `default()`) — verify: `grep -n 'DaimonPolicy::default' crates/roko-acp/src/bridge_events.rs` (0 hits after fix).
- [ ] **[P1]** Consult ExperimentStore in ACP dispatch — verify: variant selection logged per role.
- [ ] **[P1]** Reconcile `image:false` — verify: `grep -n 'image: false' crates/roko-acp/src/handler.rs`.
- [ ] **[P1]** Add roko-acp row to CLAUDE.md — verify: `grep roko-acp CLAUDE.md`.
- [ ] **[P2]** Outbound `fs/read_text_file`/`fs/write_text_file` — verify: conformance test.
- [ ] **[P2]** Per-command tool denylist.
- [ ] **[P2]** Remove `ROKO_ACP_LEGACY` path + unreachable `Command::Acp` arm — verify: `grep -rn ROKO_ACP_LEGACY crates/`.
- [ ] **[P3]** Document `session/close`/`session/resume`; refresh acp-features; formalize in `20-SURFACES.md`.

## Open questions
1. Is ACP still in active daily use? `.roko/acp.log` stops 2026-05-09. If abandoned, P1 items lose urgency but D1 (P0 safety) still matters before any relaunch.
2. Which editors are validated? 09-STATUS says Zed; Cursor config documented (features:270-291) but no session evidence.
3. Should the orphaned `request_permission` machinery (built, tested, unused) be treated as a regression from a wiring that was later removed, or as never-finished scaffolding? (History check on runner.rs:1763 / the removed `dangerously_skip_permissions` hardcode is relevant.)
4. Does per-dispatch cascade *observation* under `routing_mode=manual` count as UX34 override-learning, or is a separate override signal needed (CLAUDE.md item 15)?
5. Formalize ACP as a v2 Surface, or keep as integration guide?
