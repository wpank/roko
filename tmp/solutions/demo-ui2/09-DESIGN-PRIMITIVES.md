# 09. Composable Design Primitives

Reusable, themed, composable building blocks. Everything flows from tokens. Change one file, change everything. Every container handles loading, empty, error, and success states. Every transition is visible and progressive.

**Governing principles** (cross-reference `04-DESIGN-SYSTEM.md` sections 8-10 and `02-ARCHITECTURE.md` section 6):

- **Scrollable container layout model.** Pages scroll -- they are never viewport-locked. TopNav is sticky; everything below it lives in an `overflow-y: auto` container with content-determined height. No `min-height: 100vh` on page sections.
- **Space efficiency / density-first.** Tight padding (10-12px inside cards, 8px gaps between siblings, 16px between sections). No wasted space. Content fills available width. Empty states collapse to 48px, not 120px. See the "Density Guidelines" sub-section below and `04-DESIGN-SYSTEM.md` section 10 for concrete values.
- **Terminal / demoscene aesthetic.** All chrome uses `var(--mono)` uppercase with letter-spacing. ASCII vocabulary for labels, dividers, and frames: `⌈ LABEL ⌋`, box-drawing chars (`─│┌┐└┘`), braille density fills (`⠋⠙⠹`), block elements for inline charts (`▁▂▃▄▅▆▇█`), status glyphs (`●○◉◐◑ ✓✕`). Phosphor decay for value changes. See `04-DESIGN-SYSTEM.md` section 9.
- **Component categories.** Primitives are organized into: `design/` (visual atoms), `layout/` (structural wrappers), `cells/` (entity renderers), `motion/` (animation tokens/wrappers), `ascii/` (terminal aesthetic). Future categories: `ambient/` (WebGL backgrounds), `agent/` (agent-namespaced widgets), `feeds/` (event streams), `overlay/` (modals/drawers). See `02-ARCHITECTURE.md` section 6.3 for the full tree.

### Density Guidelines

Concrete padding/gap values for density-first layout. These are the canonical reference; components must follow these constraints.

| Property | Value | Notes |
|----------|-------|-------|
| Card internal padding | 10px 12px | Tight, not airy |
| Gap between sibling cards | 8px | 1px for mosaic cells, 8px standard |
| Gap between sections | 16px | Sections are visually distinct groups |
| Page horizontal padding | 16px | Edge-to-edge density |
| Metric strip height | auto (content) | No min-height |
| Card border radius | 6px | Slightly tighter than 8px |
| Label-to-value gap | 2px | Tight coupling |
| Tab bar padding | 0 | Tabs touch the content below |
| Empty state height | 48px | Compact, not 120px |
| Mosaic cell padding | 16px 14px | Reduced from 30px 28px |
| ScrollArea fade gradient | 24-40px | At top/bottom when content is scrollable |

**Anti-patterns** (do not do these):
- `min-height: 50vh` on content areas
- Large padding on page containers (>24px)
- Cards with more padding than content
- Empty space below content "for balance"
- Fixed-height sections that do not fill with content
- Viewport-locking (`height: 100vh`) on page-level containers

### Complete Primitives Catalog (by category)

All primitives created or specified across this doc and `10-EXPRESSIVE-PRIMITIVES.md`:

| Category | Directory | Primitives |
|----------|-----------|------------|
| **design** | `components/design/` | StatusBadge, EmptyState, ErrorState, GateBar, Skeleton, Pulse, Badge |
| **layout** | `components/layout/` | DataSurface, Stack, PageShell, SplitView, Tabs, ScrollArea, PhaseRail, MetricStrip |
| **cells** | `components/cells/` | Cell, CellGrid, TaskCell, AgentCell, PlanCell, EpisodeCell, BenchRunCell |
| **motion** | `components/motion/` | tokens, AnimatedNumber, AnimatedList, Transition |
| **ascii** | `components/ascii/` | AsciiLabel, AsciiDivider, AsciiFrame, AsciiBraille, AsciiProgress, AsciiWaveform |

Detailed specs for each primitive are in sections 1-11 below. Advanced/higher-order primitives (wave 2-4) are cataloged in section 12 and fully specified in `10-EXPRESSIVE-PRIMITIVES.md`.

---

## 0. Design Philosophy Adjustments

The bardo TUI ROSEDUST palette was designed for terminal viewing at close range with 12px mono text. The web UI needs:

| Concern | Bardo TUI | Web UI (adjusted) |
|---------|-----------|-------------------|
| Base text | `#d8ccd8` (low contrast) | `#e8dce8` (higher contrast, ~7:1 vs void) |
| Strong text | `#f0e8f0` | `#f8f0f8` (near-white, ~12:1 vs void) |
| Soft text | `#baa8b2` | `#c8b8c4` (readable on dark panels) |
| Dim text | `#8a7a88` | `#9a8a98` (still readable at 11px) |
| Body font size | 15px | 16px (min body size for readability) |
| Label size | 11px | 12px (min label size) |
| Value display | 28px | 32-38px (hero metrics should command attention) |
| Line height | 1.55 | 1.6 (more breathing room) |
| Font weight | 300-400 | 400-500 (bolder for screen legibility) |

### Updated Token Overrides

```css
:root {
  /* ── BRIGHTER TEXT (higher contrast on dark bg) ── */
  --text-primary: #e8dce8;    /* was #d8ccd8 — body text */
  --text-strong: #f8f0f8;     /* was #f0e8f0 — headings, emphasis */
  --text-soft: #c8b8c4;       /* was #baa8b2 — secondary content */
  --text-dim: #9a8a98;        /* was #8a7a88 — labels, metadata */
  --text-ghost: #605060;      /* was #504050 — placeholders, hints */

  /* ── LARGER TYPE SCALE ── */
  --text-2xs: 10px;           /* NEW — sparkline labels, tiny metadata */
  --text-xs: 12px;            /* was 11px — mono labels, badges */
  --text-sm: 14px;            /* was 13px — secondary body, nav */
  --text-md: 16px;            /* was 15px — body text */
  --text-lg: 18px;            /* was 17px — emphasis text */
  --text-xl: 24px;            /* same — sub-headings */
  --text-2xl: 32px;           /* NEW — metric values */
  --text-3xl: 38px;           /* NEW — hero metric values */
  --text-display: clamp(48px, 5vw, 72px); /* NEW — page titles */

  /* ── BOLDER WEIGHTS ── */
  --weight-normal: 400;       /* body */
  --weight-medium: 500;       /* labels, nav items */
  --weight-semibold: 600;     /* headings, values */
  --weight-bold: 700;         /* hero metrics */
}
```

---

## 1. Container Primitives

Every container in the system derives from one of these 6 base patterns. All are themed via CSS custom properties. All handle state.

### 1.1 `<Panel>` — The Universal Container

Replaces current `<Pane>` with full state lifecycle.

```tsx
interface PanelProps {
  // Identity
  label?: string;           // Mono uppercase header label
  icon?: string;            // Icon name or emoji
  badge?: ReactNode;        // Top-right badge slot

  // Appearance
  variant?: 'glass' | 'solid' | 'outline' | 'ghost';  // default: 'glass'
  size?: 'sm' | 'md' | 'lg';                           // padding scale
  accent?: 'rose' | 'bone' | 'dream' | 'success' | 'warning' | 'error';
  glow?: boolean;           // Active glow on left border

  // State
  loading?: boolean;        // Shows skeleton shimmer
  empty?: boolean;          // Shows empty state
  emptyMessage?: string;    // "No data yet"
  emptyAction?: string;     // "Run a benchmark to see results"
  error?: string;           // Shows error state with retry
  onRetry?: () => void;

  // Structure
  footer?: ReactNode;       // Bottom slot
  actions?: ReactNode;      // Header-right actions slot
  collapsible?: boolean;    // Can collapse body
  defaultCollapsed?: boolean;

  // Standard
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
  onClick?: () => void;
}
```

**Visual states (each has a distinct appearance):**

```
┌─ LOADING ─────────────────────────────────┐
│ ● BENCHMARKS                              │
├───────────────────────────────────────────┤
│ ░░░░░░░░░░░░░░░░░░░░  (shimmer bar)      │
│ ░░░░░░░░░░░░  (shimmer bar, shorter)      │
│ ░░░░░░░░░░░░░░░  (shimmer bar)            │
└───────────────────────────────────────────┘

┌─ EMPTY ───────────────────────────────────┐
│ ● BENCHMARKS                              │
├───────────────────────────────────────────┤
│                                           │
│         No benchmark runs yet             │
│    Run roko bench start to create one     │  ← action hint
│                                           │
└───────────────────────────────────────────┘

┌─ ERROR ───────────────────────────────────┐
│ ● BENCHMARKS                              │
├───────────────────────────────────────────┤
│                                           │
│     Failed to load benchmark data         │
│     [RETRY]  (button, rose accent)        │  ← retry callback
│     GET /api/bench/runs → 500             │  ← detail (collapsed)
│                                           │
└───────────────────────────────────────────┘

┌─ CONTENT (default) ───────────────────────┐
│ ● BENCHMARKS              3 runs ↗        │  ← label + badge
├───────────────────────────────────────────┤
│                                           │
│  (children rendered here)                 │
│                                           │
├───────────────────────────────────────────┤
│  Last updated 2m ago                      │  ← footer
└───────────────────────────────────────────┘
```

**CSS architecture:**
```css
.panel { /* base */ }
.panel--glass { backdrop-filter: blur(12px) saturate(180%); }
.panel--solid { background: var(--bg-raised); }
.panel--outline { background: transparent; border: 1px solid var(--border); }
.panel--ghost { background: transparent; border: none; }
.panel--sm { /* smaller padding */ }
.panel--lg { /* larger padding */ }
.panel--accent-rose { border-left: 2px solid var(--rose-dim); }
.panel--glow { box-shadow: var(--glow-rose-soft); }
.panel--loading .panel__body { /* shimmer overlay */ }
```

### 1.2 `<Card>` — Clickable Content Unit

For items in lists, grids, selection interfaces. Always interactive.

```tsx
interface CardProps {
  // Identity
  label?: string;
  icon?: ReactNode;
  badge?: ReactNode;

  // Interaction
  selected?: boolean;
  active?: boolean;        // Currently executing/live
  disabled?: boolean;
  onClick?: () => void;

  // Status
  status?: 'idle' | 'active' | 'success' | 'error' | 'blocked';

  // Content
  children: ReactNode;
  footer?: ReactNode;
}
```

**Interactive states:**
```
DEFAULT:    border-color: var(--border)
HOVER:      translateY(-2px), border-color: var(--border-strong), soft glow
SELECTED:   border-left: 2px solid var(--rose-bright), bg: var(--rose-deep)
ACTIVE:     pulsing left border, status dot, glow
DISABLED:   opacity: 0.5, cursor: not-allowed
```

**Entrance animation:** fadeUp (200ms, 40ms stagger per card in a grid)

### 1.3 `<Surface>` — Data Loading Wrapper

Handles the loading/empty/error lifecycle for ANY child content. Wraps any component.

```tsx
interface SurfaceProps<T> {
  data: T | null | undefined;
  loading?: boolean;
  error?: string | null;
  onRetry?: () => void;

  // Empty state config
  emptyWhen?: (data: T) => boolean;  // default: Array.isArray(data) && data.length === 0
  emptyMessage?: string;
  emptyAction?: string;
  emptyIcon?: ReactNode;

  // Loading config
  skeleton?: 'lines' | 'grid' | 'chart' | 'table';  // skeleton shape
  skeletonRows?: number;

  // Render
  children: (data: T) => ReactNode;
}
```

**Usage everywhere:**
```tsx
<Surface data={benchRuns} loading={loading} error={error} skeleton="table" skeletonRows={5}
  emptyMessage="No benchmark runs" emptyAction="Start one with roko bench start">
  {(runs) => <RunTable runs={runs} />}
</Surface>
```

### 1.4 `<Modal>` — Overlay Container

5 sizes matching bardo TUI modal types, with transitions.

```tsx
interface ModalProps {
  open: boolean;
  onClose: () => void;
  size?: 'sm' | 'md' | 'lg' | 'xl' | 'full';  // sm=400px, md=600px, lg=800px, xl=1000px, full=90vw
  title?: string;
  subtitle?: string;
  actions?: ReactNode;      // Footer buttons
  children: ReactNode;
}
```

**Transition:** Backdrop fades in (150ms), modal scales from 0.95→1.0 + fades in (200ms ease-snappy). Close: reverse 150ms.

**Backdrop:** `rgba(6, 6, 8, 0.72)` — matches bardo spec ("background dims to 40% brightness").

### 1.5 `<Drawer>` — Slide-In Panel

For detail views, side panels, configuration.

```tsx
interface DrawerProps {
  open: boolean;
  onClose: () => void;
  position?: 'right' | 'left' | 'bottom';
  width?: string;           // default: '400px'
  title?: string;
  children: ReactNode;
}
```

**Transition:** Slides in from edge (200ms ease-expo), backdrop fades. Push content vs overlay configurable.

### 1.6 `<Split>` — Resizable Split View

For side-by-side content (list + detail, code + terminal).

```tsx
interface SplitProps {
  direction?: 'horizontal' | 'vertical';
  defaultRatio?: number;     // 0-1, default 0.4
  minRatio?: number;         // default 0.2
  maxRatio?: number;         // default 0.8
  left: ReactNode;
  right: ReactNode;
  onResize?: (ratio: number) => void;
}
```

**Divider:** 1px border with 8px hit target, cursor changes on hover. Double-click resets to default.

---

## 2. Feedback & Loading Primitives

Every action shows visible feedback. No silent waiting.

### 2.1 `<Skeleton>` — Placeholder While Loading

```tsx
interface SkeletonProps {
  variant: 'text' | 'heading' | 'metric' | 'chart' | 'table' | 'card' | 'avatar';
  lines?: number;          // for 'text' variant
  rows?: number;           // for 'table' variant
  columns?: number;        // for 'table' variant
  height?: string;         // for 'chart' variant
}
```

**Shimmer effect:** Gradient sweep left→right, 1.8s infinite, subtle (3%→7% opacity on dark bg).

```css
.skeleton {
  background: linear-gradient(90deg,
    var(--bg-raised) 0%,
    rgba(255,255,255,0.04) 50%,
    var(--bg-raised) 100%
  );
  background-size: 200% 100%;
  animation: skeleton-shimmer 1.8s ease-in-out infinite;
  border-radius: var(--radius-sm);
}

.skeleton--text { height: 14px; margin-bottom: 8px; }
.skeleton--heading { height: 24px; width: 60%; margin-bottom: 16px; }
.skeleton--metric { height: 38px; width: 120px; }
.skeleton--chart { height: var(--height, 200px); width: 100%; border-radius: var(--radius-lg); }
```

### 2.2 `<ProgressBar>` — Determinate Progress

```tsx
interface ProgressBarProps {
  value: number;           // 0-100
  label?: string;
  showPercentage?: boolean;
  color?: 'rose' | 'bone' | 'success' | 'dream';
  size?: 'sm' | 'md';     // sm=4px height, md=8px
  animated?: boolean;      // Gradient sweep on fill
}
```

**Fill animation:** Width transitions with ease-snappy (150ms). Gradient sweep overlay on animated fills.

**Glow:** Active fills have subtle glow matching color: `box-shadow: 0 0 8px var(--color)`.

### 2.3 `<ProgressRing>` — Circular Progress

```tsx
interface ProgressRingProps {
  value: number;           // 0-100
  size?: number;           // px, default 48
  thickness?: number;      // stroke width, default 3
  color?: string;
  label?: ReactNode;       // Center content
}
```

**Animation:** stroke-dashoffset transitions with ease-expo (300ms). Track is `var(--border-soft)`.

### 2.4 `<Spinner>` — Indeterminate Loading

```tsx
interface SpinnerProps {
  size?: 'xs' | 'sm' | 'md' | 'lg';  // 12, 16, 24, 32px
  color?: string;
  label?: string;          // Screen reader + optional visible label
}
```

**Animation:** Rotating arc (not full circle) — 1s linear infinite rotation. Arc length pulses (grows/shrinks) for organic feel.

### 2.5 `<Pulse>` — Status Heartbeat

The LED dot from bardo, for any element.

```tsx
interface PulseProps {
  color?: 'rose' | 'bone' | 'success' | 'warning' | 'error' | 'dream';
  size?: number;           // default 6
  active?: boolean;        // Animating or static
  glow?: boolean;          // Box-shadow glow
}
```

**Animation:** 2.4s ease-in-out infinite pulse (opacity 1→0.4→1). Glow box-shadow pulses with same timing.

### 2.6 `<StepProgress>` — Multi-Phase Progress (PhaseRail)

```tsx
interface StepProgressProps {
  steps: string[];
  current: number;         // 0-based index
  failed?: number;         // Index of failed step (-1 = none)
  orientation?: 'horizontal' | 'vertical';
}
```

**Animations:**
- Step completion: dot scales from 0.8→1.0 (200ms), fills green
- Line draw: scaleX 0→1 (300ms ease-out)
- Current step: rose glow + pulse
- Failed: red flash (400ms) + error icon

### 2.7 `<AnimatedNumber>` — Spring-Animated Value

```tsx
interface AnimatedNumberProps {
  value: number;
  format?: (n: number) => string;  // e.g., (n) => `$${n.toFixed(3)}`
  duration?: number;       // spring duration, default 300
  flash?: boolean;         // Flash on change (default true)
}
```

**Animation:** Spring interpolation from old→new value. On change: text briefly flashes `--bone-bright` with text-shadow glow (300ms fade).

### 2.8 `<Transition>` — Generic Enter/Exit Wrapper

```tsx
interface TransitionProps {
  show: boolean;
  enter?: 'fadeUp' | 'fadeIn' | 'scaleIn' | 'slideRight' | 'slideDown';
  exit?: 'fadeDown' | 'fadeOut' | 'scaleOut' | 'slideLeft' | 'slideUp';
  duration?: number;
  delay?: number;
  children: ReactNode;
}
```

**Usage:** Wrap anything that appears/disappears to ensure it always has a transition.

---

## 3. Data Display Primitives

### 3.1 `<MetricGrid>` — Mosaic Replacement

Enhanced Mosaic with loading states, animations, and flexible layout.

```tsx
interface MetricGridProps {
  columns?: 2 | 3 | 4 | 5 | 6;
  children: ReactNode;     // MetricCell children
}

interface MetricCellProps {
  label: string;
  value: ReactNode;        // String, number, or AnimatedNumber
  sub?: string;
  icon?: ReactNode;
  color?: 'rose' | 'bone' | 'dream' | 'success' | 'warning';
  loading?: boolean;       // Individual cell can be loading
  trend?: 'up' | 'down' | 'flat';  // Trend arrow
}
```

**Animations:**
- Cell entrance: staggered fadeUp (40ms per cell)
- Value change: AnimatedNumber spring + flash
- Hover: subtle background gradient shift (150ms)

### 3.2 `<DataTable>` — Universal Table

```tsx
interface DataTableProps<T> {
  data: T[];
  columns: Column<T>[];
  loading?: boolean;
  empty?: string;
  sortable?: boolean;
  onRowClick?: (item: T) => void;
  selectedRow?: string;    // Row ID for highlighting
  keyExtractor: (item: T) => string;
}

interface Column<T> {
  key: string;
  label: string;
  render: (item: T) => ReactNode;
  width?: string;
  sortFn?: (a: T, b: T) => number;
  align?: 'left' | 'right' | 'center';
}
```

**Animations:**
- Row entrance: staggered fadeUp
- Sort: rows rearrange with layout animation (300ms)
- Selected row: rose left border + subtle background
- Loading: skeleton table rows

**Keyboard:** Arrow keys navigate rows, Enter selects, Tab moves between cells.

### 3.3 `<Badge>` — Status Indicator

```tsx
interface BadgeProps {
  label: string;
  variant?: 'status' | 'tier' | 'model' | 'count';
  color?: 'rose' | 'bone' | 'success' | 'warning' | 'error' | 'dream';
  pulse?: boolean;
  icon?: ReactNode;
  size?: 'sm' | 'md';
}
```

**Variants:**
- `status`: dot + label (RUNNING, DONE, FAILED)
- `tier`: T1/T2/T3 with tier color
- `model`: model name with provider icon
- `count`: number in circle

### 3.4 `<Chip>` — Interactive Tag

```tsx
interface ChipProps {
  label: string;
  selected?: boolean;
  onClick?: () => void;
  removable?: boolean;
  onRemove?: () => void;
  icon?: ReactNode;
  color?: string;
}
```

For model selectors, filter toggles, tag lists. Hover: lift + border brighten. Selected: filled background.

### 3.5 `<Sparkline>` — Inline Chart

```tsx
interface SparklineProps {
  data: number[];
  width?: number;          // default 60
  height?: number;         // default 16
  color?: string;
  fill?: boolean;          // Gradient fill below line
  showCurrent?: boolean;   // Dot on last point
}
```

Renders as tiny SVG (no canvas, no DPR boilerplate). Color from CSS vars.

### 3.6 `<GateBar>` — Gate Status Strip

```tsx
interface GateBarProps {
  gates: { name: string; status: 'pending' | 'running' | 'pass' | 'fail' }[];
}
```

`✓ COMPILE  ✓ TEST  ◉ CLIPPY  ○ DIFF` — each gate animates independently.

---

## 4. Layout Primitives

### 4.1 `<Stack>` — Vertical/Horizontal Stack

```tsx
interface StackProps {
  direction?: 'vertical' | 'horizontal';
  gap?: 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  align?: 'start' | 'center' | 'end' | 'stretch';
  justify?: 'start' | 'center' | 'end' | 'between' | 'around';
  wrap?: boolean;
  children: ReactNode;
}
```

Replaces hundreds of inline `style={{ display: 'flex', gap: '...', ... }}`.

### 4.2 `<Grid>` — Responsive Grid

```tsx
interface GridProps {
  columns?: number | 'auto';   // 'auto' = auto-fill with minWidth
  minWidth?: string;            // default '280px' for auto-fill
  gap?: 'xs' | 'sm' | 'md' | 'lg';
  children: ReactNode;
}
```

### 4.3 `<Tabs>` — Tab Navigation

```tsx
interface TabsProps {
  tabs: { id: string; label: string; icon?: ReactNode; badge?: ReactNode }[];
  active: string;
  onChange: (id: string) => void;
  variant?: 'line' | 'pill' | 'button';
  children: ReactNode;      // Tab panels
}
```

**Animations:**
- Tab switch: active indicator slides to new position (200ms ease-snappy)
- Content: crossfade (150ms)

### 4.4 `<ScrollArea>` — Styled Scrollable Region

```tsx
interface ScrollAreaProps {
  maxHeight?: string;
  fadeEdges?: boolean;      // Gradient fade at top/bottom edges
  children: ReactNode;
}
```

Thin rose scrollbar (4px), fade edges when content is scrolled.

### 4.5 `<PageShell>` — Scene Wrapper

```tsx
interface PageShellProps {
  title?: string;           // Document title prefix
  children: ReactNode;
}
```

Handles page-level entrance animation, sets document title, provides error boundary.

---

## 5. Interactive Primitives

### 5.1 `<Button>` — Enhanced from Current `.btn`

```tsx
interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;        // Shows spinner, disables
  icon?: ReactNode;
  iconPosition?: 'left' | 'right';
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
}
```

**Loading state:** Label replaced with Spinner, button width preserved (no layout shift), opacity 0.7.

**Variants:**
- `primary`: rose border + rose text, glow on hover
- `secondary`: glass border + dim text
- `ghost`: no border, text only, subtle hover bg
- `danger`: `--danger` border + text

### 5.2 `<Input>` — Text Input

```tsx
interface InputProps {
  label?: string;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  variant?: 'default' | 'search' | 'command';
  icon?: ReactNode;
  error?: string;
  disabled?: boolean;
}
```

**Focus transition:** Border color fades to `--rose-dim` (150ms), subtle glow appears.

### 5.3 `<Select>` — Dropdown

```tsx
interface SelectProps<T> {
  options: { value: T; label: string; icon?: ReactNode }[];
  value: T;
  onChange: (value: T) => void;
  label?: string;
  placeholder?: string;
}
```

**Dropdown animation:** scaleY from 0→1 (150ms ease-snappy), origin top.

### 5.4 `<Tooltip>` — Hover Information

```tsx
interface TooltipProps {
  content: ReactNode;
  position?: 'top' | 'bottom' | 'left' | 'right';
  delay?: number;          // ms before showing, default 300
  children: ReactNode;
}
```

**Entrance:** opacity + 4px Y travel + 0.97 scale (150ms) — physical feel per design system spec.

---

## 6. Feedback Primitives

### 6.1 `<Toast>` — Transient Notifications

```tsx
interface Toast {
  id: string;
  message: string;
  type: 'info' | 'success' | 'error' | 'warning';
  duration?: number;       // ms, default 4000
  action?: { label: string; onClick: () => void };
}
```

**Animation:** Slides in from top-right (200ms), auto-dismisses with fade + slide out.

### 6.2 `<Flash>` — Value Change Highlight

Wraps any element. When children change, triggers a brief glow.

```tsx
interface FlashProps {
  children: ReactNode;
  color?: 'bone' | 'rose' | 'success';  // Flash color
  duration?: number;       // default 300ms
}
```

**Animation:** 300ms text-shadow glow + color shift, then fades back.

### 6.3 `<ConfirmDialog>` — Action Confirmation

```tsx
interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: 'default' | 'danger';
  onConfirm: () => void;
  onCancel: () => void;
}
```

Modal with two buttons. Danger variant has red confirm button.

---

## 7. Theming Architecture

### 7.1 Single Source of Truth

ALL visual properties flow from CSS custom properties in `rosedust.css`. Components NEVER hardcode colors.

```
rosedust.css (tokens)
  └→ animations.css (keyframes referencing tokens)
  └→ primitives.css (component base classes)
  └→ Component.tsx (references classes, uses tokens for dynamic values)
```

**For canvas contexts** where CSS vars aren't available:
```typescript
// lib/theme.ts — reads CSS vars at runtime
export function getToken(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

// Cached version for hot paths (canvas draws)
const cache = new Map<string, string>();
export function token(name: string): string {
  if (!cache.has(name)) cache.set(name, getToken(name));
  return cache.get(name)!;
}

// Invalidate on theme change (if we ever add theme switching)
export function invalidateTokenCache() { cache.clear(); }
```

### 7.2 Semantic Token Layers

```css
:root {
  /* Layer 1: Raw palette (don't use directly in components) */
  --_rose-500: #b87a94;
  --_rose-400: #d89ab2;
  --_rose-300: #e8b5ce;

  /* Layer 2: Semantic tokens (USE THESE in components) */
  --color-accent: var(--_rose-400);
  --color-accent-glow: var(--_rose-300);
  --color-accent-dim: var(--_rose-500);

  --color-bg-page: var(--bg-void);
  --color-bg-panel: var(--glass-bg);
  --color-bg-card: var(--bg-raised);

  --color-border-default: var(--border);
  --color-border-hover: var(--border-strong);
  --color-border-active: var(--rose-dim);

  --color-text-heading: var(--text-strong);
  --color-text-body: var(--text-primary);
  --color-text-secondary: var(--text-soft);
  --color-text-label: var(--text-dim);
  --color-text-placeholder: var(--text-ghost);
}
```

### 7.3 Component Token Scoping

Each component can override tokens locally:

```css
.panel--accent-error {
  --_panel-border: var(--danger);
  --_panel-glow: var(--glow-error);
}
.panel--accent-success {
  --_panel-border: var(--success);
  --_panel-glow: var(--glow-success);
}
```

---

## 8. Animation System

### 8.1 Motion Tokens

```css
:root {
  /* Durations */
  --motion-instant: 80ms;
  --motion-fast: 150ms;
  --motion-normal: 220ms;
  --motion-slow: 350ms;
  --motion-reveal: 600ms;

  /* Easings */
  --ease-snappy: cubic-bezier(0.2, 0.8, 0.2, 1);
  --ease-expo: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-out: cubic-bezier(0, 0, 0.2, 1);
  --ease-bounce: cubic-bezier(0.34, 1.56, 0.64, 1);

  /* Spring configs (for JS/framer-motion) */
  --spring-gentle: stiffness 120, damping 14;
  --spring-snappy: stiffness 300, damping 30;
  --spring-bouncy: stiffness 400, damping 28;
}
```

### 8.2 Entrance Animations (Always Used)

Every element that appears uses one of these:

| Animation | Duration | Easing | Use |
|-----------|----------|--------|-----|
| `fadeUp` | 200ms | ease-expo | Default entrance (cards, panels, rows) |
| `fadeIn` | 150ms | ease-out | Subtle entrance (badges, tooltips) |
| `scaleIn` | 200ms | ease-snappy | Emphasis entrance (modals, alerts) |
| `slideRight` | 200ms | ease-expo | Side panel, drawer |
| `slideDown` | 200ms | ease-expo | Dropdown, accordion |

**Stagger:** Lists of items use 40ms delay between each item. Max 10 items staggered (after that, all appear together).

### 8.3 Loading Animations (Always Present During Waits)

| Situation | Animation |
|-----------|-----------|
| Page loading | Full-page skeleton with shimmer |
| Panel loading | Skeleton shimmer inside panel |
| Button clicked, waiting | Spinner replaces button label |
| Form submitted | Button loading state + progress bar |
| Data refreshing | Subtle shimmer overlay on existing content |
| Navigation transition | Crossfade + slide (200ms) |
| Modal opening | Scale 0.95→1.0 + fade (200ms) |
| Phase advancing | Step dot fills + line draws (300ms) |
| Value updating | Spring animation + flash highlight (300ms) |
| Terminal command running | Blinking cursor + streaming text |

### 8.4 Transition Matrix

| From | To | Animation |
|------|-----|-----------|
| Page A → Page B | Slide in direction of nav order (left/right) |
| List → Detail | Selected item morphs into detail panel |
| Tab A → Tab B | Content crossfade, indicator slides |
| Empty → Content | Skeleton → fade out, content fades up |
| Content → Loading | Content dims slightly, shimmer overlay |
| Any → Error | Red flash on border (400ms), error state fades in |
| Any → Modal | Backdrop fades, modal scales in |
| Modal → Close | Reverse of open |

### 8.5 Reduced Motion

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

All animations disabled. Color changes and opacity transitions preserved for state indication.

---

## 9. Per-Page Replacement Plan

### 9.1 Demo.tsx (832L → ~200L + 5 extracted components)

| Current Pattern | Replace With | Lines Saved |
|----------------|-------------|-------------|
| Inline `style={{ display: 'flex', gap: ... }}` (35 instances) | `<Stack>`, `<Grid>` | ~100L |
| Scenario selection cards (inline) | `<Card>` with `selected` + `onClick` | ~60L |
| Phase rail (inline dots + lines) | `<StepProgress>` | ~40L |
| Sidebar sections (raw divs) | `<Panel variant="ghost">` | ~50L |
| Pipeline status (custom rendering) | `<Badge>` + `<GateBar>` | ~30L |
| No loading states | `<Surface>` around data-dependent sections | ~20L |
| 3 copies of default agent reset | Single `DEFAULT_AGENT` constant | ~20L |
| `buildContext()` 56L closure | Extract to `lib/scenario-context.ts` | ~50L |
| Sidebar 141L of if/else | `<DemoSidebar>` component | ~120L |

**New file structure:**
```
pages/Demo.tsx                         (~200L, orchestrator only)
components/demo/ScenarioSelector.tsx   (~80L)
components/demo/PhaseTracker.tsx       (~60L)
components/demo/DemoSidebar.tsx        (~120L)
hooks/useDemoState.ts                  (~80L, state machine)
```

### 9.2 Bench.tsx (701L → ~150L + 6 tab components)

| Current Pattern | Replace With |
|----------------|-------------|
| Hero stats (inline Mosaic) | `<MetricGrid>` with `<AnimatedNumber>` |
| 7 inline tab contents | `<Tabs>` + lazy-loaded tab components |
| Run list (inline table) | `<DataTable>` with `<Badge>` status |
| Inline IIFEs for computation | `useMemo` + derived selectors |
| No loading state | `<Surface>` per section |
| Canvas charts (inline boilerplate) | `<Sparkline>` + `useCanvasSetup` |

**New file structure:**
```
pages/Bench.tsx                          (~150L, tab shell)
components/bench/BenchOverview.tsx       (~100L)
components/bench/RunList.tsx             (~100L)
components/bench/ParetoChart.tsx         (~120L)
components/bench/CompareView.tsx         (~100L)
components/bench/MatrixPanel.tsx         (~120L)
components/bench/InsightsPanel.tsx       (~80L)
```

### 9.3 BenchRunDetail.tsx (654L → ~120L + 3 chart components)

| Current Pattern | Replace With |
|----------------|-------------|
| `CostBreakdownChart` (228L inline) | Extract to `bench/CostChart.tsx` using `useCanvasSetup` |
| `TokenFlowChart` (126L inline) | Extract to `bench/TokenChart.tsx` |
| `OutputPreviewPanel` (80L inline) | `<Panel>` + `<ScrollArea>` |
| Hardcoded model pricing | Config from DataHub |
| `shortModel` inline copy | Import from `lib/format.ts` |

### 9.4 Explorer.tsx (859L → ~250L + 4 components)

| Current Pattern | Replace With |
|----------------|-------------|
| `fmtUptime` (3 copies) | Import from `lib/format.ts` |
| `drawTimeline` (222L inline) | `components/observe/TimelineCanvas.tsx` |
| 5 sparkline canvases (copy-paste boilerplate) | `<Sparkline>` SVG component |
| Health display (inline) | `<MetricGrid>` |
| Provider table (inline) | `<DataTable>` |
| No loading states | `<Surface>` per section |
| Tab content all inline | `<Tabs>` + extracted tab components |

**New file structure:**
```
pages/Explorer.tsx                       (~250L, tab shell)
components/observe/HealthMosaic.tsx      (~80L)
components/observe/TimelineCanvas.tsx    (~120L)
components/observe/ProviderTable.tsx     (~80L)
components/observe/SparklineRow.tsx      (~60L)
```

### 9.5 Builder.tsx (321L → ~200L)

| Current Pattern | Replace With |
|----------------|-------------|
| Model selection (inline buttons) | `<Chip>` group |
| Message list (inline rendering) | `<ScrollArea>` + message components |
| Input area (inline) | `<Input variant="command">` |
| Terminal toggle (inline) | `<Split>` with toggleable right pane |
| No loading state on submit | `<Button loading={sending}>` |
| Shell injection bug | Escaped via `lib/terminal-session.ts` |

### 9.6 PrdPipelinePanel.tsx (426L → ~150L + 4 sub-components)

| Current Pattern | Replace With |
|----------------|-------------|
| `.pp-section` (inline card) | `<Panel variant="outline" size="sm">` |
| Phase rail (inline) | `<StepProgress>` |
| Task board (inline rows) | `<DataTable>` with `<Badge>` + `<GateBar>` |
| Event log (inline grid) | `<ScrollArea>` + `<DataTable>` |
| Progress ring (inline SVG) | `<ProgressRing>` |
| Hero header (inline) | `<Panel variant="solid" size="lg">` |

### 9.7 MatrixBuilder.tsx + MatrixDetailView.tsx

| Current Pattern | Replace With |
|----------------|-------------|
| Model chips (inline) | `<Chip>` group with `selected` |
| Grid table (custom) | `<DataTable>` with custom cell renderer |
| Status indicators | `<Badge>` + `<Pulse>` |
| `benchlive-pulse` cross-file | Shared `animations.css` |

### 9.8 CostRace.tsx (434L)

| Current Pattern | Replace With |
|----------------|-------------|
| Canvas DPR setup (inline) | `useCanvasSetup` hook |
| `hexToRgba` (inline copy) | Import from `lib/color.ts` |
| Hardcoded color palette | Import from `lib/palette.ts`, use `token()` for canvas |
| Status badge (inline) | `<Badge>` component |

### 9.9 TopNav.tsx + TopNav.css

| Current Pattern | Replace With |
|----------------|-------------|
| `pulse-dot` (custom keyframe) | Shared from `animations.css` |
| `conn-blink` (custom keyframe) | Shared `status-blink` from `animations.css` |
| Status pill (inline) | `<Badge variant="status">` |
| Health indicator | DataHub `serverStatus` via `<Pulse>` |

### 9.10 Every Other Component

| Component | Key Change |
|-----------|-----------|
| `HeroParticleField.tsx` | `useCanvasSetup`, wrap in `<ErrorBoundary>`, `hexToRgba` from lib |
| `KnowledgeFlowPanel.tsx` | `useCanvasSetup`, remove module-level CSS cache, `hexToRgba` from lib |
| `DreamPhaseViz.tsx` | `useCanvasSetup`, `hexToRgba` from lib |
| `TokenVelocitySparkline.tsx` | Replace with `<Sparkline>` SVG component |
| `AgentOutputStream.tsx` | Wrap with `<Panel>`, add `<Surface>` for empty state |
| `GateVerdictTicker.tsx` | Use `<GateBar>` + `<Badge>` |
| `BenchLearningInsights.tsx` | Use `<Panel>` + `<MetricGrid>` + `<DataTable>` |
| `ConfigWidget.tsx` (440L) | Split into `<Panel>` sections, use `<Input>` + `<Select>` |

---

## 10. File Structure

```
src/
├── primitives/              ← ALL reusable building blocks
│   ├── containers/
│   │   ├── Panel.tsx        — Universal container (glass/solid/outline/ghost)
│   │   ├── Panel.css
│   │   ├── Card.tsx         — Clickable content unit
│   │   ├── Card.css
│   │   ├── Surface.tsx      — Data loading wrapper
│   │   ├── Modal.tsx        — Overlay dialog
│   │   ├── Modal.css
│   │   ├── Drawer.tsx       — Slide-in panel
│   │   ├── Drawer.css
│   │   └── Split.tsx        — Resizable split view
│   │
│   ├── feedback/
│   │   ├── Skeleton.tsx     — Loading placeholder
│   │   ├── Skeleton.css
│   │   ├── ProgressBar.tsx  — Determinate progress
│   │   ├── ProgressRing.tsx — Circular progress
│   │   ├── Spinner.tsx      — Indeterminate loading
│   │   ├── Pulse.tsx        — LED heartbeat dot
│   │   ├── StepProgress.tsx — Multi-phase rail
│   │   ├── StepProgress.css
│   │   ├── AnimatedNumber.tsx — Spring-animated value
│   │   ├── Flash.tsx        — Value change highlight
│   │   ├── Toast.tsx        — Transient notifications
│   │   ├── Toast.css
│   │   └── Transition.tsx   — Generic enter/exit wrapper
│   │
│   ├── data/
│   │   ├── MetricGrid.tsx   — Mosaic replacement
│   │   ├── MetricGrid.css
│   │   ├── DataTable.tsx    — Universal table
│   │   ├── DataTable.css
│   │   ├── Badge.tsx        — Status/tier/model badges
│   │   ├── Badge.css
│   │   ├── Chip.tsx         — Interactive tags
│   │   ├── Chip.css
│   │   ├── Sparkline.tsx    — Inline SVG chart
│   │   └── GateBar.tsx      — Gate status strip
│   │
│   ├── layout/
│   │   ├── Stack.tsx        — Flex stack (h/v)
│   │   ├── Grid.tsx         — Responsive grid
│   │   ├── Tabs.tsx         — Tab navigation
│   │   ├── Tabs.css
│   │   ├── ScrollArea.tsx   — Styled scrollable region
│   │   ├── ScrollArea.css
│   │   └── PageShell.tsx    — Scene wrapper
│   │
│   ├── interactive/
│   │   ├── Button.tsx       — Enhanced button
│   │   ├── Button.css
│   │   ├── Input.tsx        — Text input
│   │   ├── Input.css
│   │   ├── Select.tsx       — Dropdown
│   │   ├── Select.css
│   │   ├── Tooltip.tsx      — Hover info
│   │   └── ConfirmDialog.tsx
│   │
│   └── index.ts             — Barrel export everything
│
├── styles/
│   ├── rosedust.css         — Token source of truth (updated for brighter text)
│   ├── animations.css       — ALL shared keyframes
│   ├── primitives.css       — Base styles for all primitives
│   └── global.css           — Reset + body
│
└── lib/
    ├── theme.ts             — Runtime token reader for canvas/JS contexts
    ├── color.ts             — hexToRgba, getCssVar
    ├── format.ts            — shortModel, fmtUptime, relativeTime
    └── palette.ts           — DOMAIN_COLORS, ROLE_COLORS, MODEL_COLORS
```

---

## 11. Implementation Priority

### Wave 1: Foundation (enables everything else)
1. Update `rosedust.css` tokens (brighter text, larger scale)
2. Create `animations.css` (consolidated keyframes)
3. Build `<Panel>`, `<Surface>`, `<Skeleton>` — the 3 most-used
4. Build `<Stack>`, `<Grid>` — eliminate 200+ inline flex styles
5. Build `<Badge>`, `<Pulse>` — universal status display
6. Create `lib/theme.ts`, `lib/color.ts`, `lib/format.ts`, `lib/palette.ts`

### Wave 2: Data Display
7. Build `<MetricGrid>` + `<AnimatedNumber>` — Mosaic replacement
8. Build `<DataTable>` — universal table
9. Build `<StepProgress>` — PhaseRail replacement
10. Build `<GateBar>` — gate status
11. Build `<Sparkline>` — inline charts (SVG, no canvas)

### Wave 3: Interactive + Feedback
12. Build `<Button>` (with loading state), `<Input>`, `<Select>`
13. Build `<Modal>`, `<Drawer>`, `<Tabs>`
14. Build `<ProgressBar>`, `<ProgressRing>`, `<Spinner>`
15. Build `<Toast>`, `<Flash>`, `<Tooltip>`
16. Build `<Transition>` wrapper
17. Build `<Card>`, `<Chip>`, `<ScrollArea>`

### Wave 4: Page-by-Page Migration
18. Rebuild Demo.tsx → Orchestrate scene
19. Rebuild Explorer.tsx → Observe scene
20. Rebuild Bench.tsx + BenchRunDetail.tsx → Evaluate scene
21. Rebuild Builder.tsx → Build scene
22. Refactor TopNav, AppShell
23. Refactor all remaining components (CostRace, PrdPipelinePanel, etc.)

Each wave's primitives are tested in isolation before pages use them. Pages migrate one at a time — old pages work alongside new ones during migration.

---

## 11. ASCII / Terminal Aesthetic Primitives

The NERV institutional vocabulary translates to web UI via ASCII/terminal-style components. These provide the demoscene grid aesthetic, mono typewriter feel, and mechanical feedback.

### 11.1 `<AsciiLabel>` — Institutional Mono Label

Framed label in typewriter style with optional flicker animation.

```tsx
interface AsciiLabelProps {
  label: string;              // "● BENCHMARKS", "◉ ANALYZING"
  frameVariant?: 'brackets' | 'angles' | 'pipes' | 'corners' | 'none';
  // brackets: [ LABEL ]
  // angles:   ⟨ LABEL ⟩
  // pipes:    | LABEL |
  // corners:  ┌─ LABEL ─┐
  animation?: 'none' | 'typewriter' | 'flicker';
  size?: 'sm' | 'md' | 'lg';  // Font size: 10px / 12px / 14px
}
```

**Frame variants:**

```
Brackets:  [ LABEL ]              (ASCII baseline)
Angles:    ⟨ LABEL ⟩              (Angular, futuristic)
Pipes:     | LABEL |              (Vertical emphasis)
Corners:   ┌─ LABEL ─┐            (Box drawing)
None:      LABEL                  (Plain, all-caps mono)
```

**Animations:**
- `typewriter`: Letters appear one by one, 50ms per character
- `flicker`: Text stays, but brightness flickers (2Hz, random duration 80-120ms)
- `none`: Static text

**CSS:**
```css
.ascii-label {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  font-weight: 600;
  letter-spacing: 1px;
  color: var(--text-strong);
  text-transform: uppercase;
}

.ascii-label--flicker {
  animation: ascii-flicker 2s ease-in-out infinite;
}

@keyframes ascii-flicker {
  0%, 19%, 21%, 23%, 25%, 54%, 56%, 100% { opacity: 1; }
  20%, 24%, 55% { opacity: 0.6; }
}
```

### 11.2 `<AsciiDivider>` — Box-Drawing Separator

Horizontal divider with 6 visual styles and optional centered label.

```tsx
interface AsciiDividerProps {
  variant?: 'line' | 'double' | 'dashed' | 'dotted' | 'braille' | 'chevron';
  label?: string;              // Centered label on divider
  color?: 'default' | 'rose' | 'success' | 'error';
}
```

**Variants:**

```
line:      ───────────────────────
double:    ═══════════════════════
dashed:    ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
dotted:    · · · · · · · · · · · ·
braille:   ⠀⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏  (animated, 100ms per char)
chevron:   ▸◂ ▸◂ ▸◂ ▸◂ ▸◂ ▸◂ ▸◂  (animated right→left, 150ms)

With label:
───── RUNNING ─────
═════ COMPLETE ════
```

**CSS:**
```css
.ascii-divider {
  border: none;
  height: 1px;
  background: currentColor;
  opacity: 0.5;
}

.ascii-divider--braille {
  background: none;
  display: flex;
  align-items: center;
  font-size: var(--text-xs);
  letter-spacing: 2px;
  animation: braille-scroll 3s linear infinite;
}

@keyframes braille-scroll {
  0% { transform: translateX(0); }
  100% { transform: translateX(-100%); }
}
```

### 11.3 `<AsciiFrame>` — Box-Drawing Frame

Rectangular border around content with 4 variants and optional title/status.

```tsx
interface AsciiFrameProps {
  variant?: 'single' | 'double' | 'rounded' | 'heavy';
  title?: string;               // Top-left corner
  status?: 'idle' | 'active' | 'success' | 'error';
  children: ReactNode;
}
```

**Variants:**

```
single:    ┌─────────────┐        double:    ╔═════════════╗
           │             │                   ║             ║
           └─────────────┘                   ╚═════════════╝

rounded:   ╭─────────────╮        heavy:     ┏━━━━━━━━━━━━━┓
           │             │                   ┃             ┃
           ╰─────────────╯                   ┗━━━━━━━━━━━━━┛

With title:
┌─ TASKS ───────────────┐
│                       │
│ (content)             │
└───────────────────────┘
```

**Status colors:** `--rose-dim` (idle), `--status-active` (active), `--status-success`, `--status-error`

### 11.4 `<AsciiBraille>` — Animated Braille Pattern Fill

Decorative animated braille fill for loading states or visual interest. ~10fps animation.

```tsx
interface AsciiBrailleProps {
  pattern?: 'noise' | 'wave' | 'density' | 'spinner';
  size?: 'sm' | 'md' | 'lg';    // Font size
  color?: string;               // Defaults to --text-dim
  duration?: number;            // ms per frame, default 100
}
```

**Patterns:**

```
noise:     ⠿⠧⠧⠿⠿⠿⠧⠧⠿   (randomly shuffled)
wave:      ⠀⠂⠆⠇⠋⠙⠸⠰⠠⠀   (0-8 height, repeating)
density:   ⠏⠟⠯⠿⠽⠾⠖⠦⠤   (cycling through density levels)
spinner:   ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏   (classic braille spinner)
```

### 11.5 `<AsciiProgress>` — Terminal Progress Bar

Text-based progress indicator using block/braille characters.

```tsx
interface AsciiProgressProps {
  value: number;                 // 0-100
  variant?: 'block' | 'braille' | 'arrow';
  label?: string;
  showPercent?: boolean;
  width?: number;                // Characters wide, default 20
  color?: 'rose' | 'success' | 'error';
}
```

**Variants:**

```
block:     [████████░░░░░░░░░░░░] 42%      (▓ filled, ░ empty)
braille:   ⠟⠿⠿⠿⠿⠸⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀ 42%
arrow:     ──→─────────────────── 42%      (arrow progresses left→right)
```

**Color:** Filled region uses `--color` CSS var. Background uses `--border`.

### 11.6 `<AsciiWaveform>` — Block-Element Oscilloscope

Inline sparkline using block elements (▁▂▃▄▅▆▇█) for data visualization.

```tsx
interface AsciiWaveformProps {
  data: number[];               // Values 0-100
  height?: number;              // 1-3 rows, default 1
  width?: number;               // Characters, default 24
  label?: string;               // Left-side label
}
```

**Example (height=3, width=16):**

```
█   █
██ ██
████ ████ ██ █  (maps data to 8 height levels)

Label on left:  TOKENS  █   █
               ────────██ ██
                        ████ ████
```

Uses `var(--status-active)` for the bar color.

---

## 12. Expressive Primitives (Extended Catalog)

The foundation primitives above (sections 1-11) cover the core building blocks. The full advanced component library is specified in **[10-EXPRESSIVE-PRIMITIVES.md](10-EXPRESSIVE-PRIMITIVES.md)**, which adds 35 higher-order components across 9 categories:

### 12.1 Resizable Pane System
- **`<ResizablePane>`** — Pane container with draggable edge resize, snap-to-grid, collapse/expand
- **`<PaneGrid>`** — 2D grid layout manager with proportional resize and localStorage persistence
- **`<PaneGroup>`** — Linear pane stack (VS Code-style), double-click to equalize, keyboard-accessible resize

### 12.2 Loading & State Transitions
- **`<LoadingTransition>`** — CRT-inspired pixelated dither reveal (random, scanline, radial patterns)
- **`<ContentSwitch>`** — Crossfade through skeleton intermediate when content key changes
- **`<LazyPane>`** — Connection-lifecycle-aware pane (connecting -> skeleton -> dither reveal -> content)
- **`<ConnectionGuard>`** — Gate wrapper with retry countdown visualization and exponential backoff

### 12.3 Stepped Progress Variants
- **`<StepProgress>` (enhanced)** — Icon slots, descriptions, particle trails on active connecting lines, circular variant
- **`<GradientStepRail>`** — Continuous gradient fill with traveling comet effect and animated SVG step dots
- **`<VerticalTimeline>`** — Detailed timeline cards with timestamps, expandable detail, scroll fade edges
- **`<CircularProgress>`** — SVG ring with gradient stroke, multi-segment mode, animated sweep fill
- **`<MilestoneProgress>`** — Linear bar with labeled milestone markers that celebrate completion

### 12.4 WebGL / Canvas Backgrounds
- **`<ParticleField>`** — Generalized particle system (from HeroParticleField), state-reactive density and color
- **`<NoiseBackground>`** — Fragment shader noise (simplex/perlin/worley), palette from ROSEDUST tokens
- **`<FluidGradient>`** — Drifting gradient mesh control points, pulse-reactive
- **`<HeartbeatLine>`** — EKG waveform at three timescales (fast/medium/glacial), maps to agent affect
- **`<GlitchOverlay>`** — Scanline displacement, RGB split, chromatic aberration, intensity-scaled
- **`<AmbientContainer>`** — Panel + WebGL background convenience wrapper

### 12.5 Agent-Namespaced Components
- **`<AgentContainer>`** — Agent-scoped wrapper with header, heartbeat, metrics bar, state-reactive background
- **`<AgentMetricBar>`** — Compact C-factor / neuro / PAD / tokens / cost strip with AnimatedNumber
- **`<AgentFeed>`** — Scrolling per-agent event feed (inference, gates, tools, somatic, lifecycle)
- **`<AgentHeartbeat>`** — Three-timescale visual heartbeat (glow, border, or line mode)
- **`<AgentAvatar>`** — Procedural dot-cloud / glyph / ring identity from agent fingerprint

### 12.6 Event Feeds
- **`<EventStream>`** — Universal real-time event feed with pluggable renderers and filter bar
- **`<BlockFeed>`** — Chain block arrival visualization (Phase 2+)
- **`<InferenceFeed>`** — LLM inference events with model coloring and inline token velocity sparklines

### 12.7 Floating Chat
- **`<FloatingChat>`** — Draggable chat widget with agent selector, message thread, tool call blocks
- **`<ChatBubble>`** — Inline trigger icon with unread count badge

### 12.8 Modal & Overlay Enhancements
- **`<Modal>` (enhanced)** — Focus trap, backdrop blur, stacking, 5 sizes including full
- **`<Drawer>` (enhanced)** — Push-content mode, resizable edge, bottom position
- **`<CommandPalette>`** — Cmd+K fuzzy search for pages, agents, runs, commands

### 12.9 Layout Additions
- **`<StickyTopLayout>`** — Sticky nav + scrollable content (fixes body overflow hidden)
- **`<MasonryGrid>`** — Variable-height card grid with animated layout transitions
- **`<TreeView>`** — Hierarchical expandable tree with indent guides, keyboard nav, ARIA
- **`<VirtualList>`** — Virtualized scrolling (IntersectionObserver-based, ~25 DOM nodes regardless of data size)

### Implementation waves

The expressive primitives integrate into the existing wave schedule:

| Wave | Foundation (this doc) | Expressive (10-EXPRESSIVE-PRIMITIVES.md) |
|------|----------------------|------------------------------------------|
| **2** | MetricGrid, DataTable, StepProgress, GateBar, Sparkline | StepProgress (enhanced), CircularProgress, VerticalTimeline, MilestoneProgress, VirtualList, StickyTopLayout, TreeView |
| **3** | Button, Input, Select, Modal, Drawer, Tabs, ProgressBar, ProgressRing, Spinner, Toast, Flash, Tooltip, Transition, Card, Chip, ScrollArea | Modal/Drawer (enhanced), CommandPalette, LoadingTransition, ContentSwitch, LazyPane, ConnectionGuard, GradientStepRail, EventStream, InferenceFeed, FloatingChat, ChatBubble |
| **4** | Page-by-page migration | ResizablePane, PaneGrid, PaneGroup, AgentContainer, AgentMetricBar, AgentFeed, AgentHeartbeat, AgentAvatar, ParticleField, NoiseBackground, FluidGradient, HeartbeatLine, GlitchOverlay, AmbientContainer, MasonryGrid, BlockFeed |

See [10-EXPRESSIVE-PRIMITIVES.md](10-EXPRESSIVE-PRIMITIVES.md) for full TypeScript interfaces, visual specs, animation specs, and dependency graph.
