# Phase 4.3: the editor's finish

**Goal**: fix what phase 4.2's review images show and its checks did not catch:
- every panel's knobs fitting it, where the mock puts them, MOD/05's nine included;
- the words as the design writes them, nothing garbled, and the HEAD on one line;
- 16 px clip bands with their legend, and the clock and the values readable while the output clips;
- `Hz` after the number on its baseline, with the number below the scope;
- the rumble pump field back in RUMBLE/04.

**Depends on**: phase 4.2.

**Size**: S, 2 tasks, about 150 lines and the three review images.

---

## Scope

In scope:
- the limb panels' rows in `plugin/ui/Theme.h`;
- `plugin/ui/Status.h`, `plugin/ui/Status.cpp` and `plugin/ui/Displays.cpp`;
- one line in `UndertowEditor::drawnTexts()` (`plugin/PluginEditor.cpp`);
- the three review images, made again.

No new files, and no change to any API.

Out of scope: everything else in the editor, the engine and the processor.

## Context

- Phase 4.2's review images (`artifacts/ui/*.png` at 27daceb) show:
  - **Knobs off the mock:**
    - OSC/01's rows sit 34 and 22 px below the mock's (`docs/design/undertow-mock.svg`), leaving the panel's top
      third empty. ENV/03's sit 34 and 26 px low, and FILTER/02's first row 18 px low.
    - RUMBLE/04's knobs sit where the mock has them. The empty top half of that panel is the missing pump field (below).
  - **Squeezed rows:**
    - In MOD/05, `2 RATE` sits 12 px below its row, and `2 PHASE` is squeezed in under `2 DEPTH`'s label at 32 px,
      in the fourth column. The mock has it as a full knob under `2 RATE`.
    - In FILTER/02, `TRACK`'s dial sits 2 px under `CUTOFF`'s label.
    - ENV/03's second-row labels end 2 px above the panel's edge.
  - **Garbled words:** the mark reads `â U N D E R T O W â` and the lamp `â LIVE`. `Status.cpp` passes UTF-8 literals
    through `juce::String(const char*)`, which reads each byte as a character of its own.
  - **The HEAD on two lines:** `METAKOSMIA` sits above the mark. The mock has `METAKOSMIA ⌈ U N D E R T O W ⌋`, the
    clock and the lamp on one line.
  - **24 px clip bands** (`expanded(0, 8)`) with no legend. They cover the HEAD's words and GROUND's labels while the
    output clips.
  - **`Hz` left of and below the number.** The number itself crosses the scope's trace: its digits start at y 356 and
    the scope ends at 388.
  - **No pump field:** RUMBLE/04's display (912, 80, 264, 96) is empty. Phase 4.2's HEAD rewrite dropped it.
- The spec tests:
  - `tests/spec/p4_3/panels_test.cpp` (task 1) and `tests/spec/p4_3/finish_test.cpp` (task 2), both plugin preset.
  - They read the editor's controls, its `drawnTexts()` and its snapshots.
  - They join `tools/check.sh` as soon as they are on main, so check.sh fails on main until this phase's tasks land.
  - Change the code, never the tests.

## What to build

1. **The panels fit** (`plugin/ui/Theme.h`)
   - **Each knob in the limbs sits where the mock puts it**, its centre within 8 px of the mock's.
     - The columns stay phase 4.2's.
     - The rows' cells (a 64 px cell starts 32 px above its knob's centre):

       | Panel | The mock's row centres (y) | Cells' y | Their centres (y) |
       |---|---|---|---|
       | OSC/01 | 98, 162 | 70, 130 | 102, 162 |
       | FILTER/02 | 258, 322 | 230, 290 | 262, 322 |
       | ENV/03 | 402, 466 | 374, 434 | 406, 466 |
       | RUMBLE/04 | 210, 274 | 180, 248, as now | 212, 280 |
       | MOD/05 | 360, 424, 488 | 334, 392, 456 | 366, 424, 488 |

     - Each first row sits 4 to 6 px below the mock's, clear of its panel's stamp: MOD/05's stamp box ends at y 342.
     - MOD/05 runs LFO 1's four knobs, then LFO 2's `2 RATE`, `2 SHAPE`, `2 DEST` and `2 DEPTH`, then `2 PHASE` alone
       in the first column under `2 RATE`, where the mock has it. All nine are full 40 px knobs.
   - **The controls in a row share their centre** within 3 px.
   - **A label keeps 4 px clear of every control but its own.**
   - **Every control and label lies 4 px inside its panel.**
2. **The HEAD, the clip bands, the number and the pump field** (`plugin/ui/Status.h`, `plugin/ui/Status.cpp`,
   `plugin/ui/Displays.cpp`, `plugin/PluginEditor.cpp`)
   - **Words with non-ASCII characters** go through `juce::String::fromUTF8` or `juce::CharPointer_UTF8`:
     - the mark, `⌈ U N D E R T O W ⌋`;
     - the lamp, `◉ LIVE` while the output plays and `○ LIVE` while it is silent;
     - the clip legend, `⌈ CLIP ⌋  OUTPUT -0.3 dBFS`.
   - **The HEAD's words sit on one line**, their centres within 4 px of each other, below the top hazard band (y 16
     and down): `METAKOSMIA`, the mark, the clock (tempo, position, badge) and the lamp.
   - **The clip bands** (`docs/design/ui-spec.md` 4.8):
     - They are 16 px each, y 0 to 15 and 744 to 759.
     - A `bg_void` plate in the middle of the top band carries the legend in `rose_bright` mono.
     - `drawnTexts()` records the words drawn over the controls too: it calls `status_.paintOver(g)` after
       `paintControlText(g)`, as `paintOverChildren` does. Then the legend is in the record.
   - **Readable while clipping:**
     - The design's dual reading (`docs/design/ui-spec.md` 1.7) keeps every value and the clock legible up close,
       alarm or not.
     - So while the output clips, the HEAD's words and GROUND's labels change no pixels outside the bands.
   - **The number** (4.2):
     - `Hz` follows the digits, 8 px after them, on their baseline: draw both from one baseline, each box at
       `baseline - font.getAscent()`.
     - The note name sits under them.
     - All of it lies below the scope's bottom edge, not across its trace.
   - **The pump field** (4.6):
     - It fills RUMBLE/04's display, `pumpFieldBounds` (912, 80, 264, 96).
     - It holds the last 32 beats, newest on the right, each beat split into 24 slices.
     - A cell's colour is the rumble level there along Theme.h's `heatRamp` (`bg_void`, `rose_deep`, `rose_ember`,
       `rose`). A cell with no rumble stays dark.
     - `advance` records `frame.rumble` at the frame's beat and slice.
   - **The review images:** run `tools/spec.sh 4 budget` and commit the three 1x images it writes to
     `artifacts/ui/`. The 2x copies in `build/previews/` stay uncommitted.

## API

No change.

## Acceptance checks

Checks that share a command are one verify step: the table repeats the command on every row that command proves.

| Check | Command | Proves |
|---|---|---|
| P4.3.01 | `tools/spec.sh 4_3 panels` | In every panel, the controls of a row share their centre within 3 px |
| P4.3.02 | `tools/spec.sh 4_3 panels` | Every label keeps 4 px clear of every control but its own |
| P4.3.03 | `tools/spec.sh 4_3 panels` | Every control and label lies 4 px inside its panel |
| P4.3.04 | `tools/spec.sh 4_3 panels` | Each knob in the limbs sits where the mock puts it, its centre within 8 px |
| P4.3.05 | `tools/spec.sh 4_3 finish` | No drawn word is garbled; the mark reads ⌈ U N D E R T O W ⌋ and the lamp ◉ LIVE or ○ LIVE exactly; the HEAD's words sit on one line |
| P4.3.06 | `tools/spec.sh 4_3 finish` | While clipping, the bands are 16 px with the legend ⌈ CLIP ⌋  OUTPUT -0.3 dBFS in the top band's middle, and the HEAD's words and GROUND's labels keep their pixels outside the bands |
| P4.3.07 | `tools/spec.sh 4_3 finish` | Hz follows 55.00 on its baseline (within 3 px), and the number sits below the scope |
| P4.3.08 | `tools/spec.sh 4_3 finish` | After 8 beats of rumble, the pump field lights at least 4% of RUMBLE/04's display; without rumble it stays dark |
| P4.3.09 | `tools/spec.sh 4_2 layout` | Phase 4.2's layout checks still pass: panels, knobs, readouts, labels, stamps. A regression check: it passes on the base too |
| P4.3.10 | `tools/spec.sh 4 editor` | Phase 4's frame and control checks still pass (P4.03, P4.04). A regression check: it passes on the base too |
| P4.3.11 | `tools/spec.sh 4 display 'P4.05*'` | The panel stays half void, without white, rose dominant, bone rare. A regression check: it passes on the base too |
| P4.3.12 | `tools/spec.sh 4 steps` | The unit array still works (P4.10). A regression check: it passes on the base too |
| P4.3.13 | `tools/spec.sh 4_2 head` | Phase 4.2's HEAD and number checks still pass. A regression check: it passes on the base too |
| P4.3.14 | `tools/spec.sh 4 display 'P4.06*,P4.08*,P4.09*'` | Bone only for the number, the tunnel and the scope still pass. A regression check: it passes on the base too |
| P4.3.15 | `tools/spec.sh 4 display 'P4.07*'` | The hazard stripes still show while clipping, and only then (P4.07). A regression check: it passes on the base too |
| P4.3.16 | `tools/spec.sh 4 budget` | The frame budget holds, and the three review images are written again (P4.11, P4.12). A regression check: it passes on the base too |

## Rules every task keeps

- The rules of phases 0 to 4.2 still hold:
  - never edit `tests/spec/`, `third_party/`, `docs/` or the given tools;
  - no white, no colour as decoration, and bone only for the one number;
  - every word goes through `texts::draw`.
- No new files.

## Also required, not checked automatically

- The judged list in `docs/design/ui-spec.md` section 7.

## Notes for the planner

- This spec is `docs/specs/phase-4.3-finish.md`. Put it first among the files every task reads. What to build gives
  each row, word and band its place; the tests and `docs/design/ui-spec.md` do not.
- Two tasks, one per test file. Each task's verify steps are exactly the checks named for it, with their commands
  unchanged:
  1. **The panels fit** (`plugin/ui/Theme.h`): P4.3.01 to P4.3.04 in one step, then P4.3.09 to P4.3.12
     (`expect = "pass_on_base"`). About 40 lines.
  2. **The HEAD, the clip bands, the number and the pump field** (`plugin/ui/Status.h`, `plugin/ui/Status.cpp`,
     `plugin/ui/Displays.cpp`, `plugin/PluginEditor.cpp` and the three `artifacts/ui/*.png`): P4.3.05 to P4.3.08 in
     one step, then P4.3.13 to P4.3.16 (`expect = "pass_on_base"`).
     - It needs task 1, so its images show the fitted panels.
     - About 120 lines.
     - It commits the three images its budget step writes.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
- `tools/check.sh --plugin` is the plan's final check, not any one task's: every present spec test of phases 0 to 4.3,
  both presets, and pluginval at strictness 5.
