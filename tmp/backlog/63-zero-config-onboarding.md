# 63 — Zero-Config Onboarding Wizard

**Priority**: P3 — Largest adoption barrier: new users cannot get a working setup without editing config files manually
**Size**: M (2–3 days)
**Crates**: `crates/roko-cli` (primary: `commands/setup.rs`, `commands/init.rs`, `commands/util.rs`, `auth_detect.rs`, `doctor.rs`, `config_cmd.rs`), `crates/roko-core` (`config/presets.rs`, `config/schema.rs`)
**Depends on**: None

---

## Background

When a new user installs roko and runs `roko init`, they get a `.roko/` directory and a `roko.toml` file. That config hardcodes `claude` as the provider with no fallback. If the user does not have the Claude CLI installed (or `ANTHROPIC_API_KEY` set), every subsequent command fails with an opaque error about a missing provider. The user must then manually discover that a provider is needed, hand-edit `roko.toml` to add `[providers.*]` and `[models.*]` entries, set environment variables, and only then discover `roko doctor` validates the setup.

A five-step interactive wizard (`roko setup`, implemented in `crates/roko-cli/src/commands/setup.rs`) was added to address this. It detects available providers via `detect_auth_from_env()`, prompts for an API key when none is found, runs `roko init` if needed, runs `roko doctor`, and prints next steps. However, the wizard has a critical gap: it does not write provider or model config into `roko.toml`. Regardless of what `detect_auth_from_env()` returns, the generated config still hardcodes `claude_cli`. The API key prompt tells the user to `export` the key manually — it never persists it. No preset selection is offered.

Three presets exist in `crates/roko-core/src/config/presets.rs` (`Minimal`, `Balanced`, `Thorough`) that control model tier, gate enablement, parallelism, and budget caps. They are never exposed to the user during setup. A user wanting GPT-4o or Gemini must still hand-edit TOML after running `roko setup`. The result is that first-run success always requires manual file editing, which is the single largest barrier to adoption.

## Current State

1. **`roko setup` wizard** — `crates/roko-cli/src/commands/setup.rs` (147 lines total). The `cmd_setup` function runs five steps: detect auth (line 33), show default model (line 52), run `cmd_init` if `.roko/` is absent (line 56–62), run `run_doctor` (line 66), print next steps (line 88). It does NOT write provider config into `roko.toml`.

2. **API key prompt** — `prompt_for_api_key()` at line 102 in `commands/setup.rs`. It reads a key from stdin, detects whether it is Anthropic or OpenAI by prefix, then prints `export ANTHROPIC_API_KEY=...` to the terminal. It does NOT persist the key to any file.

3. **`cmd_init`** — `crates/roko-cli/src/commands/util.rs` line 80. Creates `.roko/` subdirectories, writes `roko.toml` via `render_init_template()` in `crates/roko-cli/src/commands/init.rs`.

4. **`render_init_template`** — `crates/roko-cli/src/commands/init.rs` line 15. Accepts a `cloud: bool` flag. Calls `detect_init_profile()` (line 16) and `command_on_path("claude")` (line 48) to conditionally include a `[providers.claude_cli]` block. Does NOT accept provider/preset selection. Writes the `claude_cli` block if `claude` is on PATH; comments it out otherwise. Always emits `[models.claude-sonnet-4-6]` with `provider = "claude_cli"`.

5. **`AuthMethod` enum** — `crates/roko-cli/src/auth_detect.rs` line 15. Variants: `ClaudeCli`, `CliProvider { label }`, `AnthropicApi { key, model }`, `OpenAiCompat { key, base_url, model }`, `NeedsSetup`.

6. **`detect_auth_from_env()`** — `crates/roko-cli/src/auth_detect.rs` line 144. Probes in order: `claude --version`, `ANTHROPIC_API_KEY`, `ZAI_API_KEY`, `OPENAI_API_KEY`. Returns first match.

7. **`detect_auth_from_config()`** — `crates/roko-cli/src/auth_detect.rs` line 71. Loads `roko.toml` and checks each `providers.*` entry. Falls back to `detect_auth_from_env()`.

8. **Config presets** — `crates/roko-core/src/config/presets.rs` lines 14–57. `Preset::Minimal/Balanced/Thorough`, each with distinct model tier, gate config, routing, budget, and conductor settings. `Preset::from_str_loose()` parses aliases (`min/fast`, `default/normal`, `max/full`). `Preset::Balanced.to_config()` equals `RokoConfig::default()` (confirmed by test at line 206).

9. **`~/.roko/.env` file** — Written by `run_init_wizard()` in `crates/roko-cli/src/config_cmd.rs` line 178. Loaded at startup by `load_startup_env_files()` in `crates/roko-cli/src/main.rs` line 3544, which first reads `~/.roko/.env` (lower priority, does not override existing env vars), then `.roko/.env` (project-level, higher priority).

10. **`run_init_wizard()`** — `crates/roko-cli/src/config_cmd.rs` line 51. Writes to `~/.roko/config.toml` using the legacy `ConfigLayer` schema (not the v2 `[providers.*]` / `[models.*]` schema). Only detects CLI backends (`claude`, `ollama`, `mods`, `llm`, `aichat`); does not detect API key providers.

11. **Provider/model config schema** — `crates/roko-core/src/config/provider.rs` (`ProviderConfig`, `ModelProfile`). `crates/roko-core/src/config/schema.rs` (`RokoConfig`). The v2 schema uses `providers: BTreeMap<String, ProviderConfig>` and `models: BTreeMap<String, ModelProfile>`.

## Implementation Plan

### Step 1: Add provider-aware `render_init_template` variant

In `crates/roko-cli/src/commands/init.rs`, add a new function (or add parameters to `render_init_template`) that accepts an `AuthMethod` and an optional `Preset`:

```rust
pub(crate) fn render_init_template_with_auth(
    cloud: bool,
    auth: &AuthMethod,
    preset: Preset,
) -> Result<String>
```

Inside, after generating the base config via `preset.to_config()` instead of `RokoConfig::default()`, build the `[providers.*]` and `[models.*]` TOML blocks based on `auth`:

| `AuthMethod` variant | Generated `[providers.*]` | Generated `[models.*]` |
|---|---|---|
| `ClaudeCli` | `kind = "claude_cli"`, `command = "claude"` | slug = `claude-sonnet-4-6`, provider = `claude_cli` |
| `AnthropicApi { .. }` | `kind = "anthropic_api"`, `api_key_env = "ANTHROPIC_API_KEY"` | slug = `claude-sonnet-4-6`, provider = `anthropic` |
| `OpenAiCompat { base_url, .. }` | `kind = "openai_compat"`, `api_key_env = "OPENAI_API_KEY"`, `base_url = ...` | slug = `gpt-5.4-mini`, provider = `openai` |
| `CliProvider { label }` | `kind = label`, `command = label` | (commented template) |
| `NeedsSetup` | (existing commented template) | (existing claude-sonnet-4-6 commented) |

### Step 2: Persist API key to `~/.roko/.env`

In `crates/roko-cli/src/commands/setup.rs`, modify `prompt_for_api_key()` to write the key to `~/.roko/.env` instead of printing an `export` instruction:

```rust
fn persist_api_key(var_name: &str, key: &str) -> Result<()> {
    let env_path = dirs::home_dir()
        .ok_or_else(|| anyhow!("cannot find home dir"))?
        .join(".roko")
        .join(".env");
    // Create ~/.roko/ if it doesn't exist
    if let Some(parent) = env_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Read existing content; update or append the key line
    let mut content = std::fs::read_to_string(&env_path).unwrap_or_default();
    let line = format!("{var_name}={key}\n");
    let prefix = format!("{var_name}=");
    if content.lines().any(|l| l.starts_with(&prefix)) {
        // Replace existing
        content = content.lines()
            .map(|l| if l.starts_with(&prefix) { line.trim_end().to_string() } else { l.to_string() })
            .collect::<Vec<_>>()
            .join("\n");
        content.push('\n');
    } else {
        content.push_str(&line);
    }
    std::fs::write(&env_path, content)?;
    Ok(())
}
```

Call this instead of the current `println!("  Set in your shell: export {var}={key}")`.

### Step 3: Add preset selection step to `cmd_setup`

In `crates/roko-cli/src/commands/setup.rs`, add a preset prompt between the provider detection step and the init step:

```
[3/6] Select configuration preset...
  1. minimal   -- fast, cheap (haiku-class models, skip tests, $5 budget)
  2. balanced  -- default (sonnet-class models, standard gates)      [default]
  3. thorough  -- maximum quality (opus-class models, all gates, $100 budget)

Select preset [2]: _
```

In non-interactive mode (`--yes`), always use `Preset::Balanced`.

Import `roko_core::config::presets::Preset` in `setup.rs`.

### Step 4: Wire the new template into `cmd_setup`

Replace the `cmd_init` call in `cmd_setup` (currently at line 60) with a call that writes `roko.toml` using `render_init_template_with_auth(cloud, &auth, preset)`. If `.roko/` already exists but `roko.toml` has the wrong provider, offer to update it (or skip update if `--yes`).

The flow becomes:

```
[1/6] Detecting project type...       <- detect_project_domain()
[2/6] Detecting available providers... <- detect_auth_from_env() + display list
[3/6] Select provider [default]...    <- interactive if >1 found
[4/6] Select preset [balanced]...     <- interactive
[5/6] Initializing workspace...       <- cmd_init + render_init_template_with_auth
[6/6] Running diagnostics...          <- run_doctor
```

### Step 5: Add project-type detection display

In step [1/6], call `crates/roko-cli/src/commands/prd.rs:detect_project_domain(&workdir)` and print the result. Pass the detected profile to `cmd_init`'s existing `profile` parameter so `append_verification_gates()` includes the right gates (Rust: `cargo check`, `cargo test`, `cargo clippy`; TypeScript: `npm run build`, `npm test`).

### Step 6: Print hint from `roko init`

In `cmd_init` (at the end of the function), add a check: if running non-interactively (i.e., called directly by `roko init`, not via `cmd_setup`), print:

```
hint: Run `roko setup` for guided provider and preset configuration.
```

This preserves backward compatibility for scripts using `roko init` while directing new users to the improved flow.

### Step 7: Update `--yes` non-interactive mode

When `yes = true` in `cmd_setup`:
1. Auto-select the first available provider from `detect_auth_from_env()`.
2. Use `Preset::Balanced`.
3. Auto-detect project profile without prompting.
4. Write config and run doctor.
5. Exit 0 if doctor passes, exit 1 with actionable output otherwise.

This enables CI/Docker: `roko setup --yes && roko plan run plans/`.

## Acceptance Criteria

1. `roko setup` in a fresh directory where `claude --version` succeeds generates a `roko.toml` containing `[providers.claude_cli]` and `[models.claude-sonnet-4-6]`. Running `roko doctor` immediately after reports all checks pass.
2. `roko setup` with only `ANTHROPIC_API_KEY` set (no Claude CLI) generates a `roko.toml` with `[providers.anthropic]` (`kind = "anthropic_api"`) and `roko run "say hello"` succeeds.
3. `roko setup` with only `OPENAI_API_KEY` set generates a `roko.toml` with `[providers.openai]` (`kind = "openai_compat"`) and `[models.gpt-5.4-mini]`.
4. `roko setup` with no providers detected prompts for an API key and writes it to `~/.roko/.env` (not to `roko.toml`). Subsequent `roko doctor` reads the persisted key and reports `provider_usable` as passing.
5. `roko setup` offers preset selection and the generated `roko.toml` reflects the chosen preset (e.g., `Preset::Minimal` sets `max_plan_usd = 5.0` and `skip_tests = true` in the budget/gates sections).
6. `roko setup --yes` completes without prompts, auto-selects the detected provider and `Preset::Balanced`, exits 0 when a provider is available.
7. `roko init` still creates `.roko/` and `roko.toml` without prompting (backward compat for scripts), and prints the hint about `roko setup`.
8. `cargo test --workspace` passes with no regressions.

## Verification Checklist

- [ ] Run `roko setup` in a clean directory with `claude` on PATH; confirm `roko.toml` contains `[providers.claude_cli]`
- [ ] Run `roko setup` with only `ANTHROPIC_API_KEY`; confirm provider block is `kind = "anthropic_api"`
- [ ] Run `roko setup` with only `OPENAI_API_KEY`; confirm provider block is `kind = "openai_compat"` with `base_url = "https://api.openai.com/v1"`
- [ ] Run `roko setup` with no env vars; enter a fake key starting with `sk-ant-`; confirm `~/.roko/.env` contains `ANTHROPIC_API_KEY=sk-ant-...`
- [ ] Select `minimal` preset; confirm `roko.toml` contains `max_plan_usd = 5.0` and `skip_tests = true`
- [ ] Select `thorough` preset; confirm `roko.toml` contains `parallel_enabled = true` and `max_plan_usd = 100.0`
- [ ] Run `roko setup --yes` with `ANTHROPIC_API_KEY` set; confirm exit 0 with no prompts
- [ ] Run `roko init`; confirm hint message appears; confirm `.roko/` and `roko.toml` are created as before
- [ ] `cargo test -p roko-cli` passes clean

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/setup.rs` | Add preset selection step, provider selection UI, persist API key to `~/.roko/.env`, call `render_init_template_with_auth` |
| `crates/roko-cli/src/commands/init.rs` | Add `render_init_template_with_auth(cloud, auth, preset)` that builds provider/model TOML from `AuthMethod` and `Preset` |
| `crates/roko-cli/src/commands/util.rs` | Add hint message in `cmd_init` pointing to `roko setup` |
| `crates/roko-core/src/config/presets.rs` | No changes needed; `Preset::from_str_loose` and `to_config()` are already sufficient |
