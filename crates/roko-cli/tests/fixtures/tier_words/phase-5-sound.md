## Notes for the planner

- This spec is `docs/specs/phase-5-sound.md`. Put it first among the files every task reads: its "What to build" holds
  the rumble's design, the hits file's format and the presets' values the tests check exactly.
- Three tasks. Each task's verify step is exactly its check command, unchanged:
  1. **The rumble and the output's high-pass.** Architectural: about 280 lines.
     - Edits: `core/include/undertow/rumble.h`, `core/src/rumble.cpp`, `core/include/undertow/output_stage.h`,
       `core/src/output_stage.cpp`.
     - Its files are those 4 paths; it creates and deletes nothing.
     - Checks: P5.01 to P5.10, one step.
  2. **The hits and the renderer.** In parallel with task 1. Integrative: about 130 lines.
     - Edits: `core/include/undertow/engine.h`, `core/src/engine.cpp`, `tools/render/main.cpp`,
       `tools/render-demos.sh`.
     - Its files are those 4 paths; it creates and deletes nothing.
     - Checks: P5.11 to P5.14, one step.
  3. **The presets.** Needs tasks 1 and 2. Focused: about 90 lines.
     - Edits: `core/src/presets.cpp`.
     - Its files are that 1 path; it creates and deletes nothing.
     - Checks: P5.15 to P5.22, one step.
- `tools/check.sh --plugin` (P5.23) is the plan's final check, not a task's.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
