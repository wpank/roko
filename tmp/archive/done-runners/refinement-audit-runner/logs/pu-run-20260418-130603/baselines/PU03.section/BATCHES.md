# Batch Execution Contract

8 batches ordered for unattended execution. The goal is not just to "cover the docs", but to let an agent turn composition parity findings into bounded work that can run overnight without guessing.

---

## Batch Posture

- Default strategy: **make existing composition policy actually affect runtime before inventing new composition theory**.
- Treat `crates/roko-compose/src/role_prompts.rs`, `templates/`, `system_prompt_builder.rs`, and `crates/roko-cli/src/orchestrate.rs` as conflict hotspots.
- If a task starts requiring learning-policy redesign, evaluation harness design, or distributed-context architecture, record the seam and stop.
- Every completed batch should leave behind:
  - code changes,
  - verification command output,
  - explicit deferrals,
  - and any follow-on dependency it creates.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning section file(s) named below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Serial Order

For a single long-running agent run, prefer:

`P1 -> P4 -> P7 -> P2 -> P3 -> P6 -> P5 -> P8`

This order establishes a single budget source of truth, closes obvious role/prompt gaps, then activates complexity-aware/runtime-heavy behavior after the underlying composition surface is cleaner.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus | Est. LOC |
|-------|-------|---------|---------------------|--------------|----------|
| P1 | C.04, C.05, E.01 | Make `budget_for()` the static role-budget authority | `roko-compose` templates + role prompts | `cargo test -p roko-compose` | 180 |
| P2 | C.08, E.02 | Activate complexity-adaptive budgets on one production path | `roko-compose`, `roko-cli` prompt build path | `cargo test -p roko-compose -p roko-cli` | 220 |
| P3 | A.10, E.06.1 | Add min-useful-context guard + prompt budget observability | `roko-compose` composer/build metadata | `cargo test -p roko-compose` | 180 |
| P4 | C.06, C.07 | Complete role-template coverage and prompt glue hygiene | `roko-compose` templates + role prompts | `cargo test -p roko-compose` | 160 |
| P5 | D.01, D.03, D.04, D.05 | Activate one real enrichment-pipeline runtime path | `roko-compose` enrichment + `roko-cli` | `cargo test -p roko-compose -p roko-cli` | 300 |
| P6 | D.08, D.12 | Harden live context assembly with HDC-aware dedup | `roko-neuro`, `roko-compose`, `roko-cli` | `cargo test -p roko-neuro -p roko-compose -p roko-cli` | 240 |
| P7 | B.09, C.08.6 | Complete cache markers and align main prompt path with MCP stanza intent | `roko-compose` system-prompt / role-prompt path | `cargo test -p roko-compose` | 100 |
| P8 | F.02 | Remove misleading active-inference naming or make the contract honest | `roko-compose`, `roko-cli` | `cargo test -p roko-compose -p roko-cli` | 100 |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| P1 | — |
| P2 | P1 |
| P3 | P2 |
| P4 | — |
| P5 | P4 |
| P6 | — |
| P7 | — |
| P8 | P2 |

Why `P1 -> P2 -> P3`:

- the complexity path should build on a single static budget source of truth,
- and min-useful-context policy is clearer once those runtime budgets are real.

Why `P4 -> P5`:

- activating enrichment through the composition path is easier once the prompt/template surface for core roles is less incomplete.

Why `P2 -> P8`:

- scorer truth-in-advertising should happen after the runtime prompt-selection path is clearer.

Parallel-safe groups:

- `{P1, P4, P6, P7}` can start immediately.
- `P2` should wait for `P1`.
- `P3` should wait for `P2`.
- `P5` should wait for `P4`.
- `P8` should wait for `P2`.

Conflict groups:

| Group | Crates / Files | Batches |
|-------|----------------|---------|
| budgets | `crates/roko-compose/src/templates/*`, `budget.rs`, `role_prompts.rs`, `crates/roko-cli/src/orchestrate.rs` | P1, P2, P3 |
| roles-builder | `crates/roko-compose/src/role_prompts.rs`, `templates/*`, `system_prompt_builder.rs` | P4, P7 |
| enrichment | `crates/roko-compose/src/enrichment/*`, `crates/roko-cli/src/orchestrate.rs` | P5 |
| context | `crates/roko-neuro/src/context.rs`, `crates/roko-compose/src/context_provider.rs`, `crates/roko-cli/src/orchestrate.rs` | P6 |
| scorers | `crates/roko-compose/src/scorer.rs`, `role_prompts.rs`, `crates/roko-cli/src/orchestrate.rs` | P8 |

---

## Batch Details

### P1 — Static Role-Budget Unification

**Owns**: `C.04`, `C.05`, `E.01`

**Read first**:
- [C-role-templates.md](C-role-templates.md)
- [E-budget-management.md](E-budget-management.md)

**Problem**: `PromptBudget` and `budget_for()` are documented as the role-budget source of truth, but templates still hardcode most of the same numbers manually.

**Scope**:
1. Refactor templates to derive static section caps from `budget_for(role)` or an equivalent shared helper.
2. Remove duplicated hardcoded role-budget literals where practical.
3. Add tests proving key roles still get the intended static caps.
4. Leave role-specific deviations explicit if they are truly intentional.

**Out of scope**:
- complexity-adaptive budgets,
- `min_tokens` guard,
- compression-controller work.

**Files**:
- `crates/roko-compose/src/templates/common.rs`
- `crates/roko-compose/src/templates/*.rs`
- `crates/roko-compose/src/role_prompts.rs` if shared helpers belong there

**Verify**:
```bash
cargo test -p roko-compose
cargo clippy -p roko-compose --no-deps -- -D warnings
```

**Acceptance criteria**:
- static role budgets come from one obvious source,
- template cap drift is reduced or eliminated,
- tests make the intended per-role cap behavior easy to discover.

---

### P2 — Complexity-Adaptive Budget Activation

**Owns**: `C.08`, `E.02`

**Read first**:
- [C-role-templates.md](C-role-templates.md)
- [E-budget-management.md](E-budget-management.md)

**Problem**: `adjusted_budget_for()` and `Complexity` exist, but the live prompt path still uses a flat token budget.

**Scope**:
1. Thread task complexity into one production prompt-build path.
2. Apply `adjusted_budget_for()` or an equivalent derived policy on that path.
3. Add tests or dry-run evidence showing complexity changes prompt budget behavior.
4. Keep the policy bounded to composition-owned surfaces.

**Out of scope**:
- regression-based budget prediction,
- section A/B decision matrices,
- dynamic layer ordering.

**Files**:
- `crates/roko-compose/src/budget.rs`
- `crates/roko-compose/src/role_prompts.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/prompting.rs`

**Verify**:
```bash
cargo test -p roko-compose -p roko-cli
cargo run -p roko-cli -- plan run plans/ --dry-run
rg -n "adjusted_budget_for|TaskComplexityBand|Budget::tokens" crates/roko-compose crates/roko-cli
```

**Acceptance criteria**:
- complexity changes prompt-budget behavior on at least one live path,
- the path is discoverable from tests or dry-run output,
- flat-budget fallback behavior is still explicit rather than implicit.

---

### P3 — Min-Useful-Context Guard + Prompt Observability

**Owns**: `A.10.1`, `A.10.2`, `E.06.1`

**Read first**:
- [A-composer-core.md](A-composer-core.md)
- [E-budget-management.md](E-budget-management.md)

**Problem**: sections can be truncated to useless slivers, and the current prompt-build metadata is too weak to explain what was dropped or how tokens were distributed.

**Scope**:
1. Add a minimum-viable inclusion policy for sections or section families.
2. Extend prompt-build metadata so dropped sections and per-layer token usage are inspectable.
3. Add tests for drop-vs-truncate behavior.
4. Keep the policy deterministic and simple.

**Out of scope**:
- BudgetPredictor / BudgetOutcome framework,
- leave-one-out CIV computation,
- compression-controller design.

**Files**:
- `crates/roko-compose/src/prompt.rs`
- `crates/roko-compose/src/templates/*` if section metadata needs updating
- `crates/roko-cli/src/prompting.rs` if prompt-build consumers need updates

**Verify**:
```bash
cargo test -p roko-compose
cargo clippy -p roko-compose --no-deps -- -D warnings
```

**Acceptance criteria**:
- tiny unusable section stubs are dropped rather than silently kept,
- prompt-build metadata shows what was dropped,
- per-layer token accounting is inspectable in code/tests.

---

### P4 — Role-Template Coverage Hardening

**Owns**: `C.06`, `C.07`

**Read first**:
- [C-role-templates.md](C-role-templates.md)

**Problem**: key roles still fall back to inline strings or generic identities, and the tool-allowlist glue still hardcodes Claude-specific phrasing.

**Scope**:
1. Add concrete template coverage for Researcher and Conductor.
2. Give Refactorer a role-specific identity surface instead of reusing the implementer-per-task identity.
3. Remove Claude-specific wording from generic tool-allowlist instructions.
4. Add tests for the new role mappings.

**Out of scope**:
- full template coverage for every secondary role in `AgentRole`,
- domain/plugin scaffolding,
- research/eval prompt programs.

**Files**:
- `crates/roko-compose/src/role_prompts.rs`
- `crates/roko-compose/src/templates/*.rs`

**Verify**:
```bash
cargo test -p roko-compose
rg -n "Researcher|Conductor|Refactorer|tool_allowlist_instructions" crates/roko-compose
```

**Acceptance criteria**:
- Researcher and Conductor no longer use inline fallback strings,
- Refactorer prompt identity differs from Implementer where intended,
- generic prompt glue no longer mentions Claude unless the path is Claude-specific.

---

### P5 — Enrichment Pipeline Runtime Activation

**Owns**: `D.01`, `D.03`, `D.04`, `D.05`

**Read first**:
- [D-enrichment-context.md](D-enrichment-context.md)
- [context-pack/composition-summary.md](context-pack/composition-summary.md)

**Problem**: the documented enrichment pipeline is a library with test-only clients, while the live CLI uses ad-hoc strategist-agent enrichment.

**Scope**:
1. Build one production `LlmClient` path or adapter.
2. Route one real CLI/runtime enrichment flow through `EnrichmentPipeline`.
3. Keep the activation narrow: one path is enough.
4. Add tests or runtime evidence proving the pipeline now has a production caller.

**Out of scope**:
- replacing every enrichment path,
- distributed/batch enrichment orchestration,
- broad PRD workflow redesign.

**Files**:
- `crates/roko-compose/src/enrichment/*`
- `crates/roko-cli/src/orchestrate.rs`

**Verify**:
```bash
cargo test -p roko-compose -p roko-cli
rg -n "EnrichmentPipeline::new|impl .*LlmClient" crates/roko-compose crates/roko-cli
```

**Acceptance criteria**:
- at least one production path constructs and uses `EnrichmentPipeline`,
- a non-test `LlmClient` implementation exists,
- the patch does not widen into a full orchestration redesign.

---

### P6 — Live Context Assembly Hardening

**Owns**: `D.08`, `D.12`

**Read first**:
- [D-enrichment-context.md](D-enrichment-context.md)

**Problem**: the real context path is in `ContextProvider` + `roko-neuro::ContextAssembler`, but HDC dedup is still missing and the doc/runtime mismatch remains sharp.

**Scope**:
1. Add HDC-aware near-duplicate handling to live context selection or compression.
2. Keep the change inside the shipped context path, not the old stub.
3. Add tests proving near-duplicates are suppressed under defined conditions.
4. Expose enough debug/trace output that the behavior is inspectable.

**Out of scope**:
- rewriting the entire context assembly architecture,
- full distributed context engineering,
- evaluation-harness work.

**Files**:
- `crates/roko-neuro/src/context.rs`
- `crates/roko-compose/src/context_provider.rs` if the surfaced data needs adjustment
- `crates/roko-cli/src/orchestrate.rs` only if telemetry exposure is needed

**Verify**:
```bash
cargo test -p roko-neuro -p roko-compose -p roko-cli
rg -n "text_fingerprint|semantic_similarity|Hamming|dedup" crates/roko-neuro crates/roko-compose
```

**Acceptance criteria**:
- live context selection performs content-level dedup under defined conditions,
- tests cover the dedup behavior,
- the batch does not pretend the old five-stage doc path is what actually runs.

---

### P7 — Cache Marker + Main-Path Prompt Parity

**Owns**: `B.09`, `C.08.6`

**Read first**:
- [B-system-prompt-builder.md](B-system-prompt-builder.md)
- [C-role-templates.md](C-role-templates.md)

**Problem**: only half the documented cache markers are emitted, and an important shared stanza (`MCP_TOOLS_STANZA`) bypasses the main role-prompt path.

**Scope**:
1. Complete or intentionally rationalize cache marker coverage.
2. Make the main role-prompt path reflect the intended MCP/tool guidance story.
3. Add tests showing marker behavior and stanza inclusion rules.
4. Keep the patch bounded to prompt assembly, not provider/tool execution semantics.

**Out of scope**:
- compression-controller work,
- global cache-control redesign,
- tool execution policy redesign.

**Files**:
- `crates/roko-compose/src/system_prompt_builder.rs`
- `crates/roko-compose/src/role_prompts.rs`
- `crates/roko-compose/src/templates/assembly.rs`

**Verify**:
```bash
cargo test -p roko-compose
rg -n "<!-- cache|MCP_TOOLS_STANZA|with_cache_markers" crates/roko-compose
```

**Acceptance criteria**:
- cache marker behavior is complete or explicitly documented in code/tests,
- the main role-prompt path no longer silently diverges from the intended MCP-stanza contract,
- downstream behavior can be inferred from tests without reverse engineering.

---

### P8 — Scorer Truth-In-Advertising

**Owns**: `F.02`

**Read first**:
- [F-advanced-allocation.md](F-advanced-allocation.md)

**Problem**: `ActiveInferenceScorer` sounds like EFE-backed active inference, but the implementation is a goal-directed heuristic and is not used in the orchestrator scorer stack.

**Scope**:
1. Make the scorer contract honest.
2. Prefer a bounded rename / API clarification unless real EFE already clearly fits within the patch.
3. Update call sites and tests accordingly.
4. Leave a precise handoff for real active-inference work.

**Out of scope**:
- full EFE implementation,
- orchestrator learning-policy redesign,
- softmax / episode-history active inference systems.

**Files**:
- `crates/roko-compose/src/scorer.rs`
- `crates/roko-compose/src/role_prompts.rs`
- `crates/roko-cli/src/orchestrate.rs` only if runtime references change

**Verify**:
```bash
cargo test -p roko-compose -p roko-cli
rg -n "ActiveInferenceScorer|GoalDirected" crates/roko-compose crates/roko-cli
```

**Acceptance criteria**:
- the scorer name and docs no longer over-claim active inference,
- tests and call sites use the honest contract,
- real EFE work is explicitly handed off to the later learning batch.
