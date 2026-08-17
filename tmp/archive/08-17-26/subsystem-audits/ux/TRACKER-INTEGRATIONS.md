# Tracker & Integration Analysis

## The Core Insight

The most impactful integrations are not "more trackers to poll." They are
**integrations that make Roko's aggregate-funnel-execute workflow faster and
more reliable.** The user's workflow today is: gather docs, funnel into plans,
execute via agents. Every integration should be evaluated by: does it make
one of these phases better?

---

## Integration Evaluation Framework

For each integration, answer three questions:

1. **Which workflow phase does it improve?** (Aggregate, Funnel, Execute, Review)
2. **Does it reduce manual work?** (If the user still has to paste context
   into Claude, the integration is not helping.)
3. **Does it improve execution quality?** (Better context = better agent
   output = fewer gate failures = less cost.)

---

## Tier 1: Workflow-Aligned Integrations

These directly improve the aggregate-funnel-execute workflow.

### 1. GitHub Integration (Issues + PRs + Actions)

**Phase improved:** All four.

**Aggregate:** GitHub issues and PRs are a source of requirements. `roko
ingest --from-github <issue-url>` should pull the issue description,
comments, referenced code, and linked PRs into a corpus.

**Funnel:** GitHub issue labels, milestones, and project boards can inform
the funnel's task decomposition. If the issue has sub-issues, those map
directly to tasks.

**Execute:** Roko should create PRs, update issue status, and respond to CI
results. The agent creates a PR; GitHub Actions runs CI; Roko reads CI
results as additional gate information.

**Review:** Post-execution, Roko should update the GitHub issue with a
summary: "Implemented via PR #42, all gates passed."

**Implementation:**
- `roko ingest --from-github owner/repo#123` -> reads issue + comments +
  referenced code into corpus
- `roko plan run --auto-pr` -> creates PR on task completion
- `roko plan run --ci-gate` -> waits for CI pass before marking done
- File: new `crates/roko-cli/src/github_integration.rs`
- Uses existing `roko-mcp-github` crate for API access
- Effort: ~600 LOC

### 2. Sentry / Error Tracking -> Auto-Fix Pipeline

**Phase improved:** Aggregate, Execute.

**Aggregate:** Production errors become corpus entries. Stack traces,
affected files, error frequency, and user impact are all context for the
agent.

**Execute:** The agent receives the error context, writes a fix + regression
test, and runs through the gate pipeline. If gates pass, creates a PR.

```
Sentry alert -> roko ingest --from-sentry SENTRY-1234
  -> builds corpus with stack trace + affected files
  -> roko funnel -> produces fix task with acceptance criteria
  -> roko plan run -> agent fixes + tests -> PR
```

**Implementation:**
- `roko ingest --from-sentry <issue-id>` -> reads stack trace, affected
  files, error context into corpus
- Acceptance criteria auto-generated: "the error from SENTRY-1234 does not
  recur when running the regression test"
- File: new `crates/roko-cli/src/sentry_integration.rs`
- Effort: ~400 LOC

### 3. MCP as Universal Connector

**Phase improved:** Aggregate (primarily).

Roko already has MCP infrastructure (`roko-mcp-code`, `roko-mcp-github`,
`roko-mcp-slack`, `roko-mcp-scripts`, `roko-mcp-stdio`). The MCP ecosystem
has 200+ servers.

Instead of building individual integrations for every tool, Roko should
use MCP servers as context sources for `roko ingest`:

```
roko ingest --mcp-server figma --resource "component/auth-dialog"
roko ingest --mcp-server notion --resource "page/architecture-doc"
roko ingest --mcp-server linear --resource "issue/PROJ-42"
```

The MCP server provides the content; Roko's corpus management indexes it.

**Implementation:**
- `roko ingest --mcp-server <name> --resource <uri>` -> calls MCP server's
  `resources/read` method, adds to corpus
- File: `crates/roko-cli/src/commands/ingest.rs` (extend)
- Effort: ~200 LOC (MCP infrastructure already exists)

### 4. Figma -> Design-to-Code Pipeline

**Phase improved:** Aggregate, Funnel, Execute.

Figma's MCP server exposes design tokens, component structure, auto layout
rules, and variables. This context feeds directly into the funnel:

**Aggregate:** Read Figma component specification into corpus.
**Funnel:** Task decomposition knows the exact component tree, variants,
and responsive breakpoints.
**Execute:** Agent receives design tokens and layout constraints as context.

```
roko ingest --mcp-server figma --resource "component/user-settings"
roko funnel --corpus user-settings-redesign
  -> Pass 1: Architecture analysis (identifies existing component structure)
  -> Pass 2: Gap analysis (compares to Figma spec)
  -> Pass 3: Task decomposition (one task per component variant)
  -> Pass 4: Dependencies (component tree determines order)
  -> Pass 5: Gates (visual regression test against Figma)
roko plan run plans/user-settings-redesign/
```

**Implementation:**
- MCP integration via `roko ingest --mcp-server figma`
- Visual regression gate: screenshot comparison against Figma export
- File: extend `crates/roko-gate/` with visual regression gate type
- Effort: ~500 LOC

### 5. Slack / Discord -> Conversational Task Dispatch

**Phase improved:** Aggregate (primarily).

Natural language task creation from where teams already communicate:

```
User in Slack: "Hey roko, the login page breaks on mobile Safari"
  -> Roko creates corpus entry with: user report + referenced page +
     codebase context for login component
  -> roko funnel -> produces fix task
  -> roko plan run -> agent fixes -> PR
  -> Roko posts to Slack: "Fixed, PR #42 ready for review"
```

**Implementation:**
- Slack bot that receives messages mentioning "roko"
- Extracts intent and references, creates corpus entry
- File: extend `crates/roko-mcp-slack/` with task creation handler
- Effort: ~400 LOC

---

## Tier 2: Execution Multipliers

These improve the execution phase specifically.

### 6. Vercel / Netlify -> Deploy Preview Gates

**Phase improved:** Execute (gate pipeline).

After an agent implements a UI task:
1. Create PR -> Vercel deploys preview
2. Gate: screenshot comparison against design spec
3. Gate: E2E test against preview URL
4. If pass: mark task done
5. If fail: rework with screenshot diff as context

**Implementation:**
- New gate type: `DeployPreviewGate`
- Waits for Vercel deploy hook to fire
- Screenshots preview URL, compares to expected
- File: new `crates/roko-gate/src/deploy_preview_gate.rs`
- Effort: ~300 LOC

### 7. Linear -> Bidirectional Sync

**Phase improved:** Review (status tracking).

For teams that use Linear as their PM tool, Roko tasks should sync
bidirectionally:

- Linear issue created -> Roko task created
- Roko task completed -> Linear issue status updated
- Linear issue reworked -> Roko task retried

This is the Symphony pattern adapted for Roko's richer execution engine.

**Implementation:**
- TrackerAdapter trait with Linear implementation
- Poll-based sync with conflict resolution (last-write-wins)
- File: new `crates/roko-cli/src/tracker_sync.rs`
- Effort: ~600 LOC

### 8. Plane (Open-Source, Self-Hosted)

**Phase improved:** Review (status tracking).

Same as Linear but for teams that want self-hosted PM. Plane has a native
MCP server, which makes integration via `roko ingest --mcp-server plane`
straightforward.

**Implementation:** Same TrackerAdapter trait, Plane implementation.
- Effort: ~400 LOC

---

## Tier 3: Ecosystem Bridges

These connect Roko to the broader tool ecosystem.

### 9. n8n / Zapier -> Workflow Automation

Instead of building individual integrations, let users wire their own:

"When Stripe webhook fires AND customer is enterprise -> create Roko corpus
entry with customer context -> funnel -> execute"

**Implementation:**
- Generic webhook receiver in roko-serve: `POST /api/webhooks/ingest`
- Accepts arbitrary JSON payload
- User configures mapping in roko.toml
- File: extend `crates/roko-serve/src/routes/` with webhook route
- Effort: ~200 LOC

### 10. Cursor / Editor Integration

**Phase improved:** Execute (agent dispatch).

Roko's ACP protocol already supports editor integration via stdio JSON-RPC.
Extending to Cursor/VS Code means:

- User selects code in editor -> "Send to Roko" -> creates task
- Task executes via Roko's dispatch (not the editor's built-in agent)
- Result appears as diff in editor

**Implementation:**
- Extend ACP session with task creation capability
- File: `crates/roko-acp/src/session.rs` (extend)
- Effort: ~300 LOC

---

## Integration Architecture for Workflow Support

### The Corpus-Centric Model

Instead of building point-to-point integrations, Roko uses the corpus as
the universal input:

```
Any Source -> roko ingest -> Corpus -> roko funnel -> Tasks -> roko plan run
```

Each integration only needs to implement one thing: "produce content for
the corpus." The funnel and execution phases are the same regardless of
where the content came from.

```rust
/// Trait for sources that can contribute to a corpus.
#[async_trait]
pub trait CorpusSource: Send + Sync {
    /// Unique identifier for this source kind.
    fn kind(&self) -> &str;

    /// Fetch content from this source.
    async fn fetch(&self, reference: &str) -> Result<Vec<CorpusEntry>>;

    /// Optional: subscribe to real-time updates from this source.
    async fn subscribe(&self, _callback: Box<dyn Fn(CorpusEntry) + Send>)
        -> Result<()>
    {
        Ok(()) // default: no real-time updates
    }
}

pub struct CorpusEntry {
    pub source_kind: String,
    pub reference: String,
    pub content: String,
    pub tokens: usize,
    pub metadata: HashMap<String, String>,
}
```

Built-in corpus sources:
- `FileSource` -- reads files from disk
- `PrdSource` -- reads PRDs from `.roko/prd/`
- `GithubSource` -- reads issues/PRs via `gh` CLI or MCP
- `McpSource` -- reads from any MCP server
- `SentrySource` -- reads error details via API
- `UrlSource` -- fetches and extracts content from URLs

### The TrackerAdapter Model (for Bidirectional Sync)

For integrations that are not just context sources but also task trackers:

```rust
#[async_trait]
pub trait TrackerAdapter: Send + Sync {
    /// Fetch tasks in active states.
    async fn fetch_active(&self) -> Result<Vec<ExternalTask>>;

    /// Update task state in the external tracker.
    async fn update_state(
        &self,
        id: &str,
        state: &str,
        comment: Option<&str>,
    ) -> Result<()>;

    /// Map external states to Roko task states.
    fn state_mapping(&self) -> &StateMapping;
}
```

Built-in tracker adapters:
- `InternalAdapter` -- Roko's own task store (default)
- `LinearAdapter` -- bidirectional sync with Linear
- `GithubAdapter` -- bidirectional sync with GitHub Issues
- `PlaneAdapter` -- bidirectional sync with Plane
- `TomlAdapter` -- Mori-style file watcher

---

## Integration Priority (Workflow-Aligned)

| Rank | Integration | Workflow Phase | User Friction Reduced | Effort |
|---|---|---|---|---|
| 1 | GitHub (issues+PRs) | All | Can ingest from GitHub, auto-create PRs | Medium |
| 2 | MCP universal ingest | Aggregate | One interface for all tools | Low |
| 3 | Sentry auto-fix | Aggregate+Execute | Error-to-fix pipeline | Medium |
| 4 | Vercel deploy preview | Execute (gates) | Visual regression testing | Medium |
| 5 | Slack conversational | Aggregate | Natural language task creation | Medium |
| 6 | Generic webhook | Aggregate | Connect any tool via n8n/Zapier | Low |
| 7 | Linear sync | Review | External PM tool support | Medium |
| 8 | Figma MCP | Aggregate+Funnel | Design-to-code pipeline | Medium |
| 9 | Plane sync | Review | Self-hosted PM tool support | Medium |
| 10 | Editor extensions | Execute | In-editor task dispatch | Medium |

### Recommended Implementation Order

**Phase A (with Phase 0-1 of PLAN.md):** GitHub ingestion + MCP universal
ingest + generic webhook. These are aggregate-phase improvements that work
with `roko ingest`.

**Phase B (with Phase 2-3):** Sentry auto-fix + Vercel deploy preview.
These are execute-phase improvements that work with the gate pipeline and
task validation.

**Phase C (with Phase 4):** Slack conversational + Linear sync. These
require the funnel workflow to exist so they can go from input to tasks
automatically.

**Phase D (post-MVP):** Figma, Plane, editor extensions.

---

## Workflow Combinations

Individual integrations are useful. Combinations are where the workflow
becomes powerful:

### Combo 1: Sentry + GitHub + Slack = Autonomous Bug Fix

```
Production error fires
  -> Sentry webhook -> roko ingest --from-sentry SENTRY-1234
  -> roko funnel (auto, single-pass for simple fixes)
  -> roko plan run (agent fixes + tests)
  -> Gate: compile + test + clippy
  -> PR created on GitHub
  -> Slack notification: "Fixed SENTRY-1234, PR#567 ready"
  -> Human merges
  -> Deploy -> Sentry error count drops
```

### Combo 2: Figma + GitHub = Design-to-Ship

```
Designer finishes component in Figma
  -> roko ingest --mcp-server figma --resource "component/settings"
  -> roko funnel (architecture + gaps + tasks + deps + gates)
  -> roko plan run (agents implement each component variant)
  -> Gate: compile + test + visual regression (vs. Figma export)
  -> PR created on GitHub
  -> Vercel deploy preview (if configured)
  -> Designer reviews preview URL
  -> Approved -> merged -> shipped
```

### Combo 3: Slack + Research + Funnel = Feature from Conversation

```
PM types in Slack: "We need a user settings page"
  -> Roko creates corpus: PM's description + existing codebase analysis
  -> roko funnel (5 passes with PM approval via Slack)
  -> roko plan run (parallel execution)
  -> All tasks pass gates
  -> PRs created
  -> Slack: "Settings page implemented, 3 PRs ready for review"
```

---

## What Makes This Different from Symphony

Symphony's integration model: **one tracker (Linear), one agent backend
(Codex), one flow (issue -> agent -> PR -> review).**

Roko's integration model: **any source -> corpus -> funnel -> DAG-parallel
execution with tiered context and built-in gates -> any tracker for review.**

The key difference is the **corpus and funnel layers.** Symphony goes
directly from a human-written Linear issue to an agent. Roko goes from any
source through progressive refinement into properly decomposed, validated,
right-sized tasks.

This means:
1. **Better context:** The agent receives a curated, right-sized prompt
   instead of a raw issue description.
2. **Better decomposition:** Complex work is split into tasks that match
   model capabilities.
3. **Better orchestration:** Dependencies are explicit, parallelism is
   maximized, file conflicts are detected.
4. **Better verification:** Acceptance criteria are testable shell commands,
   not prose.
5. **Better learning:** Every execution feeds back into the learning
   subsystem for future improvement.

---

## Sources

### External
- [GitHub Agentic Workflows](https://github.blog/ai-and-ml/automate-repository-tasks-with-github-agentic-workflows/)
- [GitHub Copilot for Jira](https://github.blog/changelog/2026-03-05-github-copilot-coding-agent-for-jira-is-now-in-public-preview/)
- [Figma MCP Server](https://www.figma.com/blog/design-systems-ai-mcp/)
- [Slack AI Agent Platform](https://slack.com/blog/news/powering-agentic-collaboration)
- [Vercel Agentic Infrastructure](https://vercel.com/blog/agentic-infrastructure)
- [Plane (Open Source PM)](https://github.com/makeplane/plane)
- [n8n (400+ Integrations)](https://n8n.io/)
- [MCP Ecosystem](https://www.essamamdani.com/blog/complete-guide-model-context-protocol-mcp-2026)
- [Symphony SPEC.md](https://github.com/openai/symphony/blob/main/SPEC.md)

### Internal
- `crates/roko-mcp-code/` -- code intelligence MCP server
- `crates/roko-mcp-github/` -- GitHub MCP integration
- `crates/roko-mcp-slack/` -- Slack MCP integration
- `crates/roko-mcp-scripts/` -- script execution MCP
- `crates/roko-mcp-stdio/` -- generic stdio MCP
- `crates/roko-serve/src/routes/` -- ~85 routes (webhook receiver extension point)
- `crates/roko-gate/` -- gate pipeline (deploy preview gate extension point)
- `crates/roko-agent/src/` -- 8+ backend files (dispatch diversity)
