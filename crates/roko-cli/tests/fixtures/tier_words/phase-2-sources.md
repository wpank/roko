## Notes for the planner

- This spec is `docs/specs/phase-2-sources.md`. Put it first among the files every task reads: its "What to build"
  holds the formulas the tests measure against (the waves' alignment, the square's PolyBLEP, the sub's lock, the
  ladder's per-sample solution).
- Two tasks, which can run in parallel: they write different files and their checks build different executables.
  1. **The oscillators and the noise.** Integrative: about 150 lines.
     - Creates: `core/include/undertow/vco.h`, `core/src/vco.cpp`, `core/include/undertow/noise.h`,
       `core/src/noise.cpp`.
     - Its files are those 4 paths; it edits and deletes nothing.
     - Checks: P2.01 to P2.09, one step.
  2. **The ladder.** Integrative: about 110 lines.
     - Creates: `core/include/undertow/ladder.h`, `core/src/ladder.cpp`.
     - Its files are those 2 paths; it edits and deletes nothing.
     - Checks: P2.10 to P2.17, one step.
- `tools/check.sh --plugin` (P2.18) is the plan's final check, not a task's.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
