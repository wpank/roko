# 14 — Providers / Dispatch Convergence

> Cross-cutting plan covering items in `tmp/workflow/providers/` (00-INDEX through 11-ACTION-PLAN). Many items are now subsumed by plans 01–04; this plan covers the gaps that aren't.

---

## Status (2026-05-01)

**PARTIAL.** Tool output, session id, cost display, streaming have all been done for the **chat session** path. `auth_detect` does not accept `ResolvedRuntimeConfig`. `OpenAiAgent` is non-streaming by design. Plan/orchestrate paths still use a separate stack (which plans 06+11 fix). Items "Done" per `tmp/workflow/providers/11-ACTION-PLAN.md`:

- Tool output capture in `dispatch_direct` — done
- Tool output rendering in `chat_inline` — done
- `session_id` capture in `dispatch_direct` — done
- `extract_text` includes Tool events — done
- `extract_tool_outputs()` method added — done
- `extract_session_id()` method added — done
- Global config merges `default_model` / `default_backend` — done
- Auth detection reordered (ZAI first) — done
- TOOL symbol added to symbols.rs — done

**What's not done (still relevant):**

- **1D** Auth detection from config — `detect_auth()` doesn't accept `RokoConfig`; ignores `[agent].default_backend` ordering preference.
- **1E** `dispatch_direct` config-driven — feature-gated, may delete instead (see plan 12)
- **1F** Cost tracking from API paths — done in chat session via `ModelCallService`; verify HTTP / API paths
- **2A** Session resume in chat — `session_id` captured but not surfaced to `chat_session` for follow-up `--resume <id>`
- **2B** Per-role tool enforcement uniform across CLI + chat — chat uses `resolve_tool_policy`/safety contract; CLI uses `claude_tool_allowlist`. Different policies.
- **2C** CascadeRouter wiring — covered by plan 08
- **2D** `--fallback-model` in all paths — only one path passes it
- **2E** Stderr classification (benign vs important) — not implemented
- **2F** DOA detection (process exits within 2s) — not implemented
- **3A** Consolidate dispatch paths — covered by plan 01 + 11
- **3B** Inference gateway — partially via `ModelCallService`; covered by plan 01
- **3C** Config consolidation — `ResolvedRuntimeConfig` exists in `roko-core::config::provenance` but **not threaded through CLI**

---

## Goal

Every leftover item from `tmp/workflow/providers/11-ACTION-PLAN.md` that is still relevant lands. The result:

- Auth detection consults `ResolvedRuntimeConfig` for `default_backend` preference
- `ResolvedRuntimeConfig` is the single source of truth, threaded from `roko.toml` load through every CLI command + HTTP route
- All Claude CLI spawns pass `--fallback-model` (sourced from config)
- Stderr classification: benign lines (`progress`, `Downloading`, etc.) are filtered; important lines (`error`, `panic`) surface
- DOA detection: process death within 2 seconds → typed error with provider/binary/auth context
- Per-role tool policy is consistent: `roko run` and `roko` (chat) both enforce role-based allowlists from the same registry
- Session resume in chat: `roko` interactive remembers `session_id` across turns and passes it via `ModelCallRequest.routing_hints` → adapter

---

## Why This Exists (Anti-Patterns Eliminated)

- **#5 Hardcoded Role Behavior** — `claude_tool_allowlist` in CLI is parallel to chat's tool policy
- **#7 Copy-Paste** — three places handle stderr; only one classifies
- **#1 Just Shell Out** — DOA detection requires the wrapper to know what spawning means

---

## Existing Code — Read These First

- `tmp/workflow/providers/00-INDEX.md` through `11-ACTION-PLAN.md` for original design
- `crates/roko-cli/src/auth_detect.rs` — current `detect_auth()`
- `crates/roko-core/src/config/provenance.rs` — `ResolvedRuntimeConfig`
- `crates/roko-cli/src/config.rs` (modified) — config load path
- `crates/roko-cli/src/commands/config_cmd.rs` (modified) — config inspection commands
- `crates/roko-agent/src/provider/claude_cli/` — CLI spawn details
- `crates/roko-agent/src/safety/contract.rs::ToolAllowlist` — role policy

---

## Implementation Steps

### Step 1 — `ResolvedRuntimeConfig` everywhere

**Goal:** one config object loaded once, threaded through every entry point. No more `RokoConfig` reads scattered across modules.

**Files:**

- `crates/roko-core/src/config/provenance.rs` — define / extend `ResolvedRuntimeConfig`:

```rust
#[derive(Debug, Clone)]
pub struct ResolvedRuntimeConfig {
    pub default_backend: Option<String>,
    pub default_model: Option<String>,
    pub providers: HashMap<String, ProviderConfig>,
    pub roles: RoleRegistry,
    pub gates: GateConfig,
    pub runtime: RuntimeSettings,                  // checkpoint_interval, log_level, etc.
    pub routing: RoutingSettings,                  // force_backend, tier_defaults
    pub features: FeatureFlags,                    // experimental toggles
    pub provenance: ConfigProvenance,              // tracks "this field came from /etc/roko.toml line 5"
}
```

`ConfigProvenance` records every field's origin (CLI flag, env var, file path + line). Useful for `roko config doctor` to explain "why is this model selected".

- `crates/roko-cli/src/main.rs` — load once, pass into `Cli` struct:

```rust
fn main() -> ExitCode {
    let cli = Cli::parse();
    let config = ResolvedRuntimeConfig::load(&cli.config_path).context("loading config")?;
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(cli.dispatch(config))
}
```

- `crates/roko-cli/src/cli.rs` — `Cli::dispatch(self, config: ResolvedRuntimeConfig)` — every command takes `&ResolvedRuntimeConfig` as input

### Step 2 — `detect_auth(config)` accepts config

**File:** `crates/roko-cli/src/auth_detect.rs`

```rust
pub fn detect_auth(config: &ResolvedRuntimeConfig) -> AuthMethod {
    // 1. If config has default_backend, try its api_key_env first
    if let Some(backend) = &config.default_backend {
        if let Some(provider) = config.providers.get(backend) {
            if let Some(key_env) = &provider.api_key_env {
                if std::env::var(key_env).is_ok() {
                    return AuthMethod::for_provider(provider);
                }
            }
        }
    }
    // 2. CLI probe (existing)
    if claude_cli_available() { return AuthMethod::ClaudeCli; }
    // 3. Env-var fallback (existing)
    if std::env::var("ANTHROPIC_API_KEY").is_ok() { return AuthMethod::AnthropicApi; }
    if std::env::var("OPENAI_API_KEY").is_ok() { return AuthMethod::OpenAiCompat(default_openai_provider()); }
    // ...
    AuthMethod::None
}
```

Update callers:

- `crates/roko-cli/src/unified.rs` — passes `config` to `detect_auth`
- `crates/roko-cli/src/chat_inline.rs` — same

### Step 3 — `--fallback-model` in all Claude CLI spawns

**File:** `crates/roko-agent/src/provider/claude_cli/mod.rs`

The Claude CLI accepts `--fallback-model <model>` for graceful degradation. Today only one spawn site uses it.

```rust
// crates/roko-agent/src/provider/claude_cli/mod.rs
fn build_invocation(opts: &ClaudeCliOpts) -> ClaudeCliInvocation {
    let mut args = vec![
        "--print".into(),
        "--output-format".into(), "stream-json".into(),
        "--model".into(), opts.model.clone(),
    ];
    if let Some(fallback) = &opts.fallback_model {
        args.push("--fallback-model".into());
        args.push(fallback.clone());
    }
    // ... existing args ...
    ClaudeCliInvocation { program: opts.program.clone(), args, env: opts.env.clone() }
}
```

`ClaudeCliOpts.fallback_model` is sourced from `ResolvedRuntimeConfig.providers["anthropic"].fallback_model`, defaulting to `claude-haiku-4`.

### Step 4 — Stderr classification

```rust
// crates/roko-agent/src/stderr_classifier.rs
#[derive(Debug, Clone)]
pub enum StderrLine {
    Benign(String),                   // progress, downloads, info logs
    Important(String),                // warnings the user should see
    Error(String),                    // errors that must surface
}

pub fn classify(line: &str) -> StderrLine {
    let lower = line.to_lowercase();

    // benign patterns
    if lower.starts_with("downloading") || lower.starts_with("compiling") || lower.starts_with("info ") || lower.contains(": progress") {
        return StderrLine::Benign(line.into());
    }

    // important patterns
    if lower.starts_with("warning") || lower.contains("deprecated") || lower.contains("unused") {
        return StderrLine::Important(line.into());
    }

    // error patterns
    if lower.starts_with("error") || lower.contains("panic") || lower.contains("aborted") || lower.contains("permission denied") {
        return StderrLine::Error(line.into());
    }

    // default: surface as Important (safe choice — better to over-report)
    StderrLine::Important(line.into())
}

pub fn benign_summary(lines: impl IntoIterator<Item = String>) -> StderrSummary {
    let mut benign = 0;
    let mut important = Vec::new();
    let mut error = Vec::new();
    for line in lines {
        match classify(&line) {
            StderrLine::Benign(_) => benign += 1,
            StderrLine::Important(s) => important.push(s),
            StderrLine::Error(s) => error.push(s),
        }
    }
    StderrSummary { benign_count: benign, important, error }
}
```

Wire into:

- `crates/roko-agent/src/provider/claude_cli/spawn.rs` — every read from child stderr classifies
- `crates/roko-cli/src/runner/agent_stream.rs` — same
- ACP cognitive dispatch — same

Surface in:

- `RuntimeEvent::AgentStderrSummary { run_id, agent_id, summary: StderrSummary }` (added per plan 10)
- `chat_inline` displays only `important` and `error`; benign hidden behind a "show stderr" toggle

### Step 5 — DOA detection

```rust
// crates/roko-agent/src/spawn_wrapper.rs
pub async fn spawn_with_doa_detection(
    invocation: Invocation,
    doa_threshold: Duration,
) -> Result<SpawnedProcess> {
    let start = Instant::now();
    let mut child = invocation.spawn().await?;

    // Race: watch for early exit
    tokio::select! {
        status = child.wait_with_deadline(doa_threshold) => {
            let elapsed = start.elapsed();
            if elapsed < doa_threshold && !status.success() {
                let stderr = child.collect_stderr().await?;
                return Err(SpawnError::Doa {
                    elapsed_ms: elapsed.as_millis() as u64,
                    exit_code: status.code(),
                    classified_error: classify_doa(&stderr, &invocation),
                });
            }
            // exited "normally" within threshold
            Ok(SpawnedProcess::Completed(status))
        }
        _ = tokio::time::sleep(doa_threshold) => {
            // process still running after threshold; not DOA
            Ok(SpawnedProcess::Running(child))
        }
    }
}

fn classify_doa(stderr: &str, invocation: &Invocation) -> DoaCause {
    if stderr.contains("command not found") || stderr.contains("No such file") {
        return DoaCause::BinaryMissing { program: invocation.program.clone() };
    }
    if stderr.contains("authentication") || stderr.contains("API key") || stderr.contains("unauthorized") {
        return DoaCause::AuthFailed { provider: detect_provider(invocation) };
    }
    if stderr.contains("rate limit") {
        return DoaCause::RateLimited;
    }
    if stderr.contains("model") && stderr.contains("not found") {
        return DoaCause::ModelNotAvailable { model: extract_model(invocation) };
    }
    DoaCause::Unknown { stderr_excerpt: stderr.lines().take(3).collect::<Vec<_>>().join("\n") }
}
```

Threshold: 2 seconds (per audit). Configurable via `ResolvedRuntimeConfig.runtime.doa_threshold_ms`.

Surface as `RuntimeEvent::AgentDoa { run_id, agent_id, cause: DoaCause }`. Chat displays a clear actionable message ("`claude` binary not found in PATH; install via `npm install -g @anthropic-ai/claude-cli`").

### Step 6 — Unified per-role tool policy

**File:** `crates/roko-agent/src/safety/role_tools.rs`

```rust
#[derive(Debug, Clone)]
pub struct RoleToolPolicy {
    role: AgentRole,
    allowed: HashSet<String>,
    denied: HashSet<String>,
    require_before_edit: Option<String>,            // e.g. read_file before write_file
}

pub fn policy_for_role(role: AgentRole, contract: &AgentContract) -> RoleToolPolicy {
    // Build from contract + role manifest
    RoleToolPolicy {
        role,
        allowed: contract.governance.iter().filter_map(|g| match g {
            GovernanceRule::AllowedTools(tools) => Some(tools.iter().cloned().collect()),
            _ => None,
        }).flatten().collect(),
        denied: contract.governance.iter().filter_map(|g| match g {
            GovernanceRule::ForbiddenTools(tools) => Some(tools.iter().cloned().collect()),
            _ => None,
        }).flatten().collect(),
        require_before_edit: contract.governance.iter().find_map(|g| match g {
            GovernanceRule::RequireToolBeforeEdit(tool) => Some(tool.clone()),
            _ => None,
        }),
    }
}
```

Replace:

- `crates/roko-cli/src/run.rs::claude_tool_allowlist` — call `policy_for_role(role, contract).allowed.into_iter().collect::<Vec<_>>()`
- `crates/roko-cli/src/chat_session.rs::resolve_tool_policy` — same call site

After this, both CLI and chat use the **same** policy from the **same** contract.

### Step 7 — Session resume in chat

**File:** `crates/roko-cli/src/chat_session.rs`

`ChatAgentSession` already stores `session_id` (per audit, ~1297-1300, ~1661-1662). Surface it via `ModelCallRequest.routing_hints`:

```rust
let mut hints = Vec::new();
if let Some(sid) = self.last_session_id.as_ref() {
    hints.push(format!("claude:resume:{}", sid));
}
let req = ModelCallRequest {
    routing_hints: hints,
    ...
};
```

In the Claude CLI adapter, parse `claude:resume:<id>` hints and add `--resume <id>` to args:

```rust
// crates/roko-agent/src/provider/claude_cli/mod.rs
let resume_id = req.routing_hints.iter().find_map(|h| h.strip_prefix("claude:resume:"));
if let Some(id) = resume_id {
    args.push("--resume".into());
    args.push(id.into());
}
```

After every successful response, capture the new `session_id` from the Result event and store it in `ChatAgentSession.last_session_id`.

### Step 8 — `roko config doctor` command

The `ResolvedRuntimeConfig` carries provenance. Add a CLI command that prints:

```
$ roko config doctor
[backend] default_backend = "anthropic"
   from: /Users/me/.config/roko/config.toml line 5

[model] default_model = "claude-sonnet-4"
   from: env var ROKO_DEFAULT_MODEL

[auth] selected: ClaudeCli (binary at /opt/homebrew/bin/claude, version 0.4.2)
   reason: default_backend "anthropic" → claude provider → ClaudeCli adapter (CLI available)

[providers] 4 configured: anthropic, openai, gemini, ollama
   anthropic: ANTHROPIC_API_KEY present (sk-ant-***)
   openai:    OPENAI_API_KEY missing — provider unavailable
   gemini:    GEMINI_API_KEY present
   ollama:    base_url http://localhost:11434 — reachable: yes

[providers.anthropic.fallback_model] "claude-haiku-4"
   from: built-in default

[runtime.routing.force_backend] not set
[runtime.checkpoint_interval_ms] 5000
   from: /Users/me/.config/roko/config.toml line 47
```

Implementation in `crates/roko-cli/src/commands/config_cmd.rs`. Operators can run this to debug "why isn't my preferred model being used".

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #3 Build another runtime | Adding a new "InferenceGateway" struct to host these features | Reuse `ModelCallService` + `ResolvedRuntimeConfig` |
| #5 Hardcoded role behavior | If/elsing "if role == implementer { allow bash }" | All policies sourced from contract YAML |
| #7 Copy-paste | Multiple stderr classifiers | One `roko-agent/src/stderr_classifier.rs` |

---

## Things NOT To Do

1. **Don't make `ResolvedRuntimeConfig` mutable.** Once loaded, it's immutable for the run. Hot-reload happens by replacing the whole struct via `ArcSwap`.
2. **Don't store API keys in `ResolvedRuntimeConfig`.** Keys are looked up via `api_key_env` at call time by `ModelCallService`. The config stores the *name* of the env var, never the value.
3. **Don't classify stderr aggressively.** When in doubt, surface (Important). Suppressing real errors is worse than showing chatter.
4. **Don't make DOA threshold too low.** 2 seconds catches "binary not found" but lets normal init proceed. Configurable, but don't go below 1 second.
5. **Don't add `--fallback-model` if the provider doesn't support it.** OpenAI compat doesn't have a fallback flag. Skip per-provider.
6. **Don't reuse `routing_hints` for arbitrary key/value.** It's a `Vec<String>` for compactness, but only well-known prefixes (`claude:resume:`, `attempt:N`, etc.) should be used. Don't reinvent JSON-in-string.
7. **Don't skip the `roko config doctor` command.** It's the difference between users debugging in 30 seconds vs filing an issue.
8. **Don't merge per-role tool policy lookups into the runtime hot path.** Pre-compute per role at startup, cache in `EffectServices`.

---

## Tests / Proof Criteria

```bash
# 1. ResolvedRuntimeConfig threaded
rg 'ResolvedRuntimeConfig' crates/roko-cli/src/ --type rust | wc -l
# expected: 5+ (main, cli, run, chat_inline, every command)

# 2. Stderr classifier exists
rg 'fn classify.*StderrLine' crates/roko-agent/src/ --type rust
# expected: 1+

# 3. DOA detection wired
rg 'spawn_with_doa_detection|DoaCause' crates/roko-agent/src/ --type rust
# expected: usage in spawn paths

# 4. Tool policy unified
rg 'claude_tool_allowlist|resolve_tool_policy' crates/roko-cli/src/ --type rust
# expected: both replaced by policy_for_role

# 5. Session resume in chat
rg '--resume' crates/roko-agent/src/provider/claude_cli/ --type rust
# expected: 1+ usage tied to routing_hints
```

Functional proofs:

- [ ] `roko config doctor` runs and explains every config field's origin
- [ ] `default_backend = "zai"` in config: `roko` (no args) launches via Z.AI provider not Claude CLI even if both auth methods are present
- [ ] `roko --no-session` runs without resume; second `roko` call resumes correctly via `--resume <session_id>`
- [ ] `roko run` against missing `claude` binary produces typed `DoaCause::BinaryMissing` error within 2s, with install hint
- [ ] Stderr lines like `Compiling foo v0.1.0` are filtered from chat display; lines like `error[E0382]:` are surfaced
- [ ] `auditor` role cannot dispatch `bash` tool both in `roko run` and in chat
- [ ] Claude CLI spawn includes `--fallback-model claude-haiku-4` (verify with `ps aux` during a long agent call)

---

## Dependencies

- **Plan 01 (ModelCallService)** — for `routing_hints` plumbing
- **Plan 09 (Safety)** — for unified contract policy
- **Plan 10 (Observability)** — for `RuntimeEvent::AgentStderrSummary` and `RuntimeEvent::AgentDoa`

Independent of plans 04-08 — can be done in parallel.

---

## Estimated Effort

**M.** ~1 week.

- Step 1 (ResolvedRuntimeConfig threading) — M (2-3 days; touches every command)
- Step 2 (auth from config) — S (half day)
- Step 3 (--fallback-model) — S (half day)
- Step 4 (stderr classifier) — S (1 day)
- Step 5 (DOA detection) — S (1 day)
- Step 6 (unified tool policy) — S (1 day)
- Step 7 (session resume in chat) — S (half day)
- Step 8 (`config doctor`) — S (1 day)
