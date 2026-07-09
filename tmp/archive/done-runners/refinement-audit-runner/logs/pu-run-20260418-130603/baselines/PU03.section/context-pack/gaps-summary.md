# Gap Inventory — 03 Composition

Concise gap list for agents working on composition parity batches.

## Focus Now

These are the gaps batch `03` should actively try to close:

### 1. Budget Policy Is Mostly Dead Code — HIGH

- `budget_for()` exists,
- `adjusted_budget_for()` exists,
- runtime mostly ignores both.

### 2. Enrichment Library Is Not The Runtime Enrichment Path — HIGH

- `EnrichmentPipeline` is substantial,
- `LlmClient` exists,
- production callers do not.

### 3. Live Context Path Lacks HDC-Aware Dedup — MEDIUM

- HDC primitives exist,
- the live context path does not use them for pruning,
- source-family diminishing returns is doing the coarse substitute.

### 4. Prompt Glue Has Real Coverage Gaps — MEDIUM

- Researcher and Conductor still use fallback strings,
- Refactorer reuses implementer phrasing,
- cache markers and MCP stanza behavior are uneven.

### 5. Some Advanced Composition Names Over-Claim — MEDIUM

- `ActiveInferenceScorer` is the clearest example,
- docs and names can mislead later agents about what actually ships.

## Defer From Batch 03

These are valid findings, but they should usually be documented and handed off:

- real EFE / active-inference learning policy -> `05`
- Thompson layer ordering -> `05`
- compression-controller design -> later composition hardening
- mechanism-design fairness / truthful auctions -> `05` or research pass
- MVT patch modeling -> `05`
- RAGAS / CLEAR / CIV / Meta-Harness -> eval pass after parity
- distributed context engineering -> post-parity roadmap

## Working Rule

If a composition task requires:

- a new learning-policy model,
- an evaluation harness,
- or distributed-context architecture,

then batch `03` should normally implement the smallest composition-layer foundation and defer the rest.
