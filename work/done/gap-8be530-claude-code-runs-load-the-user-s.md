+++
id = "gap-8be530"
kind = "gap"
title = "Claude Code runs load the user's own ~/.claude settings, hooks and plugins"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/claude-cli"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "5e50e959a"
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::build_command", "crates/roko-agent/src/claude_cli_agent.rs:370"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q -- \"setting-sources\" crates/roko-agent/src/claude_cli_agent.rs"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Claude Code runs load no user or project setting sources (--setting-sources \"\"), use --strict-mcp-config and CLAUDE_CODE_DISABLE_AUTO_MEMORY=1, and get the workdir's own CLAUDE.md and rules via --add-dir <workdir> with CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1 ('project' was rejected: it walks up to / and loads ~/.claude/CLAUDE.md); guard hooks and key-file deny rules unchanged (ea10d7bd0, bf9a5a573, merged fccbe30a0). Batch 4 gate (work/rust-batch-4; crates tree identical to MAIN after the merges): cargo check --workspace --tests clean; nightly rustfmt clean after fmt-only b0ae64620; clippy -p roko-cli -p roko-core -p roko-agent --no-deps -D warnings clean; lib tests roko-cli 3077, roko-core 1921, roko-agent 2240, 0 failed."
+++
roko passes `--settings` to the Claude CLI (`claude_cli_agent.rs:368`) but never `--setting-sources`, so the invoking user's global and project settings also apply: hooks, plugins, permissions and model defaults. The same plan can behave differently on different machines, and benchmark runs through the Claude CLI are not reproducible. The effect on runs has not been measured.

Fix: pass `--setting-sources` explicitly (or isolate the config directory), and record which settings a run used.

## Notes

- Implemented on `work/gap-8be530` at `ea10d7bd0`, revised at `bf9a5a573`; cargo verification deferred to the batch
  check.
- **Settled isolation, for reuse by the ViabilityBench Claude Code arm (gap-c4f364).** Checked against Claude Code
  2.1.282 with `claude --help` and a read-only look at its bundled source; no session was started.
  - **Flags:** `--setting-sources ""` (two argv elements; `--setting-sources=` parses the same), `--add-dir <workdir>`
    and `--strict-mcp-config` on every run, next to Roko's `--settings <json>` and, when Roko has one,
    `--mcp-config <path>`.
  - **Env:** `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1` and `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1`.
    `CLAUDE_CONFIG_DIR` is not set, so a subscription login (keychain or `~/.claude/.credentials.json`),
    `ANTHROPIC_API_KEY` and `CLAUDE_CODE_OAUTH_TOKEN` work as before. An explicit `with_env_var` value beats both.
  - **What they do:** an empty source list leaves only `--settings` and managed policy enabled. That drops user,
    project and local settings (hooks, `enabledPlugins`, permission rules, `env`, `apiKeyHelper`), skills, agents and
    commands from `.claude` directories, plugin sync, and CLAUDE.md discovery. The `--add-dir` pair then loads the
    workdir's `CLAUDE.md`, `.claude/CLAUDE.md` and `.claude/rules/*.md`, and nothing above it; `CLAUDE.local.md` stays
    out because it needs the `local` source. The workdir is the working directory, so `--add-dir` grants no file
    access. `--strict-mcp-config` resolves `~/.claude.json` and `.mcp.json` servers to none and skips claude.ai
    connectors. Auto-memory ignores setting sources; only the env var (or `autoMemoryEnabled`) turns it off.
  - **Why not `--setting-sources project`:** it keeps `~/.claude/settings.json` and `.claude/settings.local.json`
    out, but its CLAUDE.md discovery walks from the workdir up to `/`. User memory is deduplicated only when the `user`
    source is on, so for any workdir under `$HOME` (every Roko worktree here) the walk loads `~/.claude/CLAUDE.md` and
    `~/.claude/rules/*.md` as project memory, along with any `CLAUDE.md` in the directories between the repo and `/`.
    What `project` adds over the `--add-dir` route is the repo's tracked `.claude/settings.json`;
    `with_setting_sources("project")` opts into that trade.
  - **Rejected:** `--bare` (it also skips the hooks in `--settings` and never reads OAuth or the keychain),
    `--safe-mode` (turns off hooks and MCP) and `--restricted` (refuses bypassPermissions).
  - **Record:** every output carries a `setting_sources` tag (`none` by default). For the probe, the stream-json
    `system`/`init` event lists `plugins`, `mcp_servers`, `skills`, `memory_paths` and `apiKeySource`.
  - **Bench arm (gap-c4f364):** its fixtures should use `--setting-sources ""`, Roko's own value, never `project`,
    with `--strict-mcp-config` and `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`. Keep the `--add-dir <workdir>` pair too, so
    both arms see the task repo's own CLAUDE.md and nothing else. Full isolation is `""` without that pair.
  - **Fresh config dir (the arm's plan step 1):** with `CLAUDE_CONFIG_DIR` set, the macOS keychain entry's name gains
    `-<first 8 hex of sha256(config dir)>`, so the subscription login is not found. Setting
    `CLAUDE_SECURESTORAGE_CONFIG_DIR=` (empty) keeps the default entry name; it is undocumented, so confirm it in the
    probe. `CLAUDE_CODE_OAUTH_TOKEN` from `claude setup-token` is the documented alternative.
- **Behaviour changes:** agents keep the workdir's own CLAUDE.md files but no longer get the user's or an ancestor's,
  nor the repo's `.claude/settings.json`; only the workdir's files load, so a run started in a subdirectory misses the
  repo root's CLAUDE.md. Runs without `--dangerously-skip-permissions` (the `runner` default) no longer inherit the
  user's allow rules.
- **Not covered:** a managed `managed-mcp.json` makes Claude Code refuse `--strict-mcp-config`; Roko's own
  `find_mcp_config` still passes an ancestor's or `$HOME/.mcp.json`; the Bash tool sources a snapshot of the user's
  shell (`~/.claude/shell-snapshots/`); `crates/roko-cli/src/chat_session.rs` and `dispatch_v2.rs` spawn `claude`
  without these flags.
