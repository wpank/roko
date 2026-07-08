# Prompt: 01-orchestration

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate the `01-orchestration/` folder at `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/`.
This topic covers the orchestration layer (L4): plan DAGs, parallel executors, merge queues,
worktrees, snapshot/recovery, stigmergic coordination via git, niche construction, and
cross-domain orchestration.

## Step 1 — Read the context pack (MANDATORY, in order)

Read these 7 files in order:

1. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/00-ALWAYS-READ-FIRST.md`
2. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/01-naming-map.md`
3. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/02-reframe-rules.md`
4. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/03-concepts-lifecycle.md`
5. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/04-writing-rules.md`
6. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/05-source-files.md`
7. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/06-output-structure.md`

## Step 2 — Read canonical refactoring-prd sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/02-five-layers.md` — §Layer 4 Orchestration, §Stigmergy, §Cross-Domain Orchestration (full sections)
2. `/Users/will/dev/nunchi/roko/refactoring-prd/05-agent-types.md` — §7 Multi-Agent Orchestration (worktrees, pools, HEFT, Conductor Yerkes-Dodson), §Niche Construction
3. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` — §Tier 1 (production hardening)
4. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md` — reframe rules
5. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` — §IX Forensic AI (causal replay of multi-agent decisions)

## Step 3 — Read SOURCE-INDEX entry

Read `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/SOURCE-INDEX.md` and find the section `## 01-orchestration.md`. Read every file listed there (both legacy PRD, legacy tmp, and implementation-plans sources).

## Step 4 — Read implementation-plan sources

1. `/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/04-orchestrator-pipeline.md`
2. `/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/11-agent-dogfooding.md` §Phase 3-4
3. `/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/11-sections/phase-3-4.md`

## Step 5 — Read active code

Use Glob to find `.rs` files in `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/` and read the key files: `executor.rs`, `merge_queue.rs`, `dag.rs`, `worktree.rs`, and any safety-related files. Also read `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` — the 766-line runtime harness.

## Step 6 — Create output directory and plan sub-docs

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/01-orchestration
```

Write **14 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-plan-discovery-and-unified-dag.md` | Plan discovery from `plans/` directories. UnifiedTaskDag: nodes (tasks) and edges (dependencies). How plans are parsed from TOML task files. Plan lifecycle (drafting → active → blocked → completed). |
| 01 | `01-parallel-executor-state-machine.md` | The 14-phase executor state machine. Wave planning. Task pickup. Gate pipeline integration. State transitions. Error policies. Drawing from mori's parallel executor and the `orchestrate.rs` runtime harness. |
| 02 | `02-merge-queue-and-file-serialization.md` | Merge queue. File-conflict serialization. Why merges are sequential even when tasks are parallel. Post-merge regression testing. |
| 03 | `03-worktree-management.md` | Git worktree create/remove/list/prune/health. Per-agent worktree isolation. Worktree lifecycle tied to task lifecycle. |
| 04 | `04-snapshot-and-crash-recovery.md` | Snapshot format (`.roko/state/executor.json`). Crash resilience. Resume from snapshot (`roko plan run --resume`). What gets persisted, what doesn't. |
| 05 | `05-document-pipeline-prd-to-execution.md` | PRD → plan → tasks → implementation pipeline. `roko prd idea`, `roko prd draft new`, `roko prd plan`, `roko plan run`. The self-hosting loop. |
| 06 | `06-stigmergy-as-git-coordination.md` | Git as the shared environment. Each commit is a pheromone. Agents read worktree state, modify it, commit, leave traces. O(1) coordination scaling. Grassé 1959, Parunak 2002, Dorigo 1997 ACO. |
| 07 | `07-niche-construction-theory.md` | Odling-Smee et al. 2003 niche construction. Agents construct the codebase they operate in. Positive vs. negative niche construction. Affordance assessment. Information scent (Pirolli & Card 1999). MVT stopping rule (Charnov 1976). |
| 08 | `08-yerkes-dodson-multi-agent-dynamics.md` | Yerkes-Dodson law applied to multi-agent cooperation. Moderate pressure maximizes cooperation. Extreme pressure causes collapse in 5-12 turns. The Conductor adjusts pressure parameters dynamically. |
| 09 | `09-agent-pools-and-heft-scheduling.md` | AgentPool (sequential) and MultiAgentPool (parallel with warm spawning). Per-agent metrics (success rate, latency, cost, C-Factor contribution). HEFT (Heterogeneous Earliest Finish Time) scheduling with critical path + earliest finish time heuristics. |
| 10 | `10-cross-domain-orchestration.md` | Multi-domain plans: coding + chain + research tasks in one DAG. Different gates per task type. Knowledge flows between domains via shared Substrate. Example: Deploy new DeFi strategy with custom contract (5 tasks spanning coding, chain, research). |
| 11 | `11-conductor-integration.md` | How the Conductor watches multi-agent behavior. Theory-of-mind about the pipeline. Dynamic pressure adjustment. Integration with gate results. Cross-reference topic 07-conductor for depth. |
| 12 | `12-academic-foundations.md` | Full academic grounding: Hewitt 1973 Actor Model, Agha 1986, Erlang/OTP Armstrong 2003, Hoare 1978 CSP, Milner 1999 Pi-calculus, Graham 1966 scheduling, MetaGPT, ChatDev, AutoGen, CAMEL, HEFT, Yerkes-Dodson, Odling-Smee 2003, Grassé 1959, Parunak 2002, Charnov 1976, Pirolli & Card 1999. Every citation from sources. |
| 13 | `13-current-status-and-gaps.md` | What's built in roko-orchestrator (158 tests). The orchestrate.rs runtime harness. What's wired vs. scaffold. Known gaps from `11-inconsistencies.md` (e.g., dispatcher-not-called gap is relevant context). Implementation-plans references. |

Plus `INDEX.md` linking everything.

## Step 7 — Writing rules (from context-pack/04-writing-rules.md)

- DO NOT SUMMARIZE. Full substance.
- DO NOT TRUNCATE. Split sub-docs if too long, never shrink content.
- PRESERVE ALL CITATIONS.
- Minimum 200 lines per sub-doc. Target 500-1500 lines per substantial doc.
- Zero-context reader. Define every term.
- Apply naming map: mori→Roko Orchestrator, golem→agent, clade→collective/mesh.
- No death/mortality framing.
- Use Write tool with absolute paths.

## Step 8 — Write INDEX.md

Follow schema in `context-pack/06-output-structure.md`. Link all 14 sub-docs. Cross-reference topics 00-architecture (foundation), 02-agents (who runs the tasks), 04-verification (gates called per task), 07-conductor (watches pipeline), 13-coordination (stigmergy deep dive).

## Step 9 — Self-check

- [ ] 14 sub-docs + INDEX.md exist
- [ ] Each sub-doc ≥200 lines
- [ ] Total topic ≥3500 lines
- [ ] No forbidden terms (golem except in rename tables, fleet, Thanatopsis, terminal requiem)
- [ ] Required terms present (Roko, Engram, Synapse)
- [ ] ≥15 academic citations total

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL CITATIONS.
- No death framing.
- Apply naming map (mori→Roko Orchestrator, golem→agent, clade→collective/mesh).
- Make Roko Orchestrator (was "Mori") the primary framing. It's one of Roko's primary applications.
- Frame blockchain as just one kind of task in a plan, not the default.
- Use Write tool. Absolute paths. Don't ask questions. Make decisions and continue.
