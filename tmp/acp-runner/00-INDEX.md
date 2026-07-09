# ACP Runner: Multi-Agent Workflow Orchestration

## What This Is

Design docs for enabling **multi-agent workflow execution and tracking** through ACP-connected editors (Zed, Cursor). The core insight: ACP sessions should be able to not just chat with a single agent, but orchestrate entire pipelines — strategist → implementer → reviewer → auditor — with live progress, state tracking, and configurable workflow templates.

## Documents

| Doc | What |
|-----|------|
| [01-SOURCES.md](01-SOURCES.md) | Source materials and where concepts come from |
| [02-WORKFLOW-PATTERNS.md](02-WORKFLOW-PATTERNS.md) | Multi-agent workflow patterns from bardo/mori |
| [03-AGENT-ROLES.md](03-AGENT-ROLES.md) | Role definitions, capabilities, tool restrictions |
| [04-PIPELINE-STATE-MACHINE.md](04-PIPELINE-STATE-MACHINE.md) | How workflows chain agents with state tracking |
| [05-ACP-RUNNER-PROTOCOL.md](05-ACP-RUNNER-PROTOCOL.md) | ACP protocol extensions for workflow execution |
| [06-WORKFLOW-CONFIGURATION.md](06-WORKFLOW-CONFIGURATION.md) | How users configure and customize pipelines |
| [07-PROGRESS-TRACKING.md](07-PROGRESS-TRACKING.md) | Live status, plan updates, session persistence |
| [08-IMPLEMENTATION-PLAN.md](08-IMPLEMENTATION-PLAN.md) | What to build, in what order |

## Key Concept

Today ACP does: `user prompt → single agent response`

We want: `user prompt → workflow pipeline → multiple agents → live progress → verified result`

The user can:
1. Type a prompt and have it routed through the full pipeline (strategist → impl → gate → review)
2. Configure which pipeline to use (express, standard, complex, custom)
3. See live progress as agents work (plan updates, tool calls, gate results)
4. Resume interrupted workflows
5. Set up triggers (file watch, cron, webhook) that fire workflows automatically
