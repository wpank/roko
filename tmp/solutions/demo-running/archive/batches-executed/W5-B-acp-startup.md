# W5-B: ACP Workspace Auto-Creation + Log Fallback

**Priority**: P0 — blocks ACP/Zed usage
**Effort**: 30 minutes
**Files to modify**: 1 file
**Dependencies**: None

## Problem

Zed editor shows "Failed to Launch — server shut down unexpectedly" when using roko as ACP provider. The ACP handler fails silently on startup because `.roko/` directory doesn't exist.

## Root Cause

`crates/roko-acp/src/handler.rs` line 27: `setup_file_logging(config.log_file())` fails if `.roko/` doesn't exist. The error goes to stderr (invisible to Zed). Process exits 1 without sending a JSON-RPC error response.

## Exact Code to Change

### File: `crates/roko-acp/src/handler.rs`

### Change 1: Add workspace auto-creation before logging (line ~26)

```rust
pub async fn run_acp_server(config: AcpConfig) -> Result<()> {
    // Ensure workspace exists before any file operations
    let workdir = config.workdir.canonicalize()
        .unwrap_or_else(|_| config.workdir.clone());
    let roko_dir = workdir.join(".roko");
    if let Err(e) = std::fs::create_dir_all(&roko_dir) {
        // Non-fatal — we'll fall back to /tmp for logging
        eprintln!("warning: cannot create .roko/: {e}");
    }

    let _guard = setup_file_logging(config.log_file())
        .or_else(|e| {
            // Fallback: log to /tmp if .roko/ is unavailable
            let fallback = std::env::temp_dir()
                .join(format!("roko-acp-{}.log", std::process::id()));
            eprintln!("warning: {e}, falling back to {}", fallback.display());
            setup_file_logging(&fallback)
        })
        .with_context(|| "failed to initialize ACP logging")?;

    let mut transport = StdioTransport::new();
    run_acp_server_with_transport(config, &mut transport).await
}
```

### Change 2: Send JSON-RPC error before exit

If the ACP server fails to start, it should send a proper JSON-RPC error response on stdout so the editor can display a meaningful message:

```rust
// At the very top of run_acp_server, wrap everything in a catch:
pub async fn run_acp_server(config: AcpConfig) -> Result<()> {
    match run_acp_server_inner(config).await {
        Ok(()) => Ok(()),
        Err(e) => {
            // Send JSON-RPC error on stdout so editor can display it
            let error_response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {
                    "code": -32603,
                    "message": format!("ACP server failed to start: {e:#}")
                }
            });
            let _ = writeln!(std::io::stdout(), "{}", error_response);
            Err(e)
        }
    }
}

async fn run_acp_server_inner(config: AcpConfig) -> Result<()> {
    // ... existing body moved here
}
```

### Change 3: Update `setup_file_logging` to accept `&Path` instead of `PathBuf` if needed

Check the current signature. It takes `&Path` (lines 400-429). The fallback path needs to work with a temporary path.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W5-B-acp-startup.md and implement all changes described in it. Add workspace auto-creation and log fallback to crates/roko-acp/src/handler.rs. Add JSON-RPC error response on fatal startup failure. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 5 batches together. Do not commit individually.

## Checklist

- [x] Add workspace directory auto-creation before logging setup
- [x] Add log file fallback to /tmp/ if .roko/ is unavailable
- [x] Send JSON-RPC error response on stdout if startup fails
- [ ] Verify: ACP starts in directory without .roko/
- [ ] Verify: ACP sends error response (not silent crash) on failure
- [ ] Pre-commit checks pass
