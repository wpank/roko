# W4-B: Add ContextPanel Component

**Priority**: P1 — demo UI redesign
**Effort**: 1-2 hours
**Files to modify**: 1 new file
**Dependencies**: W4-A (uses same interfaces)

## What to Build

**File**: `demo/demo-app/src/components/ContextPanel.tsx`

A panel that shows stage-appropriate content as the PRD pipeline progresses.

```typescript
interface ContextPanelProps {
  stage: 'init' | 'idea' | 'draft' | 'promote' | 'plan' | 'validate' | 'run' | 'done';
  idea?: string;
  prd?: { title: string; requirements: string[]; acceptance: string[] };
  plan?: { tasks: { name: string; role: string; status: string }[] };
  gates?: { name: string; status: 'pass' | 'fail' | 'pending' }[];
  summary?: { tasksCompleted: string; cost: string; time: string };
}
```

### Render Logic (switch on stage)

- **`init`**: "Workspace created. Ready to capture an idea."
- **`idea`**: Blockquote of the captured idea text
- **`draft`**: PRD title + requirement count + acceptance criteria count (collapsible lists)
- **`promote`**: "PRD published. Ready for plan generation."
- **`plan`**: Task table from tasks.toml (id, title, role)
- **`validate`**: "Plan valid. Ready for execution." or validation errors
- **`run`**: Live gate results (list of gate name + pass/fail badges)
- **`done`**: Summary card (tasks completed, total cost, total time)

### Data Sources

- **idea text**: Captured from command output or passed through by scenario runner
- **PRD data**: From `workflow-api.ts` → `fetchWorkflowSnapshot()` → `workflowSnapshotToPrd()`
- **Plan data**: From `workflow-api.ts` → `fetchWorkflowSnapshot()` → `workflowSnapshotToPlans()`
- **Gate data**: From terminal output detection (`detectFromOutput()` in terminal-session.ts)
- **Summary**: Computed from final status + cost extraction

### Styling

Use the same design tokens as `PrdPipelinePanel.tsx` (existing component) for consistency. Check `demo/demo-app/src/components/PrdPipelinePanel.tsx` for the card/badge patterns.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W4-B-context-panel.md and implement all changes described in it. Create ContextPanel.tsx in demo/demo-app/src/components/ with stage-based rendering. Reference PrdPipelinePanel.tsx for design tokens. Do NOT run npm build — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 4 batches together. Do not commit individually.

## Checklist

- [x] Create `ContextPanel.tsx` with stage-based rendering
- [x] Each stage shows appropriate content (idea text, PRD summary, task table, gate results, summary)
- [x] Use collapsible sections for long lists
- [x] Match existing design system (badges, cards, colors)
- [ ] TypeScript compiles
- [ ] Build succeeds
