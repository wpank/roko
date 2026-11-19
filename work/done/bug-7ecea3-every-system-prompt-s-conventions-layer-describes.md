+++
id = "bug-7ecea3"
kind = "bug"
title = "Every system prompt's conventions layer describes roko's own repository: CONTEXT_LAYOUT_STANZA sends agents to .roko/plans/workspace-map.md \"instead of find/ls on crates/\" and to three other files that exist in neither roko nor the project (every Undertow try of round v7)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-compose/role_prompts"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "d7afb69f6"
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 9, run 3752b8ff), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-compose/src/templates/common.rs::CONTEXT_LAYOUT_STANZA", "crates/roko-compose/src/role_prompts.rs::conventions_text", "crates/roko-cli/src/dispatch/prompt_builder.rs::assemble"]
links = { depends_on = [], blocks = [], related = ["spec-882b36", "gap-c135ba"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn conventions_carry_no_roko_repo_layout' crates/roko-compose/src && cargo test -p roko-compose conventions_carry_no_roko_repo_layout"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T20:57:03Z"
commit = "b2feceb6a"
by = "roko-ladder (round v7)"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-26T19:33:01Z"
forced = false
evidence = "b2feceb6a: conventions_text starts from DEFAULT_CONVENTIONS_SUFFIX, and CONTEXT_LAYOUT_STANZA (roko's own crates/ and .roko/plans layout) is gone from every role's conventions layer: 919 bytes off each of the 22 recorded Undertow first prompts of phases 4.3, 5 and 5.1, 1.8-2.4% of each 38-52 KB prompt. Test conventions_carry_no_roko_repo_layout (implementer, conductor and researcher name no crates/ nor workspace-map.md); 29 system prompt snapshots lose the stanza. Red not run (the old layer opened with the stanza, which names crates/). Chain on the stack (base 5e9af6682): red at 20:54, both new tests failed on the old code; green: roko-learn lib 1408/0, roko-compose lib 528/0, roko-cli lib 4033 passed and 3 failed (reg-1bf684's two, since fixed on main, and bug-c5f406's own test, whose 'not shown' check also matched the section's intro; it now checks the note's own words); roko-compose's 3 and roko-learn's 10 integration tests pass; clippy -D warnings and nightly fmt clean. With main c39ddd6fb merged in (4d376a003): the four items' tests, their neighbours and roko-planner's three new generation tests pass (19), lib suites roko-cli 4046/0, roko-compose 528/0, roko-learn 1409/0, the integration tests pass, clippy and fmt clean."
+++

## Problem

Every system prompt's `conventions` layer opens with roko's own repository layout. That's
`CONTEXT_LAYOUT_STANZA`: 11 lines, 1,063 bytes, added whatever the project is.

> `.roko/plans/` — canonical plan-artifact root … `.roko/plans/workspace-map.md` — crate file tree; use this instead
> of `find`/`ls` on `crates/` … `.roko/plans/preflight-snapshot.md` … `.roko/plans/CONTEXT.md` …
> `.roko/plans/ignored-tests.md` — ledger of `#[ignore]` tests …

**Every Undertow try in round v7 got it.** It is in all 4 prompt logs of phase 4.3 (run 3752b8ff,
`~/dev/undertow/.roko/prompt-logs/p4-3-the-editor-s-finish-T*.txt`).
- Undertow is a C++/JUCE project. It has no `crates/` and no `#[ignore]` tests.
- Its plans live in `plans/`, and its `.roko/plans/` is empty.

**None of the four files exists in roko's own checkout either.** roko's `.roko/plans/` holds 4 JSON files, and its
plans live in `plans/`. Nothing in `crates/` writes `workspace-map.md` or `preflight-snapshot.md`: their generator
was parked (spec-882b36, gap-c135ba). The stanza came with the mori port of the prompt templates (2b41149e1,
2026-04-06).

**Expected:** the conventions name only what exists in the workspace the agent works in, or nothing.

## Why it matters

- **It's misleading guidance in every try, not just noise (the lead's call).**
  - It tells the agent to use a file that doesn't exist "instead of `find`/`ls` on `crates/`".
  - It names a plan root the project doesn't use.
  - An agent that follows it wastes turns looking for those files, or skips exploring the real tree.
- **It costs on every call.** It ships in every system prompt, about 1.1 KB, and each call re-sends it as part of
  the context.
- Goal: golden-path. Found in round v7's phase 4.3 analysis
  (`tmp/hermes-roko-v7/findings/run-analyst.md`, section 9).

## Where

- `crates/roko-compose/src/templates/common.rs::CONTEXT_LAYOUT_STANZA`: the text. Its tests in the same file
  assert that it names `workspace-map.md` and `CONTEXT.md`.
- `crates/roko-compose/src/role_prompts.rs::conventions_text`: prepends the stanza to every
  `RoleSystemPromptSpec`'s conventions, before `DEFAULT_CONVENTIONS_SUFFIX` and any `extra_conventions`.
- **Entry points:**
  - `crates/roko-cli/src/dispatch/prompt_builder.rs::assemble`, Graph dispatch, the path of every plan try;
  - `crates/roko-cli/src/prompting.rs`, the other prompt path, which adds `with_extra_conventions`.
- `crates/roko-compose/tests/system_prompt_snapshot.rs`: snapshots that include the layer.

## Current state

- The stanza is unconditional, with no workspace check and no setting.
- The rest of the layer is fine: "Keep changes minimal and prefer wiring existing modules over reimplementation.
  Use the repo's existing patterns before inventing new ones."

## Plan

1. Take `CONTEXT_LAYOUT_STANZA` out of `conventions_text`, and keep `DEFAULT_CONVENTIONS_SUFFIX`.
2. If roko's self-hosting wants a layout note, have the caller add it through `with_extra_conventions`, naming
   only paths that exist under the task's workdir. Delete the constant if nothing uses it.
3. Update the stanza's tests in `common.rs` and the system-prompt snapshots.
4. Add the test `conventions_carry_no_roko_repo_layout`. A default `RoleSystemPromptSpec`'s rendered conventions
   contain neither `crates/` nor `.roko/plans/workspace-map.md`.

## Done when

- The `[[verify]]` passes.
- The next plan run's prompt logs have no "Plans context layout" in the conventions layer. Check with
  `tmp/hermes-roko-v7/run-analyst/prompt_logs.py`.

## Notes

- Prompt logs are on in Undertow (`.roko/prompt-logs/`). Each try's first prompt is there.
- Related: the parked workspace-map generator (spec-882b36, gap-c135ba). Generating the files is a separate
  choice from naming them in every prompt.
