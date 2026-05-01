# Final Gap Analysis — What's Left to Build

## Status vs 09-MAY6-DEMO-BUILD.md P0 Checklist

### P0-1: CLI wrapper binary → SKIP (user decided to use `roko` not `nunchi`)

### P0-2: `roko agents list` with identity display
**Current state**: Exists, prints basic NAME/STATUS/PID/BIND/DOMAIN table.
**Gap**: No identity column, no clack-style formatting, no `--env` flag.
**What to build**: Reformat `run_agent_list()` output using inline primitives.
**Effort**: 1 hour.

### P0-3: `roko audit` command → DEFERRED (user said hold until unified refactoring)

### P0-4: `roko resume <run_id>` sugar
**Current state**: `--resume-plan` flag on `roko plan run` works. No standalone `resume` subcommand.
**Gap**: Need `roko resume <run_id>` as a top-level command.
**What to build**: Add `Resume { run_id: String }` variant to Command enum, delegate to plan resume.
**Effort**: 30 minutes.

### P0-5: `roko replay --as-of`
**Current state**: `roko replay <hash>` works, walks signal DAG with `--forensic` flag.
**Gap**: No `--as-of` step-level filtering. No JSON-line event output format.
**What to build**: Add `--as-of <step>` flag, add `--format json` output mode.
**Effort**: 1 hour.

### P0-6: LLM response cache
**Current state**: `ResponseCache` exists in roko-agent (blake3-keyed, TTL, in-memory).
**Gap**: No demo pre-warming. No file-backed persistence across runs. No `roko demo warm` command.
**What to build**: File-backed cache layer + warm command.
**Effort**: 2 hours.

### P0-7: Demo backup tiers
**Current state**: Nothing (no recordings, no screenshots).
**What to build**: asciinema/vhs scripts. Covered by the inline_demo example.
**Effort**: 1 hour (after everything else works).

---

## Status vs demo-research docs

### 01-benchmarks.md
**What's needed**: Task sets for benchmarks.
**Current state**: `roko bench swe` works for SWE-bench proxy. No custom roko-bench task sets.
**Gap**: Need a few hand-authored benchmark tasks.

### 02-frameworks.md (competitor adapters)
**What's needed**: Wrap LangChain/CrewAI/etc as roko backends.
**Current state**: roko dispatches to 8 native backends. No competitor wrappers.
**Gap**: Need Python adapter shims. OUT OF SCOPE for now.

### 03-cost-tokens.md
**What's needed**: Cost/token instrumentation.
**Current state**: COMPLETE. `AgentEfficiencyEvent` is 20+ fields, already richer than LiteLLM/Langfuse.
**Gap**: None. Already done.

### 04-eval-harnesses.md
**What's needed**: Evaluation harness.
**Current state**: COMPLETE. `roko-orchestrator` IS the harness.
**Gap**: None.

### 05-realtime-visualization.md
**What's needed**: F11 Bench tab in TUI.
**Current state**: TUI has F1-F10. No bench tab.
**Gap**: New bench_view.rs (~300 lines). Uses existing widgets.
**Effort**: 3 hours.

### 06-recipes.md
**What's needed**: 4 demo recipe implementations.
**Current state**: None built.
**Gap**: Recipe 1 (five-frame side-by-side) is the demo target. Needs tmux script + benchmark task.
**Effort**: 1 day for Recipe 1.

### 07-methodology.md
**What's needed**: Defensible comparison methodology.
**Current state**: Documentation only.
**Gap**: Need to actually run the benchmark.

### 08-reuse-map.md
**What's needed**: Wire existing 95% into demo format.
**Current state**: All infrastructure exists but not assembled for benchmarks.
**Gap**: Assembly + bench tab + task sets.

---

## Status vs IMPLEMENTATION-PLAN.md

### Built (complete, tested, wired):
- InlineTerminal + viewport + reveal + separator
- 11 primitives (RunBlock, StreamingBlock, ToolCallBlock, GateBlock, CostMeter,
  ReplanBlock, SessionSummary, CostWaterfall, DiffBlock, ProgressTree, ErrorBlock)
- Symbols, styled lines, markdown renderer (tables, code, quotes)
- AgentEventStream (typed WebSocket wrapper)
- Plaintext fallback
- chat_inline.rs (Claude Code-like REPL, wired to `roko agent chat`)
- run_inline.rs (wired to `roko run`)
- inline_demo example

### Not built:
- `--share` flag + shareable URL infrastructure
- `roko resume` top-level command
- `roko replay --as-of` enhancement
- `roko agents list` formatted output
- File-backed response cache + `roko demo warm`
- F11 Bench TUI tab
- Real streaming in chat (token-by-token via WebSocket — currently blocks on full response)

---

## What to Build Now (in order)

1. `roko resume <id>` command — trivial alias
2. `roko replay --as-of` + `--format json` — enhance existing command
3. `roko agents list` with inline formatting — reformat existing command
4. `--share` + RunTranscript + serve route — the "artifact that leaves the room"
5. File-backed response cache
6. Wire real streaming into chat_inline
