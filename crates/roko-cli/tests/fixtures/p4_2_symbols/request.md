# Phase 4.2: the editor laid out as designed

**Goal**: the editor looks like `docs/design/ui-spec.md` and `docs/design/undertow-mock.svg`, not only like their
colours:
- each control in its panel, OSC/01 to OUT/08, as a labelled 40 px knob;
- values shown on hover, formatted;
- the pitch as the one bone number in the CHEST's lower third;
- the HEAD as the design draws it.

Phase 4's checks pass on an editor that does none of this. This phase's checks pin the layout down. It also brings
back the default bassline, which phase 4 lost: a new instance plays `defaultPattern()` again (audio-qa's F16).

**Depends on**: phase 4.1 (merged): phase 4's seven tasks with bone only for the number.

**Size**: M, 3 tasks, about 400 lines.

---

## Scope

In scope:
- `plugin/ui/Theme.h` (the panel table) and `plugin/ui/Texts.h` (new);
- `plugin/PluginEditor.h` and `plugin/PluginEditor.cpp`;
- `plugin/ui/Look.h`, `plugin/ui/Look.cpp`, `plugin/ui/Status.h`, `plugin/ui/Status.cpp`, `plugin/ui/Displays.h` and
  `plugin/ui/Displays.cpp`;
- the three review images, made again;
- in `core/src/engine.cpp`, the default pattern of a new engine.

Out of scope:
- the unit array (`plugin/ui/UnitArray.*`), whose checks pass;
- the tunnel, scope and AT field's drawing, whose checks pass;
- the rest of the engine, the processor and the parameters themselves.

## Context

- Phase 4's editor, as its review images show it (`artifacts/ui/editor-playing.png` at 04f86ab):
  - knobs as 104 x 72 ellipses with no labels, each with a text box of raw floats ("0.3500000", "600.0000610");
  - four knobs over the CHEST, and a column running past the RIGHT LIMB and the window's bottom edge, over the unit
    array;
  - panel stamps that aren't the design's ("OSC", "FLT", "AMP"), two inside the CHEST;
  - the number as the MIDI note ("33") with "55.0 HZ" under it, near the CHEST's top;
  - a HEAD reading "HEAD / HOST · 128 BPM · RUMBLE".
- A new instance has lost the default bassline (audio-qa's F16, `findings/audio-qa.md`):
  - Phase 3's `Engine::reset()` gave the sequencer `defaultPattern()`. Phase 4's plumbing task rightly removed that,
    so a pattern survives `prepare` and `reset` (P4.02), but nothing else gives the sequencer the default.
  - `Engine::Impl()` fills only the readback slot, and every `process` copies the sequencer's own pattern, all `Step{}`
    defaults, over it.
  - So a new engine with the sequencer on plays A1 on all 16 steps, and `pattern()` returns the empty pattern once
    audio has run. The editor's unit array shows it, and `getStateInformation` saves it.
  - P3.14 checks `pattern()` on a new engine before any block, and never renders, so it missed this.
- Phase 4's checks measure colours, pixels in zones and the hooks, never where a control or a word is. This phase
  checks the layout through the component tree: each control's bounds, its kind, its text box and its readout text
  (`Slider::getTextFromValue`). A new hook lists every word and number the editor draws, with its box. The checks then
  find in the snapshot the pixels each recorded word claims.
- The design: `docs/design/ui-spec.md` sections 2 (zones), 3 (type sizes), 4.1 (the knob), 4.2 (the bone number),
  4.9 (HEAD and GROUND) and 4.10 (the static layer, with the stamps); `docs/design/undertow-mock.svg` for the panels'
  rectangles and every label.
- The spec tests: `tests/spec/p4_2/layout_test.cpp` (task 1), `tests/spec/p4_2/head_test.cpp` (task 2) and
  `tests/spec/p4_2/pattern_test.cpp` (task 3), all plugin preset. They wait for `plugin/ui/Texts.h`, so
  `tools/check.sh` runs them from task 1 on. Change the code, never the tests.

## What to build

The declarations are in **API** below.

1. **The layout, the labels and the stamps** (`plugin/ui/Theme.h`, `plugin/ui/Texts.h`, `plugin/PluginEditor.h`,
   `plugin/PluginEditor.cpp`, `plugin/ui/Look.h`, `plugin/ui/Look.cpp`)
   - The panels and their controls, in this order, with these labels (the mock's):

     | Panel | Rectangle (x, y, w, h) | Controls (label) |
     |---|---|---|
     | OSC/01 | 16, 56, 280, 160 | osc_wave (WAVE), osc_pw (PW), osc_level (LEVEL), osc_octave (OCT), osc_fine (FINE), sub_level (SUB), sub_octave (SUB OCT) |
     | FILTER/02 | 16, 216, 280, 144 | filter_cutoff (CUTOFF), filter_res (RES), filter_drive (DRIVE), filter_env (ENV), filter_track (TRACK) |
     | ENV/03 | 16, 360, 280, 160 | amp_attack (A), amp_decay (D), amp_sustain (S), amp_release (R), filt_attack (F A), filt_decay (F D), filt_sustain (F S), filt_release (F R) |
     | RUMBLE/04 | 904, 56, 280, 264 | rumble_level (LEVEL), rumble_send (SEND), rumble_kick (KICK), rumble_rate (RATE), rumble_decay (DECAY), rumble_tone (TONE), rumble_drive (DRIVE), rumble_duck (DUCK); the pump field at 912, 80, 264, 96 above them |
     | MOD/05 | 904, 320, 280, 200 | lfo1_rate (1 RATE), lfo1_shape (1 SHAPE), lfo1_cutoff (1 CUT), lfo1_phase (1 PHASE), lfo2_rate (2 RATE), lfo2_shape (2 SHAPE), lfo2_dest (2 DEST), lfo2_depth (2 DEPTH), lfo2_phase (2 PHASE) |
     | GROUND | 0, 672, 1200, 88 | VOICE/07: glide_ms (GLIDE), glide_mode (MODE), accent (ACCENT), bend_range (BEND); OUT/08: sat_drive (SAT), width (WIDTH), mono_freq (MONO), master_gain (MASTER); the sequencer's: seq_on (SEQ), seq_sync (SYNC), seq_bpm (BPM), seq_rate (RATE), seq_length (LEN), seq_gate (GATE), seq_swing (SWING), seq_root (ROOT), seq_scale (SCALE), seq_mutate (MUTATE), seq_evolve (EVOLVE), seq_seed (SEED), left to right |

   - The knobs:
     - Each control sits in a 64 x 64 cell, its knob centred in the cell and its label under it. In the limbs the
       cells run four to a row, 68 px apart, from the panel's left edge plus 4. In GROUND they run in one row, 56 px
       apart, from x 40.
     - A knob is a rotary `juce::Slider` in square bounds, 32 to 64 px, drawn as a 40 px dial (4.1).
     - `seq_on` is a lamp button no larger than 64 x 40.
     - No control overlaps the CHEST, the pump field, the GUT or another control, and each lies inside its panel.
   - Each label is drawn in `text_dim`, 9.5 to 10 px sans caps. It is centred under its knob and starts within 24 px
     of the knob's bottom.
   - The readouts:
     - No knob shows a text box. On hover or drag its value appears under its label in `text_primary` mono (4.1).
     - The value is the slider's own text, `Slider::getTextFromValue`: a Choice's name; an Int as a whole number; a
       Float with at most two decimals, fewer as it grows.
     - Each value carries its unit from the parameter table: Hz (kHz from 1000), ms (s from 1000), s, dB, oct, st, ct,
       BPM.
     - The APVTS attachment sets JUCE's raw text, so set `textFromValueFunction` after making the attachment.
   - The stamps:
     - OSC/01, FILTER/02, ENV/03, RUMBLE/04 and MOD/05 sit at their panels' top left, and SEQ/06 at the GUT's.
     - VOICE/07 and OUT/08 sit over their GROUND groups.
     - All in `rose_dim` 10 px mono, none overlapping another.
   - `plugin/ui/Texts.h`, as in the API: every word and number the editor and its widgets draw goes through
     `texts::draw`, the static layer's included. `UndertowEditor::drawnTexts()` opens a record and paints the editor
     once offscreen, with the static layer drawn again inside the record (so `Look` needs a way to drop its cache).
     It then closes the record and returns it.
2. **The HEAD and the number** (`plugin/ui/Status.h`, `plugin/ui/Status.cpp`, `plugin/ui/Displays.h`,
   `plugin/ui/Displays.cpp`)
   - The HEAD (4.9):
     - The mark at the left: `METAKOSMIA` in `rose_dim`, and `⌈ U N D E R T O W ⌋` in `rose`, with wide tracking.
     - The clock in the middle, in `text_primary` mono: the tempo with one decimal (`128.0 BPM`), then the position as
       bar.beat.sixteenth from the frame's beat (`004.2.2` at beat 13.25: bar 4, beat 2, sixteenth 2).
     - After the clock, the badge: `HOST` in `rose` on the host's clock, `INT` in `dream` on the internal one.
     - At the right: a thin output meter (the peak, `rose` to `rose_bright`), and the lamp, `◉ LIVE` while the output
       is not silent, `○ LIVE` otherwise.
     - Only the meter and the lamp move with the output.
   - The number (4.2): the pitch with two decimals (`55.00`) in `bone` 56 px mono, `Hz` in `bone_dim` after it, and
     the note name (`A1`) in `bone_dim` 18 px under it, in the CHEST's lower third. It is never the MIDI note's number.
     Phase 4's motion stays: the 120 ms lerp, and the fade when idle.
   - Their words and numbers go through `texts::draw`.
   - The review images: run `tools/spec.sh 4 budget` and commit the three 1x images it writes to `artifacts/ui/`,
     since the layout and the HEAD have changed. The 2x copies in `build/previews/` stay uncommitted. With task 3 in,
     the unit array shows the default bassline.
3. **The default pattern plays** (`core/src/engine.cpp`)
   - A new engine's sequencer holds `defaultPattern()` from the start: set it in `Engine::Impl()`, beside the readback
     slot's `activePattern = defaultPattern();`. `Sequencer::reset()` keeps its pattern, so a pattern set before
     `prepare` or `reset` still survives them (P4.02).
   - Never set it in `reset()` or `prepare()`: that loses a pattern the host restored, or the user edited, as soon as
     the host prepares again.
   - Then a new instance plays the default bassline, `pattern()` reads it back once audio runs, and a new plugin saves
     it with its state.

## API

```cpp
// plugin/ui/Texts.h (new): header-only
#pragma once
#include <juce_gui_basics/juce_gui_basics.h>
#include <vector>
struct DrawnText {
    juce::String text;            // as drawn
    juce::Rectangle<int> bounds;  // the box it was drawn in, in editor coordinates
};
namespace texts {
std::vector<DrawnText>*& record();  // inline: the open record, or null (the message thread paints)
// inline: Graphics::drawText(text, box, justification) with the current font and colour, recorded while a record
// is open
void draw(juce::Graphics& g, const juce::String& text, juce::Rectangle<int> box, juce::Justification justification);
}  // namespace texts

// plugin/PluginEditor.h, added to UndertowEditor (with #include "ui/Texts.h")
std::vector<DrawnText> drawnTexts();  // one offscreen paint, the static layer drawn again; every word and number drawn
```

## Acceptance checks

Checks that share a command are one verify step: the table repeats the command on every row that command proves.

| Check | Command | Proves |
|---|---|---|
| P4.2.01 | `tools/spec.sh 4_2 layout` | Every control lies inside its own panel and the window, and clear of the CHEST, the pump field, the GUT and each other; in GROUND, VOICE/07 then OUT/08 then the sequencer's controls |
| P4.2.02 | `tools/spec.sh 4_2 layout` | Every knob is a rotary slider in square bounds of 32 to 64 px (a 40 px dial in a 64 x 64 cell), and seq_on is a lamp button within 64 x 40 |
| P4.2.03 | `tools/spec.sh 4_2 layout` | No knob shows a text box at rest; its readout at min, default and max is a choice's name, or a number with at most two decimals and its unit |
| P4.2.04 | `tools/spec.sh 4_2 layout` | Every control has the design's label, drawn under it inside its panel (and its pixels are there) |
| P4.2.05 | `tools/spec.sh 4_2 layout` | OSC/01 to MOD/05 and SEQ/06 at their panels' top left, VOICE/07 and OUT/08 over their GROUND groups, drawn and not overlapping |
| P4.2.06 | `tools/spec.sh 4_2 head` | The HEAD: METAKOSMIA and UNDERTOW at the left; 128.0 BPM, the position 004.2.2 and HOST (INT on the internal clock) in the middle; LIVE at the right; only the right moves with the output |
| P4.2.07 | `tools/spec.sh 4_2 head` | The number: 55.00 with Hz and A1 under it, bone, five characters wide, in the CHEST's lower third; 110.00 and A2 for note 45 |
| P4.2.08 | `tools/spec.sh 4 editor` | Phase 4's frame and control checks still pass (P4.03, P4.04). A regression check: it passes on the base too |
| P4.2.09 | `tools/spec.sh 4 display 'P4.05*'` | The panel stays half void, without white, rose dominant, bone rare. A regression check: it passes on the base too |
| P4.2.10 | `tools/spec.sh 4 steps` | The unit array still works: cells, hit-testing, the lit step, edits (P4.10). A regression check: it passes on the base too |
| P4.2.11 | `tools/spec.sh 4 display 'P4.06*,P4.08*,P4.09*'` | Bone only for the number, the tunnel and the scope still pass (P4.06, P4.08, P4.09). A regression check: it passes on the base too |
| P4.2.12 | `tools/spec.sh 4 display 'P4.07*'` | The hazard stripes still show while clipping, and only then (P4.07). A regression check: it passes on the base too |
| P4.2.13 | `tools/spec.sh 4 budget` | The frame budget holds, and the three review images are written again (P4.11, P4.12). A regression check: it passes on the base too |
| P4.2.14 | `tools/spec.sh 4_2 pattern` | A new engine with the sequencer on plays the default bassline: step 2 is A2 (45), step 5 rests, and the bar is the one an engine given defaultPattern() plays |
| P4.2.15 | `tools/spec.sh 4_2 pattern` | A new engine's pattern() is still defaultPattern() after a block, and with the sequencer running |
| P4.2.16 | `tools/spec.sh 4_2 pattern` | A new plugin that has run a block saves defaultPattern() in its state: restored, it gives the default back |
| P4.2.17 | `tools/spec.sh 4 state` | The pattern still survives prepare and reset, now also once the audio thread has taken it, and the state still saves it (P4.02). A regression check: it passes on the base too |
| P4.2.18 | `tools/spec.sh 3 engine 'P3.14*,P3.15*'` | A new engine still starts with defaultPattern(), and 60 s of the default patch still renders fast enough (P3.14, P3.15). A regression check: it passes on the base too |

## Rules every task keeps

- The rules of phases 0 to 4.1 still hold: never edit `tests/spec/`, `third_party/`, `docs/` or the given tools; no
  white, no colour as decoration, bone only for the one number.
- Every word and number the editor draws goes through `texts::draw`: the checks read the record and the pixels
  together.

## Also required, not checked automatically

- The hover readout: under the label, in `text_primary` mono, while the mouse is over the knob or drags it.
- The judged list in `docs/design/ui-spec.md` section 7, now with the layout: dense margins, sparse centre, labels
  legible at 100% and on Retina, nothing cramped or clipped.

## Notes for the planner

- This spec is `docs/specs/phase-4.2-layout.md`. Put it first among the files every task reads: What to build holds
  the rules and API the exact declarations. The tests and `docs/design/ui-spec.md` do not say where each control and
  word goes, nor how the checks read them.
- Three tasks, one per test file. Each task's verify steps are exactly the checks named for it, with their commands
  unchanged:
  1. **The layout, the labels and the stamps** (`plugin/ui/Theme.h`, `plugin/ui/Texts.h`, `plugin/PluginEditor.h`,
     `plugin/PluginEditor.cpp`, `plugin/ui/Look.h`, `plugin/ui/Look.cpp`): P4.2.01 to P4.2.05 in one step, then
     P4.2.08, P4.2.09 and P4.2.10 (`expect = "pass_on_base"`). About 300 lines over 6 files, so architectural.
  2. **The HEAD and the number** (`plugin/ui/Status.h`, `plugin/ui/Status.cpp`, `plugin/ui/Displays.h`,
     `plugin/ui/Displays.cpp` and the three `artifacts/ui/*.png`): P4.2.06 and P4.2.07 in one step, then P4.2.11,
     P4.2.12 and P4.2.13 (`expect = "pass_on_base"`). Needs tasks 1 and 3: task 1 for `texts::draw` and
     `drawnTexts()`, task 3 so the images show the default bassline. About 100 lines. It commits the three images its
     budget step writes.
  3. **The default pattern plays** (`core/src/engine.cpp`): P4.2.14 to P4.2.16 in one step, then P4.2.17 and P4.2.18
     (`expect = "pass_on_base"`). About 1 line. It needs no task and shares no file with task 1, so the two run in
     parallel.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
- `tools/check.sh --plugin` is the plan's final check, not any one task's: every present spec test of phases 0 to 4.2,
  both presets, and pluginval at strictness 5.
