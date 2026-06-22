# S-term-2: Frontend useTerminalEvents hook + CommandEvent types

## Task
Add TypeScript `CommandEvent` types matching the Rust serde shape, plus a `useTerminalEvents` React hook that subscribes to `/api/terminal/sessions/{id}/events`.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-term-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/26-terminal-demo-truth.md` § Phase 4.

## Exact changes

### 1. `demo/demo-app/src/lib/terminal-events.ts`

Already created (or to be created) by T5-41. If T5-41 didn't include all types, add the hook helpers here.

### 2. `demo/demo-app/src/hooks/useTerminal.ts` (extend or sibling)

```typescript
import { useEffect, useState } from 'react';
import type { CommandEvent } from '../lib/terminal-events';
import { apiBase } from '../lib/serve-url';

export interface UseTerminalEventsState {
  events: CommandEvent[];
  exitCode: number | null;
  running: boolean;
  buffer: string;
}

export function useTerminalEvents(sessionId: string | null): UseTerminalEventsState {
  const [events, setEvents] = useState<CommandEvent[]>([]);
  const [exitCode, setExitCode] = useState<number | null>(null);
  const [running, setRunning] = useState(false);
  const [buffer, setBuffer] = useState('');

  useEffect(() => {
    if (!sessionId) return;
    const url = `${apiBase().replace(/^http/, 'ws')}/api/terminal/sessions/${sessionId}/events`;
    const ws = new WebSocket(url);

    ws.onmessage = (msg) => {
      const event: CommandEvent = JSON.parse(msg.data);
      setEvents(prev => [...prev, event]);
      switch (event.type) {
        case 'started':
          setRunning(true);
          setExitCode(null);
          setBuffer('');
          break;
        case 'output':
          setBuffer(prev => prev + event.bytes);
          break;
        case 'exited':
          setRunning(false);
          setExitCode(event.code);
          break;
        case 'spawn_failed':
        case 'cancelled':
          setRunning(false);
          break;
      }
    };

    return () => ws.close();
  }, [sessionId]);

  return { events, exitCode, running, buffer };
}
```

### 3. Update consumer components

If `useTerminal.ts` previously polled or used regex matching, refactor consumer components (`TerminalPane`, etc.) to use `useTerminalEvents`. Keep the existing `useTerminal` hook for raw IO if needed; add `useTerminalEvents` alongside.

## Write Scope
- `demo/demo-app/src/hooks/useTerminal.ts` (extend)
- `demo/demo-app/src/lib/terminal-events.ts` (extend, if T5-41 didn't include hook helpers)

## Read-Only Context
- `demo/demo-app/src/lib/serve-url.ts`
- `crates/roko-serve/src/command_events.rs`

## Verify

```bash
rg 'useTerminalEvents|CommandEvent' demo/demo-app/src/hooks/ demo/demo-app/src/lib/
# Expect: at least 3 hits

cd demo/demo-app
yarn typecheck
yarn lint
```

## Do NOT

- Do NOT use `npm`. Use `yarn`.
- Do NOT poll `/api/terminal/sessions/{id}` for status.
- Do NOT introduce a state-management library (Redux, Zustand) just for this hook.
- Do NOT bundle with other S-term batches.
