Read `/Users/will/dev/nunchi/roko/roko/CLAUDE.md` first — it has critical rules you MUST follow.

Then read `/Users/will/dev/nunchi/roko/roko/tmp/SESSION-2026-04-08.md` for context on what was discovered and what's already been done.

# Your mission: Wire existing code into the runtime

This codebase has 177K LOC across 18 crates. Most modules are fully implemented and tested but **never called from the runtime**. Your job is to CONNECT existing code, not write new code.

## Before you start: READ these files to understand what exists

1. **The runtime entry points** (this is where your changes go):
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` — plan-driven loop, currently uses ExecAgent with no Claude flags
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs` — single-shot execution

2. **What already exists and needs WIRING (not reimplementing)**:
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs` — 6-layer SystemPromptBuilder, 593 lines, fully tested
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/` — 9 role templates (implementer.rs, reviewer.rs, scribe.rs, strategist.rs, task_impl.rs, integration.rs, quick.rs, common.rs)
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs` — SafetyLayer composing bash/git/network/path/rate_limit/scrub guards
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs` — ToolDispatcher with `.with_safety()` already wired
   - `/Users/will/dev/nunchi/roko/roko/crates/bardo-runtime/src/process.rs` — ProcessSupervisor for spawn/track/kill/reap
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_agent.rs` — ClaudeAgent (HTTPS), needs system prompt field added

3. **The reference implementation** (what mori does that roko should match):
   - `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` lines 2444-2620 — how mori spawns Claude with --tools, --settings, --append-system-prompt, --fallback-model, --effort, --mcp-config
   - `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` lines 427-500 — mori's role-specific system prompts
   - `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` lines 647-750 — mori's safety hooks (--settings JSON)

4. **Implementation plans** (detailed checklists):
   - `/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/01-agent-wiring.md`
   - `/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/02-system-prompt-integration.md`
   - `/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/03-safety-hooks.md`

## Execute these 7 tasks IN ORDER. Verify each before starting the next.

### Task 1: Add `system` field to ClaudeAgent

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_agent.rs`

The `MessagesRequest` struct has `{model, max_tokens, messages}` but no `system` field. Anthropic's API supports `system: string | array<content_block>`.

Do:
- Add `#[serde(skip_serializing_if = "Option::is_none")] system: Option<String>` to `MessagesRequest`
- Add `system_prompt: Option<String>` field to `ClaudeAgent`
- Add `.with_system_prompt(prompt: impl Into<String>) -> Self` builder method
- Wire it into the `run()` method so `self.system_prompt` gets passed in the request
- Add a test that verifies the system prompt appears in the serialized request

Verify: `cargo test -p roko-agent`

### Task 2: Wire SystemPromptBuilder into orchestrate.rs

**Files**:
- Read: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs`
- Read: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/implementer.rs` (example)
- Modify: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`

Currently `orchestrate.rs` has an inline `role_system_prompt()` function (line ~660) returning 1-sentence strings like "You are an expert Rust software engineer." The `SystemPromptBuilder` already exists with 6 composable layers and does exactly what's needed.

Do:
- Replace the inline `role_system_prompt()` with calls to `SystemPromptBuilder`
- For each `AgentRole`, build a prompt using `SystemPromptBuilder::new(role_identity).with_conventions(...).with_anti_patterns(...)` etc.
- Use the role-specific content from the templates in `roko-compose/src/templates/` as reference for what each role needs
- The composed prompt should go into the agent via `--append-system-prompt` (for CLI agents) or the system field (for API agents)
- Add `roko-compose` to `roko-cli`'s Cargo.toml dependencies if not already there

Verify: `cargo test -p roko-cli` and `grep SystemPromptBuilder crates/roko-cli/src/orchestrate.rs`

### Task 3: Create ClaudeCliAgent

**Files**:
- Read: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` lines 2444-2620 (reference)
- Read: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/exec.rs` (current ExecAgent)
- Create: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_cli_agent.rs`

Create a new `ClaudeCliAgent` that wraps `claude` CLI with proper flags. This is NOT a rewrite of ExecAgent — it's a Claude-specific agent that builds a `Command` with:
- `--print --verbose --output-format stream-json`
- `--model <slug>`
- `--effort <level>`
- `--append-system-prompt <system_prompt>` (from SystemPromptBuilder)
- `--fallback-model <slug>`
- `--tools <role-allowlist>` (per-role tool restrictions)
- `--settings <json>` (safety hooks — see Task 4)
- `--dangerously-skip-permissions`
- `--bare` (when configured)
- `--mcp-config <path>` (when found)
- `--resume <session_id>` (when resuming)
- Environment: `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2`

Implement the `Agent` trait for it. Add it to `roko-agent/src/lib.rs`.

Verify: `cargo test -p roko-agent` and `cargo check -p roko-agent`

### Task 4: Build safety hooks JSON (--settings)

**Files**:
- Read: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` lines 647-750 (mori's hooks)
- Add to: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_cli_agent.rs`

Add a `fn build_settings_json() -> String` that produces the same PreToolUse hooks mori uses:
- Block `git checkout *`, `git switch *`, `git branch -m *`
- Block destructive filesystem operations

This JSON gets passed via `--settings` in the ClaudeCliAgent.

Verify: `cargo test -p roko-agent`

### Task 5: Wire ClaudeCliAgent into orchestrate.rs

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`

Replace `ExecAgent::new(...)` at line ~545 with `ClaudeCliAgent` when the config's agent command is "claude".

Do:
- Import `ClaudeCliAgent` from `roko-agent`
- In `dispatch_agent()`, check config: if command is "claude", use `ClaudeCliAgent`; otherwise fall back to `ExecAgent`
- Pass the system prompt from Task 2 via the agent
- Pass the role's tool allowlist

Verify: `cargo test -p roko-cli` and `grep -c ExecAgent crates/roko-cli/src/orchestrate.rs` (should be reduced)

### Task 6: Wire EpisodeLogger back into run.rs

**Files**:
- Read: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs`
- Modify: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`

Episode logging was previously wired into `run.rs` but got removed during a refactor. Re-add it:
- After agent run completes, persist an Episode to `.roko/memory/episodes.jsonl`
- Episode includes: prompt hash, output hash, gate verdicts, success, wall_ms
- Logging errors should be non-fatal (warn, don't fail)

Verify: `cargo test -p roko-cli`

### Task 7: Full verification

Run:
```bash
cargo test --workspace --exclude roko-demo
cargo clippy --workspace --exclude roko-demo --no-deps -- -D warnings
./scripts/verify-wiring.sh
```

Report which verify-wiring checks pass and which don't.

## Rules (from CLAUDE.md, repeated for emphasis)

1. **Search before writing**: `grep -rn 'StructName' crates/ --include='*.rs' | grep -v target/`
2. **Do NOT create new crates**
3. **Do NOT rewrite SystemPromptBuilder, SafetyLayer, ProcessSupervisor, or any existing module**
4. **Each task should be ~50-200 lines of changes, not 500+**
5. **Run tests after EVERY task**

