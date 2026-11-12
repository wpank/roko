## Notes for the planner

- This spec is `docs/specs/phase-6-window.md`. Put it first among the files every task reads, and
  `docs/design/window.md` second. The spec holds the rules, the API and which task owns which file. The design holds
  every number. The tests check both, to the pixel.
- Six tasks. Task 1 and task 2 can run in parallel; tasks 3, 4 and 5 run in parallel after task 2 (task 5 also needs
  task 1); task 6 comes last. No two parallel tasks share a file: `plugin/PluginEditor.h` and `.cpp` and
  `plugin/ui/Theme.h` belong to task 2, and each widget's header to task 2 (the stub) and then its own task. Each
  task's verify step is exactly its check command, unchanged:
  1. **The spectrum and the scope.** Integrative: about 170 lines.
     - Creates: `core/include/undertow/spectrum.h`, `core/src/spectrum.cpp`.
     - Edits: `core/include/undertow/engine.h`.
     - Its files are those 3 paths; it deletes nothing.
     - Checks: P6.01 to P6.07, one step (`tools/spec.sh 6 spectrum`).
  2. **The frame.** Architectural: about 450 lines, most of them the design's data.
     - Edits: `plugin/CMakeLists.txt`, `plugin/ui/Theme.h`, `plugin/PluginEditor.h`, `plugin/PluginEditor.cpp`.
     - Creates: `plugin/ui/Frame.h`, `plugin/ui/Controls.h`, `plugin/ui/Steps.h`, `plugin/ui/Console.h` (the stubs).
     - Deletes: `plugin/ui/Displays.h`, `plugin/ui/Displays.cpp`, `plugin/ui/Look.h`, `plugin/ui/Look.cpp`,
       `plugin/ui/Status.h`, `plugin/ui/Status.cpp`, `plugin/ui/UnitArray.h`, `plugin/ui/UnitArray.cpp`,
       `plugin/ui/UiFrame.h`.
     - 8 files written; its 9 deletions don't count towards the file budget. Its files are all 17 of those paths,
       the deleted ones included. `plugin/ui/Texts.h` stays as it is.
     - Checks: P6.08 to P6.12, one step (`tools/spec.sh 6 frame`).
     - Its first plugin build in a fresh worktree compiles JUCE (about a minute at 4 jobs).
  3. **The controls.** Needs task 2. Integrative: about 420 lines.
     - Edits: `plugin/ui/Controls.h`.
     - Creates: `plugin/ui/Controls.cpp`.
     - Its files are those 2 paths; it deletes nothing.
     - Checks: P6.13 to P6.19, one step (`tools/spec.sh 6 controls`).
  4. **The step cells.** Needs task 2. Integrative: about 250 lines.
     - Edits: `plugin/ui/Steps.h`.
     - Creates: `plugin/ui/Steps.cpp`.
     - Its files are those 2 paths; it deletes nothing.
     - Checks: P6.20 to P6.25, one step (`tools/spec.sh 6 steps`).
  5. **The console.** Needs tasks 1 and 2. Architectural: about 420 lines.
     - Edits: `plugin/ui/Console.h`.
     - Creates: `plugin/ui/Console.cpp`.
     - Its files are those 2 paths; it deletes nothing.
     - Checks: P6.26 to P6.34, one step (`tools/spec.sh 6 console`).
  6. **The window as one, its budget and its review images.** Needs tasks 3, 4 and 5. Integrative: the images, and
     any fix the whole window's checks ask for.
     - Creates: `artifacts/ui/window-idle.png`, `artifacts/ui/window-playing.png`, `artifacts/ui/window-clipping.png`.
     - Edits, only if its checks ask: `plugin/PluginEditor.cpp`, `plugin/ui/Controls.cpp`, `plugin/ui/Steps.cpp`,
       `plugin/ui/Console.cpp`.
     - Its files are those 7 paths; it deletes nothing.
     - Checks: P6.35 to P6.38, one step (`tools/spec.sh 6 budget`). It commits the three images its check writes.
- `tools/check.sh --plugin` (P6.39) is the plan's final check, not a task's.
- A test file is one executable. Each plugin test waits for its own task's file (`spec-needs`), so tasks 3 to 5 never
  compile one another's checks.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
