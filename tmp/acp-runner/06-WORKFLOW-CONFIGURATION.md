# Workflow Configuration

## How Users Configure Pipelines

### 1. Per-Session (ACP Config Options)

The simplest configuration: dropdowns in the editor's agent panel.

```
Model:      [sonnet ▼]
Effort:     [medium ▼]
Workflow:   [standard ▼]
Review:     [quick ▼]
Gates:      [clippy: on] [tests: on]
```

These map directly to session config state. Changed via `session/config/update`.

### 2. Per-Workspace (roko.toml)

```toml
[workflow]
default = "standard"                    # express | standard | full | auto
max_iterations = 2
max_parallel_tasks = 3
timeout_minutes = 45

[workflow.express]
gates = ["compile"]                     # minimal gates
review = false
model_implementer = "haiku"

[workflow.standard]
gates = ["compile", "test"]
review = "quick"                        # single reviewer
model_implementer = "sonnet"
model_reviewer = "sonnet"

[workflow.full]
gates = ["compile", "test", "clippy"]
review = "thorough"                     # architect + auditor + scribe
model_strategist = "opus"
model_implementer = "sonnet"
model_architect = "opus"
model_auditor = "opus"
model_scribe = "sonnet"

[workflow.research]
model = "perplexity"                    # or sonnet with web search
output = "report"                       # what gets produced
```

### 3. Per-Plan (tasks.toml metadata)

From bardo's proven format:
```toml
[meta]
plan = "feature-auth"
max_parallel = 3
estimated_total_minutes = 90
preferred_workflow = "full"              # override workspace default

[[task]]
id = "T1"
title = "Add auth middleware"
complexity_band = "standard"            # determines pipeline
preferred_model = "sonnet"
files = ["src/middleware/auth.rs", "src/routes/mod.rs"]
depends_on = []
acceptance = ["middleware compiles", "basic test passes"]
```

### 4. Custom Workflow Templates

Users can define custom pipeline steps:

```toml
# .roko/workflows/my-pipeline.toml
[workflow]
name = "my-pipeline"
description = "Research, plan, then implement with thorough review"

[[step]]
role = "researcher"
phase = "research"
model = "perplexity"
output = "research_context"

[[step]]
role = "strategist"
phase = "plan"
model = "opus"
input_from = "research_context"
output = "brief"

[[step]]
role = "implementer"
phase = "implement"
model = "sonnet"
input_from = "brief"
gate = ["compile", "test", "clippy"]

[[step]]
role = "architect"
phase = "review"
model = "opus"
verdict_required = true

[[step]]
role = "auditor"
phase = "review"
model = "opus"
parallel_with = "architect"
verdict_required = true

[[step]]
role = "scribe"
phase = "document"
model = "sonnet"
only_if = "files_changed > 5"
```

### 5. Trigger Configuration

```toml
# .roko/triggers/pr-review.toml
[trigger]
name = "auto-review-prs"
enabled = true

[trigger.source]
kind = "github"
event = "pull_request.opened"
filter = "base == 'main'"

[trigger.workflow]
template = "review-only"
input.diff = "{{ event.pull_request.diff_url }}"
input.title = "{{ event.pull_request.title }}"

[trigger.output]
action = "github_comment"
target = "{{ event.pull_request.number }}"
```

```toml
# .roko/triggers/file-watch.toml
[trigger]
name = "auto-test-on-save"
enabled = true

[trigger.source]
kind = "file_watch"
patterns = ["src/**/*.rs"]
debounce_ms = 2000

[trigger.workflow]
template = "express"
input.prompt = "Run tests on changed files"
```

## Role-Based Presets (From Bardo)

### Quality Preset
```toml
[preset.quality]
workflow = "full"
model_strategist = "opus"
model_implementer = "opus"
model_reviewer = "opus"
max_iterations = 3
gates = ["compile", "test", "clippy", "fmt"]
review = "thorough"
```

### Balanced Preset (Default)
```toml
[preset.balanced]
workflow = "standard"
model_implementer = "sonnet"
model_reviewer = "sonnet"
max_iterations = 2
gates = ["compile", "test"]
review = "quick"
```

### Speed Preset
```toml
[preset.speed]
workflow = "express"
model_implementer = "haiku"
max_iterations = 1
gates = ["compile"]
review = false
```

### Cost Preset
```toml
[preset.cost]
workflow = "auto"              # use cheapest pipeline that works
model_default = "haiku"
escalate_on_failure = true     # upgrade model if cheap one fails
max_iterations = 2
gates = ["compile"]
review = false
```

## Runtime Override via Slash Commands

```
/express fix the typo in main.rs       → express pipeline, haiku
/full implement the auth system         → full pipeline, opus strategist
/workflow run research                   → research pipeline
/workflow run my-pipeline                → custom workflow
```

## Workflow Discovery (Shown in Editor)

When the user types `/workflow`, ACP returns available workflows:
1. Builtin: express, standard, full, research, review-only, documentation
2. Custom: anything in `.roko/workflows/*.toml`
3. Plan-based: any plans in `plans/` or `.roko/plans/`

These appear as slash command completions and in the config dropdown.

## How Config Changes Affect In-Flight Workflows

- **Model change**: Takes effect on next agent spawn (not current)
- **Workflow change**: Takes effect on next prompt (not current run)
- **Gate toggle**: Takes effect on next gate check
- **Review toggle**: Takes effect on next review phase
- **Cancel**: Immediate, kills in-flight agents

## Interaction with Conversation History

Workflow runs produce conversation context:
- Strategist brief → stored as assistant turn (tagged `[Strategist]`)
- Gate results → stored as system context
- Review findings → stored as assistant turn (tagged `[Reviewer]`)
- Final result → stored as assistant turn

This means follow-up prompts in the same session have full context of what the workflow did.
