# Workspace Topology

Primary checkout:

- `/Users/will/dev/nunchi/roko/roko`

Related dashboard docs:

- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd`

Important Roko documentation roots:

- `docs/`
- `tmp/04-21-26/PRDs/`
- `tmp/architecture/`
- `tmp/architecture-plans/`

Runner root:

- `tmp/roko-trustworthy/`

## Core Crates

- `crates/roko-core`: core signal, traits, config-adjacent contracts, metrics.
- `crates/roko-runtime`: async runtime, event bus, process supervision, cancellation.
- `crates/roko-agent`: provider backends, agent trait, role/tool behavior.
- `crates/roko-orchestrator`: plan orchestration and multi-agent execution.
- `crates/roko-compose`: prompt/context composition surfaces.
- `crates/roko-gate`: compile/test/lint/review gates and verification rungs.
- `crates/roko-learn`: learning, policy, outcomes, experiments.
- `crates/roko-neuro`: memory/neuro/HDC-adjacent intelligence surfaces.
- `crates/roko-conductor`: reactive intelligence, diagnosis, intervention policies.
- `crates/roko-serve`: HTTP server/API and job surfaces.
- `crates/roko-agent-server`: per-agent HTTP server and projections.
- `crates/roko-cli`: CLI, TUI, plan commands, self-hosting entry points.

## Support Crates

- `crates/roko-fs`
- `crates/roko-std`
- `crates/roko-chain`
- `crates/roko-plugin`
- `crates/roko-index`
- `crates/roko-mcp-*`
- `crates/roko-lang-*`

## Existing Runner References

Use these only as formatting and orchestration references:

- `tmp/docs-parity2/`
- `tmp/refinement-audit-runner/`
- `tmp/refinements-runner/`
- `tmp/tui-parity/`
- `tmp/ux-followup-runner/`

Do not copy their Mori-specific assumptions into Roko. This runner is for generalizing Roko roles, prompts, context, and self-hosting execution.
