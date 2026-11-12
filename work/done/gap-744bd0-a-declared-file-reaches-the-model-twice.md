+++
id = "gap-744bd0"
kind = "gap"
title = "A declared file reaches the model twice: the prompt holds it, and the implementer role's rule 1 (\"Read ALL files listed in the task's read_files and files fields\") has the agent read it again in its first turn, about 8% of phase 4.3's run"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_policy"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "491f649b6"
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 9, run 3752b8ff), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-compose/src/templates/implementer.rs::IMPLEMENTER_ROLE_IDENTITY", "crates/roko-cli/src/plan_policy.rs::DECLARED_FILES_INTRO"]
links = { depends_on = ["bug-c5f406"], blocks = [], related = ["bug-a8a787", "gap-b46c16"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn implementer_works_from_its_declared_files' crates/roko-compose/src && cargo test -p roko-compose implementer_works_from_its_declared_files"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T20:59:39Z"
commit = "d7ec5e487"
by = "roko-ladder (round v7)"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-26T19:33:24Z"
forced = false
evidence = "d7ec5e487: the implementer's rule 1 says to work from the prompt's \"Declared source context\" and read only the lines a note says are not shown and the task's own files it does not hold (else read the declared files, as before); 233 bytes longer (304 against 71). The declared files it stops agents reading again were 12-30 KB of each recorded Undertow prompt, 23 KB on average. Test implementer_works_from_its_declared_files and the implementer snapshot (rule 1 twice); red at 20:54: the identity said 'Read ALL files'. Chain on the stack (base 5e9af6682): red at 20:54, both new tests failed on the old code; green: roko-learn lib 1408/0, roko-compose lib 528/0, roko-cli lib 4033 passed and 3 failed (reg-1bf684's two, since fixed on main, and bug-c5f406's own test, whose 'not shown' check also matched the section's intro; it now checks the note's own words); roko-compose's 3 and roko-learn's 10 integration tests pass; clippy -D warnings and nightly fmt clean. With main c39ddd6fb merged in (4d376a003): the four items' tests, their neighbours and roko-planner's three new generation tests pass (19), lib suites roko-cli 4046/0, roko-compose 528/0, roko-learn 1409/0, the integration tests pass, clippy and fmt clean."
+++

## Problem

A task's declared files reach the model twice. The prompt's `file_context` layer holds them ("Declared source
context (authoritative and bounded) … Use these supplied snippets directly"), and the agent reads them again with
`read_file` in its first turn.

The implementer's role says to. Its rule under "Before you write any code" is "1. Read ALL files listed in the
task's `read_files` and `files` fields".

**Phase 4.3** (Undertow, run 3752b8ff), from the prompt logs and the event index:
- **T1#1** re-read all three declared files in its first turn: the spec, `Theme.h`, and the test. The next call's
  fresh input was 10.7k tokens.
- **T2#1** re-read its three declared files, each shown whole, plus `PluginEditor.cpp`.
- **T2#2 and T2#3** re-read the two code files, plus files the prompt didn't hold.
- From the file sizes, the copies of content already in the prompt were about 9k, 8k, 5k and 2k tokens. Every
  later call carried them: 16, 43, 42 and 12 calls.
- **Estimated cost: about $0.08, 8% of the run's $0.98.**

Declared files are two thirds of every prompt there: about 31 KB, of 43.5–47.4 KB.

## Why it matters

- **About 8% of a plan run's spend, plus a turn of latency per try**, for content the model already has. Each later
  turn re-sends it as context (gap-b46c16).
- **The two instructions contradict each other.** The role says read everything; the context layer says use what's
  supplied. Agents follow the role.
- Goal: golden-path.

## Where

- `crates/roko-compose/src/templates/implementer.rs::IMPLEMENTER_ROLE_IDENTITY`: rule 1 of "Before you write any
  code".
- `crates/roko-cli/src/plan_policy.rs::DECLARED_FILES_INTRO` and `render_declared_files`: the prompt's copy, and
  the "not shown" notes.
- **Entry point:** `crates/roko-cli/src/dispatch/prompt_builder.rs::assemble`.

## Current state

The declared-files section has been in every try's prompt since bug-a8a787, with the plan's `sources` first. Its
intro already asks the agent to work from it. Generated plans declare at most 3 read files
(`plan_policy.rs::FAST_MAX_READ_FILES`), so a task's own `files` may be left out: T2's `Status.h` and
`PluginEditor.cpp` were. Reading those is legitimate.

## Plan

The lead asked which is the cheaper fix: guidance against re-reading, or a different prompt shape.

- **A. Guidance (recommended).** Rewrite rule 1 to: "Your task's declared files are in this prompt under 'Declared
  source context'. Work from them. Use `read_file` only for the lines a note says are not shown, and for files in
  your task's `files` that the prompt doesn't hold."
  - It's a one-line template change and a test.
  - It removes the second copy, about 8% here, and saves the first turn's reads.
  - It keeps bug-a8a787's guarantee that the agent sees its context.
  - It depends on the "not shown" notes being complete. Today a retry's grown file is shown short without a note
    (bug-c5f406, a dependency of this item). Land that first.
- **B. Prompt shape.** Declare only the spec and the tests, and let the agent read the code files itself.
  - It saves about as many tokens, since one copy is gone either way.
  - It costs the read turn every time, and it gives up the bounded context that made phase 4.2's plan check pass
    first time.
  - Use it only if A's next run shows agents re-reading anyway.

**Test:** `implementer_works_from_its_declared_files`. The implementer's role identity doesn't tell the agent to read
every declared file, and it points at the declared source context.

## Done when

- The `[[verify]]` passes.
- In the next plan run, no try's first turn reads a declared file that the prompt showed whole and that hasn't
  changed. Check with `tmp/hermes-roko-v7/run-analyst/prompt_logs.py`, which lists each try's re-reads.

## Notes

- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 9, "Every try re-read files its prompt already
  held". The 8% is an estimate from file sizes and each try's second-call input.
- The role identity is shared by every implementer try, so expect snapshot and golden-test updates in
  `roko-compose`.
