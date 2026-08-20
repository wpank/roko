# 76 — Example Configuration Quality (Issues Across Provider and Graph Examples)

**Priority**: P1 — significant new-user friction; examples are the primary onboarding material
**Size**: M (2-3 days)
**Crates**: `examples/` (all changes are in example/doc files; no Rust code changes required)
**Depends on**: None

---

## Background

The `examples/` directory at `/Users/will/dev/nunchi/roko/roko/examples/` contains the
primary onboarding material for new users configuring roko: 8 provider TOML files
(`roko-perplexity.toml`, `roko-ollama.toml`, `roko-openrouter.toml`, `roko-lmstudio.toml`,
`roko-glm.toml`, `roko-kimi.toml`, `roko-gemini.toml`, `roko-multi-provider.toml`) and 3
markdown guides (`adding-a-custom-protocol.md`, `adding-a-provider.md`,
`adding-custom-tools.md`).

These examples have accumulated several issues: silently-ignored TOML fields, a stale
hardcoded development path that doesn't exist, a wrong port number in curl examples, an
incomplete `ProviderKind` listing in a guide, and missing examples for two fully-supported
provider kinds. A new user who copies any of these examples verbatim may encounter
surprising behavior.

## Current State

1. **Silently-ignored `[prompt]` fields.** Five example TOML files contain a `[prompt]`
   section with `token_budget` and `role` fields:
   - `/Users/will/dev/nunchi/roko/roko/examples/roko-perplexity.toml` (lines 21-23):
     `token_budget = 12000` and `role = "You are a Roko agent..."`
   - `/Users/will/dev/nunchi/roko/roko/examples/roko-ollama.toml` (lines 21-23): same fields
   - `/Users/will/dev/nunchi/roko/roko/examples/roko-openrouter.toml`: check for `[prompt]` section
   - `/Users/will/dev/nunchi/roko/roko/examples/roko-lmstudio.toml`: check for `[prompt]` section
   - `/Users/will/dev/nunchi/roko/roko/examples/roko-glm.toml`: check for `[prompt]` section
   - `/Users/will/dev/nunchi/roko/roko/examples/roko-kimi.toml`: check for `[prompt]` section

   The actual `PromptConfig` struct in
   `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` (lines 208-231)
   has exactly two fields: `composition_strategy: ConfigCompositionStrategy` and
   `vcg_warmup_observations: u32`. It has no `token_budget` or `role` fields. Serde uses
   `#[serde(default)]` on the struct, which means unknown fields are silently ignored. Users
   who expect these fields to do something will be confused.

2. **Stale hardcoded path in `adding-custom-tools.md`.** Lines 26-28 of
   `/Users/will/dev/nunchi/roko/roko/examples/adding-custom-tools.md` reference two absolute
   paths:
   - `/Users/will/dev/nunchi/roko/roko-mr-stream-beta/tmp/implementation-plans/modelrouting/18-structural-cleanup.md`
   - `/Users/will/dev/nunchi/roko/roko-mr-stream-beta/tmp/implementation-plans/modelrouting/01-architecture.md`
   The `roko-mr-stream-beta/` directory does not exist (it was a development workspace that
   was never committed). Any user who tries to follow these references will get file-not-found
   errors. The section that references these paths is the "Context files (read these first)"
   section, which is agent-task boilerplate that leaked into user documentation.

3. **Incomplete `ProviderKind` listing in `adding-a-custom-protocol.md`.** Lines 41-48 of
   `/Users/will/dev/nunchi/roko/roko/examples/adding-a-custom-protocol.md` show an example
   `ProviderKind` enum with 7 variants: `AnthropicApi`, `ClaudeCli`, `OpenAiCompat`,
   `CursorAcp`, `PerplexityApi`, `GeminiApi`, and `YourProviderApi`.
   The actual `ProviderKind` enum in
   `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/agent.rs` (lines 59-84) has 11
   variants: `AnthropicApi`, `ClaudeCli`, `OpenAiCompat`, `CursorAcp`, `PerplexityApi`,
   `GeminiApi`, `GeminiCli`, `CerebrasApi`, `CursorCli`, `Hermes`, `OpenClaw`. The guide
   is missing `GeminiCli`, `CerebrasApi`, `CursorCli`, `Hermes`, and `OpenClaw`. A user
   reading this guide gets an incomplete picture of the available providers.

4. **Wrong port in `adding-a-provider.md`.** Lines 168-170 of
   `/Users/will/dev/nunchi/roko/roko/examples/adding-a-provider.md` show:
   ```
   roko serve --port 9090
   curl http://127.0.0.1:9090/api/providers
   curl http://127.0.0.1:9090/api/models
   ```
   The default roko serve port is `6677` (confirmed by `--arg default_value = "http://localhost:6677"`
   in `crates/roko-cli/src/main.rs` line 489). The example should use `6677` or omit the
   `--port` flag entirely and show the default port in the curl command.

5. **No `roko-anthropic-api.toml` example.** There is no example for `kind = "anthropic_api"`,
   which is the standard headless/CI path (no `claude` CLI binary needed). This is the most
   common alternative to `claude_cli` for server-side deployments.

6. **No `roko-cerebras.toml` example.** `CerebrasApi` is a fully registered and tested
   provider kind in `crates/roko-agent/src/provider/cerebras.rs` with a hardcoded default
   base URL, but no example TOML exists for users who want to use it.

7. **`roko-gemini.toml` uses preview model slugs with no availability caveat.** Lines 87-89,
   106-108, and 122-124 reference `gemini-3.1-pro-preview`, `gemini-3-flash-preview`, and
   `gemini-3.1-flash-lite-preview`. These preview slugs may be removed or renamed by Google
   at any time. The example should note that preview slugs are best-effort and suggest using
   GA slugs for production.

8. **`roko-ollama.toml` and `roko-kimi.toml` have `fallback_model` references.** The
   `roko-ollama.toml` file has `fallback_model = "glm-5-1"` at line 12. If a user copies
   only `roko-ollama.toml`, the fallback model `"glm-5-1"` is undefined in that file,
   causing silent routing failures. Same may apply to `roko-kimi.toml`; verify by reading it.

## Implementation Plan

### Fix 1: Remove silently-ignored `[prompt]` fields from all example TOMLs

For each of the example TOML files that contains `[prompt]` with `token_budget` or `role`:

1. Read the file to confirm which fields are present.
2. Remove the `[prompt]` section entirely if it only contains `token_budget` and `role`.
3. If the intent of `role` was to configure agent role behavior, replace it with the correct
   field. Based on the `PromptConfig` struct, there is no `role` field; the agent persona
   should be set in the system prompt at the task or template level, not in config. Add a
   comment explaining how to achieve the same intent.
4. If the intent of `token_budget` was to cap token usage, replace with the correct approach:
   budget caps are set in the `[budget]` section, not `[prompt]`.

Affected files:
- `/Users/will/dev/nunchi/roko/roko/examples/roko-perplexity.toml`
- `/Users/will/dev/nunchi/roko/roko/examples/roko-ollama.toml`
- Verify and fix: `roko-openrouter.toml`, `roko-lmstudio.toml`, `roko-glm.toml`, `roko-kimi.toml`

### Fix 2: Remove stale hardcoded paths from `adding-custom-tools.md`

In `/Users/will/dev/nunchi/roko/roko/examples/adding-custom-tools.md`, remove the entire
"Context files (read these first)" section (lines 25-28). This section was agent-task
boilerplate that should not appear in user documentation. The guide stands on its own
without it.

If the intent was to point users to architecture documentation, replace with a reference to
`CLAUDE.md` or `crates/roko-agent/src/mcp/` instead.

### Fix 3: Update `ProviderKind` listing in `adding-a-custom-protocol.md`

In `/Users/will/dev/nunchi/roko/roko/examples/adding-a-custom-protocol.md`, update the
`ProviderKind` enum example in Step 1 (around lines 41-48) to include all 11 current
variants. Reference the actual source:
`/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/agent.rs` lines 59-84.

The updated enum should list all 11 variants with their serde serialized names:

```rust
pub enum ProviderKind {
    AnthropicApi,    // "anthropic_api"
    ClaudeCli,       // "claude_cli"
    OpenAiCompat,    // "openai_compat"
    CursorAcp,       // "cursor_acp"
    PerplexityApi,   // "perplexity_api"
    GeminiApi,       // "gemini_api"
    GeminiCli,       // "gemini_cli"
    CerebrasApi,     // "cerebras_api"
    CursorCli,       // "cursor_cli"
    Hermes,          // "hermes"
    OpenClaw,        // "open_claw"
    YourProviderApi, // "your_provider_api" (the new one you are adding)
}
```

Also update the `label()` match and the `adapter_for_kind()` example to include the 11
existing variants.

### Fix 4: Fix port number in `adding-a-provider.md`

In `/Users/will/dev/nunchi/roko/roko/examples/adding-a-provider.md`, update all curl
examples that use port 9090 to use port 6677, or show the default (`roko serve` without
`--port`) and use `http://127.0.0.1:6677` in curl commands.

Specifically, update lines 168-170 (and line 183 if it also uses 9090):
```
roko serve --port 9090
curl http://127.0.0.1:9090/api/providers
```
Becomes:
```
roko serve
curl http://127.0.0.1:6677/api/providers
```

### Fix 5: Create `examples/roko-anthropic-api.toml`

Create `/Users/will/dev/nunchi/roko/roko/examples/roko-anthropic-api.toml` showing the
headless HTTP API configuration. Minimum viable example:

```toml
schema_version = 2

[project]
name = "roko-anthropic-api"
root = "."
fresh_base_branch = "main"

[agent]
default_model = "claude-sonnet"
effort = "high"
timeout_ms = 300000

[providers.anthropic]
kind = "anthropic_api"
base_url = "https://api.anthropic.com/v1"
api_key_env = "ANTHROPIC_API_KEY"
timeout_ms = 300000

[models.claude-sonnet]
provider = "anthropic"
slug = "claude-sonnet-4-6"
context_window = 200000
max_output = 16000
supports_tools = true
supports_thinking = true
supports_vision = true
supports_caching = true
tool_format = "anthropic_blocks"
cost_input_per_m = 3.00
cost_output_per_m = 15.00
cost_cache_read_per_m = 0.30
cost_cache_write_per_m = 3.75

[models.claude-haiku]
provider = "anthropic"
slug = "claude-haiku-4-5"
context_window = 200000
max_output = 8192
supports_tools = true
tool_format = "anthropic_blocks"
cost_input_per_m = 0.80
cost_output_per_m = 4.00
```

### Fix 6: Create `examples/roko-cerebras.toml`

Create `/Users/will/dev/nunchi/roko/roko/examples/roko-cerebras.toml` for the Cerebras
ultra-fast inference provider. Confirm the default base URL from
`crates/roko-agent/src/provider/cerebras.rs` before writing.

### Fix 7: Add preview model availability caveat to `roko-gemini.toml`

In `/Users/will/dev/nunchi/roko/roko/examples/roko-gemini.toml`, add a comment above each
preview model entry (e.g., `gemini-3.1-pro-preview`) noting that the slug is a preview and
may change. Suggest checking https://ai.google.dev/models for the current stable slug.

### Fix 8: Fix `fallback_model` self-containment

In `/Users/will/dev/nunchi/roko/roko/examples/roko-ollama.toml`, change
`fallback_model = "glm-5-1"` (line 12) to either:
- Remove the `fallback_model` line (safest for a standalone example)
- Change it to reference a model defined in the same file (e.g., `fallback_model =
  "llama3-1-8b"`)

Check `roko-kimi.toml` for the same issue and apply the same fix.

## Acceptance Criteria

1. No example TOML file contains `[prompt].token_budget` or `[prompt].role`.
2. `adding-custom-tools.md` contains no references to non-existent paths or the
   `roko-mr-stream-beta/` directory.
3. `adding-a-custom-protocol.md` lists all 11 current `ProviderKind` variants.
4. All curl examples in `adding-a-provider.md` use port 6677 (not 9090).
5. `examples/roko-anthropic-api.toml` exists and passes `roko config validate`.
6. `examples/roko-cerebras.toml` exists and passes `roko config validate`.
7. `roko-ollama.toml` either has no `fallback_model` or references a model defined in the
   same file.
8. Running `roko config validate` on each example TOML file produces 0 errors and 0
   migration warnings.

## Verification Checklist

- [ ] `rg 'token_budget|^\s*role\s*=' examples/*.toml` returns no matches
- [ ] `grep -r 'roko-mr-stream-beta' examples/` returns no matches
- [ ] The `ProviderKind` enum in `adding-a-custom-protocol.md` has 12 entries (11 existing
  + 1 `YourProviderApi` placeholder)
- [ ] `grep '9090' examples/adding-a-provider.md` returns no matches
- [ ] `ls examples/roko-anthropic-api.toml examples/roko-cerebras.toml` confirms both files exist
- [ ] `roko config validate examples/roko-anthropic-api.toml` exits 0
- [ ] `roko config validate examples/roko-cerebras.toml` exits 0
- [ ] `roko config validate examples/roko-ollama.toml` exits 0 with no migration warnings
- [ ] `grep 'glm-5-1' examples/roko-ollama.toml` returns no matches (or fallback points to
  a model defined in the same file)

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/examples/roko-perplexity.toml` | Remove `[prompt]` section with `token_budget` and `role` fields |
| `/Users/will/dev/nunchi/roko/roko/examples/roko-ollama.toml` | Remove `[prompt]` section; fix `fallback_model` reference |
| `/Users/will/dev/nunchi/roko/roko/examples/roko-openrouter.toml` | Remove `[prompt]` section if present |
| `/Users/will/dev/nunchi/roko/roko/examples/roko-lmstudio.toml` | Remove `[prompt]` section if present |
| `/Users/will/dev/nunchi/roko/roko/examples/roko-glm.toml` | Remove `[prompt]` section if present |
| `/Users/will/dev/nunchi/roko/roko/examples/roko-kimi.toml` | Remove `[prompt]` section if present; fix `fallback_model` if needed |
| `/Users/will/dev/nunchi/roko/roko/examples/roko-gemini.toml` | Add preview model availability caveats |
| `/Users/will/dev/nunchi/roko/roko/examples/adding-custom-tools.md` | Remove "Context files (read these first)" section with stale paths |
| `/Users/will/dev/nunchi/roko/roko/examples/adding-a-custom-protocol.md` | Update `ProviderKind` listing to all 11 current variants |
| `/Users/will/dev/nunchi/roko/roko/examples/adding-a-provider.md` | Replace port 9090 with 6677 in curl examples |
| `/Users/will/dev/nunchi/roko/roko/examples/roko-anthropic-api.toml` (new) | Example for `kind = "anthropic_api"` headless HTTP configuration |
| `/Users/will/dev/nunchi/roko/roko/examples/roko-cerebras.toml` (new) | Example for `kind = "cerebras_api"` ultra-fast inference |

## Not in Scope

- Adding examples for every possible provider configuration (only the two most commonly
  needed missing ones: Anthropic API and Cerebras)
- Validating model slugs against live provider APIs
- Adding `[[gate]]` shell override examples (tracked separately in backlog)
- Adding `[routing]` section examples (tracked separately in backlog)
