+++
id = "gap-6c006d"
kind = "gap"
title = "Prompt Section Effectiveness Loop Proof in Runner-v2"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/145-prompt-section-effectiveness-loop-proof.md#145 — Prompt Section Effectiveness Loop Proof in Runner-v2"
discovered_from = "audit:tmp/backlog/archive/145-prompt-section-effectiveness-loop-proof.md#145 — Prompt Section Effectiveness Loop Proof in Runner-v2"
anchors = ["crates/roko-cli/src/runner/event_loop.rs", "crates/roko-compose/src/", "crates/roko-learn/src/", ".roko/learn/section-effectiveness.json", "event_loop.rs", "tests/section_effectiveness_proof.sh", "SectionEffectivenessRegistry", "snapshot()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The `SectionEffectivenessRegistry` is populated from efficiency events and should influence prompt assembly priority, but the runner-v2 path has not been proven to read from the registry before composing each prompt, so the learning signal may be discarded.. `SectionEffectivenessRegistry` tracks…

Imported without verification from:
- `tmp/backlog/archive/145-prompt-section-effectiveness-loop-proof.md#145 — Prompt Section Effectiveness Loop Proof in Runner-v2`
- `tmp/backlog/_archive/_mori-diffs-gaps.md#§F-6 (suggested 129)`

Some cited files are gone: `.roko/learn/section-effectiveness.json`, `crates/roko-cli/src/runner/event_loop.rs`, `tests/section_effectiveness_proof.sh`.

How to verify: Check: `SectionEffectivenessRegistry::snapshot()` is called before each prompt assembly in runner-v2.; `.roko/learn/section-effectiveness.json` is written after each task completion.; Two-run proof: after run 1, at least one section weight changes… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 5 |]
