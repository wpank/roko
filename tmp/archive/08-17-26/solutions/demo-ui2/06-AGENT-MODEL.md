# 06. Portable Agent Model

How agents are represented, visualized, and interacted with across the entire UI. The agent is the central actor in roko — it should have a consistent, rich presence everywhere it appears.

---

## 1. Agent Lifecycle Modes

Roko supports four distinct agent modes. The UI must handle all of them.

### 1.1 Single-Run Ephemeral

```
User prompt → Agent spawns → Executes → Gate check → Dies
```
- **Lifespan**: seconds to minutes
- **Visibility**: task row in Orchestrate, episode entry in timeline
- **UI indicator**: pulsing status dot while active, then green check or red X
- **Example**: `roko run "Build a CLI calculator"`

### 1.2 Perpetual Agent

```
Agent starts → Heartbeat loop → Handles tasks on demand → Runs until stopped
```
- **Lifespan**: hours to indefinite
- **Visibility**: persistent card in Fleet, always-on status in Observe
- **UI indicator**: steady green LED when healthy, amber when idle, red on error
- **Heartbeat**: periodic health check visible as subtle pulse animation
- **Example**: `roko agent start --name rustsmith`

### 1.3 Multi-Agent Workflow

```
Plan → DAG of tasks → Router assigns agents → Parallel execution → Gates → Complete
```
- **Lifespan**: minutes to hours
- **Visibility**: topology graph showing agent nodes with task assignments, dependency edges
- **UI indicator**: multiple agent nodes pulsing simultaneously, edges lighting up on handoff
- **Example**: `roko plan run plans/auth-system/`

### 1.4 Agent-to-Agent (External)

```
External agent connects → Sidecar accepts → Processes request → Returns result
```
- **Lifespan**: per-request
- **Visibility**: incoming connection indicator in Fleet
- **UI indicator**: external badge, different border style
- **Example**: agent-server `/message` endpoint

---

## 2. Agent Visual Representation

### 2.1 Agent Card (Fleet View)

```
┌─ ● rustsmith ─────────────────────────────┐
│  implementer · T1-T2                       │
│                                            │
│  247 tasks completed · $0.42 total         │  ← activity summary
│  $0.0017 per task average                  │  ← derived metric
│                                            │
│  Model: claude-haiku-4.5                   │  ← current/preferred model
│  Last active: 2m ago                       │  ← recency
│                                            │
│  ┌─ CAPABILITIES ──────────────────────┐   │
│  │ scaffolding · implementation ·      │   │  ← role tags
│  │ config · simple-transforms          │   │
│  └─────────────────────────────────────┘   │
└────────────────────────────────────────────┘
```

**Status indicators:**
- ● green LED pulsing → actively executing a task
- ● green LED steady → running, idle, waiting for work
- ● amber LED → degraded (high error rate, slow responses)
- ○ gray → stopped/offline

**Glowing borders on state change:**
When an agent transitions state (idle→active, active→idle, healthy→error), the card border briefly glows in the corresponding status color for 400ms, then fades. This catches the eye during a live demo.

```css
.agent-card[data-transitioning] {
  border-color: var(--status-active);
  box-shadow: var(--glow-active);
  transition: border-color 400ms, box-shadow 400ms;
}
```

### 2.2 Agent Node (Topology Graph)

```
         ┌───────────┐
         │           │
    ──── │ rustsmith │ ────
   12    │     ●     │   8
  tasks  │   haiku   │ tasks
         │           │
         └───────────┘
```

- Node size: 20px+ radius (larger than current), scales with task count
- Node fill: tinted by status color
- Label: agent name + model badge inside node
- Edge labels: handoff count between agents
- Edge animation: brief pulse when a task is handed off (300ms travel along edge)
- Energy-based stop: simulation halts when stable, restarts on new agent/handoff

### 2.3 Agent in Task Board

```
│ ◉ RUN    Implement DeFi fetcher                           │
│          ┌──────────────┐                                 │
│          │ ● rustsmith  │  T2 · sonnet · 3.4s · $0.017   │
│          │   haiku-4.5  │  ◉ compile  ○ test              │
│          └──────────────┘                                 │
```

When a task is being executed, a mini agent badge appears inline showing:
- Agent name + status dot
- Selected model
- Live duration counter
- Gate progress

### 2.4 Agent Output Stream

For Build/Orchestrate terminal panels, agent output streams character-by-character:

```typescript
// AgentOutputStream component
interface Props {
  agentId: string;
  style?: 'terminal' | 'chat';
}
```

- **terminal style**: Raw output in xterm.js, monospace, full PTY emulation
- **chat style**: Formatted messages with role badges, tool call blocks expandable, code blocks highlighted

The stream subscribes to `AgentOutput` events via DataHub, writing directly to xterm ref (bypassing React for performance).

---

## 3. Agent Interactions

### 3.1 Start/Stop

```tsx
// Fleet view actions
<AgentCard agent={agent}>
  {agent.status === 'stopped' && (
    <button onClick={() => api.post(`/api/agents/${agent.name}/start`)}>Start</button>
  )}
  {agent.status === 'running' && (
    <button onClick={() => api.post(`/api/agents/${agent.name}/stop`)}>Stop</button>
  )}
</AgentCard>
```

Start → agent card border flashes green, LED starts pulsing.
Stop → LED fades to gray, card border fades.

### 3.2 Chat

From Build page or agent detail:

```tsx
<ChatThread agentId={agent.name}>
  <MessageInput onSend={(msg) => api.post(`/api/agents/${agent.name}/chat`, { message: msg })} />
</ChatThread>
```

Response streams via `AgentOutput` SSE events. Tool calls are rendered as expandable blocks.

### 3.3 Episode History

Click an agent card → detail panel shows:
- Recent episodes: task, result, cost, duration
- Performance over time: pass rate trend, cost trend
- Model usage: which models this agent has used and their success rates

---

## 4. Multi-Agent Visualization

### 4.1 During Plan Execution (Orchestrate)

When a plan runs with multiple agents:

```
┌─ ACTIVE AGENTS ──────────────────────────────────────────┐
│  ● rustsmith (T1)  ────→  ● ethdev (T2)  ────→  ● auditor (T3) │
│    2 done, 0 fail        1 running              0 pending       │
└──────────────────────────────────────────────────────────┘
```

Horizontal strip below task board showing:
- Active agents with status dots
- Arrow edges showing task handoff direction
- Counters: completed, running, pending per agent
- Animate: agent appears when first task assigned, arrow lights up on handoff

### 4.2 Agent Topology (Observe → Fleet)

Full force-directed graph:
- Nodes = agents, sized by total tasks
- Edges = handoff relationships, weighted by count
- Click node → detail panel slides in (shared element)
- Real-time updates: new agent spawns fade in, handoffs pulse edges

---

## 5. From Bardo: Agent Visualization Concepts

The bardo TUI had rich agent visualization that translates to the web:

| Bardo Concept | Web Translation |
|---------------|----------------|
| Sprite with procedural generation from seed | CSS-generated avatar (gradient + initials, unique per agent name) |
| Emotional state via PAD vector | Status color intensity modulation (stressed = brighter red, healthy = softer green) |
| Lifecycle degradation (Thriving→Terminal) | Progressive visual simplification: full card → compact card → dot only |
| Heartbeat animation | CSS pulse animation on LED, frequency tied to activity level |
| Activity waveform | Sparkline in agent card showing recent activity rate |

### 5.1 The Spectre: Algorithmic Agent Avatars

Inspired by the bardo creature system (`bardo-backup/prd/18-interfaces/28-creature-system.md`), every agent gets a **Spectre** — a procedurally generated dot-cloud visual identity derived deterministically from the agent's name and role. Same agent always produces the same Spectre. Operators learn to recognize agents by their Spectre at a glance, like recognizing video game characters.

#### Agent Identity Card

Every agent in roko has structured identity from `roko.toml` config and the agent manifest:

```typescript
interface AgentIdentity {
  name: string;              // e.g., "implement-auth"
  role: string;              // e.g., "implementer", "verifier", "researcher", "security"
  domain: string;            // e.g., "rust", "typescript", "infra", "defi"
  model: string;             // e.g., "claude-sonnet-4", "claude-haiku-4"
  tier: 'T1' | 'T2' | 'T3'; // routing tier
  planId?: string;           // which plan this agent serves
  taskId?: string;           // which task this agent is executing
  spectre: SpectreIdentity;  // derived visual identity
}
```

#### Generation Algorithm

```typescript
interface SpectreIdentity {
  seed: Uint8Array;          // hash(name + ":" + role) → 32 bytes
  archetype: SpectreArchetype;
  palette: SpectrePalette;
  eyeStyle: SpectreEyeStyle;
  glyph: string;             // 2-char Unicode glyph pair (the "eyes")
  shape: SpectreShape;
}

// 8 body archetypes mapped from seed bytes [0..4]
type SpectreArchetype =
  | 'orb'       // compact, focused — knowledge agents
  | 'column'    // tall, structured — implementation agents
  | 'sprawl'    // wide, exploratory — research agents
  | 'cluster'   // multi-node — parallel/coordinator agents
  | 'teardrop'  // directional — goal-oriented agents
  | 'ring'      // hollow center — monitoring/verification agents
  | 'fractal'   // branching — analysis agents
  | 'amorphous' // shifting — creative/generative agents
  ;

type SpectreEyeStyle = 'round' | 'slit' | 'compound' | 'star';
```

#### Role → Visual Mapping

| Role | Archetype | Palette | Eye Glyph | Character |
|---|---|---|---|---|
| implementer | column | rose | `◈◈` | Structured, builds things |
| researcher | sprawl | violet | `◉◉` | Wide-scanning, exploratory |
| verifier | ring | jade | `◎◎` | Watchful, monitoring |
| security | teardrop | crimson | `◆◆` | Focused, directional |
| coordinator | cluster | sapphire | `✦✦` | Multi-node, orchestrating |
| planner | fractal | amber | `◇◇` | Branching analysis |
| reviewer | orb | bone | `●●` | Dense knowledge, focused |

The seed-based hue offset (±15° within the role's palette range) ensures that two "implementer" agents have recognizably similar but distinguishable Spectres.

#### Canvas Rendering (Web)

The Spectre renders as a **dot-cloud creature** with spring physics — 40-80 particles in a shaped cloud:

```typescript
function renderSpectre(
  ctx: CanvasRenderingContext2D,
  identity: SpectreIdentity,
  state: AgentState,        // idle | active | error | done
  size: number,             // 16 | 32 | 48 | 64 px
) {
  // Archetype determines cloud shape
  // Palette from role + seed-based hue offset within ROSEDUST
  // Eyes: 2 bright glyph points at center
  // State drives animation:
  //   idle: slow breathing (expand/contract, 0.3Hz)
  //   active: faster breathing (0.7Hz), eye glow, slight shimmer
  //   error: constricted, jittery, crimson tint
  //   done: expanded, fading glow, settled
}
```

#### Size Variants

| Context | Size | Detail level |
|---|---|---|
| Inline (log entry, task row) | 16px | Glyph pair with role color (`◈◈`) |
| Badge (terminal header, card) | 32px | Simplified static dot cloud, no animation |
| Card (agent detail, topology node) | 48px | Full dot cloud, breathing animation |
| Hero (agent detail expanded) | 64px | Full detail, spring physics, eyes visible |

#### SpectreAvatar Component

```typescript
interface SpectreAvatarProps {
  name: string;
  role: string;
  state: 'idle' | 'active' | 'error' | 'done';
  size: 16 | 32 | 48 | 64;
  animate?: boolean;  // default true for 48+
}

function generateSeed(name: string, role: string): Uint8Array {
  const str = `${name}:${role}`;
  const seed = new Uint8Array(32);
  for (let i = 0; i < str.length; i++) {
    seed[i % 32] ^= str.charCodeAt(i);
    seed[(i + 7) % 32] ^= (str.charCodeAt(i) * 31) & 0xFF;
    seed[(i + 13) % 32] ^= (str.charCodeAt(i) * 127) & 0xFF;
  }
  return seed;
}

function paletteFromRole(role: string, seed: Uint8Array): string {
  const baseHue = ROLE_HUES[role] ?? 12;
  const offset = ((seed[20] / 255) * 30) - 15;  // ±15° variation
  return `oklch(0.65 0.10 ${baseHue + offset})`;
}

const ROLE_HUES: Record<string, number> = {
  implementer: 12, researcher: 290, verifier: 170,
  security: 25, coordinator: 250, planner: 85, reviewer: 55,
};

const EYE_GLYPHS: Record<string, string> = {
  implementer: '◈', researcher: '◉', verifier: '◎',
  security: '◆', coordinator: '✦', planner: '◇', reviewer: '●',
};

export default function SpectreAvatar({ name, role, state, size, animate }: SpectreAvatarProps) {
  if (size <= 16) {
    const glyph = EYE_GLYPHS[role] ?? '○';
    return <span className={`spectre-inline spectre-${state}`}>{glyph}{glyph}</span>;
  }
  if (size <= 32) return <SpectreStaticSVG name={name} role={role} state={state} size={size} />;
  return <SpectreCanvas name={name} role={role} state={state} size={size} animate={animate ?? true} />;
}
```

### 5.2 Activity Sparkline

Tiny inline chart (60px × 16px) showing task completion rate over last 30 minutes:

```
▁▂▃▅▇█▇▅▃▂▁▁▂▃▅
```

Rendered as a CSS grid of 1px-wide bars with varying heights. Updates every 10s from episode data.

---

## 6. Agent-Attributed Terminals

### 6.1 The Problem

Current: 2 terminal panes show raw shell output. No way to know which agent produced which output. When agents run sequentially in the same terminal, outputs blend together.

### 6.2 Agent-Attributed Terminal Streams

Each terminal pane gets an **agent attribution header** that updates when a different agent takes control:

```
┌──────────────────────────────────────────────────┐
│ ◈◈ implement-auth · T2 · claude-sonnet-4 · ACTIVE │
│ implementer · rust                                │
├──────────────────────────────────────────────────┤
│  $ cargo test --lib auth                         │
│  running 4 tests                                 │
│  test auth::test_jwt_creation ... ok             │
└──────────────────────────────────────────────────┘
```

The header shows: Spectre glyph (colored by role), agent name (bold), tier badge (T1/T2/T3), model (which LLM), status (active/idle/done/error), role + domain (second line, dimmer).

### 6.3 Multi-Agent Split Views

When multiple agents run in parallel, the terminal layout adapts:

- 1 agent: single pane (full width)
- 2 agents: side-by-side
- 3-4 agents: 2x2 grid
- 5+ agents: scrollable list with expandable panes (only active agents expanded)

```
┌─ 2 agents ──────────────────────────────────────────────────────┐
│ ◈◈ implement-auth · T2   │ ◎◎ verify-auth · T1                │
│ implementer · rust        │ verifier · rust                     │
│ ─────────────────────── │ ──────────────────────────────────  │
│ $ roko run "implement..." │ (waiting for implement-auth)        │
│ Creating src/auth/mod.rs  │                                     │
└─────────────────────────┴─────────────────────────────────────┘
```

### 6.4 Agent Output Stream Component

Not every agent needs a full terminal. Some agents (research, planning) produce structured output better shown as a log stream:

```typescript
interface AgentOutputStream {
  agentId: string;
  identity: AgentIdentity;
  terminal?: TerminalHandle;    // PTY-backed terminal (for shell commands)
  logStream?: LogEntry[];       // Structured log (for non-terminal output)
  mode: 'terminal' | 'log' | 'split';
}
```

---

## 7. Agent-Attributed Logs

The unified event log attributes every entry to an agent:

```
┌─ LOG ───────────────────────────────────────────────────────┐
│ 17:38:49  ◇◇ planner       PLAN    Generated 4 tasks        │
│ 17:38:51  ◈◈ implement-auth ACTIVE  Starting task auth-001   │
│ 17:38:51  ◈◈ implement-api  ACTIVE  Starting task api-001    │
│ 17:39:02  ◈◈ implement-auth GATE    compile ✓ test ✓         │
│ 17:39:05  ◈◈ implement-api  GATE    compile ✓ test ✗         │
│ 17:39:05  ◈◈ implement-api  REPLAN  Gate failure → retry     │
│ 17:39:12  ◎◎ verify-all     ACTIVE  Starting verification    │
│ 17:39:15  ◎◎ verify-all     GATE    all gates ✓              │
│ 17:39:15  ✦✦ coordinator    DONE    Plan complete            │
└──────────────────────────────────────────────────────────────┘
```

Each log line has: timestamp (mono, dim), Spectre glyph (colored by role), agent name (bold, truncated to 16 chars), event type (uppercase badge), message. Filter controls let you show/hide by agent, role, or event type.

---

## 8. Agent Topology Graph

The topology view shows agents as **Spectre nodes** connected by dependency and knowledge-sharing edges:

```
         ◇◇ planner
        ╱         ╲
   ◈◈ auth    ◈◈ api     (implementation agents)
       │          │
       └────┬─────┘
            │
       ◎◎ verify          (verification agent)
            │
       ✦✦ coordinator     (reports to plan runner)
```

Each node renders the agent's Spectre at 48px with: breathing animation when active, color ring matching status (teal=active, green=done, rose=error, gray=idle, purple=blocked), edge animations (data flowing along edges when agents communicate), hover shows agent card with full identity, click expands to detail view (shared element transition).

---

## 9. Knowledge Transfer Visualization

When agents share knowledge (via roko's neuro store), the transfer is visible:

1. **Source agent's Spectre** pulses with a brief glow
2. **A particle** (small colored dot) detaches and animates along the edge to the target agent
3. **Target agent's Spectre** absorbs the particle (brief flash)
4. **Log entry** appears: `◈◈ auth → ◎◎ verify: shared "JWT signing approach"`

This makes the invisible (knowledge store writes/reads) visible and tangible.

---

## 10. Reusable Agent Card Component

The reusable agent card used everywhere (task rows, topology, fleet, sidebar):

```typescript
function AgentCard({ identity, state, variant, showTask, showMetrics }: AgentCardProps) {
  return (
    <div className={`agent-card agent-card-${variant}`}>
      <SpectreAvatar identity={identity} state={state} size={sizeForVariant(variant)} />
      <div className="agent-card-identity">
        <span className="agent-name">{identity.name}</span>
        <span className="agent-role">{identity.role} · {identity.domain}</span>
      </div>
      <StatusBadge status={state} />
      {showTask && <span className="agent-task">{identity.taskId}</span>}
      {showMetrics && <AgentMetrics agent={identity} />}
    </div>
  );
}
```

| Variant | Size | Usage |
|---|---|---|
| `inline` | 16px Spectre | Log entries, task rows: `◈◈ implement-auth · rust · T2` |
| `badge` | 32px Spectre | Terminal headers: `[◈◈ dot-cloud] implement-auth` |
| `card` | 48px Spectre | Fleet grid, topology nodes: full animated dot-cloud |
| `hero` | 64px Spectre | Expanded detail: full spring physics, eye tracking, metrics |

---

## 11. Multi-Agent Playback Controls

When multiple agents run in parallel, the playback bar shows aggregate progress:

```
┌────────────────────────────────────────────────────────────────┐
│ ▶ ⏭ ↺ │ Auto Step │ ◈◈ 2 active  ◎◎ 1 waiting  ✓ 1 done │
│         │           │ Task 3/7 · Step 2/5 · preparing auth    │
└────────────────────────────────────────────────────────────────┘
```

---

## 12. Event-Driven Agent Animations

| Server Event | Agent Visual Response |
|-------------|---------------------|
| AgentSpawned | Card fades in with expanding ring (300ms) |
| AgentOutput (first chunk) | LED starts pulsing, border glows active |
| AgentOutput (done=true) | LED settles to steady, border glow fades |
| GateResult (pass) | Mini green check appears next to agent badge on task |
| GateResult (fail) | Agent card border flashes red (400ms), task shows failure |
| InferenceStarted | Model badge appears/updates on agent card |
| InferenceCompleted | Cost counter increments with spring animation |
| Agent stops | LED fades to gray, card dims slightly |
| KnowledgeTransfer | Particle animates along edge from source → target agent |

---

## 13. Reference Material

| Document | Path | What it covers |
|---|---|---|
| Creature System | `bardo-backup/prd/18-interfaces/28-creature-system.md` | Dot-cloud geometry, spring physics, eye rendering, lifecycle degradation |
| ROSEDUST + Spectre | `docs/v2-depth/16-surfaces/04-rosedust-and-spectre.md` | Deterministic identity, 8 archetypes, PAD animation, 4 renderers |
| Embodied Consciousness | `bardo-backup/prd/18-interfaces/perspective/03-embodied-consciousness.md` | Terminal as body metaphor, somatic zones, PAD-driven transformation |
| Visualization Primitives | `bardo-backup/prd/18-interfaces/rendering/02-visualization-primitives.md` | Braille rendering, plasma effects, force graphs |
| Design System | `bardo-backup/prd/18-interfaces/rendering/00-design-system.md` | ROSEDUST palette, contrast, degradation by lifecycle |
