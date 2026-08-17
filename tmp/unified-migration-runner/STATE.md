# Runner State (v2 Parallel)

> **Last updated: 2026-08-13**

## What is this?

This file records the last execution state of the parallel-agent migration runner
(`run.sh` in this directory). The runner executes batches from `MASTER-CHECKLIST.md`
across 4 Claude agents, each owning a partition of the crate graph. This state file
is used for `--continue` resume support.

---

| Field | Value |
|---|---|
| Run ID | `run-20260426-080451` |
| Mode | `parallel` |
| Agents | `4` |
| Source branch | `wp-arch2` |
| Model | `claude-opus-4-6` |
| Last updated | 2026-04-26T08:32:25+02:00 |

## Resume

```bash
bash tmp/unified-migration-runner/run.sh --continue run-20260426-080451
# or
bash tmp/unified-migration-runner/run.sh --continue last
```
