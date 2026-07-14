# Plan Quality Audit: E19-E30

Audited: 2026-07-10
Scope: 12 epic plans, 115 total tasks

## Summary

| Plan | Tasks | Schema | Paths | Verify | Verdict |
|------|-------|--------|-------|--------|---------|
| E19-signal-protocol | 10 | PASS | PASS | PASS | PASS |
| E20-cell-unification | 10 | PASS | PASS | PASS | PASS |
| E21-graph-engine | 10 | PASS | PASS | PASS | PASS |
| E22-execution-runtime | 10 | PASS | PASS* | PASS | PASS (minor) |
| E23-agent-cognitive-autonomy | 10 | PASS | PASS | PASS | PASS |
| E24-memory-advanced | 10 | PASS | PASS | PASS | PASS |
| E25-learning-loops-advanced | 10 | PASS | PASS | PASS | PASS |
| E26-inference-gateway | 12 | PASS* | PASS** | PASS | PASS (notes) |
| E27-feeds-system | 8 | PASS | PASS | PASS | PASS |
| E28-groups-coordination | 8 | PASS | PASS** | PASS | PASS (notes) |
| E29-connectivity-relay | 9 | PASS | PASS** | PASS | PASS (notes) |
| E30-extension-system | 8 | PASS | PASS | PASS | PASS |

**Overall: 12/12 plans pass. 0 blockers. 6 minor notes.**

---

## 1. Schema Compliance

### Checklist (all plans)

- [x] Has `[meta]` with `plan`, `total`, `status` -- all 12 pass
- [x] Uses `[[task]]` (not `[[tasks]]`) -- all 12 pass
- [x] Every task has `id`, `title`, `role` -- all 12 pass
- [x] Every implementer task has `files` + `verify` -- all 12 pass
- [x] No duplicate task IDs -- all 12 pass
- [x] `depends_on` references exist within the plan -- all 12 pass
- [x] Tiers are valid (mechanical/focused/integrative/architectural) -- all 12 pass

### Schema Notes

- **E26** has `total = 12` (matches 12 tasks), which is fine but unusual (most plans have 8-10). The extra tasks are warranted by the gateway's complexity.
- All plans have `max_parallel` set (1-3), which is good practice.
- All plans have `done = 0` and `status = "ready"`.
- E21, E22, E26 use `depends_on_plan` for cross-epic dependencies -- this is a useful extension beyond the base schema (not standard in the tasks.toml schema but harmless and informative).

---

## 2. File Path Validity

### Files that EXIST (implementation targets)

All `files` arrays in E19-E25 reference existing source files. Verified:

- `crates/roko-core/src/{engram,pulse,provenance,kind,demurrage,signal,body,score,cell,prediction,feed,connector,extension}.rs` -- all exist
- `crates/roko-core/src/lib.rs`, `crates/roko-core/src/config/{schema,chain,budget}.rs` -- all exist
- `crates/roko-graph/src/{types,engine,loader,hot,condition,topo,error,budget,cell,lib}.rs` -- all exist
- `crates/roko-graph/src/cells/mod.rs` -- exists
- `crates/roko-std/src/defaults.rs` -- exists
- `crates/roko-daimon/src/{lib,goals,mortality}.rs` -- all exist
- `crates/roko-runtime/src/heartbeat.rs` -- exists
- `crates/roko-learn/src/{active_inference,cfactor,aggregate,prompt_experiment,playbook,playbook_rules,episode_logger,hdc_clustering,hdc_fingerprint,efficiency,cost_table,cascade_router}.rs` -- all exist
- `crates/roko-neuro/src/{lib,admission,knowledge_store,hdc,temporal,lifecycle}.rs` -- all exist
- `crates/roko-dreams/src/runner.rs` -- exists
- `crates/roko-compose/src/enrichment/mod.rs` -- exists
- `crates/roko-cli/src/{orchestrate,commands/feed,runner/event_loop,runner/extension_loader}.rs` -- all exist
- `crates/roko-serve/src/{routes/feeds,routes/mod,routes/ws,relay,events}.rs` -- all exist
- `crates/roko-agent/src/{lifecycle,safety/capabilities,agent,cache,gateway_events,dispatcher/tool_selector}.rs` -- all exist
- `crates/roko-agent-server/src/registration.rs` -- exists
- `crates/roko-plugin/src/manifest.rs` -- exists
- `crates/roko-orchestrator/src/{dag,merge_queue,executor/snapshot,runtime_snapshot,event_log}.rs` -- all exist

### Files that DO NOT EXIST YET (new files to create)

These are intentional -- tasks that create new files or modules:

| File | Plan | Task | Notes |
|------|------|------|-------|
| `crates/roko-gateway/` (entire crate) | E26 | T01 | New crate -- task explicitly says "Create a new crate". OK. |
| `crates/roko-gateway/src/*.rs` (8 modules) | E26 | T02-T12 | All depend on T01 creating the crate. OK. |
| `crates/roko-core/src/groups.rs` | E28 | T01 | New module -- task says "Create crates/roko-core/src/groups.rs". OK. |
| `crates/roko-core/src/wire_protocol.rs` | E29 | T03 | New module -- task says add to lib.rs. OK. |
| `crates/roko-core/src/exoskeleton.rs` | E29 | T06 | New module -- task says add to lib.rs. OK. |
| `crates/roko-graph/src/cells/cognitive.rs` | E22 | T01 | New module -- task explicitly creates it. OK. |
| `crates/roko-serve/src/routes/groups.rs` | E28 | T05 | New route module -- task explicitly creates it. OK. |
| `.roko/graphs/cognitive-loop.toml` | E22 | T02 | New TOML definition file -- task creates it. OK. |

### Context read_files Validity

All `context.read_files` paths reference existing files. Spot-checked line ranges are reasonable for file sizes. No issues found.

### Docs References

All referenced docs exist:
- `docs/v2/{01..12}-*.md` -- all present
- `docs/v2-depth/` subdirectories -- all referenced files verified present

---

## 3. Verify Command Quality

### All plans have both structural and compile phases

Every implementer task has at least:
1. A `structural` phase (`grep -q` checking for key types/methods)
2. A `compile` phase (`cargo check -p <crate>` or `cargo test`)

### Verify commands are real shell commands (not placeholders)

All verify commands use:
- `grep -q` for structural checks (pattern presence)
- `cargo check -p <crate> 2>&1` for compilation
- `cargo test -p <crate> [filter] 2>&1` for test phases
- `test -f <path>` for file existence (E26 new crate files)
- `grep -c ... | grep -qE` for count validation (E20-T09)
- `wc -l | grep -q` for count validation (E20-T07)

All are valid, executable shell commands.

### Test-phase Coverage

| Plan | Tasks with test phase | Notes |
|------|----------------------|-------|
| E19 | T01, T02, T03(no test), T05, T06, T07, T09 | T03, T04, T08 lack test phase -- focused tasks with compile-only is acceptable |
| E20 | T01, T09 | Most tasks are additive types, compile is sufficient |
| E21 | T04 | Graph engine tests |
| E22 | None (all compile-only) | Acceptable -- new types being defined |
| E23 | T10 (dedicated test task) | Good pattern -- test task at end |
| E24 | T10 (dedicated test task) | Good pattern |
| E25 | T10 (dedicated test task) | Good pattern |
| E26 | None (all compile-only) | Acceptable -- new crate with new types |
| E27 | T01, T06 (implicit via `cargo test --lib`) | Config tests validated |
| E28 | T06 (config tests) | Config tests validated |
| E29 | T02 (connector tests) | Existing tests validated |
| E30 | T05, T06, T07 (extension tests) | Good coverage |

---

## 4. Per-Plan Detailed Notes

### E19-signal-protocol -- PASS
- 10 tasks, well-structured dependency chain
- Clean task decomposition: enum (T01) -> transitions (T02) -> bridges (T03-T04) -> economics (T05) -> registry (T06) -> IFC (T07) -> fingerprint (T08) -> lineage (T09) -> re-exports (T10)
- All files exist, all verify commands valid
- No issues

### E20-cell-unification -- PASS
- 10 tasks, focused on enriching the Cell trait
- All modifications to existing `cell.rs` -- file exists
- T07 (Cell supertrait) has `max_loc = 200` which is appropriate for the integrative change
- T09 depends on T07 correctly (Cell impl needs supertrait first)
- No issues

### E21-graph-engine -- PASS
- 10 tasks covering typed edges, policies, parallel execution, snapshots, Hot graph, fractal composition
- Uses `depends_on_plan = ["E20-cell-unification"]` in T01/T07 -- good cross-epic tracking
- All target files exist in roko-graph
- No issues

### E22-execution-runtime -- PASS (minor note)
- 10 tasks defining cognitive loop cells
- **Note**: T01 creates `crates/roko-graph/src/cells/cognitive.rs` (does not exist yet) and T02 creates `.roko/graphs/cognitive-loop.toml` (does not exist yet) -- both are valid creation tasks
- Good T0 short-circuit optimization task (T03)
- No blockers

### E23-agent-cognitive-autonomy -- PASS
- 10 tasks across roko-agent, roko-daimon, roko-runtime, roko-learn, roko-cli
- Task T10 is a dedicated test task covering type-state, behavioral phases, and energy -- good pattern
- T09 correctly depends on T02, T04, T05 (wiring task after component tasks)
- No issues

### E24-memory-advanced -- PASS
- 10 tasks focused on heuristics, temporal knowledge, HDC resonator, demurrage ODE
- T10 is a dedicated test task -- good pattern
- T07 (demurrage ODE) correctly depends on T05 (income policy)
- No issues

### E25-learning-loops-advanced -- PASS
- 10 tasks for defragmentation, hindsight relabeling, c-factor governance, significance testing
- T10 is a dedicated test task -- good pattern
- T09 (orchestrate.rs wiring) depends on T03, T05, T07 correctly
- No issues

### E26-inference-gateway -- PASS (notes)
- 12 tasks (largest plan) for the new roko-gateway crate
- **Note**: `crates/roko-gateway/` does not exist yet -- T01 creates it. All subsequent tasks (T02-T12) depend on T01, so the dependency chain is correct.
- T12 (pipeline assembly) depends on T02-T10 (all 9 pipeline stages) -- correct and complete
- Uses `tier = "architectural"` for T03 (cache) and T12 (pipeline assembly) -- appropriate
- T08 verify includes a negative check (`! grep -q 'api_key'`) -- good security practice
- **Note**: E26 `total = 12` which is above the 8-10 norm. The gateway's 9-stage pipeline justifies this.

### E27-feeds-system -- PASS
- 8 tasks, well-scoped to feed types, registry, config, routes, CLI
- T01 has 3 verify phases (structural + 2 compile) -- thorough
- T06 validates config backward compat via `cargo test -p roko-core --lib config`
- No issues

### E28-groups-coordination -- PASS (notes)
- 8 tasks for Group types, invitations, leaders, pheromones, API routes, config
- **Note**: T05 creates `crates/roko-serve/src/routes/groups.rs` (does not exist yet) -- valid creation task
- T06 validates config backward compat -- good
- T08 verify checks for both enum variant and method -- thorough
- No issues

### E29-connectivity-relay -- PASS (notes)
- 9 tasks for Connect protocol, wire protocol, reconnection, finality, exoskeleton
- **Note**: T03 creates `crates/roko-core/src/wire_protocol.rs` and T06 creates `crates/roko-core/src/exoskeleton.rs` (neither exists yet) -- valid creation tasks with lib.rs registration checks
- T02 validates existing connector tests still pass -- good backward compat check
- No issues

### E30-extension-system -- PASS
- 8 tasks for CaMeL IFC tags, hooks, manifest, dependency resolution, fault isolation
- Good progression: types (T01-T02) -> hooks (T03) -> manifest (T04) -> resolution (T05) -> fault isolation (T06) -> propagation (T07) -> wiring (T08)
- T05, T06, T07 all verify with `cargo test -p roko-core -- extension` -- good test coverage
- No issues

---

## 5. Cross-Cutting Observations

### Strengths
1. **Consistent schema**: All 12 plans follow identical TOML schema conventions
2. **Real verify commands**: No placeholders, all commands are executable
3. **Good dependency chains**: `depends_on` references are all valid within each plan
4. **Cross-epic awareness**: Plans use `depends_on_plan` to declare inter-epic dependencies (E21->E20, E22->E20/E21, etc.)
5. **Context richness**: Every task has `read_files` with line ranges and `why` explanations
6. **Anti-patterns**: Every task documents what NOT to do
7. **Tier accuracy**: mechanical/focused/integrative/architectural tiers match task complexity
8. **New file creation**: Tasks that create new files/crates explicitly state it in description and have `test -f` verify checks

### Minor Issues (non-blocking)

1. **No `domain` field on E23-E30 tasks**: Plans E19-E22 include `domain = "..."` on each task. Plans E23-E30 omit it. Not required by schema but inconsistent.

2. **E26 `total = 12` vs convention**: Most plans have 8-10 tasks. E26 has 12. This is justified by complexity but may need splitting if execution struggles with fan-in at T12 (depends on 9 tasks).

3. **E22-T02 file target is non-source**: `.roko/graphs/cognitive-loop.toml` is a runtime artifact, not source code. This is intentional (it's a Graph definition file) but unusual for a plan task.

4. **E23-E25 test tasks (T10) have no `files` field**: The dedicated test tasks list files in `files` correctly but some only have test-phase verify (no structural check). This is acceptable since they're test-only tasks.

5. **model_hint inconsistency**: E19-E22 use `model_hint = "claude-sonnet-4-5"` or `"claude-haiku-4-5"` on every task. E23-E30 omit `model_hint` entirely. Not required but inconsistent.

6. **max_loc range**: Ranges from 10 to 200. Most tasks are 40-60. The 200-LOC tasks (E20-T07, E22-T01, E26-T03, E26-T12, E28-T05) are all integrative/architectural tier, which is appropriate.

---

## 6. Recommendations

1. **No blocking fixes needed.** All 12 plans are schema-compliant and execution-ready.

2. **Optional consistency fixes**:
   - Add `domain` field to E23-E30 tasks for filtering consistency
   - Add `model_hint` to E23-E30 tasks for CascadeRouter guidance
   - These are cosmetic and do not affect executability

3. **E26 execution note**: The T12 fan-in (depends on 9 tasks) means T12 cannot start until all pipeline stages are complete. Consider whether any stages could be parallelized during execution by relaxing some dependencies (e.g., T06 ThinkingCap and T04 ToolPrune are independent of each other, and both could proceed before T12 assembles them).

4. **Cross-epic dependency tracking**: The `depends_on_plan` field is used informally. Consider standardizing it in the schema so the executor can block cross-epic tasks until prerequisites are marked done.
