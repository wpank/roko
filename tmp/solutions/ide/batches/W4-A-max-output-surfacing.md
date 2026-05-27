# W4-A: Surface Effective max_output in Config Options

## Context

The original solution doc (11-max-output-default.md) claimed the default max_output is 900
tokens. Investigation revealed this is **wrong**:

- `ModelProfile::max_output` is `Option<u64>` defaulting to `None` (provider.rs:370-371)
- When `None`, agent dispatch falls back to `DEFAULT_MAX_OUTPUT_TOKENS = 16_384` (defaults.rs:32)
- The 16K default is reasonable and NOT the truncation problem described

**The real issue is transparency**: the config surface doesn't reveal the effective value.
If a user sets `max_output = 500` thinking it's a per-turn cap, they get truncated responses
with no indication that max_output was the cause. And the session config options don't show
what value is actually in effect.

This batch:
1. Adds `effective_max_output()` helper to `ModelProfile`
2. Shows effective max_output in model option descriptions sent to IDE
3. Warns in config diagnostics when max_output is suspiciously low

## Prerequisites

None. This batch modifies independent code paths.

## File Locations

Three files:
1. `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/provider.rs` — ModelProfile (add method)
2. `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` — build_config_options (show in description)
3. `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/loader.rs` — collect_diagnostics (add warning)

## Change 1: Add effective_max_output() method to ModelProfile

**File:** `crates/roko-core/src/config/provider.rs`

The `ModelProfile` struct ends at line 461 (closing brace). There is **no existing `impl ModelProfile`
block** — you need to add one.

The struct definition closes at line 461:
```rust
    pub use_max_completion_tokens: bool,
}
```

Followed by GeminiConfig at line 463.

FIND (lines 460-463):
```rust
    pub use_max_completion_tokens: bool,
}

// ---- Gemini config -------------------------------------------------------
```

REPLACE WITH:
```rust
    pub use_max_completion_tokens: bool,
}

impl ModelProfile {
    /// Returns the effective max output tokens, falling back to the system default.
    ///
    /// When `self.max_output` is `None`, returns `DEFAULT_MAX_OUTPUT_TOKENS` (16,384).
    /// This mirrors the runtime fallback in agent dispatch.
    pub fn effective_max_output(&self) -> u64 {
        self.max_output
            .unwrap_or(crate::defaults::DEFAULT_MAX_OUTPUT_TOKENS as u64)
    }
}

// ---- Gemini config -------------------------------------------------------
```

**Key facts:**
- `DEFAULT_MAX_OUTPUT_TOKENS` is `u32 = 16_384` (defaults.rs:32), cast to `u64` to match `max_output: Option<u64>`
- The path `crate::defaults::DEFAULT_MAX_OUTPUT_TOKENS` works because provider.rs is in `roko-core`

## Change 2: Show effective max_output in model option descriptions

**File:** `crates/roko-acp/src/session.rs`

In `build_config_options` (lines 960-971), model options are built with just the slug as description:

```rust
    // ── Model options filtered by selected provider ──
    let mut model_options: Vec<ConfigOptionValue> = roko_config
        .models
        .iter()
        .filter(|(_, profile)| profile.provider == state.provider)
        .map(|(key, profile)| ConfigOptionValue {
            value: key.clone(),
            name: capitalize_model_key(key),
            description: Some(profile.slug.clone()),
        })
        .collect();
    model_options.sort_by(|a, b| a.value.cmp(&b.value));
```

FIND (lines 960-971):
```rust
    // ── Model options filtered by selected provider ──
    let mut model_options: Vec<ConfigOptionValue> = roko_config
        .models
        .iter()
        .filter(|(_, profile)| profile.provider == state.provider)
        .map(|(key, profile)| ConfigOptionValue {
            value: key.clone(),
            name: capitalize_model_key(key),
            description: Some(profile.slug.clone()),
        })
        .collect();
    model_options.sort_by(|a, b| a.value.cmp(&b.value));
```

REPLACE WITH:
```rust
    // ── Model options filtered by selected provider ──
    let mut model_options: Vec<ConfigOptionValue> = roko_config
        .models
        .iter()
        .filter(|(_, profile)| profile.provider == state.provider)
        .map(|(key, profile)| {
            let max_out = profile.effective_max_output();
            ConfigOptionValue {
                value: key.clone(),
                name: capitalize_model_key(key),
                description: Some(format!("{} (max output: {})", profile.slug, max_out)),
            }
        })
        .collect();
    model_options.sort_by(|a, b| a.value.cmp(&b.value));
```

**Note:** If W4-B is also applied (adding `ready` field to ConfigOptionValue), make sure
the `ready` field is also set here. The two changes are to different parts of the struct
literal and compose cleanly.

## Change 3: Add config diagnostic for suspiciously low max_output

**File:** `crates/roko-core/src/config/loader.rs`

The `collect_diagnostics` function (lines 213-262) ends with a duplicate slug check.
Add the max_output check after the duplicate slug check, before the closing `diagnostics`.

The ConfigDiagnostic struct (defined in provenance.rs:102) has two fields:
```rust
pub struct ConfigDiagnostic {
    pub key: String,
    pub message: String,
}
```

FIND (lines 259-262):
```rust
    }

    diagnostics
}
```

REPLACE WITH:
```rust
    }

    // Warn on suspiciously low max_output.
    for (name, profile) in &config.models {
        if let Some(max_out) = profile.max_output {
            if max_out > 0 && max_out < 1000 {
                diagnostics.push(ConfigDiagnostic {
                    key: format!("models.{name}.max_output"),
                    message: format!(
                        "model '{}' has max_output={} which is very low; responses may truncate mid-sentence",
                        name, max_out
                    ),
                });
            }
        }
    }

    diagnostics
}
```

## What NOT to Change

- Do NOT change `DEFAULT_MAX_OUTPUT_TOKENS` value (16,384 is correct)
- Do NOT change `ModelProfile::max_output` from `Option<u64>` — the `None` semantic ("use provider's maximum") is correct
- Do NOT change any agent dispatch code — the runtime fallback is already correct
- Do NOT modify `build_config_options` beyond the model option descriptions

## Verification

After Phase 2:
```bash
# Check that model options show max output info
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{}}' \
  | cargo run -p roko-cli -- acp --quiet --no-serve --config ~/.nunchi/roko/roko.toml 2>/dev/null \
  | head -1 | python3 -c "
import sys,json; d=json.load(sys.stdin)
for o in d['result'].get('configOptions') or []:
  if o['id']=='model':
    for opt in o.get('options', []):
      print(f\"{opt['value']}: {opt.get('description', 'no desc')}\")
"
# AFTER: shows "sonnet: claude-sonnet-4-20250514 (max output: 16384)"

# Check diagnostic with low max_output config
# (create a test config with max_output=500 for a model, load it, check diagnostics)
```

## Estimated Effort

15-20 minutes. Three small changes across 3 files.
