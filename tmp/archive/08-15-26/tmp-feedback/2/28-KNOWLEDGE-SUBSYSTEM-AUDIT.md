# Knowledge Subsystem Audit: What's Wired, What's Not

## Summary

The neuro knowledge store is more complete than CLAUDE.md suggests. Several items listed as
"not wired" actually work. Key remaining gap: HDC fingerprints are computed but not queried
at dispatch time for similar-task lookup.

## Status Matrix

| Component | CLAUDE.md says | Actual Status |
|-----------|---------------|---------------|
| Neuro knowledge store | Wired | **Wired** — confirmed |
| Cold substrate archival | "Not instantiated at runtime" | **Works** — `archive` command triggers it |
| Knowledge backup/restore | Wired | **Wired** — genomic bottleneck + decay restoration |
| Knowledge sync | Wired | **Wired** — mesh sync implemented |
| Dream consolidation | Partial | **Partial** — `maybe_auto_dream()` triggers at plan completion, but no cron/schedule |
| HDC fingerprint per-episode | Wired | **Computed but not queried** — stored in episodes, never used for similar-task lookup |
| Playbook store | Wired | **Wired** — queried at dispatch time, injected into system prompt |
| Knowledge-informed routing | Not wired | **Not wired** — CascadeRouter doesn't consult knowledge store for model selection |

## Corrections to CLAUDE.md

1. **Cold substrate archival** — listed as "built but not instantiated at runtime (no cron/trigger)"
   in the remaining work section. This is outdated. `roko knowledge archive` triggers archival.
   What's missing is *automatic* archival (cron/trigger), not the capability itself.

2. **HDC fingerprint** — listed as "Wired" in the status table, which is half true. The fingerprint
   is computed and stored per episode. But the intended use case (looking up similar past tasks
   to inform agent dispatch) is not implemented. The fingerprint is write-only data.

## Key Gap: HDC Similar-Task Lookup

### What exists:
```rust
// In orchestrate.rs — fingerprint computed
let fingerprint = hdc::compute_fingerprint(&task_context);
episode.hdc_fingerprint = Some(fingerprint);
```

### What's missing:
```rust
// Should exist in orchestrate.rs before dispatch:
let similar_episodes = neuro_store.query_by_hdc_similarity(&fingerprint, top_k=5);
let context = build_context_from_similar(similar_episodes);
// → inject into system prompt or use for model selection
```

### Implementation (~30 min):

**File:** `crates/roko-neuro/src/store.rs`

Add a similarity query method:
```rust
pub fn query_by_hdc_similarity(
    &self,
    fingerprint: &HdcVector,
    top_k: usize,
) -> Vec<(Episode, f64)> {
    self.episodes
        .iter()
        .filter_map(|ep| {
            ep.hdc_fingerprint.as_ref().map(|fp| {
                let similarity = hdc::cosine_similarity(fingerprint, fp);
                (ep.clone(), similarity)
            })
        })
        .sorted_by(|a, b| b.1.partial_cmp(&a.1).unwrap())
        .take(top_k)
        .collect()
}
```

**File:** `crates/roko-cli/src/orchestrate.rs`

Before dispatch, query similar tasks and add to context:
```rust
let similar = neuro_store.query_by_hdc_similarity(&fingerprint, 5);
if !similar.is_empty() {
    prompt_builder.add_similar_tasks_context(&similar);
}
```

## Files to Modify

| File | Change |
|------|--------|
| `CLAUDE.md` | Update cold substrate status |
| `crates/roko-neuro/src/store.rs` | Add HDC similarity query |
| `crates/roko-cli/src/orchestrate.rs` | Query similar tasks before dispatch |

## Priority

**P2** — The knowledge subsystem mostly works. The HDC similarity lookup is the main gap
and would enable agents to learn from past similar tasks.
