# Execution Strategy

How to read a batch prompt and turn it into a clean commit.

## The shape of every prompt

```
## Goal
  one-sentence statement of the change

## Context
  files you're touching, how they sit in the architecture

## Changes Required
  numbered, mechanical list of edits with file paths and (where
  available) line anchors

## Acceptance Criteria
  checklist a reviewer can run

## Do NOT
  scope guards specific to this batch

## Reference
  related plan section in tmp/ux/implementation-plans/<file>.md
```

Read every section before touching code. Especially "Do NOT".

## Workflow

1. **Read the prompt** end to end.
2. **Read the scope files** referenced under `Changes Required`. Every
   file in the batch's `scope` (per `batches.toml`) is also injected
   into your context.
3. **Plan the edits** mentally — which sections of which files change,
   and in what order. Compile errors in step 4 are cheaper than
   structural rewrites in step 5.
4. **Apply edits** using strict text replacement. Avoid wholesale
   rewrites; preserve unrelated code.
5. **Compile-check** the affected crates: `cargo check -p <crate>`.
6. **Read your own diff** before declaring done. Walk every chunk and
   ask: does this match the prompt's "Changes Required"? Is anything
   outside scope?
7. **Run the acceptance checklist** in your head. If something is
   unverifiable from inside Codex (e.g. UI behaviour), state that in
   the commit message.

## What "scope" means

Every batch in `batches.toml` lists `scope = [...]`. These are the
**only** files you may modify. The runner re-resets the worktree
between batches; any change outside scope is silently lost — and
worse, fails the anti-pattern grep, which sees inconsistent state.

`also_read` files are read-only context. Do not modify them.

If the prompt's "Changes Required" mention a file not in `scope`, that
is a bug in the batch definition. Stop and report in the commit
message; the next agent will redefine the batch.

## What "verify = quick" means

The runner runs `cargo check -p <crate>` (per crate touched in
`scope`) plus an anti-pattern grep over the changed files. It does
not run `cargo test` or `cargo clippy --workspace`. Those run at the
wave gate — at which point a regression *across* crates surfaces.

So: even if your batch passes verify, the wave gate may flag it. The
fix is a follow-up batch with a small `also_read` window into the
breaking call site.

## Reading the plans

Every wave has a corresponding plan in
`tmp/ux/implementation-plans/<NN>-<name>.md`. The plan has more
context than any single batch prompt — required reading lists,
deliverables across the whole wave, anti-patterns in narrative form,
"Done when" criteria for the wave as a whole.

When a prompt feels under-specified, check the plan. The prompt is the
mechanical instruction; the plan is the rationale.

## When you encounter drift

The plans were written 2026-04-14 / 2026-04-22 / 2026-05-01. The code
moves daily. If a referenced file path or line number is wrong:

1. **Don't pretend it isn't.** Open the file as it actually exists.
2. **Apply the spirit of the change.** The plan explains intent; map
   the intent onto the current shape.
3. **Note the drift in the commit message.** Format:
   `drift: <plan> says X at <path>:<line>, actually Y at <path>:<line>`.
4. **If the drift makes the change unsafe** (e.g. the plan assumes a
   field that no longer exists), abort the batch and surface the
   contradiction. The wave reviewer redefines the batch.

## Cumulative context

After every successful batch, the runner snapshots the files it
modified and adds them to a "cumulative context" prepended to
subsequent prompts. This means you see what the previous batch
actually shipped, not what the prompt described.

If the cumulative context shows a previous batch produced something
unexpected (e.g. introduced an empty function it shouldn't have), call
that out in your commit message and either (a) work around it, or (b)
abort.

## Commit message contract

```
<batch-id>: <short title>

- bullet 1: what file changed and why
- bullet 2: ...

Drift / surprises: <one line per item, or "none">
Acceptance: <copy of the prompt's checklist with [x] / [ ]>
Closes: ISSUE-TRACKER.md row <batch-id>
```

The runner extracts the `<batch-id>: <short title>` line into git log;
the rest helps the reviewer.

## When to stop

Stop and surface a problem (rather than improvising) when:

1. The plan and the actual code disagree in a way that affects
   correctness (not just line numbers).
2. The change would require modifying a file outside `scope`.
3. A foundation type would have to be duplicated to satisfy the
   prompt.
4. A wave-gate-level concern surfaces (the change works inside one
   crate but obviously breaks a downstream one).
5. You'd need to add a new dependency that's not justified by the
   prompt.

In all five cases, write the commit anyway with a `WIP: blocked` tag
in the title. The runner's verify will fail, the row stays `[ ]`, and
the reviewer redefines the batch.
