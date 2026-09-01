# Bardo Screen Specs & Rendering PRD Summary

Research extracted from 8 bardo PRD documents in `prd/18-interfaces/`.

---

## Background: What Bardo Is

Bardo was a terminal-based TUI for managing "Golems" -- mortal autonomous DeFi agents that
trade, learn, and die. The terminal was a 60fps ratatui application with 29 screens across
6 "windows" (top-level navigation groups). Every visual element was driven by live runtime
state including a PAD (Pleasure-Arousal-Dominance) emotional vector, mortality clocks, and
a lifecycle phase system. The design system was called ROSEDUST: rose-on-violet-black
with a CRT/phosphor aesthetic.

Roko's TUI inherits the structural ideas (tabs, panes, modals, depth layers) but replaces
the DeFi/mortality domain with the agent-development domain.

---

## 1. Screen-by-Screen Specifications (Mapping to Roko Tabs)

Bardo had 6 windows with 29 total screens. Below is the full inventory and the roko tab
each maps to. The roko TUI currently has F1-F10 tabs.

### Window 1: Hearth ("The Presence") -- What is happening right now?

| Bardo Tab | Content | Roko Equivalent |
|-----------|---------|-----------------|
| Overview | Heartbeat log (phosphor-decay event log), vitality number, mortality gauges, MAGI consensus panel, PAD summary | **F1: Overview** -- plan execution activity, agent health, task status |
| Signals | Probe grid (16 sensors as unit array), market regime, signal constellation (circular dots) | **F2: Signals/Events** -- telemetry lens signals, event bus |
| Operations | Full tick log (filterable by tier), cost accumulator, decision cache stats | **F5: Operations** -- runner event log with cost tracking |
| Status | Connection status (WebSocket/providers), resource usage, error log, version info | **F9: Status** -- provider health, connection state, diagnostics |

**Key rendering patterns from Hearth:**
- Phosphor-decay log: entries dim over time through chain `bone -> rose -> rose_dim -> text_ghost -> text_phantom`
- Variable-ratio reinforcement rhythm: most events dim (T0), occasional bright blazes (T2/T3)
- Peripheral monitoring design: leave-it-running screen where rhythm and color convey state without reading

### Window 2: Mind ("The Cognition") -- How does it think?

| Bardo Tab | Content | Roko Equivalent |
|-----------|---------|-----------------|
| Pipeline | Global workspace (context assembly), 9 specialist modules with connection lines, psychographic braille density, Phi gauge | **F3: Pipeline/Dispatch** -- runner dispatch, system prompt assembly, provider selection |
| Grimoire (Knowledge) | Entry list with type glyphs, confidence bars, causal graph view | **F4: Knowledge** -- neuro store entries, knowledge tiers, dream journal |
| Playbook | Document viewer with inline confidence sparklines, diff view, archaeology mode | **F6: Playbook/Learning** -- when/then playbooks, efficiency events, cascade router |
| Dreams | Dream journal, replay strip, imagination pane, consolidation progress | **F4: Knowledge** (dream sub-tab) -- dream consolidation cycle output |
| Inference | Tier breakdown (T0-T3 costs), cost sparklines, cache performance, token flow | **F5: Operations** (inference sub-view) -- provider cost tracking |

**Key rendering patterns from Mind:**
- Atmosphere changes between tabs (unique to this window): each cognitive mode has different visual character
- Causal graph view (`g` key): force-directed graph of knowledge entries as nodes, causal links as edges
- Braille density map: cognitive load visualized as organic texture (sparse dots = low, dense chaos = high)

### Window 3: Soma ("The Economy") -- What does it own?

| Bardo Tab | Content | Roko Equivalent |
|-----------|---------|-----------------|
| Portfolio | NAV number, allocation bar, position cards (LP, vault, lending) | N/A (DeFi-specific) |
| Trades | Trade log with emotional tags, phosphor decay | Limited: gate result history |
| Custody | Session keys, delegation tree, spend history | N/A |
| Bazaar | Knowledge marketplace listings | **F8: Marketplace** -- artifact marketplace listings |
| Budget | Credit balance, budget allocation bar, cost cap state, burn rate waveform | **F7: Budget** -- provider cost tracking, inference spend |
| Sanctum | Protocol browser, active protocol view, execution bar | N/A (DeFi-specific) |

### Window 4: World ("The Collective") -- What else exists?

| Bardo Tab | Content | Roko Equivalent |
|-----------|---------|-----------------|
| Solaris | Force-directed graph of all agents, emotional weather zones | **F10: Agents** -- agent groups, connectivity |
| Clade | Peer tiles (unit array), knowledge sync animation, emotional contagion | **F10: Agents** -- agent group membership, knowledge sharing |
| Lethe | Flowing text fragments (anonymous knowledge commons) | N/A |
| Bloodstains | Death-validated knowledge feed | N/A (no mortality) |

### Window 5: Fate ("The Lifecycle") -- How long will it live?

| Bardo Tab | Content | Roko Equivalent |
|-----------|---------|-----------------|
| Mortality | Three clock gauges (economic/epistemic/stochastic), defense layers, Monte Carlo survival chart | Limited: agent vitality/energy in F10 |
| Lineage | Generational tree, ancestor comparison | N/A |
| Achievements | 87 achievements in grid with progress arcs | N/A |
| Graveyard | Tombstone gallery of dead agents | N/A |

### Window 6: Command ("The Owner's Interface")

| Bardo Tab | Content | Roko Equivalent |
|-----------|---------|-----------------|
| Steer | Chat interface (owner messages right-aligned, agent left-aligned, streaming) | **roko chat** -- existing chat REPL; potential TUI integration |
| Config | Parameter groups with slider editor, impact preview | **F9: Status/Config** -- config show/edit |
| Effects | Visual settings (scanlines, noise, animation speed, CRT toggle) | Potential: TUI settings panel |
| Hermes | Cross-agent meta-chat, fleet summary | Potential: multi-agent coordination view |

---

## 2. Visualization Primitive Catalog (13 Primitives)

All primitives implement a shared `DataPrimitive` trait with `render()` and `tick()`.
Each takes a ratatui `Rect`, `Buffer`, and `AnimationContext` (elapsed time, PAD vector,
heartbeat phase, palette reference).

### P1: WaveformDisplay
- Rolling time-series oscilloscope trace
- Characters: `['_','_','_','_','_','_','_','_']` (Unicode block elements)
- Phosphor decay: older samples dim through brightness chain (100% -> 80% -> 60% -> 35% -> 15%)
- Modes: Single trace, dual trace (side-by-side comparison), multi-channel
- **Roko use:** Efficiency metrics over time, gate threshold history, provider latency waveforms

### P2: ThermalField
- 2D heatmap on grid, `Vec<Vec<f64>>` data through colormap
- Characters: `[' ', '_', '_', '_', '_']` (block fill) + `['.', '.', '+', '*']` (foreground)
- Colormaps: RoseDust (default), Viridis, Inferno, Custom
- Supports animated evolution via update function
- **Roko use:** Agent activity heatmap, knowledge density, workspace resource usage

### P3: RadarDisplay
- Polar/circular plot with concentric rings and radial value arms
- Animated sweep arc (1 rotation per 4 heartbeats), corner data boxes
- Characters: center `+`, rings `- |`, value arms `> -`
- **Roko use:** Multi-dimensional agent health (6-8 metrics as radial arms)

### P4: ForceGraph
- Node-link graph with soft-body Verlet integration
- Repulsion + spring forces, cursor gravity well, never fully settles (random perturbation)
- Node glyphs: `diamond` episode, `diamond_outline` insight, `circle` heuristic, `zap` skill, `therefore` causal
- Edge types: Strong (`===`), Normal (`---`), Weak (`...`), Contradicts (`XXX`), DeadSource (`dagger...dagger`)
- Characters for edges selected by angle: horizontal, diagonal, vertical variants
- **Roko use:** Knowledge graph visualization, plan DAG, agent group topology

### P5: TimelineRibbon
- Horizontal time axis with colored event segments
- Current time marker (bright vertical bar), hover tooltip
- Segments have fill characters and color per phase/state
- Optional Japanese timestamp format
- **Roko use:** Plan execution timeline, phase history, episode timeline

### P6: IsometricGrid
- 2.5D grid with diamond-shaped cells using half-block characters
- Fixed camera angle, per-cell color/height/symbol
- Isometric projection: `sx = (gx - gy) * cell_w/2`, `sy = (gx + gy) * cell_h/4 - value * cell_h/2`
- **Roko use:** Workspace activity territory map, multi-plan status overview

### P7: GlobeWireframe
- Sphere via latitude/longitude lines, animated rotation
- Half-block for filled arc segments
- Surrounding data panels at N/E/S/W/NE/NW/SE/SW anchor positions
- Longitude projects as sinusoidal curve: `lon.cos() * phi.sin()`
- **Roko use:** Connectivity visualization, relay network topology

### P8: IrisVisualization
- Concentric ring display with configurable ring count
- Per-ring color, fill style (Solid, Dashed, Dots), arc segmentation
- Inner ring most intense, outermost rotates slowly
- Default "AT field" variant: 4 rings from outer dim to inner bright
- **Roko use:** Safety layer status (corrigibility rings), immune graph visualization

### P9: DensityField
- 2D braille-resolution dot map (each terminal cell = 2x4 dot matrix = 8 sub-pixels)
- Uses Unicode braille U+2800-U+28FF: 256 combinations per character
- Gives 2x horizontal and 4x vertical resolution beyond character grid
- Threshold-based: sub-cells below threshold render empty
- Braille bit mapping: dot_x=0 uses bits 0,1,2,6; dot_x=1 uses bits 3,4,5,7
- **Roko use:** Cognitive load visualization, HDC vector similarity maps, knowledge density

### P10: SequencerGrid
- Multi-track horizontal event grid (retro tracker aesthetic)
- Rows = tracks, cols = time steps, active/inactive per cell
- Current step highlighted with brightness boost
- Characters: `block` (active), `light_shade` (inactive)
- **Roko use:** Plan task execution grid (tasks as tracks, steps as time), agent heartbeat pattern

### P11: PersistenceDiagram
- Scatter plot of topological features from persistent homology
- (birth, death) coordinate pairs; diagonal reference line
- Three homology dimensions: H0 `circle` (clusters), H1 `diamond_outline` (loops), H2 `filled_square` (voids)
- Reference overlay: previous diagram as dim shadow for comparison
- PAD modulation: high arousal makes points pulse at heartbeat
- **Roko use:** Knowledge structure topology, plan dependency structure analysis

### P12: SimilarityLandscape
- 2D topographic heatmap of high-dimensional similarity
- Block characters `light_shade -> medium_shade -> dark_shade -> block` encode height
- Peaks = clusters of similar states, valleys = dissimilar/transitional
- Cursor shows current state position in landscape
- Data sources: HDC similarity, consistency scores, Betti fingerprints, somatic marker clustering
- **Roko use:** Agent behavioral state landscape, knowledge similarity map

### P13: WassersteinRiver (referenced but not fully specced in primitives doc)
- Ribbon visualization where width encodes Wasserstein distance between topological diagrams
- Thin quiet stream = stable structure, wide bright flood = rapid structural change
- **Roko use:** Learning rate visualization, knowledge churn indicator

---

## 3. Transition Specifications (5 Tiers)

Every transition is driven by live state (PAD vector, lifecycle phase, knowledge density,
mortality). No canned animations. The same action looks different depending on agent state.

### Three Laws of Transitions
1. Every transition renders the agent's state, not just the action's state
2. Tier follows significance, not duration
3. Familiarity breeds evolution, not repetition (first-time events are cinematic; repeated events compress)

### Tier 0: Ambient Pulse (50-200ms)
- Every micro-action: FlashNumber updates, border brightening on focus, scanline ripples on events
- The breathing layer -- terminal never snaps, always eases
- PAD modulation: high arousal = faster/sharper, low arousal = slower/softer

### Tier 1: Gesture (200ms-1s)
- Every deliberate navigation action
- Tab switch: horizontal scan line sweeps in navigation direction, old content dissolves, new materializes
- Pane focus: newly focused pane border brightens with outward pulse (300ms fade); thin traveling line connects old/new focus for one frame
- List scroll: items fade in from 0.3 opacity; items scrolling out leave 100ms phosphor ghost; fast scroll accumulates blur trail
- Form field activation: vertical scan line expands from cursor 2-3 cells, then contracts

### Tier 2: Passage (0.5-3s)
- Major navigation: entering a view, opening a modal, switching windows
- The Threshold (entering a view): list collapses toward selected entry, entry expands, identity sigil assembles from characters, sigil dissolves into first tab
- The Descent (opening modal): selected element brightens to bone, surrounding content dims (clear text to shade characters), modal expands from element position
- The Shift (switching windows): 6 transition types (Crossfade, HorizontalSlide, VerticalDissolve, RadialWipe, GlitchCut, FadeThrough) with state-driven modulation

### Tier 3: Moment (2-8s)
- Significant events: execution, perspective query, phase transition
- The Trade Pulse: ring of box-drawing characters expands and dissolves from confirmation point; data "freezes" for 200ms (crystallization effect)
- Confirmation Arrival: profitable = warm upward sweep, loss = cool downward settle, neutral = lateral ripple
- Phase Transition: border color wave propagates from sidebar outward across screen (1s to cross full width)
- All Tier 3 moments are interruptible (any key dismisses)

### Tier 4: Cinematic (5-15s, rare)
- First-time events, milestones, major achievements
- Full screen takeover, skippable after 2 seconds
- The First Protocol: screen dims to void, sigil assembles character-by-character, philosophical text appears
- Achievement Unlock: scattered particles converge to icon, description renders letter-by-letter
- Protocol identity sigils: unique character-art patterns per entity (not logos, but visual metaphors)

### Novelty Engine
- Every action has a novelty score (0.0 = routine, 1.0 = never seen)
- Score determines transition tier: 0.0-0.2 = Tier 0-1, 0.2-0.5 = Tier 2, 0.5-0.8 = Tier 3, 0.8-1.0 = Tier 4
- Novelty decays: `novelty = base * (1 / (1 + log(occurrence_count)))`
- Four experience stages: Discovery (0-5 visits, full cinematic) -> Familiarity (5-20, compressed) -> Mastery (20-100, instant with flash) -> Home (100+, nearly invisible)
- Novelty can re-spike on unusual conditions (e.g., first time seeing something during a crisis)

### Atmospheric Stack (Always Active)
```
Layer 7: OVERLAYS          Confirmations, alerts, help, command palette
Layer 6: FRAGMENTS         Philosophical whispers, epigraphs
Layer 5: DATA              Text, numbers, widgets -- the screen content
Layer 4: PANE BORDERS      Box-drawing frames
Layer 3: ENVIRONMENTAL     Data rain, power lines, convergence wires, particles
Layer 2: NOISE FLOOR       Sparse block/dot shimmer
Layer 1: SCANLINES         Alternating row bg dimming
Layer 0: VOID              bg_void base color
Layer -1: SPECTRAL TRACES  Ghosts of previous screens
Layer -2: CRT SUBSTRATE    Phosphor persistence, burn-in
```

### Noise Floor
- Sparse random characters (light_shade, medium_shade, dot, small_dot) shimmer at background layer
- Density: 0.3% (Thriving) to 2.0% (Terminal) of cells per frame
- Warm noise (rose-shifted) for high-stress states, cool noise (indigo-shifted) for calm/dream
- Characters persist 1-3 frames then return to void
- High arousal increases density by 50% and shifts toward denser block characters

---

## 4. Spatial Grammar (Layout System)

### Zone Architecture (5 Zones)
```
+--------------------------------------------------+
|                    HEAD ZONE                       |
|            (top 2-3 rows: labels, phase)           |
+--------------------------------------------------+
|                                                    |
|                  CHEST ZONE                        |
|           (upper center: primary status)           |
|                                                    |
+-----------+------------------------+---------------+
| LIMB ZONE |       GUT ZONE        | LIMB ZONE     |
| (left 2-3 |    (lower center:     | (right 2-3    |
|  cols)     |     secondary data)   |  cols)        |
+-----------+------------------------+---------------+
|                  GROUND ZONE                       |
|            (bottom 2 rows: status bar)             |
+--------------------------------------------------+
```

| Zone | Default Height | Density | PAD Response |
|------|---------------|---------|--------------|
| HEAD | 2-3 rows | HIGH | Brightens with Pleasure, sharpens with Dominance |
| CHEST | 25-35% | MEDIUM | Color temperature shifts, heartbeat pulse manifests here |
| GUT | 35-50% | VARIABLE | Somatic pre-signals appear before conscious decisions in HEAD |
| LIMB | 2-3 cols each | MEDIUM | Border weight shifts with Dominance |
| GROUND | 2 rows | HIGH | Dims with Arousal drop; hazard stripes in crisis |

### Persistent Chrome
- **Window Bar (HEAD row 0):** Active window framed `[ WINDOW ]` in rose, inactive in dim
- **Tab Bar (HEAD row 1):** `1:Tab1  2:Tab2  3:Tab3` with number keys
- **Status Bar (GROUND row -2):** Phase indicator, tick counter, balance, breadcrumb, heartbeat
- **Command Bar (GROUND row -1):** Top 5-8 contextual keys for current depth

### Responsive Breakpoints
| Width | Sidebar | Layout | Modal Size |
|-------|---------|--------|-----------|
| < 80 cols | Hidden | Single column, stacked | 95% width |
| 80-119 | 6 cols (minimal) | Single column | 90% width |
| 120-159 | 10 cols (mini) | Two columns | 70% width |
| 160-199 | 12 cols (full) | Three columns | 60% width |
| 200+ | 14 cols (full + waveforms) | Three cols + detail sidebar | 50% width |

### Panel Border States
```
NORMAL (unfocused):    single line, dim
FOCUSED:               single line, bright
LOCKED:                double line, bright (double-line box drawing)
MODAL:                 double line, rose_bright
DEGRADED (dashed):     dashed line, dim, broken
```

### Nesting Rules
- Maximum 4 visual nesting levels before abstraction (collapse into tabs)
- Level 1: Screen pane (standard border)
- Level 2: Sub-pane (thin dim border)
- Level 3: Card/element (background tint only)
- Level 4: Inline indicator (color distinction only)

### 50% Void Rule
Interior content areas should have at least 50% of cells void or near-void.
Density concentrates at margins, headers/footers, and active data elements.
The interior is sparse. This creates the depth illusion.

### 8 Compositional Patterns
| Pattern | Description | Use |
|---------|-------------|-----|
| **Anchor + Surround** | Center element with orbital data panels at N/E/S/W | Overview screens, primary metric with context |
| **Triptych** | Three equal columns, no center emphasis | Consensus panels, comparison views |
| **Asymmetric Focus** | 65/35 split, dense content + sidebar | List/preview, cards + NAV strip |
| **Corner-Heavy** | Four strong corner labels with void center | Primary metric framing |
| **Horizon Split** | Strong horizontal mid-line, different register above/below | Above = conscious/structured, below = ambient/texture |
| **Lattice** | Uniform grid, equal cell weight | Probe grids, achievement grids, peer tiles |
| **Cascade** | Stacked priority, most important fills top | Single-metric focus views |
| **Void Pool** | Single element in wide void, meaning through isolation | Phase transitions, countdowns |

### Depth Planes (5 Layers via Brightness)
| Depth | Layer | Brightness | Characters | Colors |
|-------|-------|-----------|-----------|--------|
| 0 (deepest) | Deep void | 0.00-0.05 | braille_empty, dot | text_phantom |
| 1 | Background fill | 0.05-0.15 | light_shade, dash, + | text_ghost |
| 2 | Midground | 0.15-0.45 | medium_shade, box_vertical, box_horiz | rose_dim/text_dim |
| 3 | Foreground | 0.45-0.85 | dark_shade, block, box_horiz, box_vert | rose |
| 4 (surface) | Accents | 0.85-1.00 | block, star, circle | bone/rose_bright |

Rule: never more than 5% of cells at surface brightness.

### 6 Modal Types
| Type | Size | Layout | Use |
|------|------|--------|-----|
| Detail | 60-75% W, 70-85% H | Content top, metadata grid bottom | Entry details, trade details |
| Comparison | 80% W, 65% H | Two-column side by side | Before/after, A/B comparisons |
| Timeline | 90% W, 40% H | Horizontal timeline primary axis | History, phase timeline |
| Graph | 70% W, 70% H | Interactive graph filling modal | Causal graphs, topology |
| Editor | 50% W, 60% H | Form-style: name, value, slider | Config parameters, tuning |
| Nooscopy | 70-85% W, 85-90% H | Five structured sections, breathing border | Decision approval |

Modal backdrop: background dims to 40% brightness. Sidebar remains visible.

### FocusStack + Breadcrumb
Hierarchical navigation with layer tracking:
```
Layer 0: Window  (root, back() is no-op)
Layer 1: Tab
Layer 2: Pane
Layer 3: PaneLocked
Layer 4: Element
Layer 5: Modal (supports infinite nesting)
```
Breadcrumb renders as `WINDOW > Tab > Pane [LOCKED]` in status bar.

---

## 5. Mathematical Visualization Concepts (Math-to-Metaphor)

The system translates every mathematical construct into a user-facing metaphor.
Users never need to understand the math. Three tiers of detail:

### Three-Tier Detail Hierarchy
| Tier | Name | Effort | What You See |
|------|------|--------|-------------|
| 1 | Overview | Zero | Color, rhythm, atmosphere (metaphors only) |
| 2 | Drill-Down | Navigate | Named widgets with labels + numbers (metaphor + value) |
| 3 | Deep-Dive | Enter on element | Full math: data tables, formulas, computation traces |

### Math-to-Metaphor Translation Table
| Technical Term | User Metaphor | Widget |
|----------------|---------------|--------|
| Bayesian surprise | Unexpectedness score | FlashNumber intensity |
| Persistence diagram | Market structure map | PersistenceDiagramWidget |
| Wasserstein distance | Structure change rate | WassersteinRiver ribbon width |
| KL divergence | Belief shift | Surprise burst cinematic |
| Betti numbers | Feature count | FlashNumber inline counters |
| Somatic marker | Gut feeling | SomaticMarkerPanel |
| Hedge weights | Signal trust levels | ConfidenceBar per module |
| Ergodicity gap | Hidden risk | MortalityGauge variant |
| Sheaf consistency | Agreement score | Screen color harmony |
| Phi (integrated information) | Coherence | Spectre cloud cohesion + gauge arc |
| UMAP projection | Similarity map | SimilarityLandscape |
| HDC cosine similarity | Pattern resemblance | Proximity in SimilarityLandscape |
| Causal DAG | Cause-effect chain | ForceGraph |

### Metaphor Consistency Rules (same math = same visual, always)
| Pattern | Visual Metaphor |
|---------|----------------|
| Fragmentation | Spatial dispersal (scatter, disperse, separate) |
| Contradiction | Color temperature split (warm vs cool zones) |
| Understanding | Convergence (fragments draw together) |
| Danger | Contraction (tighten, shrink, constrict) |
| Health | Breathing rhythm (steady, calm oscillation) |

### Invertibility Guarantee
Every visual metaphor is traceable back to its data source. Enter on any ambient signal
opens its numeric value (Tier 2), Enter again opens full mathematical detail (Tier 3).
No hidden state. No information discarded.

---

## 6. Oracle Surface Concepts (Predictive/Status Visualization)

### 4-Level Progressive Disclosure
| Level | Name | Effort | Where |
|-------|------|--------|-------|
| 0 | Ambient | Zero interaction | Every screen |
| 1 | Overview | 10-second scan | Dedicated prediction screens |
| 2 | Detail | Lock and interact | Modal drills from Level 1 |
| 3 | Deep | Modal within modal | Full statistical detail |

### Level 0: Ambient Signals (always visible, no navigation required)
- **Particle coherence:** Agent sprite particle aura smoothness encodes prediction quality (coherent orbits = good, chaotic Brownian = bad, snapping particles = learning/improving)
- **Breathing modulation:** Second harmonic on heartbeat sine wave; in-phase when improving (deeper breathing), out-of-phase when declining (irregular breathing). Deliberately sub-perceptual (15% amplitude).
- **Status bar indicator:** Single accuracy number, color-coded: bone (>70%), warning (50-70%), rose_bright (<50%)
- **Decision ring glow:** Correct resolution = success flash (200ms + 1s phosphor fade). Incorrect = dim rose flash. Gate violation = amber sparks.
- **Heartbeat log gutter:** Single-character column: `dot` = stable, `up_arrow` = improved, `down_arrow` = declined

### Level 1: Overview (10-second scan)
- Per-category accuracy bars (ProbeGauge), sorted by accuracy, with ECE calibration dots
- Attention forager tier counts: ACTIVE/WATCHED/SCANNED as thin gauges
- Action gate status: per-category open/closed as filled/empty blocks
- Recent resolutions: 5 most recent prediction outcomes with check/cross marks

### Level 2: Detail (lock + interact)
- 30-day accuracy sparklines (braille, 4 rows tall)
- Per-regime accuracy breakdown table
- Calibration curve (20x10 braille): ideal diagonal vs actual curve, gap highlighted
- Position retrospective: PnL trajectory chart, entry reasoning, vs-inaction analysis
- Heuristic audit: citation timeline sparkline, per-citation PnL, promotion/demotion recommendation

### Level 3: Deep (full math)
- Full calibration curve (40x20 braille canvas, 80x80 effective resolution)
- Residual distribution histogram (block characters)
- Per-category accuracy time series (multi-line braille sparklines)
- ECE, ACE, Max CE statistics

### Progressive Complexity State Machine
- NOVICE (< 20 events): Only overview tab visible
- INTERMEDIATE (50+ events): Overview + categories
- EXPERT (200+ events, accuracy > 0.55): All tabs visible, permanent (no demotion)

---

## 7. Pixel-Level Rendering Techniques Using Terminal Unicode

### Braille Dot Matrix (DensityField)
- Each terminal cell = 2x4 dot matrix = 8 independently controllable sub-pixels
- Unicode U+2800-U+28FF: 256 possible patterns per character
- Effective resolution: 2x horizontal, 4x vertical beyond character grid
- Bit positions: column 0 = bits 0,1,2,6; column 1 = bits 3,4,5,7
- Used for: density maps, calibration curves, sparklines, similarity landscapes

### Block Characters (height encoding)
- `['_','_','_','_','_','_','_','_']` (Unicode block elements lower-1/8 through full block)
- Used for: waveform displays, histogram bars, gauge fills

### Half-Block Characters (sub-cell precision)
- Upper half block, lower half block for 2x vertical resolution
- Used for: isometric grid diagonal edges, filled arc segments on globe wireframe

### Shade Characters (depth/opacity encoding)
```
Space  -> Light shade -> Medium shade -> Dark shade -> Full block
(void)    (background)   (midground)    (foreground)   (accent)
```
Used for: depth planes, thermal fields, degradation effects

### Box-Drawing Characters (border states)
```
Normal:   single line (dim)
Focused:  single line (bright)
Locked:   double line (bright)
Degraded: dashed line (broken)
```
Border weight encodes Dominance: thin dash -> single -> double -> heavy double

### Phosphor Decay Chain (aging/dimming)
```
bone (brightest) -> rose -> rose_dim -> text_ghost -> text_phantom (invisible)
```
Applied to: log entries, knowledge entries, confirmation effects. Each step is a
discrete brightness level, not a smooth gradient.

### CRT Effects
- Scanlines: alternating row background dimming (0.05-0.50 intensity by phase)
- Noise floor: sparse random characters shimmer 1-3 frames at background layer
- Phosphor persistence: bright elements leave afterimage through decay chain

---

## 8. Perspective/Nooscopy Concepts (Agent Introspection Overlays)

### Perspective System (F2 toggle)
- Overlays the agent's inner thoughts as floating text annotations on any data screen
- Knowledge Drawer opens on right (30% width) with structured categories:
  Episodes, Insights, Heuristics, Warnings, Signals, Skills, Somatic Markers, Causal Links, Thinking
- Floating annotations are NOT tooltips -- they are "intrusive thoughts" rendered as text
- Each fragment has independent drift (gentle sine/cosine sway), breathes with heartbeat
- Fragments fade in letter-by-letter (40ms/char), have lifetime (8-20s), fade out same way
- PAD modulates annotation color, opacity, drift speed, and phrasing confidence

### Nooscopy Modal (Agent-Initiated Decision Approval)
- Agent reaches outward when a decision exceeds autonomous authority
- 5-section modal: Proposed Action, Hypothesis, Evidence, Risks, Alternatives Considered
- Countdown timer bar with breathing animation at heartbeat rate
- 4 actions: Interrogate (pause timer, open chat), Modify (edit parameters), Approve, Reject
- Interrogation mode: conversation with the agent about its reasoning, citations as portals
- Breathing double-line border shimmers at heartbeat rate (the boundary is alive)

### Knowledge Drawer Item Types
| Type | Glyph | Display |
|------|-------|---------|
| Episodes | triangle_right | Episode #, title, confidence bar, age, emotion, outcome |
| Insights | triangle_right | Insight text, confidence bar |
| Heuristics | triangle_right | Section ref, rule text, confidence bar |
| Warnings | dagger | Generation + ID, content, confidence, decay rate |
| Signals | zap/warning | Signal type, domain, chain, confirmation count, decay % |
| Skills | gear | Skill name, activation status, trigger terms, tools |
| Somatic Markers | circle | Polarity, description, firing condition, PAD effect |
| Causal Links | therefore | Linked variables, confidence, evidence count, direction |
| Thinking (Opus) | diamond_outline | Chain-of-thought trace, token count |

---

## 9. Actionable Catalog: Things to Build for Roko TUI

### Tier 1: Core Rendering Infrastructure (needed by everything else)

| # | Item | What | Bardo Source |
|---|------|------|-------------|
| 1.1 | `DataPrimitive` trait | `render(&self, area, buf, ctx)` + `tick(&mut self, dt)` with `AnimationContext` | Viz primitives overview |
| 1.2 | `AnimationContext` struct | Elapsed time, heartbeat phase, PAD vector, palette reference | Viz primitives overview |
| 1.3 | Phosphor decay chain | Color interpolation: bone -> rose -> dim -> ghost -> phantom with configurable decay rate | Hearth heartbeat log |
| 1.4 | FlashNumber widget | Number that flashes on change, lerps between values | Hearth vitality, cost accumulators |
| 1.5 | Zone layout system | 5-zone split (HEAD/CHEST/GUT/LIMB/GROUND) from terminal Rect | Spatial grammar A |
| 1.6 | PaneGrid + BorderState | Pane focus/lock/degrade with border style changes | Spatial grammar C |
| 1.7 | ModalLayer + modal stack | Push/pop modals with backdrop dimming, 6 modal types | Spatial grammar E |
| 1.8 | FocusStack + breadcrumb | Layer tracking (Window/Tab/Pane/Locked/Element/Modal) | Spatial grammar H |
| 1.9 | Responsive breakpoints | 5 width breakpoints controlling sidebar width and column count | Spatial grammar B |

### Tier 2: Visualization Primitives (add one at a time)

| # | Item | Priority | Notes |
|---|------|----------|-------|
| 2.1 | WaveformDisplay | HIGH | Simplest: rolling sparkline with phosphor decay. Use for metrics over time. |
| 2.2 | DensityField (braille) | HIGH | Highest information density. Braille bit-packing gives 8 sub-pixels per cell. |
| 2.3 | ForceGraph | HIGH | Knowledge graph, plan DAG, agent topology. Verlet physics, never settles. |
| 2.4 | TimelineRibbon | HIGH | Plan execution timeline, episode history. Colored segments + cursor. |
| 2.5 | ThermalField | MEDIUM | Heatmaps for activity, knowledge density. Multiple colormaps. |
| 2.6 | RadarDisplay | MEDIUM | Multi-dimensional agent health at a glance. |
| 2.7 | SequencerGrid | MEDIUM | Retro tracker aesthetic for multi-track event display. |
| 2.8 | IrisVisualization | LOW | Concentric rings for status/safety layer visualization. |
| 2.9 | IsometricGrid | LOW | 2.5D grid for territory/activity visualization. |
| 2.10 | GlobeWireframe | LOW | Network topology. Expensive to render. |
| 2.11 | PersistenceDiagram | LOW | Topological analysis scatter plot. Niche but visually distinctive. |
| 2.12 | SimilarityLandscape | LOW | Topographic heatmap variant. Roko has HDC data to feed this. |

### Tier 3: Transition System

| # | Item | Priority | Notes |
|---|------|----------|-------|
| 3.1 | Tier 0 ambient easing | HIGH | No snapping. All value changes ease/lerp. FlashNumber on every numeric change. |
| 3.2 | Tier 1 tab switch scan line | HIGH | Horizontal sweep in navigation direction, old dissolves, new materializes. |
| 3.3 | Tier 1 focus pulse | HIGH | Border brighten with outward pulse on focus, dim on unfocus (150ms). |
| 3.4 | Tier 1 list scroll phosphor | MEDIUM | Scroll-in fade (0.3 opacity), scroll-out phosphor ghost (100ms). |
| 3.5 | Tier 2 modal descent | MEDIUM | Selected element brightens, surroundings dim (text -> shade chars), modal expands from element. |
| 3.6 | Tier 2 window shift | MEDIUM | Crossfade/slide between windows with state-driven modulation. |
| 3.7 | Novelty engine | LOW | Track occurrence counts, compute novelty score, select transition tier. |
| 3.8 | Tier 3/4 cinematics | LOW | Full-screen takeover for milestones. Skippable. |

### Tier 4: Screen-Specific Features

| # | Item | Roko Tab | Notes |
|---|------|----------|-------|
| 4.1 | Heartbeat-style event log | F1 Overview | Phosphor-decay log with tier-based coloring (dim for routine, bright for significant). Variable-ratio rhythm. |
| 4.2 | Pipeline visualization | F3 | Context assembly view: items stream from module positions toward center. Module connection lines with traveling dots. |
| 4.3 | Knowledge graph view | F4 | ForceGraph of neuro store entries. Type glyphs as nodes, confidence as edge thickness. Toggle with `g`. |
| 4.4 | Playbook document view | F6 | Inline confidence sparklines per heuristic. Ghost text for deleted entries. Diff view for recent changes. |
| 4.5 | Cost/inference dashboard | F5 | Tier breakdown columns, cost sparklines, cache hit rate gauge. |
| 4.6 | Agent constellation | F10 | ForceGraph of agents. Color = role, size = activity, edges = group membership. |
| 4.7 | Chat integration | TUI | Streaming token-by-token display. Right-aligned owner messages, left-aligned agent responses. |
| 4.8 | Decision approval modal | TUI | Structured 5-section layout. Countdown bar. Interrogation sub-mode. |

### Tier 5: Advanced/Atmospheric (polish layer)

| # | Item | Notes |
|---|------|-------|
| 5.1 | Noise floor | Sparse random characters shimmer at background layer (0.3-2.0% density) |
| 5.2 | Scanline effect | Alternating row background dimming (configurable intensity) |
| 5.3 | Depth planes | 5 brightness layers creating implied 3D depth |
| 5.4 | Three-tier detail hierarchy | Metaphor-only -> metaphor+number -> full math per widget |
| 5.5 | Phase degradation effects | Border corruption, character glitching at low health/vitality |
| 5.6 | Knowledge Drawer (Perspective) | Right-side 30% panel with categorized knowledge items |
| 5.7 | Floating annotations | Drifting text fragments overlaid on data views |
| 5.8 | Border weight by state | Dominance modulates border weight: dashed -> single -> double |

---

## 10. Key Unicode Character Reference

### Block Elements
```
Light shade:  U+2591  (depth 1, background)
Medium shade: U+2592  (depth 2, midground)
Dark shade:   U+2593  (depth 3, foreground)
Full block:   U+2588  (depth 4, accent)
Lower blocks: U+2581-U+2587  (1/8 through 7/8, waveform heights)
Upper half:   U+2580  (isometric edges)
Lower half:   U+2584  (isometric edges)
Left half:    U+258C  (vertical splits)
Right half:   U+2590  (vertical splits)
```

### Braille (DensityField)
```
Empty:   U+2800  (no dots)
Full:    U+28FF  (all 8 dots)
Range:   U+2800-U+28FF  (256 patterns, 2x4 dot matrix per cell)
```

### Box Drawing (Borders)
```
Single:    U+2500 (horiz), U+2502 (vert), U+250C/U+2510/U+2514/U+2518 (corners)
Double:    U+2550/U+2551/U+2554/U+2557/U+255A/U+255D
Dashed:    U+254C/U+254E (single dashed horiz/vert)
Heavy:     U+2501/U+2503
```

### Data Glyphs
```
Episodes:   U+25C6 (filled diamond)
Insights:   U+25C7 (empty diamond)
Heuristics: U+25CF (filled circle)
Skills:     U+26A1 (zap)
Warnings:   U+26A0 (warning triangle)
Causal:     U+2234 (therefore)
Dead:       U+2020 (dagger)
Star:       U+2726 (four-pointed star)
Heartbeat:  U+223F (sine wave)
```

### Progress/Status
```
Filled bar:   U+25AE (filled rectangle vertical)
Empty bar:    U+25AF (empty rectangle vertical)
Filled gauge: U+25B0 (filled parallelogram)
Empty gauge:  U+25B1 (empty parallelogram)
Phase dot:    U+25CF (filled), U+25D0 (half-filled), U+25CB (empty)
```

---

## Source Files Read

1. `/Users/will/dev/uniswap/bardo/prd/18-interfaces/screens/01-screen-specs.md` -- 700 lines, full 29-screen specification across 6 windows
2. `/Users/will/dev/uniswap/bardo/prd/18-interfaces/screens/04-oracle-surfaces.md` -- 500+ lines, 4-level progressive disclosure for prediction/evaluation data
3. `/Users/will/dev/uniswap/bardo/prd/18-interfaces/screens/05-math-metaphor.md` -- 175 lines, math-to-metaphor translation table and 3-tier detail hierarchy
4. `/Users/will/dev/uniswap/bardo/prd/18-interfaces/rendering/02-visualization-primitives.md` -- 1000+ lines, 13 visualization primitives with Rust structs and render skeletons
5. `/Users/will/dev/uniswap/bardo/prd/18-interfaces/rendering/03-transitions.md` -- 500+ lines, 5-tier transition system with novelty engine and atmospheric stack
6. `/Users/will/dev/uniswap/bardo/prd/18-interfaces/19-spatial-grammar.md` -- 477 lines, zone architecture, panel hierarchy, depth planes, 8 compositional patterns
7. `/Users/will/dev/uniswap/bardo/prd/18-interfaces/perspective/00-nooscopy.md` -- 577 lines, agent-initiated decision approval modal with interrogation flow
8. `/Users/will/dev/uniswap/bardo/prd/18-interfaces/perspective/01-golem-perspective.md` -- 677 lines, F2 perspective overlay with floating annotations and knowledge drawer
