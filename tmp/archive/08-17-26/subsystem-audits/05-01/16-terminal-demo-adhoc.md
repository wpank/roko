# 16 — Terminal And Demo Ad-Hoc Fixes

Scope: `crates/roko-serve/src/terminal.rs`, `demo/demo-app/src/hooks/useTerminal.ts`, `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts`, `demo/demo-app/src/pages/Demo/ScenarioSlot.tsx`

The terminal/demo changes are classic runner-style fixes: make prompt detection pass, make scenario startup look cleaner, avoid stale WebSocket cleanup. Some are directionally useful, but several solve symptoms by mutating the environment or weakening the signal that the demo is actually connected to a real shell.

## Findings

### HIGH: Server rewrites interactive shell startup with a temp `ZDOTDIR`

`terminal.rs:138-145` creates a temp directory and writes a minimal `.zshrc` for every default shell session, then sets `ZDOTDIR` so the user's normal shell config is bypassed.

That makes prompt detection deterministic, but it changes the user's shell environment. Tools initialized by `.zshrc`, PATH modifications, aliases, language managers, and project setup can disappear from demo terminals. The demo may become easier to scrape while becoming less representative of real usage.

Expected design: prompt readiness should use an explicit terminal protocol marker or server-side session lifecycle event, not shell prompt scraping. If a clean shell is required for demos, it should be an explicit demo-mode option, not the default terminal behavior.

### MEDIUM: Temp `ZDOTDIR` directories are never cleaned up

The path created at `terminal.rs:141` is only passed to the child process. `PtySession` stores the child, master, writer, and generation (`terminal.rs:175-182`), but not the temp directory. `destroy_session` kills the child (`terminal.rs:225-231`), yet it cannot remove the generated directory.

Expected design: store temp resources in the session and clean them up on session destroy. Better: avoid creating temp shell config for normal sessions.

### MEDIUM: Generation counter does not actually protect against ID reuse during creation failure

The WebSocket path destroys the old session at `terminal.rs:322-323`, then creates a new session. If creation fails, it continues with `reader = None` and `sess_generation = 0` (`terminal.rs:345-351`). The WebSocket still upgrades and `handle_ws` accepts input, but every input send fails silently because no session exists (`terminal.rs:405`, `terminal.rs:408`).

Expected design: failed PTY creation should reject the WebSocket upgrade or send a typed terminal error and close. A connected-but-dead terminal makes the frontend diagnose prompt timeout instead of the real spawn error.

### MEDIUM: Prompt regex expansion increases false positives

`useTerminal.ts:12` expands the prompt regex to include bare `>`, arrows, and several glyphs. That can match normal command output ending in `>`, not just shell prompts. This is especially risky because `execCmd`/scenario progression depends on prompt detection.

Expected design: use explicit markers around commands, such as `printf '\n__ROKO_DONE_$id:$?\n'`, and wait for that marker. Prompt detection should be a fallback for manual interactive use, not the driver for automated scenario correctness.

### MEDIUM: Demo clears only the scraper buffer, not the visible terminal

`prd-pipeline.ts:226-228` changed `clearTerminal()` to `main.outputBuffer = ''`. The comment says it keeps visible output while clearing prompt detection state. That can leave setup output visible while the state machine proceeds as if the terminal is clean.

This is not necessarily wrong for user experience, but it confirms the UI has two sources of truth: what the user sees and what automation scrapes. When they diverge, scenario success can be a buffer artifact.

Expected design: scenario state should come from typed command results or API/workflow events. The visible terminal should be observational, not the source of workflow truth.

### LOW: Workspace creation was changed without documenting semantics

`ScenarioSlot.tsx:716` now calls `createWs` instead of `ensureWorkspace`. That may be correct if every scenario must start fresh, but it changes persistence/reuse semantics and can mask bugs related to resume, existing state, or idempotent setup.

Expected design: scenario definitions should explicitly state whether they require a fresh workspace or reusable workspace, and the runner should enforce that mode.

## Root Cause

The terminal path is still designed around transcript scraping. Once prompt scraping is the product signal, fixes naturally target shell prompts and output buffers instead of the real lifecycle: PTY spawned, command sent, command exited, workflow event emitted, artifact validated.

## Fix Direction

1. Add a command execution protocol with explicit done markers and exit codes.
2. Make terminal spawn failures typed WebSocket errors, not silent dead sessions.
3. Remove default `ZDOTDIR` rewriting or gate it behind explicit demo mode.
4. Store and clean any temp resources owned by `PtySession`.
5. Use API/workflow events as demo truth; keep terminal output as display only.
