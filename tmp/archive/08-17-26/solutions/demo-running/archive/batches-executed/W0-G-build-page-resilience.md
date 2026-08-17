# W0-G: BUILD Page Frontend Timeout + Resilience

**Priority**: P1 — BUILD page shows timeout errors and poor error UX
**Effort**: 45 minutes
**Files to modify**: 3-4 files (demo app)
**Dependencies**: W0-D (routing fix), W0-E (max_tokens fix)

## Problem

The BUILD page (`demo/demo-app/src/pages/Builder.tsx`) shows `waitForPrompt timed out after 120000 ms` in the browser console, and the UI doesn't communicate failure well.

### Issue 1: 120s timeout is too short for LLM operations

`roko run` can take 30-120 seconds for a SINGLE LLM call, plus tool iterations. With glm51 doing 6+ tool iterations at 30s each, 120s is easily exceeded. The timeout should be much higher for BUILD page operations, or the architecture should change.

### Issue 2: No streaming progress feedback

The BUILD page waits for the entire `roko run` command to complete before showing any output. The terminal emulator shows raw output, but the Gates panel only updates after the command finishes. There's no indication that the LLM is "thinking" or that tool iterations are happening.

### Issue 3: No error differentiation

When `waitForPrompt` times out, the UI doesn't distinguish between:
- LLM API error (HTTP 400/500)
- Network timeout (Zhipu unreachable)
- Command still running (just slow)
- Command failed (exit code != 0)

### Issue 4: Model display shows "unconfigured"

Both the glm51 and gpt54-mini runs show `model: unconfigured` in the terminal output, meaning the `--model` flag isn't being resolved to a display name.

## Exact Code to Change

### Fix 1: Increase timeout and add per-phase timeouts

**File**: `demo/demo-app/src/pages/Builder.tsx` — line 163

**Current:**
```typescript
await showCmd(h, cmd, {
  timeout: 120000,
  // ...
});
```

**New:**
```typescript
await showCmd(h, cmd, {
  timeout: 300000,  // 5 minutes — LLM calls can be slow
  // ...
});
```

### Fix 2: Add a "running" indicator to the Gates panel

**File**: `demo/demo-app/src/pages/Builder.tsx`

When a `roko run` command starts, show a pulsing indicator in the Gates panel before any gates are detected. Currently the panel just shows "No gates detected" until the command finishes.

Add a `running` state:
```typescript
const [isRunning, setIsRunning] = useState(false);

// In submitTask, before showCmd:
setIsRunning(true);
// After showCmd:
setIsRunning(false);
```

In the Gates panel JSX, when `isRunning && gates.length === 0`:
```tsx
{isRunning && gates.length === 0 && (
  <div className="gate-running">Running... waiting for LLM response</div>
)}
```

### Fix 3: Detect and display errors from terminal output

**File**: `demo/demo-app/src/lib/terminal-session.ts` — `detectFromOutput()`

Add error detection patterns to the output parser:
```typescript
// Add to detectFromOutput() (around line 328):
const errorPatterns = [
  /HTTP (\d{3}).*?error/i,           // HTTP errors
  /max_tokens.*not supported/i,       // OpenAI parameter mismatch
  /network error|transport error/i,   // Network failures
  /api.*?error|APIError/i,            // API-level errors
  /anyhow::Error|panic!/i,           // Rust panics
];

for (const pat of errorPatterns) {
  const match = combined.match(pat);
  if (match) {
    opts?.onError?.(match[0]);
  }
}
```

Update the `showCmd` options type and Builder.tsx to handle `onError`.

### Fix 4: Show model name in BUILD page header

**File**: `demo/demo-app/src/pages/Builder.tsx`

The command shown to the user should include the resolved model name. If the user selected glm51, show it:
```typescript
// When building the roko run command:
const cmd = `roko run "${prompt}" --model ${selectedModel}`;
```

Verify that the `selectedModel` prop/state is being passed correctly and matches the model key in roko.toml.

### Fix 5: Add abort capability

**File**: `demo/demo-app/src/pages/Builder.tsx`

Add a "Cancel" button that sends Ctrl+C to the PTY:
```typescript
const handleCancel = () => {
  if (termHandle) {
    termHandle.write('\x03');  // Ctrl+C
  }
};
```

Show the Cancel button while `isRunning` is true.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-G-build-page-resilience.md and implement all changes. Increase timeout from 120s to 300s in Builder.tsx, add running indicator to Gates panel, add error detection patterns to terminal-session.ts, add cancel button. Do NOT run cargo build/test. Mark the checklist items as done.
```

## Commit

This batch is committed with Wave 0 (critical pipeline fixes). Do not commit individually.

## Checklist

- [x] Increase `showCmd` timeout from 120000 to 300000 in Builder.tsx
- [x] Add `isRunning` state with pulsing indicator in Gates panel
- [x] Add error detection patterns to `detectFromOutput()` in terminal-session.ts
- [x] Add `onError` callback to `showCmd` options
- [x] Add Cancel button that sends Ctrl+C to PTY
- [ ] Verify: BUILD page shows "Running..." while waiting for LLM
- [ ] Verify: BUILD page surfaces HTTP 400 errors visually
