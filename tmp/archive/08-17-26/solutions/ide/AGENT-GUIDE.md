# IDE Integration — Agent Execution Guide

## For Agents: Read This First

You are implementing fixes to `roko acp`, a JSON-RPC stdio server consumed by the Nunchi IDE.
All work is in the roko monorepo at `/Users/will/dev/nunchi/roko/roko/`.

**Critical rules:**
1. Read your batch file COMPLETELY before making any changes
2. Do NOT run `cargo build`, `cargo test`, `cargo clippy`, or `cargo fmt` — defer ALL compilation to Phase 2
3. Do NOT modify files outside your batch's scope
4. Use `grep -rn 'search_term' crates/ --include='*.rs' | grep -v target/` to find code
5. After making changes, re-read the modified file to verify correctness

## How the Codebase Is Structured

```
/Users/will/dev/nunchi/roko/roko/
├── Cargo.toml                     ← workspace manifest
├── crates/
│   ├── roko-core/src/config/      ← Config types (RokoConfig, ModelProfile, ProviderConfig)
│   │   ├── schema.rs              ← Main config struct
│   │   ├── provider.rs            ← ModelProfile, ProviderConfig
│   │   ├── loader.rs              ← Config loading, merging, diagnostics
│   │   ├── agent.rs               ← AgentConfig (has bare_mode)
│   │   └── registry.rs            ← ModelRegistry
│   ├── roko-core/src/defaults.rs  ← Constants (DEFAULT_MAX_OUTPUT_TOKENS, etc.)
│   ├── roko-acp/src/              ← THE ACP SERVER (main target)
│   │   ├── types.rs               ← All JSON-RPC wire types
│   │   ├── session.rs             ← Session management, config building, slash commands
│   │   ├── handler.rs             ← JSON-RPC method dispatch
│   │   ├── bridge_events.rs       ← LLM dispatch, MCP setup, streaming
│   │   └── transport.rs           ← Stdio transport (read/write JSON lines)
│   ├── roko-agent/src/            ← LLM dispatch (Anthropic, OpenAI, Gemini, etc.)
│   ├── roko-cli/src/              ← CLI entry point
│   ├── roko-learn/src/            ← Learning subsystem
│   └── roko-serve/src/            ← HTTP control plane
└── tmp/solutions/ide/tests/       ← Integration tests for ACP
```

## Key Types You'll Encounter

```rust
// roko-core/src/config/schema.rs
pub struct RokoConfig {
    pub providers: HashMap<String, ProviderConfig>,  // ← becoming IndexMap
    pub models: HashMap<String, ModelProfile>,       // ← becoming IndexMap
    pub agent: AgentConfig,                          // has .bare_mode, .default_model
    // ...30+ other fields
}

// roko-acp/src/types.rs
pub struct SessionNewParams { session_name, client_capabilities, mcp_servers }  // ← adding model/provider/effort
pub struct SessionNewResult { session_id, modes, config_options }                // ← adding warnings
pub struct SlashCommand { name, description, input }                             // ← adding category
pub struct ConfigOptionValue { value, name, description }                        // ← adding ready
pub struct McpServerConfig { name, transport }                                   // ← adding discovery_timeout_ms

// roko-acp/src/session.rs
pub struct SessionConfigState { agent_mode, provider, model, effort, ... }
fn from_roko_config(config) -> Self  // picks default model/provider
fn create_session(params) -> Result  // creates session from params
fn build_config_options(state, config) -> Vec<ConfigOption>  // builds provider/model dropdowns
fn build_slash_commands() -> Vec<SlashCommand>  // 47 commands, no filtering

// roko-acp/src/bridge_events.rs
enum CognitiveEvent { TokenChunk, ThinkingChunk, ToolCallStart, ToolCallComplete, PlanUpdate, Complete, Failure, MaxTokens }
fn setup_session_mcp_tools(session_id, mcp_servers, event_sender) -> SessionMcpRuntime
fn map_event_to_update(event: CognitiveEvent) -> SessionUpdate  // converts events to JSON
```

## Execution Protocol

### Phase 1: Code Changes (ALL agents work here, NO compilation)

Groups run sequentially (0 → 1 → 2 → 3 → 4). Within each group, agents run in parallel.

| Group | Agents | What | Approx Time |
|-------|--------|------|-------------|
| 0 | 1 | Add indexmap to all Cargo.tomls | 10 min |
| 1 | 1 | Change schema.rs + registry.rs types | 15 min |
| 2 | 6 parallel | IndexMap refs (A,B,C) + SessionNew (D) + Fallback (E) + MCP types (F) | 30 min |
| 3 | 4 parallel | MCP accumulation (G) + Commands (H) + max_output (I) + readiness (J) | 30 min |
| 4 | 1 | MCP notification + timeout config | 30 min |

**Important: Groups 2 and 3 have file overlaps in types.rs and session.rs, but they modify
DIFFERENT sections of those files (different line ranges).** As long as agents don't touch
lines outside their batch scope, the changes compose cleanly.

### Phase 2: Compilation + Fix (1 agent)

After ALL Phase 1 code changes are committed/staged:

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo +nightly fmt --all
cargo build --workspace 2>&1 | head -200
# Fix errors iteratively (usually: missing imports, type mismatches from IndexMap)
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace 2>&1 | tail -50
```

Common issues to expect:
- Missing `use indexmap::IndexMap;` in a file
- Lifetime issues with IndexMap (unlikely — same API as HashMap)
- New fields not initialized in test fixtures (add `warnings: Vec::new()`, `category: None`, `ready: false`)

### Phase 3: Integration Test (1 agent)

```bash
cd /Users/will/dev/nunchi/roko/roko/tmp/solutions/ide/tests
bash run-all.sh
```

If tests still fail, investigate and fix.

## Corrected Facts (IMPORTANT — these override the original 01-11 .md files)

| Original Claim (in 01-11 docs) | Truth (from code investigation) |
|---|---|
| "max_output default is 900 tokens" | `ModelProfile::max_output` is `Option<u64>` defaulting to `None`. At runtime, `None` falls back to `DEFAULT_MAX_OUTPUT_TOKENS = 16,384` (in `roko-core/src/defaults.rs:32`). There is no 900-token limit. |
| "bare_mode suppresses workspace features in ACP" | `bare_mode` in `AgentConfig` (agent.rs:46) defaults to `true` and was meant to pass `--bare` to the Claude CLI binary. But Claude CLI removed `--bare` (noted in claude_cli_agent.rs:318). In `roko-acp`, `build_slash_commands()` **never checks bare_mode**. We must add this filtering. |
| "MCP discovery timeout is 10 seconds" | `DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS = 5` (defaults.rs:241). Applied separately to `initialize()` and `list_tools()` — so worst case is 10s per server, but the constant is 5. |
| "Session MCP tools work for all providers" | `openai_compat_tool_loop_supported()` only returns true for `ProviderKind::OpenAiCompat`, `PerplexityApi`, `CerebrasApi`. Anthropic API and Claude CLI skip the MCP tool loop entirely. |
| "CognitiveEvent has many variants" | It has exactly 8: TokenChunk, ThinkingChunk, ToolCallStart, ToolCallComplete, PlanUpdate, Complete, Failure, MaxTokens. Terminal events (Complete/Failure/MaxTokens) are handled before `map_event_to_update` is called. |
| "SessionNewResult has configOptions field" | The field is `config_options: Option<Vec<ConfigOption>>` (optional, serde camelCase = "configOptions"). It is NOT guaranteed to be present. |

## What Each Batch File Contains

Every batch file in `batches/` has:
1. **Context** — what the change is and why
2. **Prerequisites** — which other batches must be done first
3. **File Locations** — absolute paths to every file modified
4. **Exact Changes** — FIND/REPLACE blocks with verbatim source code
5. **What NOT to Change** — explicit scope boundaries
6. **Verification** — how to confirm correctness after Phase 2

## Troubleshooting

**"I can't find the code at the specified line number"**
Line numbers may shift if earlier batches modified the same file. Use the FIND text as a
search string instead of relying on line numbers. The FIND blocks are verbatim code that
should match exactly (unless another batch already modified that section).

**"IndexMap doesn't have method X"**
IndexMap 2.x implements the same API as HashMap: `.get()`, `.insert()`, `.entry()`, `.iter()`,
`.keys()`, `.values()`, `.contains_key()`, `.len()`, `.is_empty()`, `.clone()`, `.into_iter()`.
If you get a method-not-found error, check if it's a HashMap-specific method like `.drain()`
(use `.drain(..)` on IndexMap instead).

**"Serde doesn't compile for IndexMap"**
The `features = ["serde"]` flag on the indexmap dependency enables serde support. Make sure
the workspace Cargo.toml has `indexmap = { version = "2", features = ["serde"] }`.

**"build_slash_commands() has different arguments than expected"**
If W3-A (command categories) and W1-A (session params) are being applied to the same session.rs,
make sure you're modifying different functions. build_slash_commands is at line ~1116.
create_session is at line ~753. They don't overlap.
