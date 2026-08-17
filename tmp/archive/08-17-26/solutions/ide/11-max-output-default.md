# Issue: max_output Default Too Low for IDE Use

## Problem Statement

The default `max_output` value for model profiles appears to be 900 tokens.
This is far too low for an IDE agent that generates code, creates tiles, or
gives detailed explanations.

## Observed Behavior

When a model profile doesn't specify `max_output`:
```toml
[models.sonnet]
provider = "openai"
slug = "gpt-4o"
# max_output not specified → defaults to 900
```

The agent's responses get truncated at ~900 tokens. For an IDE that expects
code generation, this produces incomplete responses without clear error indication.

With `max_output = 16000`:
```toml
[models.sonnet]
provider = "openai"
slug = "gpt-4o"
max_output = 16000  # Explicit override needed
```

Long responses (tested: "count 1-50") complete without truncation.

## Root Cause

The default is likely set in `crates/roko-core/src/config/provider.rs` or
`crates/roko-agent/src/openai_compat_agent.rs`. The 900 value makes sense for
CLI quick-answer mode but is inappropriate as a universal default.

## Impact

- Every IDE config must explicitly set `max_output` on every model
- If forgotten, agent responses silently truncate mid-sentence
- No error or warning when truncation occurs due to max_output
- The truncation is indistinguishable from the model choosing to stop

## Proposed Solution

### 1. Raise the default to 4096

A reasonable default that works for both CLI and IDE:

```rust
impl Default for ModelProfile {
    fn default() -> Self {
        Self {
            provider: String::new(),
            slug: String::new(),
            supports_tools: false,
            context_window: 128_000,
            max_output: 4096,  // Was 900
        }
    }
}
```

### 2. Use provider-aware defaults

Different providers have different output limits. Use the provider's known
maximum as the default:

```rust
fn default_max_output(provider_kind: &str, model_slug: &str) -> u32 {
    match (provider_kind, model_slug) {
        ("openai_compat", s) if s.starts_with("gpt-4o") => 16_384,
        ("openai_compat", s) if s.starts_with("o3") => 100_000,
        ("claude_cli", _) => 8_192,
        ("gemini", _) => 8_192,
        _ => 4_096,
    }
}
```

### 3. Document the field prominently

In config documentation and `roko config init`:

```toml
# max_output: Maximum tokens the model can generate per response.
# Default: 4096. Set higher (16000+) for code generation or IDE use.
# Set to 0 to use the model's maximum.
max_output = 16000
```

### 4. Warn on suspiciously low max_output

```rust
if profile.max_output > 0 && profile.max_output < 1000 {
    warn!(
        model = key,
        max_output = profile.max_output,
        "max_output is very low; responses may be truncated"
    );
}
```

## Implementation Location

| File | Change |
|------|--------|
| `crates/roko-core/src/config/provider.rs` | Change default max_output |
| `crates/roko-core/src/config/schema.rs` | Add validation/warning |
| Config documentation | Document the field and recommended values |

## Priority

Medium-high. This is a common source of confusion — responses just stop
mid-sentence with no indication that max_output was the cause.
