# Task verification depends on runner-owned mutable state

- Severity: medium
- Status: reproduced
- Area: plan authoring / gate isolation

## Observation

E01-T01's integration verification originally required:

```sh
grep -q 'default_value = "runner-v2"' crates/roko-cli/src/main.rs && test -s .roko/episodes.jsonl
```

The structural code checks passed, but this step failed repeatedly because the fresh isolated worktree did not contain `.roko/episodes.jsonl`. Agent retries eventually changed the verification command in the worktree to remove that condition; only then did preflight pass.

## Impact

- Gate results depend on whether a runner-owned data file happens to exist in a worktree.
- A task about a CLI default can trigger unrelated agent retries and plan-file mutation.
- Verification is not hermetic or reproducible from the checked-out source alone.

## Expected

Runtime acceptance should execute the command under test with an explicit temporary data directory, then assert on artifacts created by that invocation. Static task verification should not depend on ambient `.roko` state.

