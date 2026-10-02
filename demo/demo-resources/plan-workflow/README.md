# Plan Workflow Demo

Demonstrate the plan-first loop: prompt → plan → review → run.

## The flow

```
roko plan generate "<prompt>"     # an agent writes plans/<slug>/
    ↓
review and edit plans/<slug>/tasks.toml and plan.md
    ↓
roko plan validate plans/<slug>
    ↓
roko run plans/<slug>             # agents run the tasks, gates check them
```

## Scripts

- `demo-plan-cli.sh` — CLI-only flow (no dashboard needed)
- `demo-plan-api.sh` — HTTP API flow (the endpoints the portal uses)

Both take an optional prompt; the API script also takes the serve URL first.
Each asks before it runs the plan, since running dispatches agents.

## CLI flow

```bash
# Write a plan from a prompt (runs nothing)
roko plan generate "Wire knowledge into matchmaking"

# Review and edit it, then lint it
$EDITOR plans/wire-knowledge-into-matchmaking/tasks.toml
roko plan validate plans/wire-knowledge-into-matchmaking

# Run it
roko run plans/wire-knowledge-into-matchmaking
```

`roko run --plan "Wire knowledge into matchmaking"` does the same in one
command: it writes the plan, shows it, and runs it once you confirm.

## API flow

```bash
KEY=$(cat .roko/runtime/serve.token)   # the launch token roko serve writes, or an API key

# Write a plan: 202 {"id": "<operation id>", "plan_id": "<slug>"}
curl -s -X POST http://127.0.0.1:6677/api/plans/generate -H "X-Api-Key: $KEY" \
  -H 'Content-Type: application/json' -d '{"prompt": "Wire knowledge into matchmaking"}'

# Poll until "status" is "completed"
curl -s http://127.0.0.1:6677/api/operations/<operation-id> -H "X-Api-Key: $KEY"

# Read the plan's tasks.toml; PUT the same path with {"toml": "..."} to save an edit
curl -s http://127.0.0.1:6677/api/plans/<slug>/source -H "X-Api-Key: $KEY"

# Run it
curl -s -X POST http://127.0.0.1:6677/api/plans/<slug>/execute -H "X-Api-Key: $KEY"
```

## Dashboard flow

1. Open the portal link `roko serve` prints
2. Type the prompt; the generated plan appears
3. Edit its `tasks.toml` as text; the server validates each save
4. Press Run, and watch the tasks progress

In the terminal, `roko dashboard` shows the same run: **F2 Plans** has the task
tree, **F3 Agents** the live agent output.

## What gets created

```
plans/
└── wire-knowledge-into-matchmaking/
    ├── tasks.toml                          # Generated tasks
    └── plan.md                             # The plan's narrative
.roko/
├── state/graph/wire-knowledge-into-matchmaking/  # Checkpoint, activity log, costs
└── episodes.jsonl                          # Agent turns recorded
```

## Dashboard tabs that light up

- **Plans** — Shows the generated plan with its task tree and progress
- **Agents** — Live agent output for each running task
- **Learning** — Records efficiency events, gate results
