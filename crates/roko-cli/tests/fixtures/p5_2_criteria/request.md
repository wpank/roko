# Phase 5.2: the presets as named

**Goal**: the presets are what their names and the PRD say:
- SUB Pure Foundation is the clean sine sub alone;
- every SUB preset is the sub;
- no preset spends its energy below 20 Hz, where no one hears it;
- RUMBLE Sidechain Fog keeps the sidechain and the fog it is named for while it loses that energy.

**Depends on**: phase 5.1.

**Size**: S, 1 task, about 10 lines of preset values and one showcase render made again.

---

## Scope

In scope: six presets' values in `core/src/presets.cpp`:
- SUB Pure Foundation, SUB Low Pressure and SUB Kick Companion;
- BASS Acid Undertow, BASS Warehouse Drone and RUMBLE Sidechain Fog.

Also the four committed showcase renders in `artifacts/audio/`, of which only `sub-pure-foundation.wav` changes.

No new files, no API change, and no engine change: no high-pass on the output, too wide a change this late.

Out of scope: the other presets, the engine, the renderer and the editor.

## Context

audio-qa's QA of main 41072e1 passes every spec check, and found two things the checks let through. Each preset below is
rendered as P5.09 renders it: 2 bars at 128 BPM, holding A2 at velocity 100 when its sequencer is off.

- **F17: SUB Pure Foundation is not the clean sine its name and phase 5's table say** ("the clean sine sub alone",
  and PRD 1.1's "a clean sine foundation … a pure tone").
  - It runs the main oscillator's saw at full level two octaves down. That puts 27.5 Hz at -20.3 dBFS beside the
    sine's -18.7, with harmonics to 440 Hz.
  - Only 45.4% of its energy is at its 55 Hz sine (50 to 60 Hz), and 83.9% is below 80 Hz. P5.03's SUB rule (80%)
    lets it through.
  - It is a committed showcase: `artifacts/audio/sub-pure-foundation.wav`.
  - With the main oscillator off, 99.97% of its energy is at 55 Hz, and 27.5 Hz falls to -128.6 dB re the sine. The
    output saturation (`sat_drive` at its default, 0.2) still puts its harmonics at -35 dB.
  - With saturation off too (`sat_drive` 0), its harmonics are at -130 dB and it renders at -17.55 dBFS RMS
    (audio-qa's numbers).
- **The other two SUB presets carry the same saw, at 0.9.**
  - Their table entries name the sub and never a saw: "sub at -2 oct with a little saturation, a slow swell" and "a
    short, round sub note that sits after a kick". PRD 1.1 makes the sub the floor's clean sine.
  - So they are the sub too.
  - With the saw, 84.9% and 84.3% of their energy is below 80 Hz. Without it, 99.4% and 99.3%.
  - Without the saw, Low Pressure renders at -21.2 dBFS RMS, just under P5.09's floor of -21.
- **F18: energy below 20 Hz.** RUMBLE Sidechain Fog has 11.0% there, BASS Acid Undertow 5.5% and BASS Warehouse
  Drone 2.2%. The other nine sit at 0.06% to 1.03%.
  - Acid: its -12 steps play A0 (27.5 Hz), with the sub an octave under at 13.75 Hz. With the sub off: 1.2%.
  - Drone: its -5 and -2 steps put the sub at 20.6 and 24.5 Hz, peaking at 19.9 Hz. With the sub at 0.2: 1.3%.
  - Fog: its -12 steps play A0 with the saw an octave down, 13.75 Hz, and send it into a 5 s rumble that holds it
    (a peak at 11.3 Hz).
    - Its ghost kicks don't make it: with the kicks off it is 12.6%.
    - With its line an octave up (root A2, so its low steps play A1), the send at 0.2 and the rumble's tone at
      120 Hz: 1.7%.
  - The threshold is 2%. The nine other presets are at 1.03% and under, so 2% gives them twice that, and the three
    named are over it.
- **What a cheaper fix gives up.** Three changes that strip Sidechain Fog of what it is named for each pass P5.2.01 to
  P5.2.03, P5.03 and P5.09 on their own: the send at 0 (with its root at A3), the decay cut to 1 s (re-levelled), or
  the duck at 0. P5.2.04 pins what the preset is named for.
- **The spec test:** `tests/spec/p5_2/presets_test.cpp` (core preset).
  - It waits for `presets.h`, which main has, so `tools/check.sh` fails on it until this phase lands.
  - Change the code, never the tests.

## What to build

1. **The presets as named** (`core/src/presets.cpp` and the four `artifacts/audio/*.wav`)
   - **SUB Pure Foundation** is the clean sine sub alone: `osc_level` 0 and `sat_drive` 0. Holding A2:
     - at least 98% of its energy is at its 55 Hz fundamental (50 to 60 Hz);
     - nothing sounds two octaves down: 27.5 Hz at least 60 dB under the sine;
     - its harmonics 2 to 10 at least 40 dB under the sine. The output saturation adds them, so `sat_drive` stays at
       0.1 or less.
   - **SUB Low Pressure and SUB Kick Companion** are the sub: `osc_level` 0. Every SUB preset keeps at least 95% of
     its energy below 80 Hz.
   - **Level them for P5.09**, within 3 dB of -18 dBFS RMS. Without the saw, Low Pressure needs about 2 dB more
     `master_gain`: -15 gives -19.2.
   - **At most 2% of any preset's energy below 20 Hz.** Retune the presets that carry it, not the engine. Keep each
     preset's character: move its lowest notes up (`seq_root`), or lower its sub, its send or its rumble tone. A
     RUMBLE preset keeps its decay and its duck.
     - BASS Acid Undertow: no sub under its A0 steps (`sub_level` 0).
     - BASS Warehouse Drone: a lighter sub (`sub_level` 0.2).
     - RUMBLE Sidechain Fog: its line an octave up (`seq_root` 45), the send at 0.2 and the rumble's tone at 120 Hz.
   - **RUMBLE Sidechain Fog keeps its sidechain and its fog** (phase 5's table: "full duck, long decay, the voice sent
     into the rumble"). The floors are parameter floors, as P5.03 pins `rumble_level`:
     - `rumble_duck` 0.8 or more;
     - `rumble_send` 0.15 or more;
     - `rumble_decay` 4 s or more;
     - `seq_on` 1.

     Zeroing the send, cutting the tail or dropping the duck would pass the 20 Hz check, but it is not this preset.
   - **The rest holds.** Every preset keeps P5.01-P5.03 and P5.09. The task leaves every pattern's steps, and the
     presets it doesn't name, as they are.
   - **The showcases:** run `tools/render-demos.sh` and commit the four showcase renders it writes.
     `sub-pure-foundation.wav` changes; the other three stay byte for byte.

## API

No change.

## Acceptance checks

Checks that share a command are one verify step: the table repeats the command on every row that command proves.

| Check | Command | Proves |
|---|---|---|
| P5.2.01 | `tools/spec.sh 5_2 presets` | SUB Pure Foundation is the clean sine sub alone: holding A2, at least 98% of its energy at its 55 Hz fundamental (50 to 60 Hz), 27.5 Hz at least 60 dB under the sine, and its harmonics 2 to 10 at least 40 dB under it |
| P5.2.02 | `tools/spec.sh 5_2 presets` | Every SUB preset is the sub: at least 95% of its energy below 80 Hz |
| P5.2.03 | `tools/spec.sh 5_2 presets` | No preset puts more than 2% of its energy below 20 Hz |
| P5.2.04 | `tools/spec.sh 5_2 presets` | RUMBLE Sidechain Fog keeps its sidechain and its fog: rumble_duck at least 0.8, rumble_send at least 0.15, rumble_decay at least 4 s, and its sequence on |
| P5.2.05 | `tools/spec.sh 5 presets` | P5.01-P5.03 and P5.09 still pass: the presets keep their categories, characters and levels. A regression check: it passes on the base too |
| P5.2.06 | `tools/spec.sh 5_1 render` | P5.1.02 and P5.1.03 still pass: the renderer plays each preset as the engine does. A regression check: it passes on the base too |
| P5.2.07 | `tools/render-demos.sh && tools/spec.sh 5_1 demos` | P5.1.04 and P5.1.05 still pass: the committed showcase renders are the renderer's files as written. A regression check: it passes on the base too |
| P5.2.08 | `tools/render-demos.sh && tools/spec.sh 5 demos` | P5.06 still passes. A regression check: it passes on the base too |

## Rules every task keeps

- The rules of phases 0 to 5.1 still hold.
- Only the six presets named here change. No engine change, and no high-pass.
- No new files.

## Also required, not checked automatically

- Listening: SUB Low Pressure and SUB Kick Companion should still sound like their descriptions. Without the saw they
  are close to SUB Pure Foundation; their envelopes and saturation set them apart.

## Notes for the planner

- This spec is `docs/specs/phase-5.2-presets.md`. Put it first among the files the task reads.
- One task: **The presets as named** (`core/src/presets.cpp` and the four `artifacts/audio/*.wav`).
  - Its verify steps: P5.2.01 to P5.2.04 in one step, then P5.2.05 to P5.2.08 (`expect = "pass_on_base"`), with
    their commands unchanged.
  - About 10 lines. It commits the four WAVs its demos step writes.
- The task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check before
  it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep` and
  `glob`.
- `tools/check.sh --plugin` is the plan's final check, not the task's: every present spec test, both presets, and
  pluginval at strictness 5. It runs `tools/render-demos.sh` and a test install before the core tests.
