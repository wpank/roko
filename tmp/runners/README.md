# `tmp/runners` — parallel batch runners

Each subdirectory with a `run.sh` is a **self-contained runner**: `batches.toml`, `prompts/*.prompt.md`, optional `context-pack/`, and a thin `run.sh` that delegates to `parallel-template/run-parallel.sh`.

| Runner | Notes |
|--------|--------|
| [binary-issues](binary-issues/) | Remaining `tmp/binary-issues/MASTER-INDEX.md` items (56 batches, `ISSUE-TRACKER.md`) |
| [post-parity](post-parity/) | Post–mega-parity maturation (330 batches) |
| [mega-parity](mega-parity/) | Large parity sweep |
| [converge-followup](converge-followup/) | Converge follow-up (waves A–F, `BATCHES.md`) |
| [productionizing](productionizing/) | Production hardening batches |
| [solutions](solutions/) | Solutions-oriented batches |
| [audit-2026-05-01](audit-2026-05-01/) | Dated audit runner |
| [perf](perf/) | Performance-focused batches |
| [ux-impl](ux-impl/) | UX implementation batches |

Shared machinery: [parallel-template](parallel-template/) (DAG scheduler, worktrees, gates).

```bash
bash tmp/runners/<runner>/run.sh --list
bash tmp/runners/<runner>/run.sh --dry-run
bash tmp/runners/<runner>/run.sh --parallel 16
```

Other folders here (`arch/`, `converge/`) are supporting material or older layouts without a top-level `run.sh`; use the table above for runnable runners.
