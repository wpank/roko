+++
id = "gap-c14e25"
kind = "gap"
title = "roko plan generate --help and roko run --help describe SOURCE, --from-file and --complexity as the code does not behave"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan-run"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-d2b605 (2026-10-27)"
discovered_from = "gap-d2b605"
anchors = ["crates/roko-cli/src/commands/plan.rs::PlanCmd", "crates/roko-cli/src/main.rs::RunComplexity"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-d2b605", "bug-f12b2d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qF 'free-text prompt, or path to a file' crates/roko-cli/src/commands/plan.rs && ! grep -qF 'Treat source as a file path' crates/roko-cli/src/commands/plan.rs && ! grep -qF 'standard and complex write a plan first' crates/roko-cli/src/main.rs"
+++

## Problem

Three doc comments that clap prints in `--help` say what the code does not do:

- `PlanCmd::Generate::source` (`roko plan generate [SOURCE]...`): "Source: free-text prompt, or path
  to a file (spec, requirements, etc)." The words are always the request's text
  (`source.join(" ")`); a path among them is not read as the source. A markdown document of the
  workspace that the words name is read only for its checks, and only when the words state none
  (`plan_generate/source_checks.rs::named_documents`).
- `PlanCmd::Generate::from_file`: "Treat source as a file path to read (instead of inline text)."
  `--from-file <FROM_FILE>` takes its own path and reads that file; the words are then ignored.
- `Command::Run::complexity` (`roko run --complexity`): "trivial and simple run one task, standard
  and complex write a plan first." The values `--help` lists are `trivial`, `simple`, `medium` and
  `complex`; `standard` is an alias of `medium` (`RunComplexity`).

gap-d2b605 corrected these rows in `docs/v3/28-CLI.md`; it was docs-only, so the help text still
says the old thing.

## Why it matters

`--help` is the first reference an operator or agent reads. `roko plan generate docs/spec.txt`
gives the planner the words "docs/spec.txt" as the request, not the file's text, and
`--complexity standard` is a value `--help` does not list.

## Where

- `crates/roko-cli/src/commands/plan.rs::PlanCmd`: `Generate::source`, `Generate::from_file`.
- `crates/roko-cli/src/main.rs::Command`: `Run::complexity`, with the values of `RunComplexity`.

## Current state

Checked 2026-10-27 at 0c8c3d69a against the code paths named above.

## Plan

Rewrite the three doc comments from those code paths, in the words `docs/v3/28-CLI.md` now uses.

## Done when

`roko plan generate --help` and `roko run --help` describe the three as the code behaves, and
`[[verify]]` passes. The verify checks that the three stale sentences are gone; a reviewer reads the
new text against the code.

## Notes

Found while doing gap-d2b605. No behaviour changes: doc comments only. `--force-resume`'s stale help
text is bug-f12b2d, so it is left out here.
