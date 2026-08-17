# IDE Integration — Implementation Plan

**Goal**: Fix all bugs and design gaps found during IDE (Tauri 2 + ACP stdio) testing.
Proper redesigns, not patches. All changes target the `roko` repo — the IDE is NOT modified.

## What is roko acp?

`roko acp` is a JSON-RPC 2.0 stdio server. The Nunchi IDE spawns it as a subprocess and
communicates over stdin/stdout. The IDE sends `session/new` to create sessions, `session/prompt`
to send messages, and receives streaming `session/update` notifications with text chunks,
tool calls, and usage info.

Key crate: `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/` (layer 4, depends on roko-core, roko-agent)

## Test Results (current state)

```
Core:       8 passed, 0 failed
Models:     4 passed, 1 FAILED, 1 warn
MCP:        3 passed, 2 FAILED, 1 warn
Edge:       7 passed, 0 failed, 2 warnings
Total:     22 passed, 3 FAILED, 4 warnings
```

Tests live at: `tmp/solutions/ide/tests/` (FIFO-based harness against `roko acp`)

## Branch Audit Update - 2026-05-05

Status on `wp-arch2`: most IDE/ACP plan items are implemented, but integration results in
this document have not been re-run after the merge.

Implemented:
- IndexMap provider/model ordering across core and dependent crates.
- `session/new` model/provider/effort parameters with warnings.
- Structured MCP status/update types and MCP discovery timeout config.
- Bare-mode slash command categories and filtering.
- Effective `max_output` surfacing and provider `ready` metadata.
- Unknown option, invalid provider, invalid model, and invalid effort validation.
- ACP `--global-config`, `configSources`, missing-workdir warning, and request-driven config
  live reload.
- Effort now reaches provider dispatch through effective provider config / `AgentOptions`.

Remaining gaps:
- `configSources` formatting/order does not exactly match the task spec and can include
  configured paths that do not exist.
- Implicit `~/.roko/config.toml` is not watched unless passed as `--global-config`.
- Config reload is request-driven and does not proactively notify IDE clients.
- `SessionManager.config_sources` is not recomputed after reload.
- Missing-workdir warning should include clearer Zed/global-config guidance.
- The test summary below is stale until `tmp/solutions/ide/tests/run-all.sh` is re-run.

## Issues Being Fixed

| # | Issue | Root Cause | Fix |
|---|-------|-----------|-----|
| 1 | MCP failures silent | 6 `continue` sites swallow errors in setup_session_mcp_tools | Accumulate McpServerStatus, emit structured notification |
| 2 | session/new drops model param | SessionNewParams has no model field | Add model/provider/effort fields, apply in create_session |
| 3 | Non-deterministic defaults | HashMap iteration for models/providers | Replace with IndexMap (preserves TOML order) |
| 4 | MCP timeout hardcoded | 5s constant in defaults.rs | Per-server discovery_timeout_ms in McpServerConfig |
| 5 | bare_mode shows 47 commands | build_slash_commands() is static, no filtering | Add category field, filter by bare_mode |
| 6 | max_output invisible | Default None → 16,384 deep in agent, not surfaced | Add effective_max_output(), show in config options |
| 7 | Provider readiness is strings | "Ready" / "API key not set" — not structured | Add `ready: bool` to ConfigOptionValue |

## Corrected Facts (from code investigation)

| Original Solution Doc Claim | Actual Code Finding |
|---|---|
| "max_output default is 900" | Default is `None`, falls back to `DEFAULT_MAX_OUTPUT_TOKENS = 16,384` in roko-agent dispatch |
| "bare_mode controls workspace features" | `bare_mode` controls `--bare` on Claude CLI (removed). Has NO effect in roko-acp |
| "Discovery timeout is 10s" | 5s per step (defaults.rs:241), 10s worst case per server |
| "MCP tools work for all providers" | Only OpenAiCompat, PerplexityApi, CerebrasApi — Anthropic/ClaudeCli bypass session MCP |

## Architecture of Changes

### Files Modified (by crate)

**roko-core** (kernel):
- `Cargo.toml` — add indexmap dep
- `src/config/schema.rs` — HashMap→IndexMap for providers/models
- `src/config/loader.rs` — diagnostics for low max_output
- `src/config/provider.rs` — effective_max_output() method
- `src/config/registry.rs` — HashMap→IndexMap for profiles

**roko-acp** (ACP server):
- `src/types.rs` — SessionNewParams, SessionNewResult, McpServerStatus, SlashCommand, ConfigOptionValue
- `src/session.rs` — create_session, from_roko_config, build_slash_commands, build_config_options
- `src/handler.rs` — send_slash_commands_notification
- `src/bridge_events.rs` — setup_session_mcp_tools, CognitiveEvent, mcp notification

**roko-agent** — HashMap→IndexMap in dispatch_resolver.rs, provider/mod.rs
**roko-cli** — HashMap→IndexMap in config.rs, model_selection.rs, plan_validate.rs, config_cmd.rs
**roko-learn** — HashMap→IndexMap in cost_table.rs
**roko-serve** — HashMap→IndexMap in routes/providers.rs

### Dependency Graph

```
Group 0: Add indexmap deps to Cargo.tomls
   ↓
Group 1: Change schema.rs core types
   ↓
Group 2: (6 agents in parallel)
   ├── Agent A: roko-agent IndexMap refs
   ├── Agent B: roko-cli IndexMap refs
   ├── Agent C: roko-learn + roko-serve IndexMap refs
   ├── Agent D: SessionNewParams extension (W1-A)
   ├── Agent E: Default fallback logic (W1-B)
   └── Agent F: MCP status types (W2-A)
   ↓
Group 3: (4 agents in parallel)
   ├── Agent G: MCP error accumulation (W2-B, needs F)
   ├── Agent H: Command categories (W3-A)
   ├── Agent I: max_output surfacing (W4-A)
   └── Agent J: Provider readiness (W4-B)
   ↓
Group 4: MCP notification + config (W2-C + W2-D, needs G)
   ↓
Phase 2: cargo fmt + build + clippy + test (1 agent, fix errors)
   ↓
Phase 3: Integration tests (1 agent)
```

## Batch Files

| Batch | File | Agent Group | Est. |
|-------|------|-------------|------|
| W0-A | `batches/W0-A-indexmap-migration.md` | 0 + 1 + 2(A,B,C) | 40m |
| W1-A | `batches/W1-A-session-new-params.md` | 2(D) | 30m |
| W1-B | `batches/W1-B-default-fallback-logic.md` | 2(E) | 20m |
| W2-A | `batches/W2-A-mcp-status-types.md` | 2(F) | 10m |
| W2-B | `batches/W2-B-mcp-error-accumulation.md` | 3(G) | 30m |
| W2-C | `batches/W2-C-mcp-notification.md` | 4 | 30m |
| W2-D | `batches/W2-D-mcp-config-options.md` | 4 | 20m |
| W3-A | `batches/W3-A-command-categories.md` | 3(H) | 40m |
| W4-A | `batches/W4-A-max-output-surfacing.md` | 3(I) | 15m |
| W4-B | `batches/W4-B-provider-readiness.md` | 3(J) | 15m |
| W5-A | `batches/W5-A-config-profiles.md` | deferred | 2-3h |

## Target Test Results (after all fixes applied)

```
Core Protocol:      8 passed, 0 failed
Model & Provider:   5 passed, 0 failed, 1 warn
MCP Integration:    3 passed, 0 failed, 1 warn, 2 skipped
Edge Cases:         6 passed, 0 failed, 1 warn, 2 skipped
Session Lifecycle:  9 passed, 0 failed, 1 warn
Streaming Protocol: 9 passed, 0 failed, 1 warn
Tool Loop:          2 passed, 0 failed, 3 skipped (needs bridge)
Config Options:     8 passed, 0 failed, 0 warned

Total:             50 passed, 0 failed, 5 warned
```

## Current Test Results (before fixes)

```
Total: 44 passed, 3 failed, 5 warned, 11 skipped
Failures: BUG#01 (MCP silent), BUG#02 (model param ignored)
```

## Additional Issues Found (Round 2, not in original plan)

| # | Issue | Severity | Fix |
|---|-------|----------|-----|
| 8 | Unknown optionId silently accepted | Low | Validate in update_config() |
| 9 | Invalid model value silently falls back | Low | Validate against provider models |
| 10 | config/update wire format undocumented | Medium | Document (done in 05-ide-consumer-guide.md) |
| 11 | Chunk shape uses content.text not delta | Medium | Document (done) |
| 12 | No thinking_chunk streaming | Low | Provider limitation or unimplemented |
| 13 | modes field key is availableModes | Low | Document (done) |
| 14 | Unavailable provider still selectable | Low | Related to W4-B (ready field) |

These are documented in `13-new-findings.md`. Issues 8-9 are simple validation fixes
that can be added to session.rs during Phase 2 cleanup.

## Supporting Documents

| File | Purpose |
|------|---------|
| `CHECKLIST.md` | Granular task checklist with parallel groups |
| `AGENT-GUIDE.md` | Execution protocol and agent prompts |
| `00-overview.md` | Original issue overview |
| `05-ide-consumer-guide.md` | ACP protocol reference (updated with correct wire formats) |
| `06-streaming-protocol.md` | Streaming message format |
| `08-test-harness.md` | FIFO-based test scripts |
| `12-test-results.md` | Test results (updated) |
| `13-new-findings.md` | Additional issues from round 2 testing |
