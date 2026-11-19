Set the plan title field to exactly: P0 · Undertow foundation

Use the following spec word for word as the request. Do not paraphrase it. Do not omit the acceptance checks table. Do not run the plan.

# Phase 0: foundation

**Goal**: the build system, the plain C++ core library with the full parameter table and an engine skeleton, and a
JUCE plugin skeleton (VST3, AU, Standalone) that passes pluginval, so every later phase only adds DSP and UI.

**Depends on**: nothing (the seed: `docs/`, `tests/spec/`, `third_party/doctest/`, `tools/fetch-juce.sh`,
`tools/fetch-pluginval.sh`, `tools/spec.sh`, `tools/pluginval.sh`).

**Size**: M, 4 tasks, about 600 lines (most of it the 57-row parameter table).

---

## Scope

In scope:
- `CMakeLists.txt`, `CMakePresets.json`, `cmake/Juce.cmake`.
- `core/`: the `undertow_core` library with `undertow/params.h` (the parameter table) and `undertow/engine.h`
  (the engine skeleton: parameters, MIDI and transport in, silence out, the beat clock, meters).
- `plugin/`: the JUCE plugin target `Undertow` with `UndertowProcessor` and a placeholder `UndertowEditor`.
- `tools/check.sh`.

Out of scope: any sound (phase 1 adds the voice), the real editor (phase 4), presets (phase 5). The engine outputs
silence in this phase; that is correct.

## Context

- The PRD (`docs/PRD.md`) sections 3 (architecture) and 4 (parameters) are part of this spec.
- The spec tests for this phase are `tests/spec/p0/params_test.cpp`, `engine_test.cpp` and `plugin_test.cpp`, and
  the harness that builds them is `tests/spec/CMakeLists.txt`. They exist already: change the code, never them.
- JUCE 8.0.12 comes from `tools/fetch-juce.sh --prefix`, which downloads, checks (SHA256), builds and installs it
  once into `~/Library/Caches/undertow/JUCE-8.0.12-install` and prints that path. Use it with
  `find_package(JUCE 8.0.12 EXACT CONFIG REQUIRED PATHS <prefix> NO_DEFAULT_PATH)`; never `FetchContent`, which
  would download JUCE into every worktree.
- There is no Ninja on this machine: use the Unix Makefiles generator.

## What to build

1. **The build system** (`CMakeLists.txt`, `CMakePresets.json`, `cmake/Juce.cmake`, `core/CMakeLists.txt`)
   - Top level: `cmake_minimum_required(VERSION 3.22)`; `CMAKE_OSX_DEPLOYMENT_TARGET` 11.0 (set before `project`);
     `project(Undertow VERSION 0.1.0 LANGUAGES C CXX)`; C++17 required, no extensions; `include(CTest)`;
     `option(UNDERTOW_BUILD_PLUGIN "Build the JUCE plugin" OFF)`; `add_subdirectory(core)`; when
     `UNDERTOW_BUILD_PLUGIN` is ON, `include(cmake/Juce.cmake)` and `add_subdirectory(plugin)`; when `BUILD_TESTING`
     is ON, `add_subdirectory(tests/spec)` (after the plugin, so the harness sees the `Undertow` target).
   - `CMakePresets.json` (version 3): configure presets `core` (generator "Unix Makefiles", binary dir
     `${sourceDir}/build/core`, cache `CMAKE_BUILD_TYPE=RelWithDebInfo`, `UNDERTOW_BUILD_PLUGIN=OFF`,
     `BUILD_TESTING=ON`) and `plugin` (inherits `core`; binary dir `${sourceDir}/build/plugin`,
     `UNDERTOW_BUILD_PLUGIN=ON`); build presets `core` and `plugin`; test presets `core` and `plugin` with
     `outputOnFailure` and `noTestsAction` error.
   - `cmake/Juce.cmake`: a cache path `UNDERTOW_JUCE_PREFIX`; when empty, run `tools/fetch-juce.sh --prefix`
     (`execute_process`, output stripped, fail the configure if it fails) and `find_package` JUCE there.
   - `core/CMakeLists.txt`: a static library `undertow_core` with the alias `Undertow::core`, public include dir
     `core/include`, C++17, position-independent code, warnings `-Wall -Wextra -Wpedantic`. No JUCE.

2. **The parameter table** (`core/include/undertow/params.h`, `core/src/params.cpp`)
   - Transcribe `docs/PRD.md` section 4 exactly: 57 rows in that order, with the choice labels listed under the
     table. `tests/spec/p0/params_test.cpp` holds the same table as C++ data.
   - `findParam` compares ids exactly and returns -1 for anything else.

3. **The engine skeleton** (`core/include/undertow/engine.h`, `core/src/engine.cpp`)
   - The API below, with a private `Impl` (pimpl) so later phases change only `engine.cpp`.
   - A new `Engine` holds every parameter at its default. `setParam` clamps to the table's range, rounds Int and
     Choice values to the nearest whole number, maps Bool to 0 or 1 (1 from 0.5 up), and ignores an index out of
     range; `getParam` returns 0 for one.
   - `process` writes all `numSamples` samples of both channels (silence in this phase) and nothing past them;
     `numSamples` may be 0 or larger than `maxBlockSize` (process it in chunks); MIDI event offsets are within the
     block and sorted.
   - The beat clock (PRD 3.4): with `transport.valid`, the position at the end of the block is
     `transport.ppq + numSamples / sampleRate * bpm / 60` while playing, or `transport.ppq` when stopped; without
     it, the engine's own clock advances by `numSamples / sampleRate * seq_bpm / 60` per block from 0 after `reset()`.
     `meters().ppq` reports the position at the end of the last block.
   - `meters()` may be called from another thread: keep its values in `std::atomic`s. `peak` is the last block's
     output peak; `rms` is a smoothed output RMS (about 50 ms); the other fields keep their defaults this phase.
   - No heap allocation in `process`, `setParam`, `getParam`, `meters` or `reset`.

4. **The plugin skeleton** (`plugin/CMakeLists.txt`, `plugin/PluginProcessor.h/.cpp`, `plugin/PluginEditor.h/.cpp`)
   - `juce_add_plugin(Undertow VERSION 0.1.0 COMPANY_NAME "Metakosmia" PLUGIN_MANUFACTURER_CODE Mtks PLUGIN_CODE MkUt
     BUNDLE_ID "audio.metakosmia.undertow" PRODUCT_NAME "Metakosmia Undertow" FORMATS VST3 AU Standalone IS_SYNTH TRUE
     NEEDS_MIDI_INPUT TRUE NEEDS_MIDI_OUTPUT FALSE IS_MIDI_EFFECT FALSE VST3_CATEGORIES Instrument Synth
     AU_MAIN_TYPE kAudioUnitType_MusicDevice COPY_PLUGIN_AFTER_BUILD FALSE)`.
   - Definitions `JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0 JUCE_VST3_CAN_REPLACE_VST2=0`; link `Undertow::core` and
     `juce::juce_audio_utils` PRIVATE (the spec harness takes the JUCE objects from the plugin's static library, so
     JUCE compiles once), and `juce::juce_recommended_config_flags` and `juce::juce_recommended_warning_flags` PUBLIC.
     `target_include_directories(Undertow PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})`.
   - `UndertowProcessor`: one stereo output bus, no input bus (only that layout is supported); an
     `AudioProcessorValueTreeState` built from `undertow::paramSpec`, one parameter per row in table order, id
     `juce::ParameterID{spec.id, 1}`: Float rows as `AudioParameterFloat` with `NormalisableRange(min, max)` and
     `setSkewForCentre(centre)` when centre is not 0, Int rows as `AudioParameterInt`, Choice rows as
     `AudioParameterChoice` with the table's labels, Bool rows as `AudioParameterBool`; unit labels from the table.
   - `processBlock`: wrap in `juce::ScopedNoDenormals`; copy every parameter's raw value into `engine().setParam`;
     convert the MIDI buffer into a `std::vector<undertow::MidiEvent>` reserved in `prepareToPlay` (never grown in
     `processBlock`: drop events past its capacity); read the play head into an `undertow::Transport` (`valid` when
     the host gives a ppq position); call `engine().process` on channels 0 and 1.
   - State: the APVTS state as XML (`copyXmlToBinary` / `getXmlFromBinary`).
   - `getName()` returns `JucePlugin_Name`; `getTailLengthSeconds()` 8 (the rumble's longest decay).
   - `UndertowEditor`: a placeholder, fixed 1200 x 760, not resizable, painting the void (`#060608`) with a
     hairline border (`#181420`) and the product name in rose (`#AA7088`). Phase 4 replaces its contents.
   - `juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()` returns a new `UndertowProcessor`.

5. **The check script** (`tools/check.sh`)
   - `tools/check.sh`: `cmake --preset core`, `cmake --build --preset core`, `ctest --preset core`, then print
     `check: OK`. With `--plugin`: also `cmake --preset plugin`, `cmake --build --preset plugin`,
     `ctest --preset plugin` and `tools/pluginval.sh 5`. Stop at the first failure with its exit code (`set -euo pipefail`).

## API

The headers the tasks create. Names, types and signatures are what the spec tests compile against.

```cpp
// core/include/undertow/params.h
#pragma once
#include <string_view>

namespace undertow {

enum class ParamKind { Float, Int, Choice, Bool };

struct ParamSpec {
    const char* id;              // stable host id, e.g. "filter_cutoff"
    const char* name;            // display name, e.g. "Cutoff"
    ParamKind kind;
    float min;                   // real units; Choice: 0
    float max;                   // real units; Choice: numChoices - 1
    float def;                   // default, real units
    float centre;                // value at half knob travel (setSkewForCentre); 0 = linear
    const char* unit;            // "", "Hz", "ms", "s", "dB", "oct", "st", "ct", "bpm"
    const char* const* choices;  // Choice labels, else nullptr
    int numChoices;              // Choice: number of labels, else 0
};

constexpr int kNumParams = 57;

const ParamSpec& paramSpec(int index);   // 0 <= index < kNumParams, in docs/PRD.md section 4 order
int findParam(std::string_view id);      // the index of `id`, or -1

namespace param {
enum : int {  // the table's indices: the id in CamelCase
    OscWave, OscPw, OscLevel, OscOctave, OscFine, SubLevel, SubOctave,
    FilterCutoff, FilterRes, FilterDrive, FilterEnv, FilterTrack,
    AmpAttack, AmpDecay, AmpSustain, AmpRelease,
    FiltAttack, FiltDecay, FiltSustain, FiltRelease,
    GlideMs, GlideMode, Accent, BendRange,
    Lfo1Rate, Lfo1Shape, Lfo1Cutoff, Lfo1Phase,
    Lfo2Rate, Lfo2Shape, Lfo2Dest, Lfo2Depth, Lfo2Phase,
    RumbleLevel, RumbleSend, RumbleKick, RumbleRate, RumbleDecay, RumbleTone, RumbleDrive, RumbleDuck,
    SatDrive, Width, MonoFreq, MasterGain,
    SeqOn, SeqSync, SeqBpm, SeqRate, SeqLength, SeqGate, SeqSwing, SeqRoot, SeqScale, SeqMutate, SeqEvolve, SeqSeed,
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
    float peak = 0;        // output peak of the last block, both channels
    float rms = 0;         // smoothed output RMS, about 50 ms
    bool clipping = false; // phase 2
    int note = -1;         // phase 1: the sounding note, -1 when idle
    float noteHz = 0;      // phase 1: the voice's pitch now, 0 when idle
    int step = -1;         // phase 3: the sequencer step playing, -1 when stopped
    float rumble = 0;      // phase 2: the rumble bus RMS
    double ppq = 0;        // the beat clock at the end of the last block
};

class Engine {
public:
    Engine();
    ~Engine();
    Engine(const Engine&) = delete;
    Engine& operator=(const Engine&) = delete;

    void prepare(double sampleRate, int maxBlockSize);  // allocates everything; calls reset()
    void reset();                                       // silence and idle; internal clock to 0
    void setParam(int index, float value);              // real units; clamped, rounded
    float getParam(int index) const;
    void process(float* left, float* right, int numSamples,
                 const MidiEvent* events, int numEvents, const Transport& transport);
    Meters meters() const;                              // safe from another thread

private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};

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

## Acceptance checks

Each check is one command, run from the repository root. Core checks (`params`, `engine`) build with the `core`
preset in seconds; plugin checks build the `plugin` preset (about 80 s the first time in a fresh worktree, mostly
JUCE).

| Check | Command | Proves |
|---|---|---|
| P0.01 | `tools/spec.sh 0 params 'P0.01*'` | The 57 parameters match docs/PRD.md section 4 row for row: id, name, kind, range, default, centre, unit and choice labels |
| P0.02 | `tools/spec.sh 0 params 'P0.02*'` | findParam finds every id and returns -1 for unknown ones; the undertow::param enumerators are the table indices |
| P0.03 | `tools/spec.sh 0 engine 'P0.03*'` | A new Engine holds the defaults; setParam clamps, rounds Int and Choice, maps Bool, and ignores bad indices |
| P0.04 | `tools/spec.sh 0 engine 'P0.04*'` | process writes every sample of both channels (finite) and nothing past them, for 0 to 1536-sample blocks, with MIDI |
| P0.05 | `tools/spec.sh 0 engine 'P0.05*'` | process, setParam, getParam, meters and reset make no heap allocation over 1000 blocks |
| P0.06 | `tools/spec.sh 0 engine 'P0.06*'` | The beat clock follows a playing or stopped host transport, and runs at seq_bpm from 0 without one |
| P0.07 | `tools/spec.sh 0 engine 'P0.07*'` | The core preset builds the core and its tests without configuring JUCE, and no core header includes JUCE |
| P0.08 | `tools/spec.sh 0 plugin 'P0.08*'` | createPluginFilter makes an UndertowProcessor named Metakosmia Undertow: MIDI in, no MIDI out, no inputs, one stereo output |
| P0.09 | `tools/spec.sh 0 plugin 'P0.09*'` | The plugin exposes the 57 table parameters in order with the same ids, names, ranges, defaults, skew centres and choices |
| P0.10 | `tools/spec.sh 0 plugin 'P0.10*'` | getStateInformation then setStateInformation on a new instance restores every parameter |
| P0.11 | `tools/spec.sh 0 plugin 'P0.11*'` | processBlock is finite, never allocates, and passes host parameter values to the engine |
| P0.12 | `tools/spec.sh 0 plugin 'P0.12*'` | The editor is an UndertowEditor, fixed 1200 x 760, dark (void ground), with no pure white |
| P0.13 | `tools/spec.sh 0 plugin 'P0.13*'` | The VST3, AU and Standalone bundles are built with bundle id audio.metakosmia.undertow and the AU is aumu MkUt Mtks |
| P0.14 | `tools/pluginval.sh 5` | pluginval 1.0.4 passes the VST3 at strictness 5 (seed 42, no GUI tests) |
| P0.15 | `tools/check.sh --plugin` | The whole local check passes: both presets build, every present spec test passes in ctest, and pluginval passes |

## Rules every task keeps

- Never edit `tests/spec/`, `third_party/`, `docs/` or the given `tools/` scripts; a failing check means the code
  is wrong.
- `core/` stays plain C++17: no JUCE include, no JUCE target.
- In `process`, `setParam`, `getParam`, `meters` and `reset`: no allocation, lock, I/O or logging.
- Keep the plugin a thin wrapper: all sound comes from `undertow::Engine`.

## Also required, not checked automatically

- The Standalone app opens and shows the placeholder editor (open `Metakosmia Undertow.app` from
  `build/plugin/plugin/Undertow_artefacts/RelWithDebInfo/Standalone/` once by hand).
- FL Studio 20 finds the VST3 after a rescan (once installed, phase 5).
- Code reads like Metakosmia's: small files, plain names, comments only where the why is not obvious.

## Notes for the planner

- Suggested tasks: (1) the build system and the core library target; (2) the parameter table; (3) the engine
  skeleton; (4) the plugin skeleton and `tools/check.sh`. Tasks 2 and 3 can run in parallel after 1; task 4 needs
  1 to 3.
- A plugin check's first build in a fresh worktree compiles the JUCE modules (about a minute at 4 jobs); later
  runs are incremental.