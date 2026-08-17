# Post-Parity Performance Contracts

## P-1: HTTP Connection Reuse

**Contract:** After PA lands, there is exactly ONE `reqwest::Client::new()` call per process.
**Measurement:** `grep -rn 'reqwest::Client::new()' crates/ --include='*.rs' | grep -v target/ | grep -v test` returns exactly 1 result (the shared client factory).
**Impact:** Eliminates 7-43s latency regression from TLS renegotiation per request.

## P-2: Chat Dispatch Latency

**Contract:** Chat dispatch adds < 5ms overhead to the LLM call.
**Measurement:** Time from user input to first byte from LLM provider.
**Impact:** System prompt assembly, tool marshaling, and history serialization are all sync and fast.

## P-3: Streaming Token Display

**Contract:** First token appears in TUI within 100ms of LLM emitting it.
**Measurement:** SSE event timestamp → TUI render timestamp.
**Impact:** StreamingState.append() is O(1) string append.

## P-4: Slash Command Latency

**Contract:** Slash commands complete in < 50ms (no network).
**Measurement:** Time from input to confirmation message.
**Impact:** Config writes are local file I/O only.

## P-5: Memory Stability

**Contract:** RSS does not grow unbounded during long runs.
**Measurement:** `efficiency_events` Vec is drained every 1000 events or 60 seconds.
**Impact:** Eliminates the 9.5GB RSS leak observed in production.

## P-6: Safety Default

**Contract:** Fresh `roko init` workspace has `dangerously_skip_permissions: false`.
**Measurement:** `roko init && grep dangerously roko.toml` shows false or absent (default false).
**Impact:** Agents cannot bypass permission checks without explicit opt-in.

## P-7: orchestrate.rs Freeze

**Contract:** `cargo build -p roko-cli --no-default-features` compiles without orchestrate.rs.
**Measurement:** Feature `legacy-orchestrate` is not in default features.
**Impact:** 21K LOC of dead code stops being compiled by default.
