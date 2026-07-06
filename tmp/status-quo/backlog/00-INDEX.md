# 00 — Backlog Index

> Navigation layer for the roko **executable backlog**.
> Repo HEAD `5852c93c05` (branch `main`) · authored 2026-07-09 ·
> root `/Users/will/dev/nunchi/roko/roko`.
> Parent pack index: [`../00-INDEX.md`](../00-INDEX.md).

## What this is

This backlog turns the findings of the 107-doc **status-quo pack**
(`tmp/status-quo/00–106`) into roko-native, **agent-executable tasks**. Every task is
authored to the canonical `tasks.toml` schema that `crates/roko-cli/src/task_parser.rs`
deserializes and `plan_validate.rs` enforces — each carries verify commands, gates, and
acceptance criteria, grouped into **18 epics (E01–E18)** and milestones. Where the
status-quo audit says an issue is open, the task that targets it is still current: the
audit and this backlog share the **same HEAD** (`5852c93c05`).

> ### ⚠ BOOTSTRAP — read this first
> **Roko cannot self-execute this backlog until E01 lands.** `roko plan run <dir>` with no
> flags currently defaults to the **Graph engine**, a dry-run stub: it prints `SUCCESS` in
> ~2 s, spawns 0 agents, spends $0, and changes no files. An autonomous agent following the
> self-hosting workflow will "run" every epic, see green, and do nothing. **E01 (flip the
> engine default to Runner v2)** is the gate on everything else. Until it lands, run plans
> explicitly with `--engine runner-v2`. Details: [`04-EXECUTION-READINESS.md`](04-EXECUTION-READINESS.md).

## Start here

| Doc | What it gives you |
|---|---|
| [`03-WORK-BREAKDOWN-EPICS.md`](03-WORK-BREAKDOWN-EPICS.md) | Roadmap, epic DAG, milestone sequencing (M0→M3+), critical path, parallel tracks |
| [`05-MASTER-CHECKLIST.md`](05-MASTER-CHECKLIST.md) | The flat, tickable checklist across all epics (149 tasks by milestone) |
| [`04-EXECUTION-READINESS.md`](04-EXECUTION-READINESS.md) | **M0 bootstrap** — the gate before every epic; the one fix that unblocks self-execution |
| [`01-TASK-EXECUTION-SCHEMA.md`](01-TASK-EXECUTION-SCHEMA.md) | Canonical `tasks.toml` schema — how to author a roko-executable task |
| [`02-PLANS-RECONCILIATION.md`](02-PLANS-RECONCILIATION.md) | Bridge from the authored `plans/` backlog to the status-quo findings (currency + coverage map) |

## Epics

Task count = distinct authored `EXX-Tnn` IDs in the epic file. Milestone/gate is stated
where the epic declares it; otherwise the epic's own **Depends on** is shown (most gate on
E01). The full M0→M3+ sequencing, DAG, and critical path live in
[`03-WORK-BREAKDOWN-EPICS.md`](03-WORK-BREAKDOWN-EPICS.md).

| Epic | Title | Milestone / gate | Tasks | Goal |
|---|---|---|---|---|
| [E01](epics/E01-EXECUTION-ENGINE.md) | Execution Engine | **M0 — bootstrap** | 10 | Make bare `plan run` spawn real agents, run gates, persist episodes/snapshots, resume — flip the engine default off the dry-run Graph |
| [E02](epics/E02-STORAGE-CONVERGENCE.md) | Storage Convergence | dep E01 | 12 | One canonical writer per durable `.roko/` concern so dashboards read what gates actually write |
| [E03](epics/E03-TYPE-CONSOLIDATION.md) | Type Consolidation | unblocks E02/E10 | 7 | Collapse 19 cross-crate duplicate type families to single definitions with real conversions |
| [E04](epics/E04-SECURITY-PERIMETER.md) | Security Perimeter | self-exec prereq | 19 | Close three exploitable P0s and enforce the safety funnel + audit chain before unattended self-execution |
| [E05](epics/E05-GATE-ADAPTIVITY-LIVE.md) | Gate Adaptivity on the Live Path | dep E01 | 8 | Make the live gate path honest: real rung inputs, non-passing stubs, per-rung stats that persist |
| [E06](epics/E06-COMPOSE-UNIFY.md) | Compose / Prompt Unification | dep E01 | 9 | Route the default Runner-v2 prompt path through the canonical roko-compose stack; retire 4 parallel assemblers |
| [E07](epics/E07-LEARNING-KNOWLEDGE.md) | Learning & Knowledge Loops | dep E01 | 10 | Make write-only learning loops durable & closed (persist LinUCB, credit the knowledge economy, wire HDC) |
| [E08](epics/E08-CONDUCTOR-SUPERVISION.md) | Conductor Supervision | dep E01 | 7 | Wire reactive anomaly supervision (ghost-turn, compile-loop, cost-blowout) into the live event loop |
| [E09](epics/E09-OBSERVABILITY.md) | Observability | dep E01 | 9 | Thread the built `MetricRegistry` into `RunConfig`, rotate runaway logs, give operators a trustworthy window |
| [E10](epics/E10-FRONTEND-CONTRACT.md) | Frontend / API Contract | dep E03 | 7 | Fix the web dashboard's wire contract with `roko serve` (404s, camel/snake, double SSE, replay) |
| [E11](epics/E11-CHAIN-ISFR.md) | Chain / ISFR | Phase 2+ (subset now) | 5 | Recover the core queue, implement `get_logs`, reach deploy parity for the DeFi critical-path subset (client side only) |
| [E12](epics/E12-DEAD-CODE-CLEANUP.md) | Dead-Code & Legacy Cleanup | dep E05/E06/E08 | 9 | Delete the ~52K-LOC legacy `orchestrate.rs` island after its live value is ported out |
| [E13](epics/E13-SPEC-DEBT-V2.md) | v2 Spec-Debt (long-horizon) | **M3+** | 3 | Triage ~55 zero-code v2 concepts; author tasks only for load-bearing survivors (e.g. `Lens`) — must not gate M0–M2 |
| [E14](epics/E14-PROVIDERS-TOOLS.md) | Providers & Tools | dep E01 | 7 | Harden the dispatch path: retries retry, tools survive per provider, every advertised builtin is executable |
| [E15](epics/E15-MCP-CONFIG.md) | MCP Config & Passthrough | dep E01 | 6 | Fix the MCP seams so tools actually reach the agent (config-shape normalizer first) |
| [E16](epics/E16-PRD-SELF-HOSTING.md) | PRD & Self-Hosting Pipeline | dep E01/E14 | 2 | Close the generative front-half (idea→draft→research→plan); 2 gap tasks atop plans P08/P09/P23 |
| [E17](epics/E17-ACP-COMPLETION.md) | ACP Completion | dep E04/E07/E15 | 6 | Make an editor-driven ACP turn behave like a `plan run` turn: consent-gated, learning-informed, MCP-equipped, honest |
| [E18](epics/E18-DOCS-CONFIG-OPS.md) | Docs, Config, CI & Ops Hygiene | dep E01 | 13 | Stop the repo lying to its readers and make the release pipeline prove what it claims |

Total: **149 authored tasks** across 18 epics.

## Exemplars & references

**Exemplars** — drop-in `tasks.toml` plans, each `roko plan validate`-clean and checked
against HEAD `5852c93c05`. Copy their shape when authoring real backlog plans.

| File | Demonstrates |
|---|---|
| [`exemplars/EX01-flip-engine-default.toml`](exemplars/EX01-flip-engine-default.toml) | The E01/M0 bootstrap task — flip the engine default |
| [`exemplars/EX02-unify-signal-store.toml`](exemplars/EX02-unify-signal-store.toml) | The E02 flagship — unify the signal store |
| [`exemplars/EX03-delete-orphan-statehub.toml`](exemplars/EX03-delete-orphan-statehub.toml) | An E12-style deletion task — delete an orphan StateHub |

**References**

| File | What |
|---|---|
| [`references/PLANNING-METHODOLOGY.md`](references/PLANNING-METHODOLOGY.md) | Cited (2024–2026) best practice for decomposing/sizing/gating agent-executable work, mapped to roko's schema |
| [`GAP-REPORT-V3.md`](GAP-REPORT-V3.md) | Coverage gaps in the status-quo pack (real-code-undocumented vs spec-only), feeding E13 spec-debt |

## How to execute

A human or a roko agent runs an epic like this:

1. Pick an epic (start with **E01** — nothing else is trustworthy until it lands).
2. Copy its authored tasks into a plan directory: `plans/<name>/tasks.toml` (use the
   [exemplars](exemplars/) as the drop-in template for shape/fields).
3. Lint without executing: `cargo run -p roko-cli -- plan validate plans/<name>`.
4. Execute on the live engine:
   `cargo run -p roko-cli -- plan run plans/<name> --engine runner-v2`.

> The explicit `--engine runner-v2` is **mandatory until E01 lands** — the bare default is
> the dry-run Graph stub. Once E01 flips the default, `roko plan run plans/<name>` alone
> does real work. See [`04-EXECUTION-READINESS.md`](04-EXECUTION-READINESS.md) and
> [`01-TASK-EXECUTION-SCHEMA.md`](01-TASK-EXECUTION-SCHEMA.md).

---

_Back to the full status-quo pack: [`../00-INDEX.md`](../00-INDEX.md)._
