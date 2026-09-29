# 25.07 -- Sonification

> Depth file for [25-TUI.md](../../25-TUI.md) section 11.

---

## The Eno Mandate

Brian Eno's ambient music principles (*Music for Airports*, 1978):

1. **Ignorable and interesting**: reward attention without demanding it
2. **Generative, not composed**: produced by systems, not pre-recorded
3. **Infinite duration**: never loops, never repeats, always evolves
4. **Environmental**: part of the environment, not a performance
5. **Silence is an instrument**: pauses are compositional elements

## Five Musical Layers

### Layer 1: Drone

Foundational harmonic bed. Root note from project hash (C2-C3). Volume tracks
C-Factor. Texture scales with agent count (sine for 1 agent, rich pad for 8+).

| State | Character |
|---|---|
| Engaged | Warm, full, major-mode |
| Struggling | Tense, minor seconds, detuned |
| Coasting | Thin, sustained, minimal |
| Exploring | Wide, shifting modes, chromatic |
| Focused | Pure, narrow, single harmonic series |
| Resting | Near-silent, sub-bass only |

### Layer 2: Breath

Rhythmic pulse tracking average Spectre breathing rate (0.2-1.4Hz). Phase-locks
when C-Factor > 1.2.

### Layer 3: Ghost

Melodic fragments for individual events. Per-agent timbre mapping.
Reverb depth tracks knowledge tier (dry=transient, wet=persistent).
Density: 0-8 events/minute.

### Layer 4: Weather

Granular texture encoding collective state. Color tracks C-Factor (warm=high,
cold=low). Grain size tracks knowledge flow rate. Spatialized by mesh position.

### Layer 5: Sparks

Transient high-frequency events (clicks, pops) for tool calls. Rate tracks
tool call frequency (0-20/sec). Stereo panned to agent mesh position.

## Harmonic Vocabulary

Scale selection by behavioral state:

| State | Scale | Emotional Analog |
|---|---|---|
| Engaged | Major (Ionian) | Joy, flow |
| Struggling | Phrygian / Locrian | Anxiety, effort |
| Coasting | Mixolydian | Contentment, ease |
| Exploring | Lydian / Whole-tone | Curiosity, wonder |
| Focused | Dorian | Determination |
| Resting | Aeolian (natural minor) | Calm, contemplation |

## Event Sound Mapping

| Event | Sound | Interval |
|---|---|---|
| Gate pass | Rising 3rd or 5th | Consonant |
| Gate fail | Descending minor 2nd | Dissonant |
| Knowledge created | Bell tone | Clear |
| Knowledge promoted | Rising arpeggio | Ascending |
| Task completed | V->I cadence | Resolved |
| Dreams consolidation | Major 7th | Ethereal |

## Eight Presets

1. `ambient_default` -- balanced across all layers
2. `minimal_drone` -- drone + breath only
3. `granular_texture` -- Weather-emphasized
4. `engaged_flow` -- warm, motivating, Engaged-optimized
5. `struggling_tension` -- attention-drawing, Struggling-optimized
6. `deep_dream` -- ethereal, Resting/Dreams
7. `exploring_curiosity` -- open, chromatic, Exploring
8. `emergence` -- auto-triggered at C-Factor > 1.5, harmony > 0.8

## Silence Rules

| Condition | Silence Level |
|---|---|
| All agents Resting | Drone at -30dB only |
| No active plans | Full silence |
| Between state transitions | 0.5-1s pause |

## Implementation

Audio engine targets: Web Audio API (portal), CPAL/rodio (native CLI, optional),
MIDI output (external synthesizers). State ingestion from WebSocket event stream.
