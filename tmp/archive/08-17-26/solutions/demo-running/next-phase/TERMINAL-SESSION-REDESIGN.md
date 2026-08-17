# Terminal Session Redesign — Making Demo Commands Actually Work

**Date**: 2026-05-04
**Status**: Diagnosis complete, implementation pending
**Scope**: Demo app terminal session layer — the code that types commands into the PTY and interprets results

---

## The Core Problem

The PRD Pipeline (and all clickable scenarios) fail at command execution. Step 1 may succeed by luck; step 2+ fail with garbled error messages. The user sees raw terminal protocol garbage as error text, and exit codes are misdetected.

**Screenshot evidence**: `roko init` succeeds (8.5s), `prd idea` fails with error text showing the command itself (`roko --repo '/var/folders/bn/ks_s66191...'`) instead of any actual error message.

---

## Root Causes (priority order)

### P0: `resolveRoko` reads its own echo — always resolves to bare `'roko'`

**File**: `demo/demo-app/src/lib/terminal-session.ts` — `resolveRoko()`

`resolveRoko` runs a detection command via `execCmd`:
```
command -v roko >/dev/null 2>&1 && echo RP || { test -x ./target/release/roko && echo "RR:..." || { test -x ./target/debug/roko && echo "RD:..." || echo RN; }; }
```

Then checks `handle.outputBuffer` for markers (`RP`, `RR:`, `RD:`, `RN`).

**The bug**: `execCmd` clears `outputBuffer`, then the PTY echoes the full command text back into the buffer. The echoed command text **contains all markers as literal strings** (e.g., `echo RP`, `echo "RR:..."`, etc.). So `buf.includes('RP')` **always matches the echo**, not the command output.

Result: `resolvedRoko = 'roko'` (bare), which doesn't exist in the PTY environment (the alias is in `.zshrc`, but the PTY uses a custom ZDOTDIR that overrides it). No release binary exists either — only `target/debug/roko`.

**This is why every command after `resolveRoko` fails with exit 127.**

**Why step 1 sometimes succeeds**: Module-level caching (`rokoResolved = true`) means if a previous page load or scenario resolved correctly, the stale value persists. Or the workspace API already ran `roko init` server-side, so the PTY command is redundant.

**Fix**: Use unique output markers that can't appear in the echo, OR parse only the output after the echo line, OR resolve the binary server-side.

### P1: `roko()` command builder injects redundant `--repo` (300+ char commands)

**File**: `demo/demo-app/src/lib/terminal-session.ts` — `roko()` function

Every command gets `--repo '/var/folders/bn/ks_s66191vb0zzvs08qcw_gh0000gn/T/roko-prd-pipeline-1777902515017'` injected, producing 300+ character commands that are typed **character by character** at ~7.5ms each = 2+ seconds of typing animation per command.

But `showCmd` already calls `ensureWorkspaceCwd()` to `cd` into the workspace before typing. Since the shell is already in the workspace directory, `--repo` is entirely redundant. The CLI defaults to cwd when `--repo` is not specified.

**Impact**: Fragile (long commands + PTY wrapping), slow (2s typing per command), visually ugly (user sees a wall of path text), and the `--repo` path is what appears as "error text" when exit code detection fails.

### P2: `--model` injected into commands that don't use it

**File**: `demo/demo-app/src/lib/terminal-session.ts` — `roko()` function

`prd idea`, `prd draft promote`, `plan validate`, `status` don't use models at all, but every command gets `--model 'glm-5-1'` injected. Adds visual noise and command length.

### P3: Two parallel command definitions (display vs. runtime)

**File**: `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts`

- `PRD_PIPELINE_COMMANDS` (static): clean `roko prd idea "..."` — shown in the sidebar
- `prdCommands(ctx)` (runtime): bloated `{debug_path} --repo '{temp_path}' --model 'glm-5-1' prd idea "..."` — typed into terminal

Users see clean commands in the sidebar but incomprehensible text in the terminal. When errors occur, the error text shows the runtime command, not matching what the sidebar displays.

### P4: Error capture reads clobbered buffer

**File**: `demo/demo-app/src/lib/terminal-session.ts` — `showCmd()`

~~After detecting gates/cost from output, `showCmd` runs `execCmd('(exit $?)')` which clears the output buffer. Then it tries to capture error text from `handle.outputBuffer` — which is now empty or contains only the exit-check echo.~~

**DONE** — Fixed in current session. `commandOutput` snapshot taken before exit-check.

### P5: `stripAnsi` was broken (3 implementations, 2 incomplete)

~~Two of three `stripAnsi` implementations only handled basic CSI sequences, missing OSC markers, bracketed paste, charset selection, etc.~~

**DONE** — Fixed in current session. Canonical `strip-ansi.ts` module created, all copies replaced with imports.

### P6: `resetRokoResolution()` never called on scenario reset

~~Module-level `resolvedRoko` cache was never invalidated when switching or resetting scenarios.~~

**DONE** — Fixed in current session. `handleReset` in ScenarioSlot now calls `resetRokoResolution()` and resets `workspaceEnteredRef`.

---

## The Right Design

The current architecture is fundamentally wrong: it types shell commands character-by-character into a WebSocket-connected PTY, scrapes terminal output for markers, and infers exit codes from `$?` checks. Every layer introduces fragility.

### Principle: Separate display from execution

The demo should show **clean, readable commands** to the user while executing them **reliably** behind the scenes. The terminal is a **display surface**, not the execution mechanism.

### Concrete changes:

#### Change 1: Server-side binary resolution (kills P0)

**Files**: `crates/roko-serve/src/routes/workspaces.rs`, `demo/demo-app/src/lib/terminal-session.ts`

The workspace creation API (`POST /api/workspaces`) already knows where the roko binary is — it runs `roko init` itself. Extend the response to include the resolved binary path:

```json
{
  "id": "roko-prd-pipeline-...",
  "path": "/var/folders/.../roko-prd-pipeline-...",
  "ready": true,
  "roko_bin": "/Users/will/dev/nunchi/roko/roko/target/debug/roko"
}
```

The serve process can resolve this at startup (it already has the workdir) and return it with every workspace. The demo-app stores this and uses it directly — no PTY-based detection needed.

**Fallback**: If the API doesn't return `roko_bin`, fall back to the current PTY detection but with unique markers (see Change 2).

#### Change 2: Fix PTY-based detection with unique markers (backup for P0)

**File**: `demo/demo-app/src/lib/terminal-session.ts` — `resolveRoko()`

Use markers that cannot appear in the echo of the detection command:

```typescript
const marker = `__RK_${Date.now().toString(36)}__`;
const result = await handle.execCmd(
  `command -v roko >/dev/null 2>&1 && echo "${marker}RP" || { test -x ./target/release/roko && echo "${marker}RR:$PWD/target/release/roko" || { test -x ./target/debug/roko && echo "${marker}RD:$PWD/target/debug/roko" || echo "${marker}RN"; }; }`,
  4000,
  { silent: true },
);
const buf = handle.outputBuffer;
// Only match markers with the unique prefix — can't match echo text
if (buf.includes(`${marker}RP`)) { ... }
else if (buf.includes(`${marker}RR:`)) { ... }
```

The dynamic marker (`__RK_m2abc7x__RP`) cannot appear in the echoed command text because the command was constructed after the marker was generated.

#### Change 3: Drop `--repo`, simplify `roko()` builder (kills P1 + P2)

**File**: `demo/demo-app/src/lib/terminal-session.ts` — `roko()` function

Since `showCmd` already calls `ensureWorkspaceCwd()` to `cd` into the workspace before every command, `--repo` is redundant. And `--model` should only be injected for commands that actually dispatch to an LLM.

```typescript
const LLM_COMMANDS = new Set(['prd draft new', 'prd draft edit', 'prd plan', 'plan run', 'run']);

export function roko(ctx: ScenarioContext, subcommand: string): string {
  const bin = getRoko();
  const parts = [bin];
  // Only inject --model for commands that dispatch to an LLM
  if (ctx.activeModel && LLM_COMMANDS.has(subcommand.split(/\s+/).slice(0, 3).join(' ').replace(/\s+".*/, ''))) {
    parts.push('--model', shellQuote(ctx.activeModel));
  }
  parts.push(subcommand);
  return parts.join(' ');
}
```

This produces clean commands like:
- `roko init` (no flags)
- `roko prd idea "Build a CLI..."` (no flags)
- `roko prd draft new --model 'glm-5-1' "BTC Funding Alert CLI"` (model only where needed)
- `roko status` (no flags)

**Bonus**: commands are now short enough to read in the terminal without wrapping.

#### Change 4: Unify command definitions (kills P3)

**File**: `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts`

Single source of truth. Static definitions for display, runtime function for the actual command string:

```typescript
export const PRD_PIPELINE_COMMANDS: CommandDef[] = [
  { id: 'init',     display: 'roko init',                                    description: 'Create workspace and config',    timeout: 10000  },
  { id: 'idea',     display: `roko prd idea "${PRD_IDEA}"`,                  description: 'Capture work item',              timeout: 10000  },
  { id: 'draft',    display: 'roko prd draft new "BTC Funding Alert CLI"',   description: 'Generate PRD via LLM',           timeout: 180000 },
  // ...
];
```

The `display` field is what the sidebar and command list show. The `runCommand` method still uses `roko(ctx, subcommand)` for the actual execution — but since we dropped `--repo` (Change 3), the runtime command is now close to the display string (differing only by resolved binary path and `--model` where needed).

This requires adding a `display` field to `CommandDef` (or renaming the existing `command` field to `display` and building the executable command in `runCommand`).

#### Change 5: `workspaceEnteredRef` set consistently (bug fix)

**File**: `demo/demo-app/src/pages/Demo/ScenarioSlot.tsx`

`handlePlay` calls `enterWorkspace(entries[0], wsPath)` for clickable scenarios (line 763) but doesn't set `workspaceEnteredRef.current = true` on success. This causes `handleClickableRun` to re-run `enterWorkspace` on every first click, which re-runs `resolveRoko` (with its broken detection).

Fix: Set `workspaceEnteredRef.current = true` after successful `enterWorkspace` in `handlePlay`.

#### Change 6: Skip `roko init` in demo pipeline (it's redundant)

**File**: `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts`

The workspace API (`POST /api/workspaces`) already creates a fully initialized workspace with `.roko/` and `roko.toml`. Running `roko init` in the terminal is redundant — it just says "roko.toml already exists; leaving untouched." Either:
- Remove the `init` step from the pipeline, OR
- Make the `init` step a no-op that just shows a success message, OR
- Have the workspace API NOT init (just create the dir), and let the pipeline's `roko init` do the real work

The cleanest option: have the workspace API create just an empty directory (maybe with git init), and let `roko init` do the real initialization in the terminal where the user can see it. This makes the demo honest — the user sees the actual init happening.

---

## Implementation Order

### Already done (this session)
- [x] P4: Error capture buffer snapshot (commandOutput before exit-check)
- [x] P5: Canonical stripAnsi module + all copies replaced
- [x] P6: resetRokoResolution called on scenario reset

### Phase 1: Make it work (P0 fix — binary resolution)

| # | Change | Files | Est. |
|---|--------|-------|------|
| 1a | Fix `resolveRoko` with unique markers | `terminal-session.ts` | 15m |
| 1b | Set `workspaceEnteredRef` in `handlePlay` | `ScenarioSlot.tsx` | 5m |

After Phase 1, commands should actually execute in the PTY (right binary found).

### Phase 2: Make it clean (P1+P2+P3 — command construction)

| # | Change | Files | Est. |
|---|--------|-------|------|
| 2a | Drop `--repo` from `roko()` builder | `terminal-session.ts` | 10m |
| 2b | Only inject `--model` for LLM commands | `terminal-session.ts` | 15m |
| 2c | Add `display` field to CommandDef, unify definitions | `scenarios.ts`, `prd-pipeline.ts` | 30m |

After Phase 2, commands are short, readable, and match what the sidebar shows.

### Phase 3: Make it robust (server-side resolution + workspace simplification)

| # | Change | Files | Est. |
|---|--------|-------|------|
| 3a | Return `roko_bin` from workspace API | `routes/workspaces.rs` | 30m |
| 3b | Use server-provided binary path in demo | `terminal-session.ts`, type updates | 15m |
| 3c | Simplify workspace creation (empty dir, let init run in terminal) | `routes/workspaces.rs`, `prd-pipeline.ts` | 30m |

After Phase 3, binary resolution is bulletproof (server-side), and the demo honestly shows initialization.

---

## Verification

After all changes:

1. `cd demo/demo-app && ./node_modules/.bin/tsc --noEmit` — type check passes
2. Start dev servers, open PRD Pipeline, click each step in order:
   - `roko init` — should show real initialization (not "already exists")
   - `roko prd idea "..."` — should succeed, show emoji output
   - All subsequent steps should execute with clean, readable commands
3. Remove roko from PATH entirely — should still work (server-side resolution)
4. Reset scenario, replay — binary re-resolves, all steps work
5. Commands in terminal should be short and match sidebar display

---

## Files Touched (complete list)

| File | Action | Phase |
|------|--------|-------|
| `demo/demo-app/src/lib/strip-ansi.ts` | **Created** (done) | -- |
| `demo/demo-app/src/hooks/useTerminal.ts` | Import stripAnsi (done) | -- |
| `demo/demo-app/src/lib/terminal-session.ts` | Fix resolveRoko markers, drop --repo, model gating | 1a, 2a, 2b |
| `demo/demo-app/src/lib/scenario-helpers.ts` | Re-export stripAnsi (done) | -- |
| `demo/demo-app/src/lib/scenarios.ts` | Add `display` to CommandDef | 2c |
| `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts` | Unify command defs, use display field | 2c |
| `demo/demo-app/src/pages/Demo/ScenarioSlot.tsx` | Set workspaceEnteredRef in handlePlay | 1b |
| `crates/roko-serve/src/routes/workspaces.rs` | Return roko_bin in workspace response | 3a |

---

## Relationship to Existing Batches

This document supersedes parts of:
- **W4-C** (prd-pipeline-redesign.md) — the `roko()` builder redesign and command unification
- **DONE.md** item "roko() function auto-injects --repo + --model" — this is the thing that's **wrong** and needs to be undone

It does NOT overlap with:
- W0-A through W0-G (CLI-side fixes — model resolution, dispatch routing, etc.)
- W1-A through W1-C (plan run path, extraction, schema)
- W2-A through W2-E (output quality)
- The pipeline audit issues (API keys, model resolution, cost tracking) — those are CLI/backend problems, not demo session problems
