# 25.04 -- Spectre Creature Visualization

> Depth file for [25-TUI.md](../../25-TUI.md) section 7.

---

## Design Principles

1. **Glanceable, not decorative**: every visual element encodes data
2. **Deterministic identity**: same agent always produces same body shape
3. **Smooth state transitions**: PAD vector interpolated over ~500ms with luxury easing
4. **No mortality**: Spectres never die, decay to nothing, or display terminal states
5. **Collective coherence**: synchronized breathing indicates coordination quality

## Generation Pipeline

```
Agent ID + Template Name
    |
    v
BLAKE3 hash -> 32-byte shape seed
    |
    v
Morphological parameter extraction (body archetype, symmetry, limbs, eyes, texture)
    |
    v
Dot-cloud geometry generation (L-system + SDF composition)
    |
    v
Spring connection assignment (Verlet integration)
    |
    v
Initial state: Resting
```

## Morphological Parameters from Shape Seed

| Seed Bytes | Parameter | Range |
|---|---|---|
| [0..4] | body_archetype | 0-7 (8 types) |
| [4..8] | symmetry | bilateral / radial / asymmetric |
| [8..10] | limb_count | 0-6 |
| [10..12] | limb_style | tentacle / fin / spike / tendril |
| [12..14] | eye_count | 1-4 |
| [14..16] | eye_style | round / slit / compound / star |
| [16..20] | domain_texture | geometric / organic / crystalline / fluid |
| [20..24] | color_offset | 0.0-1.0 (hue shift within ROSEDUST) |
| [24..28] | proportion_ratios | body-to-limb, head-to-body |
| [28..32] | detail_seed | minor variation details |

## Dot-Cloud Geometry

Spectres are represented as weighted 3D points connected by springs:

```rust
pub struct SpectrePoint {
    pub position: [f32; 3],   // normalized [-1, 1]^3
    pub weight: f32,          // affects rendering density
    pub kind: PointKind,      // Body, Limb, Eye, Tendril, Particle
    pub color: Option<[u8; 3]>,
}

pub struct SpectreCloud {
    pub body_points: Vec<SpectrePoint>,     // static from seed
    pub animated_points: Vec<SpectrePoint>, // dynamic from Daimon
    pub springs: Vec<Spring>,
    pub bounds: BoundingBox,
}
```

## Spring Physics

Verlet integration provides organic movement:
- Time-reversible, symplectic, energy-preserving, O(h^4) error
- Spring parameters vary by behavioral state
- Constraint satisfaction after each position update

## Procedural Generation Techniques

- **L-Systems**: stochastic parallel rewriting grammars for body topology
- **SDF composition**: Signed Distance Fields with smooth union (Quilez 2014) for body shapes
- **Gray-Scott reaction-diffusion**: deterministic surface textures (Pearson classification)
- **fBm noise**: fractional Brownian Motion for organic detail variation
- **Procedural iris generation**: radially symmetric fiber patterns with state-linked pupil dilation

## Knowledge Encoding

Body density increases with knowledge accumulation:

| Knowledge Level | Body Points | Visual Effect |
|---|---|---|
| 0-50 entries | Base density | Simple outline |
| 50-150 | 1.5x density | Texture appears |
| 150-300 | 2x density | Rich patterns |
| 300+ | 2.5x density | Dense, "experienced" appearance |

## Phase-Space Portrait

Behavioral trajectory plotted as (Energy, Arousal) reveals attractor basins:
fixed points (stable states), limit cycles (work-rest oscillations), and
separatrices (basin boundaries). Rendered using braille canvas in compact viewports.
