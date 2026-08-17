# 04: Zero-Knowledge Onboarding

## Problem

There is no path from "I installed roko" to "roko is developing itself" that doesn't require reading source code or asking someone who already knows.

### What a new user encounters

1. `roko init` — creates `.roko/` and `roko.toml`. Now what?
2. No guided setup for API keys, no model selection wizard.
3. `roko prd idea "..."` works but then what? No prompt says "run `prd draft new` next."
4. `roko prd plan` fails with model errors. User has no idea which models work.
5. `roko plan run` needs a directory path. Which directory? How is it structured?
6. The TUI exists but `roko dashboard` shows empty state with no guidance.

### The Knowledge Required (Currently)

To go from idea to self-development, you need to know:
- The exact sequence: idea → draft → plan → run
- That `default_model` in roko.toml controls generation
- That weak models fail at TOML generation
- That you need specific `[models.*]` entries for non-default models
- That `ANTHROPIC_API_KEY` or `OPENAI_API_KEY` must be set
- That the PRD needs actual content (not just a title) for plan generation to work
- How to read `tasks.toml` format
- That `roko plan run <dir>` runs the plan
- That `roko dashboard` is the TUI

---

## What `roko init` Does Today

`roko init` is implemented in `crates/roko-cli/src/commands/util.rs` (`cmd_init`) and the template renderer in `crates/roko-cli/src/commands/init.rs` (`render_init_template`).

What it creates:
- `.roko/` directory and all subdirectories (prd/, drafts/, published/, jobs/, research/, task-outputs/, subscriptions/, templates/, state/, learn/, episodes/, etc.)
- `roko.toml` with a default config skeleton
- `.roko/engrams.jsonl` (empty signal log)

What `roko.toml` template does:
- Detects `--profile rust` or `--profile typescript` and appends matching `[[gate]]` entries
- Detects if `claude` CLI is on PATH and adds a `[providers.claude_cli]` block if found; otherwise adds a commented-out block
- Always adds a `[models.claude-sonnet-4-6]` entry pointing at the `claude_cli` provider
- Sets `serve.auth.enabled = false` for local development with a comment explaining it

What it does NOT do:
- Validate that any API keys are set
- Detect `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` / etc.
- Prompt the user for their key
- Explain what to do next

---

## What `roko doctor` Checks Today

Implemented in `crates/roko-cli/src/doctor.rs`, `run_doctor` runs 10 checks:

| Check ID | What It Checks | Status on Failure |
|----------|---------------|-------------------|
| `workdir` | Workspace directory exists | `fail` → fix: `roko init` |
| `config` | `roko.toml` present (project or env override) | `fail` → fix: `roko init` |
| `layout` | `.roko/` directory and top-level subdirs exist | `fail` → fix: `roko init` |
| `claude_cli` | `claude --version` succeeds | `warn` → fix: `npm install -g @anthropic-ai/claude-cli` |
| `anthropic_api_key` | `ANTHROPIC_API_KEY` is set and non-empty | `warn` → fix: `export ANTHROPIC_API_KEY=sk-ant-...` |
| `rust_version` | `rustc --version` >= 1.91 | `fail` → fix: `rustup update stable` |
| `node_version` | `node --version` >= 22 | `warn` or `skipped` |
| `serve_auth` | If `[serve]` configured: auth key present when enabled | `fail` → fix: `roko config set serve.auth.api_key <key>` |
| `serve_health` | HTTP GET to health endpoint if `--serve-url` given | `fail` or `skipped` |
| `v2_abstractions` | Compile-time probe: Cell, Signal, Observe, Connect, Trigger importable | `fail` if symbols missing |

Each failing or warning check includes a `fix` field — a one-line command printed with `→ fix:` in human output.

### What `roko doctor` Should Also Check (Gaps)

The current checks don't cover:

1. **Configured providers are actually usable.** If `roko.toml` has a `[providers.anthropic]` entry with `api_key_env = "ANTHROPIC_API_KEY"` but the env var is empty, doctor only checks the env var directly, not whether the configured providers have resolvable keys.

2. **Default model exists in the models table.** If `agent.default_model = "claude-opus-4"` but there's no `[models.claude-opus-4]` entry, plan generation will fail with a confusing error.

3. **Plan directory has a valid structure.** `roko plan run plans/` succeeds only if `plans/` contains a `tasks.toml`. Doctor should check if any plans exist and are valid.

4. **At least one working dispatch path exists.** `detect_auth_from_config` (see below) can determine this; doctor should run it and fail if `NeedsSetup`.

5. **OpenAI and other provider keys.** Only `ANTHROPIC_API_KEY` is currently checked.

---

## Provider Auto-Detection: What Already Exists

`crates/roko-cli/src/auth_detect.rs` implements full auto-detection of available auth methods. This is already wired and used in `roko run`/`roko do`, but is NOT called from `roko doctor` or `roko init`.

```rust
pub enum AuthMethod {
    ClaudeCli,                               // `claude --version` succeeds
    AnthropicApi { key, model },             // ANTHROPIC_API_KEY set
    OpenAiCompat { key, base_url, model },   // OPENAI_API_KEY or ZAI_API_KEY set
    NeedsSetup,                              // nothing found
}

// Priority order:
pub fn detect_auth_from_env() -> AuthMethod {
    // 1. `claude` CLI binary check
    // 2. ANTHROPIC_API_KEY
    // 3. ZAI_API_KEY (Zhipu/GLM, base: https://open.bigmodel.cn/api/paas/v4)
    // 4. OPENAI_API_KEY (with optional OPENAI_API_BASE override)
    // 5. NeedsSetup
}

// Config-aware version — reads roko.toml first, falls back to detect_auth_from_env()
pub fn detect_auth_from_config(workdir: &Path) -> AuthMethod
```

This function is the hook for both `roko setup` and an enhanced `roko doctor`. If it returns `NeedsSetup`, the user needs guidance. If it returns anything else, a working config can be auto-generated.

### Auto-Generate a Config from Detected Providers

When `detect_auth_from_env` returns a non-`NeedsSetup` result, a working `roko.toml` fragment can be synthesized without user input:

```rust
fn generate_provider_block(method: &AuthMethod) -> String {
    match method {
        AuthMethod::ClaudeCli => {
            "[providers.claude_cli]\nkind = \"claude_cli\"\ncommand = \"claude\"\n\
             \n[models.claude-sonnet-4-6]\nprovider = \"claude_cli\"\n\
             slug = \"claude-sonnet-4-6\"\ncontext_window = 200000\n\
             tool_format = \"anthropic_blocks\"\nmax_tools = 32\n"
        }
        AuthMethod::AnthropicApi { .. } => {
            "[providers.anthropic]\nkind = \"anthropic_api\"\napi_key_env = \"ANTHROPIC_API_KEY\"\n\
             \n[models.claude-sonnet-4-6]\nprovider = \"anthropic\"\n\
             slug = \"claude-sonnet-4-6\"\ncontext_window = 200000\n\
             tool_format = \"anthropic_blocks\"\nmax_tools = 32\n"
        }
        AuthMethod::OpenAiCompat { base_url, model, .. } => {
            // Generates an openai-compat provider block with base_url
            format!(
                "[providers.openai]\nkind = \"openai_compat\"\n\
                 base_url = \"{base_url}\"\napi_key_env = \"OPENAI_API_KEY\"\n\
                 \n[models.{model}]\nprovider = \"openai\"\nslug = \"{model}\"\n\
                 context_window = 128000\ntool_format = \"openai_functions\"\n",
                base_url = base_url,
                model = model.as_deref().unwrap_or("gpt-5.4-mini"),
            )
        }
        AuthMethod::NeedsSetup => String::new(),
    }
}
```

---

## Proposed `roko setup` Wizard

### Design

`roko setup` is a first-run wizard. It should be callable anytime to re-configure. It should:

1. Detect what's already available (API keys, CLI binaries)
2. Confirm or ask for what's missing
3. Write a working `roko.toml`
4. Run `roko doctor` to verify
5. Show next steps

### Step-by-Step Flow

```
$ roko setup

Welcome to Roko — agent toolkit for self-developing systems.

Detecting available providers...
  claude CLI: found (v1.0.3)
  ANTHROPIC_API_KEY: not set
  OPENAI_API_KEY: not set

The claude CLI is available. Roko can use it without an API key.

? Use claude CLI as the default provider? [Y/n] Y

? Default model for plan generation:
  > claude-sonnet-4-6  (recommended — fast, reliable TOML output)
    claude-opus-4-6    (most capable, slower)
    claude-haiku-4-5   (cheapest, may fail on complex tasks)

? Project type for gate configuration:
  > auto-detect
    rust
    typescript
    none

Auto-detected: rust (Cargo.toml found)

Writing roko.toml...
Running roko init...
Running roko doctor...

  [ok] workdir
  [ok] config
  [ok] layout
  [ok] claude_cli
  [warn] anthropic_api_key — not set (ok, using claude CLI instead)
  [ok] rust_version (1.85.0)

Setup complete.

Next steps:
  roko develop "describe what you want to build"   # start from an idea
  roko dashboard                                   # watch the TUI
  roko doctor                                      # check status anytime
```

### Rust Sketch

```rust
// crates/roko-cli/src/setup.rs

pub async fn run_setup(workdir: &Path, non_interactive: bool) -> Result<()> {
    // Step 1: Detect
    let auth = detect_auth_from_env();

    match &auth {
        AuthMethod::NeedsSetup => {
            println!("No LLM provider detected.");
            if non_interactive {
                return Err(anyhow::anyhow!(
                    "No provider available. Set ANTHROPIC_API_KEY or install claude CLI."
                ));
            }
            // Prompt for API key
            let key = prompt_for_api_key()?;
            std::env::set_var("ANTHROPIC_API_KEY", &key);
        }
        other => {
            println!("Detected provider: {}", other.label());
        }
    }

    // Step 2: Model selection
    let model = if non_interactive {
        "claude-sonnet-4-6".to_string()
    } else {
        prompt_model_selection(&auth)?
    };

    // Step 3: Profile detection
    let profile = detect_project_profile(workdir);

    // Step 4: Write config
    let config_path = workdir.join("roko.toml");
    if !config_path.exists() {
        let template = render_init_template_for_auth(&auth, &model, profile)?;
        tokio::fs::write(&config_path, template).await?;
        println!("Wrote {}", config_path.display());
    } else {
        println!("{} already exists; patching provider section.", config_path.display());
        patch_config_with_provider(&config_path, &auth, &model).await?;
    }

    // Step 5: Init layout
    cmd_init(Some(workdir.to_path_buf()), false, Some(profile.to_string()), false).await?;

    // Step 6: Doctor
    let report = run_doctor(&DoctorOptions {
        workdir: workdir.to_path_buf(),
        config_override: None,
        serve_url: None,
    }).await?;
    println!("{}", report.render_human());

    // Step 7: Next steps
    if report.healthy {
        println!("\nNext steps:");
        println!("  roko develop \"describe what you want to build\"");
        println!("  roko dashboard");
    } else {
        println!("\nSetup completed with issues. Run `roko doctor` for details.");
    }

    Ok(())
}
```

---

## Enhanced `roko doctor` Checks

The following checks should be added to `run_doctor` in `crates/roko-cli/src/doctor.rs`:

### Check: `provider_usable`

```rust
fn check_provider_usable(workdir: &Path) -> DoctorCheck {
    let auth = detect_auth_from_config(workdir);
    match auth {
        AuthMethod::NeedsSetup => DoctorCheck {
            id: "provider_usable".into(),
            status: DoctorStatus::Fail,
            message: "no LLM provider is usable".into(),
            detail: Some("No claude CLI, ANTHROPIC_API_KEY, or OPENAI_API_KEY found".into()),
            path: None,
            url: None,
            fix: Some("roko setup".into()),
        },
        other => DoctorCheck {
            id: "provider_usable".into(),
            status: DoctorStatus::Ok,
            message: format!("provider available: {}", other.label()),
            detail: None, path: None, url: None, fix: None,
        },
    }
}
```

### Check: `default_model_configured`

```rust
fn check_default_model_configured(loaded_config: &LoadedConfig) -> DoctorCheck {
    let Some(config) = &loaded_config.resolved else {
        return DoctorCheck { id: "default_model".into(), status: DoctorStatus::Skipped, .. };
    };
    let model = &config.agent.default_model;
    if model.is_empty() {
        return DoctorCheck {
            id: "default_model".into(),
            status: DoctorStatus::Warn,
            message: "agent.default_model is empty".into(),
            fix: Some("roko config set agent.default_model claude-sonnet-4-6".into()),
            ..
        };
    }
    if !config.models.contains_key(model.as_str()) {
        return DoctorCheck {
            id: "default_model".into(),
            status: DoctorStatus::Fail,
            message: format!("default model '{model}' has no [models.{model}] entry"),
            fix: Some(format!("add [models.{model}] to roko.toml")),
            ..
        };
    }
    DoctorCheck { id: "default_model".into(), status: DoctorStatus::Ok, message: format!("default model '{model}' is configured"), .. }
}
```

---

## First-Run Experience: What Should Happen

When someone installs roko and runs it for the first time, one of two things should happen:

### Path A: `roko` with no subcommand

Currently shows help text. A better experience:

```
$ roko
Roko v0.x.x — agent toolkit for self-developing systems.

Looks like this is a fresh workspace. Run:
  roko setup     — configure providers and initialize the workspace
  roko doctor    — check what's missing
  roko --help    — full command reference
```

Detection: if no `roko.toml` exists AND no `.roko/` directory exists, print the above instead of the normal help.

### Path B: `roko doctor` before `roko init`

If the user runs `roko doctor` cold (no config, no layout), the current output is:

```
[fail] config: missing project roko.toml
[fail] layout: missing .roko directory
[warn] claude_cli: claude CLI not found on PATH
[warn] anthropic_api_key: ANTHROPIC_API_KEY not set
```

The enhanced output should include the setup path more prominently:

```
[fail] config: missing project roko.toml
       → fix: roko setup   (or roko init if you have roko.toml already)
[fail] layout: missing .roko directory
       → fix: roko setup
[warn] claude_cli: not found   → fix: npm install -g @anthropic-ai/claude-cli
[warn] anthropic_api_key: not set   → fix: export ANTHROPIC_API_KEY=sk-ant-...

1 suggestion: run `roko setup` to configure everything at once.
```

### Path C: Auto-Config Generation

If `ANTHROPIC_API_KEY` is set in the environment but no `roko.toml` exists, `roko init` should automatically add the Anthropic API provider block instead of just the commented-out claude CLI block. This requires calling `detect_auth_from_env` inside `render_init_template`.

The current `render_init_template` in `init.rs` only detects the `claude` CLI binary. The enhancement:

```rust
pub(crate) fn render_init_template(cloud: bool) -> Result<String> {
    let auth = detect_auth_from_env();
    match auth {
        AuthMethod::ClaudeCli => {
            // existing behavior: write [providers.claude_cli]
        }
        AuthMethod::AnthropicApi { .. } => {
            // write [providers.anthropic] with api_key_env = "ANTHROPIC_API_KEY"
        }
        AuthMethod::OpenAiCompat { base_url, model, .. } => {
            // write [providers.openai] with base_url and api_key_env
        }
        AuthMethod::NeedsSetup => {
            // write commented-out blocks for all providers with a note:
            // "# No provider detected. Set ANTHROPIC_API_KEY or install claude CLI."
        }
    }
}
```

---

## Contextual "What Next" Prompts

After every successful command, the output should include the natural next step. This requires a small `next_step` string added to the success output of each command:

### After `roko init`

```
✓ Initialized .roko/ in /path/to/project
✓ Wrote roko.toml

Next: roko develop "describe what you want to build"
  or: roko doctor   to verify the setup
```

### After `roko prd idea`

```
✓ Captured idea: implement cursor support

Next: roko develop "implement cursor support"
      (or keep adding ideas, then run `roko develop` to synthesize all)
```

### After `roko prd plan`

```
✓ Plan generated: 6 tasks, ~90 min estimated
  Written to: .roko/prd/plans/cursor-backend/tasks.toml

Next: roko plan run .roko/prd/plans/cursor-backend/
  or: roko dashboard   to approve and watch interactively
```

Implementation: each `cmd_*` function can call a `print_next_step(step: &str)` helper at the end. The helper prints to stdout only when output is a TTY (suppress in CI/pipe contexts via `atty::is(atty::Stream::Stdout)`).

---

## Summary of Changes

| File | Change | Priority |
|------|--------|----------|
| `crates/roko-cli/src/commands/init.rs` | Call `detect_auth_from_env`, auto-write provider block | High |
| `crates/roko-cli/src/doctor.rs` | Add `provider_usable` + `default_model_configured` checks | High |
| `crates/roko-cli/src/setup.rs` | New file: `roko setup` interactive wizard | High |
| `crates/roko-cli/src/main.rs` | First-run detection: no config + no layout → show setup hint | Medium |
| `crates/roko-cli/src/commands/util.rs` | `cmd_init` prints next step after success | Medium |
| `crates/roko-cli/src/commands/prd.rs` | Print next step after idea/plan | Medium |
| `crates/roko-cli/src/commands/plan.rs` | Print next step after plan run | Medium |
