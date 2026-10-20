# Phase 1: the voice

**Goal**: Undertow makes sound. A band-limited saw or square and a clean sine sub, a 24 dB/octave low-pass with
resonance and drive, two envelopes, and a last-note mono voice with glide, legato, accent and pitch bend, played by
MIDI through `undertow::Engine` and safe to run in an audio callback.

**Depends on**: phase 0 (the build system, `undertow/params.h`, `undertow/engine.h`, the plugin).

**Size**: L, 5 tasks, about 900 lines.

---

## Scope

In scope: `undertow/oscillator.h`, `undertow/filter.h`, `undertow/envelope.h`, `undertow/voice.h` with their
`core/src/*.cpp`, the voice wired into `core/src/engine.cpp`, and the new sources in `core/CMakeLists.txt`.

Out of scope: LFOs, rumble, saturation, width and the output clipper (phase 2); the sequencer (phase 3); the
editor (phase 4). Their parameters already exist in the table: the engine ignores them this phase.

## Context

- Sound-design goals: `docs/PRD.md` sections 1.1 (sub) and 1.2 (bassline); parameters: section 4.
- The spec tests: `tests/spec/p1/oscillator_test.cpp`, `filter_test.cpp`, `envelope_test.cpp`, `voice_test.cpp` and
  `engine_test.cpp`, with helpers in `tests/spec/support/` (`spec_engine.h` has `cleanPatch()` and `subPatch()`, the
  parameter sets the tests start from). Change the code, never the tests.
- Each test file is its own executable, so the oscillator, filter and envelope tasks can run in parallel; the voice
  task needs all three, and the engine task needs the voice.

## What to build

1. **Oscillators** (`core/include/undertow/oscillator.h`, `core/src/oscillator.cpp`)
   - `Oscillator`: a phase in [0, 1) advancing by `frequency / sampleRate` per sample. `process()` returns the value
     at the current phase, then advances. Saw: `2 * phase - 1` (a rising ramp). Square: `+1` while
     `phase < pulseWidth`, else `-1`. Both band-limited with a two-sample PolyBLEP correction at every
     discontinuity (the square has two per cycle: subtract the residual at the falling edge `phase - pulseWidth`).
   - `SineOscillator`: `sin(2 pi phase)`, the same phase rules, computed in double precision.
   - `reset(phase)` sets the phase (0 by default); the frequency is clamped to 0 .. sampleRate / 2; the pulse width
     to 0.05 .. 0.95.

2. **The filter** (`core/include/undertow/filter.h`, `core/src/filter.cpp`)
   - A 24 dB/octave low-pass whose cutoff is its -3 dB point at resonance 0. The simple way: two topology-preserving
     state-variable low-pass stages (Zavalishin/Simper, `g = tan(pi * cutoff / sampleRate)`) with damping
     `k1 = 1.8478` and `k2 = 0.7654` (a 4th-order Butterworth), resonance lowering `k2` towards 0.02. A ladder is
     fine too if it meets the checks.
   - Cutoff clamped to 20 Hz .. min(20 kHz, 0.45 sampleRate); resonance and drive to 0 .. 1.
   - Drive is a symmetric saturation (tanh) before the filter: exactly linear at 0, clearly distorting at 1.
   - Flush state below about 1e-20 to zero, so a silent filter never produces denormals. Flush it as a whole: zero
     both state variables of a stage together, once both are below the threshold; zeroing one of two coupled states
     on its own can keep the filter ringing near 1e-19 for ever.

3. **The envelope** (`core/include/undertow/envelope.h`, `core/src/envelope.cpp`)
   - Attack: from the current level up to 1 in a straight line, at `1 / (attackMs * sampleRate / 1000)` per sample.
   - Decay: towards the sustain level, exponentially, `level = sustain + (level - sustain) * c` with
     `c = exp(ln(0.01) / (decayMs * sampleRate / 1000))`, so 1% of the distance is left after `decayMs`; once within
     1e-5 of the sustain level, hold it exactly (Sustain stage).
   - Release: `level *= c` with the release time; below 1e-5 (-100 dB) the level is exactly 0 and the stage Idle.
   - `noteOn()` starts the attack from the current level (never from 0); `noteOff()` releases from the current
     level, from any stage but Idle. `process()` advances one sample and returns the new level.
   - Times are clamped to the parameter table's ranges (attack 0.5 .. 2000 ms, decay and release 1 .. 4000 ms).

4. **The mono voice** (`core/include/undertow/voice.h`, `core/src/voice.cpp`)
   - `setParams(values)` reads the parameter array (real units, indexed by `undertow::param`), as the engine holds it.
   - Pitch: the played note `n` (gliding, plus bend) is `440 * 2^((n - 69) / 12)` Hz; the oscillator plays that times
     `2^(osc_octave + osc_fine / 1200)`; the sub plays it divided by 2 (`-1 oct`) or 4 (`-2 oct`).
   - Mix: `amp_env * accentGain * (filter(osc * osc_level) + sub * sub_level)`. The sub is added after the filter:
     cutoff, resonance and drive never touch it.
   - Filter cutoff: `filter_cutoff * 2^(filter_env * filtEnv + accentOctaves + filter_track * (n - 36) / 12)`, where
     `filtEnv` is the filter envelope's level and `accentOctaves = 2 * accent * filtEnv` on an accented note.
   - Notes: last-note priority over a stack of up to 16 held notes. A note played while another is held is legato:
     no envelope restart. A note played with nothing held restarts both envelopes (attack from the current level).
     Releasing the sounding note returns, legato, to the newest note still held; releasing the last one releases
     the envelopes.
   - Glide: the pitch moves linearly in semitones to the new note over `glide_ms`. `glide_mode` Off never glides,
     Legato glides legato notes only, Always glides every note while the voice is still sounding (attack to
     release). `glide_ms` 0 never glides.
   - Accent: a note with velocity of 100/127 or more is accented: the level is multiplied by `1 + accent` and the
     filter envelope gets `2 * accent` extra octaves. Velocity has no other effect.
   - Pitch bend: `setPitchBend(-1 .. 1)` times `bend_range` semitones, on the oscillator and the sub.
   - Phase: a note that starts while the voice is idle resets the oscillator and the sub to phase 0 (every note from
     silence starts the same way); any other note keeps the phases running (no click).
   - Smoothing: `osc_level`, `sub_level`, `osc_pw`, `filter_res`, `filter_drive`, the accent gain and the cutoff (in
     log2 Hz) follow their targets through one-pole smoothers of about 5 ms. The first `setParams` after
     `prepare` jumps straight to the values.
   - `isActive()` is false once the amp envelope is Idle; `currentNote()` is the sounding (or releasing) note, -1
     when idle; `currentHz()` the played note's pitch now (glide and bend, without `osc_octave`/`osc_fine`), 0 when
     idle. An idle voice outputs exact zeros.

5. **The engine** (`core/src/engine.cpp`, `core/CMakeLists.txt`)
   - The engine owns one `MonoVoice`; `prepare` prepares it, `reset` resets it; each `process` call hands it the
     parameter array (`setParams`) and then, sample by sample, applies the MIDI events due at that sample before
     rendering it.
   - MIDI, on any channel: `0x9n` with velocity above 0 is note on (velocity / 127); `0x8n`, or `0x9n` with velocity
     0, is note off; `0xEn` is pitch bend (14 bits, centre 8192, to -1 .. 1); controller 123 releases all notes;
     controller 120 silences the voice at once (`reset`).
   - Output: both channels get the voice's sample times `master_gain` (dB to linear, smoothed); phase 2 adds the
     output stage after it.
   - Meters: `note` and `noteHz` from the voice after each block.
   - Add `src/oscillator.cpp`, `src/filter.cpp`, `src/envelope.cpp` and `src/voice.cpp` to `undertow_core`.

## API

```cpp
// core/include/undertow/oscillator.h
namespace undertow {
enum class Wave { Saw = 0, Square = 1 };

class Oscillator {                     // band-limited saw or square (PolyBLEP)
public:
    void prepare(double sampleRate);
    void reset(double phase = 0.0);    // phase in [0, 1)
    void setWave(Wave wave);
    void setFrequency(double hz);
    void setPulseWidth(double width);  // 0.05 .. 0.95, Square only
    float process();                   // the value at the current phase, then advance
    double phase() const;
};

class SineOscillator {                 // the sub
public:
    void prepare(double sampleRate);
    void reset(double phase = 0.0);
    void setFrequency(double hz);
    float process();                   // sin(2 pi phase), then advance
    double phase() const;
};
}  // namespace undertow

// core/include/undertow/filter.h
namespace undertow {
class LowpassFilter {                  // 24 dB/octave, -3 dB at the cutoff at resonance 0
public:
    void prepare(double sampleRate);
    void reset();
    void setCutoff(double hz);
    void setResonance(double resonance);  // 0 .. 1
    void setDrive(double drive);          // 0 .. 1, 0 = linear
    float process(float x);
};
}  // namespace undertow

// core/include/undertow/envelope.h
namespace undertow {
class Envelope {
public:
    enum class Stage { Idle, Attack, Decay, Sustain, Release };
    void prepare(double sampleRate);
    void reset();                       // Idle at 0
    void setAttackMs(double ms);
    void setDecayMs(double ms);
    void setSustain(double level);
    void setReleaseMs(double ms);
    void noteOn();
    void noteOff();
    float process();                    // advance one sample, return the new level
    float level() const;
    Stage stage() const;
};
}  // namespace undertow

// core/include/undertow/voice.h
#include "undertow/envelope.h"
#include "undertow/filter.h"
#include "undertow/oscillator.h"
#include "undertow/params.h"
namespace undertow {
class MonoVoice {
public:
    void prepare(double sampleRate);
    void reset();                         // idle, silent, no held notes
    void setParams(const float* values);  // kNumParams values, real units, indexed by undertow::param
    void noteOn(int note, float velocity);  // velocity 0 .. 1
    void noteOff(int note);
    void allNotesOff();                   // release, forget held notes
    void setPitchBend(float amount);      // -1 .. 1
    float process();                      // the next mono sample
    bool isActive() const;
    int currentNote() const;
    double currentHz() const;
};
}  // namespace undertow
```

Private members are yours. Keep the classes free of virtual calls and allocation.

## Acceptance checks

| Check | Command | Proves |
|---|---|---|
| P1.01 | `tools/spec.sh 1 oscillator 'P1.01*'` | The saw and square play the set frequency within 0.2% (55 Hz to 1.2 kHz) and the sine within 0.02 Hz |
| P1.02 | `tools/spec.sh 1 oscillator 'P1.02*'` | The saw and square are band-limited: at 1234.5 Hz the energy between harmonics below 4 kHz is 50 dB under the fundamental; the saw has 1/n harmonics |
| P1.03 | `tools/spec.sh 1 oscillator 'P1.03*'` | The square's pulse width works: no even harmonics and no DC at 50%, a strong 2nd harmonic and a mean of -0.5 at 25% |
| P1.04 | `tools/spec.sh 1 oscillator 'P1.04*'` | The sine is full scale and pure (THD -80 dB) and starts at the phase reset() gives it |
| P1.05 | `tools/spec.sh 1 filter 'P1.05*'` | At resonance 0 the filter is -3 dB at the cutoff (within 1.5 dB), flat two octaves below, -20 dB an octave above and -40 dB two octaves above |
| P1.06 | `tools/spec.sh 1 filter 'P1.06*'` | Resonance 0.8 raises a peak of at least 6 dB within 20% of the cutoff |
| P1.07 | `tools/spec.sh 1 filter 'P1.07*'` | Drive is linear at 0 and an odd-harmonic saturation at 1; a swept cutoff at full resonance and drive stays finite and within 4; a resonant tail decays with no denormals |
| P1.08 | `tools/spec.sh 1 envelope 'P1.08*'` | The attack rises in a straight line to 1 in exactly the attack time |
| P1.09 | `tools/spec.sh 1 envelope 'P1.09*'` | The decay leaves 10% of the distance at half the decay time and 1% at the decay time, then holds the sustain level exactly |
| P1.10 | `tools/spec.sh 1 envelope 'P1.10*'` | The release falls the same way to an exact 0 and Idle; a retrigger attacks from the current level, and a release during the attack starts where it got to |
| P1.11 | `tools/spec.sh 1 voice 'P1.11*'` | Notes play at their pitch with osc_octave and osc_fine; the sub sounds one or two octaves down, mixed after the filter, clean (THD -60 dB) whatever the filter does |
| P1.12 | `tools/spec.sh 1 voice 'P1.12*'` | Legato notes glide linearly over glide_ms without restarting the envelopes; the last held note wins; Off, Legato and Always glide modes behave as specified |
| P1.13 | `tools/spec.sh 1 voice 'P1.13*'` | Accent doubles the level at accent 1 and brightens the note; velocity does nothing else; pitch bend moves the oscillator and the sub by bend_range |
| P1.14 | `tools/spec.sh 1 voice 'P1.14*'` | A level change is smoothed (no one-sample jump over 0.02) and settles within 50 ms |
| P1.15 | `tools/spec.sh 1 voice 'P1.15*'` | Every note from silence starts identically (phases reset), without a click, and legato notes join without a jump |
| P1.16 | `tools/spec.sh 1 engine 'P1.16*'` | The engine plays MIDI on any channel: note on and off, velocity 0 as off, pitch bend, controllers 123 and 120; meters show the note and its pitch |
| P1.17 | `tools/spec.sh 1 engine 'P1.17*'` | Parameters set on the engine reach the voice (cutoff, sub octave), and master_gain scales the output |
| P1.18 | `tools/spec.sh 1 engine 'P1.18*'` | The engine makes no heap allocation over 10 s of notes, bends and parameter changes |
| P1.19 | `tools/spec.sh 1 engine 'P1.19*'` | 10 s of random notes and parameters stay finite and bounded; after the last note the output reaches exact zeros without denormals |
| P1.20 | `tools/spec.sh 1 engine 'P1.20*'` | 60 s of a busy, resonant, gliding bassline renders in at most 2.5 s of CPU time (24x real time) |

## Rules every task keeps

- The rules of phase 0 still hold: never edit `tests/spec/`, `third_party/`, `docs/` or the given tools; `core/` stays
  plain C++17; no allocation, lock, I/O or logging in anything `process` calls.
- Use `double` for phases and filter state; output `float`.
- Phase 0's checks must keep passing (`tools/check.sh`).

## Also required, not checked automatically

- Cutoff changes are smoothed too (in the log domain), so a fast knob turn does not zipper.
- The code is small and plain: one class per header, no inheritance, comments only for the DSP that needs them.
- Listening: a 110 Hz saw at cutoff 400 Hz, resonance 0.6, env +3 oct, decay 200 ms sounds like a techno bass, not
  a buzz; the sub alone sounds like a pure tone.

## Notes for the planner

- Five tasks, one per test file. Each task's verify steps are exactly the checks named for it, with their commands
  unchanged, and each passes once that task (and the ones it needs) is done:
  1. **Oscillators** (`oscillator.h`, `oscillator.cpp`): P1.01 to P1.04.
  2. **The filter** (`filter.h`, `filter.cpp`): P1.05 to P1.07.
  3. **The envelope** (`envelope.h`, `envelope.cpp`): P1.08 to P1.10.
  4. **The mono voice** (`voice.h`, `voice.cpp`): P1.11 to P1.15. Needs tasks 1 to 3.
  5. **The engine wiring** (`engine.cpp`, `core/CMakeLists.txt`): P1.16 to P1.20. Needs task 4.
  Tasks 1 to 3 can run in parallel. Each adds its own source to `core/CMakeLists.txt`; the engine task makes sure
  all four are listed.
- A test file is one executable, so a task owns a whole file's checks: the tests compile only once every class
  that file uses exists.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
- `tools/check.sh` (every present spec test, phases 0 and 1) is the plan's final check, not any one task's.
- The voice and engine tests start from `spec::cleanPatch()` and `spec::subPatch()` in
  `tests/spec/support/spec_engine.h`: read them to see the parameter values each test uses.
