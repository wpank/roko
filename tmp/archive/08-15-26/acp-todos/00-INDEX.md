# ACP TODO Collection -- 2026-08-15

> This folder consolidates all known ACP (Agent Client Protocol) work items
> identified during the 2026-08-15 audit of the roko-acp crate (19,915 LOC).

## Reference Locations (do not move these)

| What | Path | Status |
|---|---|---|
| ACP crate source | `crates/roko-acp/src/` | Active codebase (15 modules, 19,915 LOC + 1,249 LOC tests) |
| ACP features checklist | `tmp/acp-features/00-ACP-FEATURES.md` | Valid but stale (lists 7 modules; actual count is 15) |
| ACP runner (batch builder) | `tmp/acp-runner/` | Historical -- built the crate via 18 Codex batches |
| Gap tracker | `.roko/GAPS.md` | Canonical SSOT -- ACP entries should be synced here |
| Master execution checklist | `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md` | E17 epic -- 5/6 plans done (P28-image-support remaining) |

### Current Module Inventory

| Module | LOC | Role |
|---|---|---|
| `bridge_events.rs` | 8,430 | Provider dispatch + event streaming (god file) |
| `session.rs` | 2,839 | Session manager, config options, slash commands |
| `runner.rs` | 2,494 | Multi-agent workflow pipeline execution |
| `types.rs` | 1,321 | ACP JSON-RPC type definitions |
| `builtin_tools.rs` | 1,144 | Built-in tool implementations |
| `handler.rs` | 670 | Request/notification dispatch |
| `event_forward.rs` | 586 | Event forwarding layer |
| `pipeline.rs` | 538 | Pipeline state machine |
| `config.rs` | 521 | AcpConfig with roko.toml loading |
| `knowledge.rs` | 412 | Knowledge store integration |
| `transport.rs` | 362 | Stdio transport layer |
| `acp_adapter.rs` | 250 | Adapter shim |
| `config_watch.rs` | 167 | File-watch config reload |
| `workflow.rs` | 158 | Workflow definitions |
| `lib.rs` | 23 | Module declarations |

## TODO Documents

| # | Document | Category | Items |
|---|---|---|---|
| 01 | PANIC-AND-ERROR-FIXES.md | P0-P3 error handling in bridge_events.rs | ~N items |
| 02 | OTHER-MODULE-ERROR-FIXES.md | Error handling in all other modules | ~N items |
| 03 | SPEC-VERSION-BUMP.md | Protocol spec v0.12.2 to v0.13.6 | ~N changes |
| 04 | TEST-COVERAGE-GAPS.md | Missing tests per module | ~N tests needed |
| 05 | BRIDGE-EVENTS-REFACTOR.md | Split 8,430-line god file | ~N modules proposed |
| 06 | UNEXECUTED-BATCHES.md | ACP09-18 planned work analysis | 10 batches |
| 07 | CONCURRENCY-ISSUES.md | Race conditions, lock contention | ~N issues |
| 08 | CODE-QUALITY.md | Clippy, dead code, documentation | ~N items |
| 09 | INTEGRATION-GAPS.md | Cross-crate wiring gaps | ~N integrations |
| 10 | EDITOR-COMPATIBILITY.md | Zed/Cursor/JetBrains issues | ~N issues |

## Priority Order

### Immediate (before next Zed test)
1. Fix P0 panics (doc 01)
2. Fix clippy errors (doc 08)
3. Fix race conditions (doc 07)

### Short-term (hardening)
4. Add missing tests (doc 04)
5. Fix P1 silent failures (doc 01, 02)
6. Bump spec version (doc 03)

### Medium-term (architecture)
7. Split bridge_events.rs (doc 05)
8. Wire integration gaps (doc 09)

### Long-term (expansion)
9. Implement unexecuted batch features (doc 06)
10. Editor-specific testing (doc 10)

## Relationship to Other Tracking

- **`.roko/GAPS.md`** -- Should reference this folder for ACP-specific items. Currently
  has 5 resolved ACP entries (compile issues, cascade integration, streaming, mutation
  consent boundary, provider loop closure) and ongoing references in E32/E34.
- **`tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`** -- ACP items tracked at epic level
  (E17). Plans P19/P21/P22/P25 are done; P28-image-support (0/5) is the only open plan.
- **`tmp/acp-features/00-ACP-FEATURES.md`** -- Original feature checklist (2026-08-13).
  Accurate for protocol/slash-command coverage but lists only 7 source files and ~4,330
  LOC. The crate has since grown to 15 modules and 19,915 LOC. This collection supersedes
  it for work-item tracking.
- **`tmp/acp-runner/`** -- Historical batch builder docs (18 Codex batches). The runner
  built phases 1-3 of the crate. Phases 4-6 (multi-task plans, custom workflows, triggers)
  remain not started and should appear in doc 06.

## Context: What Has Been Resolved

These ACP issues were previously open in `.roko/GAPS.md` and are now closed. They
do not need new TODO entries but are listed here for traceability:

| Issue | Resolved | Summary |
|---|---|---|
| roko-acp compile issues | 2026-08-13 | Workspace member compiles on stable toolchain |
| P19 ACP cascade integration | 2026-08-14 | Real dispatch, session precedence, Daimon context, episode metadata |
| P21 ACP streaming | 2026-08-14 | CLI producers, live stdout/stderr bridge, retry correlation, process cancellation |
| ACP mutation consent boundary | 2026-08-15 | `session/request_permission` for write_file/edit_file/bash; AlwaysAllow trust store |
| E17 provider-loop closure | 2026-08-15 | Anthropic/OpenAI tool loops update shared limiter and outcome recorder |

## Notes

- Item counts (~N) are placeholders. Each document is being written concurrently by
  separate agents. Update this index once all 10 documents are finalized.
- The `bridge_events.rs` file at 8,430 lines is the single largest maintenance risk
  in the crate. Doc 05 should propose a concrete decomposition plan.
- The features checklist (`tmp/acp-features/00-ACP-FEATURES.md`) reports 35 slash
  commands (31 implemented, 4 deferred) and 10 feature sections. That inventory is
  still accurate for protocol-level coverage; this collection focuses on implementation
  quality and missing integrations.
