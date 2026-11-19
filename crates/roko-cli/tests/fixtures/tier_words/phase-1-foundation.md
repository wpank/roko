## Notes for the planner

- This spec is `docs/specs/phase-1-foundation.md`. Put it first among the files every task reads: its "What to
  build" and API sections hold what the tests and the PRD don't (the files to delete, what each kept file keeps).
- Three tasks. Task 1 comes first; tasks 2 and 3 both need it, and can run in parallel after it (they share no
  file, and the renderer builds with the core preset alone). Each task's verify steps are exactly the checks named
  for it, with their commands unchanged, and each passes once that task (and task 1) is done:
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
       targets (`tools/spec.sh` does): until tasks 2 and 3, the plugin and the renderer still name the first
       version's pattern and parameters, so a full build fails there, and they are those tasks' files.
  2. **The plugin re-keyed, and its install.** Needs task 1. Integrative: about 120 lines written and about 1,300
     removed; 7 files edited (its 9 deletions don't count towards the file budget).
     - Edits: `plugin/CMakeLists.txt`, `plugin/PluginProcessor.h`, `plugin/PluginProcessor.cpp`,
       `plugin/PluginEditor.h`, `plugin/PluginEditor.cpp`, `plugin/ui/Theme.h`, `tools/install.sh`.
     - Deletes: `plugin/ui/Displays.h`, `plugin/ui/Displays.cpp`, `plugin/ui/Look.h`, `plugin/ui/Look.cpp`,
       `plugin/ui/Status.h`, `plugin/ui/Status.cpp`, `plugin/ui/UnitArray.h`, `plugin/ui/UnitArray.cpp`,
       `plugin/ui/UiFrame.h`.
     - Its files are all 16 of those paths, the deleted ones included. `plugin/ui/Texts.h` stays as it is.
     - Checks: P1.10 to P1.17 (one step), P1.19.
     - `install.sh` belongs here because P1.16 runs it on the plugin's bundles.
     - Its first plugin build in a fresh worktree compiles the JUCE modules (about a minute at 4 jobs).
  3. **The renderer and the demo script.** Needs task 1. Focused: about 30 lines changed.
     - Edits: `tools/render/main.cpp`, `tools/render-demos.sh`.
     - Its files are those 2 paths; it creates and deletes nothing.
     - Checks: P1.18.
- `tools/check.sh --plugin` (P1.20) is the plan's final check, not a task's.
- P1.17 needs no grid: with the table's defaults every Float parameter comes back exactly. Its message names any
  parameter that doesn't.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
