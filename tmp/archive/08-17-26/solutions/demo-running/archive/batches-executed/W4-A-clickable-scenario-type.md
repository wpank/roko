# W4-A: Add ClickableScenario Type + CommandList Component

**Priority**: P1 — demo UI redesign
**Effort**: 2-3 hours
**Files to modify**: 3 new files + 1 modified
**Dependencies**: None

## Problem

The demo auto-play mode runs all commands sequentially — the user watches a terminal fill with raw logs. No clear indication of what each command does or what step you're on.

## What to Build

### 1. New type: `ClickableScenario`

**File**: `demo/demo-app/src/lib/scenarios.ts`

Add alongside the existing `Scenario` interface:

```typescript
export interface CommandDef {
  id: string;
  command: string;        // the shell command (may use roko() helper)
  description: string;    // human-readable 1-liner
  timeout?: number;       // ms, default 60000
}

export interface ClickableScenario extends Omit<Scenario, 'run'> {
  commands: CommandDef[];
  runCommand(ctx: ScenarioContext, commandId: string): Promise<boolean>;
}

export function isClickableScenario(s: Scenario | ClickableScenario): s is ClickableScenario {
  return 'commands' in s && 'runCommand' in s;
}
```

### 2. New component: `CommandList.tsx`

**File**: `demo/demo-app/src/components/CommandList.tsx`

```typescript
interface CommandItem {
  id: string;
  command: string;
  description: string;
  status: 'pending' | 'running' | 'success' | 'failure';
  elapsed?: number;     // ms, shown after completion
  error?: string;       // shown on failure
}

interface CommandListProps {
  commands: CommandItem[];
  onRun: (id: string) => void;
  onRetry: (id: string) => void;
}
```

**Render logic**:
- Map over commands array
- Each item shows: step number, command in `<code>`, description, status icon
- Status icons: `○` pending, animated spinner running, `✓` success, `✗` failed
- The first `pending` command after all consecutive `success` commands gets a "Run" button
- Running command shows animated spinner with elapsed time
- Failed command shows "Retry" button
- Completed commands show elapsed time dimmed
- Use existing project styling (Tailwind classes from other components)

**Styling reference**: Look at `demo/demo-app/src/components/Timeline.tsx` and `demo/demo-app/src/components/design/StepProgress.tsx` for existing step/progress patterns.

### 3. State management hook: `useCommandList`

**File**: `demo/demo-app/src/hooks/useCommandList.ts`

```typescript
import { useState, useCallback, useRef } from 'react';
import type { CommandDef } from '../lib/scenarios';

interface CommandState {
  id: string;
  command: string;
  description: string;
  status: 'pending' | 'running' | 'success' | 'failure';
  elapsed?: number;
  error?: string;
}

export function useCommandList(commands: CommandDef[]) {
  const [items, setItems] = useState<CommandState[]>(
    commands.map(c => ({ ...c, status: 'pending' as const }))
  );
  const startTime = useRef<number>(0);

  const markRunning = useCallback((id: string) => {
    startTime.current = Date.now();
    setItems(prev => prev.map(item =>
      item.id === id ? { ...item, status: 'running' } : item
    ));
  }, []);

  const markSuccess = useCallback((id: string) => {
    const elapsed = Date.now() - startTime.current;
    setItems(prev => prev.map(item =>
      item.id === id ? { ...item, status: 'success', elapsed } : item
    ));
  }, []);

  const markFailure = useCallback((id: string, error?: string) => {
    const elapsed = Date.now() - startTime.current;
    setItems(prev => prev.map(item =>
      item.id === id ? { ...item, status: 'failure', elapsed, error } : item
    ));
  }, []);

  const reset = useCallback(() => {
    setItems(commands.map(c => ({ ...c, status: 'pending' as const })));
  }, [commands]);

  const nextPendingId = items.find(i => i.status === 'pending')?.id;
  const isRunning = items.some(i => i.status === 'running');

  return { items, markRunning, markSuccess, markFailure, reset, nextPendingId, isRunning };
}
```

## Existing Code Context

- **`src/lib/scenarios.ts`**: Defines `Scenario` interface (lines 68-92), `ScenarioContext` (lines 34-66). Export is `export { allScenarios as SCENARIOS } from './scenario-runners';` (line 96).
- **`src/lib/cmd-descriptions.ts`**: Has `lookupCmdDesc(cmd)` that maps command prefixes to descriptions (66 entries). Use this for fallback descriptions.
- **`src/lib/terminal-session.ts`**: `showCmd()` function (lines 220-313) handles command execution with typing animation. Returns `CommandResult { ok, elapsed, gates, cost, tokens }`.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W4-A-clickable-scenario-type.md and implement all changes described in it. Add CommandDef + ClickableScenario types to scenarios.ts, create CommandList.tsx component, create useCommandList.ts hook. Do NOT run npm build — compilation is deferred. Just make the code changes and mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 4 batches together (W4-A + W4-B + W4-C). Do not commit individually.

## Checklist

- [x] Add `CommandDef` and `ClickableScenario` interfaces to `scenarios.ts`
- [x] Add `isClickableScenario()` type guard
- [x] Create `CommandList.tsx` component with status icons, run/retry buttons
- [x] Create `useCommandList.ts` hook for state management
- [ ] Component renders correctly in isolation (create a simple test page if needed)
- [ ] TypeScript compiles without errors
- [ ] Build succeeds
