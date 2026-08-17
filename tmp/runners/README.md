# `tmp/runners` — parallel batch runners

> **What is this?** Infrastructure for running parallel Claude/Codex agent batches against
> isolated git worktrees. Each runner is a self-contained set of batch definitions, prompts,
> and context packs. The shared machinery in `parallel-template/` handles DAG scheduling,
> worktree creation, anti-pattern checks, and merge-back. These runners were the primary
> mechanism for landing the ~2000+ tasks that brought roko from prototype to self-hosting
> (April-July 2026). Most runners have completed their work; they remain as reference
> material and can be re-used for future batch work.
>
> **Last updated: 2026-08-13**

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

> **Note (2026-08-13):** Many of these runners reference `orchestrate.rs` in their prompts
> and context packs. That file has been deleted; the runner event loop
> (`crates/roko-cli/src/runner/event_loop.rs`) is now the sole plan-execution entry point.
> The Engram-to-Signal rename is also complete (`pub type Signal = Engram` in roko-core).
> Runner prompts in git history may still use the old names.
