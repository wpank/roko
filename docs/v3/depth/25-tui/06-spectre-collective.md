# 25.06 -- Spectre as Collective Display

> Depth file for [25-TUI.md](../../25-TUI.md) section 9.

---

## Collective Layout

### TUI Gallery

Grid layout with Spectre cells connected by filament characters. Each cell shows
the agent's Spectre body, name, state, and connections to adjacent cells.

### Web Portal

True force-directed 3D layout with four forces:
1. **Repulsion**: prevent overlap
2. **Attraction**: mesh edges pull connected agents together
3. **Center gravity**: all Spectres gravitate toward viewport center
4. **Pheromone attraction**: active channels create additional pull

### Spectral Layout (large collectives)

For >8 agents, spectral layout uses eigendecomposition of the graph Laplacian
L = D - A. Eigenvectors 1 and 2 assign positions that reveal community structure.

## Filament Types

| Type | TUI | Color | Animation |
|---|---|---|---|
| Mesh peer | `──────` | Muted rose | Static pulse |
| Active data flow | `══════` | Rose | Directional particles |
| Pheromone channel | `≋≋≋≋≋≋` | Type color | Wave animation |
| Knowledge transfer | `──·──•──◉──` | Gold | Growing dots |
| Stigmergy trace | `- - - -` | Dim | Gradual fade |

## Breathing Synchronization (Kuramoto Model)

Phase coupling for agents sharing a behavioral state:

```
dtheta_i/dt = omega_i + (K/N) * sum(sin(theta_j - theta_i))
```

The order parameter r measures collective synchronization:

```rust
pub fn kuramoto_order_parameter(phases: &[f32]) -> (f32, f32) {
    let n = phases.len() as f32;
    let sum_cos: f32 = phases.iter().map(|&t| t.cos()).sum();
    let sum_sin: f32 = phases.iter().map(|&t| t.sin()).sum();
    let r = ((sum_cos / n).powi(2) + (sum_sin / n).powi(2)).sqrt();
    let psi = (sum_sin / n).atan2(sum_cos / n);
    (r, psi) // r in [0,1]: 0=incoherent, 1=synchronized
}
```

Coupling strength K maps from C-Factor:
- C < 0.8: K=0.0 (independent)
- C 0.8-1.0: K=0.1 (slight drift)
- C 1.0-1.2: K=0.3 (noticeable)
- C 1.2-1.5: K=0.6 (strong)
- C > 1.5: K=0.9 (near-perfect collective pulse)

## Pheromone Fields

Pheromone fields modeled as scalar field with exponential evaporation:
`d_phi/dt = -lambda * phi + sum(delta(x - x_i) * s_i)`

Fields visually decay: dense particles (full) to sparse (half) to rare (low)
to invisible (expired). Warning pheromones override other types visually.

## Emergent Visualization

### Boid Flocking (WebGL)

Reynolds (1986) flocking rules applied per-Spectre: separation, alignment,
cohesion. Agents with similar roles flock together. Broadcast reception causes
visual coalescence; task divergence fragments the flock.

### Information Cascade

Knowledge propagation shows as animated wave front: seed agent glows brighter,
first-wave recipients receive growing dots along filaments, second-wave shows
delayed dimmer dots. Reveals information hub topology.

## Performance Scaling

| Agents | Filaments (max) | TUI Frame Budget |
|---|---|---|
| 2 | 1 | ~2ms |
| 4 | 6 | ~4ms |
| 8 | 28 | ~8ms |
| 16 | 120 | ~14ms (near budget) |

For >8 agents: show only active data flow filaments, collapse pheromone fields
to single particles, render Spectres in braille mode.
