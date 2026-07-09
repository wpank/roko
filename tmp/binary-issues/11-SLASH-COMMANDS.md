# 11 — Comprehensive Slash Commands

**Status**: implemented (46 commands, all wired)
**Scope**: `crates/roko-cli/src/chat_inline.rs`

## Overview

Transform `roko chat` from a simple prompt-response REPL into a full interactive
control center. Every CLI subcommand and configuration option should be accessible
via slash commands, matching and exceeding the ACP runner's 44-command surface.

---

## Category 1: Session & Display

| Command | Purpose |
|---------|---------|
| `/help` | Show available commands (grouped) |
| `/version` | Show roko version, rustc, platform |
| `/stats` | Detailed session stats (turns, tokens, cost breakdown, elapsed) |
| `/context` | Show context window usage (tokens used / limit) |
| `/history` | Show input history (last 20 entries) |
| `/copy` | Copy last assistant response to system clipboard |
| `/compact` | Toggle compact output mode (shorter responses) |
| `/system <text>` | Set persistent system message for this session |
| `/reset` | Clear conversation context, fresh start (keeps session) |
| `/retry` | Resend last message |
| `/export [md|json]` | Export conversation to file |
| `/cost` | Session cost summary |
| `/clear` | Clear scrollback |

## Category 2: Configuration

| Command | Purpose |
|---------|---------|
| `/config` | Show current config summary |
| `/config set <key> <value>` | Set a roko.toml value |
| `/config providers` | List configured providers with health |
| `/config models` | List all available models across providers |
| `/config gates` | Show gate configuration |
| `/model [name]` | Show or switch model |
| `/provider` | Show current auth/provider |

## Category 3: Workspace & Git

| Command | Purpose |
|---------|---------|
| `/status` | Workspace status (signals, episodes, plans, tasks) |
| `/doctor` | Health check (roko init, config, providers) |
| `/diff` | Show git diff (staged + unstaged) |
| `/git` | Show git status |
| `/log [n]` | Show last N git commits (default 5) |
| `/branch` | Show current branch |
| `/changes` | Show changed files since last commit |

## Category 4: File Operations

| Command | Purpose |
|---------|---------|
| `/file <path>` | Read and display a file (with line numbers) |
| `/search <pattern>` | Grep workspace for pattern |
| `/find <pattern>` | Find files matching glob pattern |
| `/tree [path]` | Show directory tree |

## Category 5: Agent & Workflow

| Command | Purpose |
|---------|---------|
| `/agent` | Show current agent identity |
| `/agent list` | List configured agents |
| `/agent <name>` | Switch agent identity (changes system prompt, color) |
| `/effort <level>` | Set effort level (low/medium/high/max) |
| `/gate <name> [on|off]` | Toggle gates (compile, test, clippy) |
| `/run <prompt>` | Execute universal loop (compose→agent→gate→persist) |
| `/plan list` | List available plans |
| `/plan run <dir>` | Execute a plan |
| `/plan generate <prompt>` | Generate a plan from prompt |

## Category 6: PRD & Research

| Command | Purpose |
|---------|---------|
| `/prd idea <text>` | Capture a work item idea |
| `/prd list` | List PRDs |
| `/prd draft <slug>` | Draft a PRD |
| `/research <query>` | Research a topic with citations |

## Category 7: Knowledge & Learning

| Command | Purpose |
|---------|---------|
| `/knowledge <query>` | Query the knowledge store |
| `/knowledge stats` | Show knowledge store stats |
| `/learn` | Show learning state summary |
| `/learn tune` | Tune adaptive thresholds |

## Category 8: Server & Integration

| Command | Purpose |
|---------|---------|
| `/serve` | Start HTTP control plane (background) |
| `/dashboard` | Open TUI dashboard |
| `/index build` | Build code intelligence index |
| `/index search <query>` | Search the code index |

---

## Implementation Notes

### Command Execution Patterns

1. **Instant** (in-process): /config, /status, /stats, /context, /version
2. **Shell-out** (subprocess): /diff, /git, /log, /search, /find, /file, /tree
3. **CLI bridge** (roko subcommand): /run, /plan, /prd, /research, /knowledge
4. **Clipboard** (platform-specific): /copy

### Shell-out Helper

```rust
fn shell_output(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd)
        .args(args)
        .current_dir(workspace_dir())
        .output()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if stdout.is_empty() { stderr.to_string() } else { stdout.to_string() }
        })
        .unwrap_or_else(|e| format!("error: {e}"))
}
```

### CLI Bridge Pattern

For commands that map to roko subcommands, execute the binary and stream output:

```rust
fn roko_subcommand(args: &[&str]) -> String {
    shell_output("cargo", &[&["run", "-p", "roko-cli", "--", ], args].concat())
}
```

Or if the release binary is available:
```rust
fn roko_subcommand(args: &[&str]) -> String {
    shell_output("./target/release/roko", args)
}
```

---

## Priority Order

1. **Session & Display** — immediate utility, trivial to implement
2. **Workspace & Git** — most frequently needed during development
3. **File Operations** — eliminates context-switching to terminal
4. **Configuration** — power-user config management
5. **Agent & Workflow** — bridges chat to full orchestration
6. **PRD & Research** — bridges chat to planning pipeline
7. **Knowledge & Learning** — advanced features
8. **Server & Integration** — server management from chat

---

## Comparison with ACP Runner

| Capability | ACP Runner | Roko Chat (target) |
|-----------|-----------|-------------------|
| Slash commands | 44 | 55+ |
| Workflow templates | 8 (express→full) | Via /run, /plan |
| Config dropdowns | Model, effort, workflow | /model, /effort, /config set |
| Agent management | 10 roles | /agent list, /agent <name> |
| Gate control | Per-session toggles | /gate <name> on/off |
| Progress tracking | Tool call cards | Status bar + /status |
| Research | Integrated | /research <query> |
| File ops | Via tool calls | /file, /search, /find |
| Git integration | None | /diff, /git, /log, /branch |
| Export | None | /export md/json |
| History search | None | Ctrl+R |
| Fuzzy completion | None | Tab dropdown |
