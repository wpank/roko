---
plan: add-plan-queue
---

# Add plan queue config and documentation

The `roko plan queue` command already reads and validates a `.roko/queue.toml`
milestone manifest. What is missing is a first-class `[queue]` section in the
workspace `roko.toml` that declares workspace-level defaults (queue file path,
max agents, execution mode), and user-facing documentation for the entire
feature.

This plan closes those two gaps in three sequential tasks:

1. An implementer adds `QueueConfig` to the roko-core config schema and wires
   it into `RokoConfig`, giving operators a supported `[queue]` section in
   `roko.toml`.
2. An implementer wires the new `QueueConfig` defaults into `roko plan queue
   show` so the displayed output reflects the workspace config alongside the
   milestone table.
3. A scribe writes `docs/v3/plan-queue.md` covering the queue.toml schema,
   the new roko.toml section, and all three subcommands.

All generated artefacts are production source files; there are no demo
fixtures. The plan runs sequentially (each task depends on the previous) and
requires no external services.
