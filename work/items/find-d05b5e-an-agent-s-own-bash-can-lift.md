+++
id = "find-d05b5e"
kind = "finding"
title = "An agent's own bash can lift an immune control: nothing stops it running roko safety release (now named in an isolated tool's refusal) against the workspace root, and the audit line records the person's $USER as by"
status = "open"
triage = "verified"
severity = "p3"
size = "M"
subsystem = ["roko-agent/safety", "roko-cli/commands"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "gap-a2787f fix (2026-10-21)"
discovered_from = "gap-a2787f"
anchors = ["crates/roko-agent/src/safety/bash.rs::BashPolicy", "crates/roko-cli/src/commands/safety.rs::SafetyCmd", "crates/roko-agent/src/tool_immune.rs::check_tool_control"]
links = { depends_on = [], blocks = [], related = ["gap-a2787f"], supersedes = [], duplicate_of = "" }
+++

## Problem

An immune control is meant to be lifted by a person after review: gap-a2787f made `roko safety release` release
tool controls as well as agent controls, and an isolated tool's refusal now names the command
(`roko safety release --tool <tool> --scope <scope> --reason <why>`). Nothing stops an agent from running that
command itself through its `bash` tool:

- the default `BashPolicy` (`crates/roko-agent/src/safety/bash.rs::BashPolicy::with_defaults`) is a short denylist
  (`rm -rf /`, `sudo`, `curl | sh`, …) with nothing about `roko safety`;
- `run_tests` and `bash` keep `USER` in the child environment, and `roko safety release` records `by` as `$USER`
  when `--by` is absent, so a release an agent ran is recorded as the person's;
- from an attempt worktree, `roko safety release` without `--workdir` resolves to the worktree (it stops at the
  worktree's `.git`), whose ledger is not the canonical one; with `--workdir` pointing at the workspace root
  (a relative `../..` is not checked by the bash path confinement), it releases the control the boundary enforces.

The same holds for agent controls (`roko safety release <agent>`), though an agent never sees an agent-control
refusal, as those deny before dispatch.

## Why it matters

The isolation exists so that a tool source that returned a malformed or prompt-injecting result stays out of the
run until someone checks it. A model that can lift the control itself, under the person's name in the audit file,
makes that review optional and the audit line misleading.

## Where

- `crates/roko-agent/src/safety/bash.rs::BashPolicy::with_defaults`: what an agent's shell may not run.
- `crates/roko-cli/src/commands/safety.rs::SafetyCmd`: the release commands and their default `--by`.
- `crates/roko-agent/src/tool_immune.rs::check_tool_control`: the refusal that names the command.

## Current state

Not reproduced live: this is from reading the code at gap-a2787f's fix. Whether an agent's `PATH` holds a `roko`
binary depends on the install (a person's workspace usually does; roko's own self-hosting runs use
`target/debug/roko`).

## Plan

Options, not exclusive:

1. The bash and run_tests tools mark their child environment (say `ROKO_TOOL_CALL=1`), and `roko safety release`
   refuses to run under it, saying a person releases controls.
2. The default bash policy refuses `roko safety release` (a substring rule is easy to dodge, so only as a hint).
3. `roko safety release` asks for confirmation on a terminal unless `--yes`, and records whether it had one.

## Done when

- [ ] An agent's `bash` call that runs `roko safety release` is refused, or its audit line says it came from an
  agent's shell, and a test shows it.

## Notes

- Found while doing gap-a2787f. An agent with an unrestricted shell can also edit or delete the ledger file
  directly if it can reach it; this item is about not offering a supported path and not misattributing it.
