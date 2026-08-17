# Batch Gaps: What's NOT Fully Done

Of 56 batches "executed", only **18 are fully done**. 37 are partially done, 1 was never started.

---

## Fully Done (18 batches) — No remaining work

These are truly complete and can stay archived:

| Batch | Title |
|-------|-------|
| W0-F | Unify roko run Dispatch Paths |
| W2-C | Suppress Config Version Warning |
| W2-E | Fix Negative Cost Display |
| W3-B | Panic Hook for Terminal Restore |
| W7-B | Replace Polling Cancellation with Notify |
| W7-C | Fix PlaybookStore Nested Lock |
| W8-B | Add rust-toolchain.toml |
| W9-B | Cross-Task Output Injection |
| W9-C | Cost Tracking Fix |
| W11-C | Config Reference Validation |
| W11-D | Shell Injection Fix |
| W12-A | Gate Semaphore Per-Run |
| W12-B | Multi-Plan Agent Handle |
| W13-A | TOML Repair Pipeline |
| W13-B | Warm Cache + Dynamic Gate Buffer |
| W13-D | Atomic State Writes |
| W15-A | Prompt Quality Improvements |
| W15-D | Demo App Improvements |

---

## Not Done (1 batch) — Zero implementation

| Batch | Title | Gap |
|-------|-------|-----|
| W8-A | Remove Blanket Clippy Suppression | `#![cfg_attr(clippy, allow(clippy::all, ...))]` still present in `main.rs:10-20`. All 6 checklist items unchecked. |

---

## Dead Code (code written, zero runtime callers)

These are the highest-priority gaps — code exists but is never used:

| Batch | What's Dead | Where | How to Wire |
|-------|-------------|-------|-------------|
| W15-B | `RunOutputSink` trait | `runner/output_sink.rs` | Replace inline `if stream_to_stderr` in agent_events.rs |
| W15-B | `dispatch_and_record()` | `orchestrate.rs:17165` | Either port to v2 runner or delete (legacy path) |
| W15-C | `TimeoutConfig` | `config/timeouts.rs` | Replace hardcoded `Duration::from_secs()` in dispatcher + gates |
| W15-E | `GateRungConfig` / `effective_rungs()` | `config/gates.rs` | Make gate pipeline iterate config instead of hardcoded match |
| W15-E | `AdaptiveBudget` / `adaptive_budget_for()` | `templates/common.rs` | Replace `budget_for(role)` calls in system_prompt_builder.rs |
| W15-E | `Workspace` struct | `roko-core/workspace.rs` | Replace `workdir.join(".roko/...")` throughout codebase |

---

## Incomplete Implementation (code partially written)

### High Priority (affects runtime correctness)

| Batch | Gap | Impact |
|-------|-----|--------|
| W9-E | `StandardPipeline` and `plan show-prompt` command don't exist | No unified prompt pipeline; assembly still inline |
| W10-C | `.roko/memory` path still used in tui/state.rs, tui/dashboard.rs, roko-serve | Path inconsistency — some code reads `.roko/learn/`, some reads `.roko/memory/` |
| W10-E | Playbook outcome recording (14.14) not wired in runner event_loop | Playbook store never learns from successful runs |
| W12-C | `MAX_AGENT_OUTPUT` cap, `start_epoch_ms`, timeout double-fire guard | Unbounded agent output growth, potential OOM |
| W13-E | Error taxonomy rewrite, schema validation | Errors still classified by simple string matching |

### Medium Priority (affects completeness)

| Batch | Gap | Impact |
|-------|-----|--------|
| W9-D | Run completion summary, dispatch_ms info logging, prd retry timing | Observability gaps — can't see timing in production |
| W10-A | Dream workdir fix, INDEX.md path, mcp_servers removal | Minor path bugs, stale references |
| W11-A | `force_plan_terminal` field, 5 Fatal `let _` replacements | Fatal events may still be silently dropped in some paths |
| W11-B | parking_lot::Mutex for EnrichmentRuntimeClient stats | Lock poisoning still possible |
| W14-A | `agents_instructions_section()` not wired for all 7 templates | Some role templates may exceed token budgets |
| W14-B | relay_health try_read, SSE keepalive 8s, SSE replay .take(256) | Potential blocking in health checks, unbounded SSE replay |
| W14-C | observe_internal lock ordering, IMPORTANCE_HISTORY_LIMIT | Potential deadlock, unbounded memory |
| W14-D | ENV var docs, merge_global_into additions | Config merge incomplete for some fields |

### Low Priority (verification-only gaps)

These had code written but were never end-to-end verified. The implementation is likely correct but untested:

| Batch | What Needs Verification |
|-------|------------------------|
| W0-A through W0-E | OpenAI-compat tools param, plan discovery, timing, routing, max_tokens |
| W1-A through W1-C | plan run path, prd extraction, schema unification |
| W2-A, W2-B, W2-D | Tracing, error dedup, spinners (diverged to line output) |
| W3-A | Ctrl+C terminal UX |
| W4-A through W4-C | TypeScript compile of ClickableScenario + ContextPanel |
| W5-A through W5-D | Auth detect, ACP workspace, preflight, cat fallback |
| W6-A through W6-C | Config unify, file locking, boot sequence |
| W7-A | parking_lot→tokio in serve |
| W8-C, W8-D | enrichment_backend removal, TOCTOU fixes |
| W12-D | MCP resolution (confirmed), Permanent retry, JoinSet feedback (unconfirmed) |
| W13-C | Connection pooling docs/User-Agent header |

---

## Priority Wiring Order

If doing gap-closure work, prioritize by impact:

1. **Dead code wiring** (6 items above) — ~10-12 hours total
2. **W10-C path unification** — fix .roko/memory → .roko/learn in remaining 3+ files
3. **W12-C safety caps** — MAX_AGENT_OUTPUT prevents OOM
4. **W8-A clippy blanket removal** — iterative but mechanical
5. **W9-E prompt pipeline** — significant new work (8-12 hours)
6. **Everything else** — verify and fix as encountered
