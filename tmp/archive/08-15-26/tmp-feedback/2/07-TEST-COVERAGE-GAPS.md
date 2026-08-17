# Test Coverage Gaps: Tools × Models × Providers

## Existing Test Suites

### IDE Integration Tests (`tmp/solutions/ide/tests/`)

Shell-based, JSON-RPC over FIFO.

### Latest Run Results (2026-05-08)

| Suite | Script | Tests | Pass | Fail | Warn | Skip |
|-------|--------|-------|------|------|------|------|
| Core Protocol | `test-core.sh` | 8 | 7 | **1** | 0 | 0 |
| Model & Provider | `test-models.sh` | 6 | 6 | 0 | 0 | 0 |
| Config Options | `test-config-options.sh` | 9 | 8 | **1** | 0 | 0 |
| Session Lifecycle | `test-session-lifecycle.sh` | 10 | 10 | 0 | 0 | 0 |
| Edge Cases | `test-edge-cases.sh` | 9 | 7 | 0 | **2** | 0 |
| Streaming | `test-streaming.sh` | 10 | 9 | 0 | **1** | 0 |
| MCP Integration | `test-mcp.sh` | 6 | 2 | **1** | **1** | 2 |
| Tool Loop | `test-tool-loop.sh` | 5 | 2 | 0 | 0 | **3** |
| **Total** | | **63** | **51** | **3** | **4** | **5** |

**Improvement from 2026-05-04**: BUG #02 (model param ignored) is now FIXED (models suite 6/6 pass).

### Failures Diagnosed

#### FAIL: `clean exit on stdin close` (Core Protocol)
- **What**: After closing stdin, the ACP process is still alive after 3s timeout
- **Expected**: Process exits cleanly within 3s of stdin EOF
- **Root cause**: ACP server's stdin read loop likely blocks on async I/O and doesn't detect
  EOF promptly. May need a shutdown signal or explicit EOF detection in the read loop.
- **File**: `crates/roko-acp/src/handler.rs` (main read loop) or `transport.rs` (stdin reader)
- **Impact**: IDE integration — Zed/Cursor expect the ACP process to exit when they close the pipe

#### FAIL: `model change persists in session` (Config Options)
- **What**: After switching model to 'o3' and prompting, then checking current model,
  got 'o3' instead of expected 'sonnet'
- **Detail**: `model changed to 'o3', expected 'sonnet'` — the test expects the model to
  revert after a prompt, but the config update correctly persists the change
- **Likely**: Test logic bug — the test may be wrong (model SHOULD persist after config/update).
  Or the test intended to verify model doesn't revert and the assertion is inverted.
- **File**: `test-config-options.sh` — check the "model change persists" test assertions

#### FAIL: `MCP binary that exits → structured error` (MCP Integration)
- **What**: An MCP binary that immediately exits (exit 0) should produce a structured error
  in the session, but the session succeeds normally and the model responds with its tools list
- **Detail**: `session succeeded with immediately-exiting MCP binary, got response: Here are the tools I have available: **File Operations** - Read — read file`
- **Root cause**: ACP doesn't detect that the MCP server process died after spawning. The
  session proceeds with whatever tools were already registered (builtin tools), and the model
  happily uses those instead of reporting an MCP failure.
- **File**: `crates/roko-acp/src/session.rs` (MCP server health checking) or
  `crates/roko-acp/src/bridge_events.rs` (MCP status reporting)
- **Impact**: User won't know their MCP server failed — silent degradation

### Warnings Diagnosed

#### WARN: `config with no [models.*] → graceful error` (Edge Cases)
- **What**: Session created successfully even with no models configured — should at minimum warn
- **Expected**: Error or warning in session/new response
- **Impact**: Low — unusual config

#### WARN: `rapid-fire: 2nd prompt while 1st in-flight` (Edge Cases)
- **What**: When sending 2nd prompt while 1st is still streaming, the 2nd prompt is silently lost
- **Expected**: Either queue it, cancel 1st + start 2nd, or return error
- **Impact**: Medium — user rapidly typing could lose prompts

#### WARN: `thinking_chunk notifications with effort=high` (Streaming)
- **What**: With effort=high, expected thinking chunks from the model, but none arrived
- **Expected**: ThinkingChunk update notifications when model uses extended thinking
- **Likely**: The default model (gpt54-mini) doesn't support thinking, so no chunks are emitted.
  Test should filter by model capability or use a thinking-capable model.
- **Impact**: Low — test design issue, not a runtime bug

#### WARN: `MCP with invalid bridge token` (MCP Integration)
- **What**: With an invalid bridge token, no tool call was made
- **Expected**: Tool call attempted, bridge rejects, structured error returned
- **Impact**: Medium — error path not exercised

### Skipped Tests

| Test | Suite | Reason |
|------|-------|--------|
| `multiple tool calls in one turn` | Tool Loop | Bridge/MCP unavailable |
| `tool call creates a tile` | Tool Loop | Bridge/MCP unavailable |
| `tool error propagates to model` | Tool Loop | Bridge/MCP unavailable |
| `MCP tool discovery (valid binary)` | MCP | Bridge unavailable |
| `MCP tool returns real data` | MCP | Bridge unavailable |

All 5 skips are because the Nunchi IDE bridge (`http://127.0.0.1:6678`) is not running.
These tests require the full IDE bridge stack to execute MCP tools end-to-end.

### Previous Results (2026-05-04) for Comparison

```
Core Protocol:      8 passed                    → 7 passed, 1 FAILED (regression)
Model & Provider:   4 passed, 1 FAILED (BUG#02) → 6 passed (FIXED + new test)
MCP Integration:    1 passed, 2 FAILED, 2 skip  → 2 passed, 1 FAILED, 1 warn, 2 skip
Edge Cases:         7 passed, 2 warned           → 7 passed, 2 warned (same)
Session Lifecycle:  9 passed, 1 warned           → 10 passed (FIXED)
Streaming Protocol: 9 passed, 1 warned           → 9 passed, 1 warned (same)
Tool Loop:          2 passed, 3 skipped          → 2 passed, 3 skipped (same)
Config Options:     8 passed, 1 warned           → 8 passed, 1 FAILED (regression)
Total:              48 passed, 3 failed          → 51 passed, 3 failed
```

**Changes**:
- BUG #02 (model param) → **FIXED**
- Session lifecycle warning → **FIXED**
- Core `clean exit on stdin close` → **NEW FAILURE** (regression)
- Config `model change persists` → **NEW FAILURE** (was warn, now fail — test may have changed)
| Streaming Protocol | `test-streaming.sh` | 10 | — |
| Tool Loop | `test-tool-loop.sh` | 5 | — |
| Config Options | `test-config-options.sh` | 9 | — |

**Known failures**:
- BUG #01: MCP error cases return success instead of structured errors
- BUG #02: Model param ignored in `session/new`
- BUG #03: HashMap ordering non-determinism

### Agent Tests (`crates/roko-agent/tests/`)

32 test files with provider parity suites:

| Provider | Tests | Coverage |
|----------|-------|----------|
| Claude CLI | `chat_request.rs`, `subprocess_safety_parity.rs` | CLI invocation, safety |
| OpenAI (gpt-4o) | `openai_parity.rs`, `provider_integration.rs` | 5-test parity suite |
| Cursor | `cursor_parity.rs`, `cursor_streaming.rs` | 6 tests, streaming |
| Codex | `codex_parity.rs`, `codex_conformance.rs` | 10-turn replay |
| Ollama | `ollama_tool_loop.rs` | Tool loop iteration |
| Gemini | `gemini_integration.rs` | Function calling, grounding |
| GLM-5.1 (Zhipu) | `glm_tool_loop.rs`, `glm_web_search.rs` | Tool calls, web search |
| Kimi (Moonshot) | `kimi_tool_loop.rs`, `kimi_thinking_tools.rs` | Extended thinking + tools |
| Perplexity | `perplexity_integration.rs` | Error classification |
| OpenRouter | `openrouter_integration.rs` | Fallback routing |

### CLI Tests (`crates/roko-cli/tests/`)

27 integration tests covering workflows, plan execution, agent lifecycle.

## The Missing Matrix: Tools × Models × Providers

### What should exist: a cross-provider tool test matrix

Every backend that claims `supports_tools: true` should be tested with every tool type.

**Tools to test** (from `crates/roko-std/src/tool/builtin/`):

| Tool | Category | Side Effects |
|------|----------|-------------|
| `read_file` | File read | None |
| `write_file` | File write | Creates/overwrites |
| `edit_file` | File edit | Modifies in-place |
| `multi_edit` | File edit | Batch modify |
| `bash` | Execution | Shell command |
| `ls` | File read | None |
| `grep` | File read | None |
| `web_search` | Network | External API |
| `web_fetch` | Network | External API |
| `apply_patch` | File edit | Applies diff |
| `run_tests` | Execution | Shell command |

**Providers to test**:

| Provider | Tool Format | Tool Loop | Priority |
|----------|-----------|-----------|----------|
| Claude CLI | anthropic_blocks | CLI-owned | High |
| Anthropic API | anthropic_blocks | ToolLoopAgent | High |
| OpenAI-compat | openai_json | ToolLoopAgent | High |
| Gemini native | gemini_native | ToolLoopAgent | High |
| Ollama | openai_json | ToolLoopAgent (stream:false) | Medium |
| Perplexity | openai_json | ToolLoopAgent | Medium |
| Cursor ACP | — | None | Low (not wired) |

### Proposed Test Structure

```
tmp/solutions/ide/tests/
  tool-matrix/
    README.md                     # This document
    run-matrix.sh                 # Run all matrix tests
    lib/
      common.sh                   # Shared setup, mock server, assertions
      mock-provider.sh            # HTTP mock for each provider format
    suites/
      test-claude-cli-tools.sh    # Claude CLI + each tool
      test-anthropic-api-tools.sh # Anthropic API + each tool
      test-openai-tools.sh        # OpenAI + each tool
      test-gemini-tools.sh        # Gemini native + each tool
      test-ollama-tools.sh        # Ollama + each tool (stream:false)
      test-perplexity-tools.sh    # Perplexity + each tool
    fixtures/
      openai-tool-call.json       # Mock tool call response (OpenAI format)
      anthropic-tool-call.json    # Mock tool call response (Anthropic format)
      gemini-tool-call.json       # Mock tool call response (Gemini format)
```

### Test Scenarios Per Provider

Each provider suite should test:

```
1. TOOL DECLARATION
   - Tools are sent in correct format for provider
   - Tool schemas are valid JSON Schema
   - Provider-specific quirks handled (Kimi auto-only, Ollama no-stream)

2. TOOL CALL PARSING
   - Single tool call parsed correctly
   - Multiple parallel tool calls parsed
   - Tool call with complex JSON arguments parsed
   - Malformed tool call handled gracefully

3. TOOL RESULT INJECTION
   - Tool result fed back in correct format
   - Error result formatted correctly
   - Large result truncated to MAX_RESULT_BYTES

4. MULTI-TURN LOOP
   - Model calls tool → result → model responds (2-turn)
   - Model calls tool → result → calls another → result → responds (3-turn)
   - Model calls 3 tools in parallel → all results → responds

5. ERROR SCENARIOS
   - Tool not found → error result to model
   - Tool execution timeout → error result
   - Tool permission denied → error result
   - Rate limit during tool loop → back off
   - Context window exhaustion during tool loop → graceful stop

6. VISION (if supports_vision)
   - Image content block in first message
   - Image + text mixed content
   - Multiple images in one message
```

## Critical Gaps to Fill

### Gap 1: No Anthropic API tool tests
Only Claude CLI tested for Anthropic. The `anthropic_api` provider has a tool loop
but no integration test exercising it with real tool schemas.

### Gap 2: No research path tool tests
`roko research` doesn't exercise tools at all (see 04-TOOL-DISPATCH-BROKEN.md).
Need test proving tools work end-to-end in research commands.

### Gap 3: No cross-provider parity test
No test verifies that the same prompt+tools produces equivalent behavior across
Claude CLI, OpenAI, and Gemini backends.

### Gap 4: No MCP tool discovery e2e
IDE test suite has 5/6 MCP tests failing. MCP tool discovery is untested end-to-end.

### Gap 5: No image content in tool tests
Vision models receiving image content blocks during tool loops — untested.

### Gap 6: No concurrent tool execution test
Parallel tool calls (model requests 3 tools at once) — only sequential tested.

### Gap 7: No tool loop interruption test
What happens when user cancels mid-tool-loop? No test coverage.

## Existing Mock Infrastructure

| Component | Location | How |
|-----------|----------|-----|
| Mock HTTP server | `crates/roko-agent/tests/mock_provider.rs` | wiremock-based OpenAI-compat |
| Fixture loading | `crates/roko-agent/tests/fixture_loading.rs` | JSON response fixtures |
| Test fixtures | `crates/roko-agent/tests/fixtures/` | GLM, Kimi, common errors |
| FIFO protocol | `tmp/solutions/ide/tests/lib/` | Bash JSON-RPC helpers |
| MockHttpPoster | `crates/roko-agent/src/` | Async trait, VecDeque responses |

## Implementation Plan

### Phase 1: Fix existing failures (~2 hr)
- Fix BUG #01 (MCP errors) in `crates/roko-acp/`
- Fix BUG #02 (model param) in `crates/roko-acp/src/session.rs`
- Verify remaining IDE test suites

### Phase 2: Add tool matrix scaffold (~3 hr)
- Create `tool-matrix/` directory structure
- Write `common.sh` with mock server + assertions
- Write fixture files for each tool format
- Implement first suite: `test-openai-tools.sh`

### Phase 3: Fill provider suites (~4 hr)
- One suite per provider
- 6 scenarios per suite (declaration, parsing, injection, multi-turn, errors, vision)
- Use mock HTTP server for reproducibility

### Phase 4: Integration tests in Rust (~4 hr)
- Add `crates/roko-agent/tests/tool_matrix.rs`
- Parameterized tests across translator types
- Verify tool name aliasing works per format
- Test `ToolLoopAgent` wrapping each backend
