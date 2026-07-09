# Multi-Agent Workflow Patterns

## The Core Insight From Bardo

Bardo proved that the **pipeline pattern** — a state machine that chains agent roles through phases — is the right abstraction for multi-agent work. Not free-form agent chat, not rigid sequential scripts, but a configurable state machine where:

1. Each phase has a **role** (strategist, implementer, reviewer)
2. Roles have **capability restrictions** (reviewer can't edit, implementer can't merge)
3. Phases are connected by **gates** (compile, test, clippy must pass before review)
4. The pipeline **adapts** based on complexity (skip review for trivial, add critic for complex)
5. **Failures loop back** with specific feedback (reviewer rejects → implementer retries with feedback)

## Workflow Templates (From Bardo's Proven Patterns)

### 1. Express Pipeline
**When**: Quick fixes, simple tasks, low-risk changes.
```
User Prompt → Implementer → CompileGate → TestGate → Commit
```
- No strategist, no review
- On gate failure: spawn AutoFixer (lightweight) instead of full re-implementation
- Max 1 iteration
- Cheapest model selection

### 2. Standard Pipeline
**When**: Normal feature work, bug fixes, refactors.
```
User Prompt → Implementer → CompileGate → TestGate → QuickReviewer → Commit
```
- Single-pass review (combined arch + audit)
- On review rejection: implementer retries with feedback
- Max 2 iterations

### 3. Full Pipeline (Bardo's Complex Mode)
**When**: Multi-file features, architectural changes, cross-crate work.
```
User Prompt → Strategist → Implementer → CompileGate → TestGate
  → [Parallel: Architect + Auditor + Scribe] → Verdict → Commit
```
- Strategist produces brief before implementation
- Three parallel reviewers: architecture, security/correctness, documentation
- Critic reviews scribe output
- Review feedback loops back to implementer
- Max 2 iterations

### 4. Research Pipeline
**When**: Exploring options, competitive analysis, technical investigation.
```
User Prompt → Researcher → Synthesizer → [Optional: Writer]
```
- Researcher gathers information (web search, codebase search)
- Synthesizer combines findings into structured output
- Optional Writer produces document (PRD, RFC, analysis)
- No gates (no code changes)

### 5. PRD-to-Ship Pipeline
**When**: End-to-end feature delivery from spec to merged code.
```
PRD → Plan Generator → [For each task: Standard/Full Pipeline] → Merge Queue
```
- Reads PRD, generates tasks.toml
- Executes tasks respecting dependency DAG
- Each task runs through appropriate pipeline based on complexity
- Merge queue orders commits by dependency

### 6. Review-Only Pipeline
**When**: Code review requests, audit requests.
```
Git Diff → [Parallel: Architect + Auditor] → Verdict Report
```
- Read-only pipeline, no code modifications
- Outputs structured review with findings
- Can be triggered by PR event

### 7. Documentation Pipeline
**When**: Generating/updating docs after implementation.
```
Changed Files → Scribe → Critic → [Fix Loop] → Commit Docs
```
- Scribe generates documentation from code changes
- Critic reviews for accuracy, completeness, clarity
- Fix loop until critic approves

### 8. Custom Pipeline (User-Defined)
Users compose their own from available roles:
```toml
[workflow]
name = "my-pipeline"
steps = [
    { role = "researcher", phase = "research" },
    { role = "strategist", phase = "plan" },
    { role = "implementer", phase = "implement", gate = ["compile", "test"] },
    { role = "auditor", phase = "review" },
]
```

## Complexity-Based Pipeline Selection (From Bardo)

Bardo automatically selected the pipeline based on task metadata:

| Signal | Trivial | Simple | Standard | Complex |
|--------|---------|--------|----------|---------|
| Files touched | 1 | 1-3 | 3-10 | 10+ |
| Estimated time | <5min | 5-15min | 15-45min | 45min+ |
| Crates touched | 1 | 1 | 1-3 | 3+ |
| Cross-plan deps | 0 | 0 | 0-2 | 3+ |
| Touches core | No | No | Maybe | Yes |

| Complexity | Strategist | Reviews | Review Type | Critic | Max Iter |
|------------|------------|---------|-------------|--------|----------|
| Trivial | No | No | — | No | 1 |
| Simple | No | No | — | No | 2 |
| Standard | No | Yes | QuickReviewer | No | 2 |
| Complex | Yes | Yes | Architect+Auditor+Scribe | Yes | 2 |

**For ACP**: Surface this as a "Quality" config option:
- **Fast** → Express pipeline
- **Balanced** → Standard pipeline (auto-select per task)
- **Thorough** → Full pipeline
- **Custom** → User-defined workflow

## Patterns From Workflow-v1 Vision

### Trigger-Driven Workflows
Same workflow, different triggers:
- Manual: user types `/plan-run`
- File watch: `.roko/prd/*.md` changes → auto-plan
- Webhook: GitHub PR opened → auto-review
- Cron: nightly → run test suite + knowledge GC
- Chain: workflow A completes → workflow B starts

### Fan-Out / Fan-In
Parallel execution with synchronization:
```
                ┌─ Architect ──┐
Task Complete ──┤─ Auditor ────├── Verdict
                └─ Scribe ─────┘
```

### Conditional Branching
Route based on output:
```
Gate Result ──┬─[pass]── Review
              ├─[simple fail]── AutoFix → Gate
              └─[complex fail]── Implementer → Gate
```

### Retry with Escalation
Progressive model upgrade on failure:
```
Attempt 1 (haiku) → fail → Attempt 2 (sonnet) → fail → Attempt 3 (opus)
```

### Human-in-the-Loop
Pause for user decision:
```
Reviewer finds ambiguity → Pause → User resolves → Resume
```
Via ACP: send a session update with choices, wait for user prompt response.

## Failure Handling Matrix

| Failure | Express | Standard | Full |
|---------|---------|----------|------|
| Compile fails | AutoFix ×2 → fail | Implementer retry ×2 → fail | Implementer retry ×2 → fail |
| Test fails | AutoFix ×2 → fail | Implementer retry with test output | Implementer retry with test output |
| Review rejects (quick) | N/A | QuickFix → re-gate | QuickFix → re-gate |
| Review rejects (complex) | N/A | Implementer retry with feedback | Implementer retry with feedback |
| Review rejects (docs only) | N/A | DocRevision → commit | Scribe revision → Critic re-review |
| Budget exceeded | Cancel | Downgrade models → retry | Downgrade models → retry |
| Timeout (45min) | Cancel | Cancel + persist state | Cancel + persist state |
