# W15-D: Demo App Improvements (IMPROVEMENTS 7.1-7.4)

**Priority**: P2 -- demo reliability and maintainability
**Effort**: 2-3 hours
**Files to modify**: 2 files
**Dependencies**: None

## Problem

Four issues in the demo app reduce reliability and maintainability:

1. **Hardcoded timeouts** -- magic numbers (`4000`, `3000`, `8000`, `6000`, `5000`) in `resolveRoko()`, `enterWorkspace()`, and `ensureWorkspaceCwd()` with no way to configure them for different environments.

2. **Unstructured command failures** -- `CommandResult` has `ok: boolean` and optional `error?: string` but no machine-readable failure reason. Callers cannot distinguish timeout vs. WebSocket close vs. command error without parsing error strings.

3. **Duplicated command definitions** -- `prd-pipeline.ts` defines each command twice: once in `PRD_PIPELINE_COMMANDS` (lines 13-22, static display) and once in `prdCommands(ctx)` (lines 26-37, runtime execution). Every change requires updating both arrays.

4. **No AbortController for metrics tracking** -- `trackMetrics()` returns a `setInterval` ID that callers must manually clear. If a command is aborted or the terminal disconnects, the interval leaks.

## Exact Code to Change

### File 1: `demo/demo-app/src/lib/terminal-session.ts` (489 lines)

#### Change 1: Add timeout configuration (7.1)

**Find this code (lines 13-14):**

```typescript
export { stripAnsi } from './strip-ansi';
import { stripAnsi } from './strip-ansi';
```

**Replace with:**

```typescript
export { stripAnsi } from './strip-ansi';
import { stripAnsi } from './strip-ansi';

// ── Timeout configuration ────────────────────────────────────

export interface TimeoutConfig {
  /** Binary detection timeout (resolveRoko). Default: 4000ms */
  binaryDetection: number;
  /** Executable check timeout (test -x). Default: 3000ms */
  execCheck: number;
  /** WebSocket open wait timeout. Default: 8000ms */
  websocketOpen: number;
  /** Shell prompt detection timeout. Default: 6000ms */
  shellPrompt: number;
  /** Workspace cd timeout. Default: 5000ms */
  workspaceCd: number;
}

export const DEFAULT_TIMEOUTS: TimeoutConfig = {
  binaryDetection: 4000,
  execCheck: 3000,
  websocketOpen: 8000,
  shellPrompt: 6000,
  workspaceCd: 5000,
};

let activeTimeouts: TimeoutConfig = { ...DEFAULT_TIMEOUTS };

/** Override timeout values. Merges with defaults. */
export function setTimeouts(overrides: Partial<TimeoutConfig>): void {
  activeTimeouts = { ...DEFAULT_TIMEOUTS, ...overrides };
}

/** Get the current effective timeouts. */
export function getTimeouts(): TimeoutConfig {
  return activeTimeouts;
}
```

**Then** update the 5 hardcoded timeout values to use `activeTimeouts`:

**Timeout 1 -- `resolveRoko()` binary detection (line 54):**

**Find this code:**

```typescript
    4000,
    { silent: true },
  );
```

**Replace with:**

```typescript
    activeTimeouts.binaryDetection,
    { silent: true },
  );
```

**Timeout 2 -- `resolveRoko()` executable check (line 81):**

**Find this code:**

```typescript
    const check = await handle.execCmd(`test -x ${resolvedRoko}`, 3000, { silent: true });
```

**Replace with:**

```typescript
    const check = await handle.execCmd(`test -x ${resolvedRoko}`, activeTimeouts.execCheck, { silent: true });
```

**Timeout 3 -- `ensureWorkspaceCwd()` default parameter (line 118):**

**Find this code:**

```typescript
  timeout = 5000,
```

**Replace with:**

```typescript
  timeout = activeTimeouts.workspaceCd,
```

**Timeout 4 -- `enterWorkspace()` WebSocket wait (line 161):**

**Find this code:**

```typescript
  const wsOk = await waitForOpen(handle, 8000);
```

**Replace with:**

```typescript
  const wsOk = await waitForOpen(handle, activeTimeouts.websocketOpen);
```

**Timeout 5 -- `enterWorkspace()` shell prompt (line 170):**

**Find this code:**

```typescript
  let promptOk = await handle.waitForPrompt(6000);
```

**Replace with:**

```typescript
  let promptOk = await handle.waitForPrompt(activeTimeouts.shellPrompt);
```

---

#### Change 2: Add structured failure reasons to CommandResult (7.2)

**Find this code (lines 198-206):**

```typescript
export interface CommandResult {
  ok: boolean;
  elapsed: number;
  gates: GateResult[];
  cost: string | null;
  tokens: string | null;
  /** Last lines of terminal output when the command failed. */
  error?: string;
}
```

**Replace with:**

```typescript
export type CommandFailureReason =
  | 'timeout'
  | 'ws_closed'
  | 'command_error'
  | 'aborted'
  | 'unknown';

export interface CommandResult {
  ok: boolean;
  elapsed: number;
  gates: GateResult[];
  cost: string | null;
  tokens: string | null;
  /** Last lines of terminal output when the command failed. */
  error?: string;
  /** Machine-readable failure reason (only set when ok=false). */
  failureReason?: CommandFailureReason;
}
```

**Then** populate `failureReason` in `showCmd()` at each failure path:

**Path 1 -- Aborted check (line 275-277):**

**Find this code:**

```typescript
  if (opts?.signal?.aborted) {
    opts?.onLogComplete?.(cmd, false);
    return { ok: false, elapsed: 0, gates: [], cost: null, tokens: null };
  }
```

**Replace with:**

```typescript
  if (opts?.signal?.aborted) {
    opts?.onLogComplete?.(cmd, false);
    return { ok: false, elapsed: 0, gates: [], cost: null, tokens: null, failureReason: 'aborted' };
  }
```

**Path 2 -- Workspace cd failure (lines 282-284):**

**Find this code:**

```typescript
    if (!cwdOk) {
      opts?.onLogComplete?.(cmd, false);
      return { ok: false, elapsed: (Date.now() - startTime) / 1000, gates: [], cost: null, tokens: null };
    }
```

**Replace with:**

```typescript
    if (!cwdOk) {
      opts?.onLogComplete?.(cmd, false);
      return { ok: false, elapsed: (Date.now() - startTime) / 1000, gates: [], cost: null, tokens: null, failureReason: 'command_error' };
    }
```

**Path 3 -- Typing failure (lines 291-293):**

**Find this code:**

```typescript
  if (!typed) {
    opts?.onLogComplete?.(cmd, false);
    return { ok: false, elapsed: 0, gates: [], cost: null, tokens: null };
  }
```

**Replace with:**

```typescript
  if (!typed) {
    opts?.onLogComplete?.(cmd, false);
    return { ok: false, elapsed: 0, gates: [], cost: null, tokens: null, failureReason: 'ws_closed' };
  }
```

**Path 4 -- WebSocket closed before Enter (lines 297-299):**

**Find this code:**

```typescript
  if (!handle.ws || handle.ws.readyState !== WebSocket.OPEN) {
    opts?.onLogComplete?.(cmd, false);
    return { ok: false, elapsed: (Date.now() - startTime) / 1000, gates: [], cost: null, tokens: null };
  }
```

**Replace with:**

```typescript
  if (!handle.ws || handle.ws.readyState !== WebSocket.OPEN) {
    opts?.onLogComplete?.(cmd, false);
    return { ok: false, elapsed: (Date.now() - startTime) / 1000, gates: [], cost: null, tokens: null, failureReason: 'ws_closed' };
  }
```

**Path 5 -- Final return (lines 355-362):**

**Find this code:**

```typescript
  return {
    ok,
    elapsed,
    gates: result.gates,
    cost: result.cost,
    tokens: result.tokens,
    error,
  };
```

**Replace with:**

```typescript
  const failureReason: CommandFailureReason | undefined = ok
    ? undefined
    : !promptOk
      ? 'timeout'
      : 'command_error';

  return {
    ok,
    elapsed,
    gates: result.gates,
    cost: result.cost,
    tokens: result.tokens,
    error,
    failureReason,
  };
```

---

#### Change 3: Add AbortController support to trackMetrics (7.4)

**Find this code (lines 458-488):**

```typescript
export function trackMetrics(
  handle: TerminalHandle,
  opts: {
    onCost?: (cost: string) => void;
    onTokens?: (tokens: string) => void;
    onGate?: (name: string, status: 'pass' | 'fail') => void;
  },
  intervalMs = 500,
): ReturnType<typeof setInterval> {
  let lastCost: string | null = null;
  let lastTokens: string | null = null;
  const seenGates = new Set<string>();

  return setInterval(() => {
    const result = detectFromOutput(handle.outputBuffer);
    if (result.cost && result.cost !== lastCost) {
      lastCost = result.cost;
      opts.onCost?.(result.cost);
    }
    if (result.tokens && result.tokens !== lastTokens) {
      lastTokens = result.tokens;
      opts.onTokens?.(result.tokens);
    }
    for (const gate of result.gates) {
      const key = `${gate.name}:${gate.status}`;
      if (!seenGates.has(key)) {
        seenGates.add(key);
        opts.onGate?.(gate.name, gate.status);
      }
    }
  }, intervalMs);
}
```

**Replace with:**

```typescript
export function trackMetrics(
  handle: TerminalHandle,
  opts: {
    onCost?: (cost: string) => void;
    onTokens?: (tokens: string) => void;
    onGate?: (name: string, status: 'pass' | 'fail') => void;
    signal?: AbortSignal;
  },
  intervalMs = 500,
): ReturnType<typeof setInterval> {
  let lastCost: string | null = null;
  let lastTokens: string | null = null;
  const seenGates = new Set<string>();

  const interval = setInterval(() => {
    const result = detectFromOutput(handle.outputBuffer);
    if (result.cost && result.cost !== lastCost) {
      lastCost = result.cost;
      opts.onCost?.(result.cost);
    }
    if (result.tokens && result.tokens !== lastTokens) {
      lastTokens = result.tokens;
      opts.onTokens?.(result.tokens);
    }
    for (const gate of result.gates) {
      const key = `${gate.name}:${gate.status}`;
      if (!seenGates.has(key)) {
        seenGates.add(key);
        opts.onGate?.(gate.name, gate.status);
      }
    }
  }, intervalMs);

  // Auto-cleanup on abort signal
  if (opts.signal) {
    opts.signal.addEventListener('abort', () => clearInterval(interval), { once: true });
  }

  return interval;
}
```

---

### File 2: `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts` (96 lines)

#### Change 4: Single-source command definitions (7.3)

The file has two parallel arrays: `PRD_PIPELINE_COMMANDS` (lines 13-22) for display and `prdCommands(ctx)` (lines 26-37) for runtime. They have identical `id`, `description`, and `timeout` values that can drift.

**Find this code (lines 1-37):**

```typescript
// --- src/lib/scenario-runners/prd-pipeline.ts ---
import type { ClickableScenario, CommandDef, ScenarioContext } from '../scenarios';
import { showCmd, roko } from '../terminal-session';
import { fetchWorkflowSnapshot } from '../workflow-api';

// ── PRD pipeline idea ────────────────────────────────────────

export const PRD_IDEA =
  'Build a CLI that fetches BTC funding rates from Binance, calculates average funding over 7 days, and alerts when funding exceeds 0.1%';

// ── Static command definitions (display layer, no ctx needed) ─

export const PRD_PIPELINE_COMMANDS: CommandDef[] = [
  { id: 'init',     command: 'roko init',                                                  description: 'Create workspace and config',    timeout: 10000  },
  { id: 'idea',     command: `roko prd idea "${PRD_IDEA}"`,                                  description: 'Capture work item',              timeout: 10000  },
  { id: 'draft',    command: 'roko prd draft new "BTC Funding Alert CLI"',                 description: 'Generate PRD via LLM',           timeout: 600000 },
  { id: 'promote',  command: 'roko prd draft promote btc-funding-alert-cli',               description: 'Promote to published',           timeout: 10000  },
  { id: 'plan',     command: 'roko prd plan btc-funding-alert-cli',                        description: 'Generate implementation plan',   timeout: 600000 },
  { id: 'validate', command: 'roko plan validate .roko/plans',                             description: 'Lint the generated plan',        timeout: 10000  },
  { id: 'run',      command: 'roko plan run .roko/plans --max-retries 1',                  description: 'Execute: agents + gates',        timeout: 600000 },
  { id: 'status',   command: 'roko status',                                                description: 'View results and costs',         timeout: 10000  },
];

// ── Runtime commands factory (ctx-aware, actual command strings) ─

function prdCommands(ctx: ScenarioContext): CommandDef[] {
  return [
    { id: 'init',     command: roko(ctx, 'init'),                                              description: 'Create workspace and config',    timeout: 10000  },
    { id: 'idea',     command: roko(ctx, `prd idea "${PRD_IDEA}"`),                            description: 'Capture work item',              timeout: 10000  },
    { id: 'draft',    command: roko(ctx, 'prd draft new "BTC Funding Alert CLI"'),             description: 'Generate PRD via LLM',           timeout: 600000 },
    { id: 'promote',  command: roko(ctx, 'prd draft promote btc-funding-alert-cli'),           description: 'Promote to published',           timeout: 10000  },
    { id: 'plan',     command: roko(ctx, 'prd plan btc-funding-alert-cli'),                    description: 'Generate implementation plan',   timeout: 600000 },
    { id: 'validate', command: roko(ctx, 'plan validate .roko/plans'),                         description: 'Lint the generated plan',        timeout: 10000  },
    { id: 'run',      command: roko(ctx, 'plan run .roko/plans --max-retries 1'),              description: 'Execute: agents + gates',        timeout: 600000 },
    { id: 'status',   command: roko(ctx, 'status'),                                            description: 'View results and costs',         timeout: 10000  },
  ];
}
```

**Replace with:**

```typescript
// --- src/lib/scenario-runners/prd-pipeline.ts ---
import type { ClickableScenario, CommandDef, ScenarioContext } from '../scenarios';
import { showCmd, roko, getRoko } from '../terminal-session';
import { fetchWorkflowSnapshot } from '../workflow-api';

// ── PRD pipeline idea ────────────────────────────────────────

export const PRD_IDEA =
  'Build a CLI that fetches BTC funding rates from Binance, calculates average funding over 7 days, and alerts when funding exceeds 0.1%';

// ── Command templates (single source of truth) ──────────────

interface CommandTemplate {
  id: string;
  /** roko subcommand (without binary path or --model flag) */
  subcommand: string;
  /** Clean display text for sidebar (no model, no resolved path) */
  display: string;
  description: string;
  timeout: number;
  /** Whether --model flag should be injected at runtime */
  needsModel: boolean;
}

const TEMPLATES: CommandTemplate[] = [
  { id: 'init',     subcommand: 'init',                                     display: 'roko init',                                                  description: 'Create workspace and config',  timeout: 10000,  needsModel: false },
  { id: 'idea',     subcommand: `prd idea "${PRD_IDEA}"`,                   display: `roko prd idea "${PRD_IDEA}"`,                                description: 'Capture work item',            timeout: 10000,  needsModel: false },
  { id: 'draft',    subcommand: 'prd draft new "BTC Funding Alert CLI"',    display: 'roko prd draft new "BTC Funding Alert CLI"',                 description: 'Generate PRD via LLM',         timeout: 600000, needsModel: true  },
  { id: 'promote',  subcommand: 'prd draft promote btc-funding-alert-cli',  display: 'roko prd draft promote btc-funding-alert-cli',               description: 'Promote to published',         timeout: 10000,  needsModel: false },
  { id: 'plan',     subcommand: 'prd plan btc-funding-alert-cli',           display: 'roko prd plan btc-funding-alert-cli',                        description: 'Generate implementation plan', timeout: 600000, needsModel: true  },
  { id: 'validate', subcommand: 'plan validate .roko/plans',                display: 'roko plan validate .roko/plans',                             description: 'Lint the generated plan',      timeout: 10000,  needsModel: false },
  { id: 'run',      subcommand: 'plan run .roko/plans --max-retries 1',     display: 'roko plan run .roko/plans --max-retries 1',                  description: 'Execute: agents + gates',      timeout: 600000, needsModel: true  },
  { id: 'status',   subcommand: 'status',                                   display: 'roko status',                                                description: 'View results and costs',       timeout: 10000,  needsModel: false },
];

// ── Generated from templates ─────────────────────────────────

/** Static command definitions for display (sidebar, step list). */
export const PRD_PIPELINE_COMMANDS: CommandDef[] = TEMPLATES.map(t => ({
  id: t.id,
  command: t.display,
  description: t.description,
  timeout: t.timeout,
}));

/** Build a runtime command string from a template and scenario context. */
function runtimeCommand(ctx: ScenarioContext, template: CommandTemplate): string {
  if (template.needsModel) {
    return roko(ctx, template.subcommand);
  }
  const bin = getRoko();
  return `${bin} ${template.subcommand}`;
}

/** Get runtime CommandDef for a specific command ID. */
function getRuntimeCmd(ctx: ScenarioContext, commandId: string): CommandDef | undefined {
  const template = TEMPLATES.find(t => t.id === commandId);
  if (!template) return undefined;
  return {
    id: template.id,
    command: runtimeCommand(ctx, template),
    description: template.description,
    timeout: template.timeout,
  };
}
```

**Then** update the `runCommand` method (lines 66-69):

**Find this code:**

```typescript
    const commands = prdCommands(ctx);
    const cmd = commands.find(c => c.id === commandId);
    if (!cmd) return { ok: false, error: 'Unknown command' };
```

**Replace with:**

```typescript
    const cmd = getRuntimeCmd(ctx, commandId);
    if (!cmd) return { ok: false, error: 'Unknown command' };
```

**Import update**: The import on line 3 needs `getRoko` added:

**Find this code (line 3):**

```typescript
import { showCmd, roko } from '../terminal-session';
```

**Replace with:**

```typescript
import { showCmd, roko, getRoko } from '../terminal-session';
```

(This is already included in the full replacement above.)

## Agent Prompt

This batch has 4 changes across 2 TypeScript files. The agent should:

1. Start with `terminal-session.ts`:
   - Add `TimeoutConfig` interface and `activeTimeouts` after imports (line 14)
   - Replace 5 hardcoded timeouts with `activeTimeouts.*` references
   - Add `CommandFailureReason` type and `failureReason` field to `CommandResult`
   - Add `failureReason` to each failure return path in `showCmd()`
   - Add `signal?: AbortSignal` to `trackMetrics()` opts and auto-cleanup logic

2. Then `prd-pipeline.ts`:
   - Replace the two parallel arrays with `TEMPLATES` + generated `PRD_PIPELINE_COMMANDS`
   - Add `getRuntimeCmd()` helper
   - Update `runCommand()` to use `getRuntimeCmd()`
   - Add `getRoko` to the import

No new dependencies. All changes are pure TypeScript refactoring.

## Verification

```bash
# 1. TypeScript compilation
cd demo/demo-app && npx tsc --noEmit

# 2. Verify no duplicate arrays
grep -c 'prdCommands' demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts
# Should be 0 (function removed, replaced by getRuntimeCmd)

# 3. Verify TimeoutConfig is exported
grep -c 'TimeoutConfig' demo/demo-app/src/lib/terminal-session.ts
# Should be >= 2 (interface + DEFAULT_TIMEOUTS)

# 4. Verify CommandFailureReason is exported
grep -c 'CommandFailureReason' demo/demo-app/src/lib/terminal-session.ts
# Should be >= 2 (type + usage)

# 5. Verify AbortSignal in trackMetrics
grep -c 'AbortSignal' demo/demo-app/src/lib/terminal-session.ts
# Should be >= 1

# 6. Smoke test (run dev server briefly)
cd demo/demo-app && npm run dev &
sleep 3 && kill %1
```

## Why This Matters

- Configurable timeouts let the demo work in slow CI environments or fast local runs without code changes
- Structured failure reasons let the UI show "Timed out" vs. "Connection lost" vs. "Command failed" instead of just "Error"
- Single-source command templates eliminate drift between display and runtime arrays
- AbortController support prevents interval leaks when commands are cancelled mid-flight

## Audit Status

Audited: 2026-05-05. PASS no changes needed
