# ux-impl Runner

Runs the 12 implementation plans from `tmp/ux/implementation-plans/`
as Codex-driven batches against a fresh git worktree.

| File | Purpose |
|------|---------|
| `ISSUE-TRACKER.md` | Master checklist; one row per batch / manual step |
| `BATCHES.md` | Per-wave batch summary with deps |
| `batches.toml` | The runner's batch registry (consumed by `run.sh`) |
| `run.sh` | Thin wrapper around `tmp/runners/parallel-template/run-parallel.sh` |
| `prompts/<id>.prompt.md` | One mechanical Codex prompt per Rust batch |
| `context-pack/` | Shared context injected into every prompt |
| `manual-tracks/` | Plans 03 / 09 / 11 / 12 — non-Rust or hand-only steps |
| `fixtures/` | Captured legacy mirage-rs response fixtures (created by AG01) |
| `logs/` | Created at runtime by `run.sh` |

## Quick start

```bash
# See what's there.
bash tmp/runners/ux-impl/run.sh --list

# Plan only (dry-run produces prompts but doesn't dispatch Codex).
bash tmp/runners/ux-impl/run.sh --dry-run

# Run one wave at a time, pausing between for manual inspection.
bash tmp/runners/ux-impl/run.sh --group AG --pause

# Run a specific batch.
bash tmp/runners/ux-impl/run.sh --only AG02

# Resume the last run after a failure or interrupt.
bash tmp/runners/ux-impl/run.sh --continue

# Status of the latest run.
bash tmp/runners/ux-impl/run.sh --status
```

The dispatch layer is the existing `parallel-template` runner under
`tmp/runners/parallel-template/`. See its `README.md` for the full set
of flags, env vars, and DAG semantics. This runner only adds: a batch
registry (`batches.toml`), prompts, a context pack, and an issue
tracker.

## Sequencing (recommended)

Two parallelisable lanes per `tmp/ux/implementation-plans/00-INDEX.md`.

```
Lane A (dashboard story):  AG  →  DB(M)  →  M
                                 →  CH

Lane B (hardening):        RH(M) ┐
                           DC(M) ├→ HY  →  BP  →  MC
                           FG    ┘
```

`TU` is independent and can run in either lane. `PH` is parked.

## What this runner does **not** do

- **It does not run the manual `(M)` tracks.** Those need a human (or a
  non-Rust runner). See `manual-tracks/<plan>/CHECKLIST.md` for each.
- **It does not auto-merge to a feature branch.** It merges into a
  per-run `codex/ux-impl-<run-id>` branch. The operator inspects the
  diff and merges to their working branch when satisfied.
- **It does not run on stale checkouts.** Each run forks from `HEAD` (or
  `--base-ref`). Pull / rebase before kicking off a long run.

## When a batch fails

1. Read `logs/run-*/[batch].log` for the Codex transcript.
2. Read `logs/run-*/[batch].verify.log` for the AP-grep / cargo-check
   output.
3. If the failure is recoverable (a missing import, a typo), tweak the
   prompt and re-run with `--only <id>`.
4. If the failure is structural (the plan's assumption is wrong),
   update the plan in `tmp/ux/implementation-plans/<plan>.md` first,
   then this prompt. The plan and the prompt must agree.
5. Mark the batch `[~]` in `ISSUE-TRACKER.md` until it lands cleanly.

## How prompts are composed

Each prompt sent to Codex is the concatenation of:

1. `context-pack/00-RULES.md` (workspace coding rules)
2. `context-pack/01-ARCHITECTURE.md` (current state of the 5-layer model)
3. `context-pack/02-ANTI-PATTERNS.md` (mistakes to avoid)
4. `context-pack/03-EXECUTION-STRATEGY.md` (how to read a batch prompt)
5. The cumulative-context snapshot of files modified by previously-
   succeeded batches in this run.
6. The live contents of every file in the batch's `scope` and
   `also_read`.
7. `prompts/<id>.prompt.md` (the batch-specific instructions).

Keep `context-pack/*` lean — every byte is multiplied by the number of
batches and by the number of attempts.
