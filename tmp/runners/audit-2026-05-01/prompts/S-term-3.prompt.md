# S-term-3: Migrate knowledge-transfer scenario to typed CommandEvent

## Task
Migrate `demo/demo-app/src/lib/scenario-runners/knowledge-transfer.ts` off prompt scraping. Subscribe to `/events`. Same migration pattern as T5-41.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-term-1, S-term-2, T5-41. Wave 3.

## Source plan
`tmp/subsystem-audits/implementation-plans/26-terminal-demo-truth.md` § Phase 3.

## Exact changes

Same as T5-41 for the prd-pipeline scenario, applied to `knowledge-transfer.ts`. Use the `runCommand` / `useTerminalEvents` infrastructure already present.

### 1. Audit current regex matches

```bash
rg '\.match\(/|RegExp' demo/demo-app/src/lib/scenario-runners/knowledge-transfer.ts -n
```

### 2. Replace with `runCommand` calls

Each scenario step:

```typescript
import { runCommand } from '../terminal-events';

async function step(sessionId: string, command: string) {
  const result = await runCommand(sessionId, command, apiBase());
  if (result.failed) {
    throw new Error(`step failed: ${result.reason ?? `exit ${result.code}`}`);
  }
  return result;
}
```

### 3. Status updates from `Exited.code`

```typescript
const result = await step(sessionId, 'roko knowledge ingest ...');
setStatus(result.code === 0 ? 'ingested' : `failed (exit ${result.code})`);
```

### 4. Verify

```bash
rg '\.match\(/' demo/demo-app/src/lib/scenario-runners/knowledge-transfer.ts
# Expect: 0 hits

cd demo/demo-app
yarn typecheck
yarn lint
```

## Write Scope
- `demo/demo-app/src/lib/scenario-runners/knowledge-transfer.ts`

## Do NOT

- Do NOT use `npm`. Use `yarn`.
- Do NOT bundle with other scenarios.
- Do NOT introduce new prompt regexes.
- Do NOT skip the typecheck.
