# 31-06 -- Dogfood Evidence and Verification Status

> **Parent:** [31-SELF-HOSTING.md](../../31-SELF-HOSTING.md) section 6
> **Primary evidence:** 2026-08-13 dogfood run, regression fixes, deterministic
> self-host coverage
> **Cross-references:** [04-EXECUTION](../../04-EXECUTION.md) (graph engine,
> snapshot resume), `.roko/GAPS.md` (canonical gap tracker)

---

## 1. The 2026-08-13 Dogfood Run

The first live full self-hosting run was executed on 2026-08-13. The system
attempted to use its own plan-execute-gate-persist loop to implement a real
feature. The run exposed four blockers.

### 1.1 Environment

- Workspace: `/Users/will/dev/nunchi/roko/roko/`
- Binary: `target/debug/roko` (debug build)
- Providers: Anthropic API (Claude Opus), OpenAI-compatible (Claude Sonnet)
- Plans: An authored multi-task plan with cross-task dependencies
- Engine: Graph engine (post-#260 default)

### 1.2 Four Blockers Discovered

#### Blocker 1: Config Merge Failure

**Symptom:** Plan execution failed at startup with a confusing error about
missing configuration keys that were present in `roko.toml`.

**Root cause:** The layered configuration merge system did not correctly handle
nested TOML tables. When merging workspace-level config over default config,
nested tables were replaced wholesale rather than deep-merged. This meant that
a workspace config with `[learning]` that only set `replan_on_gate_failure`
would overwrite the entire `[learning]` section, losing all other defaults.

**Fix:** Deep-merge semantics for nested TOML tables in
`crates/roko-core/src/config/loader.rs`. Tables are merged key-by-key;
only leaf values are overwritten.

**Regression test:** Config merge test that verifies nested table preservation
across merge layers.

#### Blocker 2: Stale Snapshot Resume

**Symptom:** Resuming a plan after code changes produced errors about unknown
node IDs. The snapshot referenced nodes from the previous graph definition
that no longer existed in the updated graph.

**Root cause:** `GraphSnapshotV2` did not validate that the snapshot's graph
matched the current graph definition. A snapshot from run N could be loaded
for run N+1 even if the graph definition had changed between runs.

**Fix:** Graph fingerprint validation added to `GraphSnapshotV2`. The
fingerprint is a BLAKE3 hash of the graph definition (node IDs, edges,
cell types). On resume, the snapshot fingerprint is compared to the current
graph fingerprint. Mismatches produce a clear error suggesting `--fresh`.

**Regression test:** Snapshot resume test that verifies fingerprint mismatch
rejection.

**Source:** `crates/roko-graph/src/snapshot.rs` -- `graph_fingerprint` field

#### Blocker 3: fsmonitor Interference

**Symptom:** The TUI dashboard showed spurious file change events during
worktree operations, causing unnecessary rebuilds and confusing status
displays.

**Root cause:** macOS FSEvents watcher produced rapid-fire events during
`git worktree add` and `git worktree remove` operations. Each worktree
operation creates and modifies many files in `.git/worktrees/`, which the
file watcher treated as meaningful changes.

**Fix:** Debounce filter added to `crates/roko-cli/src/tui/fs_watch.rs`.
Events within the debounce window (100ms) are collapsed. Events from
`.git/worktrees/` paths are filtered out entirely.

**Regression test:** File watcher test that verifies debounce behavior and
`.git/` path filtering.

#### Blocker 4: Enrichment Phase Transition

**Symptom:** Plans stalled in the `Enriching` phase indefinitely. The
phase state machine did not transition to `Implementing` even though all
enrichment cells had completed.

**Root cause:** Phase transition logic checked whether enrichment cells had
produced non-empty output. But some enrichment cells legitimately produce
empty output (e.g., the KnowledgeCell when no relevant knowledge exists, or
the ExperimentCell when no active experiments match). Empty output was
treated as "not complete" rather than "complete with no enrichment."

**Fix:** Phase transition logic now treats empty output from enrichment cells
as a valid completion state. The condition changed from "all enrichers have
non-empty output" to "all enrichers have completed (with or without output)."

**Regression test:** Plan phase test that verifies transition with empty
enrichment output.

---

## 2. Fix Verification Status

All four fixes have regression tests that pass in the workspace test suite.
The deterministic self-host coverage (a test suite that exercises the
complete plan-execute-gate-persist pipeline with deterministic inputs) passes.

| Fix | Regression test | CI status |
|-----|----------------|-----------|
| Config merge | `crates/roko-core/src/config/loader.rs` tests | Passing |
| Snapshot fingerprint | `crates/roko-graph/src/snapshot.rs` tests | Passing |
| fsmonitor debounce | `crates/roko-cli/src/tui/fs_watch.rs` tests | Passing |
| Enrichment transition | Graph engine phase tests | Passing |

---

## 3. What Remains: Fresh Dogfood Proof

The regression fixes are merged and tested. What remains is a **fresh
full-cycle rerun** under the following conditions:

1. Clean workspace (no dirty tree, no stale snapshots)
2. Real LLM providers (not mocked)
3. Multi-task plan with cross-task dependencies
4. Full gate pipeline (not focused mode)
5. Resume after simulated crash
6. Merge queue integration

This rerun is tracked as a separate sign-off in `.roko/GAPS.md`. It is
not blocked by any code changes -- it requires dedicated runtime and
provider access.

---

## 4. Evidence of Operational Self-Hosting

Beyond the dogfood run, the system's operational history provides cumulative
evidence:

### 4.1 Programme Execution

| Metric | Value | Evidence |
|---|---|---|
| Epics accepted | 48/48 | Plan execution through the 8-step workflow |
| Executable tasks completed | 124/124 | Graph engine plan execution |
| Plans completed | 30/30 | All executable plans |
| Self-heal tasks completed | 57/57 (SH01-SH06) | Automated fix-and-verify cycles |

### 4.2 Learning Evidence

| Metric | Evidence |
|---|---|
| Playbook rules accumulated | Rules created from failure patterns across 48 epics |
| Cascade router adapted | Per-model pass rate statistics from 124+ task executions |
| Gate thresholds adapted | EMA-based threshold adjustment per rung |
| Prompt experiments concluded | A/B tests with significance testing |

### 4.3 Infrastructure Evidence

| Capability | Evidence |
|---|---|
| Snapshot resume | Graph fingerprint validation prevents stale resume |
| Crash recovery | Activity replay from JSONL recordings |
| Budget enforcement | Microdollar tracking with atomic reservations |
| Merge queue | File-conflict detection and priority ordering |
| Worktree isolation | Per-plan git worktrees with idle reclamation |

---

## 5. Known Limitations in Dogfood

The dogfood evidence demonstrates that the system works, but also reveals
limitations:

### 5.1 Provider Dependency

Self-hosting requires live LLM provider access. The system cannot develop
itself without API keys and network connectivity to at least one provider.
Offline development requires pre-generated plans and cached provider responses
(not currently implemented).

### 5.2 Cost Sensitivity

A full plan execution with 10+ tasks costs $5-50 depending on model selection
and retry count. The cascade router optimizes for cost/quality tradeoff, but
the baseline cost of self-hosting is non-trivial. FAST mode reduces cost by
limiting gates and retries.

### 5.3 Determinism

LLM provider outputs are non-deterministic. The same plan executed twice may
produce different code, different gate results, and different replan paths.
The Activity replay system provides deterministic resume within a single run,
but cross-run reproducibility requires seed control that current providers
do not guarantee.

### 5.4 Cold Start

A fresh workspace with no learning state (no episodes, no playbooks, no
routing data) performs worse than a workspace with accumulated learning. The
first few plans have higher failure rates and higher costs. This is expected
(the Polya urn is sparse initially) but means the self-hosting workflow is
most effective in mature workspaces.

---

## 6. Verification Commands

```bash
# Run the workspace test suite (includes dogfood regression tests)
cargo test --workspace

# Check learning state
cargo run -p roko-cli -- learn all

# Inspect gate thresholds
cargo run -p roko-cli -- learn gates

# Show cascade router state
cargo run -p roko-cli -- learn router

# Verify snapshot integrity
cargo run -p roko-cli -- doctor

# Inspect plan execution state
cargo run -p roko-cli -- plan status plans/<dir>

# Diagnose a failed plan
cargo run -p roko-cli -- diagnose <plan-id>
```
