## Notes for the planner

- This spec is `docs/specs/phase-4-sequencer.md`. Put it first among the files every task reads: its "What to build"
  holds the drift's recipe and the clock's rules the tests check exactly.
- Three tasks, in this order. Each task's verify step is exactly its check command, unchanged:
  1. **The pattern and its drift.** Integrative: about 80 lines.
     - Creates: `core/include/undertow/pattern.h`, `core/src/pattern.cpp`.
     - Its files are those 2 paths; it edits and deletes nothing.
     - Checks: P4.01 to P4.05, one step.
  2. **The sequencer.** Needs task 1. Integrative: about 140 lines.
     - Creates: `core/include/undertow/sequencer.h`, `core/src/sequencer.cpp`.
     - Its files are those 2 paths; it edits and deletes nothing.
     - Checks: P4.06 to P4.11, one step.
  3. **The engine plays the pattern.** Needs task 2. Focused: about 30 lines.
     - Edits: `core/include/undertow/engine.h`, `core/src/engine.cpp`.
     - Its files are those 2 paths; it creates and deletes nothing.
     - Checks: P4.12 to P4.18, one step.
- `tools/check.sh --plugin` (P4.19) is the plan's final check, not a task's.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
