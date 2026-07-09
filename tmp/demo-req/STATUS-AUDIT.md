# Demo Implementation Status Audit

## 1. What's Built vs Missing (from IMPLEMENTATION-PLAN.md)

### Phase 1: Inline Engine — 60% done

| Component | Status | Notes |
|---|---|---|
| `InlineTerminal` wrapper | **DONE** | Viewport::Inline + insert_before + push_lines_revealed |
| Clack symbols (`◆│└✔✖⚠→━░`) | **DONE** | `inline/symbols.rs` |
| Styled line builders | **DONE** | `inline/styled.rs` — 12 builder functions |
| Markdown renderer | **DONE** | `inline/markdown.rs` — pulldown-cmark with tables, code blocks, blockquotes, lists |
| `RunBlock` (Primitive 1) | **DONE** | agent/predict/gates/tools/cost/chain display |
| `StreamingBlock` (Primitive 2) | **DONE** | Live viewport with auto-scroll, spinner, cursor |
| `ToolCallBlock` (Primitive 3) | **DONE** | Collapsed/expanded, smart input summarization per tool type |
| `CostMeter` (Primitive 5) | **DONE** | Cumulative session: cost/tokens/cache/savings |
| `AgentEventStream` | **DONE** | Typed async channel wrapping WebSocket `StreamChunk` |
| Line-by-line reveal | **DONE** | `push_lines_revealed()` with configurable delay |
| Separators | **DONE** | `push_separator()` dim horizontal rules |
| GateBlock (Primitive 4) | **MISSING** | Gate pipeline progress with per-rung status |
| KnowledgeBlock (Primitive 6) | **MISSING** | Knowledge query result display |
| PredictionBlock (Primitive 7) | **MISSING** | Cost/time/route prediction before execution |
| AuditStepBlock (Primitive 8) | **MISSING** | Deferred until unified refactoring lands |
| ReplanBlock (Primitive 9) | **MISSING** | Gate failure → auto-replan visualization |
| SessionSummary (Primitive 10) | **MISSING** | End-of-session roll-up |
| CostWaterfall (Primitive 11) | **MISSING** | Decomposed savings breakdown |
| ChatInput (Primitive 12) | **PARTIAL** | InputState built, rendering in chat_inline |
| SpinnerLine (Primitive 13) | **DONE** | Via `styled::spinner_line()` |
| DiffBlock (Primitive 14) | **MISSING** | Inline diff display |
| ChainAnchor (Primitive 15) | **MISSING** | Chain confirmation display |
| ProgressTree (Primitive 16) | **MISSING** | Hierarchical plan progress |
| ApprovalPrompt (Primitive 17) | **MISSING** | Interactive approval |
| ErrorBlock (Primitive 18) | **MISSING** | Structured error display |
| Plain-text fallback | **MISSING** | Non-TTY renderer |

**Built: 10 of 18 primitives (56%) + supporting infrastructure**

### Phase 2: `roko chat` inline — 70% done

| Component | Status | Notes |
|---|---|---|
| `chat_inline.rs` | **DONE** | 775 lines, full event loop |
| InputState (buffer, cursor, history) | **DONE** | Insert, delete, home/end, history up/down |
| Phase state machine | **DONE** | Input → Thinking → Streaming → Done |
| Key event handling | **DONE** | All editing keys, Ctrl-C/D, history |
| `/` commands | **DONE** | /help, /cost, /clear, /quit |
| Spinner during thinking | **DONE** | Braille animation + elapsed |
| Agent response rendering | **DONE** | Markdown-rendered with bar prefix |
| Status bar | **DONE** | Cumulative cost/tokens/model |
| Session summary on exit | **DONE** | Turn count, cost, savings |
| Wired into `roko agent chat` | **DONE** | Replaces legacy REPL |
| **Real streaming (SSE/WS tokens)** | **MISSING** | Currently blocks on full response; no token-by-token |
| **Multi-line input (shift+enter)** | **MISSING** | Single-line only |
| **Autocomplete for / commands** | **MISSING** | |
| **Session persistence** | **MISSING** | Chat history not saved to disk |

### Phase 3: `roko run` inline — 40% done

| Component | Status | Notes |
|---|---|---|
| `run_inline.rs` | **DONE** | 202 lines, wraps `run_once` |
| RunBlock rendering | **DONE** | Gate verdicts, markdown output |
| Plain text fallback | **DONE** | `print_plain_report()` for non-TTY |
| **Wired into CLI command** | **MISSING** | Not yet replacing `run_once` in command handler |
| **Cost prediction display** | **MISSING** | No pre-execution prediction |
| **Knowledge inline display** | **MISSING** | No neuro store query before dispatch |
| **Live progress during execution** | **MISSING** | Shows spinner then results, no streaming |

### Phase 4: `roko audit` — DEFERRED

Held until unified refactoring from `tmp/unified` lands.

### Phase 5: `--share` + shareable URLs — 0% done

Nothing implemented. Needs: flag, RunTranscript type, `.roko/shared/` storage, serve route, HTML template, cloudflare tunnel.

### Phase 6: Response cache + demo polish — 20% done

| Component | Status | Notes |
|---|---|---|
| `ResponseCache` type | **EXISTS** | `crates/roko-agent/src/cache.rs` — blake3-keyed, TTL |
| In-memory cache | **EXISTS** | Process-wide shared instance |
| **Pre-seeded domain cache** | **MISSING** | No /finance/ or /infra/ fixtures |
| **demo-magic script** | **MISSING** | Demo shell scripts exist but not demo-magic |
| **asciinema recording** | **MISSING** | |

---

## 2. Cost / Token / Benchmark Infrastructure

### What EXISTS and works

**Cost tracking is comprehensive.** roko tracks more cost data than LiteLLM, Langfuse, or native API objects:

| Metric | Where tracked | Granularity |
|---|---|---|
| Input/output tokens | `AgentResult.usage` | Per agent turn |
| Cache read/write tokens | `AgentResult.usage` | Per agent turn |
| Cost USD (actual) | `AgentResult.usage.cost_usd` | Per agent turn |
| Cost USD (without cache) | `AgentEfficiencyEvent.cost_usd_without_cache` | Per turn (counterfactual) |
| Model used | `AgentEfficiencyEvent.model` | Per turn |
| Prompt section attribution | `AgentEfficiencyEvent.prompt_sections` | Per section per turn |
| Tool call metadata | `AgentEfficiencyEvent.tool_calls` | Per tool call |
| Time to first token | `AgentEfficiencyEvent.time_to_first_token_ms` | Per turn |
| Wall time | `AgentEfficiencyEvent.wall_time_ms` | Per turn |
| Cache hit rate | Computed: `cache_read / input_tokens` | Per turn |
| Cache savings USD | Computed: `without_cache - actual` | Per turn |
| Warm start indicator | `AgentEfficiencyEvent.was_warm_start` | Per turn |

**Storage:** `.roko/learn/efficiency.jsonl` — append-only JSONL, one record per turn.

**Aggregation functions (built):**
- `compute_role_profiles()` — per-role: avg cost, p95, cost-per-pass, warm-start %, pass rate
- `compute_frequency_profiles()` — per operating frequency
- `compute_fleet_cfactor()` — fleet-level with 5 components

**Grade system:** A-D grades based on signal ratio (40%), budget headroom (20%), cache efficiency (20%), gate pass (20%).

### `roko bench` command

**EXISTS and is functional.** SWE-bench proxy harness:
- Accepts dataset JSONL
- 4 agent modes: Gold (oracle), Empty (negative control), PredictionFile, Command
- Per-instance: resolved/failed, format_valid, apply_check, tests_passed, patch_bytes, duration_ms
- Records C-Factor before/after if `record_learning=true`
- Writes efficiency events for offline analysis

### Budget system

**EXISTS and enforced at runtime:**
- `max_plan_usd` (default: $25.0) — hard cap per plan
- `max_turn_usd` (default: $3.0) — hard cap per agent turn
- Budget exhaustion triggers replanning
- Routing uses budget pressure as model selection factor

### What's MISSING for demo benchmarking

| Gap | What's needed |
|---|---|
| **Naive vs optimized comparison** | No side-by-side harness. Would compare stock LangChain AgentExecutor vs roko on same task set. Infrastructure exists (`roko bench` + efficiency events) but no competitor adapter. |
| **Competitor framework adapters** | roko dispatches to its own backends. To compare against LangChain/CrewAI, need wrapper backends that emit the same `AgentEfficiencyEvent`. `demo-research/02-frameworks.md` describes this approach. |
| **Bench TUI tab** | Data pipeline exists (efficiency.jsonl). Need a `bench_view.rs` that groups by `backend` instead of `agent_id`. |
| **Cost waterfall display** | The data to decompose 30x exists (cache savings, routing savings, gate early-exit). No visualization yet. |
| **Canonical 30x benchmark run** | The HAL methodology is documented. The actual run hasn't been executed. |

---

## 3. Resume + Replay: Detailed Status

### Resume: WORKS but with limitations

**What resumes correctly:**
- Plan execution queue and order
- Per-task phase (Queued/Implementing/Gating/Complete/Failed/Skipped)
- Gate results from previous attempts
- Retry count per task (`iteration`)
- Assigned agents and roles
- Files changed by agents
- Merge queue state
- Circuit breaker state (failure counts + timestamps)

**What does NOT resume:**
- **In-flight agent turns** — if an agent was mid-conversation when killed, the next resume restarts that task's agent from scratch. No per-turn checkpointing.
- **Agent process state** — PIDs cleaned up, processes restarted fresh
- **TUI state** — rebuilt from events on restart

**Persistence mechanism:**
- Atomic writes: `persist::atomic_write()` — write to tmp, rename
- Snapshot format: JSON with `ExecutorSnapshot` (schema v1)
- Delta snapshots: `DeltaSnapshot` — incremental changes, BLAKE3-verified
- Cryptographic envelope: `[MAGIC "ROKO"] [8B len] [payload] [32B BLAKE3] [TRAILER "END!"]`
- Location: `.roko/state/executor.json` (or `orchestrator.json` for newer format)

**Resume entry points:**
- `roko plan run <dir> --resume-plan` — explicit snapshot path
- `roko plan run <dir> --resume-state` — alias, defaults to `.roko/state/executor.json`
- Auto-detection: if snapshot exists and plan IDs match, resumes automatically

### Replay: EXISTS but limited

**What `roko replay <hash>` does:**
- Walks the signal DAG (content-addressed engram graph) starting from a blake3 hash
- BFS traversal through parent references
- Shows: signal kind, author, created_at, lineage, tags, body preview
- `--forensic` flag: detailed per-signal metadata

**What it does NOT do:**
- No step-level replay (`--as-of` not implemented)
- No executor state machine reconstruction
- No plan execution timeline view
- No interactive step-through
- Only walks immutable signal DAG, not executor events

### What's needed for demo kill/resume beat

The demo script describes killing a run mid-execution and resuming:
```
$ roko plan run plans/
# ... Ctrl+C mid-task ...
$ roko resume run_4823
# → resumes from checkpoint, zero work lost
```

**Current state:**
- `roko plan run --resume-plan` **works** — resumes from the next incomplete task
- `roko resume <id>` sugar **exists** in `main.rs` (we added the variant)
- The kill/resume is real, not faked — it will pick up from the last completed task
- **Gap:** It does NOT resume from mid-turn within a task. If an agent was 3 turns into a 5-turn task, it restarts that task from turn 1.

---

## 4. Temporal-Like Durability: What Exists vs What's Needed

### What roko has (Temporal-adjacent)

| Temporal Feature | roko Equivalent | Status |
|---|---|---|
| **Workflow state machine** | `ParallelExecutor` with `PlanState` phases | **Implemented** |
| **Durable execution** | Atomic snapshots + crash recovery | **Implemented** |
| **Event sourcing** | `EventLog` — append-only, hash-chained | **Implemented** |
| **Replay from events** | `RecoveryEngine::merge_recovery()` | **Implemented** |
| **Activity retries** | Per-task retry with iteration count | **Implemented** (but not exponential backoff) |
| **Workflow cancellation** | `force_shutdown` + checkpoint | **Implemented** |
| **Saga pattern** | Gate failure → replan → retry | **Implemented** |
| **Heartbeat/timeout** | `time_overrun` watcher in conductor | **Implemented** |
| **Signal handling** | `DashboardEvent` pub/sub via `StateHub` | **Implemented** |

### What roko LACKS vs Temporal

| Temporal Feature | roko Status | What's needed |
|---|---|---|
| **Per-activity checkpointing** | Missing | Checkpoint after each agent turn, not just task completion |
| **Deterministic replay** | Missing | Replay exact same API calls with cached responses |
| **Distributed workers** | Partial | Worker exists but no task queue protocol (worker pulls from local plans, not a Temporal-style task queue) |
| **Versioning** | Missing | No workflow version migration when code changes |
| **Query API** | Partial | Can query state via serve routes, but no Temporal-style workflow query handlers |
| **Signals from external** | Partial | `roko inject` can push signals, but no workflow-level signal handler |
| **Continue-as-new** | Missing | Long-running plans don't rotate to fresh history |
| **Child workflows** | Missing | No nested plan execution |
| **Cron schedules** | Missing | No scheduled plan execution (dream consolidation has no cron trigger) |
| **Exponential backoff** | Missing | Retry is fixed-count, not time-based backoff |

### The demo "kill and resume" beat

For the investor demo, what matters is:
1. **Ctrl+C kills the process** — signal handler + atomic checkpoint ✓
2. **`--resume` picks up where it left off** — loads snapshot, skips completed tasks ✓
3. **Zero work lost** — completed tasks + gate results preserved ✓
4. **Zero tokens wasted** — no re-running completed tasks ✓

This **already works**. The gap is:
- Per-turn checkpointing (lost work = current task's partial turns, not whole plan)
- Pretty inline output showing the resume (currently plain text)

---

## 5. demo-research Status (from `demo/demo-research/`)

The 8 research docs are a **design specification**, not implementation. They describe how to build a benchmark comparison system. Key gaps between the docs and reality:

| Doc | What it describes | What exists | Gap |
|---|---|---|---|
| `01-benchmarks.md` | Task sets: SWE-bench, HumanEval, custom | `roko bench` with SWE-bench proxy | Need HumanEval adapter + custom task sets |
| `02-frameworks.md` | Competing frameworks as roko backends | roko dispatches to 8 backends | Need LangChain/CrewAI/AutoGen wrapper adapters |
| `03-cost-tokens.md` | Cost instrumentation | **Already done** — `AgentEfficiencyEvent` is richer than competitors | Nothing needed, just wire to benchmark view |
| `04-eval-harnesses.md` | Evaluation harness | **Already done** — `roko-orchestrator` is the harness | Need benchmark-specific task scoring |
| `05-realtime-visualization.md` | Live dashboard | TUI exists with 10 tabs | Need `bench_view.rs` tab (F11) |
| `06-recipes.md` | 4 demo recipes (cheapest-first) | Nothing built | Need to implement at least Recipe 1 |
| `07-methodology.md` | Making comparisons defensible | Documentation only | Need to execute the methodology |
| `08-reuse-map.md` | What's already in roko | **95% of benchmark stack exists** | Only adapters + bench tab + task sets needed |

### Bottom line from `08-reuse-map.md`:

> "roko has ~95% of the benchmark stack already built. The job is wiring + adapters, not new tools."

The three things that genuinely don't exist:
1. **Adapters** wrapping competing frameworks as roko backends
2. **Bench tab** in TUI that aggregates per-framework comparisons
3. **Task set** for benchmarks (SWE-bench instance loader exists; need custom tasks)

---

## Summary: Priority Gaps

### Must-fix for demo

| # | Gap | Effort | Impact |
|---|---|---|---|
| 1 | Wire `roko run` through inline engine (command handler change) | 30 min | Every `roko run` looks polished |
| 2 | Real streaming in chat (SSE token deltas) | 3 hours | Chat feels alive, not blocked |
| 3 | `--share` flag + HTML endpoint | 3 hours | "The artifact that leaves the room" |
| 4 | Pre-seeded demo scenarios (/finance/ knowledge, cached responses) | 2 hours | Demo is fast and deterministic |
| 5 | Cost waterfall display (Primitive 11) | 1 hour | Makes 30x claim concrete |
| 6 | GateBlock primitive (Primitive 4) | 1 hour | Gate progress visible during execution |

### Should-have for credibility

| # | Gap | Effort | Impact |
|---|---|---|---|
| 7 | Per-turn checkpointing for true Temporal parity | 1 day | "Zero work lost" is actually true at turn level |
| 8 | Competitor framework adapter (at least LangChain) | 1 day | Side-by-side benchmark becomes possible |
| 9 | Bench TUI tab (F11) | 3 hours | Live benchmark comparison dashboard |
| 10 | `roko replay --as-of` step-level | 3 hours | Audit trail replay |

### Nice-to-have

| # | Gap | Effort | Impact |
|---|---|---|---|
| 11 | Remaining 8 primitives (Diff, Chain, Progress, Approval, Error, etc.) | 4 hours | Full primitive library |
| 12 | Session persistence for chat | 2 hours | Resume conversations |
| 13 | Plain-text fallback renderer | 1 hour | CI/pipe compatibility |
| 14 | demo-magic scripted demo | 2 hours | Perfectly timed presentation |
