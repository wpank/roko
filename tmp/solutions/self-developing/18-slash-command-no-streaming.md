# 18: Slash Commands Don't Stream Output

## Problem

When running slash commands like `/prd-idea`, `/plan-generate`, `/build`, etc. in Zed, the UI appears to hang with no feedback. The command may take 30 seconds to 7+ minutes (for plan generation with opus), and the user sees nothing until it completes.

The user's exact complaint: "This is hanging and I can't tell if it's doing anything in the background or not, it should have better UX where it streams the tool calls"

---

## Root Cause

`bridge_events.rs` `run_slash_command()` (lines 2689–3293) read stdout line-by-line but **accumulated all output into a single `String`** and only sent it to Zed as one `TokenChunk` after the process completed:

```rust
// OLD CODE (buffered, no streaming):
loop {
    match reader.read_line(&mut line) {
        Ok(0) => break,
        Ok(_) => output.push_str(&line),  // accumulate silently
        Err(e) => break,
    }
}
// wait for process ...
event_sender.send(CognitiveEvent::TokenChunk(output)).await;  // dump everything at once
```

---

## Fix Applied (2026-05-06)

Changed to stream each line as it arrives. The new code at `bridge_events.rs:3228–3255`:

```rust
// NEW CODE (streaming):
loop {
    if cancel_token.is_cancelled() {
        let _ = child.kill().await;
        return Ok(());
    }
    line.clear();
    let read = tokio::select! {
        biased;
        _ = cancel_token.cancelled() => {
            let _ = child.kill().await;
            return Ok(());
        }
        r = reader.read_line(&mut line) => r,
    };
    match read {
        Ok(0) => break,
        Ok(_) => {
            has_output = true;
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(line.clone()))  // send immediately
                .await;
        }
        Err(e) => {
            warn!(session_id, error = %e, "error reading slash command output");
            break;
        }
    }
}
```

Uses `tokio::select!` with `biased` to prioritize cancellation. Lines are sent to Zed as `CognitiveEvent::TokenChunk` immediately as they arrive from stdout.

### Stderr handling

Stderr is still collected after stdout EOF (lines 3257–3274):

```rust
if let Some(stderr) = child.stderr.take() {
    let mut stderr_buf = String::new();
    let mut stderr_reader = tokio::io::BufReader::new(stderr);
    while let Ok(n) = stderr_reader.read_line(&mut stderr_buf).await {
        if n == 0 { break; }
    }
    let stderr_trimmed = stderr_buf.trim();
    if !stderr_trimmed.is_empty() {
        let _ = event_sender
            .send(CognitiveEvent::TokenChunk(format!(
                "\n--- stderr ---\n{stderr_trimmed}"
            )))
            .await;
    }
}
```

Stderr is still buffered — it appears as a block after all stdout, labeled with `--- stderr ---`.

---

## What Users See Now

Before: 60 seconds of blank screen → sudden wall of text
After: Output appears line by line as the CLI produces it

### How the CLI formats its output

The CLI (`roko plan generate`, `roko prd plan`, etc.) uses `FormattedStderrSink` which writes to **stderr**. The progress lines (`[plan/task] > Agent starting: ...`, `[plan/task] + Gate passed: ...`) go to stderr.

The final result (e.g. task count, summary) may go to stdout. This means:

- Progress lines appear in Zed as the `--- stderr ---` block at the end, not streamed
- Only stdout lines stream in real time

This is a structural problem: for ACP slash command streaming to work well, the CLI commands need to write progress to **stdout** (or there needs to be stderr interleaving — see remaining issues below).

---

## `run_shell_command` vs `run_slash_command`

**`run_slash_command`** (lines 2689–3293): wraps a `roko` CLI subcommand. The streaming fix has been applied here.

**`run_shell_command`** (lines 3296–3393): wraps raw shell commands for `/build`, `/test`, `/clippy`, `/run`, etc. It still uses the old buffered pattern:

```rust
// run_shell_command — STILL BUFFERED (lines 3335–3357):
loop {
    // ...
    match read {
        Ok(0) => break,
        Ok(_) => output.push_str(&line),  // accumulate silently
        Err(e) => { ... break; }
    }
}
// ... read stderr ...
let _ = event_sender.send(CognitiveEvent::TokenChunk(output)).await;  // dump at end
```

This is the same buffering bug that was fixed in `run_slash_command`. Shell commands like `cargo build` can run for 30+ seconds and produce hundreds of lines of output. All of it is silently buffered until completion.

**This is the next fix to apply.** The same `tokio::select!` streaming pattern should be applied to `run_shell_command`.

---

## Remaining Issues

### 1. Stderr interleaving

Both `run_slash_command` and `run_shell_command` read stderr sequentially after stdout closes. This means:

- If the roko CLI writes progress to stderr (which `FormattedStderrSink` does), those lines appear AFTER all stdout
- If `cargo build` writes errors to stderr, they appear after all info lines
- The ordering is misleading for commands that mix stdout/stderr

**Proper fix** — interleave stdout and stderr using `tokio::select!`:

```rust
use tokio::io::{AsyncBufReadExt, BufReader};

let stdout = BufReader::new(child.stdout.take().unwrap());
let stderr = BufReader::new(child.stderr.take().unwrap());
let mut stdout_lines = stdout.lines();
let mut stderr_lines = stderr.lines();

loop {
    tokio::select! {
        line = stdout_lines.next_line() => {
            match line {
                Ok(Some(l)) => {
                    let _ = event_sender.send(CognitiveEvent::TokenChunk(l + "\n")).await;
                }
                _ => break,
            }
        }
        line = stderr_lines.next_line() => {
            match line {
                Ok(Some(l)) => {
                    // Send stderr with a subtle prefix or as-is
                    let _ = event_sender.send(CognitiveEvent::TokenChunk(
                        format!("\x1b[2m{l}\x1b[0m\n")  // dim ANSI for stderr
                    )).await;
                }
                _ => { /* stderr closed, continue draining stdout */ }
            }
        }
        _ = cancel_token.cancelled() => {
            let _ = child.kill().await;
            return Ok(());
        }
    }
}
```

This is technically correct but tricky — when one stream closes, the select! will keep returning the closed stream. The implementation needs to track which streams are still open.

### 2. Tool calls within agents not visible

When `/plan-generate` shels out to `roko plan generate`, roko internally dispatches Claude agents. Those agents produce:
- File reads (Read tool)
- File writes (Write/Edit tools)
- Bash commands
- Text generation (tokens)

None of this is surfaced to Zed. Only the final stdout lines from the roko CLI process are visible.

**Proper fix** — structured progress events. The roko CLI should emit machine-readable progress to stdout interspersed with human-readable output:

```
ROKO_PROGRESS: {"type":"task_started","task_id":"T001","title":"Add rate limiting","role":"architect"}
[plan/task] > Agent starting: "Add rate limiting" [architect]
ROKO_PROGRESS: {"type":"tool_call","task_id":"T001","tool":"Read","path":"crates/roko-serve/src/lib.rs"}
[plan/task] > Tool: Read
ROKO_PROGRESS: {"type":"tool_complete","task_id":"T001","tool":"Read","duration_ms":45}
ROKO_PROGRESS: {"type":"task_completed","task_id":"T001","duration_ms":45234}
[plan/task] + Task completed (1/6) in 45.2s
```

The ACP `run_slash_command` would parse `ROKO_PROGRESS:` prefixed lines and convert them to `CognitiveEvent::ToolCallStart` / `CognitiveEvent::ToolCallComplete` events. Non-prefixed lines go as `TokenChunk`.

This lets Zed show a structured tool call timeline instead of flat text.

### 3. Non-slash-command prompts (Zed-side issue)

When the user types a natural language prompt (not a `/command`), the ACP dispatches to the LLM and the only feedback is the streaming text response. If the model takes a while to start, there is no "thinking..." indicator. This is a Zed-side issue — Zed should show a spinner while waiting for the first token from the ACP.

### 4. `run_shell_command` still buffered

As noted above, shell commands (`/build`, `/test`, `/clippy`) still use the old buffered pattern. Same fix as was applied to `run_slash_command` should be applied here.

---

## ACP Slash Command Architecture

```
User types /plan-generate "add rate limiting"
                    │
            bridge_events.rs:1144
            run_slash_command() called
                    │
            tokio::process::Command::new("roko")
            .args(["plan", "generate", "add rate limiting"])
            .stdin(null)
            .stdout(piped)
            .stderr(piped)
            .spawn()
                    │
            ┌───────────────────────────────────┐
            │  roko plan generate (subprocess)  │
            │                                   │
            │  FormattedStderrSink → stderr     │
            │  "[plan/T001] > Agent starting"   │
            │  "[plan/T001] + Gate passed"      │
            │                                   │
            │  stdout: (result summary only)    │
            └───────────────────────────────────┘
                    │
            stdout lines → TokenChunk (streamed immediately)
            stderr lines → TokenChunk ("\n--- stderr ---\n...") after EOF
                    │
            Zed renders as text in assistant response
```

### How `run_slash_command` resolves the CLI binary

**`bridge_events.rs:3222`**: uses `tokio::process::Command::new("roko")` — relies on `roko` being in PATH. The working directory is set to the session's workdir.

### Slash command to CLI arg mapping

**`bridge_events.rs:2726–2898`**: a `match` block maps command names to CLI arg arrays. Examples:

```rust
"prd-plan" => vec!["prd".into(), "plan".into(), args.into()],
"plan-generate" => vec!["plan".into(), "generate".into(), args.into()],
"build" => return run_shell_command(session_id, "cargo build 2>&1", workdir, ...),
"test" => return run_shell_command(session_id, "cargo test 2>&1", workdir, ...),
```

Note: `/build`, `/test`, `/clippy` go to `run_shell_command` (the still-buffered path), not `run_slash_command` (the fixed streaming path). This is the most important remaining fix — cargo commands are the longest-running and most valuable to stream.

---

## Design for Structured Progress Events

The full design for surfacing internal agent progress through the ACP layer requires:

1. **CLI emits structured events to stdout** alongside human-readable text (NDJSON with `ROKO_PROGRESS:` prefix or a dedicated file descriptor)
2. **`run_slash_command` parses structured lines** and converts them to `CognitiveEvent` variants:
   - `ROKO_PROGRESS: {"type":"task_started",...}` → `CognitiveEvent::ToolCallStart`
   - `ROKO_PROGRESS: {"type":"task_completed",...}` → `CognitiveEvent::ToolCallComplete`
   - `ROKO_PROGRESS: {"type":"agent_text",...}` → `CognitiveEvent::TokenChunk`
3. **Zed renders the tool call timeline** — not just flat text

The `RunOutputSink` trait in `output_sink.rs` already defines the right event set:
- `task_started` / `task_completed` / `task_failed`
- `agent_started` / `agent_turn_completed` / `agent_error`
- `tool_call` / `tool_output`
- `gate_result` / `gate_retry`

A new `AcpProgressSink` implementing `RunOutputSink` could emit these as JSON to stdout, parallel to `FormattedStderrSink` writing human-readable text to stderr.

```rust
pub struct AcpProgressSink;

impl RunOutputSink for AcpProgressSink {
    fn task_started(&self, plan_id: &str, task_id: &str, role: &str, title: &str, attempt: u32) {
        println!("ROKO_PROGRESS: {}", serde_json::json!({
            "type": "task_started",
            "plan_id": plan_id,
            "task_id": task_id,
            "role": role,
            "title": title,
            "attempt": attempt,
        }));
    }
    // ... other events
}
```

Then in `run_slash_command`, split on prefix:

```rust
Ok(_) => {
    has_output = true;
    if line.starts_with("ROKO_PROGRESS: ") {
        let json = &line["ROKO_PROGRESS: ".len()..];
        if let Ok(evt) = parse_progress_event(json) {
            let _ = event_sender.send(acp_event_from_progress(evt)).await;
        }
    } else {
        let _ = event_sender
            .send(CognitiveEvent::TokenChunk(line.clone()))
            .await;
    }
}
```

---

## Files Modified

| File | Change |
|------|--------|
| `crates/roko-acp/src/bridge_events.rs:3222–3254` | Stream `TokenChunk` per line instead of buffering stdout |
| `crates/roko-acp/src/bridge_events.rs:3257–3274` | Stderr still buffered — next fix target |
| `crates/roko-acp/src/bridge_events.rs:3335–3384` | `run_shell_command` still buffered — next fix target |

## Next Actions

1. Apply the same streaming fix to `run_shell_command` (lines 3335–3357)
2. Add stderr interleaving to both functions using `tokio::select!`
3. Design and implement `AcpProgressSink` for structured progress events
4. Wire `AcpProgressSink` alongside `FormattedStderrSink` in plan/do commands
