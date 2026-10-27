+++
id = "gap-19e1e3"
kind = "gap"
title = "CLAUDE.md's CLI tables have no rows for roko audit, roko showcase or roko demo"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["docs"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-171238 (2026-10-27)"
discovered_from = "gap-171238"
anchors = ["CLAUDE.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-171238", "gap-fd9dfb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for c in audit showcase demo; do grep -qF \"| \\`roko $c\" CLAUDE.md || exit 1; done"
+++

## Problem

CLAUDE.md's "CLI commands reference" tables have rows for `roko safety` and `roko effects`
(gap-fd9dfb) but none for three other top-level commands that a default build shows in
`roko --help` (`crates/roko-cli/src/main.rs::Command`, none hidden or behind a feature):

- `roko audit status/replay/reveal/incidents` (`commands/audit.rs::AuditCmd`): what the random deep
  audits of green attempts found, read-only;
- `roko showcase passphrase new/hash` (`commands/showcase.rs::ShowcaseCmd`): make or hash the public
  showcase's login passphrase;
- `roko demo setup/warm` (`main.rs::DemoCmd`, `demo_cmd.rs`).

`grep -n 'roko audit\|roko showcase\|roko demo' CLAUDE.md` finds nothing.

## Why it matters

Agents read CLAUDE.md before they work on roko; a command missing from its tables is one they do not
know to use or to keep in step. `docs/v3/28-CLI.md` documents all three (gap-171238).

## Where

- `CLAUDE.md`: "CLI commands reference" (`roko audit` fits "Learning & feedback", `roko showcase`
  "Server & deployment", `roko demo` "Utilities").
- The command definitions named above, and `docs/v3/28-CLI.md` §`roko audit`, §`roko showcase` and
  §`roko demo` for the wording.

## Current state

Checked 2026-10-27 at 0191f7858.

## Plan

Add one row per command, in the tables' style: the subcommands and one line on what each does,
with no status claims. Ask before committing a CLAUDE.md change if the round's rules say so.

## Done when

CLAUDE.md has a table row starting with each of `roko audit`, `roko showcase` and `roko demo`, and
`[[verify]]` passes.

## Notes

Docs only. Found while doing gap-171238.
