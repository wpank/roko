# W8-C: Remove resolve_enrichment_backend() Substring Heuristic

**Priority**: P2 — removes anti-pattern
**Effort**: 1 hour
**Files to modify**: 1 file
**Dependencies**: None

## Problem

`resolve_enrichment_backend()` in `orchestrate.rs` (lines 1846-1874) uses substring matching to guess provider backend:
- `"gemini"` → Codex (wrong — Gemini is its own provider)
- `starts_with("sonnet")` → could match Cursor or Claude

## Current Code (lines 1846-1874)

```rust
fn resolve_enrichment_backend(command: &str, model: &str, provider: &str) -> EnrichmentLlmBackend {
    let command = command.to_ascii_lowercase();
    let model = model.to_ascii_lowercase();
    let provider = provider.to_ascii_lowercase();

    if command.contains("cursor") || provider.contains("cursor") || model.contains("composer") {
        EnrichmentLlmBackend::Cursor
    } else if command.contains("ollama") || provider.contains("ollama") || model.contains("gemma") || model.contains("llama") || model.contains("qwen") {
        EnrichmentLlmBackend::Ollama
    } else if command.contains("codex") || command.contains("openai") || provider.contains("openai") || provider.contains("zai") || provider.contains("gemini") || model.contains("gpt") || model.contains("o3") || model.contains("o4") || model.contains("gemini") {
        EnrichmentLlmBackend::Codex
    } else {
        EnrichmentLlmBackend::Claude
    }
}
```

## Fix

Replace with config-driven provider kind lookup. The provider's `kind` field already tells us what backend to use.

```rust
fn resolve_enrichment_backend(provider_kind: &str) -> EnrichmentLlmBackend {
    match provider_kind {
        "cursor_acp" => EnrichmentLlmBackend::Cursor,
        "ollama" => EnrichmentLlmBackend::Ollama,
        "claude_cli" | "anthropic_api" => EnrichmentLlmBackend::Claude,
        // All OpenAI-compat providers (openai, zai, gemini, cerebras, etc.)
        _ => EnrichmentLlmBackend::Codex,
    }
}
```

### Update callers

Find where `resolve_enrichment_backend()` is called:
```bash
grep -rn 'resolve_enrichment_backend' crates/roko-cli/src/orchestrate.rs
```

Change each callsite to pass the provider kind from config instead of command/model/provider strings. The provider kind should be available from the `ModelProfile` or `ProviderConfig` at the callsite.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W8-C-enrichment-backend-removal.md and implement all changes. Replace the substring-matching resolve_enrichment_backend() in crates/roko-cli/src/orchestrate.rs (lines 1846-1874) with a provider kind match. Update all callers to pass provider kind from config. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 7+8 batches together. Do not commit individually.

## Checklist

- [x] Replace substring matching with provider kind match
- [x] Update all callers to pass provider kind
- [x] Verify: correct backend selected for each provider kind
- [ ] Pre-commit checks pass
