# Verification Notes (2026-05-01)

This is the audit log that produced this runner's batch list. It records, for every item in `tmp/binary-issues/MASTER-INDEX.md`, what was verified at HEAD and how the verdict mapped to a batch decision.

Status legend (see `MASTER-INDEX.md` original):

- **FIXED** — code clearly addresses the original symptom; no batch needed.
- **PARTIAL** — partial progress; a batch finishes the job (or the item is judged "good enough").
- **OBSOLETE** — only present in `legacy-orchestrate`; default build is unaffected; no batch.
- **OPEN** — symptom still present in default build; batch defined.
- **UNCLEAR** — re-verification needed; conservatively kept open.

## S1 — Dispatch as agent session

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S1.1 | PARTIAL | — | `chat_session.rs:575-577` sends system prompt; legacy `dispatch_direct.rs` does not (legacy-gated, OBSOLETE). |
| S1.2 | FIXED | — | `/system` mutates both `session.system_message` and `agent_session.system_prompt` (`chat_inline.rs:2672-2689`). |
| S1.3 | PARTIAL | **BI_54** | `ModelCallRequest` has no `tools` field. Tools only flow via Claude CLI args. |
| S1.4 | FIXED | — | `api_history` accumulated and merged each turn. |
| S1.5 | FIXED | — | `build_chat_system_prompt` includes cwd + workspace. |
| S1.6 | OBSOLETE | — | "Single-shot…" string lives in legacy `build_system_prompt`. |
| S1.7 | OPEN | **BI_16** | `knowledge_ids: Vec::new()` is hardcoded at `chat_session.rs:585`. |
| S1.8 | FIXED | — | `send_turn_api` chains `with_mcp_config`. |
| S1.9 | OBSOLETE | — | TODO in legacy `dispatch_agent`. |
| S1.10 | PARTIAL | — | `unified.rs` `config` is mainly used for serve; chat reloads layered config. Acceptable. |

## S2 — HTTP client reuse

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S2.1 | OBSOLETE | — | Legacy `dispatch_direct.rs:313`. |
| S2.2 | OBSOLETE | — | Legacy `dispatch_direct.rs:231`. |
| S2.3 | FIXED | — | Uses `shared_http_client()`. |
| S2.4 | OPEN | **BI_55** | `ProviderCallCell::execute` calls `create_agent_for_model` per call. |
| S2.5 | OBSOLETE | — | Pattern no longer present. |
| S2.6/S2.7 | PARTIAL | — | `connect_timeout(10)` set; `ttft_timeout_ms` synthesized. Acceptable. |

## S3 — Confirmation theater

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S3.1 | FIXED | — | `/system` wired. |
| S3.2 | FIXED | — | `/effort` wired. |
| S3.3 | OPEN | **BI_17** | `chat_inline.rs:2980-2986` only prints "gate toggle" string. |
| S3.4 | OPEN | **BI_18** | `chat_inline.rs:3124-3143` only echoes "set k = v". |
| S3.5a-d | OPEN | **BI_19..BI_22** | `/run` (`3390`), `/plan run` (`3427`), `/prd idea` (`3457`), `/research` (`3488`) all print "run in terminal". |
| S3.6 | OPEN | **BI_23** | `commands/learn.rs:61-108` `cmd_tune("gates")` only reads + pretty-prints; never writes. |
| S3.7 | FIXED | — | Demo speed wired through `setSpeedMultiplier` → `adjustedSleep`. |
| S3.8 | FIXED | — | `useServerHealth` correctly sets `disconnected` on `!res.ok`. |

## S4 — Silent error swallowing

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S4.1 | OBSOLETE | — | 47 `.ok()` in legacy `orchestrate.rs`. |
| S4.2 | OPEN | **BI_32** | `jsonl_logger.rs:89-92` `let _ = self.write_event(event)`. |
| S4.3 | OPEN | **BI_33** | `workflow_engine.rs:590-593` `let _ = policy.persist().await`. |
| S4.4 | OPEN | **BI_34** | `unified.rs:172-176` only `tracing::warn!`. |
| S4.5 | OBSOLETE | — | Lives in legacy `run_once`. |
| S4.6 | PARTIAL | — | `config.rs:2285-2295` warns + substitutes empty. Acceptable; could be revisited later. |
| S4.7 | OPEN | **BI_35** | `roko-serve/src/terminal.rs:589-592` `let _ = state.terminal_sessions.send_input(...)`. |
| S4.8 | OPEN | **BI_36** | `fswatcher.rs:27` JoinHandle dropped. |
| S4.9 | PARTIAL | — | `tracing::debug!` on absent sink. Acceptable. |
| S4.10 | OPEN | **BI_37** | FIXME at `roko-serve/src/lib.rs:1448-1449` for double REST event delivery. |

## S5 — Security off-by-default

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S5.1 | **FIXED** | — | `ServeAuthConfig::default { enabled: true }` at `roko-core/src/config/serve.rs:83-94`. The original audit's reading was wrong. |
| S5.2 | PARTIAL | — | `routes/mod.rs:81-84,145-154` applies `require_api_key` on non-loopback unless ack'd. Acceptable. |
| S5.3 | FIXED | — | `validate_bind_safety` rejects public binds without auth (`roko-serve/src/lib.rs:250,641-659`). |
| S5.4 | PARTIAL | — | `cors_layer` uses localhost predicate; `permissive` only when `unsafe_public_cors`. |
| S5.5 | OPEN | **BI_01** | `share.rs:156` still passes `--public` to `gh gist create`. |
| S5.6 | OPEN | **BI_02** | `roko-serve/src/terminal.rs:240-259` accepts arbitrary `program` from request. |
| S5.7 | OPEN | **BI_03** | No session cap, no idle TTL on `SessionManager`. |
| S5.8 | OPEN | **BI_04** | Hardcoded `true` at `claude_cli_agent.rs:128`, `runner/types.rs:1391`, `agent_exec.rs:147`, `commands/plan.rs:402`, `roko-acp/runner.rs` (search). |
| S5.9 | OPEN | **BI_05** | `safety/mod.rs:708-717` records secret-scrub mismatch as `ViolationSeverity::Warn`. |
| S5.10 | OPEN | **BI_06** | `AgentContract::permissive` is `pub` at `safety/contract.rs:91`. |
| S5.11 | OPEN | **BI_07** | Implementer YAML forbids `network`/`fetch` but no shell-tool sandbox enforcement. |

## S6 — Streaming

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S6.1 | PARTIAL | **BI_56** | Session mode streams (`chat_inline.rs:1614`); HTTP mode (`4307-4310`) does not. |
| S6.2 | FIXED | — | `StreamingState.append` invoked from event loop. |
| S6.3 | OBSOLETE | — | Legacy. |
| S6.4 | OBSOLETE | — | Legacy. |
| S6.5 | OPEN | **BI_24** | `WorkflowEngine.emit`s exist but `print_workflow_run_report` is post-hoc only. |
| S6.6 | OPEN | **BI_25** | `roko run` v2 returns the report and prints once at end (`run.rs:646-707`). |

## S7 — Hardcoded values

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S7.1 | OBSOLETE | — | Legacy `dispatch_direct.rs`. |
| S7.2 | OBSOLETE | — | Legacy. |
| S7.3 | OPEN | **BI_38** | `roko-core/src/config/presets.rs:126-143`, `routing.rs:163`, dreams/CLI/TUI helpers. |
| S7.4 | OPEN | **BI_39** | `claude_agent.rs:29` `DEFAULT_BASE_URL`, `provider.rs:212`. |
| S7.5 | OPEN | **BI_40** | `claude_agent.rs:32` `DEFAULT_ANTHROPIC_VERSION`. |
| S7.6 | PARTIAL | **BI_41** | `roko-compose/src/enrichment/pipeline.rs:367`, `roko-agent/src/lifecycle.rs:359`. |
| S7.7 | OPEN | **BI_42** | `naive_opus_cost` at `chat_inline.rs:4240-4241` uses `15.0` / `75.0`. |
| S7.8 | PARTIAL | **BI_43** | Small fixed list in `cost_table.rs:136-156`; unknowns return 0. |
| S7.9 | OPEN | **BI_44** | `roko-std/src/tool/builtin/web_search.rs:32-35`. |
| S7.10 | OPEN | **BI_45** | `roko-agent/src/process/registry.rs:24-27` uses `current_dir()`. |
| S7.11 | FIXED | — | Now uses `&report.model`. |

## S8 — Phantom features

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S8.1 | OPEN | **BI_08** | `episode_logger.rs:1086`+ defined; only test callers. |
| S8.2 | PARTIAL | **BI_09** | `commands/plan.rs:394-396` attaches sink without runner. |
| S8.3 | OPEN | **BI_10** | `model_router.rs:1157` `LinUCBRouter::save` no production caller; cascade snapshot misses arms. |
| S8.4 | OPEN | **BI_11** | `roko-compose/src/prompt.rs:868-897` greedy dominates unless explicit Vcg. |
| S8.5 | OPEN | **BI_12** | `safety/contract.rs:471-482` per-call only; cumulative TODO. |
| S8.6 | UNCLEAR | — | Cited line is now `RunGates`; review path uses `request_review_revision`. May be OBSOLETE. Skip. |
| S8.7 | OPEN | **BI_13** | TODO(converge) at `main.rs:2207-2210`. |
| S8.8 | PARTIAL | **BI_14** | `shared_runs.rs:180-198,321-457` enriched but edge cases remain. |
| S8.9 | OPEN | **BI_15** | `Share.tsx:103` hits `/api/share/${token}`; server registers `/api/shared/{token}`. |

## S9 — Subprocess management

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S9.1 | OPEN | **BI_26** | `auth_detect.rs:70-75` `Command::output()` no timeout. |
| S9.2 | OPEN | **BI_27** | `roko-agent/src/mcp/client.rs:167-187` `.stderr(Stdio::inherit())`. |
| S9.3 | OBSOLETE | — | Legacy. |
| S9.4 | PARTIAL | **BI_28** | No retained `AbortHandle` for spawned dispatch (`chat_inline.rs:1814-1823`). |
| S9.5 | OPEN | **BI_28** | No `CancellationToken` in chat loop. |
| S9.6 | PARTIAL | **BI_29** | Stderr to log file fixed; handle still discarded (`roko-serve/src/lib.rs:305-346`). |
| S9.7 | OPEN | **BI_30** | `unified.rs:49-74` blocks chat boot. |
| S9.8 | OPEN | **BI_31** | 15 bare `eprintln!` in `claude_cli_agent.rs`. |
| S9.9 | OPEN | **BI_31** | `main.rs:2314-2343` raw `eprintln!` before `should_use_inline()` checks. |

## S10 — Duplicate code paths

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S10.1 | OBSOLETE | — | `orchestrate.rs` legacy-gated. |
| S10.2 | OPEN | **BI_46** | Two large parallel event loops in `chat_inline.rs` (~1161 and ~1540). |
| S10.3 | OBSOLETE | — | Legacy. |
| S10.4 | OPEN | **BI_47** | Two render sites for session summary (~1101-1122, ~1483-1504). |
| S10.5 | OPEN | **BI_48** | `chat.rs` and `chat_inline.rs` both exist; `chat.rs` referenced from `chat_inline.rs:28`. |
| S10.6 | OPEN | **BI_49** | `cmd_init` (`commands/util.rs:104`) vs `run_init_wizard` (`config_cmd.rs:47-55`). |

## S11 — Mutex / unwrap

| ID | Status | Batch | Notes |
|----|--------|-------|-------|
| S11.1 | OPEN | **BI_50** | `dispatcher/mod.rs:779,786-787` `expect("audit signals lock")`. |
| S11.2 | OPEN | **BI_51** | `model_call_service.rs:921-960` LRU cache `expect`s. |
| S11.3 | OBSOLETE | — | Legacy. |
| S11.4 | OPEN | **BI_52** | `routes/feeds.rs:125-127` `expect("just registered")`. |
| S11.5 | OPEN | **BI_53** | `roko-agent/src/lib.rs:22-42` crate-level lint allows. |
