# Fresh run dispatches a seeded completed task

- Severity: high
- Status: reproduced
- Area: task selection / plan status seeding

## Observation

The plan file declares `total = 16` and `done = 6`. At `13:05:19` the runner logged `seeded completed tasks from plan status tasks_completed=6`, but at `13:05:20` it selected E01-T01, whose task status is already `done`. After completing T01 again it jumped to E01-T07.

The seeding log originates near `crates/roko-cli/src/runner/event_loop.rs:3932`.

## Impact

- `--fresh` does not consistently honor task statuses from the supplied plan.
- Already-completed tasks consume gate time and may be mutated or committed again.
- Progress totals are internally inconsistent: the run knows six tasks are complete but presents T01 as active.

## Expected

Task selection should exclude plan tasks seeded as terminal-success before the first dispatch. If `--fresh` intentionally means rerun every task, it should not seed six completions or jump from T01 to T07.

