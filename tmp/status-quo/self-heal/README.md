# Roko self-heal plans

These plans convert the `tmp/status-quo/issues` audit into executable repair work. Run them in numeric order. Keep `--max-tasks 1` until SH02 proves concurrency and task-owned worktree isolation.

Do not combine `--fresh` and `--resume-plan`. Prefer a fresh SH01 run after archiving or preserving the existing dirty E01 worktree.

Validation and preview:

```sh
target/debug/roko plan validate --strict tmp/status-quo/self-heal/plans
target/debug/roko plan run tmp/status-quo/self-heal/plans --engine runner-v2 --max-tasks 1 --dry-run
```

Run the complete dependency-ordered self-heal graph with one task at a time:

```sh
target/debug/roko plan run tmp/status-quo/self-heal/plans --engine runner-v2 --max-tasks 1 --fresh --approval
```

Resume an interrupted plan with:

```sh
target/debug/roko plan run tmp/status-quo/self-heal/plans --engine runner-v2 --max-tasks 1 --resume-plan .roko/state/state-snapshot.json --approval
```
