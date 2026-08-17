# Scenes — Detailed Visual Specifications

Each scene is a self-contained Three.js or Canvas2D visualization that consumes chain data
and renders it continuously. Scenes compose — multiple can run simultaneously with different
weights/opacities depending on the explorer mode.

---

## Scene 1: Hash Terrain

**The signature visual.** An infinite procedural landscape generated from block hashes.

### Concept

Every block hash is 32 bytes. Those bytes seed a terrain heightmap tile.
As blocks arrive (1/sec), a new tile appends to the right edge of the landscape.
The camera slowly tracks rightward, revealing new terrain as the chain grows.
You are watching the chain literally *build ground beneath itself*.

### Generation Algorithm

```
block.hash (32 bytes) → split into 8 groups of 4 bytes each

bytes[0..4]   → base elevation (0.0 - 1.0)
bytes[4..8]   → roughness / noise frequency
bytes[8..12]  → ridge presence and direction
bytes[12..16] → erosion factor
bytes[16..20] → hue offset from base rose palette
bytes[20..24] → saturation modifier
bytes[24..28] → tile-to-tile blend curve
bytes[28..32] → secondary feature placement (peaks, valleys)
```

Each tile is a 16×16 vertex grid (256 vertices). Heightmap values interpolated with Perlin noise seeded by the hash bytes. Adjacent tiles blend at edges using the `blend curve` bytes for continuity.

### Visual Treatment

- **Base color:** rose-deep (#3a2030) at valleys → rose-glow (#dca5bd) at peaks
- **Empty blocks:** very low relief (flat terrain), ghost opacity, barely visible
- **Blocks with transactions:** dramatic relief, bright peaks, bone-colored highlights at summits
- **Gas usage:** modulates terrain roughness — high gas = jagged, complex; low gas = smooth, gentle
- **State root changes:** when stateRoot differs from parent, the color palette shifts subtly — the "geology" changes

### Lighting

- Single directional light from upper-left (simulating the vignette light source)
- Ambient light: very low, rose-tinted
- Fog: exponential, starting at z=40, color = void (#060608)
- No shadows (too expensive, and fog provides depth cues)

### Camera

- Orthographic or low-FOV perspective (15°) for that flat, tilt-shift look
- Slowly tracks rightward at 1 tile/second (matching block time)
- Pointermove lerp: mouse position gently tilts camera ±5° on both axes
- Smooth lerp factor: 0.04 (glacial response, not twitchy)

### Transaction Markers

When a block contains transactions, vertical beams of light rise from the terrain at that tile:
- One beam per transaction
- Height = gas used (relative to block gas limit)
- Color = bone-bright (value transfer) or rose-bright (contract creation)
- Beam has volumetric glow (additive blended billboard quad)
- Fades over 10 seconds after block arrival

### Three.js Implementation Notes

```javascript
// Terrain as InstancedMesh of planes, or single BufferGeometry
// updated per-block
const geometry = new THREE.PlaneGeometry(1, 1, 15, 15);
// Modify vertex Y positions based on hash-derived heightmap
// Use ShaderMaterial for height-based coloring + fog

const material = new THREE.ShaderMaterial({
  uniforms: {
    uColorLow:  { value: new THREE.Color(0x3a2030) },
    uColorHigh: { value: new THREE.Color(0xdca5bd) },
    uFogColor:  { value: new THREE.Color(0x060608) },
    uFogDensity: { value: 0.025 },
  },
  vertexShader: `/* height-based color + fog */`,
  fragmentShader: `/* lerp colors by height, apply exp fog */`,
  transparent: true,
});
```

Ring buffer of ~200 tiles. Oldest tiles recycled as new blocks arrive. Camera position modulo ring size.

---

## Scene 2: Particle Constellation

**The network visualizer.** Addresses as stars. Transactions as particle streams between them.

### Concept

Every address that has appeared on-chain gets a stable position in 2D space (derived from address hash). The address exists as a softly glowing node. When a transaction connects two addresses, a stream of particles flows from sender to receiver along a curved arc.

### Node Placement

```
address (20 bytes) → deterministic (x, y) position

bytes[0..4]  → angle on a golden-ratio spiral (θ = hash_int * φ)
bytes[4..8]  → radius from center (log scale, so active addresses cluster inward)
bytes[8..12] → z-depth (subtle parallax, ±0.5 units)
```

Addresses with more transactions naturally appear at smaller radii (more active = more central) through a running average that pulls high-activity nodes inward.

### Node Rendering

- **Dormant address:** 2px dot, text-ghost color, no glow. Nearly invisible.
- **Active address (recent txn):** 4-6px dot, rose-glow, subtle pulse animation (2.4s), soft halo
- **Contract:** diamond shape (rotated square), rose-bright
- **High-value address:** larger dot, bone-bright ring, double halo
- **Hovered address:** label appears (JetBrains Mono 10px, address truncated), balance shown

### Transaction Arcs

When a transaction occurs:

1. **Arc path:** Cubic bezier from sender to receiver. Control point offset perpendicular to the line, magnitude proportional to value
2. **Particle stream:** 8-16 particles travel along the arc over 1.5-3 seconds
3. **Particle style:** 2px circles, bone-bright, additive blend, slight size variation
4. **Trail:** Each particle leaves a fading trail (0.3s decay)
5. **Value encoding:** More particles = higher value. Particle brightness = gas price relative to base fee
6. **Direction:** Particles always flow sender → receiver

After the stream completes, the arc path persists as a ghost line (text-ghost opacity) for 30 seconds, showing the connection existed.

### Idle State (Empty Chain)

When no transactions are flowing (current state of this chain):
- Known addresses pulse gently at staggered intervals
- Faint connection lines between previously-connected addresses at ghost opacity
- Slow orbital drift — all nodes rotate around center at 0.001 rad/s
- Occasional "firefly" ambient particles drift through empty space
- The constellation breathes — nodes scale up/down 2% on a 5s cycle

### With Pending Transactions (requires `eth_subscribe("newPendingTransactions")`)

- Pending tx: particle spawns at sender but doesn't arc yet — orbits the sender node, pulsing dream-bright (#9494b4)
- Inclusion: orbiting particle *snaps* along the arc to receiver, color shifts dream → bone
- Revert: orbiting particle shatters (8 fragments, outward, fade to danger red)

### Camera

- 2D top-down (orthographic), slight perspective tilt optional
- Scroll wheel zooms (smooth lerp)
- Click-drag pans
- Double-click an address → zoom to it, show detail panel

### Implementation Notes

WebGL2 particle system. Not Three.js InstancedMesh (too heavy for thousands of particles).
Use raw WebGL2 with transform feedback for GPU-side particle physics, or a lightweight lib like `regl`.

```javascript
// Particle buffer: position, velocity, lifetime, color, size
// Updated per-frame on GPU via transform feedback
// Rendered as GL_POINTS with custom fragment shader for soft circles
```

---

## Scene 3: Block Waterfall

**The timeline.** Blocks as translucent volumes falling and stacking.

### Concept

New blocks appear at the top of the screen and fall downward, stacking into a column.
Each block is a translucent rectangular volume. Its visual properties encode the block's data.
The stack compresses as it grows — older blocks become thinner, eventually just lines.

### Block Rendering

Each block is a rect with:

```
Width:    fixed (matches column width)
Height:   lerp(4px, 60px) based on gas_used / gas_limit
          empty blocks = thin slivers, full blocks = tall
Border:   1px solid rgba(255,255,255,0.07) — standard ROSEDUST border
Fill:     hash-derived pattern (see below)
Left bar: 2px accent — rose-dim normally, rose-glow if block has txns
```

### Hash-Derived Fill

Each block's interior fill is generated from its hash:
- Divide the rect into 8 vertical bands
- Each band's opacity/color comes from 4 bytes of the hash
- Result: each block has a unique "barcode" pattern — visually distinct, recognizable

### Falling Animation

1. Block spawns above viewport, transparent
2. Falls with eased deceleration (expo ease-out) over 400ms
3. Lands on top of stack with subtle "impact" — 1px white flash on bottom edge, 80ms
4. Block below compresses slightly (spring physics, 200ms settle)

### Stack Behavior

- Visible stack: 50 most recent blocks
- Blocks compress as they age: `height = base_height * (1 - age/50 * 0.7)`
- Oldest visible blocks are nearly line-thin
- Below the visible stack: fade to void

### Transaction Indicators Within Blocks

When a block has transactions, they're visible inside the block rect:
- Each transaction is a horizontal line at a Y position proportional to its index
- Line color: bone-bright (value transfer) or rose (contract interaction)
- Line length: proportional to gas used
- Hovering the block expands it and reveals transaction details

### Complementary Element: Block Pulse

Below the waterfall, a pulse indicator:
```
●───●───●───●───●───●───●───●
```
One dot per second, advancing left to right. When a block arrives, the current dot flashes rose-glow.
Missed blocks (if any) leave a gap. This is the chain's heartbeat — steady rhythm made visible.

---

## Scene 4: Consensus Ring (requires `kora_consensusState`)

**The governance visualizer.** Validators as nodes on a ring. Consensus as light accumulating.

### Concept

N validators arranged on a circle. As a consensus round progresses:
1. **Propose:** One node glows bright (the proposer), sends a pulse outward
2. **Pre-vote:** Nodes that vote glow rose as they sign. Lines connect them to center.
3. **Pre-commit:** Voted nodes intensify. The center begins to crystallize.
4. **Finalize:** Threshold reached → the center *flashes* and a block materializes, drops into the waterfall

### Visual

```
         ◉  proposer (bright rose, pulsing)
       ╱   ╲
      ●     ●  voted (rose-dim, connected to center)
     ╱  ┌─┐  ╲
    ○   │◆│   ○  center: forming block (dream → rose as votes accumulate)
     ╲  └─┘  ╱
      ●     ○  not yet voted (text-ghost)
       ╲   ╱
         ○
```

### Threshold Visualization

A circular progress arc around the ring. As votes come in, the arc fills.
2/3 threshold is marked with a faint line. When the arc crosses it: crystallization event.

### Without `kora_consensusState`

If the custom RPC isn't available, this scene can approximate by using block arrival timing.
Blocks arriving at steady 1s intervals → calm steady ring pulse.
Blocks arriving late → ring "strains," amber warning glow.
Missed blocks → ring dims, gap in pulse.

---

## Scene 5: State Diff Aurora (requires `debug_traceTransaction` or state diff data)

**The change visualizer.** State changes as color bands in an aurora-like display.

### Concept

When a block changes state (different stateRoot from parent), the *type* and *magnitude*
of changes render as flowing color bands across the top of the viewport:

- Balance changes → bone-colored band, width = total ETH moved
- Storage writes → rose-colored band, width = number of slots written
- Contract creations → bright rose flash, single vertical stripe
- Self-destructs → red band, sharp edge

The bands flow left-to-right, stretching and fading, like a slow-motion aurora borealis.
When the chain is idle (same stateRoot), the aurora dims to nothing.

This is pure atmosphere — no data to click, no interactivity. Just a visual ambient indicator
of "how much changed in the world this second."

---

## Scene 6: Contract Organism (requires `eth_getCode` + `debug_storageRangeAt`)

**The deep dive.** A single contract rendered as a living organism.

### Concept

Select a contract address. Its bytecode length determines body size.
Its storage slots are rendered as a grid of cells (like a cellular automaton).
As blocks pass and storage changes, cells light up, change color, pulse.

- **Cold storage (unchanged):** text-ghost, barely visible
- **Warm storage (changed in last 100 blocks):** rose-dim, soft pulse
- **Hot storage (changed this block):** bone-bright, sharp flash, decay over 5s
- **New slot (first write):** rose-bright border flash (birth)
- **Zeroed slot (cleared):** brief red flash, then invisible

### Layout

The storage grid wraps into a roughly square shape. The contract's "body" is this grid.
External calls to the contract appear as particles arriving from off-screen, hitting the organism,
and triggering cell changes.

This scene only works for a single contract at a time. It's the microscope view.

---

## Scene Composition

Multiple scenes can run simultaneously with controlled blending:

| Explorer Mode | Primary Scene | Secondary Scene | Overlay |
|---------------|---------------|-----------------|---------|
| TERRAIN | Hash Terrain (100%) | — | Glass panels, pulse |
| MOSAIC | — (2D tiles instead) | — | Glass panels |
| DETAIL: Block | Block Waterfall (left) | Hash Terrain tile (right bg) | Data panels |
| DETAIL: Address | Particle Constellation (focused) | — | Balance/tx panels |
| DETAIL: Contract | Contract Organism | — | Code/storage panels |
| DETAIL: Consensus | Consensus Ring | Block Waterfall (small) | Validator panels |

Scene transitions use a 500ms crossfade (opacity) with the incoming scene offset by `translateY(12px)` fading up (standard ROSEDUST entrance).
