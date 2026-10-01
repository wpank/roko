+++
id = "bug-2f33d6"
kind = "bug"
title = "Docs still describe the deleted ToolSelector, and the TUI AgentPool roster modal is reachable only from tests"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-48faa7"
anchors = ["docs/v3/05-AGENT.md", "docs/v2/ARCHITECTURE-GUIDE.md", "crates/roko-cli/src/tui/"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-48faa7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rln 'ToolSelector' docs/v3/05-AGENT.md docs/v2/ARCHITECTURE-GUIDE.md"
+++

## Problem

gap-48faa7 deleted the unattached ToolSelector, but `docs/v3/05-AGENT.md`, its depth pages and `docs/v2/ARCHITECTURE-GUIDE.md` still describe it. gap-ee8dc0 left the TUI `ModalState::AgentPool` roster modal reachable only from tests.

## Plan

Update the docs to the current tool-policy path. Delete the modal, or give it a key binding.

## Done when

- The verify passes, and the modal has a production entry point or is gone.

## Notes

- Reported on 2026-10-01 by the worker on gap-48faa7, during the evening close-out round.
