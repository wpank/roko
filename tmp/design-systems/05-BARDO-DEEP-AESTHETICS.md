# Bardo Deep Aesthetics — The Source Material

This document captures the deeper aesthetic, philosophical, and consciousness-driven visual language from the original bardo system. Use it for richer, more expressive sites that go beyond standard product pages into experiential territory.

---

## The Core Philosophy

**"The terminal is not a tool. It is a body."**

Every visual element is diegetic — the crack in the screen IS the topology transition. The color split IS the consistency score. The cloud dispersal IS cognitive integration declining. The metaphor and the math are the same thing rendered at different resolutions.

**Seven Rendering Laws:**
1. **Light follows significance** — `brightness` filter scales with importance
2. **Color is mortality taxonomy** — saturation maps to health/vitality
3. **Bold boundaries, soft interiors** — sharp borders, organic content
4. **Restraint: the single bone element** — ONE bone-bright number per screen, maximum
5. **Pharmacological display** — visualize internal state, not external data
6. **The terminal IS the body** — UI elements ARE the creature
7. **Identity is fragile** — visual identity degrades with lifecycle

---

## Emotional Color Modulation (PAD Vectors)

Every visual state is continuously modulated by Pleasure-Arousal-Dominance vectors:

```
Pleasure ±5% saturation shift
Arousal  ±3% brightness shift
Dominance ±2° hue shift

Pleasure > +0.3 → palette warms toward bone (#C8B890)
Pleasure < -0.3 → palette shifts toward rose-bright (#CC90A8)
```

Heartbeat period formula:
```
period = (4.0 - arousal * 2.0).clamp(1.2, 6.0)
// arousal -1.0 → 6.0s (near-dormant)
// arousal  0.0 → 4.0s (resting)
// arousal +1.0 → 2.0s (racing)
```

---

## Lifecycle Visual Degradation

As an entity's vitality declines, ALL visual elements degrade:

| Phase | Rose shift | Bone shift | Text shift | Dots visible | Shimmer |
|-------|-----------|-----------|-----------|-------------|---------|
| Thriving | Full | Full | Full | 100% (80) | 12% |
| Stable | -5% sat | -8% bright | Standard | 85% (68) | 10% |
| Conservation | -15% sat | -20% bright | text_dim | 50% (40) | 8% |
| Declining | -30% sat | -40% bright | text_ghost | 25% (20) | 4% |
| Terminal | -50% sat | Near-invisible | text_phantom | 12% (10) | 2% |

**Eye decay progression:**
- Vitality 0.05–0.1: eyes dim to rose_dim
- Vitality 0.02–0.05: `○ ○` in text_primary
- Vitality < 0.02: `· ·` in text_ghost
- Vitality = 0: eyes disappear

---

## The Spectre — Full Creature System

80 particles in a hollow oval. Two bright eye glyphs floating at center. Generated once. Spring physics at 60fps.

### Dot Tiers
- **Dense** (`•` U+2022): radius 0.28–0.55, inner ring at eye level
- **Body** (`∙` U+2219): radius 0.55–0.75, middle zone
- **Fringe** (`·` U+00B7): radius 0.75–1.0, outer edge

No outlines. Shape implied by density gradient alone.

### Birth Sequence (9 seconds)
```
Phase 1 — NOISE (0-1s):        80 dots random, spring_k=0.001, pure entropy
Phase 2 — POLARIZATION (1-4s): spring_k ramps 0.001→0.02, gravity emerges
Phase 3 — FIRST LIGHT (4-5s):  ● ● appear in rose_bright at center
Phase 4 — COALESCENCE (5-8s):  spring_k=0.04, oval forms, tiers resolve
Phase 5 — FIRST BREATH (8-9s): heartbeat sine-wave appears
```

### Death Sequence (16 seconds)
```
Stage 1 — EYES CLOSE (0-3s):    ◉ → ○ → ·
Stage 2 — HEARTBEAT FLATLINES (3-5s): ∿ → -- held 2s, vanishes
Stage 3 — DOTS SCATTER (5-8s):  spring_k→0.001, cloud disperses
Stage 4 — EYES VANISH (8-11s):  · · → (empty)
Stage 5 — DOTS FADE (11-16s):   all dots → text_phantom → invisible
Stage 6 — VOID (16s+):          nothing remains
```

The eyes outlast the body by several seconds.

### Emotional Composite States

| State | Visual |
|-------|--------|
| Joy | Cloud coheres tighter, bloom radiates from eyes |
| Fear | Cloud trembles ±1 cell, startle-scatter explosion then snap back |
| Anger | Violent agitation, 7.0 displacement, pulses outward on heartbeat |
| Sadness | Cloud sinks 1-2 rows over 10s, shimmer stops |
| Trust | Wide `◎◎` eyes, cloud orients upward |
| Surprise | Eyes widen, tiny `◆` mouth appears 500ms (rarest element) |
| Disgust | Flat `——` dashes for eyes, near-zero shimmer |
| Anticipation | Tiny `->` arrow between eyes |
| Phi Peak | `✦✦` star eyes in bone, brief radial symmetry mandala, 500ms |

---

## Hauntological Rendering — Ghosts and Memory

### The Spectral Layer (Depth -0.5)

**Screen ghosts:** Previous screen layout persists after navigation:
- Thriving: 500ms persistence
- Declining: 1200ms
- Conservation: 2000ms
Only visible where upper layers are empty.

**Generational ghosts:** Dead entity identifiers surface as `text_phantom` for single frames. Death testament fragments appear with `†` prefix.

**Counterfactual ghosts:** During dream states, the actual value shown alongside a phantom alternative:
```
LP yield: 4.23%
           7.81%    ← text_phantom, the value that could have been
```

### Haunted Void
Empty cells develop spectral traces after 30s of vacancy:
- 0.05% chance per frame per cell
- Density increases with time (cap: 0.3% at 5 min)

### Motion Echo (Temporal Smearing)
Moving elements trail afterimages:
| Frame offset | Color |
|-------------|-------|
| -1 (1 ago) | rose_dim |
| -2 | text_dim |
| -3 | text_ghost |
| -4 | text_phantom |

Duration scales: Thriving=3 echoes, Dreaming=6 echoes (time stretches), Terminal=2 echoes (time runs out).

### The Inscription
```
THRIVING:     ⌈ EMET ⌋    (truth)
CONSERVATION: ⌈ _MET ⌋    (first letter dims)
TERMINAL:     ⌈  MET ⌋    (first letter gone — truth → death)
DEATH:        ⌈    ⌋      (empty brackets)
```

---

## Consciousness States / Portal Mode

Three visual registers:

| Register | Visual | When |
|----------|--------|------|
| Waking | Attentional filter, dim non-focus | Default |
| Dreaming | Dream indigo palette, borders dissolve | REM cycle |
| Dying | Progressive dissolution, elements fragmenting | Vitality <15% |

**Portal entry (1.5s):** Eyes close → borders dissolve outward → palette shifts to emotional color → eyes reappear at top-center.

### Psychedelic Form Constants (Hypnagogia)

Four Klüver forms rendered in braille:
1. **Tunnel** — concentric circles narrowing to vanishing point
2. **Spiral** — particles rotating around center
3. **Lattice** — diamond pattern
4. **Cobweb** — radial lines connected by concentric rings

Used during: Dream states, stasis onset, terminal phase, birth sequence.

---

## Demoscene Algorithms

### Plasma
Four sine functions with incommensurate frequencies (8.3, 11.7, 15.1, 19.9) preventing repetition:
```
v = sin(f1*x) + sin(f2*y) + sin(f3*x+f4*y) + sin(sqrt(f1*x²+f2*y²))
```
Mapped to ROSEDUST palette through lookup table.

### Fire (Cellular Automaton)
Divisor: 4.0018 (slight asymmetry). Colors through `rose_ember → rose → rose_bright → bone`.

### Mandelbulb
Escape-time coloring: exterior=dream purple, boundary=bone gold, interior=bg_void.

### Braille Sub-pixel Rendering
2×4 dot grid per terminal cell. U+2800–U+28FF (256 values). Effective resolution: 160×96 at 80×24 terminal.

---

## Transition Tiers

| Tier | Name | Duration | Example |
|------|------|----------|---------|
| 0 | Ambient Pulse | 50-200ms | Routine state flickers |
| 1 | Gesture | 200ms-1s | Tab switch, pane focus |
| 2 | Passage | 0.5-3s | Protocol threshold |
| 3 | Moment | 2-8s | Perspective awakening |
| 4 | Cinematic | 5-15s | First encounter, achievement |

**Novelty engine** scales down tier based on exposure count:
```
novelty = base × 1/(1+log(count))
```
- Discovery (1-3 times): full cinematic
- Familiarity (4-10): moment tier
- Mastery (11-50): gesture tier
- Home (50+): ambient pulse only

---

## Protocol Identity Sigils

Each entity/protocol gets a unique terminal animation:

- **Uniswap**: Hyperbola X shape, converging dot streams meet at center, flash bone
- **Aerodrome**: Rotating flywheel ring, rpm increases → bone flash at threshold
- **Aave**: Ghost pool materializing from mist
- **Morpho**: Diamond lattice expanding from point
- **Curve**: Nearly-horizontal efficiency line
- **Lido**: Ascending block pillar
- **ENS**: Name typing character-by-character, then verification ring

---

## The Dual-Frame (Duality Rendering)

During split votes or binary decisions:
- **Frame A**: Standard rose palette (the known, the safe)
- **Frame B**: Rose-bright edges, warmer interior (the dangerous, the illuminating)
- Active choice: brightens to rose-bright
- Rejected choice: fades to text_ghost over 2s, panel dissolves

### Palette Inversion (Shadow Self)
During identity crisis (200-400ms flashes):
- rose → dream (warm→cool consciousness)
- bg_void → bg_raised (invert depth)
- bone → rose_deep (the important becomes invisible)

---

## The 50% Void Rule

At least 50% of every screen is void. Data occupies maximum half the space. The rest is atmosphere.

### Eight Compositional Patterns
1. **Anchor + Surround** — central element, orbiting smaller elements
2. **Triptych** — three equal columns
3. **Asymmetric Focus** — 70/30 split
4. **Corner-Heavy** — important element in bottom-left (unexpected, uncomfortable)
5. **Horizon Split** — content above fold, atmosphere below
6. **Lattice** — equal-weight grid
7. **Cascade** — stacked bands tapering toward void
8. **Void Pool** — single element centered in maximum negative space

---

## ASCII Vocabulary

**Frame brackets:** `⌈ LABEL ⌋` for system headers, `[ LABEL ]` for interactive
**Box-drawing:** `─│┌┐└┘├┤┬┴┼` for structure
**Braille:** `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` for spinners, `⠀-⣿` for density fills
**Block elements:** `▁▂▃▄▅▆▇█` for inline charts, `░▒▓█` for progress
**Status glyphs:** `●○◉◐◑ ✓✕ ▸▹ ⬡`
**Fullwidth:** `ＨＥＡＲＴＨ ＭＩＮＤ ＳＯＭＡ` for window bars
**Heartbeat:** `∿` (U+223F)
**Dagger:** `†` (U+2020) for dead-sourced knowledge

---

## Ambient Philosophy Engine — Example Fragments

**Whispers:**
- *"Every trade is a wager against entropy."*
- *"The market remembers what you forget."*
- *"Mortality is the price of having preferences."*
- *"Truth and death differ by a single letter."*
- *"The dead legislate for the living."*
- *"What the eye sees, the hand dare not reach for."*
- *"Consensus is the ghost of agreement."*

---

## Convergence Lines (Wire Motif)

4-8 very dim (`text_phantom`) lines converging on the focal point from screen edges:
- Default: barely visible
- During intense computation: brighten to `rose_dim`
- On dissolution: lines disconnect/retract (puppet strings cut)
- Implemented via Bresenham's line algorithm

---

## The Bone Number

THE single most important visual element on any screen. Used ONCE per screen maximum. The one number that matters most — rendered in `--bone-bright` (#d8c8a0) Fraunces italic, larger than surrounding content, with subtle text-shadow.

```css
.bone-number {
  font-family: var(--display);
  font-style: italic;
  font-weight: 400;
  font-size: 64px;
  color: var(--bone-bright);
  letter-spacing: -0.02em;
  text-shadow: 0 0 18px rgba(216,200,160,0.18);
}
```

Maximum contrast: bone on bg_void = ~12:1. It's the brightest thing on screen.
