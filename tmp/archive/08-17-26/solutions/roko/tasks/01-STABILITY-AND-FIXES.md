# 01: Stability and Fixes -- Implementation Tasks

## Overview

This document contains 78 concrete implementation tasks to fix crashes, security
vulnerabilities, silent data loss, broken features, and structural anti-patterns in
the roko codebase. Every task references verified file paths, specific structs and
functions, and testable acceptance criteria. Tasks are organized P0 (crashes/security)
first, P1 (broken features) second, P2 (correctness/quality) third.

## Anti-Patterns to Remove

| ID | Pattern | Where Found | Impact |
|---|---|---|---|
| AP-1 | Stub verdicts return PASS instead of SKIP | `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/rung_dispatch.rs` lines 132-239 | Gate pipeline falsely reports rungs 3-6 passed when they were never executed |
| AP-2 | Two parallel model selection paths | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/auth_detect.rs` vs `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/service_factory.rs` | `default_model` in roko.toml ignored by `roko run` |
| AP-3 | Config schema split (`[[gate]]` vs `[gates]`) | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/init.rs` writes `[[gate]]`; `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` reads `[gates]` | Gates from `roko init` silently discarded |
| AP-4 | CascadeRouter has zero live callers | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs` -- every caller passes `None` for router arg | Learning never informs model selection |
| AP-5 | Runner v2 writes no learning signals | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | No episodes, no routing observations, no threshold updates |
| AP-6 | Streaming events silently drained in chat | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs` | TUI shows spinner instead of streaming text |
| AP-7 | Repo context not wired into plan generation | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs` | Plans generated without repository awareness |
| AP-8 | Singleton rate limiter across all providers | `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/openai_compat_backend.rs` lines 31-35 | All providers share one 60 RPM limiter |
| AP-9 | Dual episode writes in `roko run` | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs` | Duplicate records in `.roko/episodes.jsonl` and `.roko/learn/episodes.jsonl` |
| AP-10 | `unsafe set_var` for `--provider` | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs` line 236 | UB in multi-threaded contexts since Rust 1.66 |

---

## Tasks

---

### Task 1.01: Fix `roko config mcp` unreachable panic
**Priority**: P0
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (lines 2132-2136, `dispatch_subcommand`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs` (lines 207-210)
**Depends On**: none

#### Context

The `ConfigCmd::Mcp` match arm in `config_cmd.rs` line 209 hits `unreachable!()` which panics.
However, investigation reveals this is actually intercepted in `dispatch_subcommand()` in
`main.rs` at line 2132, which calls `dispatch_mcp_cmd()` defined at line 2790. This function
handles `ConfigMcpCmd::List` and `ConfigMcpCmd::Test` and returns before reaching the
`unreachable!()`.

**Status re-assessment**: The MCP dispatch IS wired and functional. The `unreachable!()` is
defensive dead code because the arm is intercepted earlier. However, it is fragile -- if
`dispatch_subcommand` is refactored to not intercept MCP, the panic returns.

**Remaining fix**: Verify all `ConfigMcpCmd` variants are handled in `dispatch_mcp_cmd()`.
The `Add` and `Remove` subcommands (if they exist in the enum) may not have handlers.
Replace `unreachable!()` with a proper fallback that calls `dispatch_mcp_cmd` as a safety net.

#### Implementation Steps

1. In `config_cmd.rs`, replace the `unreachable!("mcp dispatched in dispatch_subcommand")` with
   a call to a shared MCP dispatch function, so both paths are safe:
   ```rust
   ConfigCmd::Mcp { cmd } => {
       let wd = resolve_workdir(cli);
       dispatch_mcp_cmd(&cmd, &wd)?;
       Ok(())
   }
   ```
2. Verify all `ConfigMcpCmd` variants (`List`, `Test`, `Add`, `Remove`) have handlers in
   `dispatch_mcp_cmd()`. Add stub handlers for any missing variants that return
   `Err(anyhow!("roko config mcp {variant} is not yet implemented"))`.
3. Add a test: `Cli::try_parse_from(["roko", "config", "mcp", "list"])` runs without panic.

#### Design Guidance

The `unreachable!()` pattern in `config_cmd.rs` is used for 4 arms (Experiments, Plugins,
Secrets, MCP). All of these rely on interception in `dispatch_subcommand`. Consider making all
4 arms handle dispatch directly as a fallback, reducing fragility.

#### Verification Criteria

- [ ] `roko config mcp list` runs without panic (outputs servers or "no MCP config found")
- [ ] `roko config mcp test <name>` runs without panic
- [ ] No `unreachable!()` remains in the MCP arm of `dispatch_config`
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.02: Move share routes inside auth middleware
**Priority**: P0
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/shared_runs.rs` (line 854, `auth_routes()`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs` (lines 117, 170)
**Depends On**: none

#### Context

The share route architecture is already partially correct. `shared_runs.rs` exports two
functions:
- `auth_routes()` (line 854): `POST /runs/{id}/share` -- mounted inside the auth layer at
  `routes/mod.rs` line 117.
- `public_routes()` (line 864): `GET /api/runs/{id}`, `GET /api/shared/{token}`,
  `GET /runs/{id}` -- mounted outside auth at line 170.

**Status re-assessment**: The `POST /runs/{id}/share` route IS behind the auth layer (merged
at line 117, which is inside the `protected` router). The public routes are read-only viewers.

The concern is: verify the actual Axum router nesting to confirm `auth_routes()` is inside
the auth middleware layer. In `routes/mod.rs`, the protected router (lines ~100-120) should
use `.layer(auth_middleware)`. The public router (lines ~160-175) does not.

#### Implementation Steps

1. Audit `routes/mod.rs` to confirm that the router block containing line 117
   (`shared_runs::auth_routes()`) is wrapped in the auth middleware layer.
2. Write an integration test: `POST /api/runs/test/share` without an auth header returns 401.
3. Write an integration test: `GET /api/shared/{token}` without auth returns 200 (public).
4. If the auth layer is NOT applied, move `shared_runs::auth_routes()` inside the auth
   middleware `.layer()` call.

#### Design Guidance

Keep the two-function split (`auth_routes` / `public_routes`). This pattern is clean and
makes auth boundaries explicit. Other route modules should follow this pattern for any
mutation endpoints.

#### Verification Criteria

- [ ] `POST /api/runs/{id}/share` without auth header returns 401
- [ ] `GET /api/shared/{token}` without auth returns 200
- [ ] Integration test covers both cases
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.03: Auto-provision auth on cloud deploy
**Priority**: P0
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/server.rs` (`cmd_deploy_railway`, `cmd_deploy_fly`, `cmd_deploy_docker`)
**Depends On**: none

#### Context

`cmd_deploy_railway()` at line 181 of `server.rs` handles Railway deployment. It writes config
files and invokes the Railway CLI but does not generate or set an API key. The deployed server
binds to `0.0.0.0:6677` with auth disabled, making all ~85 routes publicly accessible.

Same applies to `cmd_deploy_fly()` (line 333) and `cmd_deploy_docker()`.

#### Implementation Steps

1. At the top of each deploy function (`cmd_deploy_railway`, `cmd_deploy_fly`, `cmd_deploy_docker`):
   - Generate a 32-byte random hex string: `use rand::Rng; let key: String = rand::thread_rng().sample_iter(...).take(32).map(|b| format!("{:02x}", b)).collect();`
   - Or use `uuid::Uuid::new_v4().to_string()` if simpler.
2. Set the API key as an environment variable in the deployment:
   - Railway: add `ROKO_API_KEY` to the Railway service variables via GraphQL or env file.
   - Fly: add to `fly.toml` `[env]` section or `flyctl secrets set`.
   - Docker: add to generated Dockerfile or docker-compose as env var.
3. Set `api_auth.enabled = true` in the generated `roko.toml` for the deployment.
4. Print the generated key with a prominent warning:
   ```
   API Key: {key}
   Save this API key. It will not be shown again.
   ```
5. Add `rand` to `roko-cli` dev-dependencies if not already present.

#### Design Guidance

The key generation should be a shared utility function `generate_api_key() -> String` in
a common module (e.g., `config_helpers.rs`), callable from all deploy targets. Consider
adding `--no-auth` flag for deploy commands that explicitly opts out with a warning.

#### Verification Criteria

- [ ] `roko deploy railway` output includes a generated API key
- [ ] Generated deployment config has `api_auth.enabled = true`
- [ ] `ROKO_API_KEY` environment variable is set in deployment
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.04: Add secret scrubbing to CLI Gist share path
**Priority**: P0
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/share.rs`
**Depends On**: none

#### Context

`share.rs` already has `scrub_share_text()` (line 49) and `scrub_long_secret_like_strings()`
(line 61). It already applies scrubbing to the prompt and output text before creating the
share artifact (lines 93-103). The function `scrub_share_text` uses `LogScrubber` and
secondary regex patterns for long hex/base64 strings.

**Status re-assessment**: The CLI share path ALREADY scrubs secrets. Tests at lines 302-328
verify this behavior (`scrub_share_text_redacts_api_key_in_prompt`, etc.).

The audit finding may be stale or the scrubbing may have been added after the audit.
Verify that the scrubbing covers all paths (Gist upload, local file write, stdout output).

#### Implementation Steps

1. Verify that `share_run()` (or equivalent) calls `scrub_share_text()` on ALL text content
   before writing to disk or uploading to Gist.
2. Verify Gist upload path (if separate from local file write) also scrubs.
3. Add a test: transcript containing `ANTHROPIC_API_KEY=sk-ant-test123` produces Gist
   content with `[REDACTED]`.
4. If any path is found unscrubbed, add `scrub_share_text()` call.

#### Design Guidance

The scrubbing is already well-implemented. The key improvement is ensuring coverage of all
output paths, not adding new scrubbing logic. Consider also scrubbing the `tool_calls` field
in the report if tools received secrets as arguments.

#### Verification Criteria

- [ ] All share output paths (local file, Gist upload) apply `scrub_share_text()`
- [ ] Tests verify redaction of API keys, long hex strings, long base64 strings
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.05: Fix `acknowledge_public_risk` bypass
**Priority**: P0
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` (lines 641-657)
**Depends On**: none

#### Context

The current logic at line 644 of `lib.rs`:
```rust
if serve.acknowledge_public_risk {
    warn!(addr = %addr, "binding to a public address without authentication; all routes will be network-accessible");
    return Ok(());
}
anyhow::bail!("Public bind requires `serve.auth.enabled = true` or `serve.acknowledge_public_risk = true`.");
```

This means `acknowledge_public_risk = true` suppresses the bind error and allows a public
server WITHOUT auth. The user sees a log warning but no auth is enforced.

#### Implementation Steps

1. When `acknowledge_public_risk = true` AND `api_auth.enabled = false`:
   - Log a WARNING: "Public risk acknowledged but auth is NOT enabled. All routes accessible without authentication."
   - Print a prominent banner to stderr (not just tracing):
     ```
     ======================================
     WARNING: NO AUTHENTICATION ENABLED
     Server is publicly accessible at {addr}
     Set [serve.auth] enabled = true for security
     ======================================
     ```
   - Allow the bind (current behavior) -- user explicitly opted in.
2. When binding to non-localhost AND neither `auth.enabled` nor `acknowledge_public_risk`:
   - Bail with the existing error message (current behavior, correct).
3. When `auth.enabled = true`:
   - Allow bind regardless of `acknowledge_public_risk` (current behavior, correct).

#### Design Guidance

The current behavior is actually defensible -- it requires explicit opt-in to run without
auth. The fix is about making the consequences MORE visible, not changing the logic. The
warning should be impossible to miss (stderr banner, not just log line).

#### Verification Criteria

- [ ] `acknowledge_public_risk = true` with no auth shows prominent WARNING banner
- [ ] Without either flag, public bind fails with error
- [ ] With `auth.enabled = true`, bind succeeds normally
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.06: Remove `unsafe set_var` for --provider
**Priority**: P0
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs` (line 236)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs` (`resolve_effective_model`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (lines 2225, 2229 -- other `unsafe set_var` calls)
**Depends On**: none

#### Context

Line 236 of `util.rs`:
```rust
unsafe { std::env::set_var("ROKO_PROVIDER", p) };
```
This is undefined behavior in multi-threaded Rust programs since 1.66. The tokio runtime
is already spawned at this point, making this unsafe.

Two additional `unsafe set_var` calls exist in `main.rs`:
- Line 2225: `unsafe { std::env::set_var("ROKO_HIGH_CONTRAST", "1") };`
- Line 2229: `unsafe { std::env::set_var("ROKO_REDUCED_MOTION", "1") };`

#### Implementation Steps

1. In `util.rs`, remove the `unsafe { std::env::set_var("ROKO_PROVIDER", p) }` call.
2. Add a `provider_override: Option<String>` field to the relevant config/context struct
   that is threaded through dispatch. This could be on `RunConfig`, `DispatchContext`, or
   a new `CliOverrides` struct.
3. In `resolve_effective_model()` in `model_selection.rs`, add a parameter for provider
   override (or check the config struct) before falling back to env var detection.
4. For the `main.rs` accessibility env vars (HIGH_CONTRAST, REDUCED_MOTION): these are
   set before the tokio runtime starts. If they are set in `main()` before `#[tokio::main]`,
   they are safe. If set after runtime start, move them before runtime init or use a config
   field instead.

#### Design Guidance

Use a `CliOverrides` struct that carries CLI-level overrides through the call chain:
```rust
pub struct CliOverrides {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
}
```
Thread this through `cmd_run()`, `cmd_chat()`, etc. as a parameter. This is more
explicit and safe than environment variables.

#### Verification Criteria

- [ ] `grep -rn 'unsafe.*set_var\|set_var.*unsafe' crates/roko-cli/src/` returns zero matches
      (or only matches before tokio runtime init)
- [ ] `roko run --provider cerebras "hello"` uses Cerebras without env var mutation
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.07: Fix stub gate verdicts giving false PASS
**Priority**: P0
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/verdict.rs` (add `skip` constructor)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/rung_dispatch.rs` (lines 132-239, `stub_verdict` + 8 callers)
**Depends On**: none

#### Context

`stub_verdict()` at line 132 of `rung_dispatch.rs`:
```rust
fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
    let message = format!("stub gate; {}", detail.into());
    let mut verdict = Verdict::pass(gate.to_string());
    // ... sets detail
}
```
This returns a PASSING verdict when a gate cannot run (no oracle, no manifest, etc.).
Called from 8 locations (lines 146, 149, 173, 186, 201, 204, 220, 223, 237).

The `Verdict` struct in `verdict.rs` has fields: `passed`, `reason`, `gate`, `score`,
`detail`, `test_count`, `error_digest`, `duration_ms`. No `skipped` field exists.

#### Implementation Steps

1. Add a `skipped` field to `Verdict` in `verdict.rs`:
   ```rust
   pub struct Verdict {
       pub passed: bool,
       /// Whether the gate was skipped (not executed, not failed).
       #[serde(default)]
       pub skipped: bool,
       pub reason: String,
       // ... existing fields
   }
   ```
2. Add a `skip` constructor:
   ```rust
   pub fn skip(gate: impl Into<String>, reason: impl Into<String>) -> Self {
       Self {
           passed: false,
           skipped: true,
           reason: reason.into(),
           gate: gate.into(),
           score: 0.0,
           detail: None,
           test_count: None,
           error_digest: None,
           duration_ms: 0,
       }
   }
   ```
3. Update `stub_verdict()` in `rung_dispatch.rs` to use `Verdict::skip()`:
   ```rust
   fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
       let message = format!("stub gate; {}", detail.into());
       Verdict::skip(gate, &message).with_detail(message)
   }
   ```
4. Ensure `Default` for `skipped` is `false` (`#[serde(default)]`) for backward compat.
5. Update any callers that check `verdict.passed` to also consider `verdict.skipped`:
   - Callers checking "did gate pass?" should check `verdict.passed && !verdict.skipped`
     (or just `verdict.passed` since skip sets `passed = false`).
   - TUI display should show "SKIP" instead of "PASS" or "FAIL" for skipped verdicts.
6. Update episode recording to distinguish pass/fail/skip.

#### Design Guidance

The `skipped` flag allows three states: passed, failed, skipped. This is cleaner than
overloading `passed` because downstream consumers (TUI, episodes, learning) need to
distinguish "gate ran and passed" from "gate was not executed."

#### Verification Criteria

- [ ] `stub_verdict()` returns `Verdict { passed: false, skipped: true, ... }`
- [ ] Existing `Verdict::pass()` returns `Verdict { passed: true, skipped: false, ... }`
- [ ] `Verdict::fail()` returns `Verdict { passed: false, skipped: false, ... }`
- [ ] TUI shows "SKIP" for stub verdicts, not "PASS"
- [ ] Serialization/deserialization handles `skipped` field with default=false
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.08: Fix dual episode writes in `roko run`
**Priority**: P0
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs` (lines 1301-1305, `append_episode_log` call)
**Depends On**: none

#### Context

In `run.rs`, at line 1301, there is a direct `append_episode_log()` call. Then at line 2680,
`runtime.record_completed_run(completed)` also writes episodes through `LearningRuntime`.
Both are behind `#[cfg(feature = "legacy-orchestrate")]`.

The dual write produces duplicate records in different files (`.roko/episodes.jsonl` at root
vs `.roko/learn/episodes.jsonl` under the learn directory).

#### Implementation Steps

1. Remove the direct `append_episode_log()` call at line 1301 (keep the `record_completed_run`
   call which goes through `LearningRuntime`).
2. Verify that `LearningRuntime::record_completed_run()` writes to the canonical
   `.roko/learn/episodes.jsonl` path.
3. Check all episode readers (`roko status`, `roko learn episodes`, TUI episodes tab) to
   ensure they read from the learn path, not the root path.
4. If backward compatibility is needed, add a one-time migration that moves entries from
   the root path to the learn path.
5. Since both calls are behind `#[cfg(feature = "legacy-orchestrate")]`, verify if the
   non-legacy V2 path also has dual writes. If not, this fix is only needed for the
   legacy feature flag.

#### Design Guidance

Single writer principle: `LearningRuntime` should be the sole episode writer across all
paths. Any other episode write should go through it.

#### Verification Criteria

- [ ] `roko run "hello"` produces exactly one new episode entry (not two)
- [ ] Episode entry appears in `.roko/learn/episodes.jsonl` (canonical path)
- [ ] `roko learn episodes` reads from the correct path
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.09: Normalize `[[gate]]` vs `[gates]` config schema
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` (`RokoConfig::from_toml`, `GatesConfig`)
**Depends On**: none

#### Context

`RokoConfig` in `schema.rs` at line 64 has `pub gates: GatesConfig`. The `from_toml()` at
line 171 calls `toml::from_str()` which deserializes `[gates]` table format. The `[[gate]]`
array format (TOML array of tables) is a different key name and is silently ignored by serde.

`roko init` (in `commands/init.rs` at line 130) writes `[[gate]]` format:
```toml
[[gate]]
kind = "shell"
program = "cargo"
```

This format is never read by the runtime.

#### Implementation Steps

1. In `schema.rs`, add an `extra_gates` field to `RokoConfig`:
   ```rust
   #[serde(default, rename = "gate")]
   pub extra_gates: Vec<LegacyGateEntry>,
   ```
   where `LegacyGateEntry` captures the `[[gate]]` array format.
2. In `from_toml()`, after parsing, merge `extra_gates` into `gates`:
   ```rust
   pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
       let mut config: Self = toml::from_str(s)?;
       if !config.extra_gates.is_empty() {
           if config.gates.enabled.is_empty() {
               // Convert legacy format to new format
               for gate in &config.extra_gates {
                   config.gates.enabled.push(gate.kind.clone());
                   if gate.kind == "shell" {
                       config.gates.shell_gates.push(ShellGateCommand { ... });
                   }
               }
           } else {
               tracing::warn!("Both [[gate]] and [gates] found; preferring [gates]");
           }
       }
       Ok(config)
   }
   ```
3. Add a unit test: parse a TOML string with `[[gate]]` entries, verify they appear in
   `config.gates.enabled`.
4. Add a unit test: parse a TOML string with both formats, verify `[gates]` is preferred
   and a warning is emitted.
5. Add a unit test: parse with only `[gates]`, verify normal behavior unchanged.

#### Design Guidance

Use serde's `rename` attribute to map `[[gate]]` to a separate field, then merge in
`from_toml()`. This avoids complex custom deserializer logic. The merge is a one-time
normalization at load time.

#### Verification Criteria

- [ ] `RokoConfig::from_toml("[[gate]]\nkind = \"shell\"\nprogram = \"cargo\"\n")` produces a config with `gates.enabled` containing the gate
- [ ] Existing `[gates]` format continues to work
- [ ] When both are present, `[gates]` wins with a warning
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.10: Fix `roko init` emitting wrong gate format
**Priority**: P0
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/init.rs` (lines 118-131, `append_shell_gate`)
**Depends On**: Task 1.09

#### Context

`append_shell_gate()` at line 129 writes:
```rust
out.push_str("\n[[gate]]\n");
out.push_str("kind = \"shell\"\n");
```
This writes the `[[gate]]` array format. The runtime expects `[gates]` table format.

#### Implementation Steps

1. Replace `append_shell_gate()` with a function that writes `[gates]` format:
   ```toml
   [gates]
   enabled = ["compile", "clippy", "test"]

   [[gates.shell]]
   program = "cargo"
   args = ["check", "--workspace"]
   timeout_ms = 120000
   ```
2. For the "no profile" case (line 121), update the comment to reference `[gates]`:
   ```
   # Add [gates] section to configure validation gates.
   ```
3. Update init tests to verify the new format.
4. Verify that `RokoConfig::from_toml()` can parse the generated output.

#### Design Guidance

The init template should generate the simplest valid config. For most Rust projects:
```toml
[gates]
enabled = ["compile", "clippy", "test"]
```
Shell gates can be added as a commented-out example.

#### Verification Criteria

- [ ] `roko init --profile rust` generates `[gates]` format, not `[[gate]]`
- [ ] Generated `roko.toml` passes `RokoConfig::from_toml()`
- [ ] `roko plan run` respects gates from init-generated config
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.11: Wire CascadeRouter to live callers
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs` (`resolve_effective_model`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: none

#### Context

`resolve_effective_model()` in `model_selection.rs` at line 140 accepts `Option<&CascadeRouter>`
as its 4th parameter. Every live caller passes `None` (verified by grep: no non-None calls
outside test code). The CascadeRouter is a LinUCB contextual bandit with 4-stage routing,
persistence at `.roko/learn/cascade-router.json`, and cost spike detection.

#### Implementation Steps

1. Create a helper function `load_or_create_cascade_router(roko_dir: &Path) -> CascadeRouter`:
   - Try to load from `.roko/learn/cascade-router.json`
   - If file doesn't exist or is corrupt, create a new router with default config
   - Log the loaded state (number of observations)
2. In `run.rs` (`cmd_run` or equivalent):
   - Load router at startup: `let router = load_or_create_cascade_router(&roko_dir);`
   - Pass `Some(&router)` to `resolve_effective_model()`
   - After each model call completes, call `router.observe(model, role, success, cost, latency)`
   - Persist router on graceful shutdown
3. In `chat_session.rs`:
   - Load router in session setup
   - Pass to model resolution
   - Observe after each turn
4. In `runner/event_loop.rs`:
   - Load router before entering the event loop
   - Pass to dispatch context
   - Observe after each task completion
   - Persist during periodic flush (every 5 tasks or 60 seconds)
5. Add the router to `ServiceFactory::build()` return type so it can be shared.

#### Design Guidance

The router should be a shared `Arc<Mutex<CascadeRouter>>` to allow observation from multiple
async tasks. Persistence should use atomic file writes (write to `.tmp`, rename) to avoid
corruption on crash. The periodic flush interval should match the existing executor state
flush.

#### Verification Criteria

- [ ] `roko run "hello"` loads/creates cascade router
- [ ] After 2+ runs, `.roko/learn/cascade-router.json` has `observations > 0`
- [ ] Router observations include model name, success/failure, cost
- [ ] `roko plan run` also records observations
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.12: Wire feedback recording to `roko chat`
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/feedback_service.rs`
**Depends On**: none

#### Context

`chat_session.rs` makes model calls but records no episodes, no routing observations, no
cost tracking. Grep for `FeedbackService` in `chat_session.rs` returns zero matches.
Every chat session is a lost learning opportunity.

#### Implementation Steps

1. In chat session setup, instantiate `FeedbackService`:
   ```rust
   let feedback = FeedbackService::from_roko_dir_with_episodes(&roko_dir)?;
   ```
2. After each model response, emit a feedback event:
   ```rust
   feedback.record_model_call(ModelCallRecord {
       model: model_name.clone(),
       input_tokens, output_tokens,
       latency_ms, success: true,
       provider: provider_name.clone(),
   })?;
   ```
3. On session end (`/quit` or Ctrl-D), emit a session completion event:
   ```rust
   feedback.record_session_complete(SessionRecord {
       session_id, total_turns, total_cost,
       total_input_tokens, total_output_tokens,
   })?;
   ```
4. Optionally: attach CascadeRouter to observe model performance from chat.

#### Design Guidance

Keep feedback recording lightweight -- it should not add perceptible latency to chat. Use
async write-behind if needed. The feedback sink should be the same one used by `roko run`
for consistency.

#### Verification Criteria

- [ ] Start `roko chat`, send one message, quit
- [ ] `.roko/learn/efficiency.jsonl` has a new entry with non-zero tokens
- [ ] `.roko/episodes.jsonl` (or learn equivalent) has a new entry
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.13: Wire feedback recording to ACP pipeline
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs` (if exists)
**Depends On**: none

#### Context

ACP records only adaptive gate thresholds for rungs 0/1/2. No episodes, no routing
observations, no cost tracking. Editor-integrated usage (VS Code, etc.) is likely the
highest-frequency interaction but produces zero learning signal.

#### Implementation Steps

1. In ACP pipeline initialization, create `FeedbackService`:
   ```rust
   let feedback = FeedbackService::from_roko_dir_with_episodes(&roko_dir)?;
   ```
2. Thread `feedback` through the ACP runner to all model dispatch points.
3. After each model call in `runner.rs`, emit `FeedbackEvent::ModelCall`.
4. After gate execution, emit `FeedbackEvent::GateResult` (not just threshold updates).
5. On session completion, emit `FeedbackEvent::SessionComplete`.
6. Ensure the feedback service is flushed on session end (not just buffered).

#### Design Guidance

ACP sessions can be long-lived (hours in an editor). Use periodic flush (every 5 turns or
30 seconds) rather than waiting for session end. The feedback service instance should be
shared across the session lifetime.

#### Verification Criteria

- [ ] Run an ACP session, dispatch a model call, run gates
- [ ] `.roko/episodes.jsonl` has a new entry with source="acp"
- [ ] `.roko/learn/cascade-router.json` observation count increases
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.14: Forward streaming events to chat TUI
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs`
**Depends On**: none

#### Context

The audit mentioned `while let Some(_event) = event_rx.recv().await {}` draining events.
Grep for this exact pattern returns zero matches, suggesting the code may have been
refactored since the audit. The streaming infrastructure may already work.

#### Implementation Steps

1. Search `chat_inline.rs` for the event receive loop. Look for patterns like:
   - `while let Some(event) = rx.recv().await`
   - `event_rx.recv()`
   - Any channel receiver that processes agent events
2. If events are still being drained without processing:
   - Map `AgentStreamEvent::Text(text)` -> append to ratatui viewport buffer
   - Map `AgentStreamEvent::ToolCall { name, args }` -> show tool name in status bar
   - Map `AgentStreamEvent::Complete { usage }` -> show cost/token stats
3. If the event processing is already working:
   - Verify that streaming text appears character-by-character during response
   - Mark this task as resolved with a note about the audit being stale
4. Ensure the viewport auto-scrolls as new text arrives.
5. Test with a real agent call to confirm streaming works.

#### Design Guidance

The ratatui viewport should use a ring buffer for streaming text to avoid unbounded memory
growth. A reasonable cap is 64KB of visible text, with older content scrollable. Tool call
display should be ephemeral (shown for 2 seconds in the status bar, then hidden).

#### Verification Criteria

- [ ] `roko chat` displays streaming text progressively during agent response
- [ ] Tool calls are visible during execution (name at minimum)
- [ ] Cost/token stats appear after response completes
- [ ] No spinner-only period followed by text dump
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.15: Wire `build_repo_context` into plan generate
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/prd.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/repo_context.rs` (`build_repo_context`)
**Depends On**: none

#### Context

`build_repo_context()` at line 282 of `repo_context.rs` accepts a workdir and feature keywords,
returns a `RepoContextPack` with workspace map, crate structure, existing implementations.
It is called from:
- `prd.rs` line 877 (PRD draft generation)
- `commands/prd.rs` line 383 (PRD draft new)

It is NOT called from `commands/plan.rs` for plan generate, plan regenerate, or prd plan.

#### Implementation Steps

1. In `plan.rs`, in the `plan generate` handler:
   - Extract task keywords from the plan prompt or PRD content
   - Call `build_repo_context(&workdir, &keywords).await`
   - Inject the context pack into the agent prompt as a "Repository Structure" section
2. In `plan.rs`, in the `plan regenerate` handler: same treatment.
3. In `prd.rs` or `plan.rs`, in the `prd plan` handler: same treatment.
4. The context injection should use the `RepoContextPack::to_prompt_section()` method
   (or format it inline if no such method exists).

#### Design Guidance

The repo context should be positioned early in the system prompt (before task-specific
instructions) so the agent has structural awareness when generating plans. Keep the keyword
extraction simple: split the prompt on whitespace, filter stopwords, take the top 5 by
tf-idf or frequency.

#### Verification Criteria

- [ ] `roko plan generate` on a workspace with 18 crates includes crate names in the agent prompt
- [ ] Generated plan references existing crate names
- [ ] `roko prd plan` also includes repo context
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.16: Inject validation diagnostics into plan regenerate
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`
**Depends On**: none

#### Context

`plan regenerate` validates after generation but does NOT inject diagnostics into the
regeneration prompt. This is HOLLOW-3 from the audit. The validation-feedback loop is
missing.

#### Implementation Steps

1. After initial agent generation, validate the output using existing validation logic.
2. If validation fails:
   ```rust
   let mut retry_count = 0;
   let max_retries = 2;
   loop {
       let validation = validate_plan(&generated_output)?;
       if validation.is_ok() || retry_count >= max_retries {
           break;
       }
       let error_prompt = format!(
           "The generated plan has the following validation errors:\n{}\n\n\
            Fix these errors in the plan. Here is the original plan:\n{}",
           validation.errors_formatted(),
           generated_output
       );
       generated_output = run_agent_with_prompt(&error_prompt).await?;
       retry_count += 1;
   }
   ```
3. If still failing after retries, output the plan with warnings:
   ```
   WARNING: Plan has {n} validation errors after {max_retries} fix attempts:
   {errors}
   ```
4. Log the retry attempts in the episode for debugging.

#### Design Guidance

The retry prompt should be concise -- include only the specific validation errors, not the
entire plan context. This keeps token cost proportional to the error, not the plan size.
Consider adding a `--no-fix` flag that skips the retry loop for users who want raw output.

#### Verification Criteria

- [ ] Plan with a file reference error triggers a fix attempt
- [ ] After fix, the plan is re-validated
- [ ] After 2 failed fix attempts, plan is output with warnings
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.17: Wire BudgetGuardrail to live paths
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/budget.rs` (`BudgetGuardrail`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: none

#### Context

`BudgetGuardrail` in `budget.rs` implements 3-scope budget limits (per-task, per-session,
per-day) with 5 graduated actions (Ok, Warn, RouteToCheaper, BlockNewSessions, Block). It
is only referenced from `orchestrate.rs` (behind legacy feature flag) and `task_runner.rs`
in `roko-agent`. Zero live callers in runner v2 or `roko run`.

#### Implementation Steps

1. Load budget config from `roko.toml` (`[budget]` section).
2. Instantiate `BudgetGuardrail` at session start in:
   - `roko run`: before model dispatch in `run.rs`
   - `roko chat`: in session setup in `chat_session.rs`
   - `roko plan run`: before event loop in `event_loop.rs`
3. Before each model dispatch, check budget:
   ```rust
   match guardrail.check(estimated_cost) {
       BudgetAction::Ok => { /* proceed */ }
       BudgetAction::Warn(msg) => { tracing::warn!("{}", msg); /* proceed */ }
       BudgetAction::RouteToCheaper => { /* switch to fallback model */ }
       BudgetAction::Block(msg) => { return Err(anyhow!("Budget exceeded: {}", msg)); }
       // ...
   }
   ```
4. After each model call, update the guardrail with actual cost:
   ```rust
   guardrail.record_spend(actual_cost);
   ```
5. Set sensible defaults: per-turn $0.50, per-session $10.00, per-plan $100.00.

#### Design Guidance

The guardrail should be optional -- users who don't configure `[budget]` should not be
blocked. The `Block` action should produce a clear error message showing cumulative spend
and the configured limit. Consider adding a `--budget-override` CLI flag for one-off
increases.

#### Verification Criteria

- [ ] Set `budget.max_session_usd = 0.01` in roko.toml
- [ ] `roko run "write a long essay"` stops with budget exceeded message
- [ ] Without budget config, no blocking occurs
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.18: Wire ContextTier into dispatch for small models
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_provider.rs` (`ContextTier`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: none

#### Context

`ContextTier` in `context_provider.rs` defines correct budgets (Surgical: 4K, Focused: 12K,
Full: 24K tokens) and `is_local_model()` correctly identifies small models. But the dispatch
path never calls `ContextTier::from_task_and_model()`. Small models (Ollama gemma4, Cerebras
llama 8b) receive 200K-context prompts, causing silent truncation.

#### Implementation Steps

1. In the dispatch path (wherever prompt is assembled before agent call), resolve context tier:
   ```rust
   let tier = ContextTier::from_model(model_slug);
   let budget = tier.token_budget();
   ```
2. Pass budget to `PromptAssemblyService::with_token_budget(budget)`.
3. In `PromptAssemblyService`, enforce the budget:
   - Surgical (4K): identity + role + task + constraints only
   - Focused (12K): add conventions and limited context
   - Full (24K+): include all sections
4. Log the selected tier: `tracing::info!(tier = ?tier, budget, "context tier selected for {model}")`.

#### Design Guidance

The tier should be derived from the model's context window, not hardcoded per model name.
Add a `context_window` field to `ModelProfile` in config, and derive tier from that. This
makes new models automatically get the right tier.

#### Verification Criteria

- [ ] With `default_model = "ollama/gemma4"`, assembled prompt is under 4K tokens
- [ ] With `default_model = "claude-sonnet-4"`, assembled prompt uses full budget
- [ ] System log shows tier selection
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.19: Wire BudgetPredictor to prompt assembly
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/budget_predictor.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs`
**Depends On**: Task 1.18

#### Context

`BudgetPredictor` in `budget_predictor.rs` is 679 LOC with EMA-based prediction, failure
inflation, partial-match fallback, and persistence. No caller invokes `predictor.predict()`.
Token budgets are static constants.

#### Implementation Steps

1. Load `BudgetPredictor` from `.roko/learn/budget-predictions.json` at startup (or create new).
2. Before prompt assembly, call `predictor.predict(role, task_id)` to get predicted budget.
3. Use predicted budget as input to `PromptAssemblyService::with_token_budget()` (capped
   by the ContextTier from Task 1.18).
4. After task completion, call `predictor.observe(role, task_id, actual_tokens, success)`.
5. Persist predictor state during periodic flush.

#### Design Guidance

The predictor should override static budgets only when it has sufficient observations
(>= 5 for the role/task combination). Below that threshold, use the ContextTier default.
This prevents cold-start issues where the predictor has no data.

#### Verification Criteria

- [ ] Run the same task type 5 times
- [ ] By run 5, predicted budget converges toward actual usage (within 20%)
- [ ] `.roko/learn/budget-predictions.json` has entries after runs
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.20: Wire `roko chat` and dispatch_direct through PromptAssemblyService
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_direct.rs` (if still active)
**Depends On**: none

#### Context

`roko chat` sends bare prompts to the agent with zero system prompt. No role identity,
no conventions, no knowledge injection. The agent has no context about the project it is
working in.

#### Implementation Steps

1. In `chat_session.rs`, before sending to agent:
   - Create a `PromptAssemblyService` with lightweight config (skip heavy PRD context)
   - Call `assemble(role="assistant", prompt=user_input)`
   - Pass the assembled system prompt via `--append-system-prompt` to Claude CLI
2. Use a lightweight assembly profile:
   - Include: identity, role, project name, crate structure, conventions
   - Exclude: PRD context, research context, full knowledge dump
   - Budget: 2K tokens for system prompt (keep chat snappy)
3. Cache the assembled system prompt across turns (it doesn't change per-turn).
4. In `dispatch_direct.rs`: apply same treatment if this path is still reachable.

#### Design Guidance

The system prompt for chat should be cached and reused across turns since it doesn't change.
Only regenerate it when the model changes (e.g., user runs `/model <new-model>`). This keeps
chat latency low.

#### Verification Criteria

- [ ] `roko chat` -- type "what project am I working on?" -- agent knows the project name
- [ ] System prompt is under 2K tokens
- [ ] System prompt is cached across turns
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.21: Consolidate 4 stream-json parsing copies
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/translate/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_direct.rs` (if still active)
**Depends On**: none

#### Context

The stream-json parsing logic is duplicated 4 times with inconsistent output formats. All
copies independently implement 4096-byte truncation with char_boundary checks. The canonical
parser `parse_stream_line()` exists in the provider module.

#### Implementation Steps

1. Identify the canonical `parse_stream_line()` function location.
2. Replace inline parsing in `translate/mod.rs:extract_text()` with calls to the canonical parser.
3. Replace inline parsing in `translate/mod.rs:extract_tool_outputs()` similarly.
4. Replace inline parsing in `chat.rs:extract_clean_text()` similarly.
5. Leave `dispatch_direct.rs` as-is (behind `legacy-orchestrate` feature gate, will be removed).
6. Add tests verifying all replaced paths produce identical output to the canonical parser.

#### Design Guidance

The canonical parser should be in `roko-agent` (since it parses agent stream output) and
exported publicly. All consumers in `roko-cli` should import from `roko-agent`. If the
canonical parser is in `roko-cli`, consider moving it to `roko-agent` for the right
dependency direction.

#### Verification Criteria

- [ ] `grep -rn 'serde_json::from_str.*result' crates/roko-cli/src/chat.rs crates/roko-agent/src/translate/mod.rs` returns zero matches (all delegated)
- [ ] Tests verify identical output from canonical parser and removed inline parsers
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.22: Wire runner v2 CascadeRouter observations
**Priority**: P1
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: Task 1.11

#### Context

Runner v2 imports CascadeRouter types but never calls `cascade_router.observe()` after task
completion. Grep for `CascadeRouter.*observe` in the runner directory returns zero matches.

#### Implementation Steps

1. After task completion (success or failure) in `event_loop.rs`, construct a routing observation:
   ```rust
   router.observe(UsageObservation {
       model: task_result.model.clone(),
       role: task_result.role.clone(),
       success: task_result.gate_passed,
       cost: task_result.cost,
       latency_ms: task_result.duration_ms,
   });
   ```
2. Persist router state during the periodic flush (reuse the existing flush interval).

#### Design Guidance

The observation should happen synchronously in the event loop -- it is a cheap in-memory
update. Persistence can be batched with other flush operations.

#### Verification Criteria

- [ ] `roko plan run` on a 3-task plan produces `observations >= 3` in cascade-router.json
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.23: Wire runner v2 AdaptiveThreshold observations
**Priority**: P1
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/adaptive_threshold.rs`
**Depends On**: none

#### Context

Runner v2 does not call `AdaptiveThresholds::observe()` after gate execution. The adaptive
threshold system (`adaptive_threshold.rs`) includes SPC monitoring (CUSUM, EWMA, BOCPD) but
receives no observations from the default execution path.

#### Implementation Steps

1. After each gate verdict in `event_loop.rs`:
   ```rust
   thresholds.observe(rung, verdict.passed);
   ```
2. Before each gate dispatch, check adaptive skip:
   ```rust
   if thresholds.should_skip_rung(rung) {
       tracing::info!(rung, "adaptive skip: rung has high pass rate");
       continue; // skip this gate
   }
   ```
3. Record skip decisions in the episode for debugging.
4. Persist thresholds during periodic flush.

#### Design Guidance

Adaptive skipping should be conservative -- only skip when pass rate is > 0.99 over 50+
observations. This prevents skipping gates that occasionally catch issues.

#### Verification Criteria

- [ ] `roko plan run` with gates produces `.roko/learn/gate-thresholds.json` with `total_observations > 0`
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.24: Wire runner v2 episode logging
**Priority**: P1
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: none

#### Context

Runner v2 does not write episodes on task completion. Episodes are the primary learning
signal consumed by `PromptAssemblyService`, error similarity matching, and `roko learn`.

#### Implementation Steps

1. On task completion in `event_loop.rs`, construct an `Episode`:
   ```rust
   let episode = Episode {
       task_id: task_id.clone(),
       model: model_name.clone(),
       success: gate_passed,
       input_tokens, output_tokens,
       gate_verdicts: verdicts.clone(),
       duration_ms,
       timestamp: Utc::now(),
       // ...
   };
   ```
2. Write via `EpisodeSink` or `LearningRuntime::record_completed_run()`.
3. Include gate results, token counts, cost, and timing.
4. Flush immediately after write.

#### Design Guidance

Use `LearningRuntime::record_completed_run()` as the single writer (consistent with Task 1.08).

#### Verification Criteria

- [ ] `roko plan run` on a 3-task plan produces 3 new entries in `.roko/learn/episodes.jsonl`
- [ ] Each entry has model, success, tokens, gate verdicts
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.25: Wire runner v2 section effectiveness updates
**Priority**: P1
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/section_effect.rs`
**Depends On**: none

#### Context

`SectionEffectivenessRegistry` tracks lift per prompt section but receives no observations
from runner v2 plan execution.

#### Implementation Steps

1. On task completion, call:
   ```rust
   section_registry.observe(sections_included, gate_passed);
   ```
2. `sections_included` can be derived from the `PromptAssemblyService` output.
3. Persist registry during periodic flush.

#### Design Guidance

Section names should match across assemblies so observations accumulate correctly.
Use the canonical section names from `PromptAssemblyService`.

#### Verification Criteria

- [ ] After plan run, `.roko/learn/section-effects.json` has entries with `observation_count > 0`
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.26: Wire runner v2 efficiency event recording
**Priority**: P1
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: none

#### Context

Efficiency events with 30+ fields are not emitted from runner v2. This breaks `roko learn efficiency`.

#### Implementation Steps

1. After each agent completes in the event loop, construct an `AgentEfficiencyEvent`.
2. Write to `.roko/learn/efficiency.jsonl` via the efficiency sink.
3. Flush immediately after write (avoid the dogfood bug of accumulating without flush).

#### Design Guidance

Include at minimum: model, role, task_id, input_tokens, output_tokens, tool_calls_count,
duration_ms, success, cost_usd.

#### Verification Criteria

- [ ] `roko plan run` on a 3-task plan produces 3 entries in `.roko/learn/efficiency.jsonl`
- [ ] Each entry has non-zero token counts
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.27: Wire section effectiveness into PromptAssemblyService
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/section_effect.rs`
**Depends On**: Task 1.25

#### Context

`PromptAssemblyService` already has a `section_weights` concept. Section effectiveness data
is collected (after Task 1.25) but not read back during assembly.

#### Implementation Steps

1. On construction, load section effectiveness from `.roko/learn/section-effects.json`.
2. Apply weights during section budget allocation:
   - Sections with score < 0.1: exclude entirely
   - Sections with score 0.1-0.5: reduce budget proportionally
   - Sections with score > 0.5: full budget
3. Log when a section is deprioritized due to negative effectiveness.

#### Design Guidance

Use a minimum observation threshold (e.g., 10) before applying effectiveness scores. Below
that threshold, use equal weights (all sections get full budget). This prevents premature
optimization from small sample sizes.

#### Verification Criteria

- [ ] After 10+ runs with section effectiveness data, low-lift sections get less budget
- [ ] High-lift sections get full budget
- [ ] Sections with < 10 observations use default weights
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.28: Wire gate failure classification to retry/replan routing
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/compile_errors.rs` (`classify_gate_error`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: none

#### Context

`classify_gate_error` in `compile_errors.rs` computes failure actions (Retry, NeedsReplan,
Blocked, NeedsHuman) but the action is rendered as a string and discarded. The orchestrator
always retries regardless of classification.

#### Implementation Steps

1. After gate failure, call `classify_gate_failure(&output)` to get the recommended action.
2. Route based on the action:
   - `Retry`: continue with existing retry logic (feedback to agent)
   - `NeedsReplan`: emit replan event, construct a strategist prompt with the errors
   - `Blocked`: pause the task, mark as blocked, log the reason
   - `NeedsHuman`: pause the task, emit notification, set status to "needs-human"
3. Expose the classification in the episode record.
4. For runner v2: implement at least `Retry` and `NeedsReplan` actions.

#### Design Guidance

The classification should be transparent -- log the classification result so users understand
why the system chose to retry vs. replan vs. block.

#### Verification Criteria

- [ ] Gate failure classified as `Retry` triggers normal retry with feedback
- [ ] Gate failure classified as `NeedsReplan` triggers a strategist agent call
- [ ] Classification appears in episode record
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.29: Replace ACP direct subprocess spawns with provider system
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
**Depends On**: none

#### Context

Two ACP paths bypass the provider system: `run_claude_cli()` spawns a bare subprocess with
no model flag, no streaming, no system prompt, no feedback. `run_claude_cognitive_task()`
builds its own subprocess.

#### Implementation Steps

1. Replace `run_claude_cli()` calls with `create_agent_for_model()` via the provider adapter.
2. Replace `run_claude_cognitive_task()` similarly.
3. Replace `run_openai_compat_cognitive_task()` with provider adapter calls.
4. Pass model, system prompt, and feedback service through the provider adapter.
5. Cost tracking and credential management now happen automatically via the adapter.

#### Design Guidance

All model calls should go through the provider adapter system. This ensures consistent
credential management, cost tracking, rate limiting, and circuit breaking.

#### Verification Criteria

- [ ] ACP model calls appear in `.roko/learn/efficiency.jsonl`
- [ ] Cost tracking shows non-zero values for ACP sessions
- [ ] `grep -rn 'run_claude_cli\|run_claude_cognitive' crates/roko-acp/` returns zero matches
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.30: Fix ACP gate rung ordering (clippy before test)
**Priority**: P1
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` (`run_gates()`)
**Depends On**: none

#### Context

ACP runs gates in order: compile -> test -> clippy. Canonical order: compile (0) -> clippy
(1) -> test (2). Running test before clippy wastes 5-15 minutes when a trivial lint exists.

#### Implementation Steps

1. Reorder the hardcoded gate list to: compile, clippy, test.
2. Add short-circuit: if clippy fails, skip test (return early with clippy failure).
3. Alternatively: replace the hardcoded list with a call to `GateService` which orders by
   rung index.

#### Design Guidance

Prefer using `GateService` for ordering rather than hardcoding. This ensures ACP gates
stay consistent with other paths.

#### Verification Criteria

- [ ] ACP gate execution runs in order: compile, clippy, test
- [ ] If clippy fails, test is skipped
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.31: Wire gate feedback_for_agent into GateService
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/feedback.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: none

#### Context

`feedback_for_agent()` at line 202 of `feedback.rs` is exported from `roko-gate` (line 156
of `lib.rs`) but called only from `orchestrate.rs` (dead code in V2 paths). The V2 paths
run gates but dump raw stderr into the retry context.

#### Implementation Steps

1. In `GateService`, after running gates, call `feedback_for_agent()` on any failed verdicts.
2. Add a `feedback: Option<GateFeedback>` field to the gate result/report struct.
3. When the pipeline state machine handles `GatesFailed`, extract feedback and inject into
   the retry prompt (replacing raw stderr).

#### Design Guidance

Structured feedback (file, line, error message, suggestion) is far more valuable to the
retry agent than raw build output. The feedback should be under 1K tokens even for large
build failures.

#### Verification Criteria

- [ ] Task that fails compile gets structured feedback in retry prompt
- [ ] Feedback includes specific errors (not raw stderr)
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.32: Fix model showing "-" in TUI for runner v2
**Priority**: P1
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: none

#### Context

Runner v2 passes empty string for model in TUI events. The dashboard shows "-" instead of
the model name.

#### Implementation Steps

1. When dispatching an agent, include the resolved model name in the dispatch event.
2. When the agent responds with usage, include model in the progress event.
3. Populate from the dispatch context (model selection result), not from agent output.

#### Design Guidance

The model name should be set at dispatch time (before agent starts) and carried through
all events for that task. Do not rely on agent output parsing for the model name.

#### Verification Criteria

- [ ] `roko plan run` with TUI shows model name (e.g., "claude-sonnet-4") instead of "-"
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.33: Remove direct env var reads for API keys
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/episode_completion.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/web_search.rs`
**Depends On**: none

#### Context

Two live code paths read API keys directly from environment variables (`ANTHROPIC_API_KEY`,
`PERPLEXITY_API_KEY`) instead of going through provider configuration. Keys are not tracked
in cost accounting.

#### Implementation Steps

1. `episode_completion.rs`: accept a configured `Agent` or `ModelCallService` through
   dependency injection instead of constructing its own HTTP client with env var key.
2. `web_search.rs`: accept a provider config or API key through the tool's configuration
   rather than reading env vars directly.
3. Remove the `std::env::var("ANTHROPIC_API_KEY")` and `std::env::var("PERPLEXITY_API_KEY")` calls.
4. Fall back to the provider config's `api_key_env` pattern for resolution.

#### Design Guidance

API key resolution should always go through the provider configuration system. If a
provider is configured with `api_key_env = "ANTHROPIC_API_KEY"`, the provider system reads
that env var -- individual subsystems should not.

#### Verification Criteria

- [ ] `grep -rn 'env::var.*API_KEY' crates/roko-neuro/ crates/roko-std/` returns zero matches
- [ ] Both subsystems function when key is configured in `roko.toml` providers
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.34: Fix `signals.jsonl` dead path (writes to `engrams.jsonl`)
**Priority**: P1
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-fs/src/file_substrate.rs` (lines 24, 48, 72)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-fs/src/layout.rs` (lines 165-177)
**Depends On**: none

#### Context

`file_substrate.rs` writes to `engrams.jsonl` (line 48: `root.join("engrams.jsonl")`).
`layout.rs` defines both paths:
- Line 168: `engrams_log() -> root.join("engrams.jsonl")` -- the one actually used
- Line 177: `signals_log() -> root.join("signals.jsonl")` -- defined but never populated

The `roko status` command reads signals -- if it reads from `signals.jsonl`, it finds nothing.

#### Implementation Steps

1. Determine the canonical name. `engrams.jsonl` is the historical name from mori; `signals.jsonl`
   is the roko convention.
2. Option A: Change `file_substrate.rs` to write to `signals.jsonl`. Update `layout.rs` to
   remove `engrams_log()` or make it a deprecated alias.
3. Option B: Update all readers to use `engrams_log()` consistently. Deprecate `signals_log()`.
4. Add a migration helper that renames `engrams.jsonl` to `signals.jsonl` on first run.
5. Update `roko status` to read from the correct file.

#### Design Guidance

Prefer option A (use `signals.jsonl`). The roko naming convention should be consistent.
Add a fallback that checks both paths during migration.

#### Verification Criteria

- [ ] `roko run "hello"` writes to the canonical signal log path
- [ ] `roko status` reads from the same path and shows signal count
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.35: Unify model selection paths (auth_detect vs ServiceFactory)
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/auth_detect.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/service_factory.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs`
**Depends On**: none

#### Context

9+ dispatch paths have inconsistent model selection. `auth_detect.rs` scans env vars in
fixed priority, ignoring config. `ServiceFactory::resolve_model()` resolves correctly.
Setting `default_model = "glm51"` in roko.toml has no effect via `roko run`.

#### Implementation Steps

1. Make all entry points use `ServiceFactory::build()` (or its resolve_model function).
2. Demote `auth_detect.rs` to credential discovery only (not model selection).
3. Model resolution priority: CLI override > task config > role config > roko.toml
   `default_model` > env var heuristic.
4. Remove model-selection logic from `auth_detect.rs`.
5. Test all 9 entry points with `default_model` set.

#### Design Guidance

Create a single `resolve_model_for_dispatch(overrides, config) -> ModelSelection` function
that all entry points call. Keep env var scanning as the lowest-priority fallback.

#### Verification Criteria

- [ ] Set `default_model = "cerebras-70b"` in roko.toml
- [ ] `roko run`, `roko plan run`, `roko chat` all use Cerebras
- [ ] CLI `--model` override still works
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.36: Normalize model aliases at load time
**Priority**: P2
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/service_factory.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`
**Depends On**: none

#### Context

`glm-5-1` on provider "zai" vs `glm51` on provider "zhipu" both resolve to `glm-5.1`.
Multiple Claude aliases exist. Duplicate entries confuse routing.

#### Implementation Steps

1. Build an alias table at config load time.
2. Normalize all model slugs to canonical form.
3. Warn on duplicates.
4. CascadeRouter uses canonical slugs.

#### Verification Criteria

- [ ] Config with both `glm51` and `glm-5-1` produces a warning
- [ ] CascadeRouter tracks a single canonical entry
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.37: Export `rung_for_gate_name` from roko-gate
**Priority**: P2
**Estimated Effort**: 30 minutes
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs` (line 645)
**Depends On**: none

#### Context

`rung_for_gate_name()` is defined at line 645 of `effect_driver.rs` as a local function.
This duplicates logic from `roko-gate`. The comment at line 309 references it.

#### Implementation Steps

1. Add or export `pub fn rung_for_gate_name(name: &str) -> u8` from `roko-gate/src/lib.rs`.
2. In `effect_driver.rs`, delete the local `rung_for_gate_name()` function (line 645).
3. Import from `roko_gate::rung_for_gate_name`.

#### Verification Criteria

- [ ] `rung_for_gate_name` defined in one place only (roko-gate)
- [ ] `effect_driver.rs` imports it
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.38: Add TaskScheduler state to WorkflowEngine checkpoint
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/task_scheduler.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
**Depends On**: none

#### Context

WorkflowEngine checkpoints `PipelineStateV2` but not `TaskScheduler`. Resume restarts all
tasks from the beginning.

#### Implementation Steps

1. Add `Serialize, Deserialize` derives to `TaskStatus` enum.
2. Add a `checkpoint()` method to `TaskScheduler`.
3. Include TaskScheduler state in WorkflowEngine checkpoint JSON.
4. On resume, restore TaskScheduler state and skip completed tasks.

#### Verification Criteria

- [ ] Start 5-task plan, kill after task 3, resume -- tasks 1-3 skipped
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.39: Add `thinking_tokens` to UsageObservation
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/usage.rs`
- Provider adapter files in `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/`
**Depends On**: none

#### Context

`UsageObservation` tracks input/output/cache tokens but not thinking/reasoning tokens.
Models with thinking (Claude extended thinking, OpenAI o3/o4-mini) produce internal tokens
that cost money but are invisible.

#### Implementation Steps

1. Add `thinking_tokens: Option<u64>` to `UsageObservation`.
2. Update Claude CLI stream parser to extract reasoning token counts.
3. Update OpenAI-compat parser for `reasoning_tokens` field.
4. Update `CostTable` for thinking-specific pricing.
5. Surface in usage reports.

#### Verification Criteria

- [ ] Run with `--effort high` -- episode shows non-zero `thinking_tokens`
- [ ] Cost accounting includes thinking token costs
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.40: Fix singleton rate limiter across providers
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/openai_compat_backend.rs` (lines 31-35)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/rate_limit.rs`
**Depends On**: none

#### Context

`shared_rate_limiter()` at line 31 uses `OnceLock` to create a single global
`ProviderRateLimiter` with 60 RPM default. All `OpenAiCompatLlmBackend` instances share it.
A provider with 1000 RPM is throttled to 60 RPM.

#### Implementation Steps

1. Move rate limiter configuration to `ProviderConfig` with `rate_limit_rpm`.
2. Create per-provider rate limiter instances keyed by provider name.
3. `with_rate_limiter()` should be auto-wired from config.
4. Default to 60 RPM only when no config specified.

#### Verification Criteria

- [ ] Two providers with different RPM limits respect their individual limits
- [ ] No global singleton rate limiter
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.41: Add retry logic for transient provider failures
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
**Depends On**: none

#### Context

`ModelCallService` has `fallback_models` for model-level failover but no retry logic for
transient errors (500, timeout, rate limit with retry-after).

#### Implementation Steps

1. Add configurable retry policy.
2. Retry on rate limit (honor `retry_after_ms`).
3. Retry on server error (exponential backoff: 1s, 2s, 4s).
4. Retry on timeout (once with 1.5x timeout).
5. Never retry on auth failure, model not found, context overflow.
6. Max retries: configurable, default 2.
7. After retries exhausted, fall through to fallback models.

#### Verification Criteria

- [ ] Transient 500 followed by 200 succeeds without model switch
- [ ] Auth failure does not retry
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.42: Wire provider health circuit breaker to CascadeRouter
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/provider_health.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: Task 1.11

#### Context

`ProviderHealthTracker` implements circuit breaker logic but CascadeRouter does not wire it.
When a provider goes down, the router continues selecting models from that provider.

#### Implementation Steps

1. Load `ProviderHealthRegistry` at CascadeRouter initialization.
2. Before UCB scoring, filter out models whose provider circuit is open.
3. Feed provider health state from model call success/failure.
4. Log warning when circuit opens.

#### Verification Criteria

- [ ] 5 consecutive failures causes CascadeRouter to stop selecting that provider
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.43: Wire SPC alerts drain to runtime consumers
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/adaptive_threshold.rs`
**Depends On**: none

#### Context

SPC alerts from CUSUM/EWMA/BOCPD are collected in `pending_spc_alerts` but `drain_spc_alerts()`
is never called.

#### Implementation Steps

1. After each gate pipeline run, call `drain_spc_alerts()`.
2. Handle alerts: `OutOfControl` -> tighten thresholds; `ChangePoint` -> reset EMA.
3. Log alerts to efficiency events.

#### Verification Criteria

- [ ] Gate pass rate shift triggers SPC alert in logs
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.44: Wire Hotelling T-squared to runtime gate pipeline
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/hotelling.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/adaptive_threshold.rs`
**Depends On**: none

#### Context

Hotelling T-squared joint anomaly detector (439 LOC, tested) is never called. Joint anomalies
(multiple gates degrading simultaneously) go undetected.

#### Implementation Steps

1. After each full pipeline run, call `observe_pipeline()` with pass-rate vector.
2. If `joint_anomaly_detected()`, emit high-priority alert.
3. Log anomaly in episode record.

#### Verification Criteria

- [ ] Simultaneous compile and test pass rate drops trigger Hotelling alert
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.45: Wire domain profiles to AdaptiveThresholds
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/adaptive_threshold.rs`
**Depends On**: none

#### Context

Three domain profiles (coding, research, security) with per-rung priors exist but are never
instantiated. All agents start from neutral priors (0.5).

#### Implementation Steps

1. Select domain profile from agent role config.
2. Apply rung priors as initial EMA values.
3. Default: coding for implementer/reviewer, research for research, security for auditor.

#### Verification Criteria

- [ ] New workspace with "implementer" role starts with coding domain priors
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.46: Wire conductor bandit to live retry paths
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/conductor.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: none

#### Context

Conductor bandit (7 actions, 19-dim context, Thompson+linear scoring) decides retry
strategy but is never invoked. All retry decisions are hardcoded.

#### Implementation Steps

1. Load ConductorBandit from `.roko/learn/conductor.json`.
2. Call `bandit.select_action()` before each retry.
3. Feed reward after retry outcome.
4. Save state on flush.

#### Verification Criteria

- [ ] After 20+ retries, conductor learns to abort earlier for certain patterns
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.47: Wire anomaly detector to live paths
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/anomaly.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: none

#### Context

Anomaly detector (prompt loops, cost spikes, quality degradation) is never instantiated.

#### Implementation Steps

1. Create `AnomalyDetector` at session start.
2. Check prompt hash before each dispatch (detect loops).
3. Check cost after each response (detect spikes).
4. On anomaly: log warning, optionally trigger abort.

#### Verification Criteria

- [ ] Prompt loop (3 identical prompts) triggers anomaly warning
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.48: Wire regression detection alerting path
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/regression.rs`
**Depends On**: none

#### Context

`detect_regressions()` produces reports that are returned and discarded. No alerting.

#### Implementation Steps

1. Log regression alerts at WARN level.
2. Surface in `roko status` output.
3. Feed severe regressions to conductor.

#### Verification Criteria

- [ ] Pass rate drop > 15% shows warning in `roko status`
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.49: Add end-of-run summary to plan runner
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`
**Depends On**: none

#### Context

After `roko plan run` completes, no aggregate outcome summary is printed. Users must read
log files to determine results.

#### Implementation Steps

1. After all tasks complete, collect results from executor state.
2. Print summary:
   ```
   Run complete: {plan_name}
     Passed: 8/10 tasks
     Failed: T6 (gate: clippy), T9 (gate: test)
     Skipped: 0
     Cost: $8.47 | Duration: 34min
     Resume: roko plan run plans/ --resume .roko/state/executor.json
   ```
3. Save to `.roko/state/last-run-summary.json`.

#### Verification Criteria

- [ ] `roko plan run` on 3-task plan prints summary with pass/fail counts
- [ ] Summary includes cost and duration
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.50: Expose `max_concurrent_tasks` from config
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` (line 115)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`
**Depends On**: none

#### Context

Line 115 of `event_loop.rs`: `max_concurrent_tasks: 1`. Despite a full DAG scheduler,
plans execute sequentially.

#### Implementation Steps

1. Add `max_concurrent_tasks` to `[execution]` config in roko.toml.
2. Read from config instead of hardcoding 1.
3. Default to 1, allow up to 8.
4. Add `--parallel <N>` CLI flag for override.

#### Verification Criteria

- [ ] `--parallel 4` with 4 independent tasks starts all 4 simultaneously
- [ ] Default (no flag) runs sequentially
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.51: Make `dangerously_skip_permissions` configurable
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs` (line 394)
**Depends On**: none

#### Context

Line 394: `dangerously_skip_permissions: true`. Always. No configuration.

#### Implementation Steps

1. Add `skip_permissions: bool` to `[execution]` config (default: true for backward compat).
2. Generate default contract YAML during `roko init`.
3. Read config in plan.rs instead of hardcoding.
4. Log warning when running with skip_permissions = true.

#### Verification Criteria

- [ ] `skip_permissions = false` with contract YAML enforces restrictions
- [ ] Default (true) maintains current behavior
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.52: Replace ACP inline review prompts with templates
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/reviewer.rs`
**Depends On**: none

#### Context

`run_multi_role_review()` hardcodes full role descriptions in `format!()` strings that
partially duplicate `ReviewerTemplate`.

#### Implementation Steps

1. Replace inline prompts with calls to `ReviewerTemplate::architect()` and `::security()`.
2. Add template methods if they don't exist.
3. Remove inline role description strings.

#### Verification Criteria

- [ ] `grep -rn 'Architect Reviewer' crates/roko-acp/` returns zero matches in non-template code
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.53: Fix OpenAI-compat provider quirks fragmentation
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/openai_compat_backend.rs`
**Depends On**: none

#### Context

Per-provider workarounds accumulate as boolean flags. Flag combinations grow exponentially.

#### Implementation Steps

1. Create `ProviderQuirks` struct with all quirk fields.
2. Implement `ProviderQuirks::for_provider(name)` with match on known providers.
3. Replace individual boolean fields with `self.quirks.field`.

#### Verification Criteria

- [ ] Adding a new provider requires only a `ProviderQuirks::for_provider` entry
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.54: Make tool loop max iterations configurable
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/cerebras.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openai_compat.rs`
**Depends On**: none

#### Context

Cerebras uses 50 iterations, OpenAI-compat uses 30. Not configurable.

#### Implementation Steps

1. Add `max_tool_iterations` to `ModelProfile` or `ProviderConfig`.
2. Read from config in each adapter.
3. Default: 30 for API, 50 for Cerebras.

#### Verification Criteria

- [ ] `max_tool_iterations = 10` in config limits agent to 10 iterations
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.55: Unify StateHub types between serve and CLI
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` (lines 68-75, `#[path]` include)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/state_hub.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/state_hub.rs` (if separate)
**Depends On**: none

#### Context

`roko-serve` at line 68 includes the `state_hub.rs` via `#[path]`:
```rust
#[path = "../../roko-core/src/state_hub.rs"]
pub mod state_hub_compat;
```
This creates two copies of the `StateHub` type (one in serve, one in core), preventing
zero-cost sharing between the two crates.

#### Implementation Steps

1. Export `StateHub` as a public type from `roko-core`.
2. Remove the `#[path]` include from `roko-serve`.
3. Import `roko_core::StateHub` in both `roko-serve` and `roko-cli`.
4. Share a single instance when running together (`roko run --serve`).

#### Verification Criteria

- [ ] `roko run --serve "hello"` SSE endpoint emits real-time events
- [ ] No `#[path]` includes of `state_hub.rs`
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.56: Wire dream consolidation trigger
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs`
**Depends On**: none

#### Context

Dream consolidation is built but has no runtime trigger. `DreamTriggerSink` writes events
that nothing reads. Knowledge consolidation only happens via manual `roko knowledge dream run`.

#### Implementation Steps

1. Add dream loop to `roko serve` background tasks.
2. Configure via `roko.toml` (`[dreams]` section): cron interval or plan-completion trigger.
3. Alternatively: add post-run hook in `roko plan run`.
4. Report dream status in `roko status`.

#### Verification Criteria

- [ ] After plan run, dream cycle runs automatically (if configured)
- [ ] `roko status` shows last dream timestamp
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.57: Wire knowledge candidate ingestion post-run
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/knowledge_ingestion.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: none

#### Context

Knowledge candidates written to `.roko/learn/knowledge_candidates.jsonl` are never ingested
into `KnowledgeStore`.

#### Implementation Steps

1. After each plan run, read new candidates.
2. Validate and deduplicate against existing entries.
3. Ingest validated candidates.
4. Mark ingested candidates (or truncate file).

#### Verification Criteria

- [ ] After run producing candidates, `roko knowledge stats` shows new entries
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.58: Fix `--share` without `--serve` producing dead URL
**Priority**: P2
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/share.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: none

#### Context

`roko run --share` prints `http://localhost:6677/runs/{token}` which is inaccessible
without serve running.

#### Implementation Steps

1. When `--share` without `--serve`: generate self-contained HTML artifact.
2. Write to `.roko/shared/{token}.html`.
3. Print local file path instead of dead URL.
4. When `--serve` IS active: print serve URL as before.

#### Verification Criteria

- [ ] `roko run --share "hello"` without serve prints local file path
- [ ] HTML file opens in browser showing transcript
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.59: Add `--dry-run` to `roko plan run`
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`
**Depends On**: none

#### Context

No way to preview plan execution without running agents.

#### Implementation Steps

1. Add `--dry-run` flag to `plan run`.
2. Load plans, build DAG, compute execution waves.
3. Show: wave ordering, per-task model selection, estimated cost.
4. Do not dispatch agents. Exit after printing.

#### Verification Criteria

- [ ] `roko plan run plans/ --dry-run` prints wave ordering and cost estimates
- [ ] No agents spawned
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.60: Make workspace map cap proportional to context tier
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs` (line 22, `WORKSPACE_MAP_LINE_LIMIT`)
**Depends On**: Task 1.18

#### Context

`WORKSPACE_MAP_LINE_LIMIT = 200` is fixed. Should scale with context tier.

#### Implementation Steps

1. Make cap proportional: Surgical 50, Focused 150, Full 300, Extended 500.
2. Or filter to files relevant to the current task.

#### Verification Criteria

- [ ] Surgical tier: workspace map 50 lines max
- [ ] Full tier: 300 lines
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.61: Wire knowledge store to CascadeRouter model selection
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/knowledge_store.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: Task 1.11

#### Context

Knowledge store contains task-specific insights that could inform model routing. CascadeRouter
does not query it. `DreamRoutingAdvice` is generated but not loaded.

#### Implementation Steps

1. Load `DreamRoutingAdvice` at CascadeRouter initialization.
2. Apply `dream_advice_to_routing_bias()`.
3. Query knowledge for task-specific model hints.

#### Verification Criteria

- [ ] After dream cycle with routing advice, model selections reflect the advice
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.62: Fix GatePipeline / ComposedGatePipeline duplication
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_pipeline.rs`
**Depends On**: none

#### Context

`GatePipeline` and `ComposedGatePipeline` partially duplicate logic. Dead code
`let _ = pipeline;` exists.

#### Implementation Steps

1. Have `ComposedGatePipeline` Sequential mode delegate to `GatePipeline`.
2. Or deprecate `GatePipeline` in favor of `ComposedGatePipeline`.
3. Remove dead code assignments.

#### Verification Criteria

- [ ] Sequential gate execution uses a single code path
- [ ] No dead code assignments
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.63: Wire ProcessRewardModel to orchestrator
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/process_reward.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: none

#### Context

ProcessRewardModel tracks per-turn gate snapshots, derives Promise (probability of eventual
success) and Progress signals. Not instantiated. Tasks clearly failing continue consuming budget.

#### Implementation Steps

1. Instantiate PRM per-task in event loop.
2. After each gate snapshot, update PRM.
3. If Promise < 0.1, abort early.
4. Log PRM signals in episodes.

#### Verification Criteria

- [ ] Task failing compile 3 times with worsening output aborted by PRM
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.64: Wire AcceptanceContract to gate pipeline
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/acceptance_contract.rs`
**Depends On**: none

#### Context

AcceptanceContract defines formal requirements (NoStubRequirement, etc.) with evidence
collection. Not wired into the gate pipeline.

#### Implementation Steps

1. Add as optional post-gate verification step.
2. Load requirements from task definition.
3. After gate pipeline passes, check contract.
4. If contract fails, treat as gate failure.

#### Verification Criteria

- [ ] `NoStubRequirement` catches stub functions after gates pass
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.65: Add anti-pattern checks as pre-gate step
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/` (new file: `anti_pattern_gate.rs`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: none

#### Context

Grep-based anti-pattern checks (AP-1 through AP-10) catch common LLM mistakes in milliseconds.
Not integrated into any gate.

#### Implementation Steps

1. Create `AntiPatternGate` in roko-gate.
2. Patterns: stub pass, `block_on` in async, duplicate traits, raw `Command::new("claude")`,
   inline prompts, `Mutex` across `.await`, empty function bodies, `unimplemented!/unreachable!`.
3. Run as rung -1 (before compile) -- millisecond cost.
4. Return structured feedback per pattern found.

#### Verification Criteria

- [ ] `unimplemented!()` in non-test code triggers anti-pattern check
- [ ] Feedback is structured (pattern name, file, line)
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.66: Wire VerdictPublisher to all gate dispatch paths
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/verdict_publisher.rs`
- Gate dispatch paths in runner and effect_driver
**Depends On**: none

#### Context

VerdictPublisher is optional and rarely provided. Gate verdicts are not broadcast to TUI.

#### Implementation Steps

1. Provide VerdictPublisher in each gate dispatch path.
2. Wire to DashboardEvent emitter (TUI).
3. Wire to SSE channel (serve).

#### Verification Criteria

- [ ] During `roko plan run` with dashboard, gate progress appears in real time
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.67: Add gate budget tracking for LLM judge calls
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/` (LLM judge implementation)
**Depends On**: none

#### Context

LLM judge gate calls have no cost tracking. Each call is an LLM API call but no episode or
cost is recorded.

#### Implementation Steps

1. Record episode per judge invocation.
2. Track cumulative gate cost separately.
3. Cap judge invocations per task (default: 3).
4. Include gate cost in run summary.

#### Verification Criteria

- [ ] LLM judge call produces cost entry
- [ ] Total cost includes judge costs
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.68: Wire StagingBuffer lightweight promotion
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/staging.rs`
**Depends On**: none

#### Context

Candidates in StagingBuffer progress Raw -> Replayed -> Validated but promotion requires
a full dream cycle. Buffer grows unbounded without one.

#### Implementation Steps

1. Add lightweight promotion check in LearningRuntime.
2. After each run, check for Validated candidates.
3. Promote to KnowledgeStore without full dream cycle.

#### Verification Criteria

- [ ] Validated candidates appear in KnowledgeStore without manual dream run
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.69: Add cross-session cost aggregation
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/costs_db.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/budget.rs`
**Depends On**: none

#### Context

Cost tracking per session exists but no cross-session aggregation for daily budget enforcement.

#### Implementation Steps

1. `CostsDb.aggregate_since(today_start)` -> daily total.
2. Initialize `BudgetGuardrail.day_spent` from aggregate.
3. Expose aggregates via `roko learn efficiency`.

#### Verification Criteria

- [ ] 3 sessions on same day shows cumulative daily cost in `roko learn efficiency`
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.70: Add content-type-aware token counting ratios
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/token_counter.rs`
**Depends On**: none

#### Context

Flat 4:1 char-to-token ratio. Code is 3:1, markdown 5:1. Errors compound at tight budgets.

#### Implementation Steps

1. Add `content_type` parameter to `estimate_tokens()`: Code, Prose, Markdown.
2. Use ratios: Code 3.0, Prose 4.0, Markdown 5.0.
3. Unknown: use 3.5 (conservative).

#### Verification Criteria

- [ ] Code content estimates more tokens per character than markdown
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.71: Make knowledge confidence thresholds tier-dependent
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs`
**Depends On**: Task 1.18

#### Context

Knowledge thresholds (domain >= 0.5, etc.) are hardcoded. Too permissive for small models.

#### Implementation Steps

1. Make thresholds dependent on ContextTier:
   - Surgical: 0.8, 0.7, 0.5 (only high-confidence)
   - Focused: 0.5, 0.3, 0.2 (current defaults)
   - Full: 0.3, 0.2, 0.1 (include speculative)

#### Verification Criteria

- [ ] Surgical tier includes only high-confidence knowledge
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.72: Wire conversation compaction to `roko chat`
**Priority**: P2
**Estimated Effort**: 1 hour
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/compaction.rs`
**Depends On**: none

#### Context

`compact_history()` in `compaction.rs` is fully implemented but never called from chat.
Long sessions hit context limits.

#### Implementation Steps

1. After each turn, check if history exceeds 80% of context window.
2. If exceeded, call `compact_history()`.
3. Preserve anchor turns.

#### Verification Criteria

- [ ] 50-turn chat session continues without context overflow
- [ ] Old turns are summarized
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.73: Add prompt caching metrics to ModelCallService
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
**Depends On**: none

#### Context

ModelCallService has L1 response cache but no metrics. No hit rate, no eviction stats.

#### Implementation Steps

1. Add `CacheMetrics`: hits, misses, evictions, size_bytes.
2. Expose via gateway events.
3. Track Anthropic server-side cache (`cache_read_tokens`).
4. Report in cost panel.

#### Verification Criteria

- [ ] `roko learn efficiency` shows cache hit rate
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.74: Add disk pressure monitoring pre-dispatch
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs` (or new module)
**Depends On**: none

#### Context

Disk exhaustion from cargo build caches causes silent failures. No monitoring exists.

#### Implementation Steps

1. Before dispatch, check available disk space.
2. If below 5GB, pause with warning.
3. Optionally: `cargo clean --target-dir` on old worktrees.
4. Resume when space available.

#### Verification Criteria

- [ ] Dispatch pauses with "insufficient disk space" when below threshold
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.75: Add agent execution time monitoring
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
**Depends On**: none

#### Context

~5% of agents ignore instructions, taking 5-15x longer. No monitoring detects this.

#### Implementation Steps

1. Track expected time per tier (fast: 2min, standard: 10min, complex: 30min).
2. Monitor actual duration.
3. If > 3x expected, log warning.
4. Optionally: SIGTERM and retry.

#### Verification Criteria

- [ ] 10x-longer-than-expected agent triggers duration warning
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.76: Fix WorkflowEngine missing worktree integration
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
**Depends On**: none

#### Context

WorkflowEngine operates on single `workdir: PathBuf`. Parallel tasks cause file conflicts.

#### Implementation Steps

1. Add optional `WorktreeManager` to `EffectServices`.
2. Allocate worktree per parallel task.
3. Merge via MergeQueue with file-overlap detection.
4. Fall back to single-directory mode when unavailable.

#### Verification Criteria

- [ ] Two parallel tasks modifying different files both succeed
- [ ] Each runs in own worktree
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.77: Unify two PipelineState state machines
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/pipeline_state.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/phase.rs`
**Depends On**: none

#### Context

`PipelineStateV2` (10 states) and `PlanPhase` (14 states) model the same concept but are
not interoperable.

#### Implementation Steps

1. Define superset state machine with optional phases.
2. Map optional phases (Enriching, DocRevision, RegeneratingVerify) to skip-when-unconfigured.
3. Both engines use unified state machine.
4. Add adapter for backward compatibility.

#### Verification Criteria

- [ ] Single `WorkflowPhase` enum in both crates
- [ ] Only adapter/compat code references old types
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

### Task 1.78: Consolidate 4 agent dispatch implementations
**Priority**: P2
**Estimated Effort**: 10 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: none

#### Context

Four dispatch implementations with different features, error handling, timeout logic, token
counting, and safety checks. Bug fixes don't propagate across copies.

#### Implementation Steps

1. Consolidate into EffectDriver's `ModelCaller` + `PromptAssembler` trait pattern.
2. Add service traits for safety, custody, knowledge routing.
3. Compose into `EffectServices`.
4. All paths delegate to EffectDriver.
5. Delete redundant implementations.

#### Verification Criteria

- [ ] Single dispatch code path handles all cases
- [ ] Changing dispatch behavior affects all surfaces
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test --workspace` passes

---

## Dependency Graph

```
Task 1.09 -> Task 1.10              (gate config normalization before init fix)
Task 1.11 -> Task 1.22              (cascade router before runner v2 observations)
Task 1.11 -> Task 1.42              (cascade router before health circuit breaker)
Task 1.11 -> Task 1.61              (cascade router before knowledge-informed routing)
Task 1.18 -> Task 1.19              (context tier before budget predictor)
Task 1.18 -> Task 1.60              (context tier before workspace map cap)
Task 1.18 -> Task 1.71              (context tier before knowledge confidence)
Task 1.25 -> Task 1.27              (section effectiveness recording before reading)

All other tasks are independent and can be parallelized.
```

## Execution Order Recommendation

**Week 1**: Tasks 1.01-1.10 (P0 -- crashes, security, data loss). All independent, parallelizable.
**Week 2**: Tasks 1.11-1.17 (P1 foundation -- cascade router, feedback, budget, streaming).
**Week 3**: Tasks 1.18-1.27 (P1 prompt/learning -- context tiers, parsing, runner v2 learning).
**Week 4**: Tasks 1.28-1.34 (P1 gates/dispatch -- failure routing, ACP, env vars).
**Week 5-6**: Tasks 1.35-1.54 (P2 first batch -- model unification, retry, thresholds).
**Week 7-8**: Tasks 1.55-1.78 (P2 second batch -- StateHub, dreams, architecture debt).
