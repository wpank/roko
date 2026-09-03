# PRD Workflow Example

This guide covers the end-to-end workflow from capturing an idea through a
PRD (Product Requirements Document) to generating and executing an
implementation plan.

## Overview

The PRD lifecycle has four stages:

```
idea  -->  draft  -->  published  -->  plan
```

Each stage is a CLI command. The final output is a `tasks.toml` that the
runner-v2 engine can execute.

## Prerequisites

```bash
# Build roko
cargo build -p roko-cli

# Initialize workspace
cargo run -p roko-cli -- init

# Verify provider is configured (needed for draft and plan generation)
cargo run -p roko-cli -- config providers list
```

## 1. Capture an Idea

Ideas are free-form text descriptions of work items. They are appended to
`.roko/prd/ideas.md`.

```bash
cargo run -p roko-cli -- prd idea "Add a retry counter to the gate pipeline \
that tracks how many times each rung has been retried per task, and expose \
it in the efficiency events"
```

Expected output:

```
Idea captured: "Add a retry counter to the gate pipeline..."
Saved to .roko/prd/ideas.md
```

You can capture multiple ideas. They serve as a backlog:

```bash
cargo run -p roko-cli -- prd idea "Wire dream consolidation to run after plan completion"
cargo run -p roko-cli -- prd idea "Add per-model cost tracking in the cascade router"
```

List captured ideas and PRDs:

```bash
cargo run -p roko-cli -- prd list
```

## 2. Draft a PRD

Drafting uses an LLM agent to expand an idea into a structured PRD document.
The agent reads your codebase context and produces a document with sections
for motivation, requirements, acceptance criteria, and risks.

```bash
cargo run -p roko-cli -- prd draft new "gate-retry-counter"
```

This creates `.roko/prd/drafts/gate-retry-counter.md` with agent-generated
content. The slug (`gate-retry-counter`) becomes the PRD's identifier
throughout its lifecycle.

Expected output:

```
Drafting PRD: gate-retry-counter
Agent generating draft...
Draft saved to .roko/prd/drafts/gate-retry-counter.md
```

### Edit a draft

If the generated draft needs revision:

```bash
cargo run -p roko-cli -- prd draft edit "gate-retry-counter"
```

This re-runs the agent with editing instructions.

### List drafts

```bash
cargo run -p roko-cli -- prd draft list
```

## 3. Enhance with Research (Optional)

Before promoting a draft, you can enhance it with research context:

```bash
# Research the topic
cargo run -p roko-cli -- research topic "gate pipeline retry strategies"

# Enhance the PRD with research findings
cargo run -p roko-cli -- research enhance-prd gate-retry-counter
```

The research agent uses Perplexity (if configured) or other search-capable
providers to gather context that gets folded into the PRD.

## 4. Promote to Published

When the draft is ready, promote it to published status:

```bash
cargo run -p roko-cli -- prd draft promote "gate-retry-counter"
```

This moves the PRD from `.roko/prd/drafts/` to `.roko/prd/published/`.

Expected output:

```
Promoted gate-retry-counter to published
Saved to .roko/prd/published/gate-retry-counter.md
```

If `prd.auto_plan` is enabled in your config, promoting a PRD automatically
triggers plan generation (step 5).

## 5. Generate a Plan from the PRD

Generate a `tasks.toml` implementation plan from the published PRD:

```bash
cargo run -p roko-cli -- prd plan gate-retry-counter
```

This dispatches an agent that reads the PRD and produces a plan directory at
`.roko/prd/plans/gate-retry-counter/tasks.toml`.

Expected output:

```
Generating plan from PRD: gate-retry-counter
Agent reading PRD and codebase context...
Plan generated: .roko/prd/plans/gate-retry-counter/tasks.toml
  Tasks: 4
  Estimated time: 25 minutes
```

### Validate the generated plan

```bash
cargo run -p roko-cli -- plan validate .roko/prd/plans/gate-retry-counter
```

### Review the generated tasks

```bash
cargo run -p roko-cli -- plan show .roko/prd/plans/gate-retry-counter
```

## 6. Execute the Plan

Run the generated plan:

```bash
cargo run -p roko-cli -- plan run .roko/prd/plans/gate-retry-counter --engine runner-v2
```

See the [plan-execution](../plan-execution/README.md) example for detailed
execution guidance.

## 7. Check PRD Coverage

Get a coverage report across all PRDs:

```bash
cargo run -p roko-cli -- prd status
```

This shows which PRDs have plans, which plans have been executed, and
overall completion status.

## 8. Consolidate PRDs

Scan all PRDs for gaps and duplicates:

```bash
cargo run -p roko-cli -- prd consolidate
```

## Example PRD Document

Here is what a typical published PRD looks like in
`.roko/prd/published/gate-retry-counter.md`:

```markdown
# Gate Retry Counter

## Motivation

The gate pipeline currently retries failed rungs but does not track how many
retries occurred per task. This makes it difficult to identify tasks that
consistently require multiple attempts and to tune the adaptive gate
thresholds effectively.

## Requirements

1. Add a `retry_count` field to each gate rung execution record
2. Persist retry counts in the efficiency event JSONL log
3. Expose retry statistics in the `GET /api/gates/summary` endpoint
4. Update the TUI gates view to display retry counts

## Acceptance Criteria

- [ ] Each gate rung execution records the attempt number (1-based)
- [ ] `AgentEfficiencyEvent` includes a `gate_retry_counts` map
- [ ] The HTTP `/api/gates/summary` response includes `avg_retries` per gate
- [ ] The TUI F2 (Plans) tab shows retry counts next to gate results
- [ ] Existing tests continue to pass with the new fields

## Risks

- Adding fields to `AgentEfficiencyEvent` may break existing JSONL consumers
  if they use strict deserialization. Mitigation: use `#[serde(default)]`.
- Retry counts may be misleading for rungs that are skipped by adaptive
  thresholds. Mitigation: distinguish "skipped" from "not retried."

## Scope

This PRD covers only the tracking and display of retry counts. Changes to
retry strategy or threshold tuning are out of scope.
```

## Full Workflow Summary

```bash
# 1. Capture the idea
cargo run -p roko-cli -- prd idea "Add gate retry counter tracking"

# 2. Generate a draft PRD
cargo run -p roko-cli -- prd draft new "gate-retry-counter"

# 3. (Optional) Research for context
cargo run -p roko-cli -- research enhance-prd gate-retry-counter

# 4. Promote to published
cargo run -p roko-cli -- prd draft promote "gate-retry-counter"

# 5. Generate implementation plan
cargo run -p roko-cli -- prd plan gate-retry-counter

# 6. Validate the plan
cargo run -p roko-cli -- plan validate .roko/prd/plans/gate-retry-counter

# 7. Execute the plan
cargo run -p roko-cli -- plan run .roko/prd/plans/gate-retry-counter --engine runner-v2

# 8. Monitor progress
cargo run -p roko-cli -- dashboard
```

## HTTP API Equivalents

If `roko serve` is running on `:6677`, the PRD workflow is also available via
HTTP:

```bash
# List all PRDs
curl -s http://localhost:6677/api/prds | jq .

# Capture an idea
curl -s -X POST http://localhost:6677/api/prds/ideas \
  -H 'Content-Type: application/json' \
  -d '{"text": "Add gate retry counter tracking"}' | jq .

# Read a specific PRD
curl -s http://localhost:6677/api/prds/gate-retry-counter | jq .

# Draft a PRD
curl -s -X POST http://localhost:6677/api/prds/gate-retry-counter/draft | jq .

# Promote a PRD
curl -s -X POST http://localhost:6677/api/prds/gate-retry-counter/promote | jq .

# Generate a plan from a PRD
curl -s -X POST http://localhost:6677/api/prds/gate-retry-counter/plan | jq .

# PRD coverage status
curl -s http://localhost:6677/api/prds/status | jq .
```
