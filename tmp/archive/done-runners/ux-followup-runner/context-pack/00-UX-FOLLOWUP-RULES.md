# UX Follow-up Runner — Common Rules (read first)

You are running as an unattended Codex batch from `tmp/ux-followup-runner`.

## Core rules

1. **No prior chat.** This prompt pack must be self-sufficient.
2. **Repository reality only.** Work from files that exist in the worktree.
   If a file, symbol, or path may have moved or disappeared, verify the
   current tree with `rg --files` / `rg -n` and stop rather than guessing.
3. **Compile clean.** The verify gate per batch lists the exact `cargo`
   commands that must pass. They run in the runner worktree; a failure of any
   single command aborts the batch and triggers a retry.
4. **Write-scope discipline.** Stay inside the batch's write scope. If you need
   a 1-line adjacent fix for the batch to compile, make it; document it in the
   final message.
5. **Subagents authorised.** Every batch includes a "Delegation Requirement"
   section; use explorers and workers aggressively. Do not block on subagents
   when you can progress locally.
6. **Match the PR-13 bar.** Commit messages already land as
   `tui-parity(Tnn): <title>` for the sibling runner; this runner commits as
   `ux-followup(UXnn): <title>`. The trailer includes catalog refs.
7. **No destructive git.** Never force-push, reset main, rm -rf outside the
   worktree's own `.cargo-target/target`. The runner handles branch/worktree
   lifecycle.
8. **No skipping verification.** `-D warnings` means what it says. Suppressing
   with `#[allow]` is only acceptable when the rule is genuinely wrong for
   the change you made; prefer the real fix.
9. **Defer, do not delete.** If a batch's scope turns out too broad, finish
   the highest-leverage slice, leave a precise follow-up note in the last
   message, and let the runner move on. Do not leave uncompilable code.
10. **Respect tier conventions.** Naming renames (`grimoire → neuro`,
    `styx → Korai`, `clade → fleet`, no "mortal/death" concepts) are in force
    for all live docs. `bardo-backup/` is read-only and must not be mutated;
    if a warning note is needed, emit a sidecar artifact under `tmp/`.

## Batch completion bar

A batch is only complete when:

- the listed tasks are implemented
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

## Delegation etiquette

When spawning subagents:
- give each worker a **disjoint** write scope
- tell each worker they are **not alone** in the codebase
- pass the same context pack (these 6 files) to every subagent
- reassemble their work; do not merge half-complete outputs

## Context paths

Always read these files before coding:

1. `tmp/ux-followup-runner/context-pack/00-UX-FOLLOWUP-RULES.md` (this file)
2. `tmp/ux-followup-runner/context-pack/01-CATALOG-MAP.md`
3. `tmp/ux-followup-runner/context-pack/02-WORKSPACE-TOPOLOGY.md`
4. `tmp/ux-followup-runner/context-pack/03-STATE-FLOW.md`
5. `tmp/ux-followup-runner/context-pack/04-SAFETY-LAYER.md`
6. `tmp/ux-followup-runner/context-pack/05-MORI-REFERENCE-APPENDIX.md`

## Authoritative catalogue

Every batch closes one or more items in `tmp/ux-followup/` (files 01–15).
The batch prompt lists them under "Catalog refs:". If you discover a gap not
covered by the current batch scope, record it as a follow-up note — do not
silently extend scope.

## Environment invariants

- Rust toolchain ≥ 1.91 (`rustup update stable` required for `alloy` deps)
- `.roko/` and `tmp/` are gitignored at repo root (per `59835802`)
- Workspace root: `/Users/will/dev/nunchi/roko/roko`
- PR #13 (`5ff264c9`) is merged; T1–T19 TUI-parity work landed in PR #13
- CLAUDE.md "What to work on" items 10 + 11 remain open until this runner
  closes UX01 + UX02
