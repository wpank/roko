# 08: Model Discovery & CLI Ergonomics

## Problem

The user can't discover what models are available without reading `roko.toml`. Tab completion doesn't exist. Typos in model names produce unhelpful errors.

### Observed Friction

```bash
$ roko prd plan cursor-composer-backend --model opus4-6
error: cli override selected unknown model 'opus4-6'; add an explicit [models.*] entry

# User has 'opus' and 'claude-opus' configured but doesn't know that.
# No way to discover without grep'ing the TOML.
```

## Root Cause: Where Model Resolution Fails

The error originates in `crates/roko-cli/src/model_selection.rs:362`:

```rust
fn select_provider(source, model, resolved, providers) -> Result<...> {
    if let Some(profile) = resolved.profile.as_ref() {
        // profile exists → success path
    }
    // profile is None → always fails here
    Err(Error::UnknownModel {
        selection_source: source,
        model: model.to_string(),
        provider_kind: resolved.provider_kind.label().to_string(),
    })
}
```

The error message at `model_selection.rs:122`:
```
"cli override selected unknown model 'opus4-6' (inferred kind 'claude_cli');
 add an explicit [models.*] entry for this model"
```

This gives zero guidance. The user needs to know:
1. What models ARE configured
2. Which of those is closest to what they typed
3. That `opus4-6` probably means `opus` or `claude-opus`

## What Should Exist

### S1: `roko models` (or `roko config models list`)

```bash
$ roko models
NAME             PROVIDER    SLUG                    CTX     TIER     KEY STATUS
─────────────────────────────────────────────────────────────────────────────────
CONFIGURED IN roko.toml:
  opus           anthropic   claude-opus-4-6         200k    premium  ✓
  sonnet         anthropic   claude-sonnet-4-6       200k    std      ✓
  haiku          anthropic   claude-haiku-4-5        200k    fast     ✓
  gpt55          openai      gpt-5.5                 200k    std      ✓
  gpt54-mini     openai      gpt-5.4-mini            128k    fast     ✓
  o3             openai      o3                      200k    premium  ✓
  gemini-pro     gemini      gemini-2.5-pro          1M      premium  ✗ GEMINI_API_KEY not set

BUILTIN (usable without config entry):
  claude-opus    anthropic   claude-opus-4-6         ✓ ANTHROPIC_API_KEY
  claude-sonnet  anthropic   claude-sonnet-4-6       ✓ ANTHROPIC_API_KEY
  gpt-5.5        openai      gpt-5.5                 ✓ OPENAI_API_KEY
  o3             openai      o3                      ✓ OPENAI_API_KEY
  o4-mini        openai      o4-mini                 ✓ OPENAI_API_KEY
  gemini         gemini      gemini-2.5-pro          ✗ GEMINI_API_KEY not set

Default: gpt54-mini (change with: roko config set agent.default_model <name>)
```

**Implementation in `crates/roko-cli/src/commands/config_cmd.rs`**:

```rust
pub fn cmd_models_list(workdir: &Path) -> anyhow::Result<()> {
    let config = load_config_unified(workdir)?;

    println!("CONFIGURED IN roko.toml:");
    for (key, profile) in config.effective_models() {
        let provider = config.providers.get(&profile.provider);
        let available = provider.map(|p| config.is_provider_available(p)).unwrap_or(false);
        let status = if available { "✓" } else { "✗" };
        let ctx_k = profile.context_window / 1000;
        println!(
            "  {:<16} {:<12} {:<24} {:<8} {:<9} {}",
            key,
            profile.provider,
            truncate(&profile.slug, 24),
            format!("{}k", ctx_k),
            tier_label(profile.tier),
            status
        );
    }

    println!("\nBUILTIN (usable without config entry):");
    for builtin in BUILTIN_MODELS {
        let key_env_set = std::env::var(builtin.api_key_env).is_ok();
        let status = if key_env_set { "✓" } else { format!("✗ {} not set", builtin.api_key_env) };
        println!("  {:<16} {:<12} {:<24} {}", builtin.aliases[0], builtin.provider_label, builtin.slug, status);
    }

    let default = &config.agent.default_model;
    if !default.is_empty() {
        println!("\nDefault: {} (change with: roko config set agent.default_model <name>)", default);
    }

    Ok(())
}
```

### S2: Shell completions for `--model`

Roko already has `roko completions <shell>`. The completions should include model names dynamically. The generation logic is in `crates/roko-cli/src/commands/completions.rs`.

Add a `roko config models list --names-only` subcommand for shell completion scripts:

```bash
# In generated zsh completions:
'--model[Model to use]:model:->model_complete' \
# ...
(model_complete)
    local models
    models=($(roko config models list --names-only 2>/dev/null))
    _describe 'model' models
    ;;
```

After `source <(roko completions zsh)`:
```
$ roko prd plan foo --model op<TAB>
opus         claude-opus   o3-mini   o4-mini
```

### S3: Fuzzy matching for `--model` in error messages

The current error at `model_selection.rs:120-132` says nothing helpful. Replace it with a suggestion:

```rust
// In crates/roko-cli/src/model_selection.rs, update select_provider():
Err(Error::UnknownModel {
    selection_source: source,
    model: model.to_string(),
    provider_kind: resolved.provider_kind.label().to_string(),
    // NEW: pre-compute suggestions at the error site
    suggestions: suggest_models(model, &providers, &config.models),
})
```

Add a `suggestions` field to `Error::UnknownModel` and format it in the Display impl:

```rust
#[error(
    "{selection_source} selected unknown model '{model}' (inferred kind '{provider_kind}');\
     add an explicit [models.*] entry for this model{suggestion_text}"
)]
UnknownModel {
    selection_source: SelectionSource,
    model: String,
    provider_kind: String,
    suggestions: Vec<String>,
}

// In Display / thiserror, add:
fn suggestion_text(suggestions: &[String]) -> String {
    if suggestions.is_empty() {
        return String::new();
    }
    format!("\n\nDid you mean one of these?\n  {}", suggestions.join("\n  "))
}
```

The suggestion function using Levenshtein/Jaro-Winkler:

```rust
/// Find the closest model names to `input` using fuzzy matching.
/// Searches both configured model keys and builtin aliases.
fn suggest_models(
    input: &str,
    providers: &IndexMap<String, ProviderConfig>,
    configured_models: &IndexMap<String, ModelProfile>,
) -> Vec<String> {
    let mut candidates: Vec<(String, f64)> = Vec::new();

    // Score configured model keys
    for key in configured_models.keys() {
        let score = jaro_winkler(input, key);
        if score > 0.6 {
            if let Some(profile) = configured_models.get(key) {
                candidates.push((
                    format!("{key:<16} ({}, {})", profile.provider, profile.slug),
                    score,
                ));
            }
        }
    }

    // Score builtin model aliases
    for builtin in BUILTIN_MODELS {
        for alias in builtin.aliases {
            let score = jaro_winkler(input, alias);
            if score > 0.6 {
                candidates.push((
                    format!("{alias:<16} ({}, {})", builtin.provider_label, builtin.slug),
                    score,
                ));
                break; // Only show first alias per builtin
            }
        }
    }

    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(3);
    candidates.into_iter().map(|(name, _)| name).collect()
}

fn jaro_winkler(a: &str, b: &str) -> f64 {
    // Simplified: prefix match + edit distance
    if a == b { return 1.0; }
    if b.starts_with(a) || a.starts_with(b) { return 0.9; }
    // Simple character overlap metric (replace with actual strsim if available)
    let common: usize = a.chars().filter(|c| b.contains(*c)).count();
    let max_len = a.len().max(b.len()) as f64;
    if max_len == 0.0 { return 0.0; }
    (common as f64) / max_len
}
```

**Resulting error message:**

```
$ roko prd plan foo --model opus4-6
error: cli override selected unknown model 'opus4-6'; add an explicit [models.*] entry

Did you mean one of these?
  opus             (anthropic, claude-opus-4-6)
  claude-opus      (anthropic, claude-opus-4-6)
  o4-mini          (openai, o4-mini)
```

### S4: Short aliases work without config (via builtin registry)

Common aliases should Just Work without a `[models.*]` entry.

After implementing the builtin registry (see `01-model-config-ux.md` and `13-config-should-not-exist.md`), these all work without any TOML config:

| User types | Resolves to |
|-----------|-------------|
| `opus` | claude-opus-4-6 via anthropic |
| `sonnet` | claude-sonnet-4-6 via anthropic |
| `haiku` | claude-haiku-4-5 via anthropic |
| `claude-opus` | claude-opus-4-6 via anthropic |
| `claude-sonnet` | claude-sonnet-4-6 via anthropic |
| `gpt-5.5` or `gpt55` | gpt-5.5 via openai |
| `o3` | o3 via openai |
| `o4-mini` | o4-mini via openai |
| `gemini` | gemini-2.5-pro via gemini |
| `gemini-pro` | gemini-2.5-pro via gemini |
| `gemini-flash` | gemini-2.5-flash via gemini |

These are hardcoded in `BUILTIN_MODELS` in the binary, updated with releases.

**The key change in `crates/roko-core/src/agent.rs:304`** (after config miss):

```rust
// NEW Step 4: Try builtin registry before giving up
if let Some(builtin) = crate::config::model_registry::find_builtin(model_key) {
    let profile = ModelProfile {
        provider: builtin.provider_label.to_owned(),
        slug: builtin.slug.to_owned(),
        context_window: builtin.context_window,
        max_output: Some(builtin.max_output),
        supports_tools: builtin.supports_tools,
        supports_thinking: builtin.supports_thinking,
        supports_vision: builtin.supports_vision,
        use_max_completion_tokens: builtin.use_max_completion_tokens,
        tool_format: builtin.tool_format.to_owned(),
        tier: Some(builtin.tier),
        cost_input_per_m: builtin.cost_input_per_m,
        cost_output_per_m: builtin.cost_output_per_m,
        metadata_source: ModelMetadataSource::BuiltInFallback,
        ..ModelProfile::default()
    };

    // Synthesize provider if not in config
    let provider_config = config.providers.get(builtin.provider_label)
        .cloned()
        .unwrap_or_else(|| ProviderConfig {
            kind: builtin.provider_kind,
            base_url: builtin.base_url.map(str::to_string),
            api_key_env: Some(builtin.api_key_env.to_string()),
            timeout_ms: Some(crate::defaults::DEFAULT_REQUEST_TIMEOUT_MS),
            ttft_timeout_ms: Some(crate::defaults::DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(crate::defaults::DEFAULT_CONNECT_TIMEOUT_MS),
            ..ProviderConfig::default()
        });

    return ResolvedModel {
        model_key: model_key.to_owned(),
        slug: builtin.slug.to_owned(),
        provider_kind: builtin.provider_kind,
        provider_config: Some(provider_config),
        profile: Some(profile),
        backend: builtin.provider_kind.to_backend(),
    };
}
```

### S5: `roko models default <name>`

```bash
$ roko models default opus
✓ Set default model to 'opus' (claude-opus-4-6 via anthropic)
  Updated roko.toml: [agent].default_model = "opus"

$ roko models default gpt-5.5
✓ Set default model to 'gpt-5.5'
  Note: using built-in model profile (no [models.gpt55] entry needed)
  Updated roko.toml: [agent].default_model = "gpt-5.5"
```

Implementation: `roko config set agent.default_model <name>` already partially works; wrap it in a nicer command that also validates the model exists (in config or builtins) and prints confirmation.

## Unified Error Improvement Strategy

The `UnknownModel` error currently lives at:
- `crates/roko-cli/src/model_selection.rs:122` (error definition)
- `crates/roko-cli/src/model_selection.rs:362` (error site)

The full improvement plan:

1. **Immediate**: Add suggestions to error message using fuzzy matching (S3)
2. **Short-term**: Builtin registry eliminates most `UnknownModel` errors entirely (S4)
3. **Medium-term**: `roko models list` (S1) + shell completions (S2) prevent the error from being reached in the first place

## Priority

| Solution | Impact | Effort | Priority |
|----------|--------|--------|----------|
| S4 (builtin aliases) | Eliminates ~80% of UnknownModel errors | 3 days | P1 |
| S3 (fuzzy suggestions in error) | Reduces confusion for remaining errors | 1 day | P1 |
| S1 (roko models list) | Essential CLI hygiene, enables discovery | 1 day | P1 |
| S2 (tab completion) | Standard CLI hygiene, reduces typos | 0.5 days | P2 |
| S5 (roko models default) | Convenience | 0.5 days | P3 |

## The Deeper Problem: prd draft validation

The original PRD that triggered this doc was empty because `prd draft new` hit a rate limit and saved the error as content. The downstream failure (model unknown) was confusing because the real error was silently swallowed upstream.

Fix in `crates/roko-cli/src/commands/prd.rs` (prd draft new handler):

```rust
// After agent generates PRD content:
let prd_text = run_agent(prompt).await?;

// Validate before writing
let has_requirements = prd_text.contains("## Requirements") || prd_text.contains("# Requirements");
let has_acceptance = prd_text.contains("## Acceptance") || prd_text.contains("## Acceptance Criteria");
let looks_like_error = prd_text.contains("rate limit") || prd_text.contains("429") || prd_text.len() < 200;

if looks_like_error || (!has_requirements && !has_acceptance) {
    return Err(anyhow::anyhow!(
        "prd draft generation failed or produced invalid output:\n{}\n\
         Fix: check your API key, rate limits, or try again",
        &prd_text[..prd_text.len().min(500)]
    ));
}

// Only write if valid
write_prd_file(&prd_text)?;
```
