# Phase 1: the foundation of Undertow 2

**Goal**: Undertow 2's parameter table, identity and engine skeleton, on the first version's chassis. The parts it
replaces are retired; the parts it keeps (the build, the parameter and state systems, the rumble, the output stage,
the plugin wrapper, the renderer and the scripts) compile against the new table; the plugin passes pluginval. Every
later phase only adds DSP and UI.

**Depends on**: nothing. The base is the first version's code with this spec's seed: `docs/`, `tests/spec/`,
`tools/spec.sh`, `tools/pluginval.sh`, `tools/auval.sh` and the fetch scripts.

**Size**: M, 2 tasks, about 450 lines written (half of it the 49-row table) and about 2,000 removed.

---

## Scope

In scope:
- `core/`: `core/include/undertow/params.h` and `core/src/params.cpp` (the new table),
  `core/include/undertow/engine.h` and `core/src/engine.cpp` (the skeleton), `core/include/undertow/output_stage.h`
  and `core/src/output_stage.cpp` (mono), `core/include/undertow/presets.h` and `core/src/presets.cpp` (one program,
  Init), and twelve retired files deleted (listed in "What to build", item 2).
- `plugin/`: the identity in `plugin/CMakeLists.txt`, `plugin/PluginProcessor.h` and `plugin/PluginProcessor.cpp`, a
  placeholder `plugin/PluginEditor.h` and `plugin/PluginEditor.cpp`, `plugin/ui/Theme.h` cut down, and nine retired
  window files deleted (listed in item 6).
- `tools/render/main.cpp`, `tools/render-demos.sh` and `tools/install.sh`, updated to the new table and identity.

Out of scope: any voice (phase 2 adds the oscillators and the ladder, phase 3 the voice), the sequencer (phase 4),
the presets and the darker rumble (phase 5), the window (phase 6). The engine's only sound this phase is the rumble's
ghost kick through its reverb: that is correct.

## Context

- The PRD (`docs/PRD.md`) sections 3 (architecture) and 4 (parameters) are part of this spec.
- The spec tests: `tests/spec/p1/params_test.cpp`, `engine_test.cpp`, `output_test.cpp`, `rumble_test.cpp`,
  `plugin_test.cpp` and `render_test.cpp`, with helpers in `tests/spec/support/` (`spec_engine.h` has `defaults()`,
  `set()` and `render()`). Change the code, never the tests.
- **What stays from the first version, and works as it is:** the top-level `CMakeLists.txt`, `CMakePresets.json`,
  `cmake/Juce.cmake`, `core/CMakeLists.txt` (its source glob), `core/include/undertow/rumble.h` and
  `core/src/rumble.cpp` (they read only the eight `rumble_*` parameters, whose ids and enumerator names stay),
  `tools/check.sh` and the fetch scripts.
- **What stays and changes here:** `params`, `engine`, `output_stage` and `presets` in `core/`; the processor, the
  editor and `plugin/CMakeLists.txt`; the renderer, `render-demos.sh` and `install.sh`. Read each before changing it:
  most of the work is removing what the first version needed and v2 does not.
- `git show v1.0:<path>` prints any file as the first version had it.

## What to build

1. **The parameter table** (`core/include/undertow/params.h`, `core/src/params.cpp`)
   - Transcribe `docs/PRD.md` section 4 exactly: 49 rows in that order, with the choice labels listed under the table.
     `tests/spec/p1/params_test.cpp` holds the same table as C++ data. The `ParamSpec` struct, `paramSpec`,
     `findParam` and the `undertow::param` enumerators keep the first version's form (API below).
   - `findParam` compares ids exactly and returns -1 for anything else, the first version's ids included
     (`filter_cutoff`, `glide_ms`, `seq_on`).

2. **The retired core files** (delete these twelve)
   - Delete: `core/include/undertow/oscillator.h`, `core/src/oscillator.cpp`, `core/include/undertow/filter.h`,
     `core/src/filter.cpp`, `core/include/undertow/envelope.h`, `core/src/envelope.cpp`,
     `core/include/undertow/lfo.h`, `core/src/lfo.cpp`, `core/include/undertow/voice.h`, `core/src/voice.cpp`,
     `core/include/undertow/sequencer.h`, `core/src/sequencer.cpp`.
   - Phases 2 to 4 build their replacements under new names (`vco.h`, `ladder.h`, `decay.h`) or anew (`voice.h`,
     `sequencer.h`).

3. **The engine skeleton** (`core/include/undertow/engine.h`, `core/src/engine.cpp`)
   - The API below. `setPattern`, `pattern` and the pattern handoff go: the sequencer's steps are parameters now.
   - A new `Engine` holds every parameter at its default. `setParam` clamps to the table's range, rounds Int and
     Choice values to the nearest whole number, and ignores an index out of range; `getParam` returns 0 for one.
   - `process` writes all `numSamples` samples of both channels and nothing past them, the same value to each
     (the instrument is mono); `numSamples` may be 0 (nothing changes, the meters included) or larger than
     `maxBlockSize`. MIDI events are accepted and ignored this phase.
   - Per sample: the rumble (`Rumble::process(send = 0, ppq, bpm, running)`) into the output stage, the result to both
     channels and to the scope.
   - The beat clock (PRD 3.4): with `transport.valid`, the position is the host's ppq, advanced through the block at
     the host's tempo while it plays and held while it is stopped; without it, the engine's own clock advances at
     `seq_bpm` from 0 after `reset()` and always runs. `meters().ppq` is the position at the end of the last block,
     `meters().hostClock` whether the host drives it and `meters().bpm` the tempo in use.
   - `meters()` may be called from another thread: keep its values in `std::atomic`s. `peak` is the last block's
     output peak, `rms` a smoothed output RMS (about 50 ms), `clipping` the output stage's clip meter, `rumble` the
     rumble bus's smoothed RMS; `note`, `noteHz` and `step` keep their idle values (-1, 0, -1) until phases 3 and 4.
   - `copyScope` keeps the first version's behaviour: the latest `kScopeSamples` output samples, oldest first.
   - No heap allocation in `process`, `setParam`, `getParam`, `meters` or `reset`.

4. **The output stage, mono** (`core/include/undertow/output_stage.h`, `core/src/output_stage.cpp`)
   - Keep, in this order: the DC blocker (a one-pole high-pass at about 5 Hz), the master gain (`master_gain` in dB,
     smoothed with a 5 ms time constant), the clip detector (over 1.0 before the clipper lights `clipped()` for 0.5 s)
     and `softClip` (the identity up to 0.5, a tanh knee above it, never past 0.966).
   - Remove the saturation, the crossover, the side signal and the width delay: their parameters are gone.
     `process` takes one sample and returns one.
   - Flush the DC blocker's state to exact zero once it is negligible, so silence in is silence out.

5. **The presets** (`core/include/undertow/presets.h`, `core/src/presets.cpp`)
   - One program, `Init` (slug `init`): every parameter at its table default. The API below; phase 5 adds the rest.

6. **The plugin** (`plugin/CMakeLists.txt`, `plugin/PluginProcessor.h`, `plugin/PluginProcessor.cpp`,
   `plugin/PluginEditor.h`, `plugin/PluginEditor.cpp`, `plugin/ui/`)
   - `juce_add_plugin(Undertow VERSION 2.0.0 COMPANY_NAME "Metakosmia" PLUGIN_MANUFACTURER_CODE Mtks PLUGIN_CODE MkU2
     BUNDLE_ID "audio.metakosmia.undertow2" PRODUCT_NAME "Metakosmia Undertow 2" ...)`; everything else in the call,
     the definitions and the links stay as they are.
   - `UndertowProcessor`: the first version's wrapper with the pattern taken out. One stereo output bus, no input bus,
     only that layout. The `AudioProcessorValueTreeState` is built from `undertow::paramSpec`, one parameter per row
     in table order, id `juce::ParameterID{spec.id, 1}`: Float rows as `AudioParameterFloat` with
     `NormalisableRange(min, max)` and `setSkewForCentre(centre)` when centre is not 0, Int rows as
     `AudioParameterInt`, Choice rows as `AudioParameterChoice` with the table's labels. Remove the Seq Gate grid
     (that parameter is gone); no parameter has a grid.
   - Every Float parameter set by a host to its default reads its default back exactly, from either end of its range
     (P1.17): auval compares exactly. The table's defaults all come back exactly through JUCE's 0-to-1 conversion as
     it is; a changed default might not.
   - `processBlock`: as the first version's: `juce::ScopedNoDenormals`, every parameter's raw value into
     `engine().setParam`, the MIDI buffer into the reserved `std::vector<undertow::MidiEvent>` (never grown in
     `processBlock`), the play head into an `undertow::Transport`, `engine().process` on channels 0 and 1.
   - State: the APVTS state as XML (`copyXmlToBinary` / `getXmlFromBinary`) with no pattern element. After
     `replaceState`, set every parameter from the state (`updateParametersFromState`): `replaceState` skips a
     parameter whose current value already maps to the state's, which leaves a host's raw value in place.
   - Programs: `getNumPrograms()` is `undertow::numFactoryPresets()`, `setCurrentProgram(i)` sets every parameter from
     `factoryPreset(i, values)`, `getProgramName(i)` is its name.
   - `getName()` returns `JucePlugin_Name`; `getTailLengthSeconds()` 8 (the rumble's longest decay).
   - `UndertowEditor`: a placeholder, fixed 1200 x 760, not resizable, painting the void (`#060608`), a hairline
     border (`#181420`) and the product name in rose (`#AA7088`). Phase 6 replaces its contents.
   - `plugin/ui/`: cut `plugin/ui/Theme.h` down to its colour tokens, `sansFont`, `monoFont`, `editorWidth` and
     `editorHeight`, and keep `plugin/ui/Texts.h` as it is.
   - Delete these nine: `plugin/ui/Displays.h`, `plugin/ui/Displays.cpp`, `plugin/ui/Look.h`, `plugin/ui/Look.cpp`,
     `plugin/ui/Status.h`, `plugin/ui/Status.cpp`, `plugin/ui/UnitArray.h`, `plugin/ui/UnitArray.cpp`,
     `plugin/ui/UiFrame.h`.

7. **The renderer and the scripts** (`tools/render/main.cpp`, `tools/render-demos.sh`, `tools/install.sh`)
   - `undertow_render --preset <init|slug|index> --out <file.wav> [--bars N] [--bpm B] [--note N] [--sample-rate R]`:
     loads the preset into an `Engine`, holds MIDI note `--note` (default 43, G2) from the first sample to the last,
     and renders `--bars` bars (default 8) at `--bpm` (default 130) on a playing host transport from beat 0, in
     blocks of 512, as 24-bit stereo PCM. It prints the frame count, peak and RMS, as now.
   - `tools/render-demos.sh` empties `build/demos` and renders every factory preset there (this phase only
     `build/demos/init.wav`), 8 bars at 130 BPM on note 43. It writes nothing under `artifacts/` this phase.
   - `tools/install.sh`: the same options and behaviour, for the bundles `Metakosmia Undertow 2.vst3`, `.component`
     and `.app`.

## API

The headers the tasks write. Names, types and signatures are what the spec tests compile against.

```cpp
// core/include/undertow/params.h
#pragma once
#include <string_view>

namespace undertow {

enum class ParamKind { Float, Int, Choice, Bool };

struct ParamSpec {
    const char* id;              // stable host id, e.g. "vcf_cutoff"
    const char* name;            // display name, e.g. "Cutoff"
    ParamKind kind;
    float min;                   // real units; Choice: 0
    float max;                   // real units; Choice: numChoices - 1
    float def;                   // default, real units
    float centre;                // value at half knob travel (setSkewForCentre); 0 = linear
    const char* unit;            // "", "Hz", "ms", "s", "dB", "oct", "st", "bpm"
    const char* const* choices;  // Choice labels, else nullptr
    int numChoices;              // Choice: number of labels, else 0
};

constexpr int kNumParams = 49;
constexpr int kSteps = 8;      // the sequencer's steps (phase 4)

const ParamSpec& paramSpec(int index);   // 0 <= index < kNumParams, in docs/PRD.md section 4 order
int findParam(std::string_view id);      // the index of `id`, or -1

namespace param {
enum : int {  // the table's indices: the id in CamelCase
    Vco1Wave, Vco1Level, Vco2Wave, Vco2Tune, Vco2Level, VcoFm, NoiseLevel, SubLevel, SubOctave, PitchEnv, PitchDecay,
    VcfCutoff, VcfRes, VcfDrive, VcfEnv, VcfDecay,
    VcaAttack, VcaDecay,
    SeqMode, SeqRate, SeqLength, SeqDrift, SeqRoot, SeqBpm,
    Step1Pitch, Step2Pitch, Step3Pitch, Step4Pitch, Step5Pitch, Step6Pitch, Step7Pitch, Step8Pitch,
    Step1Vel, Step2Vel, Step3Vel, Step4Vel, Step5Vel, Step6Vel, Step7Vel, Step8Vel,
    RumbleLevel, RumbleSend, RumbleKick, RumbleRate, RumbleDecay, RumbleTone, RumbleDrive, RumbleDuck,
    MasterGain,
};
}  // namespace param

}  // namespace undertow
```

```cpp
// core/include/undertow/engine.h
#pragma once
#include "undertow/params.h"
#include <cstdint>
#include <memory>

namespace undertow {

constexpr int kScopeSamples = 4096;

struct MidiEvent {
    int sampleOffset;      // 0 .. numSamples - 1, sorted
    std::uint8_t status;   // raw MIDI status byte, any channel
    std::uint8_t data1;
    std::uint8_t data2;
};

struct Transport {
    bool valid = false;    // the host gave a beat position; false in the Standalone app
    bool playing = false;
    double bpm = 120.0;
    double ppq = 0.0;      // quarter notes at the block's first sample
};

struct Meters {
    float peak = 0;           // output peak of the last block
    float rms = 0;            // smoothed output RMS, about 50 ms
    bool clipping = false;    // the output stage's clip meter
    int note = -1;            // phase 3: the note the voice plays, -1 when idle
    float noteHz = 0;         // phase 3: that note's pitch, 0 when idle
    int step = -1;            // phase 4: the step playing (0 .. 7), -1 when the sequencer is idle
    float rumble = 0;         // the rumble bus's smoothed RMS
    double ppq = 0;           // the beat clock at the end of the last block
    bool hostClock = false;   // the host's transport drives the clock
    double bpm = 120.0;       // the tempo in use
};

class Engine {
public:
    Engine();
    ~Engine();
    Engine(const Engine&) = delete;
    Engine& operator=(const Engine&) = delete;

    void prepare(double sampleRate, int maxBlockSize);  // allocates everything; calls reset()
    void reset();                                       // silence and idle; own clock to 0
    void setParam(int index, float value);              // real units; clamped, rounded
    float getParam(int index) const;
    void process(float* left, float* right, int numSamples,
                 const MidiEvent* events, int numEvents, const Transport& transport);
    Meters meters() const;                              // safe from another thread
    int copyScope(float* destination, int count) const;  // latest samples, oldest first; any thread; no allocation

private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};

}  // namespace undertow
```

```cpp
// core/include/undertow/output_stage.h
#pragma once

namespace undertow {

float softClip(float x);  // the identity for |x| <= 0.5, a tanh knee above, never past 0.966

class OutputStage {
public:
    void prepare(double sampleRate);
    void reset();
    void setParams(const float* values);  // kNumParams values in real units; reads master_gain
    float process(float x);               // DC blocker, master gain (smoothed), clip detector, softClip
    bool clipped() const;                 // over 1.0 before the clipper in the last 0.5 s
};

}  // namespace undertow
```

```cpp
// core/include/undertow/presets.h
#pragma once
#include "undertow/params.h"
#include <string_view>

namespace undertow {

int numFactoryPresets();                        // phase 1: 1
const char* factoryPresetName(int index);       // "Init"; "" out of range
const char* factoryPresetSlug(int index);       // "init"; "" out of range
bool factoryPreset(int index, float* values);   // kNumParams values in real units; false out of range
int findFactoryPreset(std::string_view nameOrSlug);  // by name or slug, or -1

}  // namespace undertow
```

```cpp
// plugin/PluginProcessor.h (the parts the spec tests use)
class UndertowProcessor final : public juce::AudioProcessor {
public:
    UndertowProcessor();
    undertow::Engine& engine();                          // the engine processBlock drives
    juce::AudioProcessorValueTreeState& parameters();
    // ... the juce::AudioProcessor overrides
};

// plugin/PluginEditor.h
class UndertowEditor final : public juce::AudioProcessorEditor {
public:
    explicit UndertowEditor(UndertowProcessor&);
};
```

`rumble.h` keeps its API unchanged.

## Acceptance checks

Each check is one command, run from the repository root. Checks that share a command are one verify step: the table
repeats the command on every row it proves. Core checks build with the `core` preset in seconds; the plugin checks
build the `plugin` preset (about a minute the first time in a fresh worktree, mostly JUCE).

| Check | Command | Proves |
|---|---|---|
| P1.01 | `tools/spec.sh 1 params` | The 49 parameters match docs/PRD.md section 4 row for row: id, name, kind, range, default, centre, unit and choice labels |
| P1.02 | `tools/spec.sh 1 params` | findParam finds every id and returns -1 for unknown ones and for the first version's ids; the undertow::param enumerators are the table's indices |
| P1.03 | `tools/spec.sh 1 engine` | A new Engine holds the defaults; setParam clamps, rounds Int and Choice, and ignores bad indices |
| P1.04 | `tools/spec.sh 1 engine` | process writes every sample of both channels, finite and the same on each, and nothing past them, for 0 to 1536-sample blocks, with MIDI; an empty block changes nothing |
| P1.05 | `tools/spec.sh 1 engine` | process, setParam, getParam, meters and reset make no heap allocation over 1000 blocks |
| P1.06 | `tools/spec.sh 1 engine` | The beat clock follows a playing or stopped host transport, and runs at seq_bpm from 0 without one |
| P1.07 | `tools/spec.sh 1 engine` | The core builds without JUCE, and the retired oscillator, filter, envelope and LFO files are gone |
| P1.08 | `tools/spec.sh 1 output` | The output stage: transparent to -6 dBFS, never past -0.3 dBFS, a DC blocker that keeps 30 Hz, master_gain smoothed, the clip meter for half a second |
| P1.09 | `tools/spec.sh 1 rumble` | The kept rumble through the engine: ghost kicks on the beat grid while the clock runs and none while the host is stopped, quietest on the beat, 90% of its energy below 200 Hz, silence at level 0 |
| P1.10 | `tools/spec.sh 1 plugin` | The processor is Metakosmia Undertow 2: MIDI in, one stereo output, no inputs, only that layout |
| P1.11 | `tools/spec.sh 1 plugin` | The plugin exposes the 49 parameters of the core table, in order, with the same ranges, defaults, centres and choices |
| P1.12 | `tools/spec.sh 1 plugin` | State saves and restores every parameter, into a new processor or over values a host moved, reaches the engine, and holds no pattern |
| P1.13 | `tools/spec.sh 1 plugin` | processBlock is finite, the same on both channels, never allocates, and hands host parameter changes to the engine |
| P1.14 | `tools/spec.sh 1 plugin` | Program 0 is Init, which sets every parameter to its default; the programs are the factory presets, one each |
| P1.15 | `tools/spec.sh 1 plugin` | The window is a fixed 1200 x 760 dark ROSEDUST placeholder, never white |
| P1.16 | `tools/spec.sh 1 plugin` | The build makes VST3, AU and Standalone bundles named Metakosmia Undertow 2, bundle id audio.metakosmia.undertow2, AU aumu MkU2 Mtks, version 2.0.0, and `tools/install.sh --dest` installs those three |
| P1.17 | `tools/spec.sh 1 plugin` | Every Float parameter set by a host to its default reads its default back exactly, from either end of its range |
| P1.18 | `tools/spec.sh 1 render` | The renderer writes the Init program as a 24-bit stereo WAV, the requested bars long at the requested tempo, mono, finite, not silent and repeatable, by slug, name or index |
| P1.19 | `tools/pluginval.sh 5` | pluginval 1.0.4 passes the VST3 at strictness 5 (seed 42, no GUI tests) |
| P1.20 | `tools/check.sh --plugin` | The whole local check passes: both presets build, the demo render and the test install run, every present spec test passes in ctest, and pluginval passes |

## Rules every task keeps

- Never edit `tests/spec/`, `third_party/`, `docs/` or the given `tools/` scripts (`spec.sh`, `pluginval.sh`,
  `auval.sh`, `fetch-juce.sh`, `fetch-pluginval.sh`); a failing check means the code is wrong.
- `core/` stays plain C++17: no JUCE include, no JUCE target.
- In `process`, `setParam`, `getParam`, `meters` and `reset`: no allocation, lock, I/O or logging.
- Keep the plugin a thin wrapper: all sound comes from `undertow::Engine`.
- Don't touch `rumble.h` or `rumble.cpp`; they work as they are.
- No task installs into `~/Library/Audio/Plug-Ins` or `~/Applications`, or runs `auval`: `tools/install.sh` only
  with `--dest`.

## Also required, not checked automatically

- The Standalone app opens and shows the placeholder window (`Metakosmia Undertow 2.app` under
  `build/plugin/plugin/Undertow_artefacts/RelWithDebInfo/Standalone/`, once by hand).
- Code reads like the rest of the repository: small files, plain names, comments only where the why is not obvious.

## Notes for the planner

- This spec is `docs/specs/phase-1-foundation.md`. Put it first among the files every task reads: its "What to
  build" and API sections hold what the tests and the PRD don't (the files to delete, what each kept file keeps).
- Two tasks, in this order. Each task's verify steps are exactly the checks named for it, with their commands
  unchanged, and each passes once that task (and the one before it) is done:
  1. **The core re-keyed.** Architectural: about 350 lines written, most of them the table, and about 1,600 removed.
     - Edits: `core/include/undertow/params.h`, `core/src/params.cpp`, `core/include/undertow/engine.h`,
       `core/src/engine.cpp`, `core/include/undertow/output_stage.h`, `core/src/output_stage.cpp`,
       `core/include/undertow/presets.h`, `core/src/presets.cpp`.
     - Deletes: `core/include/undertow/oscillator.h`, `core/src/oscillator.cpp`, `core/include/undertow/filter.h`,
       `core/src/filter.cpp`, `core/include/undertow/envelope.h`, `core/src/envelope.cpp`,
       `core/include/undertow/lfo.h`, `core/src/lfo.cpp`, `core/include/undertow/voice.h`, `core/src/voice.cpp`,
       `core/include/undertow/sequencer.h`, `core/src/sequencer.cpp`.
     - Its files are all 20 of those paths, the deleted ones included.
     - Checks: P1.01 and P1.02 (one step), P1.03 to P1.07 (one step), P1.08, P1.09.
     - Every core spec test links the whole core library, so the core must compile as a whole before any of its
       checks can pass: delete the retired files and finish the engine before running them. Build only your checks'
       targets (`tools/spec.sh` does): until task 2, the renderer and the plugin still name the first version's
       pattern and parameters, so a full build fails there, and they are task 2's files.
  2. **The plugin re-keyed, and the tools.** Needs task 1. Architectural by its file count (18 paths, half of them
     deletions): about 150 lines written and about 1,300 removed.
     - Edits: `plugin/CMakeLists.txt`, `plugin/PluginProcessor.h`, `plugin/PluginProcessor.cpp`,
       `plugin/PluginEditor.h`, `plugin/PluginEditor.cpp`, `plugin/ui/Theme.h`, `tools/render/main.cpp`,
       `tools/render-demos.sh`, `tools/install.sh`.
     - Deletes: `plugin/ui/Displays.h`, `plugin/ui/Displays.cpp`, `plugin/ui/Look.h`, `plugin/ui/Look.cpp`,
       `plugin/ui/Status.h`, `plugin/ui/Status.cpp`, `plugin/ui/UnitArray.h`, `plugin/ui/UnitArray.cpp`,
       `plugin/ui/UiFrame.h`.
     - Its files are all 18 of those paths, the deleted ones included. `plugin/ui/Texts.h` stays as it is.
     - Checks: P1.10 to P1.17 (one step), P1.18, P1.19.
     - Its first plugin build in a fresh worktree compiles the JUCE modules (about a minute at 4 jobs).
- `tools/check.sh --plugin` (P1.20) is the plan's final check, not a task's.
- P1.17 needs no grid: with the table's defaults every Float parameter comes back exactly. Its message names any
  parameter that doesn't.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
