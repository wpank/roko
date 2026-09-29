+++
id = "gap-b35a57"
kind = "gap"
title = "Live tool-step targets for shell commands still show absolute workspace paths"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-agent/live-output"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d5c1dc6be"
source = "session:roko-b6 2026-09-29 portal close-out"
discovered_from = "review of gap-fa61f8's fix (15e754bda, merged in 5b7a1711b), which rewrote only path fields"
anchors = ["crates/roko-agent/src/live_output.rs::tool_step_target", "crates/roko-agent/src/live_output.rs::workspace_relative", "crates/roko-agent/src/live_output.rs::tool_step_target_is_workspace_relative"]
links = { depends_on = [], blocks = [], related = ["gap-fa61f8", "find-0d280d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn command_targets_are_workspace_relative' crates/roko-agent/ && cargo test -p roko-agent command_targets_are_workspace_relative"
+++

## Problem

The live tool step published for a shell command carries the command's first line verbatim. For a command that
names a file by its absolute path, such as `cat /private/tmp/roko-hello-JdC7dN/src/main.rs` (the case the unit test
pins), the portal and the event stream show the absolute workspace path. For a real workspace that path usually sits
under the user's home directory. It also pushes the interesting part of the command toward the 120-character cut. Since gap-fa61f8, `file_path`, `notebook_path` and
`path` targets are shown relative to the workspace; `command` targets are not.

## Why it matters

Goal `visibility`: the live step is the portal's main "what is the agent doing" signal, and the design shows
workspace-relative targets. The absolute path also leaks the local directory layout into every published step.

## Where

- `crates/roko-agent/src/live_output.rs::tool_step_target` (:95-157): the `command` branch (:118-121) keeps the first
  line as is. Only `PATH_FIELDS` (:111) go through `workspace_relative` (:162).
- The unit test `tool_step_target_is_workspace_relative` asserts today's behaviour: an absolute path inside a
  command stays absolute (:336-338).
- The provider factory hands `AgentOptions::working_dir` to the immune boundary, which calls `tool_step_target`
  with the root (see gap-fa61f8's evidence).

## Current state

Checked at `d5c1dc6be` by reading the code: `15e754bda` (gap-fa61f8) is merged and covers only the path fields.
`pattern` (Glob) and other fields are also passed verbatim. An absolute Glob pattern would show the same prefix;
that case was not observed.

## Plan

1. In the `command` branch, with a known root (as given and canonicalized, as `workspace_relative` does), replace
   `<root>/` with nothing, and a bare `<root>` token (followed by whitespace, a quote, `;`, `&`, `|`, `)` or the end)
   with `.`. Leave sibling paths that only share the prefix (`<root>-other/…`) untouched. Do it before secret
   scrubbing and truncation.
2. Update the assertion at :336-338, and add `command_targets_are_workspace_relative`, covering: `cat <root>/src/main.rs`
   gives `cat src/main.rs`; `cd <root> && cargo test` gives `cd . && cargo test`; a sibling prefix stays as is; the
   canonical root form is handled; with no root, nothing changes.
3. Optionally apply the same rewrite to an absolute `pattern`.

## Done when

- A shell step inside the workspace shows workspace-relative paths in its target.
- The `[[verify]]` command passes.

## Notes

This is a display change only. The rewritten target must never be used to run anything.
