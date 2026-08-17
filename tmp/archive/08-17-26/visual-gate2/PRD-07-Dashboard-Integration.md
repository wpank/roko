# PRD-07 — Dashboard Integration: Full-Stack Evaluation Surfaces

**Status**: Draft v2 (expanded)
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-29
**Prerequisites**: PRD-00 (System Overview), PRD-01 (Core Abstractions), PRD-06 (Standalone Visual Analysis)
**Consumes**: `roko-eval`, `roko-eval-browser`, `roko-eval-judge`, `roko-eval-metrics`, `roko-eval-community`, `roko-viz`
**Frontend**: Nunchi Dashboard (React + Vite, `nunchi-dashboard` repo)
**Backend**: `roko-serve` (Rust, HTTP control plane on :6677)

---

## 0. Scope and Current State

This PRD specifies every page, component, API endpoint, WebSocket event, and state
management pattern required to surface the unified evaluation framework across three
integration surfaces:

1. **Web dashboard** -- React SPA served via `roko-serve` on :6677
2. **Terminal TUI** -- ratatui-based `roko dashboard` (F1-F7 tabs)
3. **CLI output** -- `roko eval`, `roko plan run`, `roko status` text output

The goal is a single evaluation data model that renders consistently across all three.

### 0.1 What Exists Today

The existing gate infrastructure surfaces results through three independent paths:

| Surface | Current State | Source Files |
|---|---|---|
| Web: `/api/gates/summary` | Working. Returns per-gate pass rate, avg duration, last run. | `crates/roko-serve/src/routes/status/gates.rs` |
| Web: `/api/gates/history` | Working. Flat list or waterfall format grouped by task_id. | `crates/roko-serve/src/routes/status/gates.rs` |
| Web: `/api/gates/{name}/history` | Working. Time series for a single gate. | `crates/roko-serve/src/routes/status/gates.rs` |
| TUI: Verdicts tab | Working. `VerdictAggregator` reads from substrate, renders rolling 24h stats. | `crates/roko-cli/src/tui/verdicts.rs` |
| TUI: Operations page | Working. Shows gate pipeline steps inline with task execution. | `crates/roko-cli/src/tui/pages/operations.rs` |
| CLI: `plan run` output | Working. Prints verdict summary per task to stderr. | `crates/roko-cli/src/orchestrate.rs` |
| Web: Arena pages | **Broken**. "Failed to load arenas/leaderboard/bounties." | Demo frontend placeholders |
| Web: Measurements pages | **Broken**. "Something went wrong. Resource not found." | Demo frontend placeholders |

The existing gate API returns `GateVerdict` objects with five fields: `gate_name`, `passed`,
`skipped`, `skip_reason`, `output`, `duration_ms`. The new eval framework produces richer
`EvalTrace` objects with evidence bags, criterion results with individual findings, artifact
references, and composite verdicts. The dashboard integration must render this richer data
without breaking the existing gate summary/history endpoints.

### 0.2 Existing Gate Data Flow

The current flow through the codebase:

```
orchestrate.rs
  |-- run_gate_pipeline()           # builds GatePipeline, runs gates sequentially
  |     |-- GatePipeline::verify()  # crates/roko-gate/src/gate_pipeline.rs
  |     |     |-- CompileGate       # rung 0
  |     |     |-- ClippyGate        # rung 1
  |     |     |-- TestGate          # rung 2
  |     |     |-- DiffGate          # rung 3
  |     |     |-- FormatCheckGate   # rung 4
  |     |     |-- ShellGate         # rung 5
  |     |     |-- StubJudgeGate     # rung 6 (not yet implemented)
  |     |
  |-- run_selected_gate_pipeline()  # dispatches per-rung via rung_dispatch::run_rung
  |
  |-- tracker.last_gate_verdicts    # stored per-task in PlanTracker
  |-- tracker.last_gate_verdict_summaries  # GateVerdictSummary for runtime bus
  |
  |-- RuntimeEventBus::emit(RokoEvent::GateCompleted { ... })
  |     |
  |     |-- SSE adapter -> /api/workflow/events
  |     |-- DashboardEvent::GateResult -> StateHub -> TUI/SSE/WS
  |
  |-- EpisodeLogger::log(episode)   # gate_verdicts field
  |-- efficiency event log          # .roko/learn/efficiency.jsonl
```

Each gate verdict flows to four sinks: (1) the runtime event bus for live streaming,
(2) the episode logger for durable per-task records, (3) the efficiency log for
learning feedback, and (4) the TUI verdict aggregator for rolling statistics.

### 0.3 Design System Reference: ROSEDUST

All dashboard components follow the ROSEDUST design system:

**Palette**:
- Backgrounds: void-black `#0a0a0f`, twilight `#12101a`, dusk `#1a1726`
- Rose accent family: `#8b5e6b` (muted) through `#ffc0d0` (highlight)
- Semantic: jade `#30b070` (pass/success), amber `#f5a623` (warning), crimson `#e53e3e` (fail/error), violet `#7c3aed` (knowledge/neuro), sapphire `#3b82f6` (agents/processes)

**Glass Morphism** (three levels):
- Glass-1: `backdrop-filter: blur(8px)`, `bg: rgba(18,16,26,0.60)` -- inline cards, list items
- Glass-2: `backdrop-filter: blur(12px)`, `bg: rgba(18,16,26,0.75)` -- panels, sections
- Glass-3: `backdrop-filter: blur(16px)`, `bg: rgba(18,16,26,0.85)` -- modals, overlays

**Motion** (spring physics via Framer Motion):
- Instant (<100ms): hover, focus, click ack. CSS-only.
- Responsive (100-400ms): value ticks, status changes. Spring: stiffness 500, damping 30.
- Expressive (400-800ms): page transitions, rank reorder. Spring: stiffness 200, damping 28.
- Stagger children at 40ms intervals. Respect `prefers-reduced-motion`.

---

## 1. TUI Integration (ratatui)

### 1.1 Existing TUI Architecture

The TUI is implemented in `crates/roko-cli/src/tui/` with the following structure:

| File | Purpose |
|---|---|
| `mod.rs` | Entry point, `run_dashboard()` |
| `app.rs` | `App` state machine, event dispatch |
| `verdicts.rs` | `VerdictAggregator` -- rolling gate stats from substrate |
| `pages/operations.rs` | Task execution view with inline gate results |
| `pages/efficiency.rs` | Efficiency metrics and cost tracking |
| `views/dashboard_view.rs` | Overview tab (F1) |
| `views/learning_view.rs` | Learning tab with experiments and episodes |
| `widgets/status_bar.rs` | Bottom bar with gate pass rate |
| `widgets/error_digest.rs` | Error details panel |

The `VerdictAggregator` in `verdicts.rs` maintains rolling per-gate statistics by querying
`Kind::GateVerdict` engrams from the substrate. It renders a 24x1-hour bucketed sparkline
for each gate, with recent failures in a scrollable list.

### 1.2 New TUI Components for Visual Gate Results

The eval framework adds visual evidence (screenshots, diffs, region annotations) that
the terminal cannot render natively. The TUI strategy is:

**1.2.1 EvalTrace Summary Widget**

A new `widgets/eval_trace.rs` widget that renders an `EvalTrace` as a compact table:

```
Eval: task-42-ui-login (profile: web-component-standard)
  compile     pass   120ms   evidence: 1 artifact
  lint        pass   340ms   evidence: 2 artifacts
  test        pass  2100ms   evidence: 3 artifacts (14 passed, 0 failed)
  visual      pass  1800ms   evidence: 4 artifacts (screenshots)
    layout_integrity    0.92   finding: 1 minor (overflow at 320px)
    responsive_quality  0.88   finding: 2 minor
    visual_polish       0.95   no findings
  judge       pass  3200ms   evidence: 2 comparisons
    panel: claude-opus=pass, llava-critic=pass, prometheus=pass
  VERDICT: PASS (7/7 criteria, 3200ms total)
```

For visual evidence that cannot be displayed in terminal, the widget shows:
- Artifact count and type (screenshot, diff overlay, heatmap)
- A clickable filepath to the local artifact (opens in system viewer via `open` command)
- A URL to the web dashboard artifact viewer when `roko serve` is running

**1.2.2 Gate Waterfall Widget Enhancement**

The existing operations page shows gate pipeline steps. Enhanced to show:

```
Task: implement-login-form (wave 2, slot 3)
  Agent: claude-sonnet-4 (turn 4/8)
  Gate Pipeline (rung 4):
    [0] compile    PASS  0.12s  ████████████████████
    [1] clippy     PASS  0.34s  ████████████████████
    [2] test       PASS  2.10s  ████████████████████  14/14
    [3] diff       PASS  0.02s  ████████████████████  +142/-31
    [4] format     PASS  0.08s  ████████████████████
    [5] visual     PASS  1.80s  ████████████████░░░░  0.91 (3 findings)
    [6] judge      PASS  3.20s  ████████████████████  3/3 panel agree
  Elapsed: 7.66s | Cost: $0.042 | Tokens: 12,400 in / 3,200 out
```

The visual gate row shows a score bar (0.0-1.0) instead of binary pass/fail,
with finding count inline. The judge row shows panel agreement ratio.

**1.2.3 Evidence Browser Modal**

A new modal (`modals/evidence_browser.rs`) accessible via `Enter` on any gate row:

- Lists all `ArtifactRef` entries in the `EvidenceBag`
- For text artifacts: renders inline with syntax highlighting
- For image artifacts: shows file path + opens in system viewer on `Enter`
- For structured data (test results, lint output): renders as indented tree
- Navigation: `j/k` to scroll, `Enter` to open, `q` to close

### 1.3 TUI Data Flow

```
orchestrate.rs
  |-- emit(RokoEvent::EvalCompleted { eval_trace })
  |
  StateHub (watch::Sender<DashboardEvent>)
  |
  TuiBridge
  |-- DashboardEvent::EvalResult { plan_id, task_id, eval_trace }
  |     |
  |     |-- VerdictAggregator::ingest(eval_trace)   # updates rolling stats
  |     |-- OperationsPage::update_task_gate(eval_trace)  # inline display
  |     |-- App::store_eval_trace(task_id, eval_trace)    # for modal drill-down
```

### 1.4 Implementation Files

| New File | Purpose |
|---|---|
| `crates/roko-cli/src/tui/widgets/eval_trace.rs` | EvalTrace compact renderer |
| `crates/roko-cli/src/tui/modals/evidence_browser.rs` | Evidence artifact browser |
| `crates/roko-cli/src/tui/widgets/criterion_bar.rs` | Score bar for 0.0-1.0 values |
| `crates/roko-cli/src/tui/widgets/judge_panel.rs` | Panel agreement indicator |

| Modified File | Changes |
|---|---|
| `crates/roko-cli/src/tui/verdicts.rs` | Accept `EvalTrace` alongside `GateVerdict` |
| `crates/roko-cli/src/tui/pages/operations.rs` | Render visual gate rows, judge rows |
| `crates/roko-cli/src/tui/views/dashboard_view.rs` | Add eval summary card |
| `crates/roko-cli/src/tui/app.rs` | Store eval traces per task, modal dispatch |

---

## 2. Web Dashboard Integration

### 2.1 Navigation Restructure

The broken Arena and Measurements sections are consolidated into a unified
evaluation destination:

```
Arena (sidebar destination, icon: crosshairs)
  |-- Overview         /app/arena                 Summary dashboard
  |-- Evals Library    /app/arena/evals           Browse + author criteria, profiles
  |-- Runner           /app/arena/runner           Execute evaluations, view results
  |-- History          /app/arena/history          Gate/eval history with drill-down
  |-- Calibration      /app/arena/calibration      Trust the evaluations
  |-- Leaderboard      /app/arena/leaderboard      Agent performance ranking
```

### 2.2 New API Endpoints

All new endpoints live alongside existing gate endpoints. The existing
`/api/gates/*` endpoints remain unchanged for backward compatibility.

**2.2.1 Eval Trace Endpoints**

```
GET  /api/eval/traces                     List recent eval traces
GET  /api/eval/traces/{id}                Get single eval trace with full evidence
GET  /api/eval/traces/{id}/artifacts      List artifacts for a trace
GET  /api/eval/traces/{id}/artifacts/{n}  Download specific artifact (screenshot, etc.)
GET  /api/eval/summary                    Aggregate eval statistics
GET  /api/eval/criteria                   List all registered criteria
GET  /api/eval/profiles                   List all registered profiles
POST /api/eval/run                        Trigger an ad-hoc evaluation
```

**2.2.2 Judge Panel Endpoints**

```
GET  /api/eval/judges                     List configured judge models
GET  /api/eval/judges/calibration         Panel calibration metrics
GET  /api/eval/judges/{model}/stats       Per-model agreement stats
POST /api/eval/judges/compare             Run pairwise comparison
```

**2.2.3 Implementation Location**

New route modules in `crates/roko-serve/src/routes/`:

| File | Purpose |
|---|---|
| `eval.rs` | Eval trace CRUD, summary, criteria listing |
| `eval_artifacts.rs` | Artifact serving (images, logs, diffs) |
| `eval_judges.rs` | Judge panel configuration and calibration |

Registration in `crates/roko-serve/src/routes/mod.rs`:

```rust
mod eval;
mod eval_artifacts;
mod eval_judges;

// In build_router():
let api = Router::new()
    // ... existing routes ...
    .merge(eval::routes())
    .merge(eval_artifacts::routes())
    .merge(eval_judges::routes())
```

### 2.3 WebSocket/SSE Events

The existing SSE endpoint at `GET /api/workflow/events` already streams
`DashboardEvent` variants. New event types for eval results:

```typescript
// New SSE event types
interface EvalStarted {
  type: "eval.started";
  plan_id: string;
  task_id: string;
  profile_id: string;
  criteria_count: number;
  timestamp: string;
}

interface EvalCriterionCompleted {
  type: "eval.criterion_completed";
  plan_id: string;
  task_id: string;
  criterion_name: string;
  passed: boolean;
  score: number | null;
  findings_count: number;
  duration_ms: number;
}

interface EvalCompleted {
  type: "eval.completed";
  plan_id: string;
  task_id: string;
  trace_id: string;
  verdict: "pass" | "fail" | "error";
  criteria_passed: number;
  criteria_total: number;
  total_duration_ms: number;
  total_cost_usd: number;
}

interface EvalArtifactReady {
  type: "eval.artifact_ready";
  trace_id: string;
  artifact_index: number;
  artifact_type: "screenshot" | "diff_overlay" | "heatmap" | "log";
  url: string;
}
```

These events enable the web dashboard to show live progress during evaluation:
criteria completing one by one, screenshots appearing as they are captured,
and final verdict rendering as soon as the last criterion finishes.

### 2.4 Page Specifications

#### 2.4.1 Arena Overview (`/app/arena`)

**Layout**: Full-width dashboard with four metric cards and two data panels.

**Top Row** -- Four glass-1 metric cards:
- **Total Evaluations**: Count with 7-day sparkline
- **Pass Rate**: Percentage with trend arrow (vs. prior 7 days)
- **Mean Duration**: Average eval time with trend
- **Mean Cost**: Average eval cost with trend

**Left Panel** (60%) -- Recent Eval Timeline:
- Vertical timeline showing last 50 evaluations
- Each entry: task name, verdict badge (pass/fail/error), duration, cost
- Click to navigate to full trace view
- Live SSE updates for running evaluations

**Right Panel** (40%) -- Gate Waterfall Heatmap:
- X-axis: time (last 24h in 1h buckets)
- Y-axis: gate names (compile, clippy, test, visual, judge)
- Cell color: jade (100% pass) through crimson (0% pass)
- Corresponds to existing `GET /api/gates/history?format=waterfall`

#### 2.4.2 Evals Library (`/app/arena/evals`)

**Purpose**: Browse, create, and manage evaluation criteria and profiles.

**Layout**: Two-column master-detail with search.

**Left Column** (35%) -- Criteria/Profile Browser:
- Toggle: Criteria | Profiles
- Search bar with type-ahead
- Category filter (deterministic, statistical, visual, judge)
- Each item: name, category badge, usage count, last modified

**Right Column** (65%) -- Detail View:
- Criterion detail: description, evidence requirements, threshold config, example
- Profile detail: ordered list of criteria with weights, applicable domains
- Edit button (opens TOML editor modal for advanced users)

#### 2.4.3 Eval Runner (`/app/arena/runner`)

**Purpose**: Execute evaluations on demand against specific artifacts.

**Layout**: Split view -- configuration left, results right.

**Left Panel** -- Run Configuration:
- Profile selector (dropdown)
- Target: workspace path or artifact URL
- Options: visual gate enabled, judge panel composition, budget cap
- "Run Evaluation" button (triggers `POST /api/eval/run`)

**Right Panel** -- Live Results:
- Shows criteria completing in real-time via SSE
- Each criterion row expands to show findings
- Screenshot artifacts render inline with zoom
- Final verdict card with summary statistics

#### 2.4.4 History (`/app/arena/history`)

Replaces the broken leaderboard. Shows eval trace history with filtering.

**Layout**: Table with expandable rows.

**Columns**: timestamp, task, profile, verdict, duration, cost, criteria_passed/total

**Filters**: date range, verdict (pass/fail/error), profile, gate name

**Expanded Row**: Full eval trace with evidence artifacts, criterion-level detail,
finding list with severity badges.

### 2.5 Artifact Serving

Screenshots, diff overlays, and heatmaps are stored by the eval framework in
`.roko/eval/artifacts/{trace_id}/`. The `eval_artifacts.rs` route module serves
these files with proper MIME types:

```rust
// crates/roko-serve/src/routes/eval_artifacts.rs

async fn get_artifact(
    State(state): State<Arc<AppState>>,
    Path((trace_id, artifact_index)): Path<(String, usize)>,
) -> Result<impl IntoResponse, ApiError> {
    let artifact_dir = state.workdir
        .join(".roko/eval/artifacts")
        .join(&trace_id);
    let manifest = read_artifact_manifest(&artifact_dir).await?;
    let artifact = manifest.artifacts.get(artifact_index)
        .ok_or_else(|| ApiError::not_found("artifact index out of range"))?;

    let content_type = match artifact.kind.as_str() {
        "screenshot" => "image/png",
        "diff_overlay" => "image/png",
        "heatmap" => "image/png",
        "log" => "text/plain",
        _ => "application/octet-stream",
    };

    let body = tokio::fs::read(&artifact.path).await
        .map_err(|e| ApiError::internal(format!("read artifact: {e}")))?;

    Ok(([(header::CONTENT_TYPE, content_type)], body))
}
```

### 2.6 State Management (React)

New Zustand stores for eval state:

```typescript
// stores/evalStore.ts
interface EvalStore {
  traces: EvalTrace[];
  activeTrace: EvalTrace | null;
  criteria: CriterionDef[];
  profiles: ProfileDef[];
  summary: EvalSummary | null;

  // Actions
  fetchTraces(limit?: number): Promise<void>;
  fetchTrace(id: string): Promise<void>;
  fetchCriteria(): Promise<void>;
  fetchProfiles(): Promise<void>;
  triggerRun(config: RunConfig): Promise<string>;

  // SSE subscription
  subscribeToEvents(): () => void;
}
```

The SSE subscription hooks into the existing `workflow/events` stream and
filters for `eval.*` event types.

---

## 3. CLI Output Integration

### 3.1 `roko plan run` Output

The orchestrator already prints gate verdicts to stderr. The eval framework
enhances this output:

**Current format** (gate verdicts only):
```
[task-42] gate compile: PASS (120ms)
[task-42] gate clippy: PASS (340ms)
[task-42] gate test: PASS (2100ms, 14 passed)
[task-42] gate diff: PASS (20ms, +142/-31)
[task-42] GATES PASSED (4/4, 2580ms)
```

**Enhanced format** (with eval criteria):
```
[task-42] eval compile          PASS   0.12s
[task-42] eval lint             PASS   0.34s
[task-42] eval test             PASS   2.10s  14/14
[task-42] eval diff             PASS   0.02s  +142/-31
[task-42] eval format           PASS   0.08s
[task-42] eval visual           PASS   1.80s  score=0.91 findings=3
[task-42]   layout_integrity    0.92   1 minor finding
[task-42]   responsive_quality  0.88   2 minor findings
[task-42]   visual_polish       0.95   clean
[task-42] eval judge            PASS   3.20s  panel=3/3
[task-42] EVAL PASSED (7/7 criteria, 7.66s, $0.042)
```

The enhanced format:
- Replaces "gate" prefix with "eval" prefix
- Adds score values for continuous criteria (0.0-1.0)
- Adds finding counts with severity
- Shows judge panel agreement
- Includes cost in the summary line
- Sub-criteria indented under their parent criterion

### 3.2 `roko eval` Command Family

New CLI commands for standalone evaluation:

```
roko eval run <path>                 Run eval profile against workspace
  --profile <name>                   Evaluation profile (default: workspace default)
  --visual                           Enable visual gate (requires browser)
  --judge                            Enable judge panel
  --budget <usd>                     Maximum cost budget
  --output <format>                  Output format: text, json, jsonl

roko eval list                       List available eval profiles
roko eval show <profile>             Show profile detail with criteria
roko eval history                    Show recent eval traces
roko eval trace <id>                 Show full eval trace detail
roko eval artifacts <trace-id>       List artifacts for a trace
roko eval compare <id1> <id2>        Side-by-side comparison of two traces
roko eval calibrate                  Run judge calibration suite
```

### 3.3 `roko status` Enhancement

The `roko status` command currently reports gate pass/fail counts. Enhanced to include
eval statistics when available:

```
Evaluation Summary (last 24h):
  Runs: 47 (43 pass, 3 fail, 1 error)
  Pass rate: 91.5% (up from 87.2% prior 24h)
  Mean duration: 8.4s
  Mean cost: $0.038
  Top failing criteria: responsive_quality (3 failures)
  Judge panel agreement: 94.2%
```

### 3.4 Implementation Files

| File | Changes |
|---|---|
| `crates/roko-cli/src/orchestrate.rs` | Emit `EvalCompleted` events alongside `GateCompleted` |
| `crates/roko-cli/src/commands/mod.rs` | Register `eval` subcommand family |
| `crates/roko-cli/src/commands/eval.rs` | New: eval CLI command handlers |
| `crates/roko-cli/src/run.rs` | Enhanced output formatting for eval traces |

---

## 4. Unified Data Model

### 4.1 Backward Compatibility

The existing `GateVerdict` type and all `/api/gates/*` endpoints remain unchanged.
Every `EvalTrace` can be projected to a `GateVerdict` for backward compatibility:

```rust
impl From<&EvalTrace> for Vec<GateVerdict> {
    fn from(trace: &EvalTrace) -> Vec<GateVerdict> {
        trace.criterion_results.iter().map(|cr| {
            GateVerdict {
                gate_name: cr.criterion_name.clone(),
                passed: cr.passed,
                skipped: false,
                skip_reason: None,
                output: cr.summary.clone(),
                duration_ms: cr.duration_ms,
            }
        }).collect()
    }
}
```

This projection is used in three places:
1. The SSE adapter emits both `eval.completed` and `gate.completed` events
2. The TUI `VerdictAggregator` accepts both types
3. The episode logger records both `gate_verdicts` and `eval_trace` fields

### 4.2 Storage Layout

```
.roko/
  eval/
    traces/                          # JSONL log of all eval traces
      traces.jsonl                   # Append-only log
    artifacts/                       # Per-trace artifact storage
      {trace-id}/
        manifest.json                # Artifact index
        screenshot-01.png
        diff-overlay-01.png
        heatmap-01.png
    profiles/                        # Eval profile definitions
      web-component-standard.toml
      api-endpoint.toml
      custom/                        # User-defined profiles
    criteria/                        # Custom criterion definitions
      custom/
    calibration/                     # Judge calibration data
      human-labels.jsonl
      calibration-runs.jsonl
```

### 4.3 Event Bus Integration

The runtime event bus (`crates/roko-runtime/src/event_bus.rs`) gains new event variants:

```rust
pub enum RokoEvent {
    // ... existing variants ...

    /// An evaluation has started for a task.
    EvalStarted {
        plan_id: String,
        task_id: String,
        profile_id: String,
        criteria_count: usize,
    },

    /// A single criterion within an evaluation has completed.
    EvalCriterionCompleted {
        plan_id: String,
        task_id: String,
        criterion_name: String,
        passed: bool,
        score: Option<f64>,
        findings_count: usize,
        duration_ms: u64,
    },

    /// An evaluation has completed with a final verdict.
    EvalCompleted {
        plan_id: String,
        task_id: String,
        trace_id: String,
        verdict: EvalVerdictKind,
        criteria_passed: usize,
        criteria_total: usize,
        total_duration_ms: u64,
        total_cost_usd: f64,
    },
}
```

### 4.4 DashboardEvent Mapping

The `DashboardEvent` enum in the StateHub gains corresponding variants:

```rust
pub enum DashboardEvent {
    // ... existing variants including GateResult ...

    EvalResult {
        plan_id: String,
        task_id: String,
        trace_id: String,
        verdict: String,
        criteria_passed: usize,
        criteria_total: usize,
        total_duration_ms: u64,
    },

    EvalCriterionResult {
        plan_id: String,
        task_id: String,
        criterion_name: String,
        passed: bool,
        score: Option<f64>,
    },
}
```

---

## 5. Cross-Surface Consistency

### 5.1 Verdict Rendering Rules

All three surfaces (TUI, web, CLI) follow the same rendering rules:

| Verdict | TUI | Web | CLI |
|---|---|---|---|
| Pass | Green text `PASS` | Jade badge | `PASS` with green ANSI |
| Fail | Red text `FAIL` | Crimson badge | `FAIL` with red ANSI |
| Error | Yellow text `ERROR` | Amber badge | `ERROR` with yellow ANSI |
| Skip | Gray text `SKIP` | Ghost text | `SKIP` with dim ANSI |
| Score 0.0-0.5 | Red bar | Crimson fill | `0.XX` with red ANSI |
| Score 0.5-0.8 | Yellow bar | Amber fill | `0.XX` with yellow ANSI |
| Score 0.8-1.0 | Green bar | Jade fill | `0.XX` with green ANSI |

### 5.2 Finding Severity Mapping

| Severity | TUI | Web | CLI |
|---|---|---|---|
| Critical | `!!` prefix, red | Red dot, bold | `CRITICAL:` prefix |
| Major | `!` prefix, yellow | Amber dot | `MAJOR:` prefix |
| Minor | No prefix, white | Ghost dot | `minor:` prefix |
| Info | Dim text | Ghost text, italic | `info:` prefix |

### 5.3 Artifact Accessibility

| Artifact Type | TUI | Web | CLI |
|---|---|---|---|
| Screenshot | Path + `open` command | Inline `<img>` | Path printed |
| Diff overlay | Path + `open` command | Inline with zoom | Path printed |
| Heatmap | Path + `open` command | Inline with legend | Path printed |
| Text log | Inline render | Syntax-highlighted block | Inline (tail) |
| JSON data | Indented tree | Collapsible JSON viewer | `--output json` |

---

## 6. Performance Constraints

### 6.1 SSE Backpressure

The existing SSE adapter uses `broadcast::Sender` with a bounded channel.
Eval events are higher-frequency (one per criterion per task). Constraints:

- SSE channel capacity: 256 messages (existing default, sufficient)
- Lagged client handling: existing `RecvError::Lagged(n)` warning, no change
- Artifact URLs in events: relative paths, not base64-encoded content

### 6.2 Artifact Storage Limits

- Maximum artifact size: 10MB per file (screenshots typically 200-500KB)
- Maximum artifacts per trace: 50
- GC policy: traces older than 30 days are eligible for artifact pruning
- GC preserves trace metadata (JSONL entry) even after artifact deletion

### 6.3 TUI Rendering Budget

- `VerdictAggregator` refresh: maximum 1Hz (existing, no change)
- EvalTrace rendering: cached string representation, recomputed only on state change
- Evidence browser modal: lazy-loads artifact list on open, not on every tick

---

## 7. Testing Strategy

### 7.1 Backend Tests

| Test | Location | What it verifies |
|---|---|---|
| Eval route unit tests | `crates/roko-serve/tests/eval_routes.rs` | Endpoint responses, pagination, filtering |
| Artifact serving tests | `crates/roko-serve/tests/eval_artifacts.rs` | MIME types, 404 handling, size limits |
| SSE event integration | `crates/roko-serve/tests/eval_sse.rs` | Event stream includes eval events |
| GateVerdict projection | `crates/roko-eval/tests/compat.rs` | `From<&EvalTrace> for Vec<GateVerdict>` |

### 7.2 TUI Tests

| Test | Location | What it verifies |
|---|---|---|
| EvalTrace widget render | `crates/roko-cli/tests/tui_eval_trace.rs` | Correct line count, color codes |
| VerdictAggregator compat | `crates/roko-cli/tests/tui_verdicts.rs` | Accepts both GateVerdict and EvalTrace |
| Evidence browser navigation | `crates/roko-cli/tests/tui_evidence.rs` | j/k scroll, Enter open, q close |

### 7.3 CLI Tests

| Test | Location | What it verifies |
|---|---|---|
| `roko eval` smoke test | `crates/roko-cli/tests/eval_cmd.rs` | Command parses, help text correct |
| Output format test | `crates/roko-cli/tests/eval_output.rs` | Text and JSON formats match spec |

---

## 8. Implementation Order

Phase 1 (backend): New route modules, eval trace storage, artifact serving.
Phase 2 (CLI): `roko eval` command family, enhanced `plan run` output.
Phase 3 (TUI): EvalTrace widget, evidence browser modal, verdicts compat.
Phase 4 (web): React pages, SSE subscriptions, artifact rendering.
Phase 5 (integration): End-to-end test with visual gate producing artifacts
that flow through all three surfaces.

Each phase is a self-contained PR. The backend changes land first because
both CLI and TUI depend on the storage format and event types. The web
dashboard lands last because it depends on the API endpoints being stable.

---

## 9. Open Questions

1. **Artifact retention policy**: 30 days is the initial default. Should this be
   configurable per-profile? Per-workspace?
2. **Screenshot resolution**: Current visual gate captures at viewport size.
   Should we also capture at 2x for retina displays? Storage cost doubles.
3. **Judge panel cost attribution**: Judge model invocations have real API cost.
   Should this be shown separately from agent execution cost?
4. **TUI image rendering**: Some terminal emulators support Sixel or iTerm2
   inline images. Should we detect and use these when available?
5. **Offline artifact access**: When `roko serve` is not running, the TUI
   can still show file paths. Should it also support a local HTTP server
   for artifact viewing?
