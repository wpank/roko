# Workflow Subsystem — PRD Index

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Scope**: A unified, composable, modular subsystem for defining, configuring, executing, observing, and sharing workflows in roko. Replaces the existing PRD/plan/research command surface with a single workflow primitive that scales from doc ingestion to deploy pipelines to long-running file watchers.

---

## What This Is

Today roko has roughly six disconnected orchestration mechanisms: PRD lifecycle commands, plan execution, research enhancement, the executor DAG, gate pipelines, and the vision loop. Each is bespoke. None compose. None can be authored by users. None can be triggered by anything other than a CLI invocation.

This PRD set replaces them with three primitives:

```
Module    →  smallest unit of work (synthesize, classify, audit, deploy-step, web-search, ...)
Workflow  →  composition of Modules (state graph with conditional edges, slots, macros)
Trigger   →  what fires a Workflow (manual, cron, watch, webhook, GitHub, Slack, event-bus, ...)
```

Plus a workspace concept that elevates the current `.roko/` directory pattern into a first-class subsystem with metadata, registry, switching, templates, and inheritance — analogous to opening a project in a DAW.

Every existing roko orchestration mechanism becomes a Workflow:
- `prd draft` → `prd-draft` Workflow
- `prd plan` → `prd-plan` Workflow
- `plan run` → `plan-execute` Workflow
- `research enhance-prd` → `prd-enrich` Workflow
- The vision loop → `visual-gate` Workflow built on `visual-gate2` primitives
- The 7-rung gate pipeline → `gate-evaluate` Workflow

New workflows ship in v1 covering doc ingestion, deploy, refactor, watch, cron, code review, doc completeness, dependency updates, and more (see PRD-06).

The whole subsystem is designed so that the more workflows exist, the more synergistic they become — workflows compose, share artifacts, share context, learn from each other, trigger each other, and accumulate into a community-shared marketplace of forkable, parameterized presets.

---

## Reading Order

Read in numeric order. Each PRD declares its prerequisites in the header.

| PRD | Title | Concern |
|---|---|---|
| 01 | Workspace Subsystem | Project-level container, registry, switching, templates, inheritance |
| 02 | Workflow Abstractions | Module / Workflow / Artifact / Macro / Slot trait kernel |
| 03 | Trigger System | Manual / cron / watch / webhook / GitHub / Slack / event-bus / artifact-change / chain |
| 04 | Configuration & Authoring | TOML schema, scripts, WASM, Rust, capability declarations, validation |
| 05 | Execution Engine | State graph runtime, conditionals, loops, human-in-loop, resumability, budgets |
| 06 | Builtin Workflow Catalog | The lego pieces shipped in v1 — full workflow inventory |
| 07 | Doc-Ingest Worked Example | End-to-end walkthrough using the catalog |
| 08 | CLI Redesign | One-line entry, replaces existing prd/plan/research surface |
| 09 | TUI Redesign | ratatui surface for workflows, runs, workspace switching |
| 10 | Dashboard Redesign | Nunchi pages: library, editor, inspector, trigger manager |
| 11 | Visual Config Wizard | The "video-game" drag-drop authoring UX |
| 12 | Marketplace & Sharing | Publish, fork, install, attribution, anti-spam |

---

## Key Concepts at a Glance

**Workspace**: A directory containing a `workspace.toml` and a `.roko/` runtime dir. Users `roko workspace open` to set it as current, like opening a DAW project. Workspaces are registered in `~/.roko/workspaces.json`.

**Module**: A typed unit of work. Declares inputs, outputs, required evidence, capabilities. Implementations: built-in Rust, WASM, scripts (bash/python/node), pure TOML compositions.

**Workflow**: A named, versioned, persistable composition of Modules wired into a state graph. Carries Macros (promoted parameters consumers tune) and Slots (typed empty positions consumers fill). Forkable.

**Trigger**: A first-class primitive that fires a Workflow. Decoupled from the Workflow itself — the same Workflow runs identically whether fired by a human, a cron, a file watcher, a GitHub PR, a Slack message, or another Workflow's completion.

**Artifact**: Content-addressed, versioned, lineage-tracked output of any Module run. Inputs to other Modules. Persisted in the workspace.

**Macro**: A promoted parameter on a Workflow. The DAW Rack Macro analog. Consumers see a small set of high-level knobs (model, strictness, max-iterations, budget).

**Slot**: A typed empty position in a Workflow. Consumers plug in their own Module or sub-Workflow. ("Researcher slot — drop any web-research Module here.")

**Capability**: Declared permission a Module needs (`fs.read`, `fs.write`, `net`, `shell`, `llm`, `chain.write`). Capabilities are granted at the workspace level and inspected at install time for marketplace artifacts.

**Profile** (visual-gate2): A specialization of Workflow for evaluation. Reuses the same engine. Composes EvidenceCollector and Criterion Modules.

---

## Architectural Principles

1. **One kernel, many surfaces.** The workflow engine is a single Rust crate with stable types; CLI, TUI, dashboard, API, and marketplace all bind to it.

2. **TOML at the seams.** Composition is always TOML. Implementations may be Rust, WASM, or scripts, but how they're wired together is declarative and human-readable.

3. **Capability-typed.** Every Module declares its required capabilities. Workspaces grant capabilities. Marketplace artifacts disclose capabilities at install.

4. **Modules are tiny, Workflows compose.** Prefer 12 small Modules to 3 large ones. Composition is where value emerges.

5. **State, not just DAG.** The execution graph supports conditional edges, loops, fan-out, fan-in, sub-workflow calls, and human-in-loop nodes. Plan-style DAGs are a special case.

6. **Triggers are external, Workflows are pure.** A Workflow doesn't know how it was fired. The same Workflow runs from CLI, cron, watch, or webhook.

7. **Everything is forkable.** Modules, Workflows, Triggers, and Profiles all support `fork` as a first-class operation that preserves lineage.

8. **Artifacts are content-addressed.** Workflow outputs are immutable, hashed, deduplicated, lineage-tracked. Re-runs are idempotent.

9. **Synergy through shared context.** All Workflows in a workspace share an event bus, an artifact store, and the neuro/learning state. Stacking Workflows compounds value.

10. **No hidden magic.** Every Workflow run produces a complete audit trail: what fired it, what Modules ran, what artifacts were produced, what the state graph traversal was, what costs were incurred.

---

## Status of the Existing System

The following are deprecated by this PRD set but remain functional during the transition window (no migration code needed since there is no production state to preserve):

- `roko prd idea / draft / plan / consolidate / list / status`
- `roko plan list / show / generate / regenerate / validate / run`
- `roko research topic / search / enhance-prd / enhance-plan / enhance-tasks / analyze`
- `roko run "<prompt>"` (replaced by `roko run <workflow-name> [-- args]`)
- The existing 7-rung `GatePipeline` runtime (kept as a Workflow internally; surface goes away)
- The `vision_loop/` module (absorbed into `visual-gate` Workflow)

---

## Conventions

- **Naming**: `kebab-case` for Workflow / Module / Trigger names; `snake_case` for TOML keys; `PascalCase` for Rust types.
- **File layout**: Workflows live in `<workspace>/.roko/workflows/<name>.toml` (workspace) or `~/.roko/workflows/<name>.toml` (user-level). Workspace overrides user-level by name.
- **Versioning**: Semver on every published artifact. `name@1.2.3` resolution.
- **Run IDs**: `wf_<base32_ulid>` for Workflow runs; `mod_<base32_ulid>` for Module runs.
- **Artifact IDs**: `art_<sha256_truncated>` content hash.
