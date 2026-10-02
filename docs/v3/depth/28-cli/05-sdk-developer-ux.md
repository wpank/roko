# 28.05 -- SDK Developer UX

> Depth file for [28-CLI.md](../../28-CLI.md) -- v1/12/19.

---

## Workspace Setup

### Interactive Wizard

`roko setup` runs an interactive wizard that:

1. Detects available LLM providers (API keys, CLI tools)
2. Initializes the `.roko/` workspace directory
3. Creates `roko.toml` with detected providers
4. Verifies configuration

```bash
$ roko setup
Detecting providers...
  Found: ANTHROPIC_API_KEY (set)
  Found: claude CLI (installed)
  Found: OPENAI_API_KEY (set)
Creating workspace...
  Created: .roko/
  Created: roko.toml
Verifying...
  Provider anthropic: OK
  Provider openai: OK
Setup complete.
```

### Minimal Bootstrap

For non-interactive environments:

```bash
roko init                 # Create .roko/ and roko.toml
roko config validate      # Verify config schema
roko doctor               # Check workspace health
```

## Configuration Management

### Config Presets

Apply validated configuration presets without manual editing:

```bash
# Preview a preset change
roko config preset gates --dry-run

# Apply a routing preset
roko config preset routing --yes

# Apply a budget preset
roko config preset budget --yes

# Set model configuration
roko config preset model --yes
```

### Config Inspection

```bash
# Show current config
roko config show

# Show config file path
roko config path

# Validate against schema
roko config validate

# Run config health check (read-only)
roko config doctor

# List all recognized environment variables
roko config env

# Export config as env vars for deployment
roko config export --target railway
```

### Secret Management

```bash
# Set a secret (profile-aware)
roko config secrets set ANTHROPIC_API_KEY

# List stored secrets
roko config secrets list

# Rotate a secret
roko config secrets rotate ANTHROPIC_API_KEY

# Check for exposed secrets
roko config check-secrets
```

## Learning and Inspection

### Read-Only Subsystem Inspection

The `roko learn inspect` commands provide read-only views into learning state:

```bash
# Gate thresholds (EMA values per rung)
roko learn inspect gates

# Cascade routing state (per-tier success rates)
roko learn inspect routing

# Budget tracking (spent/remaining per session)
roko learn inspect budget
```

### Learning Overview

```bash
# Full learning state overview
roko learn all

# Cascade router state
roko learn router

# Prompt A/B experiment status
roko learn experiments

# Efficiency metrics
roko learn efficiency

# Episode history
roko learn episodes

# T0 reflex rules (count, top 5 by hits, recent demotions)
roko learn reflexes
```

## Development Workflow

### Plan-First Development

```bash
# Write a plan, review it, run it
roko run --plan "Add pagination to the API"
# 1. Writes plans/<slug>/ (tasks.toml + plan.md)
# 2. Shows the plan and asks for approval (--yes skips the question)
# 3. Executes the plan via the Graph engine
# 4. Validates each task with its gates
# 5. Reports results

# Or in steps: write the plan, edit it, then run it
roko plan generate "Add pagination to the API"
roko run plans/<slug>
```

### Direct Task Execution

```bash
# Small change: runs as one checked task
roko run "Fix the typo in README"

# Force the size: trivial/simple run one task; standard/complex write a plan first
roko run "Refactor the auth module" --complexity standard
roko run "Add WebSocket support" --complexity complex
```

### Research Before Coding

```bash
# Think about a question without changing files
roko think "What's the best way to implement rate limiting?"

# Deep research with citations
roko research topic "WebSocket authentication patterns"

# Improve a generated plan with research before running it
roko research enhance-plan auth-improvements
```

## Diagnostic Tools

### Workspace Doctor

```bash
# Full workspace health check
roko doctor

# Disk usage analysis
roko doctor disk
# Reports: free space, stale targets, worktree storage, log sizes

# Network connectivity
roko doctor network
# Checks: provider APIs, relay connectivity, webhook endpoints

# Clean orphaned files
roko doctor clean
# Removes: temp files, .corrupted files, stale locks in .roko/learn/
```

### Plan Diagnostics

```bash
# Structured failure diagnosis
roko diagnose plan-123
# Returns JSON with:
#   - Failed task details
#   - Gate verdicts
#   - Agent logs
#   - Suggested fixes

# Validate a plan without executing
roko plan validate plans/my-plan/
```

### Cache Management

```bash
# Report cache pressure
roko cache status

# Safe prune with dry-run (default)
roko cache prune --target-budget-gb 96

# Actually delete
roko cache prune --apply --target-budget-gb 96
```

## Output Formatting

### JSON Mode

All commands support `--json` for machine-readable output:

```bash
roko status --json
roko plan status plans/my-plan --json
roko learn gates --json
roko agent list --json
```

### Quiet Mode

`--quiet` suppresses progress spinners and non-essential output:

```bash
roko run "Fix the import" --quiet
```

### Log Format

`--log-format` controls tracing output:

```bash
roko serve --log-format json    # Structured JSON logs
roko serve --log-format compact # Single-line logs
roko serve --log-format pretty  # Human-readable (default)
```

## Error Messages

The CLI uses structured exit codes (see `exit_codes.rs`):

| Code | Meaning | Example |
|---|---|---|
| 0 | Success | Command completed normally |
| 1 | General failure | Config error, invalid arguments |
| 2 | Agent failure | LLM dispatch failed, gate rejection |
| 3 | System error | I/O failure, permission denied |

Error messages include actionable suggestions when possible:

```
error: no provider has resolvable credentials
hint: set ANTHROPIC_API_KEY or install the claude CLI
hint: run `roko config providers discover` to detect available providers
```

## Source

- `crates/roko-cli/src/main.rs` -- CLI definition and global flags
- `crates/roko-cli/src/commands/` -- Subcommand implementations
- `crates/roko-cli/src/doctor.rs` -- Workspace diagnostics
- `crates/roko-cli/src/config_cmd.rs` -- Config management
