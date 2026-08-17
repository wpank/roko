# Spectre as Collective Display

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> How multiple Spectre creatures compose into a collective visualization — mesh topology, filament connections, pheromone fields, breathing synchronization, and C-Factor harmony encoding.


> **Implementation**: Scaffold

**Topic**: [12-interfaces](./INDEX.md)
**Prerequisites**: [10-spectre-creature-visualization.md](./10-spectre-creature-visualization.md), [11-spectre-rendering-per-interface.md](./11-spectre-rendering-per-interface.md)
**Key sources**: `refactoring-prd/06-interfaces.md` §5, `bardo-backup/prd/18-interfaces/28-creature-system.md`

---

## Abstract

Individual Spectre creatures encode single-agent cognitive state. When viewed together, they form a **collective display** that encodes the multi-agent system's emergent properties: mesh connectivity, pheromone flow, knowledge transfer, coordination quality, and overall collective intelligence (C-Factor).

The collective display is not a separate visualization system — it is what naturally emerges when multiple Spectres are rendered in spatial proximity with their inter-agent connections visible. The rendering reveals properties that are invisible when viewing agents in isolation: synchronization patterns, communication flow, role distribution balance, and collective stress or harmony.

This document specifies how collective properties are encoded visually, building on the individual Spectre specification in [10-spectre-creature-visualization.md](./10-spectre-creature-visualization.md).

---

## Collective Layout

### Spatial Arrangement

When multiple Spectres are displayed together (Spectre Gallery — Screen 6.4, C-Factor Dashboard — Screen 5.1, or Web Portal collective view), they are arranged in a force-directed layout:

```
Layout forces:
  1. Repulsion:  Spectres push apart (prevent overlap)
  2. Attraction: Connected Spectres pull toward each other (mesh edges)
  3. Center:     All Spectres gravitate toward viewport center
  4. Pheromone:  Active pheromone channels create additional attraction
```

The result is a spatial arrangement where:
- Tightly connected agents cluster together
- Isolated agents drift to the periphery
- Pheromone-active pairs are visually proximate
- The overall layout reflects the mesh topology

### TUI Gallery Layout

In the terminal, the gallery uses a grid layout (since force-directed layout requires continuous positioning):

```
┌─ SPECTRE GALLERY ──────────────────────────────┐
│                                                  │
│  ┌──────────┐──────┌──────────┐                 │
│  │  ╭─╮     │      │   ╭╮    │                 │
│  │ ╭╯ ╰╮    │≋≋≋≋≋≋│  ╭╯╰╮   │                 │
│  │ │◉ ◉│    │      │  │◉◉│   │                 │
│  │ ╰───╯    │      │  ╰──╯   │                 │
│  │rust-impl │      │reviewer │                 │
│  │ Engaged  │      │ Focused │                 │
│  └──────────┘      └──────────┘                 │
│       │                  │                      │
│       │    ≋≋≋≋≋≋≋≋      │                      │
│       │                  │                      │
│  ┌──────────┐      ┌──────────┐                 │
│  │    ╭╮    │      │  ╭───╮   │                 │
│  │ ≋╭╯╰╮≋  │      │  │   │   │                 │
│  │  │◉ ◉│   │      │  │○ ○│   │                 │
│  │  ╰───╯   │      │  ╰───╯   │                 │
│  │researcher│      │architect │                 │
│  │Exploring │      │ Resting  │                 │
│  └──────────┘      └──────────┘                 │
│                                                  │
│  Connections: ≋≋≋ = active mesh link            │
│  C-Factor: 1.23 │ Harmony: 0.78                │
└──────────────────────────────────────────────────┘
```

### Web Portal Layout

The Web Portal uses a true force-directed 3D layout where Spectres float in a shared space with visible connecting filaments. See [13-web-portal.md](./13-web-portal.md) for the WebGL implementation.

---

## Filament Connections

Filaments are the visual representation of mesh connections between agents. They extend from one Spectre's body to another, encoding the nature and strength of the connection.

### Filament Types

| Connection Type | Visual | Color | Animation |
|---|---|---|---|
| **Mesh peer** | Thin line | Muted rose | Static, subtle pulse |
| **Active data flow** | Thick line with flow particles | Rose | Particles flow in data direction |
| **Pheromone channel** | Wavy line (≋) | Pheromone type color | Wave animation toward target |
| **Knowledge transfer** | Dotted line with growing dots | Gold | Dots grow as they travel |
| **Stigmergy trace** | Dashed line | Dim, fading | Gradually fades after event |

### TUI Filament Rendering

In the terminal, filaments are rendered as character sequences between Spectre cells:

```
Horizontal connection:  ──────  (thin)    ══════  (thick)    ≋≋≋≋≋≋  (pheromone)
Vertical connection:    │ (thin)           ║ (thick)          ≋ (pheromone)
                        │                  ║                  ≋
                        │                  ║                  ≋
Diagonal connection:    ╲ or ╱ (thin)      ╲ or ╱ (thick)
```

**Flow direction** is indicated by particle characters moving along the filament:

```
Data flow left→right:  ───·──•──◉──────  (particles grow as they approach target)
Pheromone emission:    ≋≋≋≋≋≋→           (wave characters animate rightward)
```

### WebGL Filament Rendering

In the Web Portal, filaments are rendered as 3D tubes with:
- **Radius** proportional to connection strength
- **Color** from the connection type table
- **Flow particles** as GPU-instanced point sprites moving along the tube
- **Glow** matching the ROSEDUST bloom pipeline
- **Animation** driven by `requestAnimationFrame`

---

## Pheromone Fields

Pheromone emissions create visible fields around the emitting Spectre that influence nearby Spectres.

### Field Visualization

A pheromone field is rendered as a gradient of particles or color surrounding the emitting Spectre:

```
TUI rendering of a Wisdom pheromone field:

            ✦
        ✦       ✦
    ✦      ╭─╮      ✦
        ╭──╯ ╰──╮
  ✦     │ ◉  ◉  │     ✦       ← particles float outward
        ╰───────╯
    ✦               ✦
        ✦       ✦
            ✦
```

### Pheromone Type Colors

| Pheromone Type | Color | Particle Character | Field Character |
|---|---|---|---|
| **Wisdom** | Gold (`#D4A857`) | `✦` | `·` background tint |
| **Warning** | Danger (`#C45C50`) | `⚡` | `!` background tint |
| **Discovery** | Violet (`#A08CC4`) | `◊` | `*` background tint |
| **Recruitment** | Rose (`#D4778C`) | `→` | `·` directional |
| **Completion** | Jade (`#5DB8A3`) | `✓` | `·` background tint |

### Field Decay

Pheromone fields visually decay over time:
- **Full intensity**: Dense particle cloud, bright color
- **Half intensity**: Sparse particles, muted color
- **Low intensity**: Rare particles, nearly invisible
- **Expired**: No visual effect

The decay rate matches the pheromone's configured half-life in the Stigmergy system.

### Field Overlap

When multiple pheromone fields overlap (common in dense agent clusters), the colors blend:
- Same type: intensities add (brighter)
- Different types: colors blend using the `gradient()` function from `roko-cli/src/tui/color.rs`
- Warning overrides other types visually (danger always visible)

---

## Breathing Synchronization

### Coupled Oscillator Model

When multiple agents share a behavioral state, their Spectre breathing rates gradually synchronize through a Kuramoto-inspired phase-coupling mechanism:

```
dθ_i/dt = ω_i + (K/N) × Σ sin(θ_j - θ_i)

Where:
  θ_i  = breathing phase of agent i
  ω_i  = natural breathing rate (from behavioral state)
  K    = coupling strength (proportional to C-Factor)
  N    = number of agents in the same state
```

**Coupling strength mapping:**

| C-Factor Range | Coupling K | Visual Effect |
|---|---|---|
| < 0.8 | 0.0 | No synchronization (independent breathing) |
| 0.8–1.0 | 0.1 | Slight drift toward sync |
| 1.0–1.2 | 0.3 | Noticeable synchronization |
| 1.2–1.5 | 0.6 | Strong synchronization |
| > 1.5 | 0.9 | Near-perfect synchronization (collective pulse) |

### Visual Impact

When breathing synchronizes, the collective display creates a visual "pulse" — all Spectres in the same state expand and contract together. This is immediately noticeable in the Spectre Gallery and provides an intuitive readout of collective coordination.

```
Synchronized (C-Factor > 1.2):        Desynchronized (C-Factor < 0.8):

Frame 1:  ╭─╮  ╭─╮  ╭─╮              Frame 1:  ╭─╮   ╭╮   ╭──╮
         ╭╯ ╰╮╭╯ ╰╮╭╯ ╰╮                      ╭╯ ╰╮ ╭╯╰╮ ╭╯  ╰╮
         │◉ ◉││◉ ◉││◉ ◉│                       │◉ ◉│ │◉◉│ │◉  ◉│
         ╰───╯╰───╯╰───╯                       ╰───╯ ╰──╯ ╰────╯

Frame 2: ╭──╮ ╭──╮ ╭──╮              Frame 2:  ╭╮   ╭──╮  ╭─╮
        ╭╯  ╰╮╭╯  ╰╮╭╯  ╰╮                    ╭╯╰╮ ╭╯  ╰╮╭╯ ╰╮
        │◉  ◉││◉  ◉││◉  ◉│                     │◉◉│ │◉  ◉││◉ ◉│
        ╰────╯╰────╯╰────╯                     ╰──╯ ╰────╯╰───╯
(all expand together)                  (each on own rhythm)
```

---

## C-Factor Harmony Encoding

The collective display encodes the C-Factor through multiple visual channels:

### Harmony Score

A derived metric that combines several visual indicators:

```
Harmony = w1 × breathing_sync + w2 × state_diversity + w3 × connection_density + w4 × pheromone_activity

Where:
  breathing_sync     = phase coherence of synchronized agents (0–1)
  state_diversity    = entropy of behavioral state distribution (0–1)
  connection_density = fraction of possible mesh connections that are active (0–1)
  pheromone_activity = normalized pheromone emission rate (0–1)
```

### Visual Encoding

| C-Factor | Harmony | Visual Character |
|---|---|---|
| < 0.8 | Low | Independent breathing, sparse connections, muted colors, no ambient particles |
| 0.8–1.0 | Moderate | Some synchronization, visible connections, subtle ambient glow |
| 1.0–1.2 | Good | Noticeable sync, active connections with flow particles, warm ambient glow |
| 1.2–1.5 | High | Strong sync, dense connections, rich pheromone fields, ambient sparkles |
| > 1.5 | Exceptional | Near-perfect sync, all connections glowing, collective breathing pulse, particle harmony |

### Ambient Background

The collective display's background itself encodes harmony:

- **Low harmony**: Pure void-black (`#1A1520`), no ambient effects
- **Moderate harmony**: Faint rose tint in the background
- **High harmony**: Subtle pulsing ambient glow matching the collective breathing rhythm
- **Exceptional harmony**: Rich ambient glow with floating particles between Spectres

---

## Role Distribution Visualization

### State Balance Indicator

A healthy collective shows diverse behavioral states. The collective display includes a state distribution bar:

```
State Distribution:
E:2 ████  F:1 ██  X:1 ██  R:1 ██    Balance: 0.85 (good)

vs.

S:4 ████████  E:1 ██                  Balance: 0.32 (alarm!)
```

### Visual Alarm

When the collective is unbalanced (e.g., all agents Struggling), the collective display responds:

- **All Struggling**: Background flashes amber, collective breathing becomes rapid and desynchronized
- **All Coasting**: Background dims, collective breathing slows to near-still
- **All Resting**: Minimal display, faint dim glow, very slow collective drift
- **Healthy mix**: Normal display with harmony indicators

---

## Knowledge Flow Visualization

### Inter-Agent Knowledge Transfer

When knowledge is transferred between agents (via the Neuro cross-cut), the collective display shows:

```
rust-impl ────·──•──◉──▸ reviewer
              (knowledge dot growing as it transfers)
```

The dot grows from `·` to `•` to `◉` as the knowledge engram moves from the source agent's Neuro store to the target's.

### Collective Knowledge Heat Map

The background can overlay a heat map showing knowledge density by domain:

```
High knowledge density regions:
▓▓▓▓ auth (45 entries)
▒▒▒▒ testing (30 entries)
░░░░ config (15 entries)
     deploy (5 entries)

Visualized as background tinting in the collective display,
with brighter regions indicating more accumulated knowledge.
```

---

## Implementation Architecture

### Collective State Aggregation

The collective display consumes individual Spectre states and aggregates them:

```rust
/// Aggregated collective visualization state.
pub struct CollectiveDisplay {
    /// Individual Spectre states
    pub spectres: Vec<SpectreState>,
    /// Mesh topology (adjacency list)
    pub mesh: Vec<(usize, usize, ConnectionType)>,
    /// Active pheromone fields
    pub pheromones: Vec<PheromoneField>,
    /// C-Factor and harmony metrics
    pub cfactor: f64,
    pub harmony: f64,
    /// Breathing synchronization state
    pub sync_phases: Vec<f32>,
    /// State distribution
    pub state_counts: HashMap<BehavioralState, usize>,
}
```

### Update Flow

```
Individual Spectre States (from /ws/spectre/:id per agent)
    │
    ▼
CollectiveDisplay::update()
    │
    ├── Update spatial layout (force-directed)
    ├── Update filament states (from mesh topology)
    ├── Update pheromone fields (from stigmergy events)
    ├── Compute breathing synchronization (Kuramoto step)
    ├── Compute harmony score
    └── Compute state distribution
    │
    ▼
Render (TUI Gallery or Web Portal Collective View)
```

### Performance Considerations

The collective display scales with agent count:

| Agents | Filaments (max) | Pheromone Fields (max) | TUI Frame Budget |
|---|---|---|---|
| 2 | 1 | 2 | ~2ms |
| 4 | 6 | 4 | ~4ms |
| 8 | 28 | 8 | ~8ms |
| 16 | 120 | 16 | ~14ms (near budget) |

For > 8 agents, the TUI reduces rendering detail:
- Filaments shown only for active data flow (not all mesh connections)
- Pheromone fields collapsed to single particles
- Spectre bodies rendered in braille mode (compact)

---

## Emergent Visualization — Swarm Intelligence Patterns

### Boid Flocking for Agent Collectives

When multiple agents share a behavioral state, their Spectres can exhibit emergent flocking behavior (Reynolds, 1986) in the WebGL collective view. The three flocking rules applied per-Spectre per-frame:

```rust
/// Boid-style flocking for collective Spectre layout.
pub struct FlockingParams {
    /// Separation: avoid crowding neighbors (radius, strength)
    pub separation: (f32, f32),  // default: (2.0, 1.5)
    /// Alignment: match average velocity of neighbors (radius, strength)
    pub alignment: (f32, f32),   // default: (5.0, 1.0)
    /// Cohesion: steer toward average position of neighbors (radius, strength)
    pub cohesion: (f32, f32),    // default: (8.0, 0.5)
    /// Maximum speed
    pub max_speed: f32,          // default: 2.0
}

/// Per-frame boid update for a single Spectre.
pub fn boid_update(
    idx: usize,
    spectres: &[SpectrePosition],
    params: &FlockingParams,
) -> [f32; 2] {
    let pos = spectres[idx].pos;
    let vel = spectres[idx].vel;
    let mut sep = [0.0f32; 2];
    let mut aln = [0.0f32; 2];
    let mut coh = [0.0f32; 2];
    let mut n_sep = 0usize;
    let mut n_aln = 0usize;

    for (j, other) in spectres.iter().enumerate() {
        if j == idx { continue; }
        let d = dist(pos, other.pos);
        if d < params.separation.0 {
            sep[0] -= (other.pos[0] - pos[0]) / d;
            sep[1] -= (other.pos[1] - pos[1]) / d;
            n_sep += 1;
        }
        if d < params.alignment.0 {
            aln[0] += other.vel[0];
            aln[1] += other.vel[1];
            n_aln += 1;
        }
        if d < params.cohesion.0 {
            coh[0] += other.pos[0];
            coh[1] += other.pos[1];
        }
    }

    let w = params;
    [
        vel[0] + sep[0] * w.separation.1 + aln[0] * w.alignment.1 + coh[0] * w.cohesion.1,
        vel[1] + sep[1] * w.separation.1 + aln[1] * w.alignment.1 + coh[1] * w.cohesion.1,
    ]
}
```

**Application:** Agents with similar roles/states flock together. When the orchestrator issues a broadcast, the receiving agents' Spectres visually coalesce. When agents diverge in task assignments, the flock fragments into sub-flocks. The visual topology of the flock IS the coordination topology.

### Stigmergy — Pheromone Trail Persistence

Pheromone fields are modeled as a scalar field `φ(x, y, t)` with exponential evaporation:

```
∂φ/∂t = −λ·φ + Σᵢ δ(x − xᵢ(t))·sᵢ
```

Where `λ` is the evaporation rate, `δ` is a Dirac delta at agent position `xᵢ`, and `sᵢ` is the signal strength deposited by agent `i`.

In the TUI gallery, pheromone trails appear as fading character sequences between Spectres — frequently-used communication routes glow brighter (denser `≋` characters), while rarely-used paths fade (sparse `·` characters). This creates a visual "wear pattern" of agent coordination over time, analogous to ant trails worn into grass.

### Kuramoto Phase Synchronization — Extended Model

The breathing synchronization uses the Kuramoto model with the **order parameter** `r` as a real-time readout of collective coherence:

```rust
/// Kuramoto order parameter — measures collective synchronization.
/// r ∈ [0, 1]: 0 = fully incoherent, 1 = fully synchronized.
pub fn kuramoto_order_parameter(phases: &[f32]) -> (f32, f32) {
    let n = phases.len() as f32;
    let sum_cos: f32 = phases.iter().map(|&θ| θ.cos()).sum();
    let sum_sin: f32 = phases.iter().map(|&θ| θ.sin()).sum();
    let r = ((sum_cos / n).powi(2) + (sum_sin / n).powi(2)).sqrt();
    let psi = (sum_sin / n).atan2(sum_cos / n); // mean phase
    (r, psi)
}

/// Update phase for agent i under Kuramoto coupling.
pub fn kuramoto_step(
    phases: &mut [f32],
    frequencies: &[f32],
    coupling: f32,
    dt: f32,
) {
    let n = phases.len() as f32;
    let old_phases = phases.to_vec();
    for i in 0..phases.len() {
        let coupling_sum: f32 = old_phases.iter()
            .map(|&θj| (θj - old_phases[i]).sin())
            .sum();
        phases[i] += (frequencies[i] + coupling / n * coupling_sum) * dt;
        phases[i] %= std::f32::consts::TAU;
    }
}
```

**Critical threshold:** Synchronization emerges when `K > Kc = 2 / (π · g(0))`, where `g(0)` is the frequency distribution density at the mean natural frequency. For agents with similar behavioral states (similar breathing rates), synchronization emerges at lower coupling strengths — visually confirming that coordinated agents "breathe together."

### Network Topology Layout — Spectral Alternative

For large agent collectives (>8 agents), the TUI gallery can use spectral layout instead of grid layout. Spectral layout uses eigendecomposition of the graph Laplacian `L = D − A` to assign positions that reveal community structure:

```rust
/// Spectral layout for agent mesh topology.
/// Uses the two smallest non-trivial eigenvectors of the graph Laplacian.
pub fn spectral_layout(adjacency: &[Vec<bool>], n: usize) -> Vec<[f32; 2]> {
    // L = D - A (Laplacian matrix)
    // Compute eigenvalues/eigenvectors of L
    // Use eigenvectors 1 and 2 (skip trivial 0th) as (x, y) positions
    // Tightly connected agents cluster; isolated agents drift to periphery
    let laplacian = compute_laplacian(adjacency, n);
    let (_eigenvalues, eigenvectors) = eigendecompose(&laplacian, 3); // 3 smallest
    (0..n).map(|i| [eigenvectors[1][i], eigenvectors[2][i]]).collect()
}
```

### Information Cascade Visualization

When a knowledge entry propagates from one agent to others via the Neuro cross-cut, the collective display shows the cascade as an animated "wave front":

1. **Seed agent** (source) glows brighter momentarily
2. **First-wave recipients** receive growing dots `· → • → ◉` along filaments
3. **Second-wave recipients** show delayed, dimmer dots
4. **Unaffected agents** show no change

The temporal cascade reveals the knowledge flow topology — which agents are information hubs vs. which are periphery.

---

## AR/VR Collective Visualization

### Spatial Computing for Agent Collectives

The collective display extends naturally to spatial computing environments (Apple Vision Pro, WebXR headsets):

**visionOS Volumetric Window:**
```swift
// Collective agent topology as a 3D volume
WindowGroup(id: "agent-collective") {
    RealityView { content in
        let graphEntity = try await buildAgentGraphEntity(spectres)
        content.add(graphEntity)
    }
}
.windowStyle(.volumetric)
.defaultSize(width: 1.5, height: 1.0, depth: 1.0, in: .meters)
```

**Spatial Interaction:**
- **Walk closer** to an agent cluster → progressive detail reveal (LOD)
- **Pinch gesture** on a Spectre → focus agent detail panel
- **Two-handed pull apart** → zoom into mesh topology
- **Spatial audio:** agent events emit HRTF-positioned sound from their 3D location

**Proximity-based detail levels:**

| Distance | Geometry | Data Shown |
|---|---|---|
| 0–0.5m | Full SDF Spectre with breathing | All telemetry, tool trace, last error |
| 0.5–2m | Simplified polygon mesh | Name, role, state, last 10 events |
| 2–5m | Billboard quad | Status color + icon |
| 5m+ | Instanced colored point | Cluster membership only |

### Shared Monitoring Rooms

Multiple operators can view the same collective display in a shared XR space:
- **Transport:** WebRTC data channels for low-latency state sync
- **Co-presence:** Operator avatars visible as hand/head representations
- **Spatial speech:** Collaborator voice panned to their physical position via Web Audio HRTF
- **Persistent anchors:** Azure Spatial Anchors or ARCore Cloud Anchors persist dashboard positions across sessions and devices

---

## Test Criteria

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kuramoto_fully_synchronized_gives_r_1() {
        let phases = vec![0.5, 0.5, 0.5, 0.5];
        let (r, _) = kuramoto_order_parameter(&phases);
        assert!((r - 1.0).abs() < 1e-5, "All same phase → r = 1.0");
    }

    #[test]
    fn kuramoto_uniformly_distributed_gives_r_near_0() {
        let n = 100;
        let phases: Vec<f32> = (0..n).map(|i| i as f32 * std::f32::consts::TAU / n as f32).collect();
        let (r, _) = kuramoto_order_parameter(&phases);
        assert!(r < 0.15, "Uniform distribution → r ≈ 0, got {}", r);
    }

    #[test]
    fn harmony_score_bounded() {
        let harmony = compute_harmony(0.8, 0.6, 0.4, 0.3);
        assert!(harmony >= 0.0 && harmony <= 1.0);
    }

    #[test]
    fn collective_display_handles_empty() {
        let display = CollectiveDisplay {
            spectres: vec![], mesh: vec![], pheromones: vec![],
            cfactor: 0.0, harmony: 0.0, sync_phases: vec![],
            state_counts: HashMap::new(),
        };
        // Should not panic
        let _ = display.update();
    }

    #[test]
    fn pheromone_field_decays() {
        let mut field = [[1.0f32; 32]; 32];
        let evaporation = 0.995;
        for _ in 0..100 {
            for row in field.iter_mut() {
                for cell in row.iter_mut() {
                    *cell *= evaporation;
                }
            }
        }
        assert!(field[16][16] < 0.7, "Field should decay after 100 steps");
    }
}
```

---

## Current Status and Gaps

**Built:**
- Spectre state JSON schema (WebSocket spec)
- ROSEDUST color mapping for all behavioral states
- Braille rendering widget for compact visualization
- C-Factor computation logic

**Not yet built:**
- Collective layout algorithm (force-directed or grid)
- Filament rendering (TUI or WebGL)
- Pheromone field visualization
- Breathing synchronization (Kuramoto coupling)
- Harmony score computation
- Collective state aggregation
- Spectre Gallery screen (Screen 6.4)
- AR/VR spatial collective display

---

## Cross-References

- See [10-spectre-creature-visualization.md](./10-spectre-creature-visualization.md) for individual Spectre specification
- See [11-spectre-rendering-per-interface.md](./11-spectre-rendering-per-interface.md) for per-renderer implementation
- See topic [07-cfactor](../14-identity-economy/INDEX.md) for C-Factor computation details
- See topic [08-mesh](../13-coordination/INDEX.md) for mesh topology and pheromone system
- See [09-tui-29-screens.md](./09-tui-29-screens.md) Screen 5.3 (Pheromone Landscape) and Screen 6.4 (Spectre Gallery)
