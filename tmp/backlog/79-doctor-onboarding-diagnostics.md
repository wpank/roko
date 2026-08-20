# 79 — Doctor and Onboarding Diagnostic Accuracy

**Priority**: P1 — misleading warnings cause new users to distrust the diagnostic tools entirely
**Size**: M (1-2 days)
**Crates**: `crates/roko-cli` (paths: `src/doctor.rs`, `src/auth_detect.rs`, `src/bootstrap.rs`, `src/commands/config_cmd.rs`, `src/commands/setup.rs`)
**Depends on**: None

---

## Background

When a user sets up roko with only the Claude CLI provider (no Anthropic API key), running `roko doctor` and `roko config models list` produces confusing warnings. The diagnostic output tells them API keys are missing and models are unavailable — but the system is fully operational via the Claude CLI binary. A new user sees red warnings and concludes something is broken when it isn't.

This is a credibility problem: once users learn that doctor warnings are false positives, they stop reading them. That means they also ignore the real warnings.

There are six concrete issues to fix, each a self-contained change to a single function.

## Current State

1. **`roko config models list` false "missing" status** — `crates/roko-cli/src/commands/config_cmd.rs` lines 780-808: When listing builtin models, the code checks `std::env::var(b.api_key_env)` (line 794) to determine if a model is available. If `ANTHROPIC_API_KEY` is unset, all builtin Claude models show `key_status: "missing (ANTHROPIC_API_KEY)"`. The code does not check whether a `claude_cli` provider is configured, which would make those models reachable without an API key.

2. **`roko config providers list` false "base URL missing"** — `crates/roko-cli/src/commands/config_cmd.rs` lines 1667-1694: When checking API providers, the code reads `provider.base_url` from the TOML config (line 1668). If the user hasn't set an explicit `base_url`, the code falls through to line 1694: `issues.push("base URL missing")`. But `perplexity_api`, `cerebras_api`, and `anthropic_api` all have hardcoded default base URLs that the dispatch backend uses when `base_url` is absent. The provider list check doesn't know about these defaults.

3. **`roko doctor` API key warnings when CLI-only** — `crates/roko-cli/src/doctor.rs` lines 836-964: `check_configured_provider_keys()` correctly skips the check when no API providers are configured (line 890-900 returns an `Ok` check). However, `check_available_providers()` at line 972 separately checks `ANTHROPIC_API_KEY` in a static list (line 989) and, if missing, emits a `Warn` check at line 1035-1046. This fires even when `claude-cli` is already in the `available` list from the earlier probe at line 978. The two checks are inconsistent.

4. **`roko setup` doesn't write provider entries for detected keys** — `crates/roko-cli/src/commands/setup.rs` (146 lines total): The setup wizard detects `ANTHROPIC_API_KEY` and other env vars but never offers to write a `[providers.anthropic]` TOML block. The user must edit `roko.toml` manually after running setup.

5. **`target_staleness` warning threshold is too low** — `crates/roko-cli/src/doctor.rs` line 1870: `const WARN_TARGET_MB: u64 = 10_240` sets the threshold at 10 GB. In a Rust workspace with 35 crates, `cargo build` routinely produces 10-15 GB of artifacts. The warning fires on healthy workspaces, not just stale ones.

6. **`plans_dir_conflict` fix suggestion can silently overwrite files** — `crates/roko-cli/src/doctor.rs` lines 2127-2173: `check_plans_dir_conflict()` suggests `mv .roko/plans/* plans/ && rmdir .roko/plans` (line 2161) without checking whether any plan directories in both locations have the same name. If `plans/demo-hello/` and `.roko/plans/demo-hello/` both exist, the `mv` will silently overwrite or fail.

## Implementation Plan

### Step 1 — Fix `roko config models list` to detect CLI provider availability

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs`

In the builtin-models loop (lines 791-808), before setting `key_status`, check whether any configured provider of kind `ClaudeCli` exists when the model's provider kind is `ClaudeCli`. The `config` variable (the resolved `RokoConfig`) is already in scope.

```rust
// Around line 793, replace the existing key_ok computation:
let key_ok = {
    // Check API key first
    if !b.api_key_env.is_empty()
        && std::env::var(b.api_key_env)
            .ok()
            .map_or(false, |v| !v.trim().is_empty())
    {
        true
    } else if matches!(b.provider_kind, ProviderKind::ClaudeCli) {
        // ClaudeCli models don't need an API key — check if the CLI binary exists
        std::process::Command::new("claude")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        false
    }
};
// And update key_status message accordingly:
key_status: if key_ok {
    "ok".to_string()
} else if b.api_key_env.is_empty() {
    "ok (no key required)".to_string()
} else {
    format!("missing ({})", b.api_key_env)
},
```

### Step 2 — Fix `roko config providers list` to use per-kind default base URLs

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs`

In `check_provider_availability()` (around line 1667), add a helper that returns the hardcoded default base URL for each provider kind:

```rust
fn default_base_url_for_kind(kind: ProviderKind) -> Option<&'static str> {
    match kind {
        ProviderKind::AnthropicApi => Some("https://api.anthropic.com/v1"),
        ProviderKind::PerplexityApi => Some("https://api.perplexity.ai"),
        ProviderKind::CerebrasApi => Some("https://api.cerebras.ai/v1"),
        ProviderKind::GeminiApi => Some("https://generativelanguage.googleapis.com/v1beta"),
        _ => None,
    }
}
```

Then in the base URL check (around lines 1688-1694), fall back to this default before marking "base URL missing":

```rust
let base_url = provider.base_url.as_deref()
    .filter(|u| !u.is_empty())
    .or_else(|| default_base_url_for_kind(provider.kind));

match base_url {
    Some(base_url) => {
        // existing probe logic
    }
    None => issues.push("base URL missing".to_string()),
}
```

### Step 3 — Fix `roko doctor` to not warn about missing API keys when CLI is detected

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs`

In `check_available_providers()` (starting at line 972), the function first checks for `claude-cli` in the `available` list (line 978-985), then later emits a `Warn` if `available.is_empty()` (line 1035). The logic is already mostly correct: if `claude-cli` is found, `available` won't be empty, and the `Warn` won't fire. However, the `check_configured_provider_keys()` function separately warns about missing `ANTHROPIC_API_KEY` when there are no API providers configured.

Check the fallback path in `check_configured_provider_keys()` at lines 844-870 (the "no config file" branch). When `ANTHROPIC_API_KEY` is absent but the `claude` CLI is on PATH, change the `Warn` to `Ok`:

```rust
// Around line 846, in the "No config file" branch:
let has_key = std::env::var("ANTHROPIC_API_KEY")
    .ok()
    .filter(|k| !k.is_empty())
    .is_some();
let has_cli = std::process::Command::new("claude")
    .arg("--version")
    .output()
    .map(|o| o.status.success())
    .unwrap_or(false);

return vec![DoctorCheck {
    id: "provider_api_keys".to_string(),
    status: if has_key || has_cli {
        DoctorStatus::Ok
    } else {
        DoctorStatus::Warn
    },
    message: if has_key {
        "ANTHROPIC_API_KEY is set (no roko.toml)".to_string()
    } else if has_cli {
        "claude CLI detected — no API key required".to_string()
    } else {
        "no API keys found and no roko.toml present".to_string()
    },
    // ...
}];
```

Also add a comment near the `CursorAcp` exclusion in the `api_providers` filter (around line 879-888) explaining why `CursorAcp` doesn't need an API key check:

```rust
// CursorAcp authenticates via the Cursor IDE's own session,
// not via an roko-managed API key.
```

### Step 4 — Raise the target_staleness threshold

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs`

Change line 1870:

```rust
// Before:
const WARN_TARGET_MB: u64 = 10_240; // 10 GB

// After:
const WARN_TARGET_MB: u64 = 51_200; // 50 GB — a 35-crate Rust workspace commonly reaches 10-15 GB
```

### Step 5 — Add conflict detection to plans_dir_conflict fix suggestion

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs`

In `check_plans_dir_conflict()` (lines 2127-2173), collect the directory names from both locations and check for overlap before suggesting the `mv` command:

```rust
// After counting top_count and dot_count, collect names:
let top_names: HashSet<String> = std::fs::read_dir(&top_level)
    .map(|entries| {
        entries
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect()
    })
    .unwrap_or_default();

let dot_names: HashSet<String> = std::fs::read_dir(&dot_roko)
    .map(|entries| {
        entries
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect()
    })
    .unwrap_or_default();

let conflicts: Vec<&String> = top_names.intersection(&dot_names).collect();

let fix = if conflicts.is_empty() {
    Some("mv .roko/plans/* plans/ && rmdir .roko/plans".to_string())
} else {
    Some(format!(
        "manual merge required — conflicting plan directories: {}",
        conflicts.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
    ))
};
```

### Step 6 — Enhance roko setup to offer provider entries when API keys are detected

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/setup.rs`

The setup wizard is 146 lines. After the existing key-detection logic, add a section that asks if the user wants to write provider entries to `roko.toml`:

```rust
// Check common API key env vars and offer to write provider entries:
let api_keys = [
    ("ANTHROPIC_API_KEY", "anthropic", "anthropic_api"),
    ("OPENAI_API_KEY", "openai", "openai_compat"),
    ("PERPLEXITY_API_KEY", "perplexity", "perplexity_api"),
    ("CEREBRAS_API_KEY", "cerebras", "cerebras_api"),
];

for (env_var, name, kind) in api_keys {
    if std::env::var(env_var).ok().filter(|k| !k.is_empty()).is_some() {
        // Prompt user to add a [providers.{name}] entry
        // Write to roko.toml if they confirm
    }
}
```

## Acceptance Criteria

1. `roko config models list` with only `claude_cli` configured (no `ANTHROPIC_API_KEY`) shows builtin Claude models as `ok`, not `missing (ANTHROPIC_API_KEY)`.
2. `roko config providers list` with a `perplexity_api` provider and no explicit `base_url` does not show "base URL missing".
3. `roko doctor` with only `claude_cli` and no API keys set produces zero `Warn` checks about missing API keys.
4. The `CursorAcp` exclusion from API key checks has a comment explaining why.
5. `roko doctor` in a workspace with a 12 GB `target/` directory does not produce a `target_staleness` warning.
6. `roko doctor` in a workspace where `plans/demo/` and `.roko/plans/demo/` both exist shows a "manual merge required" fix suggestion, not the `mv` command.
7. `roko setup` prompts the user to add a provider entry when `ANTHROPIC_API_KEY` is detected in the environment.

## Verification Checklist

- [ ] Unset `ANTHROPIC_API_KEY`, configure a `[providers.claude]` with `kind = "claude_cli"`, run `roko config models list` — Claude models show "ok"
- [ ] Configure `[providers.perplexity]` with `kind = "perplexity_api"` and no `base_url`, run `roko config providers list` — no "base URL missing"
- [ ] With no API keys and a working `claude` binary, run `roko doctor` — zero API key warnings
- [ ] Create both `plans/test-plan/` and `.roko/plans/test-plan/` directories, run `roko doctor` — fix suggestion mentions conflict, does not suggest `mv`
- [ ] Set `ANTHROPIC_API_KEY=test-key`, run `roko setup` — setup offers to write `[providers.anthropic]` to `roko.toml`
- [ ] Run `cargo test -p roko-cli` — all existing doctor tests pass

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs` | Fix builtin model `key_status` to check for `ClaudeCli` binary; add `default_base_url_for_kind()` helper; use it in provider base-URL check |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs` | Fix `check_configured_provider_keys()` no-config branch to treat CLI-detected as ok; add comment explaining `CursorAcp` exclusion; raise `WARN_TARGET_MB` to 51200; add conflict detection to `check_plans_dir_conflict()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/setup.rs` | After key detection, offer to write `[providers.*]` entries for detected keys |
