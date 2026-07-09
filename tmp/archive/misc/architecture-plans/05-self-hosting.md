# Layer 5: Self-hosting loop

**Goal**: Use the dashboard and roko to implement the remaining architecture. Agents build roko, monitored through the plan execution UI.

**Depends on**: Plans 02 (agent creation), 03 (agent streaming), 04 (plan execution UI)

**Effort**: L (3-5 days for the initial loop, then ongoing)

---

## Current state

Roko already self-hosts at the CLI level. The workflow described in CLAUDE.md works:

```bash
roko prd idea "Wire SystemPromptBuilder into orchestrate.rs"
roko prd draft new "system-prompt-wiring"
roko research enhance-prd system-prompt-wiring
roko prd plan system-prompt-wiring
roko plan run plans/
roko dashboard
```

What does not exist: running this loop through the dashboard, with persistent agents on Railway, and with a human review step before merging code.

### What exists

| Piece | Status | Where |
|-------|--------|-------|
| Plan generation from PRDs | Works | `roko prd plan <slug>` |
| Plan execution with agents | Works | `roko plan run <dir>` |
| Gate pipeline (compile, test, clippy, diff) | Works | orchestrate.rs per-task |
| Session persistence + resume | Works | `.roko/state/executor.json`, `--resume` |
| HTTP plan endpoints | Works | `roko-serve/src/routes/plans.rs` |
| Dashboard plan UI | Layer 4 | Not yet built |
| Persistent agents on Railway | Not built | Railway deployment exists but no persistent coding agents |
| Review/approve workflow | Not built | No dashboard diff view or approval routes |

---

## Tasks

### 5.1 Generate roko plans from every canonical doc

For every canonical architecture and product doc, create an implementation plan
or a file-level ledger entry that maps the doc to an executable plan task. The
output must be complete enough that no source doc remains "vision only" unless
it is explicitly marked as deferred with an owner, reason, dependency, and
acceptance gate.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/00-INDEX.md` (all 22 architecture docs)
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/18-roadmap.md` (phases 1-12 with task breakdowns)
- `/Users/will/dev/nunchi/roko/roko/docs/INDEX.md` and every markdown file under `/Users/will/dev/nunchi/roko/roko/docs/`
- `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/11-CHECKLIST-IMPLEMENTATION.md` (41 DeFi batches in topological order)
- `/Users/will/dev/nunchi/roko/roko/.roko/plans/` (existing plan directory)
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture-plans/07-docs-parity-closure.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture-plans/08-end-to-end-acceptance.md`

**Target**:
- `.roko/plans/` -- executable plan directories containing `plan.md` and `tasks.toml`
- `.roko/parity/docs-ledger.json` -- file-level coverage ledger for every source doc
- `.roko/parity/source-inventory.json` -- generated inventory of code, docs, contracts, routes, tests, and known stubs

**Method**: Use the dashboard Plan Designer (task 4.7 chat editor), the CLI,
or a deterministic generator. Grouping is allowed, but file-level traceability
is not optional.

```bash
# For each architecture doc:
roko prd draft new "arch-02-agent-runtime"
# Paste the architecture doc content as the PRD body
roko prd plan arch-02-agent-runtime
```

Alternatively, generate plans via `POST /api/plans/generate` with each doc or
doc section as the PRD.

**Coverage rule**:

Every markdown file in these sources must have exactly one of:
- `implemented_by`: plan slug + task id + acceptance test
- `covered_by`: another source doc with identical runtime requirement
- `deferred`: owner + dependency + reason + explicit future acceptance gate
- `reference_only`: allowed only for citation/background docs under `docs/21-references/`

**Implementation**:
- [ ] Generate `source-inventory.json` with counts for: crates, apps, route modules, contracts, tests, docs, known stubs, and TODO/FIXME markers
- [ ] Create architecture plan slugs for every file in `tmp/architecture/*.md`: `arch-01-overview` through `arch-21-tui-and-operations`
- [ ] Create docs parity plan slugs for every top-level docs directory: `doc-00-architecture` through `doc-21-references`
- [ ] Create DeFi plan slugs for every `tmp/defi/gap/*.md` executable batch
- [ ] Populate `docs-ledger.json` with one row per markdown file, including source path, owning plan, owning phase, status, code targets, route targets, tests, and acceptance gate
- [ ] Generate `.roko/plans/` entries for every ledger row whose status is `implemented_by`
- [ ] Verify every task has: source docs, target files, acceptance criteria, dependencies, rollback note, and verification command
- [ ] Record inter-plan dependencies across architecture, older docs, and DeFi batches
- [ ] Fail the generator if any non-reference source doc has no coverage row

**Acceptance criteria**:
- 100% of `tmp/architecture/*.md` files are represented in `docs-ledger.json`
- 100% of `docs/**/*.md` files are represented in `docs-ledger.json`
- 100% of `tmp/defi/gap/*.md` files are represented in `docs-ledger.json`
- 0 ledger rows have status `unknown`, `unmapped`, or blank
- Every non-reference row has either an executable plan task or an explicit deferred gate
- Plans are visible in the dashboard plan list (task 4.1)
- Running `roko parity check --strict` exits 0 and prints no uncovered docs

---

### 5.2 Deploy persistent coding agents on Railway

Set up agents that can implement code changes against the roko repo.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs` (agent CRUD routes)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs` (agent dispatch)
- `/Users/will/dev/nunchi/roko/roko/roko.toml` (provider and agent configuration)
- `/Users/will/dev/nunchi/roko/roko/deploy/` (Railway deployment config)
- `/Users/will/dev/nunchi/roko/roko/railway.toml` (Railway service definition)

**Target**:
- `/Users/will/dev/nunchi/roko/roko/roko.toml` -- agent configuration section
- `/Users/will/dev/nunchi/roko/roko/deploy/` -- Railway service definitions for persistent agents
- Agent environment setup scripts

**API contract**: Uses existing agent routes:

```
POST /api/agents
{
  "name": "coder-01",
  "domain": "coding",
  "model": "claude-sonnet-4-6",
  "mode": "persistent",
  "config": {
    "repo": "nunchi/roko",
    "branch_prefix": "agent/",
    "auto_branch": true
  }
}

Response 201:
{ "id": "agent-abc", "name": "coder-01", "status": "starting" }
```

```
POST /api/agents/{id}/start
Response 200: { "status": "running" }
```

**Implementation**:
- [ ] Define agent profiles in `roko.toml` for coding agents:
  - Profile: `coding`
  - Model: `claude-sonnet-4-6` (default), `claude-opus-4-6` (for complex tasks)
  - MCP config: include code intelligence MCP (`roko-mcp-code`)
  - Gate pipeline: compile -> test -> clippy
  - Budget: configurable daily limit
- [ ] Create a Railway service definition for persistent agents (extends existing `railway.toml`)
- [ ] Configure provider keys via `roko config set-secret` (Anthropic API key)
- [ ] Deploy a test agent: `roko agent create --name coder-01 --domain coding`
- [ ] Verify the agent can: clone the repo, read files, make changes, run `cargo test`
- [ ] Set up git branch conventions: each agent creates `agent/<agent-name>/<task-id>` branches

**Acceptance criteria**:
- An agent deployed on Railway can clone the roko repo
- The agent can make a small code change (e.g., add a comment) and run `cargo test`
- The agent appears in the dashboard agent list (requires Layer 2)
- Agent logs are visible via `roko agent status --name coder-01` or the dashboard

---

### 5.3 Implement review and approve workflow

Before merging agent work, humans review diffs, test results, and gate verdicts through the dashboard.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/events.rs` (execution events including gate results)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` (plan execution routes)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_pipeline.rs` (gate pipeline types)

**Target (backend -- NEW ROUTES)**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` -- add review endpoints

**Target (frontend)**:
- `src/pages/forge/TaskReview.tsx` (in nunchi-dashboard)

**API contract**:

List tasks pending review:
```
GET /api/plans/{id}/reviews

Response 200:
{
  "reviews": [
    {
      "task_id": "t1",
      "status": "pending_review",
      "branch": "agent/coder-01/t1",
      "diff_summary": "+42 -7 across 3 files",
      "gate_results": [
        { "gate": "compile", "passed": true },
        { "gate": "test", "passed": true, "message": "47 tests passed" },
        { "gate": "clippy", "passed": true }
      ],
      "files_changed": ["src/auth.rs", "src/middleware.rs", "tests/auth_test.rs"]
    }
  ]
}
```

Submit review decision:
```
POST /api/plans/{id}/tasks/{task_id}/review
Content-Type: application/json

{
  "decision": "approve",
  "comment": "Looks good, merge it"
}

Response 200:
{ "task_id": "t1", "status": "approved", "merged": true }
```

Decision options: `approve` (merge branch), `reject` (agent retries with feedback), `skip` (mark done without merge).

Get diff for a task:
```
GET /api/plans/{id}/tasks/{task_id}/diff

Response 200:
{
  "task_id": "t1",
  "branch": "agent/coder-01/t1",
  "base": "main",
  "files": [
    {
      "path": "src/auth.rs",
      "status": "modified",
      "additions": 35,
      "deletions": 2,
      "patch": "..."
    }
  ]
}
```

**Implementation (backend)**:
- [ ] Add `GET /api/plans/{id}/reviews` handler: scan plan tasks in `completed` or `pending_review` status, collect gate results and git branch info
- [ ] Add `POST /api/plans/{id}/tasks/{task_id}/review` handler:
  - `approve`: merge the agent's branch into the plan's base branch, mark task as `approved`
  - `reject`: send feedback to the agent (via agent message endpoint or re-queue the task with the rejection comment appended to the prompt), mark task as `needs_rework`
  - `skip`: mark task as `skipped`, no merge
- [ ] Add `GET /api/plans/{id}/tasks/{task_id}/diff` handler: shell out to `git diff main...agent/<name>/<task>` and parse the output into structured file-level diffs
- [ ] Extend `PlanTask` status to include: `pending`, `running`, `pending_review`, `approved`, `rejected`, `skipped`
- [ ] Register new routes

**Implementation (frontend)**:
- [ ] Create `TaskReview.tsx` page showing:
  - Task description and file list
  - Gate results (reuse gate display from task 4.5)
  - Unified diff view (use a diff viewer component or `react-diff-viewer`)
  - Approve / Reject / Skip buttons
- [ ] Reject shows a text input for feedback
- [ ] Approved tasks get a green merge badge
- [ ] Plan execution view (task 4.4) shows review-pending tasks with an amber "Review" button

**Acceptance criteria**:
- Agent completes a task and creates a branch
- Dashboard shows the diff and gate results for that task
- Approving merges the branch
- Rejecting sends feedback and the agent retries
- The plan execution view reflects the review state

---

### 5.4 Execute first architecture plan

Pick a small architecture doc and run it through the full loop: plan, deploy agents, execute, review, merge.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/08-auth.md` (recommended first candidate -- small scope, well-defined)
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/03-extensions.md` (alternative candidate)
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/18-roadmap.md` (Phase 1 remaining: Privy JWKS, scope enforcement, device flow)

**Target**: The architecture feature works end-to-end after agent implementation.

**Implementation**:
- [ ] Select a small architecture doc -- recommend `08-auth.md` Phase 1 remaining items:
  - Privy JWT validation (real JWKS, not structural stub)
  - Scope enforcement at route level
  - Device flow for headless CLI login
- [ ] Create a plan from the doc (via task 5.1 or dashboard)
- [ ] Deploy a coding agent (task 5.2)
- [ ] Execute the plan through the dashboard (task 4.4)
- [ ] Monitor agent progress via the execution view
- [ ] Review each completed task's diff (task 5.3)
- [ ] Approve and merge passing tasks
- [ ] Run `cargo test --workspace` and `cargo clippy --workspace --no-deps -- -D warnings` on the merged result
- [ ] Document friction points: what was slow, what failed, what needed human intervention

**Acceptance criteria**:
- A plan derived from an architecture doc runs to completion with agent execution
- At least one task produces a code change that passes gate validation
- The review workflow (diff -> approve -> merge) works end-to-end
- Post-merge, `cargo test` passes

---

### 5.5 Iterate on the loop

Identify friction points from 5.4 and feed them back into the system.

**Read**:
- Results from task 5.4 (friction log)
- `/Users/will/dev/nunchi/roko/roko/.roko/learn/efficiency.jsonl` (agent efficiency data)
- `/Users/will/dev/nunchi/roko/roko/.roko/episodes.jsonl` (agent episode log)

**Target**: Updated architecture docs, improved plans, tuned agent configuration.

**Implementation**:
- [ ] Review the friction log from 5.4 and categorize issues:
  - Agent failures: wrong model, insufficient context, bad tool usage
  - Gate failures: flaky tests, clippy false positives, overly strict thresholds
  - Workflow issues: slow review cycle, unclear diffs, missing feedback loop
  - Infrastructure issues: Railway timeouts, API rate limits, git conflicts
- [ ] For each category, determine the fix:
  - Agent issues -> update `roko.toml` agent profiles, improve system prompt templates in `roko-compose/src/templates/`
  - Gate issues -> tune thresholds via `roko learn tune gates`
  - Workflow issues -> create PRDs for missing features, add to plan backlog
  - Infrastructure issues -> update deployment config
- [ ] Apply fixes and re-run a second architecture plan to verify improvements
- [ ] Update `CLAUDE.md` "Known blockers" section with any new findings
- [ ] Create follow-up tasks in `.roko/plans/` for recurring issues

**Acceptance criteria**:
- A friction log exists documenting issues from the first run
- At least 3 issues are resolved before the second run
- The second run completes faster or with fewer failures than the first
- Findings are recorded in the project docs

---

## New roko-serve routes summary

| Route | Handler | File |
|-------|---------|------|
| `GET /api/plans/{id}/reviews` | `list_reviews` | `crates/roko-serve/src/routes/plans.rs` |
| `POST /api/plans/{id}/tasks/{task_id}/review` | `submit_review` | `crates/roko-serve/src/routes/plans.rs` |
| `GET /api/plans/{id}/tasks/{task_id}/diff` | `task_diff` | `crates/roko-serve/src/routes/plans.rs` |

---

## Dependency graph

```
5.1 (generate plans) ────── can start immediately with CLI
          │
5.2 (deploy agents) ─────── can run in parallel with 5.1
          │
          ▼
5.3 (review workflow) ───── requires Layer 2 (agent control) + Layer 4 (plan execution UI)
          │
          ▼
5.4 (first run) ──────────── requires 5.1 + 5.2 + 5.3
          │
          ▼
5.5 (iterate) ───────────── requires 5.4
```

Tasks 5.1 and 5.2 are independent and can start before Layer 4 is complete. Task 5.3 (review workflow) is the gating dependency -- it requires both backend routes and frontend components.
