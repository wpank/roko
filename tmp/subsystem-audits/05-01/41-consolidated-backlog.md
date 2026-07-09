# 41 — Consolidated Implementation Backlog

Generated: 2026-05-01. Canonical checklist for all open issues from audit docs
36-40 and checklist doc 35.

**Rules**: Each item is self-contained. An agent picks one item, executes it, and
marks it done. No reading other audit docs required. All line numbers verified
against the current worktree as of 2026-05-01.

**Status key**: `[ ]` = open, `[~]` = in progress, `[x]` = done and verified.

---

## Global Anti-Patterns

These apply to EVERY item. Violating any of these is a reject.

1. **No new dispatch paths.** 4+ exist. Fix `ModelCallService` or `DispatchResolver`.
2. **Skeletons ≠ migrations.** A type existing does not mean the product path is migrated.
3. **Unknown ≠ zero.** Missing usage/cost/context must stay `None`, never become `0`.
4. **No silent fallback.** Failed resolution → typed error, not synthesized config.
5. **Missing config → restricted.** Never grant permissions on load failure.
6. **No string-interpolated payloads.** Use `serde`/`toml` serializers.
7. **No regex prompt scraping.** Consume typed `CommandEvent`s.
8. **One item per commit.** Don't expand scope mid-item. Split follow-ups.
9. **No `unwrap()`/`panic!()` in changed code.** Return typed errors.
10. **No unrelated changes.** Don't refactor neighbors, add docstrings, or "improve" adjacent code.

## Pre-Commit (mandatory before every commit)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

# TIER 0: Stop Active Bleeding

7 items. Each is 1-15 lines changed. ~1 session total.

---

## [ ] T0-1: Mask secrets in GET /api/config/toml

**Why**: The TOML config endpoint returns raw secrets including an Ethereum private key.
The JSON endpoint masks them. The TOML endpoint does not.

**File**: `crates/roko-serve/src/routes/config.rs`

**What to change** (lines 48-58):

The function `get_config_toml` currently does:
```rust
let cfg = state.load_roko_config();
let toml_str = toml::to_string_pretty(cfg.as_ref())?;
Ok(([(CONTENT_TYPE, "application/toml")], toml_str))
```

Change to:
```rust
let cfg = state.load_roko_config();
let mut value = serde_json::to_value(cfg.as_ref())
    .map_err(|e| ApiError::internal(format!("serialize config: {e}")))?;
mask_secret_fields(&mut value);
let toml_value: toml::Value = serde_json::from_value(value)
    .map_err(|e| ApiError::internal(format!("convert to toml: {e}")))?;
let toml_str = toml::to_string_pretty(&toml_value)
    .map_err(|e| ApiError::internal(format!("serialize toml: {e}")))?;
Ok(([(CONTENT_TYPE, "application/toml")], toml_str))
```

**Reference**: The JSON endpoint at lines 36-43 already does this correctly:
```rust
let mut value = serde_json::to_value(cfg.as_ref())?;
mask_secret_fields(&mut value);
```

**Test to add**: In the existing config route tests, add a test that constructs
a config with `serve.auth.api_key = "test-secret-key"`, calls `get_config_toml`,
and asserts `"test-secret-key"` does not appear in the response body.

**Verify**:
```bash
cargo test -p roko-serve config_toml --lib
```

**Do not**: Remove the endpoint (builder workspaces use it). Do not change `mask_secret_fields` itself (that's T0-2).

---

## [ ] T0-2: Expand mask_secret_fields coverage

**Why**: `mask_secret_fields()` only masks 3 of 5+ secret fields. `chain.wallet_key`
(Ethereum private key) and `webhooks.github.secret` (HMAC key) are exposed.

**File**: `crates/roko-serve/src/routes/config.rs`

**What to change** (lines 245-259):

Current `mask_secret_fields` calls:
```rust
mask_secret_field(value, &["serve", "auth"], "api_key", "ROKO_SERVE_AUTH_API_KEY");
mask_secret_field(value, &["server"], "auth_token", "ROKO_SERVER_AUTH_TOKEN");
mask_secret_field(value, &["deploy"], "railway_api_token", "ROKO_DEPLOY_RAILWAY_API_TOKEN");
```

Add after the existing calls:
```rust
mask_secret_field(value, &["chain"], "wallet_key", "ROKO_CHAIN_WALLET_KEY");
mask_secret_field(value, &["webhooks", "github"], "secret", "ROKO_WEBHOOKS_GITHUB_SECRET");
// Mask per-provider api_key fields
if let Some(providers) = value.get_mut("providers").and_then(|v| v.as_object_mut()) {
    for (_name, provider) in providers.iter_mut() {
        if let Some(obj) = provider.as_object_mut() {
            if obj.contains_key("api_key") {
                obj.insert("api_key".to_string(), Value::String("****".to_string()));
            }
        }
    }
}
```

**Test to add**: Construct a `serde_json::Value` with `chain.wallet_key = "0xdeadbeef"`,
`webhooks.github.secret = "ghsecret"`, `providers.anthropic.api_key = "sk-ant-xxx"`.
Call `mask_secret_fields`. Assert all three are `"****"`.

**Verify**:
```bash
cargo test -p roko-serve mask_secret --lib
```

**Do not**: Change `providers.*.api_key_env` (env var names, not secrets). Do not touch the response scrubber middleware.

---

## [ ] T0-3: Add path validation to shared runs

**Why**: Every other route group uses `validate_path_segment()` to prevent path traversal.
Shared runs is the only exception. User-supplied IDs go directly into file paths.

**File**: `crates/roko-serve/src/routes/shared_runs.rs`

**What to change** (lines 270-276):

In `load_transcript_record`, before the path construction:
```rust
fn load_transcript_record(state: &AppState, id: &str) -> Option<LoadedTranscript> {
    // ADD THIS LINE:
    crate::error::validate_path_segment(id).ok()?;
    let path = state.workdir.join(".roko").join("shared").join(format!("{id}.json"));
```

Also add validation in `create_share` (line ~219) for the token parameter:
```rust
crate::error::validate_path_segment(&token)?;
```

**Reference**: `validate_path_segment` is defined at `crates/roko-serve/src/error.rs:155-162`
and rejects `..`, `/`, and `\\`. Already used in `routes/jobs.rs` (12 sites),
`routes/plans.rs` (6 sites), `routes/auth.rs`, `routes/research.rs`, `routes/prds.rs`.

**Test to add**:
```rust
#[test]
fn shared_run_rejects_path_traversal() {
    // create mock AppState
    let result = load_transcript_record(&state, "../../etc/passwd");
    assert!(result.is_none());
}
```

**Verify**:
```bash
cargo test -p roko-serve shared_run --lib
rg 'validate_path_segment' crates/roko-serve/src/routes/shared_runs.rs  # should match
```

**Do not**: Change `validate_path_segment` itself. Do not add validation to other route groups (they already have it).

---

## [ ] T0-4: Move generic webhook behind auth

**Why**: `/webhooks/generic` is mounted outside the auth layer. It accepts arbitrary JSON
and persists it as a signal. No auth, no signature. Its own docstring says "intended for
internal use behind auth."

**File**: `crates/roko-serve/src/routes/webhooks.rs` (handler at lines 158-169)
**File**: `crates/roko-serve/src/routes/mod.rs` (mounting at line 167)

**What to change**:

In `webhooks.rs`, split the routes function so `generic_webhook` is separate:
```rust
pub fn public_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/webhooks/github", post(github_webhook))
        .route("/webhooks/slack", post(slack_webhook))
}

pub fn authenticated_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/webhooks/generic", post(generic_webhook))
}
```

In `mod.rs` line 167, change `.merge(webhooks::routes())` to
`.merge(webhooks::public_routes())` for the public router, and add
`webhooks::authenticated_routes()` to the `api` router (inside the auth layer,
around line 120).

**Test to add**: With auth enabled, POST to `/api/webhooks/generic` without an
API key returns 401. With a valid key, it returns 200.

**Verify**:
```bash
cargo test -p roko-serve webhook --lib
rg 'generic_webhook' crates/roko-serve/src/routes/mod.rs  # should be in api router
```

**Do not**: Move GitHub/Slack webhooks behind API auth (they use HMAC, which is correct). Do not remove the generic webhook.

---

## [ ] T0-5: Validate agent registration URLs against SSRF

**Why**: `RegisterAgentRequest` accepts arbitrary URLs. Server-side requests to these
URLs enable SSRF. Register `http://169.254.169.254/` → read cloud metadata via
`GET /api/agents/{id}/logs`.

**File**: `crates/roko-serve/src/routes/agents.rs`
- `RegisterAgentRequest` struct: lines 1635-1678
- `proxy_agent_logs`: line 1025 (uses `agent.endpoints.rest` directly)
- `send_message`: line 1092 (uses rest endpoint directly)

**What to change**:

Add a validation function (in `agents.rs` or `crate::error`):
```rust
fn validate_agent_url(url: &str) -> Result<(), ApiError> {
    let parsed = url::Url::parse(url)
        .map_err(|_| ApiError::bad_request("invalid URL"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        s => return Err(ApiError::bad_request(format!("unsupported scheme: {s}"))),
    }
    let host = parsed.host_str()
        .ok_or_else(|| ApiError::bad_request("URL has no host"))?;
    if host == "localhost" || host == "127.0.0.1" || host == "[::1]"
        || host.starts_with("10.") || host.starts_with("172.16.")
        || host.starts_with("172.17.") || host.starts_with("172.18.")
        || host.starts_with("172.19.") || host.starts_with("172.2")
        || host.starts_with("172.30.") || host.starts_with("172.31.")
        || host.starts_with("192.168.") || host.starts_with("169.254.")
        || host.starts_with("fe80:") {
        return Err(ApiError::bad_request("internal/private URLs not allowed"));
    }
    Ok(())
}
```

Call in `RegisterAgentRequest::validate_payload()` (line 1681-1683):
```rust
fn validate_payload(&self) -> Result<(), ApiError> {
    validate_with_validator(self)?;
    if let Some(ref url) = self.rest_endpoint { validate_agent_url(url)?; }
    if let Some(ref url) = self.websocket_endpoint { validate_agent_url(url)?; }
    if let Some(ref url) = self.a2a_endpoint { validate_agent_url(url)?; }
    if let Some(ref url) = self.mcp_endpoint { validate_agent_url(url)?; }
    Ok(())
}
```

**Note**: `url` crate is already a dependency in the workspace.

**Test to add**:
```rust
#[test]
fn rejects_internal_urls() {
    assert!(validate_agent_url("http://169.254.169.254/").is_err());
    assert!(validate_agent_url("http://10.0.0.1/").is_err());
    assert!(validate_agent_url("http://localhost/").is_err());
    assert!(validate_agent_url("https://api.example.com/").is_ok());
}
```

**Verify**:
```bash
cargo test -p roko-serve validate_agent_url --lib
```

**Do not**: Validate at proxy time (too late). Do not block all non-loopback (agents may run remotely).

---

## [ ] T0-6: Fix knowledge sink filename mismatch

**Why**: Sink writes `knowledge_candidates.jsonl` (underscore). Reader expects
`knowledge-candidates.jsonl` (hyphen). Output is orphaned.

**File**: `crates/roko-cli/src/commands/plan.rs` (line 396)

**What to change**:

Find the `KnowledgeIngestionSink::at(...)` call. The path is constructed from a
`knowledge_path` variable. Trace it to where the filename is set. Change
`knowledge_candidates.jsonl` to `knowledge-candidates.jsonl`.

The correct constant is `roko_neuro::admission::DEFAULT_KNOWLEDGE_CANDIDATES_FILE`
at `crates/roko-neuro/src/admission.rs:26`:
```rust
pub const DEFAULT_KNOWLEDGE_CANDIDATES_FILE: &str = "knowledge-candidates.jsonl";
```

Ideally, use the constant directly instead of a string literal:
```rust
KnowledgeIngestionSink::at(&learn_dir.join(
    roko_neuro::admission::DEFAULT_KNOWLEDGE_CANDIDATES_FILE
))
```

**Verify**:
```bash
rg 'knowledge_candidates' crates/ -g '*.rs'  # should return 0 matches
rg 'knowledge.candidates' crates/ -g '*.rs'  # should show only hyphen variant
```

**Do not**: Change the neuro admission constant. Do not wire `.with_ingestor()` (that's T4-29).

---

## [ ] T0-7: Fix duplicate model slugs with wrong context_windows

**Why**: 6 model alias pairs have conflicting `context_window` values. Wrong values
cause incorrect prompt truncation.

**File**: `roko.toml`

**Exact changes** (update the WRONG alias to match the CORRECT value):

| Line | Section | Current `context_window` | Change to |
|------|---------|------------------------|-----------|
| 203 | `[models.sonnet]` | 128000 | **200000** |
| 621 | `[models.opus]` | 128000 | **200000** |
| 393 | `[models.gemini-pro]` | 128000 | **1048576** |
| 108 | `[models.kimi-k25]` | 128000 | **262144** |
| 678 | `[models.kimi-k26]` | 128000 | **262144** |
| 469 | `[models.sonar]` | 128000 | **127000** |

**Verify**:
```bash
# No duplicate backend slugs should have different context_window values
rg 'context_window' roko.toml
```

**Do not**: Change model slugs, provider assignments, or aliases. Only fix `context_window` values.

---

# TIER 1: Fix Silent Data Corruption

8 items. Things running but producing wrong/empty/duplicate data. ~2 sessions.

---

## [ ] T1-8: Propagate dispatch metadata into RunnerEvent

**Why**: `AgentOutcome` is constructed with `model: String::new(), provider: String::new()`
(event_loop.rs:1376-1377). This makes RoutingObservationSink a no-op, episodes have
empty model fields, and knowledge candidates have no attribution.

**File**: `crates/roko-cli/src/runner/event_loop.rs`

**What to change**:

1. Find the `RunnerEvent::TaskAttemptCompleted` variant (grep for it across the runner
   module). Add fields: `model: String, provider: String`.

2. In the dispatch code that emits `TaskAttemptCompleted`, populate from the agent's
   actual dispatch metadata. The `RunState` already has `agent_model` (populated by
   `AgentEvent::AgentStarted`). Thread it through.

3. In `runner_event_to_feedback` (line 1358+), at lines 1376-1377, replace:
   ```rust
   model: String::new(),
   provider: String::new(),
   ```
   with:
   ```rust
   model: event.model.clone(),
   provider: event.provider.clone(),
   ```

**Verify**:
```bash
rg 'String::new\(\)' crates/roko-cli/src/runner/event_loop.rs | grep -E 'model|provider'
# should return 0 matches
cargo test -p roko-cli runner --lib
```

**Do not**: Remove legacy `emit_feedback` (that's T1-9, depends on this). Do not change CascadeRouter.

---

## [ ] T1-9: Remove dual feedback path (legacy emit_feedback + facade)

**Depends on**: T1-8

**Why**: Two parallel paths fire on the same events, writing duplicate episodes to the
same JSONL file (one with real model data, one previously with empty strings).

**File**: `crates/roko-cli/src/runner/event_loop.rs`
- Legacy `emit_feedback`: line ~2489
- Facade `FeedbackFacade`: line ~1339

**What to change**:

1. After T1-8 lands, verify the facade carries real model/provider data.
2. Check every side-effect of `emit_feedback()` has a facade sink equivalent:
   - `append_jsonl(episodes)` → `EpisodeSink` ✓
   - `observe_cascade_router` → `RoutingObservationSink` ✓ (after T1-8)
   - `observe_bandit_policy` → bandit observation ✓
   - `update_gate_thresholds` → verify coverage, add to a sink if missing
3. Remove the `emit_feedback()` call site and function body.

**Verify**:
```bash
rg 'emit_feedback' crates/roko-cli/src/runner/ -n  # should be 0
# Run a plan, check episodes.jsonl has no duplicates per task attempt
cargo test -p roko-cli --lib
```

**Do not**: Remove before T1-8 lands. Do not change episode schema.

---

## [ ] T1-10: Replace gate catch-all with explicit match arms

**Why**: `_ =>` in `selected_gate_steps()` silently drops rungs 3 (Symbol), 5 (PropertyTest),
6 (Integration). The 7-rung pipeline is actually a 3-rung pipeline.

**File**: `crates/roko-cli/src/orchestrate.rs`
**Function**: `selected_gate_steps` starting at line 17240

**What to change**:

Find the `match rung` block (around lines 17270-17298). Replace:
```rust
_ => {
    skipped_count = skipped_count.saturating_add(1);
}
```

With explicit arms:
```rust
Rung::Symbol => {
    tracing::debug!(rung = 3, "Symbol gate skipped: capability detection pending (T1-11)");
    skipped_count = skipped_count.saturating_add(1);
}
Rung::PropertyTest => {
    tracing::debug!(rung = 5, "PropertyTest gate skipped: capability detection pending (T1-11)");
    skipped_count = skipped_count.saturating_add(1);
}
Rung::Integration => {
    tracing::debug!(rung = 6, "Integration gate skipped: capability detection pending (T1-11)");
    skipped_count = skipped_count.saturating_add(1);
}
```

This makes each rung's skip explicit with a reason and ensures adding a new `Rung`
variant in the future causes a compile error.

**Verify**:
```bash
rg '_ =>' crates/roko-cli/src/orchestrate.rs | grep -i 'selected_gate'
# should return 0 matches inside selected_gate_steps
cargo check -p roko-cli --lib
```

**Do not**: Implement gate construction for 3/5/6 here (that's after T1-11). Do not add `#[allow(unreachable_patterns)]`.

---

## [ ] T1-11: Fix gate_rung_caps hardcoded false values

**Why**: Three caps are hardcoded `false`, making `select_rungs()` exclude Symbol,
PropertyTest, and Integration even if T1-10 adds match arms.

**File**: `crates/roko-cli/src/orchestrate.rs`
**Function**: `gate_rung_caps` at lines 17209-17223

**What to change**:

Replace:
```rust
has_symbol_manifest: false,
has_property_tests: false,
has_integration_scenario: false,
```

With detection logic:
```rust
has_symbol_manifest: exec_dir.join("symbols.json").exists()
    || exec_dir.join(".roko").join("symbols").exists(),
has_property_tests: exec_dir.join("proptest-regressions").exists()
    || exec_dir.join("tests").join("property").exists(),
has_integration_scenario: exec_dir.join("tests").join("integration").exists()
    || exec_dir.join("integration-tests").exists(),
```

If this is too aggressive (these dirs may not exist in the workspace), use `true`
for all and let the gate implementations return `Skipped` when input is insufficient.
The gates already handle this gracefully.

**Verify**:
```bash
rg 'has_symbol_manifest: false' crates/roko-cli/src/orchestrate.rs  # should be 0
cargo check -p roko-cli --lib
```

**Do not**: Add expensive fs scans. Do not change `select_rungs` logic. Do not change gate implementations.

---

## [ ] T1-12: Wire validate_strict_config_toml into production load

**Why**: The validator exists and is unit-tested but never called. Users can set
`dangerously_skip_permissions = true` in shared config and it's silently accepted.

**File**: `crates/roko-core/src/config/mod.rs`
**Function**: `load_config` at lines 97-103

**What to change**:

After reading the file as a string, before `toml::from_str()`:
```rust
pub fn load_config(workdir: &Path) -> Result<RokoConfig, LoadConfigError> {
    let path = workdir.join("roko.toml");
    if !path.exists() {
        return Ok(RokoConfig::default());
    }
    let raw = std::fs::read_to_string(&path)?;

    // ADD: validate before deserializing
    use crate::config::validation::{validate_strict_config_toml, StrictConfigSource};
    if let Err(e) = validate_strict_config_toml(&raw, &StrictConfigSource::SharedFile) {
        return Err(LoadConfigError::Validation(e.to_string()));
    }

    let mut config: RokoConfig = toml::from_str(&raw)?;
    // ... rest of function
```

If `LoadConfigError` doesn't have a `Validation` variant, add one.

**Verify**:
```bash
cargo test -p roko-core config --lib
# Manual test: add dangerously_skip_permissions = true to a temp roko.toml,
# run `cargo run -p roko-cli -- config show`, verify it errors
```

**Do not**: Silently downgrade invalid config to defaults. Do not mutate config files.

---

## [ ] T1-13: Remove ContextualBanditPolicy shadow mode

**Why**: The bandit is permanently in Shadow mode. It records observations but always
picks the first candidate. No code transitions to Active. The CascadeRouter already
handles model selection learning.

**File**: `crates/roko-cli/src/commands/plan.rs` (line 355)
**File**: `crates/roko-cli/src/serve_runtime.rs` (line 542)

**What to change** (Option B — remove, since CascadeRouter handles this):

1. Remove `BanditPolicyMode::Shadow` construction from `plan.rs:355`.
2. Remove `BanditPolicyMode::Shadow` construction from `serve_runtime.rs:542`.
3. Remove the `ContextualBanditPolicy` from the runner's feedback observation chain.
4. Remove the `observe_bandit_policy` call from the runner event loop.
5. Keep `contextual_bandit.rs` module (don't delete the implementation).

If Option A (activate) is preferred instead: change `Shadow` to `Active` in both
files. But the CascadeRouter already does this job better (verified in audit 40§5.1).

**Verify**:
```bash
rg 'BanditPolicyMode::Shadow' crates/ -g '*.rs'  # should be 0 (except tests)
cargo check --workspace
```

**Do not**: Change CascadeRouter. Do not change reward math. Do not delete `contextual_bandit.rs`.

---

## [ ] T1-14: Wire observe_pipeline and drain_spc_alerts

**Why**: Per-rung `observe()` works (EMA pass rates update). But `observe_pipeline()`
(cross-rung Hotelling T² anomaly detection) and `drain_spc_alerts()` (CUSUM/EWMA/BOCPD
alerts) are never called. SPC alerts accumulate and are lost.

**File**: `crates/roko-cli/src/orchestrate.rs`
**Location**: After per-rung observe loop at line ~16859-16862

**What to change**:

After the existing loop:
```rust
for recorded in &recorded_verdicts {
    self.adaptive_thresholds.observe(recorded.rung.as_index(), recorded.verdict.passed);
}
```

Add:
```rust
// Cross-rung anomaly detection
let pass_rates: Vec<f64> = recorded_verdicts
    .iter()
    .map(|r| if r.verdict.passed { 1.0 } else { 0.0 })
    .collect();
if !pass_rates.is_empty() {
    self.adaptive_thresholds.observe_pipeline(&pass_rates);
}

// Drain and log SPC alerts
let spc_alerts = self.adaptive_thresholds.drain_spc_alerts();
for (rung, alert) in &spc_alerts {
    tracing::warn!(rung, ?alert, "gate SPC alert detected");
}
```

**Reference**: `observe_pipeline` at `crates/roko-gate/src/adaptive_threshold.rs:468`.
`drain_spc_alerts` at line 446.

**Verify**:
```bash
cargo test -p roko-gate adaptive_threshold --lib
cargo check -p roko-cli --lib
rg 'observe_pipeline' crates/roko-cli/src/orchestrate.rs  # should match
```

**Do not**: Change per-rung `observe()`. Do not block pipeline on alerts (log only). Do not change gate implementations.

---

## [ ] T1-15: Replace permissive safety fallback with restricted

**Why**: Missing/invalid safety config GRANTS broader permissions than configured safety.
`SafetyLayer::with_defaults()` uses `permissive("default")`. `contract_for_role()`
falls back to `permissive()` for missing YAML.

**File**: `crates/roko-agent/src/safety/mod.rs`

**What to change**:

1. At line ~246-251, `SafetyLayer::with_defaults()`:
   Find where `AgentContract::permissive` is used. Replace with
   `AgentContract::restricted` (or `AgentContract::default()` if default is restricted).

2. At line ~866-871, `contract_for_role()`:
   Find the `Err(ContractLoadError::MissingAsset { .. })` arm. Replace the
   `AgentContract::permissive(...)` return with `AgentContract::restricted(role)`.
   Add `tracing::warn!(role, "missing safety contract YAML, using restricted defaults")`.

**Verify**:
```bash
rg 'permissive\(' crates/roko-agent/src/safety/mod.rs -n
# should only appear in test code or explicit test helpers
cargo test -p roko-agent safety --lib
```

**Do not**: Change safety check logic. Do not remove `permissive()` from the API (tests need it). Do not change tool allowlists.

---

# TIER 2: Delete Dead Code

6 items. Pure subtraction. Net negative LOC. ~1 session.

---

## [x] T2-16: Delete 4 orphan learn files (not compiled)

**Why**: Files exist but are not in `lib.rs`. Rustc never compiles them.

**Files to DELETE**:
- `crates/roko-learn/src/resonant_patterns.rs`
- `crates/roko-learn/src/signal_metabolism.rs`
- `crates/roko-learn/src/shapley.rs`
- `crates/roko-learn/src/kalman.rs`

**Pre-check**: Confirm not in lib.rs:
```bash
rg 'mod resonant_patterns|mod signal_metabolism|mod shapley|mod kalman' crates/roko-learn/src/lib.rs
# must return 0 matches
```

**Verify**:
```bash
cargo check -p roko-learn
cargo test -p roko-learn
```

---

## [x] T2-17: Delete 8 unused learn modules (6 kept due to internal callers)

**Why**: Exported from `lib.rs`, compile, but zero external callers. ~4K+ LOC of
unreachable infrastructure.

**Modules** (all in `crates/roko-learn/src/lib.rs`):
`adversarial`, `adas`, `calibration_policy`, `causal`, `reinforce_kind`,
`research_pipeline`, `regression`, `bandit_research`, `forensic_replay`,
`drift`, `local_reward`, `section_outcome`, `post_gate_reflection`, `verdict_scorer`

**Pre-check for each**:
```bash
rg 'roko_learn::MODULE_NAME' crates/ -g '*.rs' --glob '!crates/roko-learn/'
# must return 0 matches for each module
```

**What to change**: Remove `pub mod MODULE_NAME;` from `lib.rs` for each.
Delete the corresponding `.rs` files or directories.

**Verify**:
```bash
cargo check --workspace
cargo test --workspace
```

**Do not**: Wire unused modules into runtime to justify them. Do not delete modules with callers (check first).

---

## [x] T2-18: Remove 7 phantom config sections

**Why**: Defined in config structs, present in roko.toml, zero runtime reads.

**Sections to remove**: `OneirographyConfig`, `DemurrageConfig`, `AttentionConfig`,
`ImmuneConfig`, `TemporalConfig`, `GoalsConfig`, `EnergyConfig`

**Files**:
- `crates/roko-core/src/config/tools.rs` — `OneirographyConfig` (lines 124-135)
- `crates/roko-core/src/config/learning.rs` — `DemurrageConfig` (116), `AttentionConfig` (194), `ImmuneConfig` (239), `TemporalConfig` (284), `GoalsConfig` (320)
- `crates/roko-core/src/config/budget.rs` — `EnergyConfig` (53)
- `crates/roko-core/src/config/schema.rs` — field declarations on `RokoConfig`
- `roko.toml` — section entries
- `crates/roko-core/src/config/hot_reload.rs` — remove diff comparisons for these

**Pre-check per section**:
```bash
rg 'config\.oneirography|cfg\.oneirography' crates/ -g '*.rs'
# must return 0 runtime reads (only schema/reload/example)
```

**Verify**:
```bash
cargo check --workspace
```

**Do not**: Remove `[conductor]` (some fields ARE used). Do not add `#[deprecated]` (just delete).

---

## [x] T2-19: Remove 6 phantom conductor fields

**Why**: 6 of 12 `ConductorConfig` fields are never read by orchestration/runner.

**Fields to remove from `ConductorConfig`** (schema.rs lines 1051-1095):
- `auto_advance_batch` (TUI display only)
- `auto_merge_on_complete` (TUI display only)
- `pre_plan` (example TOML only)
- `conductor_model` (compat migration only)
- `warm_implementers_per_plan` (example TOML only)
- `enabled_roles` (TUI display only)

**Keep**: `max_agents`, `max_parallel_plans`, `parallel_enabled`, `express_mode`,
`max_auto_fix_attempts`, `auto_fix_model`, `watchers`

**Also update**: TUI `config_meta.rs` (remove display rows), `compat.rs` (remove
migration refs), `roko.toml` (remove entries).

**Verify**:
```bash
cargo check --workspace
```

---

## [x] T2-20: Remove write-only sinks (conductor, dreams)

**Why**: `ConductorObservationSink` writes to a JSONL nobody reads. `DreamTriggerSink`
writes triggers no worker drains. Both have zero consumers.

**Files to DELETE**:
- `crates/roko-cli/src/runtime_feedback/conductor.rs`
- `crates/roko-cli/src/runtime_feedback/dreams.rs`

**Also change**:
- `crates/roko-cli/src/runtime_feedback/mod.rs` — remove `pub mod conductor;` and `pub mod dreams;`
- `crates/roko-cli/src/commands/plan.rs` — remove `ConductorObservationSink` (line 399) and `DreamTriggerSink` (line 402) from `FeedbackFacade` construction

**Verify**:
```bash
rg 'ConductorObservationSink|DreamTriggerSink' crates/ -g '*.rs'  # should be 0
cargo check -p roko-cli --lib
```

**Do not**: Remove `EpisodeSink`, `RoutingObservationSink`, or `KnowledgeIngestionSink`. Do not remove conductor/dream subsystems themselves.

---

## [x] T2-21: Remove phantom agent config fields

**Why**: `policy_manifests` (line 70), `domain` (line 90) are never read.
`data_llm` (line 72) has an implementation but is never wired from orchestrate.rs.

**File**: `crates/roko-core/src/config/agent.rs`

**What to change**:
- Remove `pub policy_manifests: Vec<String>` field and its Default value.
- Remove `pub domain: Option<String>` field and its Default value.
- For `data_llm`: add doc comment `/// Reserved for future CaMeL dual-LLM isolation. Not wired.`
  (keep the field since `DataLlmRouter` implementation is substantial).

**Verify**:
```bash
cargo check --workspace
```

**Do not**: Remove `data_llm.rs` implementation. Do not remove used fields (mcp_config, backends, etc).

---

# TIER 3: Security Hardening

7 items. Required before non-localhost deployment. ~2 sessions.

---

## [x] T3-22: Flip auth default to enabled

**File**: `crates/roko-core/src/config/serve.rs` lines 83-91
**Change**: `enabled: false` → `enabled: true` in `ServeAuthConfig::default()`
**Also**: Update `roko init` template to include `serve.auth.enabled = false` with
a comment `# disable auth for local development only` so existing local workflows
aren't broken.

**Verify**: `cargo test --workspace` (update tests that assume auth disabled)

---

## [x] T3-23: Add rate limiting

**File**: `crates/roko-serve/src/routes/mod.rs`
**Add**: `tower::limit::RateLimitLayer` or `governor` middleware. Global: 100 req/sec.
Per-endpoint overrides for `POST /api/terminal/sessions` (5/min),
`POST /api/inference/complete` (30/min), `POST /api/agents/register` (10/min).

**Verify**: Test that exceeding limits returns 429.

---

## [x] T3-24: Add request body size limits

**File**: `crates/roko-serve/src/routes/mod.rs`
**Add**: `DefaultBodyLimit::max(4 * 1024 * 1024)` (4 MiB global).
Webhook `Bytes` extractors: 1 MiB explicit cap.

**Verify**: Send >4 MiB body, assert 413.

---

## [x] T3-25: Require explicit opt-in for non-loopback bind

**File**: `crates/roko-serve/src/lib.rs` lines 231-237
**Change**: When `PORT` env is set, override port only, not bind address. Keep
`127.0.0.1` unless `serve.bind` is explicitly set to `0.0.0.0` in config.

**Verify**: `PORT=8080` without `serve.bind = "0.0.0.0"` binds to `127.0.0.1:8080`.

---

## [x] T3-26: Add WebSocket message size limits

**File**: `crates/roko-serve/src/routes/ws.rs` line 29-31
**Change**: Add `.max_message_size(1024 * 1024)` and `.max_frame_size(256 * 1024)`
to all WebSocket upgrade calls.

**Verify**: Oversized WS message causes connection close.

---

## [x] T3-27: Fix path traversal + TOML injection in agent creation

**File**: `crates/roko-serve/src/routes/agents.rs`
**Change**: (a) Canonicalize agent paths, verify within workspace root.
(b) Replace string-interpolated TOML with `toml::to_string_pretty(&manifest_struct)`.
**Test**: Agent name `"../../../etc"` rejected. Prompt with `\n[malicious]\n` doesn't inject.

**Verify**: `cargo test -p roko-serve agent_manifest --lib`

---

## [x] T3-28: Restrict CORS methods/headers

**File**: `crates/roko-serve/src/routes/middleware.rs` lines 432-463
**Change**: Replace `allow_methods(Any)` with explicit `[GET, POST, PUT, DELETE, PATCH, OPTIONS]`.
Replace `allow_headers(Any)` with explicit `[CONTENT_TYPE, AUTHORIZATION, X_API_KEY, ...]`.

**Verify**: Preflight with non-standard method/header is rejected.

---

# TIER 4: Feedback Loop Completion

6 items. Turn write-only paths into real learning loops. ~2-3 sessions.

---

## [ ] T4-29: Wire KnowledgeIngestionSink.with_ingestor()

**Depends on**: T0-6
**File**: `crates/roko-cli/src/commands/plan.rs` line 396
**Change**: Construct a `KnowledgeIngestor` from the neuro store and call
`.with_ingestor(ingestor)` on the sink.

**Verify**: After plan run, `roko knowledge query` returns new entries.

---

## [ ] T4-30: Thread real RoutingContext through dispatch

**Depends on**: T1-8
**File**: `crates/roko-cli/src/runtime_feedback/routing.rs`
**Change**: After T1-8 provides real model/provider, construct `RoutingContext` from
dispatch metadata (model, provider, task complexity, budget pressure, selection reason).
Call `observe_multi_objective()` instead of `record_confidence_outcome("")`.

**Verify**: After 50+ observations, `roko learn router` shows Stage 2 progression.

---

## [ ] T4-31: Migrate provider parsers to UsageObservation

**Files**: `crates/roko-agent/src/` provider adapters (one per commit)
**Change**: Parse usage into `UsageObservation` with `Option<u64>` fields. Absent = `None`. Zero = `Some(0)`.
**Providers**: Anthropic, Ollama, Gemini, Cerebras, Cursor (one per commit).

**Verify per provider**: Tests distinguish absent vs zero.

---

## [ ] T4-32: Wire playbook store into system prompt builder

**File**: `crates/roko-cli/src/orchestrate.rs` (dispatch_agent_with prompt assembly section)
**Change**: Pass playbook query results into `SystemPromptBuilder` enrichment.

**Verify**: Dispatch with matching playbooks includes them in system prompt.

---

## [ ] T4-33: Add JSONL rotation for episodes/efficiency

**Files**: Episode/efficiency JSONL writers
**Change**: Before append, check file size. If >10 MB, rename to `.jsonl.1` and start new.
Keep last 5 rotated files.

**Verify**: Test writes until threshold, verify rotation.

---

## [ ] T4-34: Make /model switch atomic

**File**: `crates/roko-cli/src/chat_inline.rs` (line ~2761)
**Change**: Resolve into temp struct. On success, commit all fields. On failure,
leave ALL fields unchanged and show error. Remove partial-update fallback.

**Verify**: Failed `/model` leaves `model_selection.backend_slug` unchanged.

---

# TIER 5: Architectural Extraction

8 items. Restructure on clean, smaller codebase. ~3-5 sessions.

---

## [ ] T5-35: Extract dispatch_agent_with into composable units

**File**: `crates/roko-cli/src/orchestrate.rs` lines 14554-16613 (2,059 lines)
**Change**: One extraction per commit:
1. `select_model()` — steps 4-6 (~335 lines)
2. `build_prompt()` — steps 14-19 (~350 lines)
3. `launch_agent()` — step 24 (~330 lines)
4. `record_outcome()` — steps 25-31 (~295 lines)

**Rule**: Pure mechanical move. No logic changes. All tests pass after each commit.

---

## [ ] T5-36: Migrate remaining serve dispatch to ModelCallService

**Files**: `crates/roko-serve/src/routes/` (one endpoint per commit)
**Change**: Replace route-local HTTP construction with `ModelCallService::call/stream`.

**Verify per endpoint**: Same response. `rg reqwest::Client` count decreases.

---

## [ ] T5-37: Remove or quarantine dispatch_direct

**Depends on**: T5-36
**File**: `crates/roko-cli/src/` dispatch_direct module
**Change**: `#[deprecated]` + feature-gate behind `legacy-direct-dispatch`.

**Verify**: `rg 'dispatch_direct' crates/ -g '*.rs'` shows only definition + tests.

---

## [ ] T5-38: Collapse config into validated model

**File**: `crates/roko-core/src/config/`
**Change**: `load_config()` returns `ValidatedConfig` wrapper with provenance + semantic checks.

---

## [ ] T5-39: Add budget guardrail to Ollama dispatch path

**File**: `crates/roko-cli/src/orchestrate.rs` lines 15910-16011
**Change**: Wrap Ollama `ToolLoop` in `TaskRunner` or add `RunnerBudgetGuardrail`.

---

## [ ] T5-40: Replace event-replay reports with RunLedger

**Files**: `crates/roko-runtime/src/`, `crates/roko-cli/src/orchestrate.rs`
**Change**: One source per commit: gates → ledger, artifacts → ledger, events → ledger, resume → ledger.

---

## [ ] T5-41: Migrate demo automation off prompt scraping

**File**: `demo/demo-app/src/lib/scenario-runners/`
**Change**: Consume `CommandEvent` lifecycle events. Success = exit code, not regex.

---

## [ ] T5-42: Provider-native structured history for all adapters

**Files**: `crates/roko-agent/src/` (one adapter per commit)
**Change**: Accept `Vec<Message>`, convert to provider-native format. Start with Anthropic.

---

# Progress Summary

| Tier | Items | Done | Description |
|------|-------|------|-------------|
| T0 | 7 | 7 | Stop active bleeding (security + data) |
| T1 | 8 | 8 | Fix silent data corruption |
| T2 | 6 | 6 | Delete dead code (~5K+ LOC) |
| T3 | 7 | 7 | Security hardening |
| T4 | 6 | 0 | Feedback loop completion |
| T5 | 8 | 0 | Architectural extraction |
| **Total** | **42** | **28** | |

Note: T0/T1 verified by audit checklist. T2 and T3 cherry-picked from
`t2-deadcode-cleanup` and `wp-arch2-t3-security-1777605153` branches
into `wp-arch2` on 2026-05-01 with no merge conflicts; the audit-specific
verifications (`shared_run_rejects_path_traversal`,
`validate_agent_url_rejects_internal_and_private_hosts`,
`mask_secret_fields_redacts_extended_secrets`,
`generic_webhook_requires_auth_when_enabled`,
`agent_manifest_create_rejects_traversal_name`,
`agent_manifest_prompt_cannot_inject_toml_table`,
`cors_preflight_rejects_disallowed_method`,
`cors_preflight_rejects_disallowed_header`,
`cors_preflight_allows_listed_method_and_header`) all pass.
T4 and T5 remain open.
