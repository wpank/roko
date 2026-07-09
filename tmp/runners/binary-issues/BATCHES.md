# Binary Issues — Batch Index

Runner: `tmp/runners/binary-issues/run.sh`  
Tracker: `ISSUE-TRACKER.md`  
Source audit: `tmp/binary-issues/MASTER-INDEX.md`

## All batches (56)

| ID | Group | Title | Deps |
|----|-------|-------|------|
| BI_01 | BI_SEC | Default share to --secret gist with scrubbed payload | — |
| BI_02 | BI_SEC | Add command allowlist to /api/terminal/sessions | — |
| BI_03 | BI_SEC | Cap PTY session count and apply idle TTL | — |
| BI_04 | BI_SEC | Default dangerously_skip_permissions to false at every hardcoded site | — |
| BI_05 | BI_SEC | Promote secret-leak / forbidden-write violations to Block severity | — |
| BI_06 | BI_SEC | cfg(test)-gate AgentContract::permissive | — |
| BI_07 | BI_SEC | Audit and document implementer Python sandbox bypass risk | — |
| BI_08 | BI_PHN | Auto-trigger EpisodeLogger::compact on session start / N episodes | — |
| BI_09 | BI_PHN | Wire DreamTriggerSink::with_runner and consume dream_triggers.jsonl | — |
| BI_10 | BI_PHN | Persist LinUCB arm matrices via CascadeSnapshot | — |
| BI_11 | BI_PHN | Activate VCG strategy when budget pressure crosses threshold | — |
| BI_12 | BI_PHN | Enforce cumulative MaxCostPerTurn across tool calls | — |
| BI_13 | BI_PHN | Share single SharedStateHub across TUI and serve | — |
| BI_14 | BI_PHN | Harden create_share empty-transcript edge cases | — |
| BI_15 | BI_PHN | Fix Share.tsx endpoint mismatch (/api/share -> /api/shared) | — |
| BI_16 | BI_PHN | Query knowledge store and populate knowledge_ids in chat path | — |
| BI_17 | BI_CMD | `/gate <name> on|off toggles runtime config` | — |
| BI_18 | BI_CMD | `/config set writes to runtime overlay (and optionally roko.toml)` | — |
| BI_19 | BI_CMD | `/run <prompt> executes inline via WorkflowEngine` | BI_24 |
| BI_20 | BI_CMD | `/plan run <dir> executes inline` | BI_24 |
| BI_21 | BI_CMD | `/prd idea <text> writes idea + opens flow` | — |
| BI_22 | BI_CMD | `/research <query> runs research backend inline` | — |
| BI_23 | BI_CMD | `roko learn tune gates --dry-run actually applies (or removes the flag)` | — |
| BI_24 | BI_STR | Forward WorkflowEngine lifecycle events to terminal during plan run | — |
| BI_25 | BI_STR | Stream incremental output for roko run v2 | BI_24 |
| BI_26 | BI_SUB | Add 3s timeout to claude --version probe in auth_detect | — |
| BI_27 | BI_SUB | Capture MCP server stderr to log file (no Stdio::inherit) | — |
| BI_28 | BI_SUB | Thread CancellationToken into chat dispatch (Ctrl+C cancels) | — |
| BI_29 | BI_SUB | Store chain-watcher join handle and abort on shutdown | — |
| BI_30 | BI_SUB | Non-blocking background-serve startup | — |
| BI_31 | BI_SUB | Replace bare eprintln! in claude_cli_agent.rs and guard main.rs | — |
| BI_32 | BI_ERR | Log JSONL write/flush errors at warn instead of let _ = | — |
| BI_33 | BI_ERR | Log AffectPolicy::persist failures (warn + counter) | — |
| BI_34 | BI_ERR | Surface background-serve failure to user | — |
| BI_35 | BI_ERR | Return PTY send_input errors to WS client | — |
| BI_36 | BI_ERR | Store fswatcher join handle, log spawn failures | — |
| BI_37 | BI_ERR | Fix double REST event delivery FIXME on EventBus | — |
| BI_38 | BI_HRD | Consolidate claude-opus-4-6 literals into one preset key | — |
| BI_39 | BI_HRD | Anthropic base URL through provider config | — |
| BI_40 | BI_HRD | Anthropic API version through provider config | — |
| BI_41 | BI_HRD | Consolidate 8192 max_tokens into per-role / per-provider config | — |
| BI_42 | BI_HRD | Replace naive_opus_cost $15/$75 with CostTable::lookup | — |
| BI_43 | BI_HRD | CostTable loaded from config with hardcoded fallbacks | BI_42 |
| BI_44 | BI_HRD | Perplexity URL/model through web_search config | — |
| BI_45 | BI_HRD | PID file path uses .roko discovery (not current_dir) | — |
| BI_46 | BI_COD | Extract shared chat event-loop body (HTTP + Session) | — |
| BI_47 | BI_COD | Extract render_session_summary() helper | — |
| BI_48 | BI_COD | Remove or merge legacy chat.rs into chat_inline.rs | — |
| BI_49 | BI_COD | Unify roko init and roko config init | — |
| BI_50 | BI_MTX | Switch dispatcher audit-signals lock to parking_lot::Mutex | — |
| BI_51 | BI_MTX | LRU/cache mutexes in model_call_service use parking_lot | — |
| BI_52 | BI_MTX | Replace expect(just registered) with ok_or in routes/feeds.rs | — |
| BI_53 | BI_MTX | Remove roko-agent crate-level lint suppressions, fix call sites | — |
| BI_54 | BI_PRT | Add typed tools field to ModelCallRequest | — |
| BI_55 | BI_PRT | Cache and reuse provider agent in ProviderCallCell::execute | — |
| BI_56 | BI_PRT | Stream HTTP DispatchMode::Http deltas (parity with Session mode) | — |

## Dependency graph (only edges)

```
BI_24 → BI_19, BI_20, BI_25
BI_42 → BI_43
```

Everything else is wave-1 parallel (DAG depth 2 total).

## Wave schedule (from `run.sh --dry-run`)

- **Wave 1**: All batches except BI_19, BI_20, BI_25, BI_43 (52 batches, 16 concurrent).
- **Wave 2**: BI_19, BI_20, BI_25, BI_43 (4 batches).

## Commands

```bash
bash tmp/runners/binary-issues/run.sh --list
bash tmp/runners/binary-issues/run.sh --dry-run
bash tmp/runners/binary-issues/run.sh --group BI_SEC
bash tmp/runners/binary-issues/run.sh --only BI_15,BI_24
```
