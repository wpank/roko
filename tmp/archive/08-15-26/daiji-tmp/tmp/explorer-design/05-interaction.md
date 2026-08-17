# Interaction Model

The explorer has two interaction postures: **ambient** (lean back, watch) and **investigative** (lean in, explore). The default is ambient. Interaction pulls you into investigative. Inactivity fades you back to ambient.

---

## Ambient Mode (Default)

The explorer runs unattended. No cursor visible, no panels highlighted. Just the scene,
the atmosphere, and the data flowing.

**What happens:**
- Terrain grows rightward, camera tracks slowly
- Constellation drifts, nodes pulse
- Waterfall blocks fall and stack
- Fee waveform scrolls
- Block pulse ticks

**No interaction required.** This is the "put it on the big screen in the office" mode.
It is visually interesting without anyone touching it.

**Idle timeout:** After 30 seconds of no mouse/keyboard activity, panels fade to 50% opacity
and eventually to 20%. The scene takes full visual priority. Atmospheric layers remain.

---

## Investigative Mode (On Interaction)

Any mouse movement or keypress transitions to investigative mode (200ms fade-in of panels).

### Mouse

| Action | Terrain Mode | Constellation Mode | Mosaic Mode |
|--------|-------------|-------------------|-------------|
| **Move** | Camera tilt (±5° lerp) | Highlight nearest node | Highlight nearest tile |
| **Click** | Nothing (terrain is ambient) | Select address → detail panel | Select block → detail panel |
| **Scroll** | Zoom in/out (camera Z) | Zoom in/out | Scroll grid |
| **Drag** | Pan terrain (camera X/Y) | Pan constellation | — |
| **Double-click** | — | Zoom to address | Expand tile to full detail |

### Hover States

All hover states follow ROSEDUST rules:
- `translateY(-2px)` on panels/tiles (small, not dramatic)
- `background: var(--bg-glass-hover)` (rose-tinted glass)
- Transition: 80ms on hover-in, 120ms on hover-out (asymmetric)
- Never `transform: scale()` on hover — only Y translate

**Address node hover (constellation):**
```
       ┌─ 0xa3f2...8b01 ──────┐
       │  12.45 ETH            │    ← appears 200ms after hover
       │  142 txns             │    ← JetBrains Mono 10px
       │  ● ACTIVE             │    ← LED + status
       └───────────────────────┘
             │
             ◉  ← node brightens, halo expands
```
Tooltip: glass panel, appears with `fadeUp` animation (opacity 0→1, translateY 4px→0, 200ms expo ease).
Offset: always above node, centered horizontally, clamped to viewport.

**Block tile hover (mosaic):**
Tile border brightens to `--border-strong`. Block number label appears inside tile.
Adjacent tiles dim slightly (0.7 opacity) to focus attention.

**Waterfall block hover:**
Block expands to show transaction detail lines. Mono labels appear.

### Keyboard

| Key | Action |
|-----|--------|
| `1` | Switch to TERRAIN mode |
| `2` | Switch to MOSAIC mode |
| `3` | Switch to DETAIL mode (last viewed entity) |
| `Space` | Pause/resume auto-scroll (terrain camera, waterfall) |
| `←` `→` | Step through blocks (when paused) |
| `/` | Focus search input |
| `Esc` | Close detail panel / exit search / return to ambient |
| `F` | Toggle fullscreen |

### Search

Triggered by `/` key or clicking the search area.

```
┌─ ⌕ ─────────────────────────────────────────────┐
│  0xa3f2...                                       │    ← JetBrains Mono 14px
│                                                  │    ← glass panel, auto-focus
│  RESULTS                                         │
│  ┌─ ADDRESS ──────────────────────────────────┐  │
│  │  0xa3f2...8b01    12.45 ETH    142 txns    │  │
│  └────────────────────────────────────────────┘  │
│  ┌─ BLOCK ────────────────────────────────────┐  │
│  │  #286,401    0 txns    1s ago              │  │
│  └────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

Search input accepts:
- Block number (decimal or hex)
- Transaction hash (0x...)
- Address (0x...)

Results appear instantly as you type (local cache first, then RPC fallback).
Select a result → navigate to detail view for that entity.

---

## Detail Views

### Block Detail

```
┌──────────────────────────────────────────────────────────────┐
│  ← BLOCKS                                   BLOCK 286,401   │
│                                                              │
│  ┌─ OVERVIEW ──────────────┐  ┌─ HASH ART ────────────────┐ │
│  │                         │  │                            │ │
│  │  HASH                   │  │  ┌────────────────────┐    │ │
│  │  0x6d1fb175dce4b679...  │  │  │                    │    │ │
│  │                         │  │  │  128×128 canvas    │    │ │
│  │  PARENT                 │  │  │  generative art    │    │ │
│  │  0x469162b1b3e2cc0b...  │  │  │  from block hash   │    │ │
│  │                         │  │  │                    │    │ │
│  │  STATE ROOT             │  │  │  (click to expand  │    │ │
│  │  0x3eef323af7ecaed3...  │  │  │   to fullscreen)   │    │ │
│  │                         │  │  │                    │    │ │
│  │  ┌─ METRICS ──────────┐ │  │  └────────────────────┘    │ │
│  │  │ GAS    │ FEE       │ │  │                            │ │
│  │  │ 0      │ 1 gwei    │ │  │  block.hash → visual      │ │
│  │  │ / 250M │           │ │  │  unique, deterministic     │ │
│  │  └────────┴───────────┘ │  │  downloadable as PNG       │ │
│  │                         │  │                            │ │
│  └─────────────────────────┘  └────────────────────────────┘ │
│                                                              │
│  —— TRANSACTIONS ──────────────────────────────────────────  │
│                                                              │
│  (empty — no transactions in this block)                     │
│                                                              │
│  ← BLOCK 286,400              BLOCK 286,402 →               │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

Hash values: JetBrains Mono 11px, text-dim, click to copy (flash bone on copy).
Metric cells: standard ROSEDUST mosaic component (1px gap grid, glass bg).
Navigation: arrow keys or click to step through blocks.

### Address Detail

```
┌──────────────────────────────────────────────────────────────┐
│  ← ADDRESSES                  0xa3f2...8b01                 │
│                                                              │
│  ┌─ IDENTITY ──────────────┐  ┌─ CONSTELLATION ────────────┐│
│  │                         │  │                            ││
│  │  BALANCE                │  │  zoomed-in view of this    ││
│  │  12.450000 ETH          │  │  address in the            ││
│  │                         │  │  constellation, showing    ││
│  │  NONCE                  │  │  connected addresses       ││
│  │  142                    │  │  and recent arcs           ││
│  │                         │  │                            ││
│  │  TYPE                   │  │         ◌                  ││
│  │  ◈ CONTRACT             │  │       ╱   ╲               ││
│  │                         │  │     ◉ ← you  ◌            ││
│  │  CODE SIZE              │  │       ╲   ╱               ││
│  │  4,892 bytes            │  │         ◌                  ││
│  │                         │  │                            ││
│  └─────────────────────────┘  └────────────────────────────┘│
│                                                              │
│  —— RECENT TRANSACTIONS ───────────────────────────────────  │
│                                                              │
│  HASH              TO              VALUE        GAS          │
│  0x1a2b...  →  0xc3d4...8901   0.5 ETH     21,000          │
│  0x5e6f...  →  0x7a8b...2345   1.0 ETH     21,000          │
│  ...                                                         │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

Balance: Fraunces italic 38px, bone-bright (the hero number).
Transaction table: standard ROSEDUST table component (hover rows, stagger entrance).

### Transaction Detail

```
┌──────────────────────────────────────────────────────────────┐
│  ← BLOCK 286,401                              TX 0x1a2b...  │
│                                                              │
│  ┌─ FLOW ──────────────────────────────────────────────────┐ │
│  │                                                          │ │
│  │    0xa3f2...8b01  ──────── 0.5 ETH ────────▶  0xc3d4... │ │
│  │    SENDER                                     RECEIVER   │ │
│  │                                                          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌─ DATA ──────────────┐  ┌─ EXECUTION ───────────────────┐ │
│  │                     │  │                                │ │
│  │  STATUS  ● SUCCESS  │  │  GAS LIMIT     21,000         │ │
│  │  BLOCK   286,401    │  │  GAS USED      21,000         │ │
│  │  INDEX   0          │  │  GAS PRICE     1 gwei         │ │
│  │  NONCE   141        │  │  FEE PAID      0.000021 ETH   │ │
│  │  TYPE    0x2 (1559) │  │                                │ │
│  │  VALUE   0.5 ETH    │  │  ┌──────────────────────┐     │ │
│  │                     │  │  │ ██████████████████ █ │     │ │
│  │                     │  │  │ gas usage bar       │     │ │
│  │                     │  │  └──────────────────────┘     │ │
│  │                     │  │                                │ │
│  └─────────────────────┘  └────────────────────────────────┘ │
│                                                              │
│  —— CALL TRACE (requires debug_traceTransaction) ──────────  │
│                                                              │
│  CALL  0xa3f2... → 0xc3d4...  value: 0.5 ETH               │
│    └─ SSTORE  slot 0x01 = 0x...                             │
│    └─ LOG     Transfer(from, to, amount)                    │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

Flow diagram: bone-bright arc with animated particles (reuses constellation arc renderer).
Call trace (if available): tree structure, ROSEDUST table style, each opcode type color-coded.

---

## Transitions

### Mode Switch (1 → 2 → 3)

1. Current scene fades out (opacity 1→0, 300ms)
2. 100ms gap (void visible — intentional beat)
3. New scene fades in (opacity 0→1, translateY 12px→0, 300ms expo ease)

Total: 700ms. Feels deliberate, not instant. Not sluggish.

### Detail Panel Open

1. Clicked element pulses once (rose-glow flash, 200ms)
2. Background scene dims to 30% opacity (300ms)
3. Detail panel slides in from right (translateX 100%→0, 400ms expo ease)

### Detail Panel Close

1. Panel slides out (translateX 0→100%, 300ms)
2. Background scene restores to 100% (300ms, overlapping)
3. If came from a specific element, camera smoothly returns to its position

### Block Arrival (Ambient)

Every 1 second:
1. Terrain: new tile fades in at right edge (opacity 0→1, 400ms)
2. Waterfall: new block falls from top (400ms expo deceleration)
3. Pulse: current dot flashes (80ms)
4. Status panel: block number increments (value-flash animation, 300ms)
5. If block has transactions: rose-wash intensity pulses (0→0.15→0, 2s)

All 5 happen simultaneously. The block arrival is a *chord*, not a sequence.

---

## Touch (Mobile/Tablet)

| Gesture | Action |
|---------|--------|
| Tap | Select (same as click) |
| Long press | Show tooltip (same as hover) |
| Pinch | Zoom |
| Two-finger drag | Pan |
| Swipe left/right | Navigate between blocks (detail view) |
| Swipe down | Close detail panel |

No 3D scenes on mobile — defaults to MOSAIC mode with hash-art tiles.
Tiles are tap targets (minimum 44×44px). Detail panels are full-screen on mobile.

---

## Audio (Optional, Off by Default)

Toggle with speaker icon in status bar.

| Event | Sound |
|-------|-------|
| Block arrival | Soft click / tick (wood block, 50ms) |
| Transaction confirmed | Gentle chime, pitch ∝ value (higher value = higher pitch) |
| Transaction pending | Low hum note (sustained while pending) |
| Transaction reverted | Dull thud |
| Connection lost | Ambient drone fades out |
| Connection restored | Ambient drone fades in |

Ambient layer: continuous generative drone.
Base frequency: 55Hz (A1). Harmonics added based on chain activity.
Gas pressure modulates filter cutoff. More gas = brighter timbre.
Empty chain = deep, dark, minimal. Busy chain = rich, shimmering.

Implementation: Web Audio API, OscillatorNode + BiquadFilterNode.
No audio files — everything synthesized from chain state.
