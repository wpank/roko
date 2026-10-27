+++
id = "gap-171238"
kind = "gap"
title = "docs/v3/28-CLI.md has no section for roko safety, effects, showcase, audit or demo"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["docs"]
created = 2026-10-26
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-fd9dfb (2026-10-26)"
discovered_from = "gap-fd9dfb"
anchors = ["docs/v3/28-CLI.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-fd9dfb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for c in safety effects showcase audit demo; do grep -q \"^### .roko $c\" docs/v3/28-CLI.md || exit 1; done"

[[verify]]
command = "for p in '^roko safety release --tool <tool> --scope <scope> --reason <why>' '^roko effects approve <effect-id>' '^roko showcase passphrase hash' '^roko audit reveal <run-id>' '^roko demo warm' '(#safety-and-audits)' 'roko started for an agent or a check' 'by_source'; do grep -q -- \"$p\" docs/v3/28-CLI.md || exit 1; done"
+++

## Problem

`docs/v3/28-CLI.md`, the CLI reference, has no section for five top-level commands that a default `roko` build
shows in `roko --help` (`crates/roko-cli/src/main.rs::Command`, none hidden or behind a feature):

- `roko safety controls` / `roko safety release` (`commands/safety.rs::SafetyCmd`): list the immune controls in
  force and release one, which only a person may do (find-d05b5e);
- `roko effects list/show/approve/reject` (`commands/effects.rs::EffectsCmd`): the tool calls runs hold for
  approval;
- `roko showcase passphrase new/hash` (`commands/showcase.rs::ShowcaseCmd`);
- `roko audit status/replay/reveal/incidents` (`commands/audit.rs::AuditCmd`): what M4's random deep audits found;
- `roko demo` (`main.rs::DemoCmd`).

`grep -n 'roko safety\|roko effects\|roko showcase\|roko audit\|roko demo' docs/v3/28-CLI.md` finds nothing.

## Why it matters

The CLI reference is where an operator looks up a command. Without these sections they cannot find how to review
and release an immune control, approve or reject a held tool call, read the audit results, or set the showcase
login passphrase. CLAUDE.md's CLI tables carry rows for `roko safety` and `roko effects` (gap-fd9dfb), but none for
`showcase`, `audit` or `demo`.

## Where

- `crates/roko-cli/src/main.rs::Command`: the variants `Safety`, `Effects`, `Showcase`, `Audit` and `Demo`, with
  their doc comments and `after_help` examples; `DemoCmd` is in the same file.
- `crates/roko-cli/src/commands/{safety,effects,showcase,audit}.rs`: the subcommands, flags and doc comments.
- `docs/v3/28-CLI.md`: section 15 "Utilities" (or the section each fits better) and the table of contents.

## Current state

Checked 2026-10-26 at 754f45ebd by comparing the visible variants of `Command` with the `roko <name>` mentions in
the doc: these five are the only ones missing (`complete` is hidden). `docs/v3/26-HTTP-API.md` §8.27 documents the
safety and effects routes.

## Plan

Add one `### \`roko <name>\`` section per command, in the style of the doc's other sections (a synopsis block, a
short description, flags where they matter), written from each command's clap doc comments and examples. For
`roko safety release`, say that only a person releases a control: the default bash policy refuses the command and
it exits 1 under the variables roko sets on its agents and checks.

## Done when

- `docs/v3/28-CLI.md` has a `### \`roko <name>\`` heading for each of `safety`, `effects`, `showcase`, `audit` and
  `demo`, and the `[[verify]]` command passes.
- The sections give each subcommand's synopsis (the second `[[verify]]` checks one per command), the table of
  contents lists the new "Safety and audits" section, and the `roko safety` section says that the release refuses
  to run in a process roko started for an agent or a check, and what its audit line's `via` records (`by_source`).

## Notes

- Docs only. Found while doing gap-fd9dfb.
- Placement (2026-10-27): `roko safety`, `roko effects` and `roko audit` went into a new section 11, "Safety and
  audits"; `roko showcase` follows `roko deploy` in "Server and deployment"; `roko demo` ends "Utilities". The
  environment table gained `ROKO_AUDIT_HOME` and `ROKO_SHOWCASE_PASSPHRASE_HASH`. Filed while writing them:
  gap-362b00 (`roko demo warm` caches nothing), bug-a1cff1 (`roko demo setup` exits 0 when its build fails) and
  gap-d2b605 (28-CLI.md leaves out flags of `roko run`, `roko plan run` and `roko plan generate`).
