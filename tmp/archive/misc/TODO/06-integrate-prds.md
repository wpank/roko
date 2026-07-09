# integrate-prds/ — PRD Integration Tracker

**Directory**: `tmp/integrate-prds/`
**Status**: ACTIVE — Phase A done, Phase B 85%, Phase C 15%
**Files**: 13 documents covering mechanical refactoring, structural wiring, and new features

## Phase A: Mechanical Refactoring — 100% DONE

| Item | Status | Source |
|------|--------|--------|
| Rename bardo-runtime -> roko-runtime | DONE | `crates/roko-runtime/` |
| Rename bardo-primitives -> roko-primitives | DONE | `crates/roko-primitives/` |
| Dissolve roko-golem | DONE | Hypnagogia -> roko-dreams, witness -> roko-chain |
| Signal -> Engram rename | DONE | `crates/roko-core/src/engram.rs` |
| Update workspace metadata | DONE | Root `Cargo.toml` |

## Phase B: Structural Wiring — 85% DONE

| Item | Status | Source |
|------|--------|--------|
| SafetyLayer into orchestrator | DONE | `crates/roko-agent/src/safety/mod.rs`, `orchestrate.rs:4605` |
| Neuro into context assembly | WIRED | `orchestrate.rs:2728` |
| Daimon into dispatch | WIRED | `orchestrate.rs:3287-3319` |
| Conductor into executor | PARTIAL | `orchestrate.rs:755-775` (circuit breaker); intervention policy partial |
| task.verify execution | WIRED | `orchestrate.rs:2855-2863` |
| read_files injection | WIRED | `orchestrate.rs:18014-18070` |
| SkillLibrary extract/inject | WIRED | `crates/roko-learn/src/skill_library.rs`, `orchestrate.rs:4571` |
| CascadeRouter feedback | WIRED | `orchestrate.rs:10123-10284` |
| model_hint override | WIRED | `orchestrate.rs:3971-3984` |
| Worktree isolation | WIRED | `crates/roko-orchestrator/src/worktree.rs`, `orchestrate.rs:5880` |
| VCG Auction | PARTIAL | 8-bidder auction works; greedy not optimal |
| Emotional tagging | WIRED | `crates/roko-core/src/engram.rs` (EmotionalTag) |
| Attestation signing | PARTIAL | Type extension done; signing workflow missing |
| Lineage tracking | PARTIAL | Field exists; runtime emission inconsistent |

Phase B remaining checklist:

- [ ] Wire broader conductor intervention policy (beyond circuit breaker)
- [ ] Implement welfare-maximizing knapsack for VCG auction (currently greedy)
- [ ] Wire attestation signing workflow
- [ ] Make lineage tracking consistent across dispatch/policy/persistence flows

## Phase C: New Features — 15% DONE

| Item | Status | Notes |
|------|--------|-------|
| NREM Replay Modes | SCAFFOLD | No Mattar-Daw utility formula |
| REM Imagination | NOT DONE | No Pearl structural causal models |
| Hypnagogia Engine | MOVED | `crates/roko-dreams/src/hypnagogia.rs` — not activated |
| Heartbeat Theta/Delta | GAMMA ONLY | Theta (~75s) and Delta (hours) not integrated |
| T0 Probes | NOT DONE | 0 of 16 zero-LLM probes |
| EWC Regularizer | NOT DONE | Bandit learning unstable on new task types |
| Curriculum Learning | NOT DONE | No DifficultyModel |
| Pheromone System | NOT DONE | 0 code |
| Agent Mesh (P2P) | NOT DONE | 0 code |
| Code Intelligence MCP | PARTIAL | Built, not wired to agents |
| DAG Optimization | NOT DONE | No CPM, fusion, speculation, or partitioning |
| Dynamic DAG Mutation | NOT DONE | No DagMutation enum |
| Agent Composition | NOT DONE | No CompositeAgent |
| OCaps Security | NOT DONE | RBAC only; no capability tokens |
| Supervision Strategies | NOT DONE | No Erlang/OTP restart strategies |

Phase C checklist (prioritized):

- [ ] Wire Heartbeat Theta loop (MetaCognitionHook exists, not called periodically)
- [ ] Implement T0 Probes (zero-LLM cost; 16 probes specced)
- [ ] Wire Code Intelligence MCP to agents during dispatch
- [ ] Implement DAG optimization passes (CPM, task fusion)
- [ ] Activate hypnagogia engine in dreams crate
- [ ] Implement NREM replay strategies
- [ ] Implement curriculum learning (DifficultyModel)

## Source Files

- **Integration docs**: `tmp/integrate-prds/*.md`
- **Safety wiring**: `crates/roko-agent/src/safety/`, `crates/roko-cli/src/orchestrate.rs`
- **Neuro wiring**: `crates/roko-neuro/src/`, `orchestrate.rs:128,2728`
- **Daimon wiring**: `crates/roko-daimon/src/`, `orchestrate.rs:48-72,3287-3319`
- **Dreams**: `crates/roko-dreams/src/`
- **Code intel**: `crates/roko-index/`, `crates/roko-mcp-code/`
- **Orchestrator DAG**: `crates/roko-orchestrator/src/dag.rs`
