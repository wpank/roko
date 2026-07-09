# ACP Runner — Overnight Codex Batch Runner for `roko-acp`

Creates the `roko-acp` crate from scratch via 18 batches in 4 groups. Implements ACP (Agent Client Protocol) JSON-RPC server over stdio, enabling Roko to work from any ACP-compatible editor (JetBrains, Zed, Neovim, VS Code).

## Quick start

```bash
# See what will run
bash tmp/acp-runner/run-acp.sh --list

# Dry run (compose prompts, no Codex)
bash tmp/acp-runner/run-acp.sh --dry-run --group scaffold

# Night 1: scaffold + core (ACP01–ACP08)
bash tmp/acp-runner/run-acp.sh --group scaffold,core

# Night 2: bridges (ACP09–ACP14)
bash tmp/acp-runner/run-acp.sh --continue last --group bridges

# Night 3: config + tests (ACP15–ACP18)
bash tmp/acp-runner/run-acp.sh --continue last --group config

# Re-run a failed batch
bash tmp/acp-runner/run-acp.sh --continue last --only ACP05 --force

# Verify only (no Codex, just run gates)
bash tmp/acp-runner/run-acp.sh --verify-only --continue last
```

## Architecture

```
run-acp.sh (main)
  ├── lib/common.sh    (batch metadata, paths, logging)
  ├── lib/spawn.sh     (Codex exec, prompt composition)
  └── lib/verify.sh    (cargo check/clippy/test gates, scope gate, commit)
```

Each batch follows the same cycle:
1. Compose prompt = context pack + delegation guidance + batch prompt
2. Spawn Codex in worktree with `codex exec --full-auto`
3. Verify: scope gate → diff gate → required-terms → cargo check → clippy → tests
4. Commit if OK, or backup state + retry

## Batch groups

| Group | Batches | What |
|-------|---------|------|
| **scaffold** | ACP01–03 | Crate skeleton, protocol types, stdio transport |
| **core** | ACP04–08 | Handler, sessions, streaming, CLI command, conformance tests |
| **bridges** | ACP09–14 | FS, terminal, permissions, gates, plans, usage bridges |
| **config** | ACP15–18 | Config options, slash commands, elicitation, lifecycle tests |

## Verify gates (per batch)

1. **Scope** — Only allowed paths modified (`crates/roko-acp/`, plus `crates/roko-cli/` for ACP07)
2. **Diff** — Batch produced some changes
3. **Required terms** — Expected vocabulary present in changed files
4. **Cargo check** — Code compiles (temp CARGO_TARGET_DIR, cleaned after)
5. **Clippy** — No warnings (`-D warnings`)
6. **Tests** — Batch-specific test commands (where applicable)

## Environment

| Variable | Default | Description |
|----------|---------|-------------|
| `ACP_MODEL` | `gpt-5.4` | Codex model |
| `ACP_REASONING` | `high` | Reasoning effort |
| `ACP_TIMEOUT` | `5400` | Per-batch timeout (90 min) |
| `ACP_MAX_RETRIES` | `2` | Retries per batch |
| `ACP_BASE_REF` | `HEAD` | Git ref for new worktree |
| `ACP_MAX_BATCHES` | `0` | Batch limit (0 = unlimited) |

## Logs

Runtime logs go to `tmp/acp-runner/logs/<run-id>/`:
- `manifest.env` — Run config
- `status.tsv` — Per-batch status timeline
- `ACP01.log` — Codex output per batch
- `ACP01.result` — Final status per batch
- `prompts/ACP01.prompt.md` — Composed prompt snapshot
- `backups/` — Worktree state on failure
