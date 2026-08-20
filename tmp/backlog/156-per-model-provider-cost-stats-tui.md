# 156 — Per-Model/Provider Cost Stats TUI Display

**Priority**: P2 — cost visibility; operators cannot see which models are expensive or efficient without inspecting raw JSON files
**Size**: S (1 day)
**Crates**: `crates/roko-cli/src/tui/`
**Depends on**: #127 (F7 inspect view parity — this item adds a specific panel within that view)
**Sources**: `tmp/mori-old/IMPLEMENTATION-CHECKLIST.md` S3.2, `tmp/mori-old/09-LEARNING-METRICS-COMPARISON.md`

---

## Background

Mori's F7:inspect view showed per-model statistics including pass rate, average duration, retry rate, tokens per run, and cost per run. This let operators quickly identify which models were performing well and which were expensive.

Roko has all the underlying data:

- `.roko/learn/cascade-router.json` contains per-model observation counts (`trials`, `successes`), confidence stats, and routing stage metadata. The file deserializes into `CascadeRouterState` (defined in `crates/roko-cli/src/tui/dashboard.rs:1399`) which has `model_slugs: Vec<String>` and `confidence_stats: HashMap<String, CascadeRouterModelStats>` with `trials`/`successes` per model.
- `.roko/learn/efficiency.jsonl` contains one `AgentEfficiencyEvent` (defined in `crates/roko-learn/src/efficiency.rs:80`) per agent turn, with fields: `model` (exact slug), `backend` (provider name, e.g. `"claude"`, `"gemini"`), `cost_usd`, `input_tokens`, `output_tokens`, `wall_time_ms`, `gate_passed`, `iteration`, and `role`.
- The TUI already loads both files: `DashboardData` reads cascade-router.json into `cascade_router: CascadeRouterState` and efficiency.jsonl into `efficiency_events: Vec<AgentEfficiencyEvent>`, refreshing on file-stamp change. The TUI state (`TuiState`) mirrors these via `tui_state.cascade_router` and `tui_state.efficiency_events`.

However, the data is spread across two separate panels that each show only partial information:

1. **F7 "Cost by Model" panel** (`context_view.rs:393-487`): Aggregates efficiency events into a `ModelCostAggregate` with cost, input/output tokens, wall time, and turn count per model slug -- but does not show pass rate, retry rate, or provider rollup.
2. **F7 "Cascade Route" panel** (`context_view.rs:490-623`): Shows cascade-router trials/successes/pass-rate per model slug -- but does not show cost, tokens, or duration.
3. **F10 "Per-Model Stats" table** (`learning_view.rs:124-186`): Shows cascade-router trials/successes/pass-rate plus a sparkline -- but no cost or duration data.

No panel currently combines both data sources into a single "Model Performance" view with pass rate + cost + tokens + duration, and no panel shows provider-level cost rollup.

## Current State

### F7 Inspect tab layout (`context_view.rs`)

The F7 tab has four sub-views selectable by sub-tab index:

| Sub-tab | View | Renderer |
|---------|------|----------|
| 0 | Overview | `render_with_context_data` (four-section layout) |
| 1 | Signal DAG | `render_signal_dag` |
| 2 | Episode Replay | `render_episode_replay` |
| 3 | Knowledge Browse | `render_knowledge_browse` |

The Overview (sub-tab 0) has this layout:
- **Top 20%**: System Health (token totals, pass rate, C-Factor)
- **Mid-left 40%**: Token Burn by Role (role aggregate table: tokens, cost, turns, cache%)
- **Mid-right 40%**: Cost by Model (model aggregate table: cost, in, out, avg time)
- **Bottom-left 40%**: Cascade Route (model table: tries, wins, rate%)
- **Bottom-right 40%**: Alerts & Gates (conductor alerts + gate pass rates)

The "Cost by Model" panel (`render_cost_by_model`) aggregates efficiency events by model slug and shows cost, input tokens, output tokens, and average wall time -- sorted by cost descending, with color coding (red >$1, yellow >$0.10). It uses a local `ModelCostAggregate` struct.

The "Cascade Route" panel (`render_cascade_router`) reads `tui_state.cascade_router.model_slugs` and `confidence_stats` to show trials/successes/pass-rate per model. It does NOT cross-reference efficiency events.

### Data available but not surfaced

From `AgentEfficiencyEvent`:
- `backend` field (provider name) -- never used in any TUI panel
- `iteration` field -- could derive retry rate (turns with iteration > 1)
- `gate_passed` field -- available but only used in F10 sparklines, not in F7

From `CascadeRouterModelStats`:
- `trials` and `successes` -- used in F7 Cascade Route panel but not combined with cost data

### Provider mapping

The `backend` field on `AgentEfficiencyEvent` contains the provider name (e.g. `"claude"`, `"gemini"`, `"openai"`, `"perplexity"`, `"cerebras"`). The `slug_family` function in `crates/roko-learn/src/cascade/helpers.rs:485` maps model slugs to families (e.g. `"sonnet"`, `"haiku"`, `"gemini-2.5-pro"`) but this is model-family grouping, not provider grouping. For provider rollup, the `backend` field is the correct source.

## Implementation Plan

1. **Add a `ModelPerformanceStats` struct to `context_view.rs`** that merges data from both sources per model slug:

   ```rust
   struct ModelPerformanceStats {
       model: String,
       backend: String,        // provider name from efficiency events
       pass_rate: f64,         // from cascade router: successes / trials
       avg_duration_secs: f64, // from efficiency events: mean wall_time_ms / 1000
       retry_rate: f64,        // from efficiency events: count(iteration > 1) / total
       tokens_per_run: f64,    // from efficiency events: mean(input + output)
       cost_per_run: f64,      // from efficiency events: mean(cost_usd)
       total_cost: f64,        // from efficiency events: sum(cost_usd)
       total_runs: u64,        // from cascade router: trials (or efficiency event count)
   }
   ```

2. **Add a `ProviderRollup` struct** for provider-level aggregation:

   ```rust
   struct ProviderRollup {
       provider: String,
       model_count: usize,
       total_runs: u64,
       total_cost: f64,
   }
   ```

3. **Add an aggregation function** `build_model_performance_stats(tui_state: &TuiState) -> (Vec<ModelPerformanceStats>, Vec<ProviderRollup>)` that:
   - Iterates `tui_state.efficiency_events`, grouping by `model` slug to compute mean cost, mean tokens, mean duration, and retry rate.
   - Joins against `tui_state.cascade_router.confidence_stats` to get pass rate (successes/trials).
   - Groups by `backend` field for provider rollup.
   - Sorts models by total cost descending.

4. **Replace the existing "Cost by Model" panel** (`render_cost_by_model`) with a new `render_model_performance` function showing the unified table:

   ```
   +--Model Performance------------------------------------------------+
   | Model              Pass%  AvgDur  Retries  Tok/Run   $/Run  Total |
   | claude-sonnet-4-6   87%   129s    0.3      45.2k    $1.43  $207  |
   | claude-haiku-4-5    92%    23s    0.1      12.1k    $0.12   $17  |
   | gemini-2.5-pro      78%    89s    0.5      38.7k    $0.89   $20  |
   | Total (3 models)    85%    80s    0.3      32.0k    $0.81  $244  |
   +-------------------------------------------------------------------+
   ```

   Column descriptions:
   - **Model**: model slug, truncated to 20 chars (display via existing `display_model` helper)
   - **Pass%**: pass rate from cascade router (successes/trials), colored green/yellow/red
   - **AvgDur**: average wall-clock duration in seconds from efficiency events
   - **Retries**: retry rate (fraction of turns where `iteration > 1`)
   - **Tok/Run**: average total tokens (input+output) per turn, formatted with `format_count` (e.g. `45.2k`)
   - **$/Run**: average cost in USD per turn
   - **Total**: total cost in USD across all turns for this model
   - **Total row**: weighted averages across all models

5. **Add a provider rollup section below** the model table in the same panel area, using `Layout::vertical` to split the mid-right area into two:

   ```
   +--Provider Costs-------------------------------------+
   | Provider    Models  Runs  Total Cost                |
   | Anthropic   2       145   $187.34                   |
   | Google      1        23    $20.47                   |
   | Total       3       168   $207.81                   |
   +-----------------------------------------------------+
   ```

   The split is 70% for the model table and 30% for the provider rollup.

6. **Highlight extremes**: The most expensive model by `$/Run` gets `theme.danger()` styling. The cheapest model (with >0 runs) gets `theme.success()` styling. Other models use `theme.text()` for cost columns.

7. **Handle empty state**: When `tui_state.efficiency_events` is empty and `tui_state.cascade_router.model_slugs` is empty, render `"No model data available -- run agents to populate"` in the panel area. When efficiency events exist but cascade router data does not (or vice versa), show whatever data is available with `"-"` for unavailable columns.

## Acceptance Criteria

1. F7 Inspect tab (sub-tab 0) shows a "Model Performance" table replacing the old "Cost by Model" panel, with columns: model, pass rate, avg duration, retries, tokens/run, cost/run, total cost.
2. Provider-level cost rollup is shown below the model table.
3. Data comes from real `tui_state.cascade_router` (pass rate) and `tui_state.efficiency_events` (cost, tokens, duration, retries) -- not hardcoded.
4. Models are sorted by total cost descending.
5. Total row shows aggregate stats across all models.
6. Most expensive model by $/Run is highlighted in danger color; cheapest in success color.
7. Empty state shows a descriptive message instead of crashing or rendering an empty table.

## Verification Checklist

- [ ] Run `roko plan run` to generate efficiency events and cascade router observations
- [ ] Open TUI with `roko dashboard`, switch to F7 -- model performance table is visible in sub-tab 0
- [ ] Values in the table match what `roko learn route` and `roko learn efficiency` show via CLI
- [ ] Provider rollup totals equal the sum of per-model totals for that provider
- [ ] Empty state (no `.roko/learn/efficiency.jsonl` or empty cascade-router.json) shows "No model data available" instead of crashing
- [ ] Table renders without overflow on a 120-column terminal
- [ ] Most expensive model row has danger color; cheapest has success color

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/views/context_view.rs` | Replace `render_cost_by_model` with `render_model_performance`; add `ModelPerformanceStats`, `ProviderRollup`, and `build_model_performance_stats` aggregation; add provider rollup rendering |
| `crates/roko-cli/src/tui/views/context_view.rs` | Update mid-right panel layout to split 70/30 between model table and provider rollup |
