# Ratatui Capabilities Audit: What Roko Underutilizes

Dependency: `ratatui = "0.29"` with feature `unstable-rendered-line-info`

## Current Widget Usage in Roko TUI

### Widgets actively used

| Widget | Where used | How |
|---|---|---|
| `Paragraph` | Everywhere (30+ files) | Primary text display, styled spans, wrapping |
| `Block` | Everywhere | Borders, titles, styling containers |
| `Table` | 8 views/widgets | `cost_by_model`, `parallel_pool`, `learning_view`, `plans_view`, `marketplace_view`, `git_view`, `context_view`, `config_view` |
| `List` / `ListItem` | 5 views | `affect_view`, `error_digest`, `context_view`, `marketplace_view`, `atelier_view`, `git_view` |
| `Sparkline` | 1 view | `dashboard_view` (single call at line 1324) |
| `Gauge` | 1 view | `affect_view` (3 PAD gauges: Pleasure/Arousal/Dominance) |
| `BarChart` / `Bar` / `BarGroup` | 1 view | `learning_view` (cascade router stage distribution) |
| `Scrollbar` / `ScrollbarState` | 2 widgets | `task_progress`, `plan_tree` (plan_tree rolls its own buffer-direct scrollbar) |
| `Clear` | 11 modals | Every modal uses Clear before rendering popup content |
| `Wrap` | 15+ files | Text wrapping in paragraphs |

### Widgets NOT used

| Widget | What it does | Opportunity |
|---|---|---|
| **Canvas** | Draw arbitrary shapes with coordinate system | DAG visualization, network topology, agent relationship maps |
| **Chart** | Line/scatter/bar graphs with axes and legend | Token burn over time, latency trends, cost curves, gate pass rates |
| **LineGauge** | Thin horizontal progress bar | Context window usage, disk usage, memory pressure |
| **Calendar** | Monthly calendar view | Episode history, dream schedule, activity heatmap |
| **Tabs** (widget) | Native tab bar rendering | Currently hand-rolled in `header_bar.rs` |

### Features available but barely used

| Feature | Status | What's possible |
|---|---|---|
| `StatefulWidget` / `render_stateful_widget` | 2 calls total | Proper scrollable tables, selectable lists with keyboard nav |
| `ListState` | 1 use (context_view) | Row selection, scroll tracking for all list-based panels |
| `TableState` | Never used | Row highlight, column selection, scroll position tracking |
| `Layout::flex()` | Never used | Flex::SpaceBetween, SpaceAround, SpaceEvenly, Center for responsive layouts |
| `Constraint::Ratio` | Never used | Proportional layouts without percentage rounding |
| `Constraint::Fill` | Never used | Fill remaining space (cleaner than Min(0)) |
| Custom `Widget` trait | Never implemented | All widgets use free functions that take Frame, not the Widget trait |

---

## 1. Canvas Widget: Full Capabilities

The Canvas widget provides a coordinate-mapped drawing surface. It supports five
marker types and six built-in shapes.

### Shapes available

| Shape | What it draws |
|---|---|
| `Line` | A line segment between two (x,y) points |
| `Rectangle` | Axis-aligned rectangle |
| `Circle` | A circle with center and radius |
| `Points` | A scatter of individual points |
| `Map` | A world map outline (MapResolution::High / Low) |
| `Label` | Positioned text on the canvas |

### Custom shapes via the Shape trait

```rust
use ratatui::widgets::canvas::{Painter, Shape};

struct DagEdge {
    from: (f64, f64),
    to: (f64, f64),
}

impl Shape for DagEdge {
    fn draw(&self, painter: &mut Painter) {
        // Bresenham or bezier curve between nodes
        let steps = 50;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let x = self.from.0 + (self.to.0 - self.from.0) * t;
            // Quadratic bezier for curved edges
            let mid_y = (self.from.1 + self.to.1) / 2.0 + 3.0;
            let y = (1.0 - t).powi(2) * self.from.1
                  + 2.0 * (1.0 - t) * t * mid_y
                  + t.powi(2) * self.to.1;
            if let Some((px, py)) = painter.get_point(x, y) {
                painter.paint(px, py, ratatui::style::Color::Rgb(185, 120, 148));
            }
        }
    }
}
```

### Roko opportunities for Canvas

**Plan DAG visualization**: Render task dependency graphs with nodes as circles/labels
and edges as lines. Each node colored by status (green=done, amber=active, red=failed).

```rust
use ratatui::symbols::Marker;
use ratatui::widgets::canvas::{Canvas, Circle, Label, Line as CanvasLine};

let canvas = Canvas::default()
    .marker(Marker::Braille)  // highest resolution
    .x_bounds([0.0, 100.0])
    .y_bounds([0.0, 50.0])
    .paint(|ctx| {
        // Draw task nodes
        for task in &plan.tasks {
            let (x, y) = task_position(task);
            ctx.draw(&Circle {
                x, y,
                radius: 2.0,
                color: status_color(task.status),
            });
            ctx.print(x - 1.0, y - 0.5, task.name.clone());
        }
        // Draw dependency edges
        for (from, to) in &plan.edges {
            ctx.draw(&CanvasLine {
                x1: from.0, y1: from.1,
                x2: to.0, y2: to.1,
                color: Theme::TEXT_DIM,
            });
        }
    })
    .block(Block::default().title("Plan DAG").borders(Borders::ALL));
```

**Agent network topology**: Show agents as nodes, communication channels as edges.
Active agents pulse with HalfBlock markers for visual weight.

---

## 2. Marker Types: Resolution Comparison

Five marker types in 0.29 (three more in 0.30):

| Marker | Resolution per cell | Visual | Best for |
|---|---|---|---|
| `Dot` | 1x1 | `*` | Low-res scatter plots, simple indicators |
| `Block` | 1x1 | `#` | Solid fill, heatmaps |
| `Bar` | 1x1 | `_` | Bar segments, histograms |
| `HalfBlock` | 1x2 | `[upper/lower]` | **Best general-purpose**: square pixel grid, wide font support |
| `Braille` | 2x4 | `[braille dots]` | **Highest resolution**: dense graphs, fine detail |

### Resolution math

A 40-column x 10-row terminal area provides:
- Dot/Block/Bar: 40 x 10 = 400 pixels
- HalfBlock: 40 x 20 = 800 pixels (2x vertical)
- Braille: 80 x 40 = 3,200 pixels (2x horizontal, 4x vertical)

### New in ratatui 0.30 (not yet available in 0.29)

| Marker | Resolution | Notes |
|---|---|---|
| `Quadrant` | 2x2 | Dense pseudo-pixels, no visible banding |
| `Sextant` | 2x3 | 2x3 grid per cell |
| `Octant` | 2x4 | Same resolution as Braille but no visible bands between rows |

**Recommendation**: Roko already does manual braille rendering in `widgets/braille.rs`
and `postfx.rs`. The Canvas widget with `Marker::Braille` would replace the manual
braille code with the standard API, getting coordinate mapping for free.

### Roko's current manual braille vs Canvas

Roko's `braille.rs` (79 LOC) manually maps data to braille characters. This works
for single-row sparklines but cannot do 2D shapes. The Canvas widget provides the
full 2D coordinate system with automatic braille dot mapping. Upgrading to 0.30 would
additionally unlock Octant markers which fill without visible banding.

---

## 3. Chart Widget: Underutilized for Data Visualization

The Chart widget supports multiple datasets, two axes, legends, and three graph types.

### API

```rust
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Block};
use ratatui::symbols::Marker;

let datasets = vec![
    Dataset::default()
        .name("Token burn")
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Theme::ROSE))
        .data(&token_series),  // &[(f64, f64)]
    Dataset::default()
        .name("Cost ($)")
        .marker(Marker::Dot)
        .graph_type(GraphType::Scatter)
        .style(Style::default().fg(Theme::WARNING))
        .data(&cost_series),
];

let chart = Chart::new(datasets)
    .block(Block::default().title("Token Efficiency").borders(Borders::ALL))
    .x_axis(
        Axis::default()
            .title("Time")
            .bounds([0.0, 60.0])
            .labels(vec!["0m", "15m", "30m", "45m", "60m"])
    )
    .y_axis(
        Axis::default()
            .title("Tokens")
            .bounds([0.0, max_tokens])
            .labels(vec!["0", &format!("{:.0}k", max_tokens / 1000.0)])
    )
    .legend_position(Some(LegendPosition::TopRight));

frame.render_widget(chart, area);
```

### Where Chart should replace current implementations

| Current approach | Location | Chart replacement |
|---|---|---|
| Custom braille sparkline for token burn | `widgets/token_sparkline.rs` | Chart with Braille markers, proper axes, multi-dataset overlay |
| Hand-rolled bar segments for tier distribution | `widgets/token_sparkline.rs` (lines 203-225) | BarChart or Chart with GraphType::Bar |
| Single-value Sparkline widget | `views/dashboard_view.rs` | Chart with axis labels for context |
| Manual progress bars via unicode block chars | `widgets/plan_tree.rs` | LineGauge or Gauge with proper percentage |

### Concrete Chart opportunities in Roko

1. **F1 Dashboard**: Token burn rate over time (line), overlaid with cost (scatter)
2. **F10 Learning**: Cascade routing stage transitions as a time-series line chart
3. **F10 Efficiency**: Latency distribution as a scatter plot
4. **Plans view**: Gate pass rate trend line across tasks
5. **Agents view**: Per-agent token consumption comparison (multi-dataset)

---

## 4. Color Capabilities

### Color enum variants (ratatui 0.29)

```rust
enum Color {
    Reset,                    // Terminal default
    Black, Red, Green, Yellow, Blue, Magenta, Cyan, Gray,
    DarkGray, LightRed, LightGreen, LightYellow,
    LightBlue, LightMagenta, LightCyan, White,
    Indexed(u8),              // 256-color palette (0-255)
    Rgb(u8, u8, u8),          // 24-bit true color
}
```

### What Roko currently uses

Roko's theme (`theme.rs`) uses `Color::Rgb()` extensively -- the ROSEDUST palette
has 25+ named RGB constants. This is correct and takes full advantage of true color.
The postfx system also uses RGB throughout for gradients and blending.

### What's underutilized

**`Color::Indexed(u8)`**: The 256-color palette is never used. This matters for
terminals that support 256 colors but not true color (e.g., older tmux configs,
some CI environments). A fallback path mapping ROSEDUST RGB values to their closest
256-color equivalents would improve compatibility.

**Indexed color map for the ROSEDUST palette**:

```rust
impl Theme {
    /// Approximate 256-color fallback for the ROSEDUST palette.
    pub const fn indexed_fallback(rgb: Color) -> Color {
        match rgb {
            Color::Rgb(185, 120, 148) => Color::Indexed(175), // ROSE -> mauve
            Color::Rgb(215, 198, 158) => Color::Indexed(187), // BONE -> light khaki
            Color::Rgb(125, 158, 140) => Color::Indexed(108), // SAGE -> dark sea green
            Color::Rgb(195, 110, 85)  => Color::Indexed(167), // EMBER -> dark salmon
            Color::Rgb(120, 115, 165) => Color::Indexed(104), // DREAM -> medium purple
            _ => rgb,
        }
    }
}
```

---

## 5. Per-Cell Styling Capabilities

### Cell struct methods

```rust
// Direct cell access
if let Some(cell) = buf.cell_mut((x, y)) {
    cell.set_char('X');
    cell.set_symbol("hello");       // Multi-byte symbols
    cell.set_fg(Color::Rgb(r, g, b));
    cell.set_bg(Color::Rgb(r, g, b));
    cell.set_style(Style::default()
        .fg(color)
        .bg(bg)
        .add_modifier(Modifier::BOLD | Modifier::ITALIC)
    );
}

// Read cell properties
let symbol = cell.symbol();
let fg = cell.style().fg;
let bg = cell.style().bg;
```

### Available style modifiers

```rust
Modifier::BOLD
Modifier::DIM
Modifier::ITALIC
Modifier::UNDERLINED
Modifier::SLOW_BLINK
Modifier::RAPID_BLINK
Modifier::REVERSED
Modifier::HIDDEN
Modifier::CROSSED_OUT
```

### What Roko uses

- `BOLD` -- widely used for emphasis
- `fg()` and `bg()` -- used everywhere
- Direct `cell_mut()` access -- used extensively in `postfx.rs` for buffer manipulation

### What Roko doesn't use

- **`DIM`**: Perfect for deemphasized/ghost content. The postfx `dim_overlay()` does
  this manually with color math; `Modifier::DIM` achieves the same natively.
- **`ITALIC`**: Could mark agent "thinking" text, draft content, or uncertain values.
- **`UNDERLINED`**: Natural for clickable/navigable items, hyperlink-style references.
- **`SLOW_BLINK`**: Could replace the manual spinner animation for active items.
  Note: terminal support varies.
- **`CROSSED_OUT`** (strikethrough): Skipped/cancelled tasks, deprecated items.
- **`REVERSED`**: Selection highlight without explicit bg color -- simpler than
  computing inverse colors.

---

## 6. Custom Widget Trait

Roko currently renders everything via free functions:
```rust
pub fn render_plan_tree(frame: &mut Frame<'_>, area: Rect, state: &TuiState, focused: bool) { ... }
```

The Widget trait pattern is:
```rust
pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer);
}

pub trait StatefulWidget {
    type State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
```

### Benefits of converting to Widget trait

1. **Composability**: Widgets can contain other widgets, used inside Canvas paint closures.
2. **Testing**: TestBackend + Widget::render = snapshot testing without Frame.
3. **Third-party integration**: tui-scrollview, tui-widget-list, etc. accept `impl Widget`.
4. **Consistency**: `frame.render_widget(my_widget, area)` vs `my_widget::render(frame, area, ...)`.

### Conversion example: PlanTree as a Widget

```rust
pub struct PlanTreeWidget<'a> {
    state: &'a TuiState,
    focused: bool,
}

impl<'a> PlanTreeWidget<'a> {
    pub fn new(state: &'a TuiState, focused: bool) -> Self {
        Self { state, focused }
    }
}

impl Widget for PlanTreeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // ... existing render_plan_tree logic, writing directly to buf
        // instead of using frame.render_widget for sub-elements
    }
}

// Usage:
frame.render_widget(PlanTreeWidget::new(&state, focused), area);
```

### StatefulWidget for scrollable views

```rust
pub struct TaskList<'a> {
    tasks: &'a [TaskEntry],
    theme: &'a Theme,
}

pub struct TaskListState {
    selected: usize,
    offset: usize,
}

impl StatefulWidget for TaskList<'_> {
    type State = TaskListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Built-in scroll tracking, keyboard selection, highlight rendering
    }
}

// Usage:
frame.render_stateful_widget(
    TaskList::new(&tasks, &theme),
    area,
    &mut task_list_state
);
```

---

## 7. Buffer Manipulation: Direct Access for Effects

### Buffer methods

```rust
// Set content at position
buf.set_string(x, y, "text", style);
buf.set_style(area, style);           // Apply style to entire rect
buf.set_spans(x, y, &spans, width);   // Set styled spans

// Cell-level access
buf.cell(pos) -> Option<&Cell>        // Read cell (safe, no panic)
buf.cell_mut(pos) -> Option<&mut Cell> // Write cell (safe, no panic)
buf[(x, y)]                           // Direct index (panics if OOB)

// Bulk operations
buf.merge(&other_buf);                // Overlay another buffer
buf.resize(area);                     // Resize buffer
buf.reset();                          // Clear all cells
buf.content                           // Direct access to Vec<Cell>
```

### What Roko does well

The `postfx.rs` module (1056 LOC) is an excellent example of direct Buffer
manipulation. It implements:
- Bloom (glow bleed from bright cells)
- Vignette (radial darkening)
- Dim overlay (modal background)
- Modal glow (colored halo)
- Ambient orbs (drifting light sources)
- Dream atmosphere (vignette + grain + breathing)
- Amber color grade
- Drop shadow
- State visualization (progress field, activity ripples, data rain)
- Particle overlay

This is already more sophisticated than most ratatui apps. The manual braille
rendering and per-cell color manipulation is production-quality.

### What could be improved

**`buf.merge()`**: Create off-screen buffers for complex composites, then merge.
This would allow building the postfx pipeline as stacked buffer layers instead of
in-place mutations.

```rust
// Compositing with off-screen buffer
let mut overlay = Buffer::empty(area);
// Render particles to overlay buffer
particle_overlay(area, &mut overlay, elapsed, density, brightness, seed);
// Merge with alpha blending
for y in area.top()..area.bottom() {
    for x in area.left()..area.right() {
        if let (Some(src), Some(dst)) = (overlay.cell((x, y)), buf.cell_mut((x, y))) {
            if src.symbol() != " " {
                *dst = src.clone();
            }
        }
    }
}
```

---

## 8. Scrollbar Widget: Proper Implementation

### API (ratatui 0.29)

```rust
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

let mut scrollbar_state = ScrollbarState::new(total_items)
    .position(current_scroll_offset);

let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
    .begin_symbol(Some("^"))
    .end_symbol(Some("v"))
    .track_symbol(Some("|"))
    .thumb_symbol("#");

// Area should be the content area (scrollbar renders at the edge)
frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
```

### Orientations

```rust
ScrollbarOrientation::VerticalRight   // Most common
ScrollbarOrientation::VerticalLeft    // RTL layouts
ScrollbarOrientation::HorizontalTop
ScrollbarOrientation::HorizontalBottom
```

### What Roko does

- `task_progress.rs`: Uses the proper Scrollbar StatefulWidget (lines 224-238).
- `plan_tree.rs`: Rolls its own buffer-direct scrollbar (line 743+), writing
  track/thumb characters manually to the buffer.
- `plans_view.rs`: Also has a custom buffer-direct scrollbar (line 1197+).

### Recommendation

Standardize on the ratatui Scrollbar widget. The manual implementations work but
are duplicated code. The native widget handles thumb sizing, position calculation,
and symbol rendering automatically.

---

## 9. Table Widget: Advanced Features

### Features roko doesn't use

**TableState for row selection**:
```rust
let mut table_state = TableState::default()
    .with_selected(Some(selected_row));

let table = Table::new(rows, widths)
    .highlight_style(Style::default()
        .bg(Theme::BG_HIGHLIGHT)
        .add_modifier(Modifier::BOLD))
    .highlight_symbol(">> ")
    .highlight_spacing(HighlightSpacing::Always);

frame.render_stateful_widget(table, area, &mut table_state);
```

**Currently**: All 8 Table uses render via `frame.render_widget()` (stateless).
None use `TableState` for keyboard-navigable row selection. The `cost_by_model`,
`parallel_pool`, and `plans_view` tables would all benefit from row selection
for drill-down detail views.

**Column-level features**:
- `Row::height()` -- set minimum row height (useful for multi-line cells)
- `Row::top_margin()` / `Row::bottom_margin()` -- spacing between rows
- `Table::column_spacing()` -- gap between columns
- `Table::flex()` -- responsive column distribution (0.29+)

---

## 10. Layout Engine: Underutilized Features

### Current usage

Roko uses:
- `Constraint::Percentage(n)` -- heavily used
- `Constraint::Length(n)` -- heavily used
- `Constraint::Min(n)` -- used for flexible sections
- `Layout::vertical()` / `Layout::horizontal()` -- used

### Never used

**`Layout::flex()`** with `Flex` variants:

```rust
use ratatui::layout::{Layout, Constraint, Flex};

// Space items evenly across the width
let chunks = Layout::horizontal([
    Constraint::Length(20),
    Constraint::Length(20),
    Constraint::Length(20),
])
.flex(Flex::SpaceBetween)
.split(area);

// Center a single panel
let chunks = Layout::horizontal([
    Constraint::Length(60),
])
.flex(Flex::Center)
.split(area);

// Equal spacing around each item
let chunks = Layout::horizontal([
    Constraint::Length(15),
    Constraint::Length(15),
    Constraint::Length(15),
])
.flex(Flex::SpaceEvenly)
.split(area);
```

**`Constraint::Ratio(num, den)`**:
```rust
// Exact 1/3 - 2/3 split without percentage rounding
Layout::horizontal([
    Constraint::Ratio(1, 3),
    Constraint::Ratio(2, 3),
])
```

**`Constraint::Fill(weight)`**:
```rust
// Fixed sidebar + fill remaining space (cleaner than Min(0))
Layout::horizontal([
    Constraint::Length(30),     // sidebar
    Constraint::Fill(1),       // main content fills remaining
])
```

**`Layout::spacing(n)`**:
```rust
// Automatic gaps between items
Layout::horizontal([
    Constraint::Length(20),
    Constraint::Length(20),
    Constraint::Length(20),
])
.spacing(1)  // 1-cell gap between each
```

### Roko's layout.rs

The current `layout.rs` (115 LOC) has four helpers:
- `centered_rect()` -- percentage-based centering (could use Flex::Center)
- `responsive_outer_margin()` -- manual margin for large terminals
- `split_horizontal()` -- percentage split
- `split_vertical()` -- percentage split

**All four could be simplified with Flex**:

```rust
// centered_rect with Flex (simpler)
pub fn centered_rect_flex(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
```

---

## 11. Popup/Overlay Rendering

### Current approach (correct)

Roko's modals (11 files in `modals/`) correctly use `Clear` before rendering:

```rust
// From modals/approval.rs pattern
let popup_area = centered_rect(60, 40, area);
frame.render_widget(Clear, popup_area);  // Prevent bleed-through
frame.render_widget(popup_block, popup_area);
```

### What could be added

**Z-ordered overlay system**: Instead of each modal managing its own Clear/render,
a modal stack could handle ordering.

```rust
struct ModalStack {
    layers: Vec<Box<dyn ModalLayer>>,
}

impl ModalStack {
    fn render(&self, frame: &mut Frame, area: Rect) {
        for layer in &self.layers {
            let modal_area = layer.area(area);
            frame.render_widget(Clear, modal_area);
            // Dim background for depth
            postfx::dim_overlay(area, frame.buffer_mut(), 0.5);
            layer.render(frame, modal_area);
            // Glow around modal
            postfx::modal_glow(modal_area, frame.buffer_mut(), area,
                              Theme::ROSE_DIM, 0.15);
        }
    }
}
```

**Roko already has `postfx::modal_glow()` and `postfx::dim_overlay()`** --
these exist but I found no evidence they are called from the modal rendering
code. Wiring them into the modal pipeline would add visual depth.

---

## 12. Popular Ratatui Extension Crates

### High-value additions for Roko

| Crate | What | Why for Roko |
|---|---|---|
| **tui-scrollview** | Scrollable viewport widget | Replace manual scroll logic in 5+ views |
| **tui-tree-widget** | Hierarchical tree display | Plan tree, file tree, knowledge hierarchy |
| **ratatui-textarea** | Multi-line text editor | Inline editing for PRD drafts, prompt editing |
| **tui-big-text** | Large pixel text (8x8 font) | Splash screen, "ROKO" branding, status banners |
| **tui-logger** | Log capture widget | Replace manual log display in logs_view |
| **tui-nodes** | Node graph visualization | Agent network, plan DAG, knowledge graph |
| **ratatui-image** | Image rendering (sixel/kitty/halfblock) | Agent avatars, architecture diagrams |
| **tui-piechart** | Pie charts | Model tier distribution, cost breakdown |
| **tui-term** | Terminal emulator widget | Embedded shell for agent output |
| **tui-menu** | Nested menus | Context menus, action menus |
| **malevich** | Advanced plotting (heatmap, histogram, box) | Latency heatmaps, token distribution |
| **ratatui-markdown** | Markdown rendering with syntax highlighting | PRD display, research output, plan descriptions |

### Integration examples

**tui-tree-widget for plan hierarchy**:
```rust
use tui_tree_widget::{Tree, TreeItem, TreeState};

let items = plans.iter().map(|plan| {
    TreeItem::new(
        plan.name.clone(),
        plan.name.clone(),
        plan.tasks.iter().map(|task| {
            TreeItem::new_leaf(
                task.id.clone(),
                format!("{} {} {}", status_icon(task), task.name, task.progress)
            )
        }).collect()
    ).unwrap()
}).collect::<Vec<_>>();

let tree = Tree::new(&items)
    .highlight_style(Style::default().bg(Theme::BG_HIGHLIGHT))
    .highlight_symbol(">> ");

frame.render_stateful_widget(tree, area, &mut tree_state);
```

**tui-scrollview for long content panels**:
```rust
use tui_scrollview::{ScrollView, ScrollViewState};

let mut scroll_view = ScrollView::new(Size::new(area.width, content_height));
// Render content into the scroll view's buffer
scroll_view.render_widget(content_paragraph, content_rect);
// Render the scroll view (handles scrollbar, viewport clipping)
frame.render_stateful_widget(scroll_view, area, &mut scroll_state);
```

---

## 13. Frame Rate Control and Double Buffering

### How ratatui rendering works

Ratatui uses **immediate-mode double buffering**:

1. `Terminal::draw()` creates a fresh `Frame` with a mutable `Buffer`
2. All widgets render into this buffer within the closure
3. `Terminal::flush()` diffs the current buffer against the previous frame
4. **Only changed cells** are written to the terminal (minimal I/O)
5. Buffers swap -- the next draw uses the alternate buffer

### Frame rate in Roko

Roko controls frame rate via the event loop tick interval, not ratatui itself.
The `Atmosphere` struct tracks elapsed time and frame count. The rendering pipeline
then uses `elapsed` for animations.

### What Roko could improve

**Adaptive frame rate**: Skip rendering when nothing changed.

```rust
// In the event loop
let needs_redraw = state_changed || animation_active;
if needs_redraw {
    terminal.draw(|frame| { ... })?;
    atmosphere.tick();
} else {
    // Sleep longer when idle to save CPU
    std::thread::sleep(Duration::from_millis(100));
}
```

**Frame budget tracking**: Measure render time per frame, reduce effects when
rendering takes too long.

```rust
let start = Instant::now();
terminal.draw(|frame| { ... })?;
let render_time = start.elapsed();
if render_time > Duration::from_millis(16) {
    // Disable expensive effects (bloom, particles)
    effects.bloom_enabled = false;
    effects.particles = false;
}
```

---

## 14. Screenshot-Worthy Ratatui Projects (Inspiration)

### Visual techniques to study

| Project | URL | Technique |
|---|---|---|
| **Gitui** | https://github.com/extrawurst/gitui | Complex multi-pane layout, diff rendering, real-time updates |
| **Spotify-player** | https://github.com/aome510/spotify-player | Audio visualization, full Spotify parity in TUI |
| **Yazi** | https://github.com/sxyazi/yazi | Async I/O, image preview, smooth scrolling |
| **Scope-TUI** | https://github.com/alemidev/scope-tui | Real-time oscilloscope, Canvas+Braille for waveforms |
| **tui-globe** | https://github.com/d10n/tui-globe | 3D globe with Braille characters |
| **ratatui-wireframe** | https://crates.io/crates/ratatui-wireframe | 3D wireframe model rotation |
| **Malevich** | https://crates.io/crates/malevich | Millions of data points, heatmaps, box plots |
| **Rat-Commander** | https://github.com/dividebysandwich/rat-commander | Truecolor, process/disk explorers |

### Techniques Roko could adopt

1. **From Scope-TUI**: Canvas + Braille markers for real-time data streams.
   Roko's token burn sparkline could become a live oscilloscope-style display.

2. **From Gitui**: StatefulWidget tables with keyboard navigation everywhere.
   Gitui's diff view technique is applicable to Roko's diff_panel.

3. **From Yazi**: Async rendering with partial updates. Don't re-render entire
   TUI when only one panel changes.

4. **From Malevich**: Heatmap visualization for time-of-day activity patterns,
   cost distribution across tasks/models.

---

## Summary: Priority Recommendations

### High impact, low effort

1. **Use Chart widget** for token burn and efficiency views -- replace manual
   braille sparkline with proper axes, labels, multi-dataset overlay
2. **Use TableState** everywhere tables exist -- add keyboard row selection
   to `cost_by_model`, `plans_view`, `parallel_pool`, etc.
3. **Use Layout::flex()** with `Flex::Center` -- simplify centered_rect and
   modal positioning
4. **Standardize on ratatui Scrollbar** -- remove two duplicate manual
   scrollbar implementations
5. **Use Constraint::Fill(1)** instead of `Constraint::Min(0)` -- cleaner intent

### Medium impact, medium effort

6. **Canvas widget for plan DAG** -- visual dependency graph on the Plans tab
7. **LineGauge for context utilization** -- thin progress bars for agent context
   window usage, replacing manual unicode bar construction
8. **Wire postfx::dim_overlay + modal_glow into modal stack** -- these effects
   exist but appear unwired in actual modal rendering
9. **Add Modifier::DIM, ITALIC, UNDERLINED, CROSSED_OUT** to semantic styling --
   ghost text, draft markers, navigable links, cancelled items

### Lower priority, higher effort

10. **Convert free-function widgets to Widget trait** -- enables third-party
    composition (tui-scrollview, tui-widget-list)
11. **Add tui-tree-widget** for plan hierarchy -- proper collapse/expand
12. **Add ratatui-textarea** for inline editing -- PRD drafting in TUI
13. **Calendar widget** for activity heatmap / schedule view
14. **256-color fallback path** for terminal compatibility

### Future (ratatui 0.30 upgrade)

15. **Octant markers** -- same resolution as Braille without visible banding
16. **Quadrant/Sextant markers** -- intermediate resolution options
17. **Widget restructure** -- widgets move to `ratatui-widgets` crate but are
    re-exported, so upgrade should be transparent
