# 25.05 -- Spectre Rendering Per Interface

> Depth file for [25-TUI.md](../../25-TUI.md) section 8.

---

## Shared Data Model

All renderers consume the same `SpectreState` JSON:

```json
{
  "agent_id": "rust-impl-01",
  "behavioral_state": "Engaged",
  "pad": {"pleasure": 0.7, "arousal": 0.5, "dominance": 0.8},
  "body": {
    "shape_seed": "a3f2b1c4...",
    "archetype": "Teardrop",
    "symmetry": "bilateral",
    "knowledge_density": 1.5
  },
  "animation": {
    "breathing_rate": 0.7,
    "eye_state": "open",
    "glow_color": "#D4778C",
    "glow_intensity": 0.8,
    "spring_stiffness": 0.8,
    "spring_damping": 0.6
  },
  "knowledge": { "persistent": 23, "working": 89, "transient": 30 },
  "mesh_connections": ["reviewer-01"],
  "pheromone_emission": {"type": "Wisdom", "intensity": 0.4}
}
```

## Renderer 1: TUI ASCII

Rasterization pipeline:
1. Project 3D points to 2D (orthographic, front view)
2. Quantize to character grid
3. Assign characters by density and kind
4. Apply ROSEDUST color + glow composite
5. Render as ratatui Cells

Frame budget: ~1.5ms within 16.6ms total (60fps).

Braille mode for compact viewports: 2x4 sub-pixels per character cell.

## Renderer 2: Web Portal WebGL

Three.js scene graph with instanced point cloud, custom shader pipeline
(body SSS, bloom post-process, emissive eyes), and Kawase blur bloom.
Interactive: orbit camera, zoom, hover tooltips, click-to-focus.

## Renderer 3: CLI Inline

Single-line: `rust-impl-01: ◉ Engaged [▓▓▓▓▓░░] P:0.7 A:0.5 D:0.8`
Mini Spectre (5x3): body + eyes + state color in 3 lines.

## Renderer 4: API JSON

Raw state via `/ws/spectre/:id` (30Hz during active work, 1Hz during Resting)
and `/api/agents/:id/spectre`. Minimal response available via `?minimal=true`.

## Renderer 5: AR/VR Spatial

WebXR/visionOS with proximity-based LOD:
- <1m: ray-marched SDF with SSS
- 1-3m: polygon mesh with status text
- 3-8m: billboard sprite
- 8m+: colored point

HRTF spatial audio, hand gesture interaction.

## Consistency Guarantees

All renderers share:
- Same `state_color()` mapping (behavioral state to ROSEDUST color)
- Same morphological parameter tables
- Same breathing rate, phase, and asymmetry values
