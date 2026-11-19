# Phase 4.1: bone only for the one number

**Goal**: in phase 4's editor, bone (`#C8B890`) and bone_dim (`#8A7A5A`) mark the pitch in the CHEST and nothing
else. The knob ticks, the knob thumb, tooltips, the clock readout and the rumble label go back to the ROSEDUST tokens
that `docs/design/ui-spec.md` gives them.

**Depends on**: phase 4's seven tasks (`roko/plan/p4-undertow-phosphor-editor`, a16b906), whose plan check failed
only P4.06.

**Size**: S, 1 task, about 5 lines and the three review images.

---

## Scope

In scope: colours in `plugin/ui/Look.cpp` and `plugin/ui/Status.cpp`, and the review images phase 4 committed
(`artifacts/ui/editor-idle.png`, `editor-playing.png` and `editor-clipping.png`), made again. No new files, no new
parameters, no change to any header, layout, size, font or behaviour.

Out of scope: the bone number itself (`plugin/ui/Displays.cpp`) and every other part of the editor.

## Context

- The rule: "One bone number. The pitch the voice is playing (`55.00 Hz` with `A1` under it). Nothing else is bone"
  (`docs/design/ui-spec.md` section 1, rule 3). Phase 4 lists it among the rules every task keeps, but its tasks never
  saw it: roko's context limit cut it from their prompts.
- P4.06 checks it. Before any note, at most 1 pixel in 10,000 is bone-like (`bone` or `bone_dim`, as
  `tests/spec/support/spec_ui.h` `boneLike` measures). While a note plays, every bone-like pixel lies in the CHEST
  (312, 56, 576, 464), in one place. On phase 4's tasks it fails: 1,145 bone pixels before any note, and while a note
  plays they span 61, 17, 1058 x 715, most of the editor.
- Where the bone comes from:
  - `plugin/ui/Look.cpp`: `drawRotarySlider` draws each knob's tick at the value in `boneDim`, and
    `Slider::thumbColourId` and `TooltipWindow::textColourId` are `boneDim` too.
  - `plugin/ui/Status.cpp`: the HEAD's BPM readout is drawn in `bone`, and the pump field's `RUMBLE` label in `bone`
    at 0.88.
- The spec tests: phase 4's `tests/spec/p4/display_test.cpp`. Change the code, never the tests.

## What to fix

1. **Bone only for the one number** (`plugin/ui/Look.cpp`, `plugin/ui/Status.cpp`)
   - The knob's tick at the value, and `Slider::thumbColourId`: `rose`, as the value arc (`docs/design/ui-spec.md`
     4.1).
   - Tooltip text: `text_primary`, the colour of a knob's value on hover.
   - The HEAD's clock readout (the BPM): `text_primary` (4.9).
   - The pump field's `RUMBLE` label: `rose_dim`, the colour of labels and corner stamps.
   - Nothing else changes. Bone and bone_dim stay in `plugin/ui/Displays.cpp`'s bone number alone.
   - The review images: phase 4's three `artifacts/ui` PNGs show the bone, and they are what people see first. Run
     `tools/spec.sh 4 budget` after the change and commit the three 1x images it writes there; the 2x copies it writes
     to `build/previews/` stay uncommitted.

## API

No change.

## Acceptance checks

Checks that share a command are one verify step: the table repeats the command on every row that command proves.

| Check | Command | Proves |
|---|---|---|
| P4.06 | `tools/spec.sh 4 display 'P4.06*,P4.08*,P4.09*'` | No bone before any note; while a note plays, bone only in the CHEST, in one place. The tunnel, scope and AT field checks (P4.08, P4.09) still pass |
| P4.05 | `tools/spec.sh 4 display 'P4.05*'` | The panel stays at least half void, without white, rose dominant (its knob panels too), bone rare, and its CHEST calm at rest. A regression check: it passes on the base too |
| P4.07 | `tools/spec.sh 4 display 'P4.07*'` | The hazard stripes show while the output clips, and only then: without clipping the top band stays quiet. A regression check: it passes on the base too |
| P4.11-P4.12 | `tools/spec.sh 4 budget` | A full repaint still keeps 60 Hz with headroom, and the three review images are written again: artifacts/ui at 1x under 2 MB, build/previews at 2x. It passes on the base too; on this task it also remakes the images the task commits |

## Rules every task keeps

- The rules of phases 0 to 4 still hold: never edit `tests/spec/`, `third_party/`, `docs/` or the given tools.
- Colours only, with tokens from `plugin/ui/Theme.h`: no layout, size, font or behaviour changes.

## Also required, not checked automatically

- Looking: at rest the panel shows no bone; while a note plays, the eye finds the pitch in the CHEST first.
- The tokens named above. The checks see only where bone is, not what replaced it: a clock in `rose_bright` would
  pass them.

## Notes for the planner

- This spec is `docs/specs/phase-4.1-bone.md`. Put it first among the files the task reads: What to fix names each bone
  use and the token it becomes. The tests and `docs/design/ui-spec.md` do not say where the bone is.
- One task, about 5 lines: **Bone only for the one number** (`plugin/ui/Look.cpp`, `plugin/ui/Status.cpp` and the
  three images `artifacts/ui/editor-idle.png`, `artifacts/ui/editor-playing.png`, `artifacts/ui/editor-clipping.png`).
  Its verify steps are exactly the four checks above, with their commands unchanged: P4.06 (it fails on the base),
  then P4.05, P4.07 and P4.11-P4.12 (`expect = "pass_on_base"`). It commits the three images its budget step writes.
- The task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` checks
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
- `tools/check.sh --plugin` is the plan's final check, not the task's: every present spec test of phases 0 to 4, both
  presets, and pluginval at strictness 5.
