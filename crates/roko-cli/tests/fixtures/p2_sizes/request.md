# Phase 2: rumble and modulation

**Goal**: the hypnotic part of the sound. Two tempo-synced LFOs locked to the beat position, the rumble (a ghost
kick into a dark reverb, low-passed, saturated and ducked on the beat), saturation, a mono-sub crossover with width
above it, and a soft clipper that holds every sample under -0.3 dBFS.

**Depends on**: phase 1 (the voice in the engine).

**Size**: L, 4 tasks, about 900 lines.

---

## Scope

In scope: `undertow/lfo.h`, `undertow/rumble.h`, `undertow/output_stage.h` with their `core/src/*.cpp`; a
`VoiceModulation` input on `MonoVoice`; the engine wiring; `core/CMakeLists.txt` registering the core sources by glob.

Out of scope: the sequencer (phase 3) and the editor (phase 4). `seq_*` parameters other than `seq_bpm` and
`seq_seed` do nothing yet.

## Context

- Sound-design goals: `docs/PRD.md` sections 1.3 (rumble), 1.4 (hypnotic motion) and 1.5 (level and safety); the
  signal flow is the diagram in section 2; the beat clock is section 3.4 (phase 0 built it).
- The spec tests: `tests/spec/p2/lfo_test.cpp`, `rumble_test.cpp`, `output_test.cpp` and `engine_test.cpp`. Change the
  code, never the tests. Phase 0 and 1's tests must keep passing (`tools/check.sh`).
- Phase 1's tests already set every phase 2 stage to neutral (rumble level 0, saturation 0, width 0, LFO depths 0,
  master gain 0 dB) and keep their levels at or under 0.5, where the clipper is exactly transparent: wiring the
  stages in must not change what they measure.

## What to build

1. **The LFO** (`core/include/undertow/lfo.h`, `core/src/lfo.cpp`), and the core sources by glob (`core/CMakeLists.txt`)
   - `core/CMakeLists.txt` builds the core library from every `src/*.cpp`, found by
     `file(GLOB … CONFIGURE_DEPENDS …)`, instead of a list, so this phase's other sources (and later phases') join the
     library without editing it.
   - `lfoCycleBeats(rate)` gives each `lfo*_rate` choice's cycle in beats: 16, 8, 4, 2, 1, 0.5, 0.25, 0.125, 2/3, 1/3,
     1/6, 0.75, 1.25, 1.75, 3, 5, 7 (PRD section 4).
   - `valueAt(ppq)` is a pure function of the beat position: the phase is `frac(ppq / cycleBeats + phaseOffset)`.
     Shapes at phase `p`: Sine `sin(2 pi p)`; Triangle `4p` below 0.25, `2 - 4p` below 0.75, else `4p - 4`; Saw Down
     `1 - 2p`; Saw Up `2p - 1`; Square `+1` below 0.5, else `-1`; Sample & Hold a value in -1 .. 1 from a well-mixed
     hash of (seed, cycle number `floor(ppq / cycleBeats + phaseOffset)`), held for the whole cycle.
   - No state: the same beat position always gives the same value, so a host loop replays the same motion.

2. **The rumble** (`core/include/undertow/rumble.h`, `core/src/rumble.cpp`)
   - `process(send, ppq, bpm, running)` makes one mono sample. `running` says whether the beat clock moves.
   - The ghost kick fires each time `ppq` crosses a multiple of the `rumble_rate` grid (1/4: 1 beat, 1/2: 2, 1 bar:
     4, 1/8: 0.5) while running, and on the first running sample when the clock starts or jumps (a loop) exactly on a
     grid point (within half a sample). Never while stopped.
   - The kick: a sine whose pitch falls from 150 to 45 Hz (`45 + 105 exp(-t / 30 ms)`) under an `exp(-t / 80 ms)`
     envelope, 300 ms long, times `rumble_kick`. It feeds the reverb only; it is never heard dry.
   - The reverb input is the kick plus `send` (the engine passes the voice times the rumble send). A mono reverb
     whose tail falls 60 dB in `rumble_decay` seconds: a four-line feedback delay network (delays of 31 to 62 ms,
     Householder feedback, per-line gain `10^(-3 delay / decay)`, a one-pole low-pass of about 1.5 kHz in each line)
     does it. Size its delay lines in `prepare`.
   - Then a 24 dB/octave low-pass at `rumble_tone`, saturation `tanh(k x) / k` with `k = 1 + 9 rumble_drive`, and a
     12 dB/octave low-pass at twice `rumble_tone`.
   - The duck: after each kick the gain falls linearly to `1 - rumble_duck` within 2 ms, then recovers as
     `1 - rumble_duck * exp(-beats / (0.2 * grid))`, where `beats` counts from the end of the 2 ms; `duckGain()`
     reports it (1 before the first kick).
   - The output is the low-passed, saturated tail times the duck gain times `rumble_level` (exact zeros at level 0).

3. **The output stage** (`core/include/undertow/output_stage.h`, `core/src/output_stage.cpp`)
   - `softClip(x)`: `x` for `|x| <= 0.5`; above, `sign(x) * (0.5 + 0.466 * tanh((|x| - 0.5) / 0.466))`, never above 0.966.
   - `process(mono, left, right)`, in order:
     - a DC blocker: a one-pole high-pass at about 5 Hz, whose state flushes to exact zero once the input is silent
       (a narrow pulse's offset would otherwise reach both channels through the low band and make the saturation and
       the clipper asymmetric);
     - saturation `tanh(g x) / g` with `g = 1 + 3 sat_drive` (bypassed at 0; quiet signals pass at unity gain and loud
       peaks are rounded down, by up to 12 dB at full drive, which `master_gain` makes up);
     - `master_gain` (dB, smoothed over a few milliseconds: an 18 dB step never jumps a sample, and settles within
       50 ms);
     - a 4th-order Linkwitz-Riley crossover at `mono_freq` (two cascaded Butterworth low-passes and high-passes),
       always running; the low band goes to both channels; the high band gets width: with `theta = width * pi / 4`
       and `d` the high band delayed by 12 ms, `left = low + cos(theta) * high + sin(theta) * d` and
       `right = low + cos(theta) * high - sin(theta) * d`;
     - then `softClip` on each channel.
   - `clipped()` is true for 0.5 s after any channel goes over 1.0 before the clipper.

4. **Voice modulation** (`core/include/undertow/voice.h`, `core/src/voice.cpp`)
   - Add `VoiceModulation` and `MonoVoice::setModulation`. The voice adds `cutoffOctaves` to its cutoff,
     `resonance` to `filter_res` (clamped 0 .. 1), `pulseWidth` to `osc_pw` (clamped 0.05 .. 0.95), `pitchSemitones`
     to the played pitch (oscillator and sub), and multiplies its output by `gain`. The default is no modulation.

5. **The engine** (`core/src/engine.cpp`)
   - Per sample: the beat position `ppq` (with a valid transport, `transport.ppq` advanced at the host's tempo while it
     plays, held while stopped; without one, the internal clock at `seq_bpm`), `running` (valid and playing, or no
     valid transport at all), and the tempo (the host's, or `seq_bpm`).
   - LFO 1 adds `lfo1_cutoff * value` octaves to the cutoff. LFO 2 (`lfo2_depth` d, value v) acts on `lfo2_dest`:
     Cutoff `4 d v` octaves; Resonance `0.5 d v`; Pulse Width `0.45 d v`; Rumble Send `d v` added to `rumble_send`
     (clamped 0 .. 1); Pitch `12 d v` semitones; Amp a gain of `1 - d + d (v + 1) / 2`. Phase offsets from
     `lfo*_phase`; Sample & Hold seeds `2 seq_seed + 1` and `2 seq_seed + 2`.
   - Then `mono = voice + rumble.process(voice * send, ppq, tempo, running)` and `output.process(mono, left, right)`.
     The master gain moves from the engine into the output stage.
   - `master_gain`'s default becomes -12 dB, in `core/src/params.cpp` (the PRD's table says so too). With every
     parameter at its default, accented notes then peak near -8 dBFS, below the clipper's knee, instead of over full
     scale.
   - Meters: `clipping` from the output stage; `rumble`, the rumble bus RMS over about 50 ms.
   - Controller 120 (all sound off) now also resets the rumble and the output stage, so the output is silent at once
     (phase 1's check P1.16 requires it now that the crossover always runs).
   - Add `src/lfo.cpp`, `src/rumble.cpp` and `src/output_stage.cpp` to `undertow_core`.

## API

```cpp
// core/include/undertow/lfo.h
#include <cstdint>
namespace undertow {
enum class LfoShape { Sine = 0, Triangle, SawDown, SawUp, Square, SampleHold };
constexpr int kLfoRates = 17;
double lfoCycleBeats(int rate);              // the lfo*_rate choice's cycle in beats
class Lfo {
public:
    void setRate(int rate);                  // a choice index, 0 .. 16
    void setShape(LfoShape shape);
    void setPhaseOffset(double offset);      // a fraction of a cycle
    void setSeed(std::uint32_t seed);        // Sample & Hold
    double valueAt(double ppq) const;        // -1 .. 1
};
}  // namespace undertow

// core/include/undertow/rumble.h
namespace undertow {
class Rumble {
public:
    void prepare(double sampleRate);         // allocates the reverb
    void reset();                            // silence the tail, open the duck
    void setParams(const float* values);     // the rumble_* parameters; kNumParams values, real units
    float process(float send, double ppq, double bpm, bool running);
    double duckGain() const;                 // the duck's gain now, 1 = open
};
}  // namespace undertow

// core/include/undertow/output_stage.h
namespace undertow {
float softClip(float x);                     // identity to 0.5, a tanh knee to 0.966
class OutputStage {
public:
    void prepare(double sampleRate);
    void reset();
    void setParams(const float* values);     // sat_drive, width, mono_freq, master_gain
    void process(float mono, float& left, float& right);
    bool clipped() const;                    // over full scale in the last 0.5 s
};
}  // namespace undertow

// core/include/undertow/voice.h, added
namespace undertow {
struct VoiceModulation {
    double cutoffOctaves = 0.0;
    double resonance = 0.0;
    double pulseWidth = 0.0;
    double pitchSemitones = 0.0;
    double gain = 1.0;
};
// in class MonoVoice:
void setModulation(const VoiceModulation& modulation);
}  // namespace undertow
```

## Acceptance checks

Checks that share a command are one verify step: the table repeats the command on every row that command proves.

| Check | Command | Proves |
|---|---|---|
| P2.01 | `tools/spec.sh 2 lfo 'P2.01*'` | The 17 rate choices are the right cycle lengths in beats, polyrhythms included |
| P2.02 | `tools/spec.sh 2 lfo 'P2.02*'` | The six shapes have their values at quarter points; the phase offset shifts them; Sample & Hold holds a seeded value per cycle |
| P2.03 | `tools/spec.sh 2 lfo 'P2.03*'` | valueAt is a function of the beat position: every rate repeats on its cycle and 5/16 and 3/16 realign with the bar after 5 and 3 bars |
| P2.04 | `tools/spec.sh 2 rumble 'P2.04*'` | The ghost kick and the duck follow the beat grid: the duck is down within 5 ms of each beat and open again by 3/4 of a beat; no kick while stopped |
| P2.05 | `tools/spec.sh 2 rumble 'P2.05*'` | Fed by the ghost kick, and separately by a bright send (white noise), at least 90% of the rumble's energy is below 200 Hz at tone 110 Hz, at drive 0.4 and at drive 1 |
| P2.06 | `tools/spec.sh 2 rumble 'P2.06*'` | The tail falls from -10 to -40 dB in half of rumble_decay (within 35%), for 2 s and 6 s |
| P2.07 | `tools/spec.sh 2 rumble 'P2.07*'` | The ghost kick is never heard dry, the send feeds the reverb, and rumble_level 0 is exact silence |
| P2.08 | `tools/spec.sh 2 rumble 'P2.08*'` | At full settings with full-scale noise sent in, the rumble stays finite and within 2 and never allocates; after a stop its tail ends in exact zeros, no denormals |
| P2.09 | `tools/spec.sh 2 output 'P2.09*,P2.10*'` | softClip is exactly transparent to 0.5, odd, rising, gives 0.869 at 1.0 and never passes 0.966 |
| P2.10 | `tools/spec.sh 2 output 'P2.09*,P2.10*'` | Saturation is linear at 0, distorts a 0.4 sine by 5% or more at 1 with odd harmonics and no DC, and keeps quiet signals at unity gain |
| P2.11 | `tools/spec.sh 2 output 'P2.11*,P2.12*'` | Width 0 leaves both channels equal and the bands summing flat; width keeps the band under mono_freq mono and decorrelates the band above |
| P2.12 | `tools/spec.sh 2 output 'P2.11*,P2.12*'` | The clip meter lights when the signal goes over full scale and goes out 0.5 s later; the output never passes the ceiling |
| P2.13 | `tools/spec.sh 2 engine 'P2.13*,P2.14*,P2.23*'` | LFO 1 moves the cutoff in time with the host's bar (peak at ppq 1 for a 1-bar sine), replays after a host loop, and runs on the internal clock without a host |
| P2.14 | `tools/spec.sh 2 engine 'P2.13*,P2.14*,P2.23*'` | LFO 2 reaches pitch (+-12 semitones), amp (full to silent), pulse width and cutoff |
| P2.15 | `tools/spec.sh 2 engine 'P2.15*,P2.16*'` | In the engine the mono rumble pumps on the host's beats, kicks again after a loop to ppq 0, and runs on the internal clock without a host |
| P2.16 | `tools/spec.sh 2 engine 'P2.15*,P2.16*'` | At width 1 the full mix keeps the side signal 25 dB under the mid below mono_freq / 2, and spreads the band above 500 Hz |
| P2.17 | `tools/spec.sh 2 engine 'P2.17*,P2.22*'` | A hot patch (no saturation, master +6 dB) never passes 0.966, stays finite, lights meters().clipping, and the meter clears once quiet |
| P2.18 | `tools/spec.sh 2 engine 'P2.18*,P2.19*'` | With LFOs, rumble, saturation and width on, and their parameters moving, the engine never allocates |
| P2.19 | `tools/spec.sh 2 engine 'P2.18*,P2.19*'` | 60 s of the full engine renders in at most 4 s of CPU time (15x real time) |
| P2.20 | `tools/spec.sh 2 output 'P2.20*,P2.21*'` | master_gain is smoothed: an 18 dB step on a held sine never jumps a sample (four steps at different phases), and the level settles within 50 ms |
| P2.21 | `tools/spec.sh 2 output 'P2.20*,P2.21*'` | A DC blocker: a constant 0.4 at the input leaves the output's mean within 0.01 after 0.2 s, and a 30 Hz sine keeps its level within 0.5 dB |
| P2.22 | `tools/spec.sh 2 engine 'P2.17*,P2.22*'` | With every parameter at its default, accented notes (velocity 127) on A1, A2 and A3 peak between -12 and -6 dBFS: below the clipper's knee |
| P2.23 | `tools/spec.sh 2 engine 'P2.13*,P2.14*,P2.23*'` | A 10% pulse swept by LFO 2 (pulse width) leaves the output's mean within 0.01 in every quarter second |

## Rules every task keeps

- The rules of phases 0 and 1 still hold: never edit `tests/spec/`, `third_party/`, `docs/` or the given tools;
  `core/` stays plain C++17; no allocation, lock, I/O or logging in anything `process` calls.
- Every stage at its neutral setting (level 0, drive 0, depth 0, width 0) must leave phase 1's sound alone.
- Flush filter and reverb state below about 1e-20 to zero, so a long silent tail ends in exact zeros and never makes
  denormals (phase 1's P1.19 checks the whole engine for it). Flush a filter's state as a whole: set both of a
  biquad's state variables to zero only when both are below the threshold. Zeroing one of two coupled states on its
  own keeps the filter ringing near 1e-19 for ever.

## Also required, not checked automatically

- LFO 2 on Resonance and on Rumble Send work (they are wired the same way as the checked destinations).
- Listening: at 128 BPM with the defaults, a held A1 rolls: the rumble swells between the beats and drops on them,
  the bass stays centred, and the top of the saw spreads a little at width 0.25.
- The width delay and the reverb lines are sized for the highest sample rate a host may use (192 kHz).

## Notes for the planner

- Four tasks, one per test file. Each task's verify steps are exactly the checks named for it, with their commands
  unchanged, and each passes once that task (and the ones it needs) is done:
  1. **The LFO, and sources by glob** (`lfo.h`, `lfo.cpp`, `core/CMakeLists.txt`): P2.01 to P2.03. It replaces the
     core library's source list with a glob, so a task adds a source without editing `core/CMakeLists.txt`:
     `file(GLOB undertow_core_sources CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp")` and
     `add_library(undertow_core STATIC ${undertow_core_sources})`. `CONFIGURE_DEPENDS` makes a build re-glob, so a new
     file joins even an already-configured build tree.
  2. **The rumble** (`rumble.h`, `rumble.cpp`): P2.04 to P2.08. Needs task 1. About 200 lines, so architectural.
  3. **The output stage** (`output_stage.h`, `output_stage.cpp`): P2.09 to P2.12, P2.20 and P2.21, in three verify
     steps. Needs task 1. About 160 lines, so architectural.
  4. **Voice modulation and the engine wiring** (`voice.h`, `voice.cpp`, `engine.cpp`, `params.cpp` for the master
     default): P2.13 to P2.19, P2.22 and P2.23, in four verify steps. Needs tasks 1 to 3. About 80 lines.
  Tasks 2 and 3 run in parallel after task 1, and only add files. Only task 1 lists `core/CMakeLists.txt` in its
  files: no other task edits it.
- A test file is one executable, so a task owns a whole file's checks: the tests compile only once every class
  that file uses exists.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
- `tools/check.sh` (every present spec test, phases 0 to 2) is the plan's final check, not any one task's. Phases 0
  and 1's checks must keep passing.
- The rumble tests drive `Rumble` directly at 128 BPM (22500 samples a beat at 48 kHz); read `rumble_test.cpp` for the
  parameter values.
