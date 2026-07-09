# Gap Summary — PU03 Composition Audit

Concise audit gap list for `03`.

## Focus Now

These are the gaps PU03 should actively audit or tighten.

### 1. Wired-Path Docs Drift — HIGH

- The runtime story is narrower than the old docs implied.
- The real path is `orchestrate.rs` -> `ContextProvider` -> `prompting.rs` -> `RoleSystemPromptSpec` / `SystemPromptBuilder` / `PromptComposer`.
- Audit fixes should prefer that path over reviving dormant helpers.

### 2. Budget Contract Is Split Across Two Layers — HIGH

- `templates/common::budget_for()` is the base table.
- `budget::adjusted_budget_for()` adds complexity scaling.
- PU03 should verify where the split is actually wired and where it is still only a helper seam.

### 3. Live Role Identity Coverage Still Has Honest Gaps — MEDIUM

- `Researcher` still uses an inline fallback string.
- `Conductor` still uses an inline fallback string.
- `Refactorer` still reuses `TaskImplTemplate` identity.

### 4. Cache-Marker And MCP Coverage Need Real Call-Site Checks — MEDIUM

- `with_cache_markers()` and cache comments exist in the builder.
- `MCP_TOOLS_STANZA` exists in templates.
- PU03 should verify the wired prompt path uses them where docs claim, not assume broad coverage.

### 5. Helper Libraries Can Be Overstated — MEDIUM

- `EnrichmentPipeline` is real, but it is not the same thing as the live per-dispatch context path.
- HDC similarity helpers exist, but broader dedup/distributed-context claims are not the default audit target.
- `ActiveInferenceScorer` is the clearest naming-vs-runtime-contract risk.

## Defer From PU03

Record these as handoffs unless a small wired-path fix directly depends on them:

- VCG / mechanism-design fairness work
- calibrated MVT patch modeling
- distributed context engineering or agent-mesh design
- eval-theory work such as RAGAS, CLEAR, CIV, or Meta-Harness
- full active-inference or EFE learning-policy redesign
- broad enrichment redesign beyond the existing runtime seam

## Single-Agent Rule

If the task cannot plausibly be completed, verified, and written up by one agent in about 90 minutes, narrow it to:

1. a wired-path clarification,
2. one small runtime fix,
3. or a deferred follow-on note.
