# Agent Roles

## Role Definitions (Adapted from Bardo)

Each role defines: identity, allowed tools, permission mode, model preference, context budget, and output format.

### Conductor (Meta-Orchestrator)
- **Identity**: Reasons about workflow state, decides next actions, never modifies repo
- **Tools**: Read, Glob, Grep, WebFetch, WebSearch
- **Permission mode**: `plan` (read-only)
- **Model**: opus (needs strong reasoning)
- **Context**: Full plan state, all task statuses, review feedback history
- **Output**: Structured decisions (which task next, what pipeline to use, when to escalate)
- **Use in ACP**: Not user-facing. Internal to runner.

### Strategist
- **Identity**: Analyzes before implementation. Synthesizes patterns, produces brief.
- **Tools**: Read, Glob, Grep, WebFetch, WebSearch
- **Permission mode**: `plan` (read-only, no code changes)
- **Context**: Plan, workspace map, PRD, relevant code sections
- **Output**: `brief.md` — analysis, approach, risks, file targets
- **Use in ACP**: "Plan" mode. User sees strategist thinking before implementation.

### Implementer
- **Identity**: Writes and edits code. Gets the heaviest context budget.
- **Tools**: Read, Glob, Grep, Edit, Write, Bash, WebFetch (at high effort), WebSearch (at high effort)
- **Permission mode**: `bypassPermissions` (full access)
- **Context budget**: Plan 50K, workspace map 20K, PRD 12K, brief 8K, skills 8K, file context 8K
- **Output**: Code changes, committed to worktree
- **Use in ACP**: "Code" mode default. Standard single-turn chat.

### AutoFixer
- **Identity**: Lightweight fix agent for simple gate failures (compile errors, lint issues)
- **Tools**: Read, Glob, Grep, Edit, Write, Bash
- **Permission mode**: `bypassPermissions`
- **Context budget**: Smaller than Implementer (just error output + relevant files)
- **Model**: sonnet or haiku (fast, cheap)
- **Output**: Minimal targeted fixes
- **Use in ACP**: Automatic on gate failure in express mode. User sees "fixing..." status.

### Researcher
- **Identity**: Gathers information from web, codebase, docs. Never modifies code.
- **Tools**: Read, Glob, Grep, Bash, WebFetch, WebSearch
- **Permission mode**: `plan` (read-only)
- **Model**: sonnet or perplexity (for web research)
- **Context**: Research query, existing knowledge, related files
- **Output**: Structured research report with citations
- **Use in ACP**: "Research" mode. `/research` slash command.

### QuickReviewer
- **Identity**: Single-pass combined review (architecture + correctness + security)
- **Tools**: Read, Glob, Grep, Bash, WebFetch, WebSearch
- **Permission mode**: `plan` (read-only)
- **Model**: opus or sonnet (needs judgment)
- **Output format**: Structured JSON verdict:
  ```json
  {
    "verdict": "approve" | "revise" | "reject",
    "findings": [{ "severity": "blocking|major|minor", "file": "...", "line": N, "issue": "...", "suggestion": "..." }],
    "summary": "..."
  }
  ```
- **Use in ACP**: Session update showing review findings. User sees pass/fail.

### Architect (Full Review)
- **Identity**: Architecture review. Checks design patterns, coupling, API contracts.
- **Tools**: Read, Glob, Grep, Bash, WebFetch, WebSearch
- **Permission mode**: `plan` (read-only)
- **Output**: Structured JSON (same as QuickReviewer but focused on architecture)
- **Use in ACP**: Runs in parallel with Auditor + Scribe. Results shown as plan update.

### Auditor (Full Review)
- **Identity**: Security + correctness audit. Checks for vulnerabilities, logic errors, edge cases.
- **Tools**: Read, Glob, Grep, Bash, WebFetch, WebSearch
- **Permission mode**: `plan` (read-only)
- **Output**: Structured JSON verdict focused on correctness/security
- **Use in ACP**: Results shown alongside Architect findings.

### Scribe (Documentation)
- **Identity**: Writes documentation. Module docs, API docs, architecture diagrams.
- **Tools**: Read, Glob, Grep, Write, Edit, WebFetch, WebSearch
- **Permission mode**: limited write (docs only)
- **Context budget**: Heavier on PRD context (16K)
- **Output**: Documentation files, diagrams, READMEs
- **Use in ACP**: DocRevision phase. User sees doc generation progress.

### Critic (Reviews Scribe)
- **Identity**: Reviews documentation for accuracy, completeness, clarity.
- **Tools**: Read, Glob, Grep, Bash, WebFetch, WebSearch
- **Permission mode**: `plan` (read-only)
- **Output**: Structured verdict on documentation quality
- **Use in ACP**: Internal to full pipeline, not directly user-facing.

## Tool Restrictions by Role (Principle of Least Privilege)

```
Role            │ Read │ Glob │ Grep │ Edit │ Write│ Bash │ Web  │ Schema
────────────────┼──────┼──────┼──────┼──────┼──────┼──────┼──────┼────────
Conductor       │  ✓   │  ✓   │  ✓   │  ✗   │  ✗   │  ✗   │  ✓   │  ✗
Strategist      │  ✓   │  ✓   │  ✓   │  ✗   │  ✗   │  ✗   │  ✓   │  ✗
Implementer     │  ✓   │  ✓   │  ✓   │  ✓   │  ✓   │  ✓   │  ✓*  │  ✗
AutoFixer       │  ✓   │  ✓   │  ✓   │  ✓   │  ✓   │  ✓   │  ✗   │  ✗
Researcher      │  ✓   │  ✓   │  ✓   │  ✗   │  ✗   │  ✓   │  ✓   │  ✗
QuickReviewer   │  ✓   │  ✓   │  ✓   │  ✗   │  ✗   │  ✓   │  ✓   │  ✓
Architect       │  ✓   │  ✓   │  ✓   │  ✗   │  ✗   │  ✓   │  ✓   │  ✓
Auditor         │  ✓   │  ✓   │  ✓   │  ✗   │  ✗   │  ✓   │  ✓   │  ✓
Scribe          │  ✓   │  ✓   │  ✓   │  ✓   │  ✓   │  ✗   │  ✓   │  ✗
Critic          │  ✓   │  ✓   │  ✓   │  ✗   │  ✗   │  ✓   │  ✓   │  ✓

* Implementer gets Web at high/max effort only
Schema = JSON schema for structured output (review verdicts)
```

## Model Selection by Role

Default model assignments (overridable via config):

| Role | Default | Rationale |
|------|---------|-----------|
| Conductor | opus | Needs strong reasoning for meta-decisions |
| Strategist | opus | Deep analysis, strategic thinking |
| Implementer | sonnet | Good balance of capability and speed |
| AutoFixer | haiku/sonnet | Simple fixes, speed matters |
| Researcher | sonnet/perplexity | Web research, synthesis |
| QuickReviewer | sonnet | Judgment without overthinking |
| Architect | opus | Architecture decisions need depth |
| Auditor | opus | Security needs careful analysis |
| Scribe | sonnet | Documentation writing |
| Critic | sonnet | Doc review is lighter than code review |

The CascadeRouter adjusts these based on observed success rates per (role, task-type, complexity).

## System Prompt Templates by Role

Each role gets a system prompt that:
1. Establishes identity and constraints
2. Specifies output format expectations
3. Lists available tools explicitly
4. Includes role-specific guidelines

**Example — Auditor:**
```
You are a security and correctness auditor. Your role is to review code changes for vulnerabilities, logic errors, and edge cases.

You CANNOT modify files. You can only read, search, and analyze.

Output your findings as structured JSON matching this schema:
{
  "verdict": "approve" | "revise" | "reject",
  "findings": [...]
}

Guidelines:
- Check for OWASP top 10 vulnerabilities
- Verify error handling covers all paths
- Check for race conditions in concurrent code
- Verify input validation at system boundaries
- Flag any use of unsafe, unwrap in production paths
```

## Context Budget by Role (From Bardo)

Context is assembled from priority-ranked sections, aligned to prompt caching layers:

| Role | Plan | Workspace Map | PRD | Brief | Skills | File Context |
|------|------|---------------|-----|-------|--------|--------------|
| Implementer | 50K | 20K | 12K | 8K | 8K | 8K |
| Strategist | 50K | 20K | 12K | — | — | — |
| Scribe | 50K | — | 16K | 8K | — | — |
| Reviewers | 50K | 6K | 6K | — | — | — |
| Researcher | — | 6K | 12K | — | — | — |
| AutoFixer | 20K | — | — | — | — | error output |
