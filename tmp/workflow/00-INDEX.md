# Multi-Agent Workflow: Mori vs Roko

Deep comparison of how multi-agent workflows work in both systems, plus the unified implementation plan.

## START HERE

**[UNIFIED-IMPLEMENTATION-PLAN.md](UNIFIED-IMPLEMENTATION-PLAN.md)** -- The singular checklist. 80+ granular tasks to converge all three runtimes into one clean engine. Covers every feature from mori, orchestrate.rs, runner v2, and the ACP pipeline. Designed from scratch with the best patterns from each.

## Reference Documents

| Document | What |
|---|---|
| [01-mori-architecture.md](01-mori-architecture.md) | Mori's full multi-agent architecture |
| [02-mori-config-and-plans.md](02-mori-config-and-plans.md) | Mori's configuration, plan files, task format |
| [03-mori-prompts.md](03-mori-prompts.md) | Mori's prompt system per role |
| [04-roko-architecture.md](04-roko-architecture.md) | Roko's current multi-agent architecture |
| [05-roko-config-and-plans.md](05-roko-config-and-plans.md) | Roko's configuration, plan files, task format |
| [06-roko-acp-pipeline.md](06-roko-acp-pipeline.md) | Roko's new ACP pipeline (per-prompt workflow) |
| [07-comparison.md](07-comparison.md) | Side-by-side diff: what's the same, what's different, what's missing |
| [08-how-to-run.md](08-how-to-run.md) | How to actually run a multi-agent workflow in roko today |

## Subsystem Audits

| Document | What |
|---|---|
| [09-inference-dispatch-audit.md](09-inference-dispatch-audit.md) | Every LLM call site: 13+ paths, 4 spawn mechanisms, duplicated parsing, dead feedback loops |
| [10-cli-chat-tui-audit.md](10-cli-chat-tui-audit.md) | Every rendering path: 5 modes, 2 terminal systems, duplicated chat loops, tool output gaps |
| [11-gate-pipeline-audit.md](11-gate-pipeline-audit.md) | 7-rung gate system, 3 separate dispatch paths, adaptive thresholds, LLM judge bypass |
| [12-learning-feedback-audit.md](12-learning-feedback-audit.md) | 10 learning components fully built — all wired only from dead code |
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
