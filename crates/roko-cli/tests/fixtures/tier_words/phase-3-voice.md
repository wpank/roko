## Notes for the planner

- This spec is `docs/specs/phase-3-voice.md`. Put it first among the files every task reads: its "What to build"
  holds the order of the voice's sample and the formulas the tests measure against (the half-cosine rise, the
  restart rule, the thud, the filter's swing, the attack times).
- Three tasks, in this order. Each task's verify step is exactly its check command, unchanged:
  1. **The envelope.** Integrative: about 80 lines.
     - Creates: `core/include/undertow/decay.h`, `core/src/decay.cpp`.
     - Its files are those 2 paths; it edits and deletes nothing.
     - Checks: P3.01 to P3.05, one step.
  2. **The voice.** Needs task 1. Architectural: about 200 lines.
     - Creates: `core/include/undertow/voice.h`, `core/src/voice.cpp`.
     - Its files are those 2 paths; it edits and deletes nothing.
     - Checks: P3.06 to P3.14, one step.
  3. **The engine plays the voice.** Needs task 2. Integrative: about 50 lines.
     - Edits: `core/src/engine.cpp`.
     - Its files are that 1 path; it creates and deletes nothing.
     - Checks: P3.15 to P3.19, one step.
- `tools/check.sh --plugin` (P3.20) is the plan's final check, not a task's.
- Every task builds C++: its agent needs `bash` and `edit_file`, to run `cmake` and its own `tools/spec.sh` check
  before it hands off. Leave `allowed_tools` out, or include `bash`, `edit_file`, `read_file`, `write_file`, `grep`
  and `glob`.
