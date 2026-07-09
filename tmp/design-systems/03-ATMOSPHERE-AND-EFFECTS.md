# Atmosphere, Effects & Advanced Visual Systems

Use this document for the deeper visual effects layer — procedural avatars, crystallization animations, atmospheric layering, ambient motion, and the hauntological/consciousness-driven visual language.

---

## The Perpetual Motion Principle

**Nothing is ever at rest.** Every element is driven by at least one continuously changing variable. The UI is a living system — ambient breathing, pulsing dots, drifting particles, value animations. Removing animation doesn't make it cleaner, it makes it dead.

Three simultaneous timescales run concurrently:
| Timescale | Period | What it drives |
|-----------|--------|---------------|
| Fast | 0.7–2.4s | LED pulse, heartbeat, color saturation |
| Medium | 3–5s | Breathing animations, cloud density |
| Glacial | 5+ min | Background hue drift, lifecycle progression |

---

## Spectre System — Procedural Agent Avatars

Every agent gets a **Spectre**: a procedurally generated dot-cloud creature with spring physics, deterministically derived from the agent's name and role. Same inputs always produce the same visual.

### Generation
```typescript
interface SpectreIdentity {
  seed: Uint8Array;           // hash(name + ":" + role) → 32 bytes
  archetype: SpectreArchetype;
  palette: SpectrePalette;
  eyeStyle: SpectreEyeStyle;
  glyph: string;              // 2-char Unicode eye pair
  shape: SpectreShape;
}
```

### 8 Body Archetypes
| Archetype | Shape | Maps to |
|-----------|-------|---------|
| `orb` | Compact, focused | Knowledge agents |
| `column` | Tall, structured | Implementation agents |
| `sprawl` | Wide, exploratory | Research agents |
| `cluster` | Multi-node | Coordinator agents |
| `teardrop` | Directional | Goal-oriented agents |
| `ring` | Hollow center | Monitoring/verification |
| `fractal` | Branching | Analysis agents |
| `amorphous` | Shifting | Creative/generative |

### Role → Visual Mapping
| Role | Archetype | Eye Glyph | OKLCH Hue |
|------|-----------|-----------|-----------|
| implementer | column | `◈◈` | 12° (rose) |
| researcher | sprawl | `◉◉` | 290° (violet) |
| verifier | ring | `◎◎` | 170° (jade) |
| security | teardrop | `◆◆` | 25° (crimson) |
| coordinator | cluster | `✦✦` | 250° (sapphire) |
| planner | fractal | `◇◇` | 85° (amber) |
| reviewer | orb | `●●` | 55° (bone) |

### Palette Formula
```typescript
function paletteFromRole(role: string, seed: Uint8Array): string {
  const baseHue = ROLE_HUES[role] ?? 12;
  const offset = ((seed[20] / 255) * 30) - 15;  // ±15° variation
  return `oklch(0.65 0.10 ${baseHue + offset})`;
}
```

### Canvas Rendering
40-80 particles in a shaped cloud. Spring physics at 60fps.
- **idle**: slow breathing (expand/contract, 0.3Hz)
- **active**: faster breathing (0.7Hz), eye glow, shimmer
- **error**: constricted, jittery, crimson tint
- **done**: expanded, fading glow, settled

### Size Variants
| Context | Size | Detail |
|---------|------|--------|
| Inline (log, task row) | 16px | Glyph pair with role color |
| Badge (header, card) | 32px | Simplified dot cloud, no animation |
| Card (detail, node) | 48px | Full dot cloud, breathing |
| Hero (expanded) | 64px | Full detail, spring physics, eyes visible |

### Spring Physics
```
Damping: 0.88 (velocity preserved, dots overshoot and wobble)
spring_k: 0.04 (standard), 0.01 (dreaming), 0.001 (dying)
```

Four forces prevent rest:
1. Ambient orbit (per-dot elliptical, random parameters)
2. Shimmer impulse (stochastic velocity kicks)
3. Damping at 0.88
4. Variable inputs changing targets continuously

### Eye Emotion Glyphs
| Emotion | Glyph | Unicode |
|---------|-------|---------|
| Joy/Anger | `◉` fisheye | U+25C9 |
| Trust | `◎` double circle | U+25CE |
| Fear | `●` filled | U+25CF |
| Surprise | `⦾` ring | U+29BE |
| Sadness | `◯` large circle | U+25EF |
| Disgust | `—` em dash | U+2014 |
| Anticipation | `◊` lozenge | U+25CA |
| Dreaming | `◇` diamond | U+25C7 |
| Phi peak | `✦` star | U+2726 |

---

## Crystallization Effect — Dopamine Reward Animation

Triggers when metrics cross significant thresholds. Four composited layers:

### Layer 1: Sparkle Particles
```css
@keyframes sparkle-float {
  0% { opacity: 1; transform: translate(0, 0) scale(1); }
  100% {
    opacity: 0;
    transform: translate(var(--sparkle-dx), var(--sparkle-dy)) scale(0.3);
  }
}
```
6-12 particles burst from element center. Randomized `--sparkle-dx` (-30 to 30px), `--sparkle-dy` (-40 to 10px, biased upward). Stagger 40-80ms. Total: ~800ms.

### Layer 2: Prismatic Shimmer
```css
@keyframes prismatic-shimmer {
  0% { background-position: -100% 0; opacity: 0; }
  30% { opacity: 1; }
  100% { background-position: 200% 0; opacity: 0; }
}
.crystallize-shimmer {
  background: linear-gradient(135deg,
    rgba(138,196,160,0.08),  /* T0 green */
    rgba(200,184,144,0.08),  /* T1 bone */
    rgba(204,144,168,0.08),  /* T2 rose */
    rgba(122,122,152,0.08)   /* dream blue */
  );
  background-size: 200% 100%;
  animation: prismatic-shimmer 600ms var(--ease-out) forwards;
  mix-blend-mode: screen;
}
```

### Layer 3: Ring Pulse
```css
@keyframes crystal-ring {
  0% { transform: scale(0.8); opacity: 0.6; border-color: rgba(220,165,189,0.4); }
  100% { transform: scale(1.6); opacity: 0; border-color: transparent; }
}
.crystallize-ring {
  position: absolute; inset: -4px;
  border: 2px solid rgba(220,165,189,0.4);
  animation: crystal-ring 500ms var(--ease-expo) forwards;
}
```

### Layer 4: Sound (optional)
```javascript
const playCrystallize = () => {
  const ctx = new AudioContext();
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.type = 'sine';
  osc.frequency.setValueAtTime(880, ctx.currentTime);       // A5
  osc.frequency.exponentialRampToValueAtTime(1760, ctx.currentTime + 0.08); // A6
  gain.gain.setValueAtTime(0.08, ctx.currentTime);
  gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.3);
  osc.connect(gain).connect(ctx.destination);
  osc.start(); osc.stop(ctx.currentTime + 0.3);
};
```

### Composite Timing
```
0ms     Sparkles + Ring pulse begin
50ms    Prismatic shimmer
500ms   Ring complete
600ms   Shimmer complete
800ms   Last sparkle fades
```
Debounce: max one per 3 seconds per element.

---

## Slot Machine Animation (Model Selection)

When displaying a model name or value that changes:

```css
@keyframes slot-roll {
  0%   { transform: translateY(-100%); opacity: 0.3; }
  70%  { transform: translateY(5%); opacity: 1; }
  100% { transform: translateY(0); opacity: 1; }
}
.slot-char { display: inline-block; overflow: hidden; height: 1.2em; width: 0.6em; }
.slot-char__inner {
  animation: slot-roll 300ms var(--ease-snappy) forwards;
  animation-delay: calc(var(--char-index) * 60ms);
}
```
Pre-roll: 2-3 random characters cycle at 50ms each before settling.

---

## Artifact Collectibles

Data entities displayed as shaped icons:

| Type | Shape | Color | Glyph |
|------|-------|-------|-------|
| Episode | Hexagon | rose | `⬡` |
| Insight | Diamond | bone | `◇` |
| HDC | Circle | dream | `●` |

```css
.artifact-icon--episode {
  clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
  background: linear-gradient(135deg, var(--rose-dim), var(--rose));
}
```

Generation sparkle:
```css
@keyframes artifact-appear {
  0%   { transform: scale(0); opacity: 0; filter: brightness(2); }
  60%  { transform: scale(1.3); opacity: 1; filter: brightness(1.5); }
  100% { transform: scale(1); opacity: 1; filter: brightness(1); }
}
/* 300ms — brightness flare sells the "just created" moment */
```

---

## Progressive Intensity System

Maps a 0.0–1.0 value to visual weight. Four bands:

| Band | Range | Text | Border | Background | Glow |
|------|-------|------|--------|------------|------|
| Ghost | 0.0–0.3 | `--text-ghost`, opacity 0.5 | `1px dotted --border-soft` | transparent | none |
| Building | 0.3–0.6 | `--text-dim`, opacity 0.8 | `1px solid --border` | `rgba(122,80,96,0.04)` | none |
| Confident | 0.6–0.8 | `--text-primary`, opacity 1.0 | `1px solid --rose-dim` | `rgba(122,80,96,0.08)` | `0 0 8px var(--rose-dim)` |
| Crystallized | 0.8–1.0 | `--text-strong`, opacity 1.0 | `1px solid --rose` | `rgba(122,80,96,0.12)` | `var(--glow-rose)` |

Use `data-intensity="0"` through `data-intensity="10"` attributes for CSS-driven states.

---

## HeartbeatLine (EKG Waveform)

Scrolling waveform. Classic EKG: flat baseline → P wave → QRS spike → T wave → flat.

Three timescales:
| Speed | Period | Maps to |
|-------|--------|---------|
| Fast | 0.7s | System pleasure/activity |
| Medium | 3-5s | Agent health |
| Glacial | 5+ min | Lifecycle phase |

```typescript
interface HeartbeatLineProps {
  speed?: 'fast' | 'medium' | 'glacial';
  bpm?: number;
  color?: string;            // default '--rose'
  thickness?: number;        // px, default 2
  height?: number;           // canvas height, default 40
  glowIntensity?: number;    // 0-1, default 0.3
  amplitude?: number;        // 0-1
}
```

---

## Glitch Overlay

Visual corruption at configurable intensity:

| Intensity | Effects |
|-----------|---------|
| 0.0–0.2 | Occasional scanline shift (1-2px), every 3-5s |
| 0.2–0.5 | Multiple scanlines, RGB split (1-2px), every 1-2s |
| 0.5–0.8 | Scanline blocks (8-16px), RGB split (2-4px), flickering |
| 0.8–1.0 | Heavy corruption: large block shifts, chromatic aberration, opacity flicker |

Burst mode: intensity spikes on trigger then decays exponentially.

---

## Phosphor Decay

Value changes don't snap — they ghost:
```
Frame 0 (current):  full brightness
Frame -1 (1 ago):   rose_dim
Frame -2:           text_dim
Frame -3:           text_ghost
Frame -4:           text_phantom
```

CRT persistence creates a trail effect on moving or changing elements. Duration scales by lifecycle phase.

---

## Ambient Philosophy Engine

Floating text fragments that appear during idle states:

| Tier | Pool | Trigger | Cooldown |
|------|------|---------|----------|
| Whisper | ~150 | Idle >30s | 60s |
| Epigraph | ~80 | Phase transition | 5 min |
| Apparition | ~40 | Major event | 10 min |
| Inscription | ~30 | Rare achievement | Once |

Fragment animation: 2s fade in → 5-8s hold → 3s fade out.
Position: lower third or margins. Never obscures data. `text-ghost` monospace.

---

## Knowledge Transfer Visualization

When agents share knowledge:
1. Source agent's Spectre pulses with brief glow
2. A colored dot detaches and animates along the edge (300ms travel)
3. Target agent's Spectre absorbs the particle (brief flash)
4. Log entry: `◈◈ auth → ◎◎ verify: shared "JWT signing approach"`

---

## Topology Graph

Force-directed graph with Spectre nodes:
- Node size = citation/task count
- Edges = handoff relationships, weighted by count
- Active agents: breathing animation
- Status rings: teal=active, green=done, rose=error, gray=idle, purple=blocked
- Edge animation: data particles flow along edges during communication
- Simulation never fully settles — background force keeps all nodes in motion

---

## Stepped Progress Rail

```
  (✓)═══════════(✓)═══════════(◉)───────────(○)───────────(○)
  Parse         Compose       Dispatch      Gate          Persist
```

Gradient across connecting lines:
- Done: `--status-success` → `--bone`
- Active: `--bone` → `--rose-glow` (partial fill)
- Pending: `--text-ghost` (flat)

**Comet effect:** 8px bright spot with 20px blur glow travels along filled portion. CSS: `translateX(0%) → translateX(100%)` over 2s infinite.

**Step dots:** 16px circles. Active: rotating arc spinner inside dot.

---

## Pixelated Dither Loading Transition

CRT-inspired content reveal:
1. While loading: show skeleton
2. On load: canvas overlay covers content
3. Over duration, pixels clear in pattern:
   - `random`: Bayer matrix dither
   - `scanline`: horizontal sweep top→bottom
   - `radial`: expanding rings from center
4. Canvas removed after transition

---

## Telemetry Sidebar

Fixed right-edge panel with live-updating system metrics:

```css
.telemetry-sidebar {
  position: fixed; right: 0; top: 50%; transform: translateY(-50%); z-index: 150;
  display: flex; flex-direction: column;
  border-left: 1px solid var(--border);
  background: rgba(6,6,8,0.85); backdrop-filter: blur(10px);
}
.telemetry-sidebar .tel-row {
  padding: 12px 18px; border-bottom: 1px solid var(--border-soft);
  font-family: var(--mono); font-size: 9px; letter-spacing: 0.18em;
  text-transform: uppercase; color: var(--text-dim);
}
.telemetry-sidebar .tel-row b {
  display: block; font-family: var(--display); font-style: italic;
  font-size: 18px; color: var(--rose-glow); text-shadow: 0 0 8px rgba(204,144,168,0.32);
}
```

---

## Hover Definition Terms

Interactive vocabulary tooltips sprinkled through content:

```css
.defterm {
  border-bottom: 1px dashed var(--rose-dim); cursor: help;
  color: var(--rose-glow); font-style: italic;
  text-shadow: 0 0 8px rgba(204,144,168,0.18);
}
.defterm::after {
  content: attr(data-def); position: absolute; left: 0; top: calc(100% + 14px);
  width: 340px; padding: 16px 20px;
  background: rgba(6,6,8,0.96); border: 1px solid var(--rose-dim);
  border-left: 2px solid var(--rose-glow);
  font-family: var(--display); font-size: 13px; color: var(--text-primary);
  opacity: 0; transform: translateY(-8px);
  transition: opacity 0.25s, transform 0.25s;
  box-shadow: 0 14px 40px rgba(0,0,0,0.6), 0 0 32px rgba(204,144,168,0.08);
}
.defterm:hover::after { opacity: 1; transform: translateY(0); }
```

---

## Loading Curtain

Full-screen loading overlay with animated diamond:

```css
#curtain {
  position: fixed; inset: 0; background: #060608; z-index: 99999;
  display: flex; align-items: center; justify-content: center; flex-direction: column; gap: 18px;
  transition: opacity 0.8s ease, visibility 0s linear 0.8s;
}
#curtain.gone { opacity: 0; visibility: hidden; }
#curtain .pulse {
  width: 22px; height: 22px; background: var(--rose-glow);
  clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
  animation: rotate 2.4s ease-in-out infinite;
  box-shadow: 0 0 18px var(--rose-glow);
}
@keyframes rotate {
  0%   { transform: rotate(0) scale(1); }
  50%  { transform: rotate(180deg) scale(0.7); }
  100% { transform: rotate(360deg) scale(1); }
}
```

---

## Terminal Simulation

Full CLI demo with realistic terminal chrome:

```css
.terminal {
  border: 1px solid var(--border); background: #0c0a14;
  box-shadow: 0 24px 80px rgba(0,0,0,0.55),
    0 0 0 1px rgba(204,144,168,0.04),
    inset 0 1px 0 rgba(220,165,189,0.06);
}
.terminal::before {  /* Top glow line */
  content: ""; position: absolute; top: 0; left: 0; right: 0; height: 1px;
  background: linear-gradient(90deg, transparent 10%, rgba(220,165,189,0.32) 50%, transparent 90%);
}
.term-bar .dots span:nth-child(1) { background: #cc4866; }  /* Close */
.term-bar .dots span:nth-child(2) { background: #c89a68; }  /* Minimize */
.term-bar .dots span:nth-child(3) { background: #7a8a78; }  /* Maximize */

.term-body {
  font-family: var(--mono); font-size: 13.5px; line-height: 1.75; color: #c8b8c0;
  /* Subtle scanline overlay in the terminal body */
  background-image: linear-gradient(rgba(220,165,189,0.012) 50%, transparent 50%);
  background-size: 100% 4px;
}
.term-body .pmt { color: #7a8a78; }           /* Prompt */
.term-body .pmt::before { content: "❯ "; color: #cc90a8; }
.term-body .cmd { color: #dca5bd; text-shadow: 0 0 8px rgba(220,165,189,0.32); }
.term-body .ok  { color: #7a8a78; }
.term-body .ok::before { content: "  ✓ "; }
.term-body .info { color: #c8b890; }
.term-body .info::before { content: "  ◇ "; color: #aa7088; }
.term-body .cursor {
  display: inline-block; width: 9px; height: 16px; background: #dca5bd;
  animation: cursorblink 1s steps(2) infinite;
  box-shadow: 0 0 8px rgba(220,165,189,0.5);
}
```

---

## Event-Driven Animation Map

| Event | Visual Response |
|-------|----------------|
| Plan started | Phase rail lights up |
| Agent spawned | Cell fades in with expanding ring (300ms) |
| Agent first output | LED starts pulsing, border glows |
| Agent done | LED settles steady, glow fades |
| Gate pass | Green check scales in + ripple |
| Gate fail | Red shake, failure card slides in |
| Plan complete | Summary mosaic scales in |
| Phase transition | Phase dot slides to next |
| Metrics update | Numbers spring to new values |
| Knowledge transfer | Particle animates along edge between agents |
| Error | Red border flash + shake |

---

## Anti-Patterns (Never Do)

- No pure white (`#fff`) text — max is `--text-strong` (#d8c8d0)
- No `transition: all` — always list specific properties
- No animation exceeding 400ms for UI feedback
- No viewport-locking inner panels
- No 8px/9px body text — minimum 11px labels
- No blank screens — skeleton for loading, message for empty
- No pie charts for more than 3 segments
- No decorative illustrations in empty states
- No "Welcome, User!" headers
- No grid lines in charts beyond axis scales
- No border-radius (ROSEDUST uses sharp corners)
