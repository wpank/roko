+++
id = "bug-2f33d6"
kind = "bug"
title = "Docs still describe the deleted ToolSelector, and the TUI AgentPool roster modal is reachable only from tests"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "c58c7c2ba"
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
- 2026-10-01 (wk-guard2): implemented on work/bug-7f15df; cargo verification deferred to the batch check.
- Docs: the ToolSelector field and submodule rows and the `selector_active` snapshot field are gone from 05-AGENT.md, docs/v2/ARCHITECTURE-GUIDE.md and three depth/05-agent pages; step 2 now reads "task tool filters", and 05-AGENT.md §9 gains a "Tool policy" paragraph on the contract path. The pool docs (05-AGENT.md §6 and its status note, depth/05-agent/agent-pools.md, and the deferred-items row in 39-ROADMAP.md) now say AgentPool and MultiAgentPool were removed and WarmPool is the only pool.
- TUI: deleted the roster modal (`tui/modals/agent_pool_modal.rs` and `ModalState::AgentPool` with its key, scroll, hint, layout and render arms) rather than binding a key. It was built for the removed MultiAgentPool, its rows need a per-agent cost that the TUI state does not hold, and the Agents tab already shows the roster. Its two input tests went with it; the modal-scroll test now scrolls the batch-review modal.
