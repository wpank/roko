+++
id = "bug-f5470e"
kind = "bug"
title = "apply_patch rejects the *** Begin Patch envelope that OpenAI-family models send to a tool of that name, and its error does not say which format it takes, so gpt-oss-120b retries it a dozen times"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-std/tools"]
created = 2026-10-15
updated = 2026-10-15
source = "tmp/hermes-roko-v5/findings/roko-demos.md (roko-demos, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: demo plans run from Hermes in the roko repo"
anchors = ["crates/roko-std/src/tool/builtin/apply_patch.rs::parse_single_file_patch", "crates/roko-std/src/tool/builtin/apply_patch.rs::DESCRIPTION"]
links = { depends_on = [], blocks = [], related = ["bug-0c7f25"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn apply_patch_accepts_the_begin_patch_envelope' crates/roko-std/ && cargo test -p roko-std apply_patch_accepts_the_begin_patch_envelope"
+++

## Problem

Runs started from Hermes on 2026-10-15 in clones of the roko repo (roko-main 7b6a5af44, plans at a8e27e4b7, the
workspace ladder's `cheap` rung = `gpt-oss-120b` on Cerebras):

- `demo-multistage`, task STAGE-3 (implementer, editing the checked-in `demo/multistage-plan/validate.sh`, run
  `c4018d4b…`). The agent called `apply_patch` 13 times with the patch envelope OpenAI-family models use for a tool
  of that name:

  ```
  *** Begin Patch
  *** Update File: demo/multistage-plan/validate.sh
  @@
  -# Verify required Markdown headings are present.
  -rg -q '^## Repository signals' demo/multistage-plan/discovery.md
  -rg -q '^## Decision' demo/multistage-plan/decision.md
  +# Verify required Markdown headings are present.
  +rg -q '^## Repository signals' demo/multistage-plan/discovery.md
  +rg -q '^## Decision' demo/multistage-plan/decision.md
  +rg -q '^## Evidence considered' demo/multistage-plan/decision.md
  …
  ```

  Every call failed with "tool failure: apply_patch: no `+++ ` header found". The agent then fell back to
  `write_file`, which worked. The task cost $0.116 of the run's $0.185, and its context grew to 293k cached tokens
  (`.roko/learn/efficiency.jsonl`; the STAGE-3 rows of `.roko/tool_audit.jsonl`).
- `demo-greeting-module` (run `2ef2635a…`): 8 failed `apply_patch` calls, then `write_file`.

`apply_patch` takes only a single-file unified diff (`--- a/x`, `+++ b/x`, `@@ -l,n +l,n @@`). Its description,
"Apply a unified-diff patch to a single file in the worktree.", and its error do not show the model what to send.
gpt-oss, GPT-5 and Codex models are trained on the `*** Begin Patch` format for `apply_patch`, so they keep sending it.

## Why it matters

The `cheap` rung does most mechanical and focused work on the default ladder. A task that edits an existing file can
spend most of its budget in this loop before it falls back to rewriting the whole file.

## Where

- `crates/roko-std/src/tool/builtin/apply_patch.rs::parse_single_file_patch`: needs a `+++ ` header.
- `crates/roko-std/src/tool/builtin/apply_patch.rs::DESCRIPTION` and the `patch` parameter's description.

## Plan

1. Accept the envelope: `*** Begin Patch` … `*** End Patch` with one `*** Update File: <path>` section. Its hunks start
   with `@@` (often with no line numbers) and hold ` `, `-` and `+` lines. Locate each hunk by its context and removed
   lines, matched as strictly as unified hunks are today. Refuse `*** Add File`, `*** Delete File` and more than one
   file with an error that names the supported forms.
2. When a patch is in neither format, answer with the formats the tool takes and a three-line example of each.
3. Say in the tool and parameter descriptions which formats it takes.
4. Tests: the STAGE-3 envelope applies to the file it was meant for, and a malformed patch gets the explanatory error.

## Done when

`apply_patch_accepts_the_begin_patch_envelope` passes (the `[[verify]]` command).
