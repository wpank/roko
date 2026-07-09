# IDE Integration Test Suite

Automated regression tests for roko ACP behavior as consumed by IDE clients.
Designed for parallel execution by multiple agents.

## Quick Start

```bash
# Run everything (parallel, fastest)
./run-all.sh --parallel

# Run a single suite (for one agent)
./run-one.sh core --json --bail
./run-one.sh models --json --bail
./run-one.sh mcp --json --bail

# Run specific suites
SUITES="core,models" ./run-all.sh --parallel

# Bail on first failure (fast feedback)
./run-all.sh --bail

# Machine-readable JSON output (for agents/CI)
./run-all.sh --json --parallel 2>/dev/null

# Filter to specific test(s) within a suite
./run-one.sh core --filter=session --json

# Skip slow tests
./run-all.sh --parallel --quick
```

## For Agents (Claude, automated workers)

### Running tests in parallel (multiple agents)

Each agent runs a different suite — all fully isolated:

```bash
# Agent 1: core protocol
./run-one.sh core --json --bail

# Agent 2: models & providers
./run-one.sh models --json --bail

# Agent 3: MCP integration
./run-one.sh mcp --json --bail

# Agent 4: edge cases
./run-one.sh edge --json --bail

# Agent 5: session lifecycle
./run-one.sh lifecycle --json --bail

# Agent 6: streaming protocol
./run-one.sh streaming --json --bail

# Agent 7: tool loop
./run-one.sh toolloop --json --bail

# Agent 8: config options
./run-one.sh config --json --bail
```

Each agent gets:
- **Own ACP subprocess** per test (no shared state)
- **Own FIFO directory** (`mktemp`, unique per PID+timestamp)
- **Own log directory** (timestamped, never shared)
- **Full isolation** — zero interference between concurrent agents

### JSON output for parsing

`--json` mode outputs one JSON line per test + one summary line per suite:

```json
{"suite":"Core Protocol Tests","test":"session/new returns sessionId","status":"pass","ms":1184,"detail":"sess_abc...","instance":"12345_167...","log":"/tmp/roko-ide-tests/12345_167.../acp_....log"}
{"suite":"Core Protocol Tests","test":"session/prompt streams response","status":"fail","ms":5023,"detail":"expected CORE_TEST_OK...","instance":"12345_167...","log":"/tmp/roko-ide-tests/12345_167.../acp_....log"}
{"suite":"Core Protocol Tests","summary":true,"passed":7,"failed":1,"warned":0,"skipped":0,"ms":21000,"instance":"12345_167...","log_dir":"/tmp/roko-ide-tests/12345_167..."}
```

Parse failures: `./run-one.sh core --json | python3 -c "import sys,json; [print(json.dumps(d)) for l in sys.stdin if (d:=json.loads(l)).get('status')=='fail']"`

### Building before testing

If you need to ensure the binary is built first:

```bash
./run-all.sh --build --parallel   # builds once, then runs all suites
```

Or manually:
```bash
cd /Users/will/dev/nunchi/roko/roko && cargo build --release -p roko-cli
```

### Debugging failures

When a test fails, the output includes:
- **Log file path** — stderr from the ACP process
- **Last 5 lines of stderr** — immediate context for the error
- **Elapsed time** — helps identify timeouts vs. real failures

To get more detail, re-run with `--verbose`:
```bash
./run-one.sh core --verbose --filter=session
```

This shows every JSON-RPC message sent/received.

## Flags

| Flag | Effect |
|------|--------|
| `--parallel` | Run suites concurrently with isolated temp dirs |
| `--bail` | Stop on first failure (per suite) |
| `--quick` | Skip slow tests (multi-turn, thinking, etc.) |
| `--json` | One JSON object per test result (for machine parsing) |
| `--verbose` | Show raw JSON-RPC traffic (debug) |
| `--filter=X` | Only run tests whose name matches X (grep pattern) |
| `--build` | Build roko binary before running tests (run-all.sh only) |

## Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `ROKO_BIN` | `$(which roko)` or `target/release/roko` | Path to roko binary |
| `ROKO_CONFIG` | `~/.nunchi/roko/roko.toml` | Config file for ACP |
| `ACP_TIMEOUT` | `15` | Default timeout in seconds for ACP reads |
| `NUNCHI_MCP` | auto-detected | Path to nunchi-mcp binary |
| `BRIDGE_URL` | `http://127.0.0.1:6678` | IDE bridge URL |
| `LOG_DIR` | auto (per-instance) | Override log directory |
| `SUITES` | all | Comma-separated: `core,models,mcp,edge,lifecycle,streaming,toolloop,config` |

## Test Suites

| Suite | Script | run-one name | Tests | Notes |
|-------|--------|-------------|-------|-------|
| Core Protocol | `test-core.sh` | `core` | 8 | Session lifecycle, error handling, disconnect |
| Model & Provider | `test-models.sh` | `models` | 6 | Model param, provider routing, output limits |
| MCP Integration | `test-mcp.sh` | `mcp` | 6 | Tool discovery, error cases (needs bridge) |
| Edge Cases | `test-edge-cases.sh` | `edge` | 9 | Concurrent sessions, config variations |
| Session Lifecycle | `test-session-lifecycle.sh` | `lifecycle` | 10 | Config update, cancel, list, close, modes |
| Streaming Protocol | `test-streaming.sh` | `streaming` | 10 | Update notifications, chunk shapes, usage |
| Tool Loop | `test-tool-loop.sh` | `toolloop` | 5 | Multi-tool calls, errors (needs bridge) |
| Config Options | `test-config-options.sh` | `config` | 9 | Defaults, provider switching, persistence |

## After Fixing a Bug

```bash
# After fixing BUG #02 (model param):
./run-one.sh models --bail --filter=model
# Expected: "session/new respects model param" → PASS

# After fixing BUG #01 (MCP errors):
./run-one.sh mcp --bail --filter=nonexistent
# Expected: "nonexistent MCP binary → structured error" → PASS

# After fixing BUG #03 (HashMap ordering):
./run-one.sh config --bail --filter=deterministic
# Expected: "default provider is consistent" stays PASS

# Verify all fixes together:
./run-all.sh --parallel --bail
```

## Latest Results (2026-05-08)

```
Core Protocol:      7 passed, 1 FAILED (stdin close — process still alive after 3s)
Model & Provider:   6 passed  (BUG#02 FIXED, new test added)
MCP Integration:    2 passed, 1 FAILED (crashing binary not detected), 1 warned, 2 skipped
Edge Cases:         7 passed, 2 warned (no-models config, rapid-fire prompt lost)
Session Lifecycle:  10 passed (warning from 05-04 FIXED)
Streaming Protocol: 9 passed, 1 warned (no thinking chunks — model-dependent)
Tool Loop:          2 passed, 3 skipped (no bridge)
Config Options:     8 passed, 1 FAILED (model persists as 'o3', expected 'sonnet')
--------------------------------------------
Total:             51 passed, 3 failed, 4 warned, 5 skipped
Wall time (parallel): ~62s
```

### Changes from 2026-05-04
- **FIXED**: BUG #02 (model param ignored) — models suite now 6/6 pass
- **FIXED**: Session lifecycle warning — now 10/10 pass
- **NEW FAIL**: Core `clean exit on stdin close` — ACP process doesn't exit on stdin EOF
- **NEW FAIL**: Config `model change persists` — may be test logic issue (model correctly persists)
- **IMPROVED**: MCP `nonexistent binary` now passes (was FAIL, now returns structured warning)

### Open Bugs

| Bug | Suite | Test | Detail |
|-----|-------|------|--------|
| BUG #04 | Core | `clean exit on stdin close` | ACP process still alive after 3s of stdin EOF |
| BUG #05 | Config | `model change persists in session` | Model change persists (may be test assertion wrong) |
| BUG #01 | MCP | `MCP binary that exits → structured error` | Crashing MCP binary not detected, session succeeds silently |

### Systemic Issues (not covered by these tests)

| Issue | Impact | Detail |
|-------|--------|--------|
| **Graph Engine no-op** | `roko plan run` does nothing | TaskExecutorCell is a stub; no agents spawn; all tasks return synthetic output |
| **Tool dispatch broken** | `roko research` emits raw JSON | No tool loop wrapper; tools declared but never dispatched |
| **Runner v2 feature-gated** | Working runner hidden | `legacy-runner-v2` Cargo feature not enabled by default |
| **Preflight verify skip** (BUG #06) | Agents never spawn | Runner v2 skips agent when stub code compiles (`event_loop.rs:3992`) |
| **Gate crate name bug** (BUG #07) | Gates fail on nested crates | `crate_name_for_path("crates/hdc/core/")` → `hdc`, package is `kora-hdc` |
| **Bridge-dependent tests** | 5 tests always skip | Tool loop + MCP tests need `http://127.0.0.1:6678` bridge |

See `tmp/tmp-feedback/2/` for full diagnosis of each issue.

### Previous Results (2026-05-04)

```
Core Protocol:      8 passed
Model & Provider:   4 passed, 1 FAILED (BUG#02)
MCP Integration:    1 passed, 2 FAILED (BUG#01), 2 skipped
Edge Cases:         7 passed, 2 warned
Session Lifecycle:  9 passed, 1 warned
Streaming Protocol: 9 passed, 1 warned
Tool Loop:          2 passed, 3 skipped (no bridge)
Config Options:     8 passed, 1 warned
--------------------------------------------
Total:             48 passed, 3 failed, 5 warned, 5 skipped
Wall time (parallel): ~21s
Wall time (sequential): ~160s
```
