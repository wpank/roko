# Service Integrations and Automation Workflows

> **Audience**: DevOps engineers, platform teams, workflow designers
> **Scope**: How roko agents integrate with Slack, GitHub, Linear, Notion, and how event-driven workflows compose multi-service actions

---

## Architecture: MCP as Universal Adapter

The agent does not know about Slack's Events API or Linear's GraphQL. Everything is an MCP tool: `post_slack_message`, `create_linear_issue`, `search_notion`. Adding a new service = adding a new MCP server. Agent code is unchanged.

Each MCP server runs as a separate child process (JSON-RPC over stdio). Isolated, swappable, no crosstalk.

---

## Service-Specific Integration Details

### Slack

**Connection modes**: Socket Mode (WebSocket, local dev) or HTTP Mode (production)

**Key events**: `message.channels`, `app_mention`, `message.im`, `reaction_added`, `channel_created`

**MCP tools**: `send_message`, `reply_to_thread`, `add_reaction`, `list_channels`, `search_messages`, `get_channel_history`, `get_thread_replies`, `get_users`

**Rate limits**: Posting ~1/sec, reads ~20/min, search ~50/min, events 30K/hr/workspace

### GitHub

**Auth**: GitHub App (JWT → installation token, 1hr auto-rotation)

**Webhook events**: `issues.opened`, `pull_request.opened`, `issue_comment.created`, `pull_request_review_comment.created`, `push`

**MCP tools**: `create_issue`, `create_pull_request`, `create_or_update_file`, `push_files`, `search_code`, `get_file_contents`, `list_issues`, `fork_repository`

**Rate limits**: 5,000/hr per token

### Linear

**API**: Single GraphQL endpoint at `https://api.linear.app/graphql`

**MCP tools**: `create_issue`, `update_issue`, `search_issues`, `get_issue`, `list_teams`, `add_comment`, `create_project`

**Rate limits**: 1,500 requests/hr, 250K complexity points/hr

### Notion

**API**: REST at `https://api.notion.com/v1/`

**MCP tools**: `create_page`, `update_page`, `search`, `query_database`, `append_blocks`, `retrieve_page`

**Rate limits**: 3 requests/second (tightest of all services)

**No webhooks** — requires polling via `last_edited_time`

---

## Event-Driven Automation Workflows

### The Pattern

```
Event → Event Router → Agent (intelligence in middle) → Actions (across services)
```

Not data forwarding — the agent reads, analyzes, makes decisions, and takes multi-service actions.

### Concrete Workflow Examples

**1. Paper → Research Artifacts**

Trigger: Academic paper URL posted in Slack `#research` channel.
Agent (Researcher, Opus): Fetches paper, creates Notion summary, creates GitHub repo + PRD if relevant, creates Linear issues, posts Slack thread reply.
Cost: ~$0.42

**2. Meeting Transcript → Action Items**

Trigger: Transcript posted in Slack `#meetings` (detected by timestamp regex `\[\d{2}:\d{2}\]`).
Agent (Strategist, Opus): Extracts decisions, action items (who/what/when), open questions. Updates Notion meeting notes, creates Linear issues, creates GitHub issues for code changes.

**3. Slack Discussion → GitHub PRD + Autonomous Build**

Trigger: `@roko-bot "turn this into a project"` in any thread.
Agent: Reads full thread history, synthesizes PRD, decomposes into plans, commits to repo, triggers roko plan execution.

```toml
[post_agent]
trigger_roko = true
roko_queue = "plans/"
roko_branch = "auto"
```

**4. GitHub Issue → Investigation + Slack Report**

Trigger: `issues.opened` with label `bug` or `needs-triage`.
Agent (Researcher): Clones repo, searches code, checks git history, assesses severity, posts analysis comment + Slack notification.

**5. Daily Standup Bot**

Trigger: Cron `0 9 * * 1-5` (9 AM weekdays).
Agent (Researcher, Sonnet): Checks Linear completed/in-progress, GitHub PRs, Notion milestones. Compiles team status, posts to Slack.

**6. PR Review + Notification**

Trigger: `pull_request.opened` (excluding drafts, excluding dependabot).
Agent (Reviewer, Opus): Clones PR branch, reviews code, posts GitHub review with inline comments, notifies Slack `#code-review`.

### Workflow Configuration

```toml
[trigger]
type = "slack_message"
channel = "#research"
pattern = "arxiv\\.org|papers\\.ssrn\\.com"

[agent]
role = "researcher"
model = "claude-opus-4-6"
effort = "high"
prompt = """
1. Fetch and read the paper
2. Create Notion summary
3. If relevant: create GitHub repo, PRD, Linear issues
4. Post Slack thread reply
"""

[tools]
required = ["slack-mcp", "github-mcp", "linear-mcp", "notion-mcp", "web-fetch"]

[output]
slack_thread_reply = true
```

### Durable Execution (Temporal)

Multi-service workflows use Temporal for crash-safe execution:
- If crash after step 3 of 6: replay from history, resume at step 4
- Per-step retry policies with independent timeouts
- UI shows running/completed workflows, every activity attempt

### Rate Limiting Infrastructure

Per-service token bucket with backoff:

```rust
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}
```

Agent doesn't know about rate limiting — infrastructure handles it transparently.

| Service | Limit | Backoff |
|---|---|---|
| Slack posting | ~1/sec | HTTP 429 + Retry-After |
| Linear | 1,500/hr | HTTP 429 + X-RateLimit-Reset |
| Notion | 3/sec | HTTP 429 + Retry-After |
| GitHub | 5,000/hr | HTTP 403 + X-RateLimit-Reset |

---

## CLI Modes (Seven Usage Patterns)

| Mode | Detection | Use Case |
|---|---|---|
| **Interactive REPL** | `isatty(stdin)` | Developer conversation |
| **One-shot** | CLI argument provided | Single task, exit code indicates success |
| **Pipe** | `!isatty(stdin)` and no prompt arg | Process piped input (issue body, diff) |
| **GitHub Bot** | Webhook server mode | Auto-respond to issues/PRs |
| **CI Agent** | One-shot in GitHub Actions | Fix failing tests, review PRs |
| **HTTP Service** | `serve` subcommand | REST + SSE, Agent-as-a-Service |
| **Daemon** | `serve --daemon` | Background workflow engine |

### HTTP API Endpoints

```
POST   /agent/run              Start one-shot agent task
GET    /agent/events/{task_id} SSE stream of agent events
POST   /agent/turn             Send new turn to existing session
GET    /sessions               List active sessions
DELETE /sessions/{id}          End session, kill agent
GET    /health                 Liveness probe
```

### GitHub Bot Mode

| Event | Agent Role | Action |
|---|---|---|
| `issues.opened` | Strategist | Label, triage, post analysis |
| `pull_request.opened` | Reviewer | Clone, review, post inline comments |
| `issue_comment` (@mention) | Implementer | Resume session, respond |
| `push` (watched branch) | Researcher | Analyze changes, post summary |

---

## Deployment Options

| Target | RAM | Cost | Use Case |
|---|---|---|---|
| Fly.io (single agent) | 256 MB | ~$2/mo | Personal use |
| Fly.io (pool, 3-5 concurrent) | 1 GB | ~$7/mo | Small team |
| Fly.io (review bot, 10+ repos) | 2 GB | ~$30/mo | Organization |
| macOS Launch Agent | Local | $0 | Local daemon |
| Linux systemd | Local | $0 | Server daemon |

### Execution Presets

```bash
--preset quality   # opus, max_iterations=5, full review
--preset balanced  # sonnet, max_iterations=3, standard review
--preset cost      # sonnet, max_iterations=2, token optimization
--preset speed     # sonnet, express mode, minimal review
```

---

## Workflow Template Library (10 Pre-Built)

| # | Template | Trigger | Agent Role | Model | Temporal? |
|---|---|---|---|---|---|
| 1 | Paper Ingestion | Slack: arxiv URL | Researcher | Opus | Yes |
| 2 | Meeting Actions | Slack: transcript | Strategist | Opus | Yes |
| 3 | Issue Triage | GitHub: issue opened | Researcher | Opus | No |
| 4 | PR Review | GitHub: PR opened | Reviewer | Opus | No |
| 5 | Discussion → Project | Slack: @mention | Strategist | Opus | Yes |
| 6 | Daily Standup | Cron: 9 AM weekdays | Researcher | Sonnet | No |
| 7 | Changelog | GitHub: release published | Scribe | Sonnet | No |
| 8 | Dependency Alert | GitHub: dependabot PR | Auditor | Sonnet | No |
| 9 | Incident Response | PagerDuty: incident | Researcher | Opus | Yes |
| 10 | Onboarding | Linear: new member | Scribe | Sonnet | Yes |

### Durable Execution (Temporal) — When to Use

| Characteristic | Fire-and-Forget | Use Temporal |
|---|---|---|
| Single output service | Yes | No |
| Multiple output services | No | **Yes** |
| Idempotent actions only | Yes | No |
| Non-idempotent (create repo) | No | **Yes** |
| < 30 seconds | Yes | No |
| > 2 minutes | No | **Yes** |
| Low failure cost | Yes | No |
| High failure cost | No | **Yes** |

Temporal benefits:
1. **Durable state** — crash after step 3 of 6 → replay from history, resume at step 4
2. **Per-step retry** — each activity has independent timeout/backoff
3. **Visibility** — UI shows running workflows, every attempt, every retry

### Error Handling Matrix

| Error Type | Example | Handling |
|---|---|---|
| Trigger mismatch | Regex false positive | Agent recognizes irrelevance, posts nothing |
| Agent failure | Timeout, rate limit | Retry agent turn (up to 2×), then post error |
| MCP tool failure | API down | Per-tool retry with backoff, Temporal retries independently |
| Partial completion | 3 of 5 actions done | Temporal resumes from checkpoint |
| Duplicate execution | Webhook retry | Dedup by event ID hash (1-hour window) |

### Workflow Configuration Structure

```
~/.roko/
  config.toml                 # Global agent config
  integrations/
    slack.toml               # Slack connection settings
    linear.toml              # Linear connection settings
    notion.toml              # Notion connection settings
    github.toml              # GitHub App credentials
    google.toml              # Google OAuth credentials
  workflows/
    paper-to-research.toml   # Workflow definition
    meeting-to-actions.toml
    custom/
      my-workflow.toml       # User-defined workflows
  sessions/                  # Session state
  mcp-servers.json           # Auto-generated MCP config
```

### Workflow Observability

Every execution produces structured telemetry:

```json
{
  "workflow": "paper-to-research",
  "trigger_event_id": "ev_abc123",
  "started_at": "2026-04-02T14:30:00Z",
  "completed_at": "2026-04-02T14:32:15Z",
  "duration_ms": 135000,
  "agent": {
    "role": "researcher",
    "model": "claude-opus-4-6",
    "tokens_in": 45200,
    "tokens_out": 8900,
    "cost_usd": 0.42
  },
  "actions": [
    { "service": "notion", "action": "create_page", "status": "success", "duration_ms": 1200 },
    { "service": "github", "action": "create_repo", "status": "success", "duration_ms": 3400 },
    { "service": "linear", "action": "create_issues", "count": 5, "duration_ms": 2800 },
    { "service": "slack", "action": "reply_thread", "status": "success", "duration_ms": 400 }
  ],
  "status": "completed"
}
```

### MCP Server Configuration (Per-Service)

```json
{
  "mcpServers": {
    "slack": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-slack"],
      "env": { "SLACK_BOT_TOKEN": "$SLACK_BOT_TOKEN", "SLACK_TEAM_ID": "$SLACK_TEAM_ID" }
    },
    "linear": {
      "command": "npx",
      "args": ["-y", "mcp-linear"],
      "env": { "LINEAR_API_KEY": "$LINEAR_API_KEY" }
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_PERSONAL_ACCESS_TOKEN": "$GITHUB_TOKEN" }
    },
    "notion": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-notion"],
      "env": { "NOTION_TOKEN": "$NOTION_TOKEN" }
    }
  }
}
```

Each server runs as an isolated child process (JSON-RPC over stdio). Adding a new service = adding a new MCP server entry. Agent code is unchanged.

### Authentication Patterns

| Service | Method | Token Prefix | Rotation | Scope |
|---|---|---|---|---|
| Slack | Bot Token | `xoxb-...` | Manual | Per-workspace |
| Linear | API Key | `lin_api_...` | Manual | Per-workspace |
| Notion | Integration Token | `ntn_...` | Manual | Per-integration |
| GitHub | GitHub App (JWT → Install Token) | `ghs_...` | Auto (1hr) | Per-repository |
| Google | OAuth 2.0 + Service Account | Bearer | Auto (1hr) | Per-scope |

**GitHub App auth flow**: Private key (permanent) → JWT (10 min) → Installation token (1 hr) → API calls. The token auto-rotates; no manual key management.

---

## The Human-Agent Interface

The TUI is not a dashboard. It is a cockpit for operators managing multiple parallel agent pipelines. The distinction matters: a dashboard presents information passively; a cockpit enables decisions under time pressure. An operator running 12 concurrent plans with 30+ agents needs the same kind of cognitive support that a pilot needs managing flight systems -- not more data, but the right data at the right moment.

### Trust Calibration

Operators assess system reliability per-plan and per-phase, adjusting autonomy dynamically. A new plan touching unfamiliar crates gets careful review -- the operator watches agent output, inspects gate results, and approves phase transitions manually. A proven pattern (adding a derive macro, updating doc comments, wiring an existing trait implementation) gets fast-tracked because the operator has seen it succeed repeatedly.

Trust is not binary. It operates on a continuous scale informed by:

- **Plan novelty**: first-time patterns demand attention; repeated patterns earn trust
- **Crate risk**: safety-critical crates (agent dispatch, gate pipeline) get more scrutiny than leaf utilities
- **Historical success rate**: a plan type that has passed gates 50 consecutive times earns a different trust level than one that fails 30% of the time
- **Blast radius**: a plan that touches 1 file in 1 crate is low-risk regardless of novelty; a plan that touches 8 crates needs earned trust

Trust calibration feeds directly into autonomy levels. The system tracks plan-type success rates and surfaces them to the operator as a trust score, enabling informed delegation decisions rather than gut-feel ones.

### Mixed-Initiative Interaction

The protocol for who initiates action -- the system or the operator -- follows a decision-theoretic framework. Not every decision deserves the same treatment.

**System proposes, operator approves** (high-risk decisions):
- Merging a plan that touches safety-critical code
- Restarting an agent that has been running for 20+ minutes with no output
- Escalating a plan from Sonnet to Opus after repeated gate failures
- Force-advancing past a stuck phase

**System acts autonomously** (low-risk proven patterns):
- Restarting an agent after a ghost turn (no output for 60 seconds)
- Applying compile-fix escalation (nudge → restart → model upgrade)
- Collapsing completed plans in the TUI
- Rotating agent context when approaching token limits

The boundary between these categories shifts as trust builds. A conductor intervention that initially required operator approval becomes autonomous after the operator has approved the same pattern 10 times with no overrides. Initiative flows to whichever participant -- human or system -- is better positioned for the current subtask. The conductor handles repetitive mechanical failures; the operator handles novel architectural judgments.

### Cognitive Load Management

Working memory holds approximately 7 plus or minus 2 chunks simultaneously (Miller, 1956). When 12 plans are running, each with multiple agents, each with streaming output, gate results, and phase transitions, the raw information volume exceeds working memory by an order of magnitude. The interface must aggressively compress.

Strategies for staying within cognitive limits:

- **Attention-driven rendering**: the 4 plans that need attention expand with full detail; the 8 healthy plans collapse to single-line status summaries
- **Exception-based surfacing**: only deviations from expected behavior generate visible events. A gate pass is normal and silent. A gate failure is abnormal and prominent.
- **Temporal compression**: events older than the operator's attention window fade. The operator sees the last 30 seconds of agent output, not the last 30 minutes.
- **Hierarchical grouping**: plans group by wave, agents group by plan, errors group by file. The operator navigates a tree, not a flat list.

The goal: at any moment, the operator can answer "what needs my attention right now?" with a single glance, not a scanning operation.

### Graduated Autonomy (Sheridan's Levels)

New agents start constrained. The system requires human approval for everything -- plan decomposition, phase transitions, merge attempts. As trust builds through successful outcomes, constraints relax automatically. This is Sheridan and Verplank's (1978) 10-level automation taxonomy applied to agent orchestration.

The progression for a given plan type:

| Phase | Autonomy Level | Operator Role |
|---|---|---|
| First execution | Level 5: execute if human approves | Active participant |
| After 5 successes | Level 6: execute and inform | Attentive monitor |
| After 15 successes | Level 7: execute, human can override | Passive monitor with veto |
| After 50 successes | Level 9: inform only if system decides to | Notified on exceptions only |

Full autonomy is earned through demonstrated competence, not configured. The operator never needs to manually adjust autonomy levels -- the system tracks outcomes and proposes promotions. The operator can always demote a plan type back to a lower autonomy level if conditions change (new crate dependencies, API changes, team policy updates).

### Explanation and Transparency

The operator needs to understand not just what happened but why a decision was made. Every conductor intervention, phase transition, and plan-level decision surfaces structured reasoning.

A restart decision surfaces as:

```
[conductor] Plan roko-gate/adaptive-thresholds: restarting agent (attempt 2/3)
  reason: ghost turn detected (90s no output)
  context: phase=implement, task=3/7, gate_failures=0
  alternatives_considered: nudge (rejected: already nudged at t-45s), escalate_model (premature: first restart)
  expected_outcome: agent resumes from last tool output
```

This is not verbose logging. It is structured explanation at the right abstraction level -- enough for the operator to evaluate the decision, not so much that it becomes noise. The operator who disagrees can override; the operator who agrees glances and moves on. Transparency builds the trust that enables graduated autonomy.

---

## Information Architecture for Operators

### Three Attention Layers

Not all information demands the same cognitive processing. The interface organizes events into three attention layers that map to distinct operator responses:

**Alert layer** -- immediate action required:
- Gate failures after maximum retry attempts
- Budget overruns (token spend exceeding plan allocation by 2x)
- Agent crashes with no automatic recovery
- Merge conflicts that block dependent plans
- Conductor circuit breaker tripped (too many failures in a time window)

**Monitor layer** -- track but do not act:
- Plan progress (phase transitions, task completion percentages)
- Resource consumption (token usage trending, model distribution)
- Agent pool utilization (active/idle/blocked counts)
- Gate pass rates by crate (normal variance, not yet concerning)

**Review layer** -- post-hoc analysis:
- Completed plans (final gate results, total cost, duration)
- Learning trends (playbook growth, efficiency improvements, model routing accuracy)
- Codebase health metrics (affordance trends, scent trends, rework rates)

Each layer has different rendering treatment. Alerts get color, sound (terminal bell), and persistent visibility until acknowledged. Monitor items get compact inline display that updates in place. Review items are available on demand but never interrupt the operator's current focus.

### The 7 Plus or Minus 2 Principle

No more than 7 active concerns displayed simultaneously. This is not a guideline -- it is a hard constraint derived from working memory research (Miller, 1956; Cowan, 2001 refines to approximately 4 chunks for unrehearesed items).

If 12 plans are running:
- Show the 4 that need attention (failed, stuck, budget-warning) with expanded detail
- Collapse the remaining 8 into a single summary line: "8 plans healthy (3 implementing, 2 reviewing, 3 gating)"
- The operator drills into collapsed plans only when curious, never because the interface forces them to scan

If 40 agents are active across all plans:
- Show the 2-3 that are in abnormal states (stalled, high token consumption, repeated gate failures)
- Aggregate the rest: "37 agents nominal"

The interface dynamically adjusts what counts as "needs attention" based on the current state. During a healthy cruise, the threshold is strict -- only failures surface. During a crisis (multiple plans failing, circuit breaker near tripping), the threshold loosens to give the operator more situational awareness.

### Progressive Disclosure

Information follows a three-level drill-down: summary, detail, raw logs.

**Summary**: plan name, status badge, phase indicator, health score. One line. The operator sees 12 of these and identifies the 2 that matter.

**Detail**: task list with completion checkboxes, gate result history, agent turn count, token consumption, conductor intervention log. The operator expands a specific plan to understand its situation.

**Raw logs**: full agent output stream, complete gate stdout/stderr, git diff of changes. The operator drills to raw when diagnosing a specific failure.

The operator drills down when curious, surfaces when satisfied. The interface never forces the operator into detail they did not request. Back-navigation is instant -- one keystroke returns to summary level.

### Event Priority Scoring

Each event receives a priority score computed as:

```
priority = severity * novelty * operator_interest
```

- **Severity**: gate failure (1.0), budget warning (0.7), phase transition (0.3), routine progress (0.1)
- **Novelty**: first occurrence of this event type in this plan run (1.0), seen before in this run (0.5), seen in previous runs of the same plan type (0.2)
- **Operator interest**: inferred from behavior -- plans the operator has expanded recently (1.0), plans the operator has never inspected (0.3), plans explicitly marked low-priority (0.1)

Conductor interventions always surface regardless of score. Routine gate passes on healthy plans are collapsed. The scoring adapts: if the operator consistently expands a particular plan after an event type, that event type's operator_interest weight increases for that plan.

---

## The Agent Ecology: Co-Evolution with Codebases

Agents do not merely inhabit the codebase. They construct it. And the codebase constructed by one agent becomes the environment inherited by the next. Every commit is simultaneously an act of implementation and an act of world-building. The distinction between "agent" and "environment" dissolves when both are continuously rewriting each other.

### Niche Construction

Odling-Smee, Laland, and Feldman (2003) formalize the observation that organisms do not passively adapt to fixed environments -- they actively modify their selective landscape. The canonical example: beavers build dams, creating lakes, which select for aquatic adaptations in beavers, which build better dams. The feedback loop is causal, not metaphorical.

Applied to roko: each plan's implementation merges to the working branch, changing the codebase for all subsequent plans. An agent that adds documentation, writes tests, and maintains clean interfaces improves the codebase for future agents -- positive niche construction. An agent that creates tangled dependencies, skips tests, and leaves undocumented public APIs degrades it -- negative niche construction.

Three properties matter:

1. **Ecological inheritance**: an agent inherits not just its plan instructions (the PRD) but a modified codebase -- the accumulated work of all previous agents. The codebase at plan 100 is fundamentally different from plan 1, not because the PRD changed, but because 99 agents have constructed it.
2. **Positive and negative construction**: agents can improve or degrade their environment. The system must measure which direction construction is going and intervene when it turns negative.
3. **Cumulative construction**: small, individually insignificant changes accumulate into massive environmental transformations. A single doc comment is trivial. A thousand doc comments across a hundred plans transform a codebase from opaque to self-documenting.

### Affordances

Gibson (1979) introduced affordances: the action possibilities that an environment offers relative to an agent's capabilities. Well-documented code with clean public APIs and comprehensive tests "affords" easy modification -- the agent reads the API, understands the contract, writes conforming code, and the tests validate it. Monolithic undocumented code with circular dependencies affords nothing but full rewrites -- the agent must read everything, guess at invariants, and hope.

For an LLM-based coding agent, affordance is concrete and measurable:

- **High affordance**: clear `pub` interfaces with doc comments, small focused modules (under 300 lines), comprehensive tests with descriptive names, explicit trait-based extension points, low coupling. The agent modifies this code with minimal context and high confidence.
- **Low affordance**: `pub(crate)` everything with no docs, monolithic modules (over 1000 lines), no tests, implicit conventions, circular dependencies. The agent must gather massive context and still operates at low confidence.

The critical insight: agents should assess affordances before acting. A task on high-affordance code is fundamentally different from the same task on low-affordance code, even if the plan description is identical.

### Information Foraging

Pirolli and Card (1999) adapted optimal foraging theory to information environments. Agents follow "scent" cues -- descriptive names, doc comments, test names, module paths that predict content -- to find relevant code. Strong scent means efficient navigation: the agent locates the right file, reads the right function, and proceeds. Weak scent means wasted tokens: the agent reads a file, finds nothing useful, moves to another, finds nothing, burns through context budget on exploration instead of implementation.

Information foraging theory predicts a patch-leaving strategy: agents stay in an information patch (a file, a module, a crate) as long as the expected information gain per token exceeds the cost of switching. High-scent environments keep agents in productive patches longer. Low-scent environments trigger excessive patch-switching.

Practical scent signals in a codebase:
- Descriptive function and type names (`pub fn validate_transaction()` not `pub fn process()`)
- Module paths that predict content (`crates/roko-gate/src/adaptive.rs` not `crates/roko-gate/src/utils.rs`)
- Doc comments on public items
- Test names that describe behavior (`test_capability_consumed_after_single_use()`)
- Import statements that trace dependency chains

### Stigmergic Coordination

Agents coordinate through the shared environment -- code, pattern files, context documents -- not through direct messaging. This is stigmergy (Grasse, 1959): coordination through modification of a shared environment rather than explicit communication.

Plan 50's implementer does not message plan 80's implementer. Plan 50 commits code; plan 80's implementer reads it. The code IS the communication channel. `CONTEXT.md`, discovered patterns files, and completion summaries are explicit stigmergic marks -- deliberate signals left in the environment for future agents.

The scaling advantage is decisive: stigmergic coordination costs O(1) per agent. Direct messaging costs O(N-squared) -- every agent pair needs a communication channel. With 40 concurrent agents, direct messaging produces 780 potential channels. Stigmergy produces 1 shared environment that all agents read and write.

### The Exponential

If each plan improves the average affordance by even 1% -- a modest target achievable through deliberate niche improvement instructions in the agent prompt -- then after 100 plans the cumulative improvement is approximately 170% (1.01 to the 100th power equals 2.70). After 200 plans, approximately 625%. The environment becomes radically easier to work in, not through any single dramatic improvement, but through the relentless accumulation of small ones.

The inverse is equally powerful and more dangerous. Agents that degrade affordances -- adding complexity without documentation, creating tangled dependencies, skipping tests -- make every future agent less effective. Those less-effective agents produce lower-quality code, which further degrades affordances. This is the death spiral of negative niche construction: gate failure rates increase, rework rates climb, and throughput collapses.

### Co-Evolution Metrics

Surface these in the TUI as a "Codebase Health" panel:

| Metric | What it measures | Healthy trend |
|---|---|---|
| Affordance trend | Mean affordance score at each plan boundary | Stable or rising |
| Scent trend | Doc comment count, test count, descriptive-name ratio over time | Rising |
| Gate pass rate by crate | Per-crate success rate through the gate pipeline | Stable above 80% |
| Context efficiency | Average token budget consumed per task | Declining (codebase becoming self-documenting) |
| Rework rate | Files modified, then re-modified within 3 plans by non-dependent plans | Declining |

When trends turn negative, the system injects a warning into the orchestrator's planning context: "crate roko-gate affordances declining over last 10 plans -- consider scheduling a refactoring task." The operator can approve, defer, or dismiss. Monitoring and preventing negative niche construction is a survival requirement for any long-running autonomous build system.

---

## Observability Architecture

### Structured Event Pipeline

The event bus is the nervous system of the observability stack. Every significant action in the system produces a typed event with a monotonic sequence number, enabling total ordering and replay.

Core properties of the event bus:
- **Typed events**: each event is a Rust enum variant with structured fields, not a string message. Type safety from emission through storage through rendering.
- **Monotonic sequence numbers**: events are globally ordered. Sequence 4712 happened before 4713, regardless of wall clock skew between threads.
- **Ring buffer**: the last 10,000 events are held in memory for immediate access. Older events are available from persistent storage. The ring buffer enables fast TUI rendering without disk reads.
- **Replay capability**: any consumer can replay from a sequence number. A TUI that reconnects after a network drop replays from its last-seen sequence, not from the beginning.

### Event Types

Events specific to the plan-execute-gate-persist loop:

| Event | Payload | When |
|---|---|---|
| `PlanQueued` | plan_id, priority, estimated_complexity | Plan enters the execution queue |
| `PlanStarted` | plan_id, assigned_model, context_budget | Executor begins plan processing |
| `PlanCompleted` | plan_id, status, duration, token_cost | Plan finishes (success or failure) |
| `PhaseTransitioned` | plan_id, from_phase, to_phase, reason | Phase boundary crossed |
| `AgentSpawned` | plan_id, agent_id, role, model | Agent process created |
| `AgentCompleted` | agent_id, status, turns, tokens_used | Agent finishes its work |
| `GateExecuted` | plan_id, gate_name, passed, details | Individual gate runs |
| `ReviewSubmitted` | plan_id, reviewer_verdict, comments | Review phase completes |
| `ConductorIntervention` | plan_id, intervention_type, reason | Conductor takes corrective action |
| `MergeAttempted` | plan_id, result, conflicts | Plan output merged to branch |
| `ContextAssembled` | plan_id, token_count, sources | Context window constructed |

### Four-Layer Architecture

The observability stack has four layers, each serving a different consumer:

**Layer 1 -- Event Bus**: in-process typed events with ring buffer. Consumers: TUI widgets, conductor watchers, internal metrics. Latency: microseconds.

**Layer 2 -- Persistent Storage**: JSONL append-only log on disk. Consumers: crash recovery, post-hoc analysis, audit. Each event is one JSON line, enabling `tail -f` for debugging and structured replay for recovery. The append-only property means the log is never corrupted by partial writes -- the last line may be incomplete, but all preceding lines are valid.

**Layer 3 -- Streaming**: real-time event delivery to external consumers. TUI receives events via the in-process bus. External dashboards connect via WebSocket or SSE endpoints. Each stream can filter by event type and plan ID, avoiding firehose overwhelm.

**Layer 4 -- Distributed Tracing**: OpenTelemetry integration for cross-service visibility. A plan execution maps to a trace. Each phase maps to a span. Agent spawns and gateway inference calls map to child spans. This enables correlation with external systems -- when an MCP tool call to GitHub's API is slow, the trace shows the latency attributed to the right agent in the right plan in the right phase.

### Trace Structure

```
Trace: plan-execution (plan_id=roko-gate/adaptive-thresholds)
  Span: phase-implement (duration=4m32s)
    Span: agent-spawn (role=implementer, model=opus)
      Span: tool-call (tool=read_file, file=adaptive.rs, 120ms)
      Span: tool-call (tool=edit_file, file=adaptive.rs, 85ms)
      Span: tool-call (tool=cargo_check, 3.2s)
    Span: gate-pipeline (duration=12s)
      Span: gate-compile (passed=true, 8s)
      Span: gate-test (passed=true, 3s)
      Span: gate-clippy (passed=true, 1s)
  Span: phase-review (duration=2m15s)
    Span: agent-spawn (role=reviewer, model=opus)
```

### Crash Recovery via Event Replay

The JSONL append-only log enables crash recovery that is more robust than state snapshots alone. A state snapshot captures the system at a point in time; an event log captures the full history. On crash recovery, the system replays the event log from the last checkpoint, reconstructing state by processing each event in sequence. This is event sourcing (Fowler, 2005): the log of events IS the system of record, and the current state is a derived view.

The advantage over snapshot-only recovery: if the crash corrupts the snapshot, the event log can reconstruct it from scratch. If the crash occurs during a state transition, the event log shows exactly which events were processed and which were not, enabling precise recovery rather than "restart from last known good."

Research: Sigelman et al. (2010) -- Dapper, distributed tracing at Google. Fowler (2005) -- Event Sourcing, state as a sequence of events.

---

## Security Architecture for Service Integrations

### Capability-Based Access

Each agent receives typed capability tokens scoped to its role. The token is not a string permission -- it is a Rust type that the compiler enforces. An implementer agent receives a `WorktreeWrite` capability scoped to its assigned worktree. A reviewer agent receives a `WorktreeRead` capability. The conductor receives `InterventionCapability` tokens that authorize specific corrective actions.

Capabilities are:
- **Typed**: `WorktreeWrite` and `WorktreeRead` are different types. You cannot pass a read capability where a write is required. The compiler catches this, not a runtime check.
- **Scoped**: a `WorktreeWrite` token includes the worktree path. It cannot be used to write to a different worktree. An agent assigned to plan A cannot modify plan B's worktree.
- **Non-transferable**: capabilities are bound to the agent process that received them. An agent cannot pass its capabilities to another agent or to a subprocess it spawns.
- **Revocable**: the conductor can revoke capabilities at any time. When a plan is cancelled, all agents in that plan have their capabilities revoked immediately.
- **Auditable**: every capability grant and use is logged to the audit chain.

### Taint Tracking

Agent outputs are tainted until validated by gates. This is not a metaphor -- taint is a property tracked through the system.

- **Tainted code**: agent-generated code carries a taint marker. Tainted code cannot be merged to the target branch. Gates (compile, test, clippy, diff review) validate the code; passing all gates removes the taint.
- **Tainted context**: agent-generated context entries (discoveries, pattern observations, completion summaries) are tainted until the plan they originated from passes all gates. A tainted context entry can be read by other agents but is marked as unvalidated.
- **Taint propagation**: if tainted code is used as input to another agent (e.g., an agent reads a file modified by a different agent whose gates have not yet passed), the downstream agent's output inherits the taint. The taint clears only when the upstream gates pass.

The taint system prevents a failing agent's output from contaminating the rest of the system. A plan that produces code which fails compilation cannot have its "discoveries" promoted to the shared knowledge base, because those discoveries were made in the context of incorrect code.

### PRISM Lifecycle Hooks

The PRISM framework (2025) defines 10 hook points for runtime enforcement across the agent lifecycle. Each hook is a point where safety checks can intercept, validate, or modify agent behavior:

| Hook | When | What it enforces |
|---|---|---|
| **Ingress** | External input arrives | Input sanitization, size limits, format validation |
| **Prompt construction** | System prompt assembled | Role boundary enforcement, no privilege escalation in prompt |
| **Tool selection** | Agent chooses a tool | Tool allowlist per role, deny dangerous tools for low-trust agents |
| **Execution** | Tool runs | Sandboxing, resource limits, timeout enforcement |
| **Output generation** | Agent produces text | Output filtering, PII detection, secret scanning |
| **Memory write** | Agent writes to episodic memory | Taint checking, consistency validation |
| **Retrieval** | Agent reads from memory/context | Access control, taint annotation on retrieved entries |
| **Tool invocation** | MCP tool called | Rate limiting, capability checking, audit logging |
| **External communication** | Agent sends data outside the system | Egress filtering, data loss prevention |
| **Shutdown** | Agent process terminates | Cleanup, capability revocation, final audit entry |

Each hook runs synchronously -- the agent is blocked until the hook completes. Hooks can approve (proceed), modify (alter the payload and proceed), or reject (abort the action and return an error to the agent). Hook failures are themselves events in the observability pipeline.

### Memory Poisoning Defense

Episodic memories from agent execution -- what worked, what failed, what patterns were discovered -- are valuable for improving future agent performance. They are also a vector for corruption: if a confused or manipulated agent writes incorrect "lessons learned" into the memory system, those lessons poison all future interactions that retrieve them.

Defense layers:

1. **Gate validation**: memories generated during a plan that fails its gates are quarantined. They are not promoted to the active memory store until the plan passes or an operator explicitly approves them.
2. **Consistency checking**: new memories are checked against existing memories for contradiction. A memory claiming "always use `unwrap()` in production code" contradicts established patterns and is flagged for review.
3. **Provenance tracking**: every memory entry records which plan, which agent, which model, and which gate results produced it. When a memory is later found to be incorrect, its provenance enables targeted cleanup -- delete all memories from the same failing plan, not a blanket purge.
4. **Decay and validation**: memories that are retrieved but consistently lead to gate failures have their confidence scores reduced. Memories that are retrieved and consistently lead to gate passes have their confidence reinforced. Natural selection on memories.

### Audit Chain

All safety-relevant actions are recorded in a SHA-256 hash-linked append-only log. Each entry contains:

```rust
struct AuditEntry {
    sequence: u64,              // monotonic, gapless
    timestamp: DateTime<Utc>,   // wall clock
    prev_hash: [u8; 32],        // SHA-256 of previous entry
    actor: ActorId,             // agent, conductor, or operator
    action: AuditAction,        // typed enum of safety-relevant actions
    capability_used: Option<CapabilityId>,
    outcome: ActionOutcome,     // approved, denied, error
    payload_hash: [u8; 32],     // SHA-256 of the action's data
}
```

The hash chain provides tamper detection: `verify()` walks the chain and confirms each entry's `prev_hash` matches the computed hash of the preceding entry. Any modification to a historical entry breaks the chain at that point.

Periodic checkpoints anchor the chain to external immutable storage. The checkpoint records the chain head hash, the sequence number, and a timestamp. On-chain anchoring (writing the checkpoint hash to a blockchain or similar immutable ledger) provides external attestation that the audit log existed in a specific state at a specific time. This matters for compliance and for detecting sophisticated attacks that might attempt to rewrite the entire log consistently.
