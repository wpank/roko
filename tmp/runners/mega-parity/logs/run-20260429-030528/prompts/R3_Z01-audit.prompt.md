# AUDIT: Batch R3_Z01 — Audit current dispatch and chat paths

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R3_Z01`.
Your job is to verify correctness and fix any issues — do NOT rewrite from scratch.

## Audit Checklist

1. **Compiles:** `cargo check -p <crate>` for each crate touched by this batch
2. **Clippy clean:** `cargo clippy -p <crate> --no-deps -- -D warnings`
3. **Prompt compliance:** Compare the implementation against the original prompt below
4. **No regressions:** Changed files don't break existing functionality
5. **Anti-patterns:** No stubs that silently pass, no inline prompts, no raw CLI spawns
6. **Correct types:** Field names, method signatures, and imports match the actual codebase
7. **Tests pass:** If the prompt required tests, verify they pass

## If You Find Issues

Fix them directly in the files. Then run the verification commands from the prompt.
If you cannot fix an issue, leave a comment in the file explaining why.

## Scope

Only touch files in the batch's write scope. Do NOT refactor unrelated code.

---

## Original Implementation Prompt

## Task

Audit current dispatch and chat paths

## Runner Context

You are working in runner `mega-parity`, batch R3_Z01.
This batch is part of Runner 3: agent-session-parity — Make interactive roko and one-shot use a real agent session via existing adapters.

## Problem

Three separate code paths handle LLM dispatch for interactive chat, one-shot prompts, and plan execution. They duplicate command construction, stream parsing, session tracking, and tool output collection. Before building `ChatAgentSession`, we need a precise map of what each path does, what it drops, and where the existing adapters already solve the problem.

Current paths:
- `chat_inline.rs` calls `dispatch_direct::dispatch_prompt` for interactive chat
- `unified.rs::cmd_oneshot_inline` calls `dispatch_direct::dispatch_via_model_call_service` then falls back to `dispatch_prompt`
- `orchestrate.rs` uses `ClaudeCliAgent` with full system prompt, tools, MCP, resume, and safety

The gap: `chat_inline.rs` builds its own command strings and parses stream-json independently of `ClaudeCliAgent`, which already handles all of that correctly. The session_id returned from each dispatch turn is extracted but never stored, so each turn starts a fresh Claude CLI session with no memory.

## Architecture Contract

This is a context-only audit batch. No code changes. Produce a reference document mapping exact call chains for all three dispatch paths.

## Step-by-Step Instructions

### Step 1: Verify chat_inline.rs ChatSession struct

Grep for the struct definition:

```bash
grep -n "struct ChatSession" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs
```

Expected: line 740. Read lines 740–764. Confirm:
- `phase: Phase` (line 741)
- `dispatch: DispatchMode` (line 749)
- `response_rx: Option<tokio::sync::mpsc::Receiver<...>>` (line 751)
- `turn_count: u32` (line 753)
- `conversation: Vec<ConversationMessage>` (line 757)
- `system_message: Option<String>` (line 761)
- `compact: bool` (line 763)
- **NO `session_id` field** — confirmed absent from lines 740–764.

### Step 2: Verify session_id extraction and discard

Grep for session_id handling in chat_inline.rs:

```bash
grep -n "session_id" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs
```

Expected output: line 3549 only: `session_id: None`.
This is in a test/helper, not the main response-handling path.

Grep for where dispatch result is processed:

```bash
grep -n "result\.text\|result\.model\|result\.input_tokens\|result\.session" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs | head -20
```

Expected: lines around 1356–1376 process `result.tool_outputs`, `result.text`, `result.model`,
`result.input_tokens`, `result.output_tokens` — but `result.session_id` is never accessed.

### Step 3: Verify dispatch_direct.rs command construction

Grep for the command built in dispatch_claude_cli:

```bash
grep -n "Command::new\|args.*print\|output-format\|session_id" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_direct.rs | head -20
```

Expected key lines:
- Line 141: `Command::new("claude")`
- Line 142: `.args(["--print", "--output-format", "stream-json"])`
- Line 166: `let mut session_id: Option<String> = None;`
- Line 206–208: session_id extracted from `"result"` event and set

Confirm missing flags by checking what is NOT present in dispatch_direct.rs:

```bash
grep -n "verbose\|model\|effort\|settings\|dangerously\|max.turns\|fallback.model\|append.system\|tools\|mcp.config\|resume\|env.*CARGO" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_direct.rs
```

Expected: zero matches. None of these flags are passed in `dispatch_claude_cli`.

### Step 4: Verify ClaudeCliAgent::build_command flags

Grep for every flag added in build_command:

```bash
grep -n "arg.*--\|--print\|--verbose\|--model\|--effort\|--settings\|--dangerously\|--max-turns\|--fallback-model\|--append-system-prompt\|--tools\|--mcp-config\|--strict-mcp\|--resume" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_cli_agent.rs | head -30
```

Expected lines from build_command (lines 302–359):
- 306: `--print`
- 307: `--verbose`
- 309: `--output-format stream-json`
- 310–311: `--model <model>`
- 312–313: `--effort <effort>`
- 314–315: `--settings <settings_json>`
- 316–318: `--dangerously-skip-permissions` (conditional)
- 319–321: `--max-turns <n>` (conditional)
- 323–327: `--fallback-model <slug>` (conditional)
- 328–330: `--append-system-prompt <text>` (conditional)
- 331–335: `--tools <csv>` (conditional)
- 336–338: `--mcp-config <path>` (conditional)
- 338: `--strict-mcp-config`
- 340–342: `--resume <session_id>` (conditional)

### Step 5: Verify unified.rs call chains

Grep for what unified.rs passes to chat_inline:

```bash
grep -n "chat_inline\|run_unified_inline\|detect_auth\|dispatch_via_model" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/unified.rs | head -20
```

Expected:
- Line 29: `let auth = detect_auth();`
- Line 64: `chat_inline::run_unified_inline(&auth)` — passes ONLY `&auth`, no config/workdir/model
- Line 85: `let auth = detect_auth();` (oneshot)
- Line 96: `dispatch_via_model_call_service(prompt).await`
- Line 100: fallback to `dispatch_prompt(&auth, prompt)`

### Step 6: Verify provider adapter routing

Grep for `create_agent_for_model` and adapter routing:

```bash
grep -n "create_agent_for_model\|adapter_for_kind\|ProviderKind::ClaudeCli\|AgentOptions" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs | head -20
```

Grep what ClaudeCliAdapter.create_agent accepts:

```bash
grep -n "system_prompt\|tools\|mcp_config\|effort\|env\|extra_args\|bare_mode\|dangerously" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli.rs | head -20
```

Expected: lines 59–79 of claude_cli.rs show ClaudeCliAdapter maps each `AgentOptions` field
to the corresponding `ClaudeCliAgent` builder method.

### Step 7: Write the context document

Create the directory and file:

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/tmp/runners/mega-parity/context
```

Write to `tmp/runners/mega-parity/context/R3_Z01_dispatch_audit.md` (the file you may modify).

The document MUST cover all six sections below:

```markdown
# R3_Z01: Dispatch and Chat Path Audit

## 1. Interactive Chat Call Chain (roko with no args)

1. `main.rs` → `cmd_unified_chat` (`unified.rs:23`)
2. `detect_auth()` → `AuthMethod` variant (`unified.rs:29`)
3. `chat_inline::run_unified_inline(&auth)` (`unified.rs:64`)
4. Creates `ChatSession` with `dispatch: DispatchMode::Direct { auth: auth.clone() }` (`chat_inline.rs:1289`)
5. On each user input: `dispatch_prompt(&mut session, prompt)` (`chat_inline.rs:1457`)
6. For `DispatchMode::Direct`: spawns `dispatch_direct::dispatch_prompt(&auth_clone, &text)` (`chat_inline.rs:1486`)
7. `dispatch_prompt` routes to `dispatch_claude_cli(prompt)` for `AuthMethod::ClaudeCli` (`dispatch_direct.rs:47`)
8. `dispatch_claude_cli` builds: `Command::new("claude").args(["--print", "--output-format", "stream-json"])` (`dispatch_direct.rs:141-142`)
9. Returns `DispatchResult { text, model, input_tokens, output_tokens, tool_outputs, session_id }` (`dispatch_direct.rs:271-278`)
10. Chat loop processes result at `chat_inline.rs:1356–1376`, accesses `.tool_outputs`, `.text`, `.model`, `.input_tokens`, `.output_tokens` — **never accesses `.session_id`**
11. `session_id` is dropped. Next turn starts a new Claude CLI session with no `--resume`.

## 2. One-Shot Call Chain (roko "prompt")

1. `main.rs` → `cmd_oneshot_inline(prompt, quiet)` (`unified.rs:84`)
2. `detect_auth()` → `AuthMethod` (`unified.rs:85`)
3. Tries `dispatch_via_model_call_service(prompt)` (`unified.rs:96`)
   - Uses `ModelCallService` from `roko-agent`, provides cost tracking
   - Returns `DispatchResult` with `session_id: None` (line 132 in dispatch_direct.rs)
4. On failure: falls back to `dispatch_prompt(&auth, prompt)` (`unified.rs:100`)
5. No system_prompt, no tools, no MCP, no session_id passed or retained

## 3. Plan Execution Call Chain (roko plan run)

1. `orchestrate.rs` constructs `ClaudeCliAgent::new(program, workdir, model)` with full builder chain
2. Sets: `with_system_prompt`, `with_tools`, `with_mcp_config`, `with_effort`, `with_resume`,
   `with_settings_json`, `with_dangerously_skip_permissions`, `with_env_var`, `with_max_turns`
3. Calls `agent.run(input, ctx)` → `ClaudeCliAgent::run` at `claude_cli_agent.rs:404`
4. `build_command()` at `claude_cli_agent.rs:302` adds ALL flags including `--verbose`, `--model`,
   `--effort`, `--settings`, `--dangerously-skip-permissions`, `--max-turns`, `--fallback-model`,
   `--append-system-prompt`, `--tools`, `--mcp-config`, `--strict-mcp-config`, `--resume`
5. AgentResult returned; session_id NOT extracted (ClaudeCliAgent::run does not return session_id)

## 4. Fields ClaudeCliAgent Accepts That dispatch_direct Does Not Pass

| Flag | dispatch_direct | ClaudeCliAgent |
|------|----------------|----------------|
| `--print` | YES | YES |
| `--output-format stream-json` | YES | YES |
| `--verbose` | NO | YES (line 307) |
| `--model` | NO | YES (line 310) |
| `--effort` | NO | YES (line 312) |
| `--settings` | NO | YES (line 314) |
| `--dangerously-skip-permissions` | NO | YES (line 316) |
| `--max-turns` | NO | YES (line 319) |
| `--fallback-model` | NO | YES (line 323) |
| `--append-system-prompt` | NO | YES (line 328) |
| `--tools` | NO | YES (line 331) |
| `--mcp-config` | NO | YES (line 336) |
| `--strict-mcp-config` | NO | YES (line 338) |
| `--resume` | NO | YES (line 340) |
| env vars (CARGO_INCREMENTAL etc) | NO | YES (line 344) |

## 5. Session ID Lifecycle

1. `dispatch_claude_cli` (dispatch_direct.rs:166): declares `let mut session_id: Option<String> = None`
2. Line 206–208: extracts `session_id` from the `"result"` stream-json event
3. Line 277: returns `DispatchResult { ..., session_id }` — session_id is Some("...")
4. `chat_inline.rs:1351`: receives `DispatchResult` via mpsc channel
5. Lines 1356–1376: processes `result.tool_outputs`, `result.text`, `result.model`,
   `result.input_tokens`, `result.output_tokens` — **`result.session_id` is never read**
6. `session_id` is DROPPED on every turn
7. `ChatSession` has no `session_id` field (lines 740–764, confirmed absent)
8. `dispatch_claude_cli` is always called without `--resume` — each turn is a fresh session
9. **BUG**: No memory between turns. Context window reuse is impossible.

## 6. Slash Commands and State Mutations

Full list from `chat_inline.rs:58–112` (`SLASH_COMMANDS`) with mutation status:

| Command | Mutates ChatSession? | Field |
|---------|---------------------|-------|
| `/quit`, `/exit`, `/q` | phase → Done | `session.phase` (line 1883) |
| `/compact` | YES | `session.compact` toggled (line 2116) |
| `/system <text>` | YES | `session.system_message = Some(msg)` (line 2134) |
| `/reset` | YES | `session.conversation.clear()`, `session.turn_count = 0`, `session.cost = CostMeter::new()`, `session.last_prompt = None`, `session.system_message = None` (lines 2138–2143) |
| `/retry` | YES | re-dispatches `session.last_prompt` |
| `/model` | NO | only displays info; does NOT change model slug |
| `/effort` | NO | listed in help text (line 1914) but `handle_slash_command` has NO case for it that mutates state |
| All other commands | NO | display-only |

**Critical gap**: `/effort` and `/model` appear in help text but neither mutates the `ChatSession`.
The session has no `model` or `effort` field — it cannot change per-turn model or effort.

## 7. Provider Adapters

- `ClaudeCliAdapter` (`claude_cli.rs:17`): maps `AgentOptions` → `ClaudeCliAgent` (lines 50–82)
  - Passes: `system_prompt`, `tools`, `mcp_config`, `effort`, `env`, `extra_args`, `name`,
    `bare_mode`, `dangerously_skip_permissions`
  - Does NOT pass: `resume` (not in `AgentOptions`)
- `AnthropicApiAdapter` and `OpenAiCompatAdapter`: handle API-based providers
- `create_agent_for_model` in `provider/mod.rs:110–213`: resolves provider kind from config,
  routes to the appropriate adapter via `adapter_for_kind(provider_config.kind)`
```

## Write Scope (files you may modify)

- `tmp/runners/mega-parity/context/R3_Z01_dispatch_audit.md` (create this file)

## Read-Only Context (do not modify these)

- `crates/roko-cli/src/chat_inline.rs`
- `crates/roko-cli/src/dispatch_direct.rs`
- `crates/roko-cli/src/unified.rs`
- `crates/roko-agent/src/claude_cli_agent.rs`
- `crates/roko-agent/src/provider/claude_cli.rs`
- `crates/roko-agent/src/provider/mod.rs`

## Acceptance Criteria

- [ ] Document exists at `tmp/runners/mega-parity/context/R3_Z01_dispatch_audit.md`
- [ ] Section 1 traces interactive chat from `cmd_unified_chat` to `dispatch_claude_cli` with file:line for each step
- [ ] Section 2 traces one-shot from `cmd_oneshot_inline` to dispatch result with file:line
- [ ] Section 3 traces plan execution through `ClaudeCliAgent::run` and `build_command`
- [ ] Section 4 has complete before/after flag table showing every flag ClaudeCliAgent passes that dispatch_direct does not
- [ ] Section 5 traces session_id extraction (dispatch_direct.rs:206) through discard (never stored in ChatSession)
- [ ] Section 6 lists every slash command and which field it mutates (or "NO" for display-only)
- [ ] Section 7 identifies all three provider adapters and what AgentOptions fields they consume
- [ ] No source code was modified

## Verification

```bash
# Verify document was created
ls /Users/will/dev/nunchi/roko/roko/tmp/runners/mega-parity/context/R3_Z01_dispatch_audit.md

# Verify session_id is not stored in chat_inline (confirms the bug)
grep -n "session_id" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs

# Verify ChatSession has no session_id field
grep -n "struct ChatSession" -A 30 \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs | head -35
```

## Do NOT

- Modify any source files
- Propose code changes (that is for subsequent batches)
- Skip any of the seven sections above

---

## Read-Only Context (do not modify)

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
