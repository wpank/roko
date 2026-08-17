# Gemini CLI as Agent Backend

## Problem

Roko has 3 Gemini implementations, all HTTP-only requiring `GEMINI_API_KEY`:

| Implementation | File | Auth |
|---|---|---|
| GeminiNativeAgent | `crates/roko-agent/src/gemini/native.rs` | GEMINI_API_KEY |
| GeminiCompatAgent | `crates/roko-agent/src/gemini/compat.rs` | GEMINI_API_KEY |
| GeminiEmbedAgent | `crates/roko-agent/src/gemini/embed.rs` | GEMINI_API_KEY |

User has Gemini CLI installed with **Google OAuth** (no API key needed). Can't use it.

## User's Setup (from screenshot)

- **Gemini CLI v0.37.1** — installed and working
- **Auth**: Google account ("Signed in with Google /auth")
- **Plan**: Gemini Code Assist for individuals (free tier)
- **Available models**: gemini-3-flash-preview, glm-5-turbo, glm-5.1, many others
- **Missing models**: gemini-2.5-flash, gemini-2.5-pro (free tier restriction)
- **Binary**: `gemini` on PATH

## Implementation Plan

### 1. Add ProviderKind::GeminiCli (`crates/roko-core/src/agent.rs`)

```rust
pub enum ProviderKind {
    // ... existing variants ...
    GeminiCli,  // NEW: spawns `gemini` subprocess, Google OAuth
}

// In AgentBackend mapping:
ProviderKind::GeminiCli => AgentBackend::GeminiCli,
```

### 2. Add ProviderTransport for CLI (`crates/roko-core/src/config/provider.rs`)

```rust
// GeminiCli uses the Cli transport variant:
ProviderTransport::Cli {
    command: "gemini".to_string(),
    args: vec![],
}
```

### 3. Create GeminiCliAdapter (`crates/roko-agent/src/provider/gemini_cli.rs`)

New file, modeled after `claude_cli.rs`:

```rust
pub struct GeminiCliAdapter { /* ... */ }

impl ProviderAdapter for GeminiCliAdapter {
    fn create_agent(&self, opts: &AgentOptions) -> Result<Box<dyn Agent>> {
        Ok(Box::new(GeminiCliAgent::new(opts)?))
    }
}
```

### 4. Create GeminiCliAgent (`crates/roko-agent/src/gemini_cli_agent.rs`)

New file, modeled after `claude_cli_agent.rs`:

```rust
pub struct GeminiCliAgent {
    program: String,    // "gemini"
    model: String,      // "gemini-3-flash-preview"
    // ...
}

impl GeminiCliAgent {
    fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.arg("-m").arg(&self.model);  // model selection
        cmd.arg("-p");                    // non-interactive prompt mode
        // Gemini CLI outputs to stdout
        cmd
    }
}
```

### 5. Gemini CLI Interface Reference

```bash
# Single prompt (non-interactive)
gemini -p "your prompt here"

# With model selection
gemini -m gemini-3-flash-preview -p "your prompt"

# With file context
gemini -p "analyze this" @path/to/file.rs

# With image input
gemini -p "describe this" @path/to/image.png

# JSON output (if supported)
gemini --output json -p "your prompt"
```

### 6. Model Registry Entries (`crates/roko-core/src/config/registry.rs`)

```rust
BuiltinModel {
    slug: "gemini-3-flash-preview",
    provider: "gemini-cli",
    context_window: 1_048_576,
    max_output: 8192,
    supports_tools: true,   // verify
    supports_vision: true,  // verify
    supports_thinking: false,
    tool_format: ToolFormat::GeminiNative,  // or determine from CLI output
},
```

### 7. Doctor Check (`crates/roko-cli/src/doctor.rs`)

Add detection of `gemini` binary on PATH:
```rust
fn check_gemini_cli() -> DiagnosticCheck {
    // which gemini → if found, report as available provider
    // No API key check needed (uses Google OAuth)
}
```

### 8. Auth Detection (`crates/roko-cli/src/auth_detect.rs`)

Add to fallback chain:
```rust
// After claude CLI check, before API key checks:
if which("gemini").is_ok() {
    return AuthMethod::GeminiCli;
}
```

### 9. Config Example

```toml
[providers.gemini-cli]
kind = "gemini_cli"
# No api_key_env needed — uses Google OAuth via `gemini /auth`

[models.gemini-3-flash]
provider = "gemini-cli"
slug = "gemini-3-flash-preview"
```

## Open Questions

1. **Tool support**: Does Gemini CLI support function calling / tool use in non-interactive mode?
   Need to test `gemini -p "..." --tools ...` or similar.
2. **Output format**: What's the structured output format? JSON? Plain text?
   Need to test `gemini --output json -p "..."`.
3. **Streaming**: Does Gemini CLI stream output or buffer? Important for TUI progress.
4. **Session management**: Does it support multi-turn via session IDs?

## Effort Estimate

- Provider kind + adapter scaffold: ~2 hr
- Subprocess spawn + output parsing: ~2 hr
- Tool calling support (if CLI supports it): ~2 hr
- Config + doctor integration: ~1 hr
- Total: ~7 hr
