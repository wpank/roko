# Phase 5: ship

**Goal**: Undertow is ready to use and to show. Twelve factory presets in four categories, as the plugin's
programs; an offline renderer and demo renders anyone can listen to; strict validation (pluginval at strictness
10); and an install script that puts signed bundles where FL Studio and the Finder find them (the checks install into
a test folder; installing for the person is their step, at the end).

**Depends on**: phase 4.2 (merged): phase 4's editor laid out as designed, on phase 4 and 4.1.

**Size**: M, 7 tasks, about 715 lines (most of it preset values).

---

## Scope

In scope: `undertow/presets.h` with `core/src/presets.cpp`; the presets as the plugin's programs; `tools/render/`
(the `undertow_render` program, plain C++ on the core); `tools/render-demos.sh`; `tools/install.sh`; the review
artifacts under `artifacts/audio/` and `artifacts/reports/`; the top of the filter's resonance knob
(`core/src/filter.cpp`); and the sequencer's random draws (`core/src/sequencer.cpp`).

Out of scope: a preset browser in the editor, user preset files, Developer ID signing and notarization (ad hoc
signing only), an installer package.

## Context

- Sound-design goals: `docs/PRD.md` section 1. The renders are what people will hear first: make the presets sound
  like hypnotic techno at 128 BPM.
- The spec tests: `tests/spec/p5/presets_test.cpp`, `render_test.cpp`, `demos_test.cpp`, `install_test.cpp`,
  `filter_test.cpp`, `sequencer_test.cpp` (core preset) and `programs_test.cpp` (plugin preset).
  `tests/spec/support/spec_wav.h` reads WAVs. Change the code, never the tests.
- Given tools: `tools/pluginval.sh 10 <report>` (strictness 10, keeping its log; it skips the GUI tests, which open
  windows, unless given `--gui`, which only the person's step below uses) and `tools/auval.sh` (auval on the installed
  AU, keeping its log; used by hand after a real install, never by a task).
- `render_test.cpp` runs the renderer the build makes: the spec harness gives it the path of the `undertow_render`
  target once that target exists, so `tools/render` must be added before `tests/spec` in `CMakeLists.txt`.

## What to build

1. **The factory presets** (`core/include/undertow/presets.h`, `core/src/presets.cpp`)
   - At least these 12, in this order, each a full parameter set (start from the defaults and change what the sound
     needs) and a pattern:

     | # | Name | Intent |
     |---|---|---|
     | 0 | SUB Pure Foundation | the clean sine sub alone, -1 oct, no rumble, no width: the floor |
     | 1 | SUB Low Pressure | sub at -2 oct with a little saturation, a slow swell |
     | 2 | SUB Kick Companion | a short, round sub note that sits after a kick |
     | 3 | BASS Hypnotic Roller | the default pattern on the sequencer, saw, resonant, LFO on the cutoff, light rumble |
     | 4 | BASS Acid Undertow | square, high resonance, accents and slides, filter envelope |
     | 5 | BASS Dub Pulse | dark saw, long release, LFO 2 at 3/16 on the cutoff |
     | 6 | BASS Warehouse Drone | slow 7/4 LFO on the cutoff, Evolve every 4 bars |
     | 7 | RUMBLE Dark Room | rumble up front (level 0.7, decay 4 s, tone 90 Hz), a quiet sub under it |
     | 8 | RUMBLE Tunnel Floor | 1/8 ghost kicks, more drive, a tight duck |
     | 9 | RUMBLE Sidechain Fog | full duck, long decay, the voice sent into the rumble |
     | 10 | SEQ Evolving Minor | minor scale, Mutate 0.3, Evolve every 2 bars |
     | 11 | SEQ Phrygian Pulse | phrygian pattern, swing, probability on the off-steps |

   - The category is the name's first word. Each preset's slug is its name in lower case with spaces as dashes
     (`bass-hypnotic-roller`). `findFactoryPreset` takes a name or a slug.
   - Character (checked): SUB presets keep 80% of their energy below 80 Hz; BASS presets have at least 5% above
     300 Hz; RUMBLE presets have `rumble_level` 0.4 or more and 60% of their energy below 200 Hz; SEQ presets and
     BASS Hypnotic Roller have `seq_on` 1. Every preset, rendered at 128 BPM (holding A2, note 45, at velocity 100
     when the sequencer is off, so a `-1 oct` sub sounds at 55 Hz), is finite, under -0.3 dBFS and at least -30 dBFS
     RMS (the checks render 2 bars).
   - Level (checked): the presets are loudness-matched. Each preset's render, as above, has an RMS over both channels
     within 3 dB of -18 dBFS, the level the init patch renders at holding A2 at velocity 100 with the -14 dB master
     default (-18.2). Velocity 100 is an accented note (phase 1 accents 100/127 and up, at `1 + accent` times the
     level): at a plain velocity the init patch renders 3.2 dB lower (-21.5), and with its sequencer on, whose plain
     steps play at 90/127, at -21.0. So a SEQ preset built on the defaults needs about +3 dB of `master_gain`. Set
     each preset's `master_gain` to land it there: a render X dB off needs `master_gain` X dB the other way, and the
     check prints X. The default suits few presets.

2. **The programs and strict validation** (`plugin/PluginProcessor.h`, `plugin/PluginProcessor.cpp`)
   - `getNumPrograms()` is the number of factory presets; `getProgramName(i)` its name; `setCurrentProgram(i)` sets
     every parameter (through the APVTS, so the host sees it) and the pattern; `getCurrentProgram()` reports it.
   - Strict validation (P5.08): at strictness 10 pluginval also fuzzes the parameters and then checks that
     `setStateInformation` restores every one, bools included. A host can leave a bool parameter at a fractional raw
     value, and `AudioProcessorValueTreeState::replaceState` skips a parameter whose value already matches the state,
     so after `replaceState` set each parameter from the state
     (`setValueNotifyingHost(convertTo0to1(*getRawParameterValue(id)))`).

3. **The offline renderer** (`tools/render/CMakeLists.txt`, `tools/render/main.cpp`)
   - The target `undertow_render`, a plain C++ program linking `Undertow::core` (no JUCE), built with both presets:
     add `tools/render` to the top-level `CMakeLists.txt` before `tests/spec`.
   - `undertow_render --preset <slug|name|index|init> [--bars 4] [--bpm 128] [--note 45] [--sample-rate 48000] --out <file.wav>`:
     loads the preset (`init` is the default parameters and pattern), runs the engine with a host transport from
     beat 0 at `--bpm` (holding `--note` at velocity 100 when the preset's sequencer is off, as the checks render)
     for exactly `bars * 4 * 60 / bpm * sample-rate` frames (rounded to the nearest frame), and writes a 24-bit PCM
     stereo WAV.
   - Prints one line: `wrote <path>: <frames> frames, peak <x.x> dBFS, rms <x.x> dBFS`. The same arguments give the
     same file, byte for byte. Unknown presets and bad arguments print a message and exit non-zero.

4. **The demo renders** (`tools/render-demos.sh`)
   - Builds `undertow_render` (core preset), empties `build/demos` (so it holds exactly this run's renders) and
     renders, 8 bars at 128 BPM: every factory preset into `build/demos/<slug>.wav`, and the four showcase sounds into
     `artifacts/audio/`: `init.wav`, `sub-pure-foundation.wav`, `bass-hypnotic-roller.wav` and `rumble-dark-room.wav`
     (48 kHz, 24-bit stereo, about 4.3 MB each, under 5 MB). Exits non-zero if any render fails. The task commits the
     four `artifacts/audio` files.

5. **The install script** (`tools/install.sh`)
   - Builds the plugin preset's `Undertow_All` target, then copies `Metakosmia Undertow.vst3` to
     `~/Library/Audio/Plug-Ins/VST3/`, `Metakosmia Undertow.component` to `~/Library/Audio/Plug-Ins/Components/` and
     `Metakosmia Undertow.app` to `~/Applications/`, replacing older copies; signs each ad hoc
     (`codesign --force --deep --sign -`); asks macOS to rescan Audio Units (`killall -9 AudioComponentRegistrar`,
     ignoring a failure); and prints what it installed where. `--dest <dir>` installs into `<dir>/VST3`,
     `<dir>/Components` and `<dir>/Applications` instead (for tests) and leaves the system alone (no rescan);
     `--uninstall` removes the installed copies. The checks run it only with `--dest`; installing for real is the
     person's step (below).

6. **The top of the resonance knob** (`core/include/undertow/filter.h`, `core/src/filter.cpp`)
   - Phase 2.2's resonance curve stops at Q 10 in its resonant stage (damping floored at 0.1): a quiet tone at the
     cutoff gains about +14.5 dB at resonance 0.9 and at 1. That is tamer than phase 1's law (+15 and +27.4) and than
     the curve phase 2.2's spec sketched (+20.4 and +28.5). Phase 2.2's checks covered the middle of the knob (P2.2.11)
     and a held tone's bound (P2.2.13), not its top.
   - A quiet tone (0.01) at the cutoff must gain at least +17 dB at resonance 0.9 and +20 dB at 1. Keep everything the
     earlier checks hold: the middle (P2.2.11), full-level linearity at 0.8 (P2.1.05, P2.1.06), and a held full-scale
     tone within 4 (P2.2.13).
   - Lowering the floor alone is not enough: the current curve is already near its top by 0.8, so it would push 0.8
     past what stays linear at full level. A curve linear in dB does it: `g = -3.01 + 19.39 r` dB up to 0.8, then
     `12.5 + 80.75 (r - 0.8)`, with the resonant stage's damping `k = 0.5412 * 10^(-g / 20)` (+20.6 dB at 0.9 and
     +28.6 at 1, the same Q 50 phase 1 reached).

7. **Random draws below 1** (`core/src/sequencer.cpp`)
   - Phase 3's rule: a step plays when `random(seq_seed, k) < probability`, with `random` in [0, 1), so probability 1
     always plays. Phase 3's sequencer turns its 32-bit hash into a float with `static_cast<float>(h) / 2^32`. A float
     holds 24 bits, so the top 128 hash values round up to exactly 1.0, and a probability-1 step at one of those step
     numbers never plays: seed 1089 drops step 3692 (beat 923, about 7 minutes in at 128 BPM), seed 2443 step 3787,
     and about one seed in a thousand drops a note within an hour. `mutate`'s draws round the same way.
   - Keep every draw below 1: take the hash's top 24 bits (`(h >> 8) * 2^-24`), or divide in double. The same seed
     and step still decide the same way, and phase 3's checks keep passing (P5.14).

## API

```cpp
// core/include/undertow/presets.h
#pragma once
#include "undertow/params.h"
#include "undertow/sequencer.h"
#include <string_view>
namespace undertow {
int numFactoryPresets();
const char* factoryPresetName(int index);    // "BASS Hypnotic Roller"
const char* factoryPresetSlug(int index);    // "bass-hypnotic-roller"
// Fill all kNumParams values (real units) and the pattern of preset `index`; false when out of range.
bool factoryPreset(int index, float* values, Pattern* pattern);
int findFactoryPreset(std::string_view nameOrSlug);   // its index, or -1
}  // namespace undertow
```

## Acceptance checks

Checks that share a command are one verify step: the table repeats the command on every row that command proves.

| Check | Command | Proves |
|---|---|---|
| P5.01-P5.03 | `tools/spec.sh 5 presets` | At least 12 presets with unique names and slugs in four categories and valid values; each renders clean (finite, under -0.3 dBFS, at least -30 dBFS RMS) and repeatably; each category has its character |
| P5.04 | `tools/spec.sh 5 programs` | The presets are the plugin's programs: names in order, and setCurrentProgram sets every parameter and the pattern |
| P5.05 | `tools/spec.sh 5 render` | undertow_render writes a bar-exact 24-bit stereo WAV of a preset by slug, name or init, in level, byte-identical on a second run, and fails on bad input |
| P5.06 | `tools/render-demos.sh && tools/spec.sh 5 demos` | The four showcase renders are in artifacts/audio (48 kHz, 24-bit stereo, 8 bars, under 5 MB) and every preset has a render in build/demos |
| P5.07 | `tools/install.sh --dest build/install-test && tools/spec.sh 5 install` | In a test folder, never the user's own: the three bundles in VST3, Components and Applications, whole, signed, with the Metakosmia ids and the AU's component entry |
| P5.08 | `tools/pluginval.sh 10 artifacts/reports/pluginval-strict.txt` | pluginval passes at strictness 10 (without its GUI tests, so no windows open), and its log is kept for review |
| P5.09 | `tools/spec.sh 5 presets` | The presets are loudness-matched: each renders (holding A2 at velocity 100 when its sequencer is off) within 3 dB of -18 dBFS RMS, the init patch's level there, so none sounds weak or hot next to another |
| P5.10 | `tools/spec.sh 5 filter` | At the cutoff a quiet tone gains at least +17 dB at resonance 0.9 and +20 dB at 1: the top of the knob rings |
| P5.11 | `tools/spec.sh 2_2 filter` | Phase 2.2's filter checks still pass: drive keeps its level, the middle of the knob, a held tone within 4. A regression check: it passes on the base too |
| P5.12 | `tools/spec.sh 2_1 filter` | Phase 2.1's filter checks still pass: the resonance holds at full level, and drive 0 stays linear. A regression check |
| P5.13 | `tools/spec.sh 5 sequencer` | A step at probability 1 always plays: seed 1089 plays step 3692 and seed 2443 step 3787, which phase 3's float division dropped |
| P5.14 | `tools/spec.sh 3 sequencer` | Phase 3's sequencer checks still pass: the grid, rests, accents, slides, probability and its repeatability, mutate and evolve. A regression check: it passes on the base too |

## Rules every task keeps

- The rules of phases 0 to 4 still hold: never edit `tests/spec/`, `third_party/`, `docs/` or the given tools; `core/`
  and `tools/render/` stay plain C++17; nothing the audio thread calls allocates, locks, does I/O or logs.
- The committed artifacts are exactly: `artifacts/audio/init.wav`, `artifacts/audio/sub-pure-foundation.wav`,
  `artifacts/audio/bass-hypnotic-roller.wav`, `artifacts/audio/rumble-dark-room.wav` and
  `artifacts/reports/pluginval-strict.txt`, plus phase 4's `artifacts/ui/*.png` (and `artifacts/reports/auval.txt`
  once the person has installed, below). Everything else a check writes stays in `build/`.
- No task installs into the user's own folders (`~/Library/Audio/Plug-Ins`, `~/Applications`) or runs `auval`: the
  checks install into `build/install-test`.

## Also required, not checked automatically

- pluginval at strictness 10 WITH its GUI tests passes: `tools/pluginval.sh 10 --gui`, run once at the end with a
  heads-up to the person, since it opens editor windows on their screen.
- Installing for the person into `~/Library/Audio/Plug-Ins` and `~/Applications` is their step, at the end, only once
  they agree: `tools/install.sh && tools/auval.sh`. The AU must pass auval (its log is kept in
  `artifacts/reports/auval.txt`, committed afterwards), and FL Studio then finds Undertow after a plug-in rescan.
- Listening, in FL Studio at 128 BPM: each preset sounds finished and like its name; the showcase renders are the
  four best first impressions.
- The presets' patterns use the scale they claim.

## Notes for the planner

- This spec is `docs/specs/phase-5-ship.md`. Put it first among the files every task reads: What to build holds the
  rules and API the exact declarations. The tests and the PRD do not state the rules (the presets' order and how to
  set each one's level, the bool restore pluginval needs, the renderer's arguments and frame count, the install
  paths, a resonance curve that keeps the earlier checks, and why a probability-1 step could drop).
- Seven tasks. Each task's verify steps are exactly the checks named for it, with their commands unchanged, and each
  passes once that task (and the ones it needs) is done:
  1. **The factory presets** (`core/include/undertow/presets.h`, `core/src/presets.cpp`): P5.01-P5.03 and P5.09, in one
     verify step: `tools/spec.sh 5 presets` runs all four.
  2. **The programs and strict validation** (`plugin/PluginProcessor.h`, `plugin/PluginProcessor.cpp`): P5.04 and P5.08.
     Needs task 1. It commits `artifacts/reports/pluginval-strict.txt`. P5.08 already passes on phase 4.2's base:
     phase 4's processor restores every parameter, bools included, at strictness 10. So P5.08 is
     `expect = "pass_on_base"`; it must still pass once task 2 is done.
  3. **The offline renderer** (`tools/render/main.cpp`, `tools/render/CMakeLists.txt`, the top-level `CMakeLists.txt`):
     P5.05. Needs task 1.
  4. **The demo renders** (`tools/render-demos.sh`): P5.06. Needs task 3. It commits the four `artifacts/audio` WAVs.
  5. **The install script** (`tools/install.sh`): P5.07. Needs tasks 2 to 4.
  6. **The top of the resonance knob** (`core/include/undertow/filter.h`, `core/src/filter.cpp`): P5.10, and the
     regression checks P5.11 and P5.12 (`expect = "pass_on_base"`). About 10 lines. It shares no file with the other
     tasks, so it runs in parallel with them.
  7. **Random draws below 1** (`core/src/sequencer.cpp`): P5.13, and the regression check P5.14
     (`expect = "pass_on_base"`). About 5 lines. It shares no file with the other tasks, so it runs in parallel with
     them.
- The core library's sources are registered by the glob in `core/CMakeLists.txt` (phase 2): `presets.cpp` joins it as
  a new file, and no task lists or edits `core/CMakeLists.txt`. Only task 3 edits the top-level `CMakeLists.txt`.
- `tools/check.sh --plugin` is the plan's final check, not any one task's: every present spec test of phases 0 to 5,
  both presets, and pluginval at strictness 5.
- A test file is one executable, so a task owns a whole file's checks.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own check before it hands
  off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep` and `glob`.
- pluginval at strictness 10 runs for several minutes; the checks skip its GUI tests, so no windows open.
