# Visual Architecture

The explorer is three things layered on top of each other:
a **scene** (3D/WebGL), a **dashboard** (glass panels), and an **atmosphere** (grain, scanlines, vignette).

## Design Language: ROSEDUST Applied to Chain Data

Every visual decision maps from the ROSEDUST system. The explorer is not a separate design —
it is the same terminal-existentialist aesthetic rendering a new domain.

### Color Mapping

| Chain concept | ROSEDUST color | Rationale |
|---------------|----------------|-----------|
| Blocks (confirmed) | `--rose` / `--rose-glow` (#aa7088 / #dca5bd) | Blocks are the heartbeat, rose is the life signal |
| Transactions (confirmed) | `--bone-bright` (#d8c8a0) | Transactions carry value — bone = value |
| Pending transactions | `--dream` / `--dream-bright` (#7a7a98 / #9494b4) | Not yet real — dream state |
| Gas / fees | `--warning` (#c89a68) | Burnt amber = combustion, cost |
| Contract creation | `--rose-bright` (#cc90a8) | Birth = bright rose |
| Failed / reverted | `--danger` (#cc5555) | Coral red, used sparingly |
| State roots | `--text-dim` (#6a5a68) | Structural, not featured |
| Validator activity | `--success` (#7a8a78) | Sage green = healthy participation |
| Empty blocks | `--text-ghost` (#3a303a) | Nearly invisible — nothing happened |

### Typography Mapping

| Element | Font | Size | Style |
|---------|------|------|-------|
| Block number (hero) | Fraunces italic | 38px | bone-bright, metric style |
| Block hash | JetBrains Mono | 11px | text-dim, tracking 0.06em |
| Transaction value | JetBrains Mono | 14px | bone-bright |
| Address labels | JetBrains Mono | 10px | uppercase, text-dim, tracking 0.28em |
| Section tags | JetBrains Mono | 11px | `—— 01 · BLOCKS`, tracking 0.32em |
| Status text | JetBrains Mono | 10px | uppercase, LED dot prefix |

### No Border Radius

All panels, cards, buttons: sharp corners. Rectangles. The chain is precise, not friendly.

---

## Layout Modes

The explorer has three modes. User toggles between them.
Each mode foregrounds different data at different scales.

### Mode 1: TERRAIN (Default — Immersive)

Full-viewport Three.js scene. Chain data rendered spatially.
Dashboard elements float as glass overlays at screen edges.

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  ┌─ KORA ──────────────────────┐     ┌─ BLOCK 286,401 ────┐ │
│  │  ● CONNECTED  1337          │     │  0x6d1f...ac1c     │ │
│  │  ▁▂▃▅▇▅▃▁ gas              │     │  0 txns  0 gas     │ │
│  └─────────────────────────────┘     │  1.00s ago         │ │
│                                      └────────────────────┘ │
│                                                              │
│              ╱╲                                              │
│          ╱╲╱  ╲╱╲        ← hash terrain, growing rightward  │
│      ╱╲╱        ╲╱╲                                          │
│  ╱╲╱              ╲╱╲                                        │
│                        ╲                                     │
│                                                              │
│    ◌ ─────── ● ─────── ◌      ← transaction particle arcs   │
│   0xa3..    value      0xb7..                                │
│                                                              │
│                                                              │
│  ┌─ FEE HISTORY ──────────────────────────────────────────┐  │
│  │ ▁▁▁▁▁▂▁▁▁▁▁▁▁▁▁▁▂▁▁▁▁▁▁ base fee (1 gwei flat)      │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

**Scene occupies 100vh.** Glass panels are `position: fixed`, `pointer-events: auto` on the panels only.
Background: the void (#060608). Scene renders on a transparent-alpha canvas.

### Mode 2: MOSAIC (Data-Dense)

Grid layout. Each block is a tile. Click to expand. No 3D — pure 2D generative art + data panels.

```
┌──────────────────────────────────────────────────────────────┐
│  —— 01 · BLOCK MOSAIC                          KORA / 1337  │
│                                                              │
│  ┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┐   │
│  │▓▓▓▓│░░░░│▒▒▒▒│████│░░░░│▓▓▓▓│▒▒▒▒│░░░░│████│▓▓▓▓│▒▒▒▒│  │
│  │▓▓▓▓│░░░░│▒▒▒▒│████│░░░░│▓▓▓▓│▒▒▒▒│░░░░│████│▓▓▓▓│▒▒▒▒│  │
│  │ 91 │ 92 │ 93 │ 94 │ 95 │ 96 │ 97 │ 98 │ 99 │ 00 │ 01 │  │
│  └────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┘   │
│  ┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┐   │
│  │    │    │    │    │    │    │    │    │    │    │    │  │
│  │    │    │    │    │    │    │    │    │    │    │    │  │
│  │ 80 │ 81 │ 82 │ 83 │ 84 │ 85 │ 86 │ 87 │ 88 │ 89 │ 90 │  │
│  └────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┘   │
│                                                              │
│  ┌─ BLOCK 286,394 ─────────────────────────────────────────┐ │
│  │  HASH      0x03c3ced73420b86f54501198836e6b...          │ │
│  │  STATE     0x3eef323af7ecaed386b057ac7fba53...          │ │
│  │  GAS       0 / 250,000,000                              │ │
│  │  TXNS      0                                            │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

**Tile generation:** Each block hash → deterministic visual pattern.
The 32 bytes of the hash seed: hue rotation, pattern density, symmetry axis, fill algorithm.
Empty blocks are ghost-opacity. Blocks with transactions are vivid.
The grid scrolls — new blocks append at top-right, old blocks flow down-left.

### Mode 3: DETAIL (Single Entity Focus)

Full-page view of one block, one transaction, or one address.
Split layout: data on left, visualization on right.

```
┌──────────────────────────────────────────────────────────────┐
│  ← BACK                                    BLOCK 286,401    │
│                                                              │
│  ┌─ DATA ────────────────────┐  ┌─ VISUAL ────────────────┐ │
│  │                           │  │                          │ │
│  │  HASH                     │  │   ┌──────────────────┐   │ │
│  │  0x6d1fb175dce4b6795c...  │  │   │  generative art  │   │ │
│  │                           │  │   │  from this block  │   │ │
│  │  PARENT                   │  │   │  hash — unique    │   │ │
│  │  0x469162b1b3e2cc0b52...  │  │   │  to this block   │   │ │
│  │                           │  │   └──────────────────┘   │ │
│  │  STATE ROOT               │  │                          │ │
│  │  0x3eef323af7ecaed386...  │  │   Transaction flow       │ │
│  │                           │  │   diagram (if any txns)  │ │
│  │  TRANSACTIONS    0        │  │                          │ │
│  │  GAS USED        0        │  │   or                     │ │
│  │  GAS LIMIT       250M     │  │                          │ │
│  │  BASE FEE        1 gwei   │  │   State diff tree        │ │
│  │  TIMESTAMP       286401   │  │   (if debug_trace avail) │ │
│  │                           │  │                          │ │
│  └───────────────────────────┘  └──────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

---

## Atmospheric Layers

Applied globally across all modes. These are what make it feel *crafted* instead of *built*.

### Layer 1: Grain (z-index 9997)
```css
.grain {
  position: fixed; inset: 0; pointer-events: none; z-index: 9997;
  opacity: 0.035; mix-blend-mode: overlay;
  background-image: url("data:image/svg+xml;utf8,<svg ...>
    <feTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='3' stitchTiles='stitch'/>
  </svg>");
}
```
Fractal noise at 3.5% opacity. Gives every surface subtle texture. Never clean digital.

### Layer 2: Vignette (z-index 9998)
```css
body::before {
  background: radial-gradient(ellipse at 50% 30%, transparent 50%, rgba(6,6,8,0.72) 100%);
}
```
Directional darkening. Edges recede. Center focuses. Cinematic framing.

### Layer 3: Scanlines (z-index 9999)
```css
body::after {
  background: repeating-linear-gradient(to bottom,
    transparent 0, transparent 2px,
    rgba(0,0,0,0.45) 2px, rgba(0,0,0,0.45) 3px);
  opacity: 0.06; mix-blend-mode: multiply;
}
```
CRT artifact. 2px transparent, 1px dark, repeating. At 6% opacity it's subliminal — you feel it more than see it.

### Layer 4: Rose Wash (unique to explorer)
A subtle `radial-gradient` that tracks the most recent block's activity level.
Empty blocks → wash is invisible. Full blocks → faint rose light emanates from the center.
```css
.rose-wash {
  background: radial-gradient(ellipse at 50% 50%,
    rgba(170, 112, 136, var(--activity)) 0%,
    transparent 60%);
  transition: --activity 2s ease-out;
}
```
The room *breathes* with chain activity.

---

## Glass Panel System

All data overlays use the same glass component:

```
┌─ LABEL ──────────────────────────┐
│                                  │  ← inset 0 1px 0 rgba(255,255,255,0.06)
│   content                        │  ← backdrop-filter: blur(12px) saturate(180%)
│                                  │  ← background: rgba(8, 8, 12, 0.45)
│                                  │  ← border: 1px solid rgba(255,255,255,0.07)
└──────────────────────────────────┘
```

- Left rose accent border (2px `--rose-dim`) on panels showing active/live data
- LED status dot (5px, pulsing) in panel headers for connected/streaming states
- Mono 10px uppercase label with tracking 0.28em
- Sharp corners. Always.

---

## Responsive Behavior

| Breakpoint | Adaptation |
|------------|------------|
| > 1400px | Full layout, terrain mode default, side panels |
| 1100-1400px | Panels stack below scene, scene height reduced |
| 760-1100px | Mosaic default (no 3D), panels full-width |
| < 760px | Single-column, mosaic tiles 3-wide, simplified panels |

Mobile does not attempt 3D. It defaults to MOSAIC mode with hash-art tiles, which is visually striking on its own and doesn't drain batteries.
