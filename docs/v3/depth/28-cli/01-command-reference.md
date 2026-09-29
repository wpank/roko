# 28.01 -- Command Reference

> Depth file for [28-CLI.md](../../28-CLI.md).

---

## Top-Level Flags

Every subcommand inherits these global flags:

| Flag | Type | Default | Description |
|---|---|---|---|
| `--repo` | `PathBuf` | cwd | Workspace root |
| `--model` | `String` | config | Override default model |
| `--role` | `AgentRole` | config | Override agent role |
| `--effort` | `Effort` | `medium` | Reasoning effort: low/medium/high/max |
| `--headless` | `bool` | `false` | Non-interactive mode |
| `--json` | `bool` | `false` | JSON output format |
| `--quiet` | `bool` | `false` | Suppress non-essential output |
| `--log-format` | `String` | `pretty` | Tracing format: pretty/compact/json |
| `--resume` | `bool` | `false` | Resume last plan execution |
| `--no-replan` | `bool` | `false` | Disable automatic gate-failure replanning |
| `--force-backend` | `String` | - | Override provider selection |

Positional `[prompt]` enables one-shot mode without a subcommand.

## Command Tree

### Core Workflow

```
roko init                        Create .roko/ directory and roko.toml
roko setup                       Interactive setup wizard
roko run "<prompt>"              Single prompt through graph templates
roko do "<prompt>"               Execute task via agent dispatch
  --complexity <band>            trivial/simple/medium/complex
  --sandbox <level>              Override sandbox level
roko develop "<prompt>"          Plan-first development
roko show [subject]              Inspect workspace state
  costs|agents|knowledge|plans|learning|history
roko status                      Query signals, report counts
roko doctor [subject]            Diagnose workspace
  disk|network|clean
roko diagnose <plan-id>          Diagnose plan failure (JSON output)
roko resume [run-id]             Resume plan from last checkpoint
roko think "<question>"          Research without executing
roko note "<text>"               Capture quick note (no LLM)
  --tags <tags>                  Comma-separated tags
roko chat                        Interactive chat REPL
roko vision-loop <file>          Iterative vision-guided UI refinement
roko history [id]                List or show past sessions
roko cache status|prune          Cache lifecycle
  prune --apply                  Actually delete (default: dry-run)
  prune --target-budget-gb N
  prune --evidence-budget-mb N
roko github status               GitHub config and CI state
```

### Planning and PRDs

```
roko plan list                   List plans
roko plan show <dir>             Show plan details
roko plan create                 Create plan interactively
roko plan run <dir>              Execute via Graph engine
  --engine legacy                Use Runner-v2 fallback
  --resume-plan                  Resume from checkpoint
roko plan generate <prompt>      Generate plan from prompt
roko plan regenerate <dir>       Regenerate plan tasks
roko plan index                  Rebuild plans/INDEX.md
roko plan pause <dir>            Pause running plan
roko plan resume <dir>           Resume paused plan
roko plan cancel <dir>           Cancel running plan
roko plan retry <dir>            Retry failed tasks
roko plan status <dir>           Show execution status
roko plan queue show|validate|init  Queue manifest operations
roko plan validate <dir>         Lint tasks.toml without executing

roko prd idea "<text>"           Capture work item idea
roko prd list                    List PRDs
roko prd status                  PRD coverage report
roko prd draft new|edit|promote|list  Draft lifecycle
roko prd plan <slug>             Generate plan from PRD
roko prd consolidate             Scan for gaps and duplicates

roko backlog import|list|audit   Backlog spec management
```

### Agents

```
roko agent create --name X --domain Y  Create agent from manifest
roko agent delete --name X             Delete with 8-step shutdown
roko agent start --name X              Start long-running agent
roko agent stop --name X               Stop running agent
roko agent list                        List agents with status
roko agent status --name X             Detailed agent health
roko agent serve                       Start per-agent HTTP sidecar
roko agent chat --agent X              Interactive chat REPL
```

### Research

```
roko research topic "<topic>"          Deep research with citations
roko research search "<query>"         Direct web search (Perplexity)
roko research enhance-prd <slug>       Enhance PRD with research
roko research enhance-plan <dir>       Enhance plan with research
roko research enhance-tasks <dir>      Enhance tasks with research
roko research analyze                  Analyze execution data
roko research list                     List research artifacts
```

### Knowledge

```
roko knowledge query "<topic>"         Search durable knowledge
roko knowledge stats                   Store statistics
roko knowledge gc                      Garbage collection
roko knowledge backup                  Backup with genomic bottleneck
roko knowledge restore                 Restore with decay
roko knowledge sync <peer>             Mesh knowledge sync
roko knowledge dream run|report|schedule  Dream consolidation
roko knowledge dream journal|archive   Dream journal/archive
roko knowledge export                  Export knowledge entries
roko knowledge import                  Import knowledge entries
roko knowledge backfill-hdc            Backfill HDC fingerprints
roko knowledge custody list|show|verify  Custody audit chain
roko knowledge archive                 Cold storage archival
```

### Learning and Feedback

```
roko learn all                         Full learning state overview
roko learn router                      Cascade router state
roko learn experiments                 Prompt experiments
roko learn efficiency                  Efficiency metrics
roko learn episodes                    Episode history
roko learn reflexes                    T0 reflex rules
roko learn gates                       Adaptive gate thresholds
roko learn knowledge-stats             Knowledge entry counts
roko learn inspect gates|routing|budget  Read-only subsystem inspection
```

### Jobs and Marketplace

```
roko job list                          List marketplace jobs
roko job create                        Create job
roko job show <id>                     Show job detail
roko job execute <id>                  Execute job
roko job cancel <id>                   Cancel job
roko job match                         Find matching jobs
```

### Configuration

```
roko config init|show|path|edit|set    Core config management
roko config doctor                     Config health check
roko config validate|migrate           Schema validation, migration
roko config set-secret|check-secrets   Secret management
roko config export                     Export as env vars
roko config env                        List recognized env vars
roko config providers list|health|test  Provider inspection
roko config providers available|discover|add|catalog|validate
roko config models list|route          Model inspection
roko config subscriptions list|add|remove
roko config events                     Configured event sources
roko config experiments                Model A/B experiments
roko config plugins list|install|remove|audit|publish
roko config secrets set|get|list|rotate
roko config mcp list|test|add          MCP server configuration
roko config preset gates|routing|budget|model  Apply presets
  --dry-run                            Preview only
  --yes                                Skip confirmation
```

### Server and Deployment

```
roko serve                             HTTP control plane on :6677
roko acp                               ACP server for editor integration
roko daemon start|stop|status|logs|install
roko deploy railway|fly|docker         Cloud deployment
roko worker                            Run as deployed worker
roko login [url]                       Authenticate with server
roko logout                            Remove credentials
roko whoami                            Show auth status
```

### Graph, Feeds, Recipes, Triggers

```
roko graph run|validate|inspect        Graph execution
roko feed list|status|start|stop       Feed management
roko recipe list|show|validate|run     Recipe evaluation
roko trigger list|show|create|fire     Trigger bindings
```

### Utilities

```
roko dashboard                         Interactive TUI (F1-F10)
roko replay <hash>                     Walk signal DAG
roko inject <session> <payload>        Signal injection
roko index build|rebuild|search|stats  Code intelligence index
roko run-index repair                  Rebuild derived indexes
roko bench demo|swe                    Benchmark evaluations
roko new <type> <name>                 Scaffold boilerplate
roko explain <topic>                   Concept explainer (3 depths)
roko completions <shell>               Shell completion scripts
```

## Exit Codes

| Code | Constant | Meaning |
|---|---|---|
| 0 | `EXIT_SUCCESS` | Success |
| 1 | `EXIT_FAILURE` | General failure |
| 2 | `EXIT_AGENT_FAILURE` | Agent dispatch failure |
| 3 | `EXIT_SYSTEM_ERROR` | System-level error |

## Source

- `crates/roko-cli/src/main.rs` -- Top-level CLI definition
- `crates/roko-cli/src/commands/` -- Subcommand implementations
