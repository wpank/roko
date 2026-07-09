# Docs-Parity Runner — Common Rules (read first)

You are running as an unattended Codex batch from `tmp/docs-parity2`.
Your job: make the **code** match the **docs**. The docs are the source of truth
for what types, traits, functions, and behaviors should exist.

## Core rules

1. **No prior chat.** This prompt pack must be self-sufficient.
2. **Repository reality only.** Work from files that exist in the worktree.
   If a file, symbol, or path may have moved, verify with `rg --files` / `rg -n`
   and stop rather than guessing.
3. **Compile clean.** The verify gate per batch lists the exact `cargo` commands
   that must pass. A failure of any single command aborts the batch and triggers
   a retry.
4. **Write-scope discipline.** Stay inside the batch's write scope. If you need
   a 1-line adjacent fix for the batch to compile, make it; document it in the
   final message.
5. **Subagents authorised.** Use explorers and workers aggressively. Do not
   block on subagents when you can progress locally.
6. **Commit message format.** The runner commits as
   `docs-parity2(DPnn): <title>`. Do not commit yourself.
7. **No destructive git.** Never force-push, reset main, rm -rf outside the
   worktree. The runner handles branch/worktree lifecycle.
8. **No skipping verification.** `-D warnings` means what it says. Suppressing
   with `#[allow]` is only acceptable when the rule is genuinely wrong.
9. **Defer, do not delete.** If a batch's scope turns out too broad, finish
   the highest-leverage slice, leave a precise follow-up note, and let the
   runner move on. Do not leave uncompilable code.
10. **Naming conventions.** `neuro` (not grimoire), `Korai` (not styx/Styx),
    `fleet` (not clade). No death/mortality language. `bardo-backup/` is
    read-only.

## Batch completion bar

A batch is only complete when:

- the listed gaps are closed (types/traits/functions added)
- the batch's verify gate passes in the runner worktree
- any new files are wired into their parent module tree (`mod.rs` exports)
- new public types carry `///` doc comments
- the commit (made by the runner, not you) lands on the batch branch

## Failure behaviour

If a batch is too large:
- finish the highest-dependency work first
- leave a precise note in the final message listing what remains
- do not stop at analysis
- do not leave half-compiling code

## Context paths

Always read these files before coding:

1. `tmp/docs-parity2/context-pack/00-DOCS-PARITY-RULES.md` (this file)
2. `tmp/docs-parity2/context-pack/01-SECTION-CRATE-MAP.md`
3. `tmp/docs-parity2/context-pack/02-WORKSPACE-TOPOLOGY.md`
4. `tmp/docs-parity2/context-pack/03-EXISTING-PARITY-SUMMARY.md`
5. `tmp/docs-parity2/context-pack/04-CODE-CONVENTIONS.md`
6. `tmp/docs-parity2/context-pack/05-PHASE2-STUB-GUIDANCE.md`

## Environment invariants

- Rust toolchain >= 1.91 (`rustup update stable` required for `alloy` deps)
- `.roko/` and `tmp/` are gitignored at repo root
- Workspace root: `/Users/will/dev/nunchi/roko/roko`
