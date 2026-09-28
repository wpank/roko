+++
id = "gap-f57fe5"
kind = "gap"
title = "roko setup wizard is stdin-based, not the specified ratatui wizard"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/setup"]
created = 2026-08-31
updated = 2026-09-28
source = "gaps-md#partial-10/223"
anchors = ["crates/roko-cli/src/commands/setup.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

UX #223: `roko setup` prompts on stdin. The spec calls for a ratatui wizard that handles provider detection, workspace init and verification in one TUI flow. Backlog #223 is archived without a status.

Fix: implement the TUI wizard, or record a decision to keep the stdin flow.
