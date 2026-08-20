# 95 — Config Loader Silent Failures and Unsafe Fallbacks

**Priority**: P1 — Reliability: four silent failure modes cause downstream errors (failed API calls, wrong directories, missing keys) that appear far from their root cause
**Size**: S (1 day)
**Crates**: `crates/roko-core/` (`src/config/loader.rs`, `src/config/schema.rs`)
**Depends on**: None

---

## Background

Roko's config loading pipeline has four places where legitimate errors are swallowed and execution continues on incorrect defaults. The user sees a confusing symptom — an "unauthorized" HTTP 401, a missing provider, a wrong file path — instead of a clear message at startup explaining what is broken.

The four issues are:

1. **`$HOME` fallback to `.`**: If the `HOME` environment variable is not set, `global_config_path()` uses `"."` (the current working directory) as the home directory, producing paths like `./.roko/config.toml`. This can load the wrong config or silently search the wrong directory without any warning.

2. **Global config parse errors are silent**: If `~/.roko/config.toml` exists but has a syntax error, `merge_global_into()` logs a WARN and returns. The caller has no indication that a config file existed but was skipped. The process continues with defaults that may lack API keys or provider entries.

3. **Undefined `${VAR}` interpolation produces empty strings**: Config values like `api_key = "${ANTHROPIC_API_KEY}"` are interpolated at load time. If the env var is not set, `interpolate_vars` silently replaces `${ANTHROPIC_API_KEY}` with `""`. The provider then makes API calls with an empty key, receiving a generic 401 rather than "ANTHROPIC_API_KEY is not set."

4. **Unreadable `*_file` secrets are silently dropped**: `resolve_file_secrets()` processes `extra_headers` entries ending in `_file` by reading the file at the given path and inserting the contents under the stripped key. If the file cannot be read, the error is silently discarded and the header is omitted. The provider makes requests without the required auth header.

## Current State

All line numbers are from the most recent reads of these files.

### Issue 1: HOME fallback

`crates/roko-core/src/config/loader.rs`, line 1390 (in `global_config_path()`):
```rust
pub fn global_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let canonical = PathBuf::from(&home).join(".roko").join("config.toml");
    // ...
}
```

When `HOME` is unset, `canonical` becomes `./.roko/config.toml`.

### Issue 2: Global config parse errors

`crates/roko-core/src/config/loader.rs`, lines 1449–1458 (in `merge_global_into()`):
```rust
let global = match deserialize_migrated_toml(&text) {
    Ok(g) => g,
    Err(e) => {
        tracing::warn!(
            path = %global_path.display(),
            error = %e,
            "failed to parse global config"
        );
        return;   // <-- silent return, caller never knows
    }
};
```

The caller (`load_config_file`, called transitively by `load_config_unified`) has no way to detect that global config merging was skipped due to a parse error.

### Issue 3: Missing env var produces empty string

`crates/roko-core/src/config/schema.rs`, lines 1802–1811:
```rust
fn interpolate_vars(value: &str, env_fn: &dyn Fn(&str) -> Option<String>) -> String {
    if !value.contains("${") {
        return value.to_string();
    }
    let re = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("valid regex");
    re.replace_all(value, |caps: &regex::Captures| {
        env_fn(&caps[1]).unwrap_or_default()   // <-- empty string for missing var
    })
    .into_owned()
}
```

### Issue 4: Unreadable file secrets silently dropped

`crates/roko-core/src/config/schema.rs`, lines 996–1013 (`resolve_file_secrets()`):
```rust
if key.ends_with("_file") {
    let base_key = key.trim_end_matches("_file").to_string();
    if let Ok(content) = std::fs::read_to_string(value.trim()) {
        resolved.insert(base_key, content.trim().to_string());
    }
    // <-- read failure: key silently absent from resolved headers
}
```

The existing test at `crates/roko-core/src/config/schema.rs`, line 2911 (`resolve_file_secrets_reads_from_file`), only tests the success path.

## Implementation Plan

### Fix 1: Warn when HOME is absent, skip global config instead of falling back to `.`

In `crates/roko-core/src/config/loader.rs`, replace lines 1389–1391 with:

```rust
pub fn global_config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))  // Windows fallback
        .ok()?;
    if home.is_empty() {
        return None;
    }
    let home = PathBuf::from(home);
    let canonical = home.join(".roko").join("config.toml");

    if canonical.exists() {
        return Some(canonical);
    }

    // Legacy: $XDG_CONFIG_HOME/roko/config.toml or ~/.config/roko/config.toml
    let legacy = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            PathBuf::from(xdg).join("roko").join("config.toml")
        } else {
            home.join(".config").join("roko").join("config.toml")
        }
    } else {
        home.join(".config").join("roko").join("config.toml")
    };

    if legacy.exists() {
        return Some(legacy);
    }

    Some(canonical)  // neither exists; return canonical for new installs
}
```

**Important**: changing the return type to `Option<PathBuf>` requires updating all callers. Search for all call sites:
```
grep -rn "global_config_path()" crates/ --include="*.rs"
```

Callers that currently do `if !global_path.exists() { return; }` can be updated to:
```rust
let Some(global_path) = global_config_path() else {
    tracing::debug!("HOME not set; skipping global config merge");
    return;
};
```

Callers in `doctor.rs` that display the path for diagnostic purposes can use `.unwrap_or_else(|| PathBuf::from("<HOME not set>"))`.

### Fix 2: Surface global config parse errors to callers

Change `merge_global_into()` to return a `Result<(), String>` (or add a dedicated error type) so callers can log the error more prominently:

```rust
pub fn merge_global_into(config: &mut RokoConfig) -> Result<(), String> {
    let global_path = match global_config_path() {
        Some(p) => p,
        None => return Ok(()),
    };
    if !global_path.exists() {
        return Ok(());
    }

    let text = match std::fs::read_to_string(&global_path) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(path = %global_path.display(), error = %e, "failed to read global config");
            return Ok(());  // Missing file is not an error
        }
    };

    let global = match deserialize_migrated_toml(&text) {
        Ok(g) => g,
        Err(e) => {
            // Parse failure is a real error: the file exists and is malformed.
            tracing::error!(
                path = %global_path.display(),
                error = %e,
                "global config file exists but could not be parsed — using defaults without global config"
            );
            return Err(format!(
                "global config {}: {}",
                global_path.display(),
                e
            ));
        }
    };

    merge_global_config_into(config, global);
    Ok(())
}
```

Update callers of `merge_global_into` to handle the `Result`. The primary caller is inside `load_config_file`. Callers can either propagate the error or log it and continue — the caller has full context to decide. The doctor command (in `crates/roko-cli/src/doctor.rs`) should be updated to call `merge_global_into` during the config diagnostics check and report the error as a doctor finding.

### Fix 3: Warn on undefined env var in interpolation

In `crates/roko-core/src/config/schema.rs`, change `interpolate_vars` to emit a warning when a variable is not set:

```rust
fn interpolate_vars(value: &str, env_fn: &dyn Fn(&str) -> Option<String>) -> String {
    if !value.contains("${") {
        return value.to_string();
    }
    let re = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("valid regex");
    re.replace_all(value, |caps: &regex::Captures| {
        let var_name = &caps[1];
        match env_fn(var_name) {
            Some(val) => val,
            None => {
                tracing::warn!(
                    var = var_name,
                    "config references environment variable that is not set; using empty string"
                );
                String::new()
            }
        }
    })
    .into_owned()
}
```

The existing test `interpolate_vars_expands_env_references` (line 2642) tests that `${MISSING}` returns `""`. Update it to also confirm the warn is emitted (or simply update the comment to document the new behavior — capturing tracing output in tests requires additional test machinery).

### Fix 4: Warn on unreadable file secrets

In `crates/roko-core/src/config/schema.rs`, update `resolve_file_secrets()`:

```rust
if key.ends_with("_file") {
    let base_key = key.trim_end_matches("_file").to_string();
    match std::fs::read_to_string(value.trim()) {
        Ok(content) => {
            resolved.insert(base_key, content.trim().to_string());
        }
        Err(err) => {
            tracing::warn!(
                key = %key,
                path = %value.trim(),
                error = %err,
                "could not read secret file; header '{}' will be missing from provider config",
                base_key
            );
        }
    }
}
```

Add a failing-path test to confirm the warn is triggered:

```rust
#[test]
fn resolve_file_secrets_warns_on_missing_file() {
    let mut config = RokoConfig::default();
    let provider = config.providers.entry("test".to_string()).or_default();
    let mut headers = std::collections::HashMap::new();
    headers.insert(
        "authorization_file".to_string(),
        "/nonexistent/path/secret.txt".to_string(),
    );
    provider.extra_headers = Some(headers);

    // Should not panic; should silently skip the key with a warning.
    config.resolve_file_secrets();

    // The authorization header should be absent (not replaced with empty string).
    assert!(
        config.providers["test"]
            .extra_headers
            .as_ref()
            .map(|h| !h.contains_key("authorization"))
            .unwrap_or(true),
        "missing file secret should result in absent header, not empty string"
    );
}
```

## Acceptance Criteria

1. When `HOME` and `USERPROFILE` are both unset, `global_config_path()` returns `None` and no global config is loaded; no path starting with `./.roko` is constructed.
2. A malformed `~/.roko/config.toml` produces an `error`-level log and a `Result::Err` from `merge_global_into`, not a silent return.
3. A config value like `"${UNSET_VAR}"` produces a `warn`-level log identifying the variable name; the value becomes `""` as before (no behavior change, just observability).
4. An unreadable `*_file` secret path produces a `warn`-level log and the corresponding header is absent from the resolved config.
5. All existing config tests pass.
6. New test: `global_config_path()` with unset `HOME` returns `None` or a path that does not start with `"./"`.
7. New test: invalid TOML in global config causes `merge_global_into` to return `Err`.
8. New test: `resolve_file_secrets_warns_on_missing_file` passes.

## Verification Checklist

- [ ] `cargo test -p roko-core -- config` passes
- [ ] `HOME= cargo run -p roko-cli -- doctor` does not look for `./.roko/config.toml` (check log output)
- [ ] Write a malformed `~/.roko/config.toml` temporarily; run `roko status`; confirm error message is visible (not just a WARN in debug logs)
- [ ] `grep -rn "global_config_path()" crates/ --include="*.rs"` — confirm all callers handle the new `Option<PathBuf>` return type
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-core/src/config/loader.rs` | Change `global_config_path()` to return `Option<PathBuf>`, skip fallback to `"."`, update callers; change `merge_global_into` to return `Result<(), String>` with error-level log on parse failure |
| `crates/roko-core/src/config/schema.rs` | Add `warn!` in `interpolate_vars` for unset vars; add `warn!` in `resolve_file_secrets` for unreadable files; add failing-path test |
| Any caller of `global_config_path()` | Update to handle `Option<PathBuf>` |
| Any caller of `merge_global_into()` | Update to handle `Result<(), String>` |
