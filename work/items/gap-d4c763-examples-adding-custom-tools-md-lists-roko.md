+++
id = "gap-d4c763"
kind = "gap"
title = "examples/adding-custom-tools.md lists roko-mcp-scripts and roko-mcp-slack as example crates; neither exists"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs/examples"]
created = 2026-10-16
updated = 2026-10-16
source = "found while closing gap-2eea20, 2026-10-16"
discovered_from = "gap-2eea20"
anchors = ["examples/adding-custom-tools.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'roko-mcp-scripts\\|roko-mcp-slack' examples/adding-custom-tools.md"
+++

## Problem

`examples/adding-custom-tools.md`'s "Examples in this repo" list (near the top of the file) names
four MCP crates: `crates/roko-mcp-scripts`, `crates/roko-mcp-github`, `crates/roko-mcp-slack`,
`crates/roko-mcp-stdio`. Only two exist: `crates/roko-mcp-github` and `crates/roko-mcp-stdio`.
`roko-mcp-scripts` and `roko-mcp-slack` are not workspace members (checked with `ls crates/` at
HEAD `322479f02`; CLAUDE.md's Key crates table lists `roko-mcp-code`, `roko-mcp-github` and
"roko-mcp-stdio" under `roko-mcp-github / stdio`, with no `scripts` or `slack` crate).

Found while fixing gap-2eea20 (dead links in the same file); out of that item's scope, filed
separately per CLAUDE.md's "a new gap or bug becomes a new item".

## Why it matters

Goal `tooling`: a reader of this onboarding doc who goes looking for `roko-mcp-scripts` or
`roko-mcp-slack` finds nothing, in a file whose whole purpose is "here are real examples to copy
from".

## Where

`examples/adding-custom-tools.md`, the "Examples in this repo" bullet list (near line 9-12).

## Current state

Unverified beyond the one HEAD check above; not investigated whether `roko-mcp-scripts` or
`roko-mcp-slack` existed previously and were removed, or were always aspirational.

## Plan

1. Drop `crates/roko-mcp-scripts` and `crates/roko-mcp-slack` from the list, or replace them with
   two crates that actually exist (e.g. `roko-mcp-code`), whichever better represents the real
   set of MCP server examples.

## Done when

The `[[verify]]` command passes.

## Notes

Docs only.
