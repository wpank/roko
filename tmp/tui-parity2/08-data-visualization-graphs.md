# 08 - Data Visualization & Graphing Opportunities

**Audit date:** 2026-09-01
**Scope:** TUI widgets in `crates/roko-cli/src/tui/widgets/` and `views/`, ratatui chart primitives, bardo visualization PRDs

---

## 1. Current State: What Exists

### 1.1 Existing Custom Visualization Widgets (7)

| Widget | File | Technique | What it renders |
|---|---|---|---|
| **Braille sparkline** | `widgets/braille.rs` | Custom braille U+2800-U+28FF encoding | Single-row 2x-density sparkline from `f64`/`f32`/`u64` data |
| **Token sparkline** | `widgets/token_sparkline.rs` | Braille sparkline + solid-block tier bars | Token usage over time + T0/T1/T2 model distribution bars |
| **Wave progress** | `widgets/wave_progress.rs` | Solid blocks with ocean gradient per-cell | Proportional wave segments with animated gradient fill |
| **Plan tree** | `widgets/plan_tree.rs` | Solid blocks + box drawing + scrollbar | Collapsible wave/plan hierarchy with inline progress bars |
| **Sys metrics** | `widgets/sys_metrics.rs` | Mini gauge (solid blocks) + braille sparkline | CPU/MEM inline gauges with history sparklines |
| **Phase compact** | `widgets/phase_compact.rs` | Segmented solid-block bar | 8-phase pipeline bar color-coded by status |
| **Header bar** | `widgets/header_bar.rs` | Fire gradient solid blocks | Full-width progress bar with animated gradient |

### 1.2 Existing ratatui Built-in Chart Usage (3)

| Widget | File | ratatui Type | What it renders |
|---|---|---|---|
| **Selection frequency** | `views/learning_view.rs:271` | `BarChart` | Per-model selection trial counts |
| **Avg cost per model** | `views/learning_view.rs:554` | `BarChart` | Cost comparison across models |
| **Trend sparklines** | `views/dashboard_view.rs:1330` | `Sparkline` | Token/cost trend lines in dashboard panels |

### 1.3 ratatui Chart Types NEVER Used

| ratatui Widget | Status | Potential |
|---|---|---|
| `Canvas` | **Never used** | Pixel-level drawing with Braille/HalfBlock/Dot markers; supports Lines, Rectangles, Circles, Maps, custom shapes |
| `Chart` (LineChart) | **Never used** | Multi-dataset line/scatter plots with axes, labels, legends; supports Braille markers for high-resolution lines |
| `Gauge` / `LineGauge` | **Never used** | Built-in progress gauge with labels and ratios |

### 1.4 Data Available But Rendered Only as Text

These `TuiState` fields contain graphable data currently displayed as numbers or tables:

| Data | Current Rendering | Field |
|---|---|---|
| Cost over time | Single `$X.XX` number in header | `cost_dollars`, `cost_rate` |
| Token rate | Single `N/min` number | `token_rate` |
| Gate pass/fail history | Text list of verdicts | `gate_results: Vec<GateResultEntry>`, `gate_result_summaries` |
| Agent context usage | Percentage text `XX%` | `AgentRow::input_tokens`/`output_tokens`/`context_limit` |
| Agent turn count | Plain number | Per-agent turn counts in `AgentRow` |
| Cost by model | Table of numbers | `cost_by_model.rs` renders a `Table`, not a chart |
| Plan elapsed times | Text duration "2m", "1h" | `PlanEntry::elapsed_secs` |
| Plan completion trajectory | Static fraction `3/10` | `PlanEntry::tasks_done`/`tasks_total` |
| Gate threshold EMA | No visualization | `.roko/learn/gate-thresholds.json` |
| Cascade router confidence | Bar chart of trials only | Missing pass-rate-over-time line |
| Process CPU/MEM history | Braille sparkline only | `ProcessMetrics::cpu_history`/`mem_history` |
| Efficiency events timeline | Flat event list | `efficiency_events: Vec<AgentEfficiencyEvent>` |
| Provider health/latency | Text table | Provider circuit breaker state |

---

## 2. ratatui Chart API Reference

### 2.1 `Chart` Widget (Line/Scatter Plots)

The `Chart` widget draws multi-dataset line or scatter plots with labeled axes. It is the single most impactful built-in widget that roko does not use at all.

```rust
use ratatui::widgets::chart::{Axis, Chart, Dataset};
use ratatui::symbols::Marker;

let datasets = vec![
    Dataset::default()
        .name("Cost ($)")
        .marker(Marker::Braille)      // High-resolution dots
        .style(Style::default().fg(Color::Cyan))
        .data(&cost_data),             // &[(f64, f64)] — (x, y) pairs
    Dataset::default()
        .name("Tokens (k)")
        .marker(Marker::Dot)
        .style(Style::default().fg(Color::Yellow))
        .data(&token_data),
];

let chart = Chart::new(datasets)
    .block(Block::bordered().title("Cost & Tokens Over Time"))
    .x_axis(
        Axis::default()
            .title("Time (min)")
            .bounds([0.0, 60.0])
            .labels(vec!["0", "15", "30", "45", "60"]),
    )
    .y_axis(
        Axis::default()
            .title("$")
            .bounds([0.0, max_cost])
            .labels(vec!["$0", &format!("${:.2}", max_cost / 2.0), &format!("${:.2}", max_cost)]),
    );
frame.render_widget(chart, area);
```

**Key features:**
- Multiple overlapping datasets with different markers and colors
- Braille marker gives 2x4 sub-pixel resolution per character cell
- HalfBlock marker gives 1x2 resolution with full foreground+background color per half
- Dot and Block markers for simpler terminals
- Built-in axis labels, bounds, and title
- Scatter plot mode (just data points, no lines) via `GraphType::Scatter`
- Line mode with interpolation via `GraphType::Line`

### 2.2 `Canvas` Widget (Pixel-Level Drawing)

The `Canvas` widget provides a virtual coordinate system mapped to terminal cells via configurable markers. It is the most flexible primitive for custom 2D rendering.

```rust
use ratatui::widgets::canvas::{Canvas, Line, Circle, Rectangle, Points};
use ratatui::symbols::Marker;

let canvas = Canvas::default()
    .block(Block::bordered().title("DAG Topology"))
    .x_bounds([0.0, 100.0])
    .y_bounds([0.0, 50.0])
    .marker(Marker::Braille)           // 160x96 effective on 80x24
    .paint(|ctx| {
        // Draw edges as lines
        ctx.draw(&Line {
            x1: 10.0, y1: 40.0,
            x2: 50.0, y2: 20.0,
            color: Color::DarkGray,
        });
        // Draw nodes as circles
        ctx.draw(&Circle {
            x: 10.0, y: 40.0,
            radius: 3.0,
            color: Color::Cyan,
        });
        // Scatter points for data density
        ctx.draw(&Points {
            coords: &[(15.0, 35.0), (20.0, 30.0)],
            color: Color::Yellow,
        });
        // Text labels (always on top, not affected by marker)
        ctx.print(10.0, 42.0, "task-001".fg(Color::White));
        // Layers for z-ordering
        ctx.layer();
        ctx.draw(&Circle { x: 50.0, y: 20.0, radius: 5.0, color: Color::Green });
    });
frame.render_widget(canvas, area);
```

**Marker variants and effective resolution on 80x24 terminal:**

| Marker | Resolution per cell | Effective pixels (80x24) | Color support |
|---|---|---|---|
| `Braille` | 2x4 dots | 160x96 | 1 fg color per cell |
| `HalfBlock` | 1x2 blocks | 80x48 | fg + bg color (2 colors per cell) |
| `Octant` | Dense pseudo-pixels | Similar to Braille, visually denser | 1 fg color per cell |
| `Quadrant` | 2x2 blocks | 160x48 | 1 fg color per cell |
| `Sextant` | 2x3 blocks | 160x72 | 1 fg color per cell |
| `Dot` | 1x1 | 80x24 | 1 fg color per cell |
| `Block` | 1x1 | 80x24 | 1 fg color per cell |

### 2.3 `BarChart` Widget (Already Used)

Already used in `learning_view.rs`. Key addition: ratatui 0.29+ supports `direction(Direction::Horizontal)` for horizontal bar charts, which would be useful for model comparison and agent utilization views.

### 2.4 `Sparkline` Widget (Already Used)

Already used in `dashboard_view.rs`. The built-in `Sparkline` uses block characters for rendering. The custom `braille.rs` implementation provides higher density. Both should coexist: `Sparkline` for quick one-liners, braille for precision.

### 2.5 `Gauge` / `LineGauge` Widgets (Never Used)

Built-in progress indicators with percentage labels. Could replace many hand-rolled solid-block progress bars, though the custom ones have gradient/breathing effects that `Gauge` lacks.

---

## 3. Proposed Visualizations

### 3.1 Cost-Over-Time Line Graph

**Priority: P0 (highest impact)**
**Where:** F1 dashboard, F6 efficiency tab
**Data source:** `efficiency_events` timestamps + `cost_usd` per event; or accumulate `cost_dollars` samples into a `VecDeque<(f64, f64)>` (elapsed_minutes, cumulative_cost)

**Implementation sketch:**

```rust
// In TuiState, add:
pub cost_history: VecDeque<(f64, f64)>,  // (elapsed_secs, cumulative_usd)

// In the render function:
fn render_cost_over_time(frame: &mut Frame, area: Rect, state: &TuiState) {
    let data: Vec<(f64, f64)> = state.cost_history
        .iter()
        .map(|(t, c)| (*t / 60.0, *c))  // Convert to minutes
        .collect();

    if data.is_empty() { return; }

    let max_t = data.last().map(|d| d.0).unwrap_or(1.0);
    let max_c = data.iter().map(|d| d.1).fold(0.01_f64, f64::max);

    let dataset = Dataset::default()
        .name("cost ($)")
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Theme::WARNING))
        .data(&data);

    let chart = Chart::new(vec![dataset])
        .block(Block::bordered().title(" Cost Over Time ")
            .border_style(Style::default().fg(Theme::ROSE_DIM))
            .title_style(Theme::title_style()))
        .x_axis(Axis::default()
            .title("minutes")
            .style(Style::default().fg(Theme::TEXT_DIM))
            .bounds([0.0, max_t])
            .labels(vec![
                Span::styled("0", Style::default().fg(Theme::TEXT_GHOST)),
                Span::styled(format!("{:.0}", max_t), Style::default().fg(Theme::TEXT_GHOST)),
            ]))
        .y_axis(Axis::default()
            .title("$")
            .style(Style::default().fg(Theme::TEXT_DIM))
            .bounds([0.0, max_c * 1.1])
            .labels(vec![
                Span::styled("$0", Style::default().fg(Theme::TEXT_GHOST)),
                Span::styled(format!("${:.2}", max_c), Style::default().fg(Theme::WARNING)),
            ]));

    frame.render_widget(chart, area);
}
```

**Value:** Cost is the single most important operational metric. Seeing the accumulation curve reveals whether cost is linear (expected) or exponential (runaway agent). A flat region means idle; a steep ramp means expensive model routing. Currently this is a single number that gives no trajectory information.

---

### 3.2 Token Throughput Rate Line Graph

**Priority: P0**
**Where:** F1 dashboard, F6 efficiency tab
**Data source:** Derive from `token_history` or accumulate `(elapsed_secs, tokens_per_minute)` samples

**Implementation sketch:**

```rust
fn render_token_throughput(frame: &mut Frame, area: Rect, state: &TuiState) {
    // Dual-dataset: input rate + output rate
    let input_data: Vec<(f64, f64)> = /* ... sample input token deltas ... */;
    let output_data: Vec<(f64, f64)> = /* ... sample output token deltas ... */;

    let datasets = vec![
        Dataset::default()
            .name("input")
            .marker(Marker::Braille)
            .style(Style::default().fg(Theme::DREAM))
            .data(&input_data),
        Dataset::default()
            .name("output")
            .marker(Marker::Braille)
            .style(Style::default().fg(Theme::ROSE))
            .data(&output_data),
    ];

    let chart = Chart::new(datasets)
        .block(Block::bordered().title(" Token Rate (tok/min) "))
        .x_axis(Axis::default().title("time").bounds([x_min, x_max]))
        .y_axis(Axis::default().title("tok/min").bounds([0.0, max_rate * 1.1]));

    frame.render_widget(chart, area);
}
```

**Value:** Shows burst patterns (agent submitting large prompts), idle gaps (waiting for provider), and throughput degradation in real time. The dual input/output traces reveal prompt-heavy vs. generation-heavy phases.

---

### 3.3 Gate Pass/Fail Timeline

**Priority: P1**
**Where:** F5 logs tab, F2 plans tab detail view
**Data source:** `gate_result_summaries: Vec<GateResultSummary>` (has `plan_id`, `passed`, timestamps)

**Implementation sketch — scatter plot with pass/fail markers:**

```rust
fn render_gate_timeline(frame: &mut Frame, area: Rect, state: &TuiState) {
    let pass_data: Vec<(f64, f64)> = state.gate_result_summaries.iter()
        .filter(|g| g.passed)
        .enumerate()
        .map(|(i, _)| (i as f64, 1.0))
        .collect();

    let fail_data: Vec<(f64, f64)> = state.gate_result_summaries.iter()
        .filter(|g| !g.passed)
        .enumerate()
        .map(|(i, _)| (i as f64, 0.0))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name("pass")
            .marker(Marker::Dot)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(Theme::SAGE))
            .data(&pass_data),
        Dataset::default()
            .name("fail")
            .marker(Marker::Block)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(Theme::EMBER))
            .data(&fail_data),
    ];

    let chart = Chart::new(datasets)
        .block(Block::bordered().title(" Gate Verdicts "))
        .x_axis(Axis::default().title("attempt").bounds([0.0, total as f64]))
        .y_axis(Axis::default().bounds([-0.5, 1.5])
            .labels(vec!["FAIL", "", "PASS"]));

    frame.render_widget(chart, area);
}
```

**Value:** Reveals gate failure clusters (systemic issues), gradual improvement trends (learning working), and single outlier failures (flaky tests). Currently gate results are a flat text list with no temporal pattern visibility.

---

### 3.4 Agent Utilization Heatmap

**Priority: P1**
**Where:** F3 agents tab
**Data source:** `agents: Vec<AgentRow>` with `input_tokens`/`output_tokens`/`context_limit`; or derive from `efficiency_events` per-agent per-time-window

**Implementation — Canvas with HalfBlock for 2-color-per-cell heatmap:**

```rust
fn render_agent_heatmap(frame: &mut Frame, area: Rect, state: &TuiState) {
    let agents = &state.agents;
    if agents.is_empty() { return; }

    // Build utilization matrix: rows = agents, columns = time windows
    // Each cell value 0.0..1.0 = context usage in that window
    let canvas = Canvas::default()
        .block(Block::bordered().title(" Agent Utilization "))
        .x_bounds([0.0, time_windows as f64])
        .y_bounds([0.0, agents.len() as f64])
        .marker(Marker::HalfBlock)  // Supports fg+bg for 2 colors per cell
        .paint(|ctx| {
            for (row, agent) in agents.iter().enumerate() {
                for (col, utilization) in windows.iter().enumerate() {
                    let color = heatmap_color(*utilization);
                    ctx.draw(&Rectangle {
                        x: col as f64,
                        y: row as f64,
                        width: 1.0,
                        height: 1.0,
                        color,
                    });
                }
                ctx.print(-2.0, row as f64 + 0.5,
                    truncate(&agent.role, 8).fg(Theme::TEXT_DIM));
            }
        });

    frame.render_widget(canvas, area);
}

fn heatmap_color(v: f64) -> Color {
    // Viridis-inspired: dark blue -> teal -> green -> yellow
    let v = v.clamp(0.0, 1.0);
    if v < 0.25 {
        Color::Rgb(13, 8, 135)       // deep indigo
    } else if v < 0.5 {
        Color::Rgb(42, 120, 142)     // teal
    } else if v < 0.75 {
        Color::Rgb(94, 201, 98)      // green
    } else {
        Color::Rgb(253, 231, 37)     // yellow
    }
}
```

**Value:** Shows which agents are working hard vs. idle, context exhaustion patterns, and parallelism utilization. Currently agent status is a text table; a heatmap makes temporal patterns immediately visible.

---

### 3.5 Plan DAG as Canvas Graph

**Priority: P2**
**Where:** F2 plans tab, replacing or supplementing the tree view
**Data source:** `PlanEntry::tasks` dependency relationships; `execution_waves` for layer ordering

**Implementation — Canvas with Braille lines and labeled nodes:**

```rust
fn render_plan_dag(frame: &mut Frame, area: Rect, state: &TuiState) {
    // Layout: waves on x-axis, tasks within wave on y-axis
    // Edges from task dependencies
    let canvas = Canvas::default()
        .block(Block::bordered().title(" Task DAG "))
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 50.0])
        .marker(Marker::Braille)
        .paint(|ctx| {
            // Draw dependency edges first (behind nodes)
            for edge in &edges {
                ctx.draw(&Line {
                    x1: edge.from_x, y1: edge.from_y,
                    x2: edge.to_x, y2: edge.to_y,
                    color: Color::DarkGray,
                });
            }
            ctx.layer();  // Nodes on top

            // Draw task nodes
            for node in &nodes {
                let color = match node.status {
                    TaskStatus::Done => Theme::SAGE,
                    TaskStatus::Active => Theme::WARNING,
                    TaskStatus::Failed => Theme::EMBER,
                    _ => Theme::TEXT_GHOST,
                };
                ctx.draw(&Circle {
                    x: node.x, y: node.y,
                    radius: 1.5,
                    color,
                });
                ctx.print(node.x + 2.0, node.y,
                    Span::styled(&node.label, Style::default().fg(color)));
            }
        });

    frame.render_widget(canvas, area);
}
```

**Value:** The plan tree shows hierarchy but not topology. A DAG visualization reveals the critical path, parallelism opportunities, and bottlenecks. Tasks that block many downstream tasks are visually prominent. Failed nodes and their downstream blast radius become obvious.

---

### 3.6 Real-Time Streaming Token Rate Chart

**Priority: P1**
**Where:** F1 dashboard, always visible during plan execution
**Data source:** Sample `token_rate` at render time, maintain sliding window in `TuiState`

**Implementation — scrolling Chart with fixed window:**

```rust
// Add to TuiState:
pub token_rate_samples: VecDeque<f64>,  // bounded to 120 samples (2 min at 1Hz)
const MAX_RATE_SAMPLES: usize = 120;

// Sample on each tick:
fn sample_token_rate(&mut self) {
    self.token_rate_samples.push_back(self.token_rate);
    while self.token_rate_samples.len() > MAX_RATE_SAMPLES {
        self.token_rate_samples.pop_front();
    }
}

// Render:
fn render_live_rate(frame: &mut Frame, area: Rect, state: &TuiState) {
    let data: Vec<(f64, f64)> = state.token_rate_samples.iter()
        .enumerate()
        .map(|(i, &rate)| (i as f64, rate))
        .collect();

    let max_rate = data.iter().map(|d| d.1).fold(100.0_f64, f64::max);

    let dataset = Dataset::default()
        .name("tok/min")
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Theme::ROSE));

    let chart = Chart::new(vec![dataset.data(&data)])
        .block(Block::bordered().title(" Live Token Rate "))
        .x_axis(Axis::default()
            .bounds([0.0, MAX_RATE_SAMPLES as f64])
            .labels(vec!["2m ago", "1m ago", "now"]))
        .y_axis(Axis::default()
            .bounds([0.0, max_rate * 1.1])
            .labels(vec!["0", &format!("{:.0}k", max_rate / 1000.0)]));

    frame.render_widget(chart, area);
}
```

**Value:** The current `token_rate` is a single EMA-smoothed number. A scrolling chart shows burst patterns, provider latency spikes (rate drops to zero), and sustained throughput plateaus. This is the "heartbeat monitor" of a running plan execution.

---

### 3.7 Cost-by-Model Horizontal Bar Chart

**Priority: P2**
**Where:** F6 efficiency tab, replacing or supplementing the `cost_by_model.rs` table
**Data source:** Same aggregation logic already in `cost_by_model.rs`

```rust
fn render_cost_by_model_bars(frame: &mut Frame, area: Rect, models: &BTreeMap<String, ModelCostEntry>) {
    let bars: Vec<Bar> = models.iter()
        .map(|(model, entry)| {
            Bar::default()
                .value((entry.total_cost_usd * 10000.0) as u64)
                .label(Line::from(truncate_model(model, 16)))
                .style(Style::default().fg(model_color(model)))
        })
        .collect();

    let bar_chart = BarChart::default()
        .block(Block::bordered().title(" Cost Distribution "))
        .direction(Direction::Horizontal)    // Horizontal bars
        .data(BarGroup::default().bars(&bars))
        .bar_width(1)
        .bar_gap(0);

    frame.render_widget(bar_chart, area);
}
```

**Value:** A horizontal bar chart is far more scannable than a table when comparing 3-5 models. The relative cost differences are immediately visible. Currently the table requires reading and mentally comparing seven columns of numbers.

---

### 3.8 Provider Latency Sparklines

**Priority: P2**
**Where:** F8 providers tab
**Data source:** Provider health registry latency samples (would need to be plumbed to TuiState)

Currently the provider health view is a text table. Adding per-provider braille sparklines inline (like `sys_metrics.rs` already does for CPU/MEM) would show latency trends without needing a full chart.

```rust
// In the provider table, add a sparkline column:
let latency_spark = braille::braille_spans_f64(
    &provider.latency_history,
    provider.p99_ms,
    spark_width,
    if provider.error_rate > 0.1 { Theme::EMBER } else { Theme::SAGE },
);
```

---

## 4. Animated Progress Indicators Beyond Simple Bars

### 4.1 Current Animations

The TUI already has several animation primitives:
- **Breathing brightness** (`atmosphere.breathing_brightness()`) — sinusoidal pulse on active elements
- **Heartbeat** (`atmosphere.heartbeat()`) — double-pulse timing
- **Spinners** (`atmosphere.spinner()`) — rotating character sequences
- **Ocean gradient** (`gradient_ocean()`) — animated per-cell color sweep on wave progress bars
- **Fire gradient** (`gradient_fire()`) — header bar gradient

### 4.2 Proposed: Pulsing Ring Progress

For long-running operations (gate execution, agent dispatch), a ring indicator using braille characters that fills clockwise:

```rust
const RING_CHARS: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

fn ring_progress(pct: f64, frame_idx: usize) -> char {
    let filled = (pct * 8.0) as usize;
    if filled >= 8 { '⣿' }
    else { RING_CHARS[(filled + frame_idx) % 8] }
}
```

### 4.3 Proposed: Block-Fade Transition

When a plan completes, the progress bar could fade through the block density sequence rather than snapping:

```
Active:    ████████████████████░░░░░░░░░░
Fading:    ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░
Done:      ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
Settled:   ─────────────── ✓ ─────────────
```

### 4.4 Proposed: ETA Countdown with Confidence Band

Replace the static "ETA ~4m" text with a mini visualization showing the estimate and its uncertainty:

```
ETA: ├──────╫────────┤ ~4m  (confidence: ±2m)
     2m     4m       6m
```

This uses box-drawing characters and a single `╫` marker for the estimate.

---

## 5. Demoscene-Style Effects in ratatui

The bardo PRDs describe extensive demoscene effects (plasma, fire, tunnel, metaballs, particles). Here is what is feasible in ratatui and what applies to roko:

### 5.1 Feasible Effects

| Effect | ratatui technique | Complexity | Use case in roko |
|---|---|---|---|
| **Plasma background** | Buffer post-processing: iterate cells, compute `sin()` value, set `cell.bg` to HSV color | Medium | Idle/waiting state background |
| **Fire effect** | Cellular automaton on value buffer, map to `░▒▓█` characters | Medium | Gate failure dramatic emphasis |
| **Particle burst** | Maintain `Vec<Particle>`, update positions per tick, render via `buf.cell_mut()` | Medium | Task completion celebration |
| **Phosphor decay** | Store previous frame brightness, multiply by decay factor per tick | Low | Agent output trail persistence |
| **Scanline overlay** | Darken every Nth row by small amount in post-processing pass | Low | Aesthetic atmosphere layer |
| **Breathing colors** | Already implemented in `atmosphere.rs` | Done | Active element emphasis |
| **Chromatic aberration** | Render text at three horizontal offsets with R/G/B colors | Low | Error state emphasis |
| **Starfield** | Random dots with parallax (near dots move faster) via braille | Low | Idle state background |

### 5.2 Not Feasible / Not Applicable

| Effect | Why not |
|---|---|
| Tunnel effect | Requires pre-computed LUTs and high frame rate; too distracting for a developer tool |
| Mandelbulb fractals | CPU intensive, no operational value |
| Full DMT progression | Bardo-specific consciousness metaphor, not applicable to roko's operational focus |
| Ego dissolution | Bardo-specific; dissolving panel borders would break usability |
| Metaballs | Technically feasible but no data to map to; purely decorative |

### 5.3 Recommended Demoscene Elements for roko

1. **Scanline overlay** (Priority: P3) — Subtle darkening of every 3rd row adds depth without hurting readability. Apply as a post-processing pass after all widgets render.

2. **Phosphor decay on agent output** (Priority: P3) — When streaming agent output, new lines are bright; old lines fade. Already partially achieved via the rosedust `brighten()` function; could be extended to a per-line age-based dimming.

3. **Particle burst on task completion** (Priority: P4) — When a task passes its gate, spawn 5-10 braille dots that radiate outward from the task's row position and fade. Purely celebratory but gives visceral feedback.

4. **Starfield idle background** (Priority: P4) — When no agents are active and the dashboard is waiting, render sparse random braille dots drifting slowly downward in the empty space of the plan tree or agent grid. Signals "alive but idle" vs. "frozen/crashed."

---

## 6. `symbols::Marker` Deep Dive

ratatui 0.29 exposes seven marker variants through `ratatui::symbols::Marker`:

```rust
pub enum Marker {
    Dot,       // Simple ASCII dot '•'
    Block,     // Full block '█'
    Braille,   // Unicode U+2800-U+28FF (2x4 dots per cell)
    HalfBlock, // '▀' upper / '▄' lower (1x2 with fg+bg color)
    Octant,    // Dense pseudo-pixels (similar density to Braille, different visual weight)
    Quadrant,  // 2x2 quarter blocks
    Sextant,   // 2x3 blocks (Unicode 13+, limited terminal support)
}
```

**Recommended per-use-case:**

| Use case | Best marker | Reason |
|---|---|---|
| Line charts (cost, tokens, latency) | `Braille` | Highest resolution (2x4), smooth curves |
| Scatter plots (gate verdicts) | `Dot` overlaid on `Block` | Visually distinct pass/fail points |
| Heatmaps | `HalfBlock` | Two colors per cell for smooth gradients |
| DAG graph edges | `Braille` | Diagonal lines need sub-cell resolution |
| DAG graph nodes | N/A (use `ctx.print()`) | Text labels more readable than markers |
| Filled area charts | `HalfBlock` | Smooth fill with color gradient |
| Background effects | `Braille` | Sparse dots for starfield/particle effects |

**Terminal compatibility note:** Braille and HalfBlock work in all modern terminals (iTerm2, Alacritty, WezTerm, Kitty, Windows Terminal). Sextant requires Unicode 13+ support and should be avoided for now. Octant and Quadrant are widely supported.

---

## 7. Priority-Ranked Visualization List

| Priority | Visualization | Impact | Effort | Where |
|---|---|---|---|---|
| **P0** | Cost-over-time line chart | Critical operational insight | Medium | F1/F6, new widget |
| **P0** | Live token rate scrolling chart | Real-time execution heartbeat | Medium | F1, new widget |
| **P1** | Gate pass/fail timeline scatter | Failure pattern recognition | Low | F5, new widget |
| **P1** | Real-time streaming rate chart | Burst/idle pattern visibility | Medium | F1, new widget |
| **P1** | Agent utilization heatmap | Parallelism/bottleneck insight | High | F3, new widget |
| **P2** | Plan DAG canvas graph | Critical path visualization | High | F2, new widget |
| **P2** | Cost-by-model horizontal bars | Replace/supplement table | Low | F6, modify `cost_by_model.rs` |
| **P2** | Provider latency inline sparklines | Latency trend per provider | Low | F8, modify provider table |
| **P2** | Dual-axis cost+tokens chart | Correlation visibility | Medium | F6, new widget |
| **P3** | Scanline post-processing | Atmosphere/depth | Low | Global post-pass |
| **P3** | Phosphor decay on output | Visual recency signal | Low | Agent output panel |
| **P3** | ETA confidence band | Estimate quality signal | Low | Header/phase bar |
| **P3** | Gate threshold EMA trend | Adaptive learning visibility | Low | F10, new sparkline |
| **P4** | Particle burst on completion | Visceral task feedback | Medium | Plan tree/task list |
| **P4** | Starfield idle background | Alive-but-idle signal | Low | Empty space fill |

### Implementation Order Recommendation

**Phase 1 (P0, ~2 days):** Add `cost_history: VecDeque<(f64, f64)>` and `token_rate_samples: VecDeque<f64>` to `TuiState`. Implement `render_cost_over_time()` and `render_live_token_rate()` using `Chart` with `Marker::Braille`. Wire into the F1 dashboard layout (requires a layout adjustment to add a chart row below the existing header/plan-tree split).

**Phase 2 (P1, ~2 days):** Gate timeline scatter plot; agent utilization heatmap using `Canvas` with `HalfBlock`. These require new data plumbing: gate results need timestamps preserved, and agent utilization needs per-window sampling.

**Phase 3 (P2, ~3 days):** Plan DAG canvas; cost-by-model horizontal bars; provider latency sparklines. The DAG canvas is the most complex — it requires a layout algorithm (topological sort already exists in the plan runner; extract positions from wave/task ordering).

**Phase 4 (P3-P4, ~1 day):** Post-processing effects (scanline, phosphor decay) and particle bursts. These are polish items that layer on top of existing rendering without new data plumbing.

---

## 8. Required Data Plumbing

Several visualizations need data that exists in the runtime but is not yet plumbed to `TuiState`:

| Visualization | Missing data | Source | Plumbing needed |
|---|---|---|---|
| Cost-over-time | `cost_history: VecDeque<(f64, f64)>` | Sample `cost_dollars` in `update_efficiency_rates()` | Add field to `TuiState`, sample in existing rate update path |
| Token rate chart | `token_rate_samples: VecDeque<f64>` | Sample `token_rate` in `update_efficiency_rates()` | Add field to `TuiState`, sample in existing rate update path |
| Gate timeline | Timestamps on gate results | `GateResultSummary` already has `plan_id`; needs `created_at_ms` or ordering index | Plumb timestamp from `gate_results_page.gate_rows` which has timestamps |
| Agent heatmap | Per-window utilization samples | Derive from `efficiency_events` per `agent_id` per time bucket | Compute on render from existing data, no new plumbing |
| Provider latency | `latency_history: VecDeque<f64>` per provider | Provider health registry in `roko-agent` | New field on provider health, pushed via `DashboardSnapshot` |
| DAG topology | Task dependency edges | `TasksFile::tasks` has `deps` fields | Parse `tasks.toml` dependencies into edge list on plan load |

The first two (cost history, token rate samples) are trivial additions to the existing `update_efficiency_rates()` method and unblock the two highest-priority visualizations.

---

## 9. Bardo PRD Gap Summary

The bardo `02-visualization-primitives.md` PRD specifies 13 visualization primitives:

| # | Primitive | roko status | Applicable? |
|---|---|---|---|
| 1 | WaveformDisplay (oscilloscope) | Not implemented | Yes — maps to token rate or cost rate time series |
| 2 | ThermalField (2D heatmap) | Not implemented | Yes — maps to agent utilization heatmap |
| 3 | RadarDisplay (polar plot) | Not implemented | Low priority — could show multi-dimensional agent health |
| 4 | ForceGraph (node-link) | Not implemented | Yes — maps to plan DAG / knowledge graph |
| 5 | TimelineRibbon (event timeline) | Partially implemented as `wave_progress.rs` | Yes — extend for gate verdict timeline |
| 6 | IsometricGrid (2.5D) | Not implemented | No — purely decorative for roko's use case |
| 7 | GlobeWireframe | Not implemented | No — no spatial data to render |
| 8-13 | (Not read — file truncated) | Not implemented | Likely decorative for roko |

The bardo `01-demoscene.md` specifies demoscene algorithms. Of these, only the rendering primitives (braille system, half-block, particle systems) and subtle atmospheric effects (scanlines, phosphor decay, breathing) are directly applicable to roko. The consciousness-state rendering (DMT progression, ego dissolution, near-death tunnel) maps to bardo's mortality metaphor and is not relevant to roko's operational focus.

---

## 10. Key Takeaways

1. **The `Chart` widget is the biggest gap.** roko uses `BarChart` and `Sparkline` but has never used the line/scatter `Chart` or `Canvas`. These are the two most powerful ratatui visualization primitives and they are completely absent.

2. **Cost and token data exists but is rendered as numbers.** The `TuiState` already tracks `cost_dollars`, `token_rate`, `cost_rate`, `token_history`, and `efficiency_events`. Converting these to time-series `Chart` visualizations requires only adding bounded sample history (`VecDeque`) and a render function.

3. **Gate results lack temporal visualization.** Pass/fail verdicts are shown as text lists. A scatter plot or timeline would reveal patterns (clusters of failures, improving trends, regression points) that are invisible in the current rendering.

4. **The custom braille sparkline is good but isolated.** It is used only in `token_sparkline.rs` and `sys_metrics.rs`. It should be the standard inline chart primitive across all tabular widgets (provider latency, agent turns, gate threshold EMA).

5. **Canvas is the right tool for the DAG.** The plan tree widget uses text/box-drawing for hierarchy. A `Canvas`-based DAG with Braille lines and labeled nodes would show the dependency topology that the tree view cannot express.

6. **Demoscene effects should be subtle.** Roko is a developer tool, not a consciousness visualizer. Scanlines, phosphor decay, and breathing colors add depth; tunnels, plasma, and fractals do not serve the operational use case.
