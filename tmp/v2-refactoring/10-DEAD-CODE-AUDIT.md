# Dead Code Audit — Delete vs Wire vs Keep

## Decision Framework

For each piece of floating code, one of three decisions:

| Decision | Criteria |
|----------|----------|
| **DELETE** | No v2 equivalent, or v2 supersedes it entirely |
| **WIRE** | Has a v2 role AND a concrete wire target exists today |
| **KEEP (tag)** | Has a v2 role but wire target requires Phase 2+ work |

"Keep" means: add a status comment, don't delete, but don't pretend it's wired.

## orchestrate.rs — 23,331 lines

**Decision: DELETE (feature-gate is already done, remove entirely when confident)**

- Already behind `legacy-orchestrate` feature, not enabled by default
- Runner v2 replaced it completely
- Keeping it adds confusion: AI assistants reference it, new contributors think it's active
- The useful patterns (system prompt builder integration, episode logging) are
  already reimplemented in Runner v2 or roko-compose

**Action**: After confirming nothing depends on it, remove the file and feature flag.
Keep a note in CLAUDE.md about the removal.

## roko-runtime (8 floating modules)

| Module | LOC | Decision | Reasoning |
|--------|-----|----------|-----------|
| theta_consumer | ~200 | KEEP (tag) | v2 has React Cells for state drift — but needs Engine first |
| delta_consumer | ~200 | KEEP (tag) | Same — becomes a React Cell in Engine |
| demurrage_consumer | ~200 | WIRE | Can wire NOW: run periodic Store::prune() via tokio interval |
| energy | ~150 | KEEP (tag) | Maps to v2 cognitive energy in Hot Graphs — needs Engine |
| heartbeat_attention | ~200 | KEEP (tag) | Maps to v2 attention auction — needs Engine |
| heartbeat_probes | ~200 | KEEP (tag) | Maps to v2 T0 probes — needs Engine |
| run_ledger | ~200 | WIRE | Can wire NOW: track per-run costs in Runner v2 event loop |
| task_scheduler | ~300 | KEEP (tag) | Maps to v2 Trigger protocol — needs Engine |

**Immediate actions**:
- Wire `demurrage_consumer`: add a periodic task in `roko serve` that calls Store::prune()
- Wire `run_ledger`: add cost tracking to Runner v2's per-task completion handler
- Tag the other 6 with `//! STATUS: NOT WIRED — requires Engine (Phase 2)`

## roko-learn (14 floating modules)

| Module | LOC | Decision | Reasoning |
|--------|-----|----------|-----------|
| active_inference | ~300 | KEEP (tag) | Maps to v2 EFE routing — needs Engine |
| baseline | ~200 | WIRE | Can wire NOW: add regression checks after gate pipeline |
| bayesian_confidence | ~200 | KEEP (tag) | Maps to v2 predict-publish-correct — needs Bus wiring |
| calibration_policy | ~300 | WIRE | Can wire NOW: connect to CascadeRouter (see QW-8) |
| error_enrichment | ~200 | WIRE | Can wire NOW: enrich gate errors before retry prompt |
| event_subscriber | ~150 | KEEP (tag) | Generic Bus subscriber — needs Engine |
| jsonl_rotation | ~100 | WIRE | Can wire NOW: add log rotation to episode/efficiency logs |
| local_reward | ~200 | KEEP (tag) | Maps to v2 Verify reward signal — needs Engine |
| oracles | ~200 | KEEP (tag) | Maps to v2 Verify oracles — needs Engine |
| pareto | ~200 | KEEP (tag) | Maps to v2 multi-objective optimization — needs Engine |
| post_gate_reflection | ~200 | WIRE | Can wire NOW: add reflection step after gate failure |
| quality_judge | ~200 | KEEP (tag) | Maps to v2 Score protocol — needs Engine |
| section_outcome | ~150 | WIRE | Can wire NOW: track which prompt sections led to success |
| verdict_scorer | ~200 | KEEP (tag) | Maps to v2 Verify → reward — needs Engine |

**Immediate actions** (6 items can be wired now):
- Wire `calibration_policy` → CascadeRouter feedback loop
- Wire `error_enrichment` → Runner v2 gate failure handler
- Wire `post_gate_reflection` → Runner v2 gate failure prompt shaping
- Wire `section_outcome` → EpisodeLogger after successful task
- Wire `baseline` → Gate pipeline regression detection
- Wire `jsonl_rotation` → `.roko/episodes.jsonl` and `.roko/learn/efficiency.jsonl`

## Language parsers (3 crates, ~5K LOC)

| Crate | Decision | Reasoning |
|-------|----------|-----------|
| roko-lang-rust | KEEP (tag) | Useful for code-intelligence context — needs MCP wiring |
| roko-lang-typescript | KEEP (tag) | Same |
| roko-lang-go | KEEP (tag) | Same |

**Action**: These should be wired into `roko-mcp-code` for code-intelligence
context assembly. Tag as "wire into MCP" until that happens.

## MCP integrations (3 crates)

| Crate | Decision | Reasoning |
|-------|----------|-----------|
| roko-mcp-github | WIRE | Can wire NOW: mount in agent MCP dispatch config |
| roko-mcp-slack | KEEP (tag) | Needs Slack token + webhook config — deferred |
| roko-mcp-scripts | WIRE | Can wire NOW: mount custom scripts as MCP tools |

## Other

| Item | Decision | Reasoning |
|------|----------|-----------|
| roko-calc | DELETE | Empty skeleton, no purpose |
| roko-acp | KEEP (tag) | Agent Compute Protocol — Phase 2+ |
| VCG auction | KEEP (tag) | Built in roko-compose, needs Engine to replace greedy path |
| Safety contracts | WIRE | Can wire NOW: load YAML contracts, enforce at dispatch |

## Summary

| Decision | Count | LOC estimate |
|----------|-------|-------------|
| DELETE | 2 items | ~23.5K (orchestrate.rs + roko-calc) |
| WIRE NOW | 10 items | ~2K (small integrations) |
| KEEP (tag) | 18 items | ~5K (needs Engine or Phase 2+) |

## Wiring Priority (items that can be wired NOW)

1. `calibration_policy` → CascadeRouter (highest impact: closes learning loop)
2. `error_enrichment` → Runner v2 gate failure (improves retry success)
3. `post_gate_reflection` → Runner v2 gate failure (improves next attempt)
4. `section_outcome` → EpisodeLogger (improves prompt quality over time)
5. `run_ledger` → Runner v2 event loop (cost visibility)
6. `demurrage_consumer` → roko serve periodic task (storage hygiene)
7. `jsonl_rotation` → episode/efficiency logs (prevents unbounded growth)
8. `baseline` → gate pipeline (regression detection)
9. `roko-mcp-github` → agent MCP config (agent capabilities)
10. `roko-mcp-scripts` → agent MCP config (custom tool support)
