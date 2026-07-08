# context-pack/ — Agent bootstrap context

> Every fresh Claude agent spawned by `run-migration.sh` reads this directory first,
> before touching any source files. These 7 files together give the agent the minimum
> context needed to produce correct, consistent, well-cited output for its assigned topic.

## Files in order

| # | File | What it contains |
|---|---|---|
| 00 | [`00-ALWAYS-READ-FIRST.md`](./00-ALWAYS-READ-FIRST.md) | Who you are, what Roko is, architecture summary, Engram, 6 traits, universal loop, 5 layers, cross-cuts, C-Factor, 14 blue ocean innovations, 18-crate structure, critical distinction that refactoring-prd wins |
| 01 | [`01-naming-map.md`](./01-naming-map.md) | Authoritative old→new naming map. Every rename. Tokens (KORAI/DAEJI). Crate dissolution plan for roko-golem. |
| 02 | [`02-reframe-rules.md`](./02-reframe-rules.md) | Conceptual reframes (Mortality→Resource Management, Succession→Backup/Restore, Styx→Mesh, Golem-specific→Domain-agnostic). Incompatibility flags. Citation preservation rules. |
| 03 | [`03-concepts-lifecycle.md`](./03-concepts-lifecycle.md) | REMOVED / KEPT / KEPT-REFRAMED / INTRODUCED concept lists. Files to SKIP. Files to extract citations only. |
| 04 | [`04-writing-rules.md`](./04-writing-rules.md) | 20 non-negotiable rules: no summarize, no truncate, preserve citations, zero-context readers, naming, reframing, layers, Synapse, domain-agnostic, research context, index, sub-docs, Rust code, markdown, self-check, no clarification, logging, tool usage, context limits, quality bar. |
| 05 | [`05-source-files.md`](./05-source-files.md) | Absolute path layout. Legacy PRD sections. Research/tmp directories. Implementation-plans files. Reference code. How to find sources for a specific topic. |
| 06 | [`06-output-structure.md`](./06-output-structure.md) | Output root, topic folder layout, INDEX.md schema, sub-doc schema, minimum content requirements, writing order, parallel agent protocol. |

## How prompts use this pack

Each prompt in `../prompts/<topic>.prompt.md` tells the agent to read every file in
this directory first. The prompt then references specific files for deeper context
and adds topic-specific instructions.

## If something is missing here

If an agent needs information not in this pack, add it to the appropriate file and
re-run. Do not have agents resolve it themselves — they won't have consistent context
and different agents will make different decisions.

## Maintenance

When the refactoring-prd docs are updated:
1. Review this pack for any statements that contradict the new spec.
2. Update the affected context-pack files.
3. Re-run any previously-completed topics whose output might be affected.

When new source files are added:
1. Update `05-source-files.md` with the new paths.
2. Update `../SOURCE-INDEX.md` to map the new files to the appropriate topic.
3. Re-run the affected topic if its output doesn't reference the new material.
