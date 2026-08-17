# Multi-Agent Workflow: Mori vs Roko

> Last updated: 2026-08-13

## What is this?

This directory tracks **code quality patterns and refactoring plans** for roko's
multi-agent execution engine. It was created in July 2026 when the codebase had three
separate runtime engines (orchestrate.rs, runner/event_loop.rs, and the ACP pipeline)
that duplicated work and didn't share code. The goal was to converge them into one clean
execution engine.

If you are new to the codebase, start with `ANTI-PATTERNS.md` (what NOT to do) and
`UNIFIED-IMPLEMENTATION-PLAN.md` (the roadmap for fixing it). The reference docs (01-08)
and subsystem audits (09-17) are historical snapshots from July 2026 -- useful for
understanding design decisions, but they describe the architecture as it was then.

**Current status (August 2026):**
- `orchestrate.rs` (the 21K-line dead monolith) has been **deleted** -- a major win.
- The unified plan's Phase 0 services have been **partially built**: `ModelCallService`
  (`roko-agent/src/model_call_service.rs`), `PromptAssemblyService`
  (`roko-compose/src/prompt_assembly_service.rs`), `FeedbackService`
  (`roko-learn/src/feedback_service.rs`), `EffectDriver`
  (`roko-runtime/src/effect_driver.rs`), `PipelineState`
  (`roko-runtime/src/pipeline_state.rs`), `TaskScheduler`
  (`roko-runtime/src/task_scheduler.rs`), and `WorkflowEngine`
  (`roko-runtime/src/workflow_engine.rs`) all exist as modules. `PersistenceService`
  was never extracted as a standalone service.
- **The god-file problem migrated, not solved.** `runner/event_loop.rs` remains the
  primary live runtime and has grown to ~19,846 lines (up from 3K), absorbing features
  from orchestrate.rs without decomposing into the planned services. The services exist
  but event_loop.rs has not been refactored to delegate to them.
- The Signal rename is **complete** (Signal is now the core noun; formerly called Engram, renamed 2026-08-12).
- The `eprintln!`-to-`tracing` conversion is **mostly complete** (810 tracing calls vs
  ~250 remaining eprintln calls across 36 files, mostly in CLI output paths where
  direct stderr is intentional).

## START HERE

**[UNIFIED-IMPLEMENTATION-PLAN.md](UNIFIED-IMPLEMENTATION-PLAN.md)** -- The singular checklist. 80+ granular tasks to converge all three runtimes into one clean engine. Covers every feature from mori, orchestrate.rs, runner v2, and the ACP pipeline. Designed from scratch with the best patterns from each. See the status annotations inside for what has been built vs what remains.

**[ANTI-PATTERNS.md](ANTI-PATTERNS.md)** -- 10 documented anti-patterns with real codebase examples. Updated with current status annotations showing which problems have been fixed, which persist, and which have shifted form.

## Reference Documents

Historical architecture docs from July 2026. Mori is the predecessor system that roko replaced.

| Document | What |
|---|---|
| [01-mori-architecture.md](01-mori-architecture.md) | Mori's full multi-agent architecture |
| [02-mori-config-and-plans.md](02-mori-config-and-plans.md) | Mori's configuration, plan files, task format |
| [03-mori-prompts.md](03-mori-prompts.md) | Mori's prompt system per role |
| [04-roko-architecture.md](04-roko-architecture.md) | Roko's architecture as of July 2026 |
| [05-roko-config-and-plans.md](05-roko-config-and-plans.md) | Roko's configuration, plan files, task format |
| [06-roko-acp-pipeline.md](06-roko-acp-pipeline.md) | Roko's ACP pipeline (per-prompt workflow) |
| [07-comparison.md](07-comparison.md) | Side-by-side diff: what's the same, what's different, what's missing |
| [08-how-to-run.md](08-how-to-run.md) | How to actually run a multi-agent workflow in roko today |

## Subsystem Audits

Historical audits from July 2026. Useful for understanding design decisions and recurring
problems. Numbers are from July 2026 -- file sizes and counts have changed since then.

| Document | What |
|---|---|
| [09-inference-dispatch-audit.md](09-inference-dispatch-audit.md) | Every LLM call site: 13+ paths, 4 spawn mechanisms, duplicated parsing, dead feedback loops |
| [10-cli-chat-tui-audit.md](10-cli-chat-tui-audit.md) | Every rendering path: 5 modes, 2 terminal systems, duplicated chat loops, tool output gaps |
| [11-gate-pipeline-audit.md](11-gate-pipeline-audit.md) | 7-rung gate system, 3 separate dispatch paths, adaptive thresholds, LLM judge bypass |
| [12-learning-feedback-audit.md](12-learning-feedback-audit.md) | 10 learning components fully built -- all wired only from dead code |
| [13-prompt-assembly-audit.md](13-prompt-assembly-audit.md) | 9-layer SystemPromptBuilder used by 1 of 6+ entry points, VCG auction overengineered |
| [14-cognitive-layer-audit.md](14-cognitive-layer-audit.md) | Neuro/dreams (keep), daimon 40K LOC (replace), pheromones 68K LOC (delete) |
| [15-orchestration-plan-execution-audit.md](15-orchestration-plan-execution-audit.md) | 3 runtimes, 2 state machines, 21K-line dead monolith, features never ported |
| [16-http-serve-persistence-audit.md](16-http-serve-persistence-audit.md) | ~175 routes, 30 modules, 50+ persistence files, StateHub pattern, persistence duplication |
| [17-safety-agent-system-audit.md](17-safety-agent-system-audit.md) | 8 backends, 10-stage tool dispatch, behavioral contracts that fail open |
| [ANTI-PATTERNS.md](ANTI-PATTERNS.md) | 10 documented anti-patterns with real codebase examples |

## Related

- `tmp/mori-diffs/` -- The existing 41-document audit package (gap ledger, per-subsystem audits)
- `tmp/mori-diffs/29-CURRENT-RUNTIME-GAP-LEDGER.md` -- Canonical gap tracker from the audit
- `tmp/mori-diffs/21-FEATURE-PARITY-MATRIX.md` -- Mori parity acceptance tracker
- Root `CLAUDE.md` -- The canonical project-level context document (always current)
