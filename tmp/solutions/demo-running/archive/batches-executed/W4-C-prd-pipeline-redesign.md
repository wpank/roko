# W4-C: Refactor PRD Pipeline Scenario to Click-to-Run

**Priority**: P1 — demo UI redesign
**Effort**: 3-4 hours
**Files to modify**: 3 files
**Dependencies**: W4-A (ClickableScenario type), W4-B (ContextPanel component)

## What to Change

### 1. Rewrite `prd-pipeline.ts` as ClickableScenario

**File**: `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts`

The current implementation (lines 109-371) is a monolithic `run(ctx)` function that calls `showCmd()` sequentially. Convert to data-driven commands:

```typescript
import type { ClickableScenario, CommandDef, ScenarioContext } from '../scenarios';
import { showCmd, roko } from '../terminal-session';
import { fetchWorkflowSnapshot } from '../workflow-api';

const PRD_IDEA = 'Build a CLI that fetches BTC funding rates from Binance, calculates average funding over 7 days, and alerts when funding exceeds 0.1%';

function prdCommands(ctx: ScenarioContext): CommandDef[] {
  return [
    { id: 'init', command: roko(ctx, 'init'), description: 'Create workspace and config', timeout: 10000 },
    { id: 'idea', command: roko(ctx, `prd idea "${PRD_IDEA}"`), description: 'Capture work item', timeout: 10000 },
    { id: 'draft', command: roko(ctx, 'prd draft new "BTC Funding Alert CLI"'), description: 'Generate PRD via LLM', timeout: 180000 },
    { id: 'promote', command: roko(ctx, 'prd draft promote btc-funding-alert-cli'), description: 'Promote to published', timeout: 10000 },
    { id: 'plan', command: roko(ctx, 'prd plan btc-funding-alert-cli'), description: 'Generate implementation plan', timeout: 300000 },
    { id: 'validate', command: roko(ctx, 'plan validate .roko/plans'), description: 'Lint the generated plan', timeout: 10000 },
    { id: 'run', command: roko(ctx, 'plan run .roko/plans --max-retries 1'), description: 'Execute: agents + gates', timeout: 600000 },
    { id: 'status', command: roko(ctx, 'status'), description: 'View results and costs', timeout: 10000 },
  ];
}

export const prdPipelineScenario: ClickableScenario = {
  id: 'prd-pipeline',
  title: 'PRD Pipeline',
  subtitle: 'Click each command to walk through the full development pipeline',
  panes: 1,
  labels: ['Terminal'],
  panel: true,
  promptBar: false,
  steps: [ /* keep existing steps for timeline */ ],
  category: 'pipeline',
  features: ['PRD generation', 'Task planning', 'Gate validation'],
  durationHint: '2-5 min',
  accent: 'rose',
  icon: 'pipeline',
  commands: [], // populated dynamically in runCommand

  async runCommand(ctx: ScenarioContext, commandId: string): Promise<boolean> {
    const commands = prdCommands(ctx);
    const cmd = commands.find(c => c.id === commandId);
    if (!cmd) return false;

    const main = ctx.entries[0];
    const result = await showCmd(main, cmd.command, {
      timeout: cmd.timeout ?? 60000,
      customDesc: cmd.description,
      workspaceDir: ctx.workspaceDir,
    });

    // Update context panel with live data
    if (['draft', 'promote', 'plan', 'run'].includes(commandId)) {
      try {
        const snapshot = await fetchWorkflowSnapshot(ctx.workspaceDir);
        // Pass snapshot data to context panel via scenario context
        if (snapshot?.prd) {
          ctx.setMetric?.('prd-title', snapshot.prd.title ?? '');
        }
      } catch { /* non-fatal */ }
    }

    return result.ok;
  }
};
```

### 2. Update `ScenarioSlot.tsx` for ClickableScenario

**File**: `demo/demo-app/src/pages/Demo/ScenarioSlot.tsx`

When the active scenario is a `ClickableScenario`, render differently:
- Left 70%: single terminal pane (existing)
- Right 30% top: `CommandList` component (from W4-A)
- Right 30% bottom: `ContextPanel` component (from W4-B)
- Remove auto-play controls for this scenario type

```tsx
import { isClickableScenario } from '../../lib/scenarios';
import { CommandList } from '../../components/CommandList';
import { ContextPanel } from '../../components/ContextPanel';
import { useCommandList } from '../../hooks/useCommandList';

// Inside ScenarioSlot component:
if (isClickableScenario(scenario)) {
  const commands = prdCommands(ctx);
  const { items, markRunning, markSuccess, markFailure, nextPendingId, isRunning } = useCommandList(commands);

  const handleRun = async (id: string) => {
    markRunning(id);
    const ok = await scenario.runCommand(ctx, id);
    if (ok) markSuccess(id);
    else markFailure(id);
  };

  return (
    <div className="flex h-full">
      <div className="w-[70%]">
        {/* Single terminal pane */}
      </div>
      <div className="w-[30%] flex flex-col border-l border-white/10">
        <div className="flex-1 overflow-auto p-3">
          <CommandList commands={items} onRun={handleRun} onRetry={handleRun} />
        </div>
        <div className="border-t border-white/10 p-3">
          <ContextPanel stage={currentStage} {...contextData} />
        </div>
      </div>
    </div>
  );
}
```

### 3. What to Remove (for PRD pipeline only)

- Auto/Stop toggle button
- Speed selector (0.5x-4x)
- Playback countdown (3…2…1…)
- Multi-pane grid (keep single terminal)

**Keep for other scenarios**: multi-pane, auto-play, speed selector still work for race, provider-race, etc.

### 4. Update scenario registry

**File**: `demo/demo-app/src/lib/scenario-runners/index.ts`

The `prd-pipeline` export should use the new `ClickableScenario`. Other scenarios remain unchanged.

## Existing Integration Points

- **`roko()` function** (terminal-session.ts:114-125): Auto-injects `--repo <workspace>` and `--model`. Already wired correctly.
- **`showCmd()`** (terminal-session.ts:220-313): Handles typing animation, exit code capture, gate detection. No changes needed.
- **`fetchWorkflowSnapshot()`** (workflow-api.ts:105-123): Fetches live PRD/plan state. Already exists.
- **`openWorkflowSubscriptions()`** (workflow-api.ts:125-247): SSE + WS for live updates. Already exists.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W4-C-prd-pipeline-redesign.md and implement all changes described in it. This depends on W4-A and W4-B being done first. Rewrite prd-pipeline.ts as ClickableScenario, update ScenarioSlot.tsx for 2-column layout. Do NOT run npm build — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 4 batches together. Do not commit individually.

## Checklist

- [x] Rewrite `prd-pipeline.ts` as `ClickableScenario` with `commands` array + `runCommand()`
- [x] Update `ScenarioSlot.tsx` to detect `ClickableScenario` and render 2-column layout
- [x] Wire `CommandList` into right sidebar (top)
- [x] Wire `ContextPanel` into right sidebar (bottom)
- [x] Hide auto-play controls for ClickableScenario
- [x] Keep auto-play working for non-clickable scenarios (race, etc.)
- [x] Click command → runs in terminal → status updates → next unlocks
- [x] Failed command shows retry button
- [ ] TypeScript compiles
- [ ] Build succeeds
- [ ] Visual test: layout looks good at 1920x1080
