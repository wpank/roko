# D — Enrichment + Context Assembly (Docs 04, 08)

Parity analysis of `docs/03-composition/04-enrichment-pipeline-13-step.md` + `08-5-stage-assembly-pipeline.md` vs actual codebase.

Two separate systems live under these docs:
- **Enrichment Pipeline** (doc 04): a 13-step plan-artifact pre-computation pipeline in `crates/roko-compose/src/enrichment/`. Fully built as a library but has **zero production callers** (only test mocks).
- **Context Assembly** (doc 08): the doc describes a 5-stage `ContextAssembler` pipeline. The code path in production is actually `ContextProvider` (`crates/roko-compose/src/context_provider.rs`, 1,679 LOC), which is tier-aware rather than 5-stage. `ContextAssembler` exists in `roko-neuro` with a gather/rank/compress pipeline (not 5 stages) and is NOT called anywhere in the orchestration path.

---

## D.01 — EnrichmentPipeline<C: LlmClient> (Doc 04 §3)

- **Status**: SCAFFOLD
- **Priority**: P1
- **Estimated LOC**: 150 (wire a real `LlmClient` impl + call from `prd plan` / `plan run`)
- **Dependencies**: None
- **Files to modify**: `crates/roko-cli/src/prd.rs`, `crates/roko-cli/src/orchestrate.rs`

### What the doc says
`EnrichmentPipeline<C: LlmClient>` with `run_step(step) -> Result<PathBuf>` and `run_all() -> Vec<StepResult>`. The pipeline is generic over `LlmClient`. `run_all` continues on failure, logging warnings. Claimed canonical source: `crates/roko-compose/src/enrichment/pipeline.rs` (~1,187 lines).

### What exists
`EnrichmentPipeline<C: LlmClient>` at `crates/roko-compose/src/enrichment/pipeline.rs:29-201`. `pipeline.rs` is 773 lines (not 1,187 — the 1,187 figure in doc 04 refers to the whole `enrichment/` directory, which actually totals ~3,195 LOC with batch/direct/config/etc.).

Methods implemented:
- `new(config, client)` at line 38
- `config()` at line 43
- `run_step(step, plan_base)` at line 51 — returns `StepOutcome` not `Result<PathBuf>` (richer outcome type with Generated/Skipped/Failed)
- `run_all(plan_base)` at line 117 — continues past failures via `outcomes.push(...)`
- `validate_and_write()` at line 126
- `finalize_output()` at line 168

Supporting:
- `output_is_stale()` at line 209 — mtime comparison against input dependencies
- `read_step_inputs()` at line 230 — reads per-step input files

**However**: grep across the workspace confirms `EnrichmentPipeline::new` is only called inside its own `#[cfg(test)]` module. There are zero production callers. The CLI command `roko prd plan <slug>` and `roko research enhance-plan` do their own agent-based enrichment in `orchestrate.rs:5715-5784` (strategist enrichment prompt) — they do NOT invoke `EnrichmentPipeline`. Also, **no non-test `LlmClient` implementations** exist anywhere in the workspace (confirmed via `grep 'impl LlmClient for'` — only `MockLlmClient` in `pipeline.rs:361`).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.01.1 | Zero production callers — `EnrichmentPipeline::new` is only called in `pipeline.rs` tests | crates/roko-cli/ | HIGH |
| D.01.2 | No production `LlmClient` implementations — only `MockLlmClient` in tests | workspace-wide | HIGH |
| D.01.3 | `run_step` returns `StepOutcome` (richer), not `Result<PathBuf>` per doc — cosmetic doc drift | pipeline.rs:51 | LOW |
| D.01.4 | Doc claims 1,187 LOC in pipeline.rs; actual pipeline.rs is 773 LOC (1,187 = whole enrichment/ dir total) | doc 04 line 3 | LOW |

### Verify
```bash
grep -rn 'EnrichmentPipeline::new\|impl LlmClient for' crates/ --include='*.rs' | grep -v '#\[cfg(test)\]\|/tests/'
# Expected: only pipeline.rs tests
```

---

## D.02 — EnrichStep Enum (13 variants) (Doc 04 §1)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
13-variant enum in `crates/roko-compose/src/enrichment/step.rs`:
`Prd, Briefs, Tasks, Decompose, Research, Dependencies, Fixtures, Integration, Verify, Reviews, Tests, Invariants, Scribe`. `ALL_ORDERED` constant holds them in dependency order.

### What exists
All 13 variants at `crates/roko-compose/src/enrichment/step.rs:30-57`. Confirmed by test `all_ordered_has_13_entries` at line 227. `ALL_ORDERED` at line 62-76 matches doc order exactly.

Per-variant metadata implemented:
- `output_filename()` at line 83 — 13-arm match
- `needs_llm()` at line 106 — returns false for 7 pure-extraction steps (Prd, Briefs, Tasks, Research, Dependencies, Fixtures, Integration) and true for 6 LLM steps (Decompose, Verify, Reviews, Tests, Invariants, Scribe). **NOTE: this contradicts doc 04 Table in §1 which states all 13 steps require an LLM call.**
- `default_model()` at line 131 — per-backend model selector for Claude/Codex/Cursor/Ollama (not just Claude)
- `is_toml()` at line 188 — 6 TOML steps: Tasks, Verify, Reviews, Scribe, Dependencies, Fixtures. Comment at line 183-186 explicitly notes this diverges from spec (which says 7).

Output filename table (code line 83 vs doc §1 Table):

| Step | Doc filename | Code filename | Match |
|------|--------------|---------------|-------|
| Prd | `prd-extract.md` | `prd-extract.md` | MATCH |
| Briefs | `brief.md` | `brief.md` | MATCH |
| Tasks | `tasks.toml` | `tasks.toml` | MATCH |
| Decompose | `decomposition.md` | `decomposition.md` | MATCH |
| Research | `research.md` | `research.md` | MATCH |
| Dependencies | `dependency-manifest.toml` | `dependency-manifest.toml` | MATCH |
| Fixtures | `fixture-manifest.toml` | `fixture-manifest.toml` | MATCH |
| Integration | `integration.md` | `integration.md` | MATCH |
| Verify | `verify.sh` | `verify-tasks.toml` | DIFFERS |
| Reviews | `review-tasks.toml` | `review-tasks.toml` | MATCH |
| Tests | `test-tasks.toml` | `testing-backlog.md` | DIFFERS |
| Invariants | `invariants.md` | `rubric.md` | DIFFERS |
| Scribe | `scribe-tasks.toml` | `scribe-tasks.toml` | MATCH |

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.02.1 | Doc §1 says "All 13 steps require an LLM call"; code says 7 are pure extraction (`needs_llm = false`) — major doc drift | step.rs:106 vs doc 04 §1 | MEDIUM (doc outdated) |
| D.02.2 | 3 output filenames differ (Verify: `verify.sh` -> `verify-tasks.toml`; Tests: `test-tasks.toml` -> `testing-backlog.md`; Invariants: `invariants.md` -> `rubric.md`) | step.rs:83 | LOW |
| D.02.3 | Doc says 7 TOML steps; code implements 6 (comment at step.rs:183-186 explicitly notes this divergence) | step.rs:188 | LOW |

### Verify
```bash
grep -c 'EnrichStep::' crates/roko-compose/src/enrichment/step.rs | head -1
# Expected: many occurrences; test at line 227 asserts ALL_ORDERED.len() == 13
```

---

## D.03 — LlmClient Trait (Doc 04 §2)

- **Status**: SCAFFOLD
- **Priority**: P1
- **Estimated LOC**: 100 (implement against one backend, e.g. reuse `ClaudeAgent`)
- **Dependencies**: D.01
- **Files to modify**: `crates/roko-cli/src/` (new `LlmClient` impl wrapping an existing agent)

### What the doc says
```rust
pub trait LlmClient: Send + Sync {
    fn complete(&self, model: &str, system_prompt: &str, user_message: &str) -> Result<String>;
}
```
Located in `crates/roko-compose/src/enrichment/mod.rs`. Four backend implementations defined: Claude, Codex, Cursor, Ollama.

### What exists
Trait at `crates/roko-compose/src/enrichment/client.rs:12-31`:

```rust
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    async fn call(
        &self,
        model: &str,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}
```

Differences vs doc:
- Method name is `call`, not `complete` — cosmetic rename
- `async fn` (not sync) with `#[async_trait]` attribute
- Extra `max_tokens: u32` parameter
- Error type is `Box<dyn std::error::Error + Send + Sync>`, not `anyhow::Result`

`LlmBackend` enum at `step.rs:14-23`: 4 variants (`Claude`, `Codex`, `Cursor`, `Ollama`) — matches doc. **But** `LlmBackend` is only used inside `EnrichmentConfig` and `EnrichStep::default_model()`; there are no backend-specific `LlmClient` impls in the workspace — the enum is a model-selector tag, not a strategy dispatcher.

**No production `LlmClient` implementations**: `grep -rn 'impl LlmClient for' crates/` returns only `MockLlmClient` in `pipeline.rs:361` (test-only).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.03.1 | Zero production `LlmClient` implementations — only `MockLlmClient` in tests | workspace-wide | HIGH |
| D.03.2 | `LlmBackend` enum exists but never dispatches to concrete backends (informational only) | step.rs:14 | MEDIUM |
| D.03.3 | Method signature differs from doc: `call(model, system, user, max_tokens)` vs doc `complete(model, system, user)` | client.rs:24 vs doc §2 | LOW |

### Verify
```bash
grep -rn 'impl LlmClient for\|impl.*client::LlmClient' crates/ --include='*.rs'
# Expected: MockLlmClient (test) only
```

---

## D.04 — DirectClient + BatchClient Implementations (Doc 04 §2)

- **Status**: SCAFFOLD
- **Priority**: P2
- **Estimated LOC**: 200 (concrete HTTP transports)
- **Dependencies**: D.03
- **Files to modify**: new `crates/roko-cli/src/enrichment_client.rs` or similar

### What the doc says
Pipeline uses two client modes:
- **Batch client**: for steps producing many artifacts (batches requests for cost efficiency — "50% cost" per Anthropic Batch API)
- **Direct client**: for steps producing single artifacts (individual requests)

### What exists
Both abstractions are defined but no HTTP transport is wired:

**DirectClient** at `crates/roko-compose/src/enrichment/direct_client.rs`:
- `DirectRequest` struct at line 42 (model, system, messages, max_tokens, temperature)
- `DirectResponse` struct at line 120 (content, usage, model, stop_reason)
- `DirectUsage` struct at line 103
- `Message` struct at line 15
- `StreamChunk` struct at line 133
- `DirectTransport` trait at line 147 — abstracts HTTP transport
- `DirectClient<T: DirectTransport>` at line 177 with `complete()`, `stream()`, `simple_complete()` methods
- `to_api_body()` at line 81 — serializes to Anthropic Messages API JSON

**BatchClient** at `crates/roko-compose/src/enrichment/batch_client.rs` (447 lines):
- `BatchId`, `BatchRequest`, `BatchResponse`, `BatchStatus`, `BatchUsage` types (lines 19-150)
- `BatchTransport` trait — abstracts HTTP transport for batch submission/polling

Neither has a production transport implementation. The `DirectTransport` and `BatchTransport` traits are only implemented by test mocks (`MockDirectTransport` at `direct_client.rs:237`, similar for batch).

**Neither is wired to the `LlmClient` trait**: `DirectClient` exposes `simple_complete(model, system, user_message, max_tokens) -> Result<String>` which matches `LlmClient::call` by shape, but does not implement the trait. An adapter is missing.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.04.1 | No production `DirectTransport` implementation | direct_client.rs:147 | HIGH |
| D.04.2 | No production `BatchTransport` implementation | batch_client.rs | HIGH |
| D.04.3 | `DirectClient`/`BatchClient` do NOT implement `LlmClient` — adapter missing to bridge the two | direct_client.rs:177 | MEDIUM |

### Verify
```bash
grep -rn 'impl DirectTransport for\|impl BatchTransport for' crates/ --include='*.rs'
# Expected: only mock impls in tests
```

---

## D.05 — EnrichmentConfig Struct (Doc 04 §3)

- **Status**: DONE (as a struct) / SCAFFOLD (unused in production)
- **Priority**: P2
- **Estimated LOC**: 0
- **Dependencies**: D.01
- **Files to modify**: None (struct itself complete)

### What the doc says
Configuration carried separately from the pipeline. Fields implied by `max_staleness`, `force`, `dry_run` flags (doc §3.1, §3.2). Doc references `crates/roko-compose/src/enrichment/config.rs` as canonical source.

### What exists
`EnrichmentConfig` at `crates/roko-compose/src/enrichment/config.rs:18-46` — 9 fields:

| Field | Type | Purpose |
|-------|------|---------|
| `repo_root` | `PathBuf` | Project root (plans live at `{repo_root}/.roko/plans/`) |
| `backend` | `LlmBackend` | Claude/Codex/Cursor/Ollama |
| `gateway_url` | `Option<String>` | Route through bardo-gateway |
| `gateway_key` | `Option<String>` | Gateway API key |
| `batch_mode` | `bool` | Use batch API instead of real-time |
| `model_override` | `Option<String>` | Override default model for all steps |
| `force` | `bool` | Regenerate even if output is fresh |
| `dry_run` | `bool` | Print without executing |
| `quiet` | `bool` | Suppress stdout/stderr (TUI-friendly) |

No `Default` impl — callers must be explicit (line 15 comment). Helper: `plan_dir(plan_base) -> PathBuf` at line 52.

**Difference vs doc**: Doc references a `max_staleness` field (§3.1: "Default max_staleness is 24 hours"), but the code has no such field — staleness is instead determined by mtime comparison against input dependencies (see `output_is_stale()` at `pipeline.rs:209`), which is a different (and better) staleness policy.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.05.1 | Doc describes `max_staleness` field; code uses dependency-graph mtime comparison (superior) | doc 04 §3.1 vs pipeline.rs:209 | LOW (doc outdated) |
| D.05.2 | Struct is never constructed in production (no CLI caller) | crates/roko-cli/ | MEDIUM (follows from D.01) |

### Verify
```bash
grep -rn 'EnrichmentConfig\s*{' crates/ --include='*.rs' | grep -v test
# Expected: no non-test constructions
```

---

## D.06 — build_prompt + build_repair_prompt (enrichment/prompts.rs)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Each of 13 steps has its own prompt template. A repair prompt exists for TOML parse errors, sending `parse_error` and raw output back to the LLM with instructions to fix (doc §3.2).

### What exists
At `crates/roko-compose/src/enrichment/prompts.rs` (499 lines):

**`build_prompt(step, inputs) -> (String, String)`** at line 46:
- Dispatches per-step to templates in `crate::templates::prompts` module
- 13-arm match covering every `EnrichStep` variant
- Returns `(system_prompt, user_message)` tuple
- Pulls from `inputs: &StepInputs` (9 optional input files) per step

**`build_repair_prompt(step, raw_output, error_message) -> (String, String)`** at line 138:
- Fixed system prompt: "You repair invalid TOML... Return only valid TOML. Preserve the original intent... Do not add markdown fences or commentary."
- User message embeds the step, error, and raw output in a triple-backtick TOML block

**`generate_without_llm(step, inputs) -> Result<String, String>`** at line 165:
- Handles the 7 non-LLM steps (contradicting doc §1 which says all 13 need LLM)
- Dispatches to pure-extraction helpers: `extract_prd`, `extract_brief`, `extract_tasks`, `generate_research`, `generate_dependency_manifest`, `generate_fixture_manifest`, `generate_integration`

`StepInputs` struct at line 19-38: `plan_content: String` + 8 `Option<String>` fields for tasks, brief, decomposition, verify, review, research, dependency manifest, fixture manifest.

### Gaps
None. Implementation matches and exceeds spec.

### Verify
```bash
grep -n 'pub fn build_prompt\|pub fn build_repair_prompt\|pub fn generate_without_llm' crates/roko-compose/src/enrichment/prompts.rs
```

---

## D.07 — validate_step_output + repair_toml_output (enrichment/validate.rs)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
After LLM output, TOML steps get a validation + repair pass. "One retry only. If the repair also fails, the step is marked as failed and the pipeline continues" (§3.2).

### What exists
At `crates/roko-compose/src/enrichment/validate.rs` (158 lines):

- **`validate_step_output(step, content)`** at line 17: empty check + `toml::from_str::<toml::Value>(trimmed)` for TOML steps
- **`normalize_step_output(step, content)`** at line 35: strips markdown fences, trims to `[meta]` marker for TOML, drops trailing fences
- **`repair_toml_output(step, repaired_raw)`** at line 58: calls `normalize_step_output` then `validate_step_output` on the already-retried content (does not itself call the LLM — I/O at boundary)
- Private `strip_fences()` at line 65

One-retry policy enforced in `EnrichmentPipeline::validate_and_write` at `pipeline.rs:126-164`:
- Call #1: generate
- If validation fails AND step is TOML: build repair prompt via `build_repair_prompt`, call LLM again, call `repair_toml_output` on the result
- If repair also fails: `StepOutcome::Failed` with embedded error message

Tests at `validate.rs:81-157` cover: empty rejection, whitespace rejection, valid TOML acceptance, invalid TOML rejection, fence stripping, trim-to-meta, repair acceptance, repair-still-invalid rejection. Also tested at `pipeline.rs:489-545` (`toml_repair_succeeds_after_one_retry`, `toml_repair_hard_fail_both_invalid`).

### Gaps
None.

### Verify
```bash
grep -n 'pub fn validate_step_output\|pub fn repair_toml_output\|pub fn normalize_step_output' crates/roko-compose/src/enrichment/validate.rs
```

---

## D.08 — 5-Stage ContextAssembler Pipeline (Doc 08)

- **Status**: NOT DONE (as specified); DIFFERENT IMPLEMENTATION present
- **Priority**: P1
- **Estimated LOC**: 500 (align with doc spec, or update doc to match reality)
- **Dependencies**: None
- **Files to modify**: `docs/03-composition/08-5-stage-assembly-pipeline.md` (doc update) OR `crates/roko-neuro/src/context.rs`

### What the doc says
A 5-stage pipeline `ContextAssembler` at `crates/roko-compose/src/context_assembler.rs`:

1. **Query** — HDC fingerprint similarity + keyword search, fused with Reciprocal Rank Fusion (RRF, K=60)
2. **Score** — composite `source_priority + relevance*0.4 + track_record*0.3 + confidence*0.2 + recency*0.1` with affect modulation
3. **Deduplicate** — Hamming distance < 0.15 on HDC fingerprints removes near-duplicates
4. **Budget** — greedy fit to token budget, never truncate entries
5. **Format** — U-shaped placement with per-entry metadata (`[Type: Insight] [Age: 3d] [Weight: 0.82]`)

Doc 08 §10 status table claims: Stage 1 Implemented, Stage 2 static Implemented, Stage 3 partial (compression), Stage 4 Implemented, Stage 5 partial.

### What exists

**`crates/roko-compose/src/context_assembler.rs` is a 4-line re-export:**
```rust
//! Canonical context assembly lives in `roko-neuro`; `roko-compose` re-exports
//! the memory-facing assembly primitives for prompt construction.

pub use roko_neuro::{ContextAssembler, ContextChunk, PadState};
```

The actual `ContextAssembler` is at `crates/roko-neuro/src/context.rs:221-520`. Its pipeline is 3 phases (gather -> rank -> compress) driven by `gather()` at `context.rs:267-288`, not 5 stages.

Stage-by-stage parity:

| Doc Stage | Code Equivalent | Match |
|-----------|-----------------|-------|
| Stage 1 Query (HDC+keyword RRF) | `gather_knowledge`/`gather_episodes`/`gather_read_files`/`gather_recent_signals` (context.rs:522+). Hybrid keyword+HDC similarity is present via `semantic_similarity` at line 924 (HDC-feature-gated). **RRF is NOT implemented** — knowledge store uses multiplicative product score (`keyword_score * confidence * recency * emotional` at `knowledge_store.rs:307`). | PARTIAL (no RRF) |
| Stage 2 Score | `score_chunk` at `context.rs:1188-1212`. Uses `track_record * similarity / uncertainty` (active inference formula) as primary; falls back to composite when `track_record` absent (`similarity*0.3 + recency*0.2 + confidence*0.3 + source_priority*0.2 + relevance_prior*0.08 + dream_bonus + affect_bias`). **Source priority values differ from doc Table §3.3**: code has KnowledgeEntry=1.0, Episode=0.8, InlineFile=0.5, RecentSignal=0.3, rest=0.2 (vs doc: AntiPattern=1.0, Verification=0.9, TaskBrief=0.8, InlineFile=0.7, KnowledgeEntry=0.6, etc.). | DIFFERENT (EFE-style active inference instead of ad hoc) |
| Stage 3 Deduplicate | **No Hamming-based deduplication.** Zero matches for `hamming`/`dedup` in `roko-neuro/src/context.rs`. The `compress()` method at `context.rs:365-520` instead runs an **auction-style allocator**: per-chunk bids with token cost, diminishing-returns penalty for same source family (`SAME_SOURCE_DIMINISHING_RETURNS=0.82`), reserve prices by priority, contrarian slot reservation, marginal-value stopping rule. | NOT DONE (auction replaces dedup) |
| Stage 4 Budget | Budget enforcement is interleaved with Stage 3 inside `compress()`: `while used_tokens < budget` loop at `context.rs:440-499`. Greedy with MARGINAL_VALUE_STOP_RATIO=0.5 early-stop. | MATCH (interleaved) |
| Stage 5 Format (U-shape) | **Not in `ContextAssembler`**. U-shape lives in `ContextProvider::into_prompt_sections` at `context_provider.rs:202-223` and `apply_attention_curve_placements` at line 263-283. Uses edge/middle slots based on rank. No per-entry `[Type: X] [Age: Nd]` metadata annotation — sections keep their original content. | PARTIAL (U-shape yes, metadata no) |

**Critical doc drift**: Doc 08 lines 86-109 embeds a Rust code example showing `impl ContextAssembler { pub fn gather(...) -> Vec<ContextChunk> { ... chunks.extend(self.gather_knowledge(...)); ... } }`. This signature matches the actual code at `context.rs:267-288` very closely. However the assertion at doc line 369 that the canonical source is `crates/roko-compose/src/context_assembler.rs` is false — that file is a 4-line re-export.

**Production wiring**: `grep 'ContextAssembler' crates/roko-cli/` returns zero matches. The orchestrator uses `ContextProvider`, not `ContextAssembler`. `ContextAssembler::new` is called only from `roko-neuro` tests and nowhere else.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.08.1 | `context_assembler.rs` is a 4-line re-export; actual canonical source is `roko-neuro/src/context.rs` — doc 08 line 369 cites wrong path | docs/03-composition/08-5-stage-assembly-pipeline.md:369 | MEDIUM (doc error) |
| D.08.2 | No Reciprocal Rank Fusion in production — knowledge store uses multiplicative product instead | knowledge_store.rs:307 | MEDIUM |
| D.08.3 | No Hamming-based deduplication — `compress()` uses auction-style allocator instead | context.rs:365 | HIGH (design divergence) |
| D.08.4 | `ContextAssembler` has zero production callers — orchestrator uses `ContextProvider` instead | workspace-wide | HIGH |
| D.08.5 | Stage 2 source priorities differ significantly from doc §3.3 table (code optimizes different set) | context.rs:935 vs doc 08 §3.3 | MEDIUM |
| D.08.6 | No per-entry metadata annotations (`[Type: Insight] [Age: 3d]`) in Stage 5 output | context.rs / context_provider.rs | LOW |
| D.08.7 | Doc status table §10 is stale: claims Stage 1 Implemented via HDC+keyword+RRF, but RRF is not implemented | doc 08 §10 | MEDIUM |

### Verify
```bash
wc -l crates/roko-compose/src/context_assembler.rs
# Expected: 4 lines (re-export stub)

grep -rn 'ContextAssembler' crates/roko-cli/ --include='*.rs'
# Expected: no matches (orchestrator uses ContextProvider)

grep -rn 'hamming\|dedup' crates/roko-neuro/src/context.rs
# Expected: no matches (no HDC dedup in the pipeline)
```

---

## D.09 — ContextProvider (context_provider.rs, 1,679 LOC)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 13 §2.1 lists `ContextProvider` as **Wired** into orchestrate.rs. Doc 08 does NOT describe `ContextProvider` — it describes `ContextAssembler`. In practice `ContextProvider` is the production path that the doc's `ContextAssembler` is supposed to model. Doc drift: the actual tier-aware provider is a substantial redesign that has overtaken the 5-stage spec.

### What exists
`ContextProvider` at `crates/roko-compose/src/context_provider.rs:442-455` with 6 fields:
- `workdir: PathBuf` (line 444)
- `budgets: ContextBudgets` (line 446)
- `symbol_resolver: SymbolResolver` (line 448)
- `brief_generator: TaskBriefGenerator` (line 450)
- `context_average_tracker: ContextAverageTracker` (line 452) — rolling EMA from `.roko/learn/context-averages.json`
- `pheromone_signals: Vec<Engram>` (line 454)

Public API:
- `new(workdir)` at line 460
- `with_budgets(budgets)` at line 481
- `with_pheromone_signals(signals)` at line 488
- `resolve(frequency, task, model_slug, plan_artifacts, siblings, prior_outputs) -> ResolvedContext` at line 497 — the main entry point

`resolve()` flow (line 505-554):
1. Compute `ContextTier::from_task_and_model(task.tier, model_slug)` at line 506
2. Compute `budget = budgets.for_frequency(frequency)` at line 507
3. `add_surgical_context` always (line 521)
4. `add_focused_context` if Focused/Full (line 524)
5. `add_full_context` if Full (line 536)
6. `apply_average_based_demotions` — demote Normal->Low when rolling ref rate < 0.10 (line 541)
7. `enforce_budget` — drop lowest-priority sections until within budget (line 544)

Production wiring: called from `crates/roko-cli/src/orchestrate.rs:10132` inside the task dispatch path:
```rust
let context_provider = ContextProvider::new(self.workdir.clone())
    .with_budgets(self.config.prompt.context_budgets.to_context_budgets());
...
let resolved = context_provider.resolve(frequency, &task_input, &selected_model, &plan_artifacts, &siblings, &prior_outputs);
```

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.09.1 | Doc 08 describes `ContextAssembler` as the production path; reality is `ContextProvider`. Doc 08 needs rewrite to describe tier-aware assembly | doc 08 | HIGH (doc drift) |

### Verify
```bash
grep -n 'ContextProvider::new\|context_provider.resolve' crates/roko-cli/src/orchestrate.rs
# Expected: wiring at line 10132 + 10157
```

---

## D.10 — ContextTier Enum (Surgical / Focused / Full)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 08 §5.1 mentions "Per context tier: 4K / 12K / 24K". Doc 13 §2.1 lists `ContextTier (Surgical/Focused/Full)` as **Wired** via `from_task_and_model()`.

### What exists
`ContextTier` at `crates/roko-compose/src/context_provider.rs:35-42`:
```rust
pub enum ContextTier {
    Surgical,   // Haiku / Ollama / Gemma
    Focused,    // Sonnet
    Full,       // Opus
}
```

Methods:
- `from_task_and_model(task_tier, model_slug)` at line 50: local models always get Surgical; "mechanical" -> Surgical; "architectural" -> Full; else Focused
- `default_token_budget()` at line 64: Surgical=4_000, Focused=12_000, Full=24_000 — exactly matches doc's 4K/12K/24K

Also implements bidirectional conversion with `OperatingFrequency`:
- `From<OperatingFrequency>` at line 73: Gamma->Surgical, Theta->Focused, Delta->Full
- `From<ContextTier>` at line 83: Surgical->Gamma, Focused->Theta, Full->Delta

`is_local_model()` helper at line 95 detects ollama/llama/gemma/qwen/mistral/codellama/deepseek/phi/starcoder prefixes plus generic `model:tag` format.

### Gaps
None.

### Verify
```bash
grep -n 'pub enum ContextTier\|fn from_task_and_model\|fn default_token_budget' crates/roko-compose/src/context_provider.rs
```

---

## D.11 — ResolvedContext, ContextSection, PlanArtifacts, PriorTaskOutput, SiblingTask

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: D.09, D.10
- **Files to modify**: None

### What the doc says
Doc 13 §2.1 lists these as wired. Doc 08 references `ContextChunk` (`content`, `source`, `relevance`, `track_record`, `confidence`, `recency`) as the carrier. `ContextSource` is used for attribution (Knowledge/Episode/File/etc.).

### What exists

**`ContextSection`** at `context_provider.rs:168-173`:
```rust
pub struct ContextSection {
    pub section: PromptSection,
    pub source: ContextSource,
}
```
`estimated_tokens()` helper at line 177.

**`ResolvedContext`** at `context_provider.rs:185-193`:
```rust
pub struct ResolvedContext {
    pub sections: Vec<ContextSection>,
    pub tier: ContextTier,
    pub total_tokens_estimate: usize,
    pub budget_tokens: usize,
}
```
Methods:
- `into_prompt_sections()` at line 202 — sorts by cache layer then placement then priority, applies attention U-curve, attaches `AttentionBidder` per section
- `sources()` at line 227

**`PlanArtifacts`** at `context_provider.rs:347-352`:
```rust
pub struct PlanArtifacts {
    pub plan_dir: PathBuf,
    pub plan_id: String,
}
```
Accessors for each enrichment artifact: `plan_brief()` (brief.md), `research_memo()` (research.md), `invariants()` (rubric.md), `prd_extract()` (prd-extract.md), `decomposition()` (decomposition.md), `cross_plan_context()` (context.md), `plan_doc()` (plan.md). Lines 371-409.

**`PriorTaskOutput`** at `context_provider.rs:429-434`:
```rust
pub struct PriorTaskOutput {
    pub task_id: String,
    pub summary: String,
}
```

**`SiblingTask`** at `context_provider.rs:416-423`:
```rust
pub struct SiblingTask {
    pub id: String,
    pub title: String,
    pub status: String,
}
```

**`ContextSource`** (re-exported from roko-neuro, defined at `roko-neuro/src/context.rs:77-144`) — 14 variants: KnowledgeEntry, Episode, InlineFile, RecentSignal, SymbolSignature, AntiPattern, Verification, TaskBrief, PriorTaskOutput, PlanBrief, ResearchMemo, Invariants, CrossPlanContext, PrdExtract, Decomposition, SiblingTasks. Far richer than the doc's 9-entry Source Priority table (§3.3).

**`ContextChunk`** (re-exported from roko-neuro, at `roko-neuro/src/context.rs:190-206`): content, source, relevance, track_record, confidence, recency, emotional_tag. Has one field beyond the doc: `emotional_tag: Option<EmotionalTag>` for mood-congruent retrieval.

### Gaps
None for these five types.

### Verify
```bash
grep -n 'pub struct ResolvedContext\|pub struct ContextSection\|pub struct PlanArtifacts\|pub struct PriorTaskOutput\|pub struct SiblingTask' crates/roko-compose/src/context_provider.rs
```

---

## D.12 — HDC-Based Deduplication (Doc 08 §4.1 / Doc 13 §3)

- **Status**: NOT DONE
- **Priority**: P2
- **Estimated LOC**: 200
- **Dependencies**: D.08
- **Files to modify**: `crates/roko-neuro/src/context.rs` (add dedup phase before compress)

### What the doc says
Doc 08 §4.1 describes the intended Hamming-distance dedup:
```
For each candidate (in score order, highest first):
    If Hamming_distance(candidate.fingerprint, any_selected.fingerprint) < 0.15:
        Skip candidate (near-duplicate)
    Else:
        Select candidate
```
Doc 13 §3 classifies "HDC-based deduplication" as **Scaffold** with the note: "HDC exists in bardo-primitives, compress() exists; D16 in 12a plan: wire HDC into dedup."

### What exists

**HDC infrastructure is present in three crates**:
- `crates/roko-primitives/src/hdc.rs:16` — XOR bind, majority-vote bundle, Hamming similarity
- `crates/roko-primitives/src/hdc.rs:210` — `Hamming similarity in the range [0, 1]`
- `crates/roko-index/src/hdc.rs:83-115` — `hamming_distance` function + cosine-similarity approximation
- `crates/roko-neuro/src/knowledge_store.rs:576-721` — `MemoryIndex` (HDC-feature-gated) + `hdc_similarity` score bonus in queries

**HDC usage in context assembly**:
- `crates/roko-neuro/src/context.rs:923-928` — `semantic_similarity(left, right)` calls `text_fingerprint(left).similarity(&text_fingerprint(right))` under `#[cfg(feature = "hdc")]`
- `crates/roko-neuro/src/knowledge_store.rs:309-310` — `score + hdc_similarity(&entry, topic)` added to knowledge query score
- Everywhere else: `hdc_vector: None` is the default (`context.rs:1734, 1830, 1881, 1903, 1925, 2127, 2151, 2222, 2246`)

**HDC-based dedup in the context pipeline is absent**. Zero matches for `hamming` or `dedup` related to chunk pruning in `roko-neuro/src/context.rs` or `roko-compose/`. The actual near-duplicate prevention is the source-family diminishing-returns factor (`SAME_SOURCE_DIMINISHING_RETURNS=0.82`) inside the auction allocator at `context.rs:365-520`, which is coarse-grained (source family, not content fingerprint).

**Doc 13 classifies this as SCAFFOLD**; verification confirms SCAFFOLD: HDC primitives exist, the `compress()` function exists, but the two have not been joined. The `compress()` method uses an unrelated auction-allocator design that does not consult HDC fingerprints.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.12.1 | HDC dedup primitives exist in roko-primitives/roko-index but are not wired into context assembly pruning | context.rs:365 | MEDIUM |
| D.12.2 | `ContextChunk` has no HDC fingerprint field — would need to carry one to enable Hamming comparison during selection | roko-neuro/src/context.rs:190 | MEDIUM |
| D.12.3 | Dedup is currently family-granularity (via SAME_SOURCE_DIMINISHING_RETURNS), not fingerprint-granularity | context.rs:233 | LOW |

### Verify
```bash
grep -rn 'hamming\|Hamming' crates/roko-compose/src/ crates/roko-neuro/src/context.rs
# Expected: no matches in context.rs for dedup

grep -rn 'text_fingerprint\|HdcVector' crates/roko-neuro/src/context.rs
# Expected: one match at context.rs:925 (semantic_similarity), no dedup usage
```

---

## Section Summary

| Item | Doc | Status | Parity |
|------|-----|--------|--------|
| D.01 | EnrichmentPipeline<C> | SCAFFOLD | 60% — code complete but zero production callers |
| D.02 | EnrichStep (13 variants) | DONE | 90% — all 13 variants + metadata; doc drift on needs_llm and filenames |
| D.03 | LlmClient trait | SCAFFOLD | 50% — trait defined, no production impls (only MockLlmClient) |
| D.04 | DirectClient + BatchClient | SCAFFOLD | 50% — types defined, no production transports, no LlmClient adapter |
| D.05 | EnrichmentConfig | DONE | 90% — struct complete; staleness uses better mtime approach than doc says |
| D.06 | build_prompt + build_repair_prompt | DONE | 100% — per-step dispatch + fixed repair template + generate_without_llm |
| D.07 | validate_step_output + repair_toml_output | DONE | 100% — empty + TOML validation + normalize + one-retry repair |
| D.08 | 5-Stage ContextAssembler pipeline | NOT DONE (as specified) | 30% — context_assembler.rs is a 4-line re-export; actual pipeline is 3-phase auction |
| D.09 | ContextProvider | DONE | 100% — 6 fields, tier-aware resolve(), wired into orchestrate.rs:10132 |
| D.10 | ContextTier enum | DONE | 100% — 3 variants, 4K/12K/24K budgets exactly match doc |
| D.11 | ResolvedContext / ContextSection / PlanArtifacts / PriorTaskOutput / SiblingTask | DONE | 100% — all 5 types + U-curve placement + artifact accessors |
| D.12 | HDC-based deduplication | NOT DONE | 20% — HDC primitives exist in other crates; never wired into context pruning |

### Priority actions

1. **P1 (D.01, D.03)**: Wire `EnrichmentPipeline` into the CLI. Build a production `LlmClient` impl (e.g. adapter over `ClaudeAgent` or `DirectClient`) and call `pipeline.run_all()` from `prd plan <slug>` instead of the ad-hoc strategist agent enrichment.
2. **P1 (D.08)**: Reconcile doc 08 with reality. Either rewrite doc 08 to describe `ContextProvider` tier-aware assembly, or add a genuine 5-stage `ContextAssembler` path. The current doc cites a 4-line re-export as canonical.
3. **P2 (D.04)**: Implement `DirectTransport` and `BatchTransport` backends so `DirectClient` / `BatchClient` become functional.
4. **P2 (D.12)**: Wire HDC fingerprint dedup into `ContextChunk` selection. Either add an HDC-based prune phase inside `compress()` or replace the `SAME_SOURCE_DIMINISHING_RETURNS` heuristic with real Hamming comparison.

### Big surprises

- **context_assembler.rs is a 4-line re-export** — confirmed as suspected. The canonical location is `roko-neuro/src/context.rs`.
- **EnrichmentPipeline has zero production callers.** The entire 13-step pipeline is library code only — every `EnrichmentPipeline::new` call is in `#[cfg(test)]`. The CLI `roko prd plan` and `roko research enhance-plan` use agent-based strategist enrichment in `orchestrate.rs` instead.
- **No production `LlmClient` implementations exist.** The trait is fully defined and the pipeline is generic over it, but only `MockLlmClient` (test-only) implements it. The app layer never wires a real backend.
- **Doc 08's 5-stage pipeline does not match reality.** The actual context pipeline is gather -> rank -> auction-compress (3 phases), not Query -> Score -> Dedup -> Budget -> Format. Stage 3 uses source-family diminishing returns, not HDC Hamming distance. Stage 1 uses multiplicative product score in the knowledge store, not RRF.
- **Doc 04 §1 states "All 13 steps require an LLM call"** but `EnrichStep::needs_llm()` returns false for 7 of 13 steps (Prd, Briefs, Tasks, Research, Dependencies, Fixtures, Integration use pure extraction via `generate_without_llm`).
- **Doc 04 output filenames diverge for 3 steps**: Verify uses `verify-tasks.toml` (not `verify.sh`), Tests uses `testing-backlog.md` (not `test-tasks.toml`), Invariants uses `rubric.md` (not `invariants.md`).
- **Doc 04 figure of "1,187 lines" refers to the whole `enrichment/` directory**, not `pipeline.rs` — pipeline.rs is 773 lines; the directory totals ~3,195 LOC.

---

## Agent Execution Notes

### D.01 / D.03 / D.04 / D.05 — Enrichment Runtime Activation

This is the main “dormant subsystem” batch in `03`.

Recommended slice:

1. add one production `LlmClient` path,
2. route one real CLI/runtime flow through `EnrichmentPipeline`,
3. keep the activation narrow and testable.

Acceptance criteria:

- one production path constructs `EnrichmentPipeline`,
- a non-test `LlmClient` implementation exists,
- the patch does not turn into a broad orchestration rewrite.

### D.08 / D.12 — Live Context Path

Operate on the live path, not the doc stub.

Recommended slice:

1. make HDC-aware dedup part of the actual `ContextAssembler` path,
2. add tests for near-duplicate suppression,
3. expose enough telemetry that the selection behavior is inspectable.

Acceptance criteria:

- dedup happens in the shipped context path,
- tests prove it,
- the patch does not pretend `context_assembler.rs` is where the real logic lives.
