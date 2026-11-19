# Phase 4: the ROSEDUST phosphor editor

**Goal**: the editor of `docs/design/ui-spec.md`. A fixed 1200 x 760 instrument in the ROSEDUST language: 57 knobs in
dense margins, and in the sparse centre the sound made visible (a tunnel that follows the energy, a phosphor scope
inside an AT field, the playing pitch as the one bone number), a rumble pump field, a unit-array step display you
can edit, and NERV hazard stripes when the output clips. Plus the plumbing it needs: a scope tap in the engine, the
clock in the meters, and the pattern saved with the plugin's state.

**Depends on**: phases 2 and 3 (the meters, the rumble and the sequencer's pattern).

**Size**: L, 7 tasks, about 1000 lines.

---

## Scope

In scope: `Engine::copyScope` and two clock fields in `Meters`; `UndertowProcessor::setPattern`/`pattern` and the
pattern in the plugin state, kept through `prepare` and `reset`; the editor (`plugin/PluginEditor.h`,
`plugin/PluginEditor.cpp`) and its parts in `plugin/ui/`, one file pair each: `UiFrame.h`, `Theme.h`, `Look`,
`UnitArray`, `Displays` and `Status`; the review images under `artifacts/ui/`.

Out of scope: presets and the program list (phase 5); resizing (the editor has one size); OpenGL (JUCE's software
renderer is fast enough at this size).

## Context

- The design is `docs/design/ui-spec.md`: zones and their rectangles, tokens (exact RGB), type, every widget's
  drivers and motion, the frame budget, and which properties are checked here and which a person judges.
- The spec tests: `tests/spec/p4/scope_test.cpp` (core) and `state_test.cpp`, `editor_test.cpp`, `display_test.cpp`,
  `steps_test.cpp`, `budget_test.cpp` (plugin preset), with helpers in `tests/spec/support/spec_ui.h` (fixed
  `UiFrame`s and pixel statistics). They drive a frozen editor with fixed frames and measure offscreen snapshots.
  Change the code, never the tests.
- Phases 0 to 3 must keep passing (`tools/check.sh --plugin`).

## What to build

The editor is split into parts with one file pair each, so the tasks can build them side by side and no file has more
than two owners. The editor frame writes the four widget headers as the stubs given in the API (inline, empty); each
widget's task replaces its header, keeping its public members, and adds its `.cpp`, which the `plugin/ui/*.cpp` glob
picks up.

1. **The plumbing** (`core/include/undertow/engine.h`, `core/src/engine.cpp`, `plugin/PluginProcessor.h`,
   `plugin/PluginProcessor.cpp`, `plugin/CMakeLists.txt`)
   - `copyScope(destination, count)`: the latest `count` mono output samples (`(left + right) / 2`), oldest first, at
     most `kScopeSamples` (4096); returns how many it copied (fewer only before that much audio). A ring buffer the
     audio thread writes and an atomic count, so the UI thread reads without locks; no allocation.
   - `Meters::hostClock` (the host's transport drives the clock, phase 3's rule) and `Meters::bpm` (the tempo in use).
   - `UndertowProcessor::setPattern(pattern)` hands the pattern to the engine (`Engine::setPattern`); `pattern()`
     returns `engine().pattern()`. `getStateInformation` writes the 16 steps (semitones, gate, accent, slide,
     probability) with the parameters; `setStateInformation` restores them through `Engine::setPattern`. Old states
     without steps keep the default.
   - The pattern survives `Engine::prepare` and `reset`. A host restores its state (so `Engine::setPattern`) before
     `prepareToPlay`, and prepares again for a new sample rate or block size. Phase 3's engine sets `defaultPattern()`
     in every `reset` (its spec said the sequencer holds it after `prepare`), which loses a restored or edited
     pattern: only a new engine starts with `defaultPattern()` (P3.14), and `prepare` and `reset` keep the pattern.
   - `plugin/CMakeLists.txt` adds every `plugin/ui/*.cpp` to the plugin's sources by
     `file(GLOB undertow_ui_sources CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/ui/*.cpp")` (in `target_sources`
     of `Undertow`).

2. **The editor frame** (`plugin/ui/UiFrame.h`, `plugin/ui/Theme.h`, `plugin/PluginEditor.h`, `plugin/PluginEditor.cpp`,
   and the stub headers `plugin/ui/Look.h`, `plugin/ui/UnitArray.h`, `plugin/ui/Displays.h`, `plugin/ui/Status.h`)
   - `UiFrame` and `StepEdit` as below. `captureFrame()` reads the processor: `meters()` (peak, rms, clipping, note,
     noteHz, step, rumble, ppq, hostClock, bpm), `filter_res` and `seq_length` from the engine, the latest 2048 scope
     samples, and `energy` = the RMS in dB mapped from -48 .. -6 to 0 .. 1.
   - `Theme.h`: the ROSEDUST tokens of `docs/design/ui-spec.md` section 3 as `juce::Colour`s, the two fonts, the zone
     rectangles of section 2 (and the scope's inside the CHEST), and the heat ramp (void, rose_deep, rose_ember, rose).
   - The editor: fixed 1200 x 760, not resizable. It owns a `Look`, a `Displays`, a `Status` and a `UnitArray` (a
     child component over the GUT, whose `pattern` and `onEdit` it connects to the processor). `paint` draws the look,
     then the displays and the status; `paintOverChildren` the status's hazard bands. `stepBounds`, `stepAt` ask the
     unit array; `editStep` sets the processor's pattern to `UnitArray::edited(...)` at once.
   - A 60 Hz timer: each tick `advance(captureFrame(), elapsed)` (which advances every widget) and repaint. `freeze()`
     stops the timer, so tests and previews drive the editor only through `advance`. Everything drawn depends only on
     the frames advanced and the pattern, never on wall time, so a frozen editor draws the same picture every time.
   - `controlFor` returns null until the controls exist.

3. **The controls** (`plugin/ui/Theme.h`, `plugin/PluginEditor.h`, `plugin/PluginEditor.cpp`)
   - `Theme.h` gains the panel table: each panel's stamp, area and parameters, and the cell of each control in it.
   - One control per parameter: `juce::Slider` rotary knobs, and `seq_on` a `juce::ToggleButton`, attached with
     `AudioProcessorValueTreeState::SliderAttachment`/`ButtonAttachment`; `controlFor(id)` finds it.

4. **The unit array** (`plugin/ui/UnitArray.h`, `plugin/ui/UnitArray.cpp`)
   - 16 cells in the GUT (`stepBounds(i)` in the editor's coordinates, `stepAt(point)`); the playing step lit and
     fading back over 250 ms; rests dashed; steps past `seq_length` dim; accent, slide and probability marks.
   - `edited(pattern, index, edit)` applies a `StepEdit`. A click calls `onEdit` with the gate's toggle, a shift-click
     the accent's, an alt-click the slide's; a drag up or down moves the step a semitone per 8 px.

5. **The look** (`plugin/ui/Look.h`, `plugin/ui/Look.cpp`)
   - A ROSEDUST `LookAndFeel` for the knobs and the button, installed on the editor. `LookAndFeel_V4` draws toggle
     text, ticks and tooltips in white: it sets each of those colours to a token.
   - The static layer (void, panels, hairlines, stamps, labels, crosshair grid, scanlines, seeded noise floor, the
     mark), drawn once per display scale at that scale (`g.getInternalContext().getPhysicalPixelScaleFactor()`) and
     then drawn 1:1.

6. **The CHEST displays** (`plugin/ui/Displays.h`, `plugin/ui/Displays.cpp`)
   - The tunnel (a quarter of the CHEST's resolution, from precomputed tables), the scope with its phosphor history,
     the AT field around it, and the bone number, as `docs/design/ui-spec.md` section 4 describes them.

7. **The status displays, the budget and the review images** (`plugin/ui/Status.h`, `plugin/ui/Status.cpp`,
   `artifacts/ui/`)
   - The HEAD clock and meter, the rumble pump field, and the hazard stripes while the output clips, as section 4
     describes them.
   - Keep 60 Hz with headroom (`docs/design/ui-spec.md` section 5): no per-pixel trigonometry or blurs over
     full-resolution layers, and each tick repaints only the dynamic regions.
   - Run `tools/spec.sh 4 budget` and commit the three images it writes: `artifacts/ui/editor-idle.png`,
     `artifacts/ui/editor-playing.png` and `artifacts/ui/editor-clipping.png` (1200 x 760, each under 2 MB). The 2x
     copies it writes to `build/previews/` stay uncommitted.

## API

```cpp
// core/include/undertow/engine.h, added
namespace undertow {
constexpr int kScopeSamples = 4096;
// in struct Meters:
bool hostClock = false;   // the host's transport drives the clock
double bpm = 120.0;       // the tempo in use
// in class Engine:
int copyScope(float* destination, int count) const;  // latest samples, oldest first; any thread; no allocation
}  // namespace undertow

// plugin/PluginProcessor.h, added to UndertowProcessor
void setPattern(const undertow::Pattern& pattern);
undertow::Pattern pattern() const;

// plugin/ui/UiFrame.h
#include <array>
struct UiFrame {               // everything the editor draws that changes over time
    float energy = 0.0f;       // 0 .. 1
    float peak = 0.0f;
    float rms = 0.0f;
    bool clipping = false;
    int note = -1;
    float noteHz = 0.0f;
    int step = -1;
    int length = 16;
    float resonance = 0.0f;
    float rumble = 0.0f;
    double ppq = 0.0;
    double bpm = 120.0;
    bool hostClock = false;
    std::array<float, 2048> scope{};   // the latest mono output, oldest first
};
enum class StepEdit { ToggleGate, ToggleAccent, ToggleSlide, Up, Down };

// plugin/PluginEditor.h
class UndertowEditor final : public juce::AudioProcessorEditor, private juce::Timer {
public:
    explicit UndertowEditor(UndertowProcessor&);
    UiFrame captureFrame() const;                         // read the processor, as each tick does
    void freeze();                                        // stop the timer: only advance() changes the picture
    void advance(const UiFrame& frame, double seconds);   // take this state, move every animation on by `seconds`
    juce::Rectangle<int> chestBounds() const;
    juce::Rectangle<int> scopeBounds() const;
    juce::Rectangle<int> stepBounds(int index) const;     // the cell of step 0 .. 15
    int stepAt(juce::Point<int> point) const;             // the cell under a point, or -1
    void editStep(int index, StepEdit edit);
    juce::Component* controlFor(const juce::String& parameterId) const;
};

// The widget headers as the editor frame writes them (stubs). Each widget's task replaces its header with the real
// class, keeping these public members, and adds the .cpp.
// plugin/ui/Look.h
class Look {
public:
    void install(juce::Component&) {}      // the editor and its controls take the ROSEDUST LookAndFeel
    void uninstall(juce::Component&) {}    // before the editor goes away
    void paint(juce::Graphics&) {}         // the static layer, first in the editor's paint
};
// plugin/ui/Displays.h
class Displays {
public:
    void advance(const UiFrame&, double) {}
    void paint(juce::Graphics&) {}         // the CHEST: tunnel, AT field, scope, bone number
};
// plugin/ui/Status.h
class Status {
public:
    void advance(const UiFrame&, double) {}
    void paint(juce::Graphics&) {}         // HEAD clock and meter, pump field
    void paintOver(juce::Graphics&) {}     // the hazard bands, over the controls
};
// plugin/ui/UnitArray.h
class UnitArray final : public juce::Component {   // a child of the editor over the GUT
public:
    std::function<undertow::Pattern()> pattern;            // the pattern to draw
    std::function<void(int index, StepEdit edit)> onEdit;   // a click or drag on a step
    void advance(const UiFrame&, double) {}
    juce::Rectangle<int> stepBounds(int) const { return {}; }   // in the editor's coordinates
    int stepAt(juce::Point<int>) const { return -1; }
    static undertow::Pattern edited(undertow::Pattern pattern, int, StepEdit) { return pattern; }
};
```

## Acceptance checks

Checks that share a command are one verify step: the table repeats the command on every row that command proves.

| Check | Command | Proves |
|---|---|---|
| P4.01 | `tools/spec.sh 4 scope` | copyScope returns the latest mono output samples in order across odd blocks, clamps to kScopeSamples, never allocates; meters() names the clock in use and its tempo |
| P4.02 | `tools/spec.sh 4 state` | The processor passes patterns to the engine, the 16 steps are saved with the plugin's state and restored, and the engine keeps its pattern through prepare and reset (a host restores the state before prepareToPlay) |
| P4.03 | `tools/spec.sh 4 editor 'P4.03*'` | captureFrame reads the processor: meters, pitch, RMS, energy, step, clock, length and scope |
| P4.04 | `tools/spec.sh 4 editor 'P4.04*'` | Every parameter has its control on the editor, attached to it |
| P4.05 | `tools/spec.sh 4 display 'P4.05*'` | The panel is at least half void, without white, rose dominant (its knob panels too), bone rare, and its CHEST calm at rest |
| P4.06 | `tools/spec.sh 4 display 'P4.06*,P4.08*,P4.09*'` | The pitch in bone while a note plays, in one place in the CHEST, and no bone when idle |
| P4.07 | `tools/spec.sh 4 display 'P4.07*'` | The hazard stripes show while the output clips, and only then |
| P4.08 | `tools/spec.sh 4 display 'P4.06*,P4.08*,P4.09*'` | The tunnel is brighter with energy, and moving |
| P4.09 | `tools/spec.sh 4 display 'P4.06*,P4.08*,P4.09*'` | The scope draws the waveform across its width |
| P4.10 | `tools/spec.sh 4 steps` | 16 cells left to right in the GUT, hit-testing, the playing step the brightest cell, and gate, accent, slide and note edits reaching the pattern |
| P4.11-P4.12 | `tools/spec.sh 4 budget` | A full repaint keeps 60 Hz with headroom (CPU time: median 11 ms, 95th percentile 16 ms at 1x; median 27 ms at 2x), and the three review images written: artifacts/ui at 1x under 2 MB, build/previews at 2x |

## Rules every task keeps

- The rules of phases 0 to 3 still hold: never edit `tests/spec/`, `third_party/`, `docs/` or the given tools; `core/`
  stays plain C++17; nothing the audio thread calls allocates, locks, does I/O or logs.
- The UI thread reads the engine only through `meters()`, `copyScope()`, `getParam()` and `pattern()`, and changes
  it only through parameters and `setPattern`.
- No white, no colour as decoration, bone only for the one number (`docs/design/ui-spec.md` section 1).

## Also required, not checked automatically

- The judged list in `docs/design/ui-spec.md` section 7: one instrument, calm at rest and alive when playing; the
  bone number first; the hazard stripe reads as an alarm; dense margins, sparse centre; legible at 100% and on Retina.
- Mouse editing of the unit array works in the Standalone app and in FL Studio; knob values show on hover.
- The editor opens and closes cleanly many times (no leaks: JUCE's leak detector stays quiet).

## Notes for the planner

- This spec is `docs/specs/phase-4-ui.md`. Put it first among the files every task reads: What to build holds the
  rules and API the exact declarations. The tests and `docs/design/ui-spec.md` do not state the rules (the files each
  task owns and the stub headers it replaces, the editor's hooks the tests drive, copyScope's ring, the clock meters,
  and the pattern in the saved state and through prepare).
- Seven tasks, one per part above. Each task's verify steps are exactly the checks named for it, with their commands
  unchanged, and each passes once that task (and the ones it needs) is done:
  1. **The plumbing** (`core/include/undertow/engine.h`, `core/src/engine.cpp`, `plugin/PluginProcessor.h`,
     `plugin/PluginProcessor.cpp`, `plugin/CMakeLists.txt`): P4.01 and P4.02. About 75 lines.
  2. **The editor frame** (`plugin/ui/UiFrame.h`, `plugin/ui/Theme.h`, `plugin/PluginEditor.h`,
     `plugin/PluginEditor.cpp` and the four stub headers `plugin/ui/Look.h`, `plugin/ui/UnitArray.h`,
     `plugin/ui/Displays.h`, `plugin/ui/Status.h`): P4.03. Needs task 1. About 220 lines over 8 files, so architectural.
     It declares every hook of the API, so every phase 4 test file compiles from here on.
  3. **The controls** (`plugin/ui/Theme.h`, `plugin/PluginEditor.h`, `plugin/PluginEditor.cpp`): P4.04. Needs task 2.
     About 90 lines.
  4. **The unit array** (`plugin/ui/UnitArray.h`, `plugin/ui/UnitArray.cpp`): P4.10. Needs task 2. About 150 lines.
  5. **The look** (`plugin/ui/Look.h`, `plugin/ui/Look.cpp`): P4.05. Needs task 3 (its static layer draws the controls'
     labels). About 135 lines.
  6. **The CHEST displays** (`plugin/ui/Displays.h`, `plugin/ui/Displays.cpp`): P4.06, P4.08 and P4.09. Needs task 2.
     About 195 lines, so architectural.
  7. **The status displays, the budget and the review images** (`plugin/ui/Status.h`, `plugin/ui/Status.cpp` and the
     three `artifacts/ui/*.png`): P4.07, P4.11 and P4.12. Needs tasks 3 to 6. About 120 lines. It commits the three
     images its check writes.
  Tasks 3, 4 and 6 run in parallel after task 2; task 5 runs after 3.
- No file has more than two owners: `plugin/PluginEditor.h`, `plugin/PluginEditor.cpp` and `plugin/ui/Theme.h` belong to
  tasks 2 and 3, each widget header to task 2 (the stub) and its widget's task, and every other file to one task. The
  core library's sources are registered by phase 2's glob in `core/CMakeLists.txt`, and no task edits it.
- `tools/check.sh --plugin` is the plan's final check, not any one task's: every present spec test, both presets, and
  pluginval at strictness 5.
- A test file is one executable, so a task's checks run its file's test cases by filter; the file compiles from task
  2 on. The plugin checks build the `plugin` preset: the first build in a fresh worktree compiles JUCE (about a
  minute), later ones are incremental.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
- The display tests read colours by hue and luminance (`tests/spec/support/spec_ui.h`); draw with the ROSEDUST tokens
  of `docs/design/ui-spec.md` section 3 and they hold.
