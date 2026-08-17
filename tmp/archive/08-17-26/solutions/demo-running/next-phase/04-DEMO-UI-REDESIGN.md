# Demo UI Redesign: Auto-Play → Click-to-Run Commands

## Context: What Is the Demo App?

The demo app is a React frontend (Vite + React 19 + Zustand) at `demo/demo-app/` that connects to the `roko serve` backend on :6677. It showcases Roko's capabilities through interactive scenarios.

**Current architecture**:
- 14 scenarios defined in `src/lib/scenario-registry.ts`
- Each scenario is an async `run(ctx)` function in `src/lib/scenario-runners/*.ts`
- Scenarios control 1-8 xterm.js terminal panes via WebSocket PTY connections
- Commands are typed with animation via `showCmd()` in `src/lib/terminal-session.ts`
- A `PlaybackController` manages auto vs step mode (`src/lib/playback-controller.ts`)
- Live PRD/plan state fetched via SSE/WS from `src/lib/workflow-api.ts`
- Main demo page: `src/pages/Demo/index.tsx` with `ScenarioSlot.tsx` per scenario

**Current problems**:
1. Auto-play mode runs all commands sequentially — user watches a terminal fill with raw logs
2. Multi-pane layout (up to 8 terminals) adds visual complexity without clarity
3. Terminal output is noisy (tracing logs mixed with user-facing output)
4. No clear indication of what each command does or what step you're on
5. Speed selector (0.5x-4x) and playback controls are confusing for a demo walkthrough

---

## Design: Click-to-Run Command List

### Layout (2-column)

```
┌─────────────────────────────────┬──────────────────────────┐
│                                 │  COMMANDS                │
│                                 │                          │
│   TERMINAL                      │  ✓ 1. roko init          │
│   (single pane, full output)    │      Create workspace    │
│                                 │                          │
│                                 │  ✓ 2. roko prd idea "…"  │
│                                 │      Capture work item   │
│                                 │                          │
│                                 │  ⠋ 3. roko prd draft new │
│                                 │      Generate PRD (42s)  │
│                                 │                          │
│                                 │  ○ 4. roko prd promote   │
│                                 │                          │
│                                 │  ○ 5. roko prd plan      │
│                                 │                          │
│                                 │  ○ 6. roko plan run      │
│                                 │                          │
│                                 ├──────────────────────────┤
│                                 │  CONTEXT                 │
│                                 │                          │
│                                 │  PRD: BTC Funding Alert  │
│                                 │  ▸ 8 requirements        │
│                                 │  ▸ 3 acceptance criteria │
│                                 │                          │
└─────────────────────────────────┴──────────────────────────┘
```

### Left: Terminal (70% width)
- **Single xterm.js pane** — no multi-pane for PRD pipeline
- Shows the actual command being typed (animation) and its output
- Separator lines between commands (existing `drawSeparator` function)
- Scrollable for reviewing past output
- Uses existing `useTerminal` hook and WebSocket PTY connection

### Right Top: Command List (30% width, upper portion)
Each item shows:
- **Step number** (1, 2, 3...)
- **Command text** (monospace, truncated if long)
- **1-line description** (from existing `cmd-descriptions.ts` mappings)
- **Status icon**: `○` pending, `⠋` running (animated), `✓` success, `✗` failed
- **Elapsed time** (shown dimmed after completion)

**Interaction**: Only the NEXT command is clickable (sequential). Click it → command runs in terminal → status updates → next becomes clickable.

### Right Bottom: Context Panel (30% width, lower portion)
Dynamically shows content based on pipeline stage:
- After `prd idea`: The captured idea text (blockquote)
- After `prd draft new`: PRD title, requirements list, acceptance criteria
- After `prd plan`: Task table from tasks.toml (name, role, dependencies)
- During `plan run`: Gate results live (✓ compile, ✓ test, ✗ clippy, etc.)
- After `plan run`: Summary card (tasks done, cost, time)

Content fetched from backend via existing `workflow-api.ts` → `fetchWorkflowSnapshot()`

---

## Component Specs

### New: `CommandList` Component

**File**: `demo/demo-app/src/components/CommandList.tsx`

```tsx
interface CommandItem {
  id: string;
  command: string;           // the shell command to run
  description: string;       // human-readable 1-liner
  status: 'pending' | 'running' | 'success' | 'failure';
  elapsed?: number;          // ms, shown after completion
  error?: string;            // shown on failure
}

interface CommandListProps {
  commands: CommandItem[];
  onRun: (id: string) => void;
  onRetry: (id: string) => void;
  activeId?: string;
}
```

**Render logic**:
- Map over commands array
- For each: show step number, command (in `<code>`), description, status icon
- The first `pending` command after all `success` commands gets a "Run" button
- Running command shows animated spinner
- Failed command shows "Retry" button
- Completed commands show elapsed time dimmed

### New: `ContextPanel` Component

**File**: `demo/demo-app/src/components/ContextPanel.tsx`

```tsx
interface ContextPanelProps {
  stage: 'init' | 'idea' | 'draft' | 'promote' | 'plan' | 'validate' | 'run' | 'done';
  idea?: string;
  prd?: { title: string; requirements: string[]; acceptance: string[] };
  plan?: { tasks: { name: string; role: string; status: string }[] };
  gates?: { name: string; status: 'pass' | 'fail' | 'pending' }[];
  summary?: { tasksCompleted: string; cost: string; time: string };
}
```

**Render logic**: Switch on `stage`, show appropriate content. Use collapsible sections for long lists.

### Modified: `ScenarioSlot.tsx` (for PRD pipeline)

**File**: `demo/demo-app/src/pages/Demo/ScenarioSlot.tsx`

For the `prd-pipeline` scenario:
- Replace multi-pane `TerminalPaneWithHandle` array with single terminal (left)
- Add `CommandList` + `ContextPanel` to right sidebar
- Keep multi-pane layout for comparison scenarios (race, provider-race)

### Modified: `prd-pipeline.ts` Scenario Runner

**File**: `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts`

Change from monolithic `run(ctx)` to data-driven command list:

```typescript
// CURRENT: One big async function that runs everything
export const prdPipelineScenario: Scenario = {
  run: async (ctx) => {
    await showCmd(ctx.entries[0], 'roko init', ...);
    await showCmd(ctx.entries[0], 'roko prd idea "..."', ...);
    // ... all commands sequentially
  }
};

// NEW: Commands as data, individual run functions
export const prdPipelineScenario: ClickableScenario = {
  id: 'prd-pipeline',
  commands: [
    { id: 'init', command: 'roko init', description: 'Create workspace and config' },
    { id: 'idea', command: 'roko prd idea "Build a CLI..."', description: 'Capture work item' },
    { id: 'draft', command: 'roko prd draft new "BTC Funding Alert CLI"', description: 'Generate PRD via LLM' },
    { id: 'promote', command: 'roko prd draft promote btc-funding-alert-cli', description: 'Promote to published' },
    { id: 'plan', command: 'roko prd plan btc-funding-alert-cli', description: 'Generate implementation plan' },
    { id: 'validate', command: 'roko plan validate .roko/plans', description: 'Lint the plan' },
    { id: 'run', command: 'roko plan run .roko/plans --max-retries 1', description: 'Execute: agents + gates' },
    { id: 'status', command: 'roko status', description: 'View results and costs' },
  ],

  // Called when user clicks a command
  async runCommand(ctx: ScenarioContext, commandId: string): Promise<boolean> {
    const cmd = this.commands.find(c => c.id === commandId);
    if (!cmd) return false;

    const result = await showCmd(ctx.entries[0], cmd.command, {
      timeout: 120000,
      customDesc: cmd.description,
      workspaceDir: ctx.workspaceDir,
    });

    // Update context panel with results
    if (commandId === 'draft' || commandId === 'promote' || commandId === 'plan') {
      const snapshot = await fetchWorkflowSnapshot(ctx.workspaceDir);
      if (snapshot?.prd) ctx.setPipeline({ prd: snapshot.prd });
      if (snapshot?.plans?.[0]) ctx.setPipeline({ plan: snapshot.plans[0] });
    }

    return result.ok;
  }
};
```

### New Type: `ClickableScenario`

```typescript
interface ClickableScenario extends Omit<Scenario, 'run'> {
  commands: Array<{
    id: string;
    command: string;
    description: string;
  }>;
  runCommand(ctx: ScenarioContext, commandId: string): Promise<boolean>;
}
```

---

## What to Remove (for PRD pipeline scenario)

1. **Auto/Stop toggle** — replaced by click-to-run (keep optional "Run All Remaining" button)
2. **Speed selector** (0.5x-4x) — typing speed is fixed
3. **Multi-pane** (for PRD pipeline) — one terminal is clearer
4. **Playback countdown** (3…2…1…) — user clicks when ready
5. **Scenario preview overlay** — command list IS the preview

## What to Keep

1. **xterm.js terminal** — real PTY output is the whole point
2. **Command typing animation** — shows exact command being run
3. **Separator lines** — visual clarity between commands
4. **Workflow API subscriptions** — for live PRD/plan content in context panel
5. **Gate detection from output** — populates context panel gate status
6. **Tab bar** — keep multi-scenario navigation
7. **Multi-pane for comparison scenarios** — race/provider-race need side-by-side
8. **Bottom status bar** — connected indicator, active model, server URL

---

## Command List Per Scenario

### PRD Pipeline (primary demo, click-to-run)

| # | Command | Description |
|---|---------|-------------|
| 1 | `roko init` | Create workspace and config |
| 2 | `roko prd idea "Build a CLI that fetches BTC funding rates..."` | Capture the work item |
| 3 | `roko prd draft new "BTC Funding Alert CLI"` | LLM generates a PRD |
| 4 | `roko prd draft promote btc-funding-alert-cli` | Promote draft to published |
| 5 | `roko prd plan btc-funding-alert-cli` | Generate implementation plan |
| 6 | `roko plan validate .roko/plans` | Lint the generated plan |
| 7 | `roko plan run .roko/plans --max-retries 1` | Execute: agents + gates |
| 8 | `roko status` | View results and costs |

### Cost Race (keep auto-play or 2-column click)

| # | Left (Naive) | Right (Smart) |
|---|---|---|
| 1 | `roko run "..." --model glm-5-1 --no-replan` | `roko run "..."` |
| 2 | — | `roko learn all` |

### Other Scenarios

Other scenarios (gate-retry, explore, knowledge, dream) can keep their current auto-play pattern or be migrated to click-to-run incrementally.

---

## Error Handling in Click-to-Run

When a command fails:
1. Status icon shows `✗` with red highlight
2. Context panel shows error message + suggestion
3. User sees two buttons: **Retry** (re-run same command) or **Skip** (move to next)
4. Terminal keeps the error output visible and scrollable
5. If user retries and succeeds, status updates to ✓ and next command unlocks

---

## File Summary

| File | Action |
|------|--------|
| `src/components/CommandList.tsx` | **New** — clickable command list component |
| `src/components/ContextPanel.tsx` | **New** — stage-aware context display |
| `src/pages/Demo/ScenarioSlot.tsx` | **Modify** — use CommandList for prd-pipeline |
| `src/lib/scenario-runners/prd-pipeline.ts` | **Modify** — data-driven commands + runCommand() |
| `src/lib/scenarios.ts` | **Modify** — add ClickableScenario type |
| `src/lib/cmd-descriptions.ts` | **Use as-is** — already maps commands to descriptions |
| `src/lib/terminal-session.ts` | **Use as-is** — showCmd still handles execution |
| `src/lib/workflow-api.ts` | **Use as-is** — fetches live PRD/plan state |
| `src/hooks/useTerminal.ts` | **Use as-is** — terminal management unchanged |
