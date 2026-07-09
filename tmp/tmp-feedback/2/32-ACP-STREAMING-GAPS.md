# ACP Streaming Gaps: Buffered Output, Invisible Tool Calls

## Problem

Most ACP slash commands don't stream output to Zed — they buffer the entire result and
send it at the end. Tool calls inside CLI subprocesses are invisible to the ACP client.

## Root Cause

### A. `run_slash_command` output is buffered

**File:** `crates/roko-acp/src/bridge_events.rs`

Most slash commands use `run_cli_subprocess()` which captures stdout/stderr and returns
the full text when the process exits:

```rust
async fn run_cli_subprocess(args: &[&str], session: &AcpSession) -> Result<String> {
    let output = Command::new("roko")
        .args(args)
        .output()  // ← waits for completion, captures all output
        .await?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

The user sees nothing until the command completes. For a 30-second research task, Zed
shows a blank panel for 30 seconds, then dumps all output at once.

### B. Some commands DO stream

`/express` and `/full` (the primary chat paths) stream via `run_streaming_agent()` which
uses `tokio::io::BufReader` on stdout and sends each line as an ACP `text` event.

This works well. The problem is that slash commands don't use this path.

### C. Tool calls inside subprocess are invisible

When a CLI subprocess runs Claude CLI, Claude CLI may make tool calls (reading files,
running bash commands). These tool calls are logged to Claude CLI's output but:
- They're mixed into the stdout stream as progress text
- The ACP protocol has no structured way to represent "the agent is calling Read on file X"
- The TUI dashboard shows tool calls, but ACP doesn't

## Fix

### Fix 1: Stream slash command output (~20 min)

**File:** `crates/roko-acp/src/bridge_events.rs`

Replace `run_cli_subprocess()` with a streaming variant for long-running commands:

```rust
async fn run_streaming_subprocess(
    args: &[&str],
    session: &AcpSession,
    sender: &AcpEventSender,
) -> Result<String> {
    let mut child = Command::new("roko")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = BufReader::new(child.stdout.take().unwrap());
    let mut lines = stdout.lines();
    let mut full_output = String::new();

    while let Some(line) = lines.next_line().await? {
        full_output.push_str(&line);
        full_output.push('\n');
        sender.send_text(&line).await?;  // ← stream each line
    }

    child.wait().await?;
    Ok(full_output)
}
```

Use this for: `/do`, `/research`, `/analyze`, `/plan-run`, `/develop`, `/enhance-*`

Keep buffered for fast commands: `/status`, `/doctor`, `/prd-list`, `/config-*`

### Fix 2: Parse tool call markers from subprocess output (~15 min)

If the subprocess outputs tool call markers (e.g., Claude CLI's `⏺ Read(path)` lines),
parse them and send structured ACP events:

```rust
if line.starts_with("⏺ ") {
    // Parse tool call: "⏺ Read(crates/roko-cli/src/main.rs)"
    sender.send_tool_call(&parsed_tool_name, &parsed_args).await?;
} else {
    sender.send_text(&line).await?;
}
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-acp/src/bridge_events.rs` | Add streaming subprocess variant |
| `crates/roko-acp/src/bridge_events.rs` | Use streaming for long-running slash commands |

## Priority

**P1** — Users staring at a blank panel for 30 seconds think the command is broken.
Streaming output provides immediate feedback and progress indication.
