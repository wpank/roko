# S-prompt-3: Re-enable HDC similarity in context layer with kill switch

## Task
The HDC similarity step (retrieves similar past tasks for prompt context) is disabled. Re-enable it gated by a `prompt.hdc_similarity_enabled` config flag (default `false` until quality measurements justify enabling).

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/30-prompt-assembly-completion.md` § PA-3.

## Exact changes

### 1. Add config field

`crates/roko-core/src/config/schema.rs`: extend `[prompt]` (or add it):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptConfig {
    /// Re-enable HDC similarity retrieval in the context layer.
    /// Default: false. Quality measurement still pending.
    pub hdc_similarity_enabled: bool,
    pub hdc_similarity_top_k: usize,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            hdc_similarity_enabled: false,
            hdc_similarity_top_k: 3,
        }
    }
}
```

Add `pub prompt: PromptConfig` field to `RokoConfig`.

### 2. Re-enable in context layer

`crates/roko-prompt/src/context_layer.rs` (or wherever the HDC retrieval lives):

```rust
pub fn build_context_layer(req: &ContextLayerReq) -> Option<String> {
    if !req.config.prompt.hdc_similarity_enabled {
        return None;
    }
    let similar = req.codeintel.find_similar(&req.task_fingerprint, req.config.prompt.hdc_similarity_top_k)?;
    if similar.is_empty() { return None; }

    let mut s = String::from("## Relevant Prior Work\n");
    for entry in similar {
        s.push_str(&format!("- {}: {}\n", entry.title, entry.summary));
    }
    Some(s)
}
```

### 3. Plumb through `BuildPromptReq` (post T5-35b)

If T5-35b has landed, `BuildPromptReq` carries the necessary config; use `req.config.prompt`. If not, the context-layer caller in `dispatch_agent_with` reads from `cfg.prompt`.

### 4. Tests

```rust
#[test]
fn context_layer_off_when_kill_switch_disabled() {
    let req = mk_req_with_hdc_disabled();
    assert!(build_context_layer(&req).is_none());
}

#[test]
fn context_layer_renders_similar_when_enabled() {
    let req = mk_req_with_hdc_enabled_and_hits();
    let s = build_context_layer(&req).unwrap();
    assert!(s.contains("Relevant Prior Work"));
}
```

## Write Scope
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-prompt/src/context_layer.rs` (or `lib.rs`)
- `roko.toml` (add `[prompt]` block with `hdc_similarity_enabled = false` for documentation)

## Verify

```bash
rg 'hdc_similarity_enabled' crates/ roko.toml
# Expect: at least 4 hits (config def, default, kill-switch check, toml)
```

## Do NOT

- Do NOT bundle with other S-prompt batches.
- Do NOT enable by default. Quality measurement first.
- Do NOT change `find_similar` API in `roko-codeintel`.
- Do NOT plumb HDC similarity into ACP / serve dispatch in this batch.
