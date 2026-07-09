# IDE Integration — Test Results

Last run: 2026-05-04

## Quick Summary

```
Total: 44 passed, 3 failed, 5 warned, 11 skipped (63 tests)
```

All 3 failures are pre-existing known bugs covered by the implementation plan.

## By Suite

| Suite | P | F | W | S | Notes |
|-------|---|---|---|---|-------|
| Core Protocol | 8 | 0 | 0 | 0 | All basic ACP operations working |
| Model & Provider | 4 | 1 | 1 | 0 | FAIL: model param in session/new ignored (BUG#02) |
| MCP Integration | 1 | 2 | 1 | 2 | FAIL: silent errors for bad binaries (BUG#01) |
| Edge Cases | 6 | 0 | 1 | 2 | Rapid-fire timing warn |
| Session Lifecycle | 9 | 0 | 1 | 0 | config/update, cancel, list, close, modes all work |
| Streaming Protocol | 8 | 0 | 0 | 2 | Chunks, usage, context all correct |
| Tool Loop | 1 | 0 | 0 | 4 | MCP tool tests skipped (bridge not running) |
| Config Options | 7 | 0 | 1 | 1 | Provider/model switching verified |

## Failures (All Pre-Existing Bugs)

### 1. session/new ignores model param (BUG#02)

```
test-models.sh: "session/new respects model param"
Expected: currentValue=haiku after passing model:"haiku" in session/new
Got: currentValue=sonnet (model param silently dropped by serde)
```

**Fix**: W1-A batch -- add model/provider/effort fields to `SessionNewParams` struct.

### 2-3. MCP binary failures are silent (BUG#01)

```
test-mcp.sh: "nonexistent MCP binary -> structured error"
test-mcp.sh: "MCP binary that exits -> structured error"
Both: prompt succeeds with no indication of MCP failure
```

**Fix**: W2-B/W2-C batches -- accumulate McpServerStatus errors, emit structured notification.

## Warnings

| Test | Warning | Severity |
|------|---------|----------|
| nonexistent model silently accepted | config/update returns success for bad model | Low |
| invalid model accepted (lifecycle) | Falls back to previous model silently | Low |
| unknown optionId accepted (config) | No error for nonexistent option | Low |
| no thinking chunks (streaming) | effort=high doesn't stream thinking tokens | Low |
| rapid-fire 2nd prompt lost (edge) | Concurrent prompts -- 2nd silently dropped | Medium |

## How to Run

```bash
cd /Users/will/dev/nunchi/roko/roko/tmp/solutions/ide/tests

# Full suite
bash run-all.sh

# Quick (skip slow multi-turn/thinking tests)
bash run-all.sh --quick

# Individual suites
bash test-core.sh
bash test-models.sh
bash test-mcp.sh
bash test-edge-cases.sh
bash test-session-lifecycle.sh
bash test-streaming.sh
bash test-tool-loop.sh
bash test-config-options.sh
```

## Prerequisites

- `roko` binary in PATH (or set `ROKO_BIN`)
- `~/.nunchi/roko/roko.toml` with at least one working provider
- For MCP tests: `nunchi-mcp` binary (set `NUNCHI_MCP`)
- For tool loop MCP tests: HTTP bridge running on `BRIDGE_URL`

## Key Discoveries (from testing)

1. **Wire format for `session/config/update`** is a flat struct:
   `{sessionId, optionId, newValue}` -- NOT an `updates` array.

2. **Streaming chunks** use `content.text` (a content block), not a `delta` string.

3. **Default provider selection** is non-deterministic (HashMap ordering picks cerebras
   instead of openai). Workaround: send `config/update` for provider immediately after session/new.

4. **Model names are provider-scoped** -- setting model to "sonnet" only works if the
   current provider has a model with that key. Cross-provider model names silently fail.

5. **Modes** are in `result.modes.availableModes` (not `result.modes.modes`).

6. **`session_info_update`** is a notification type not previously documented.

7. **Context window**: `usage_update` reports `size=128000, used=~2900` for a minimal
   single-turn conversation. Grows ~30-50 tokens per short turn.
