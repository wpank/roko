# Web, MCP, Packaging, And DX Checklist

## Scope

Use this file for serve/API parity, MCP distribution, package/marketplace CLI, and shell/deployment DX improvements.

## Implementation checklist

- [ ] Treat `roko-serve` as the backend system of record.
  - expose status, plans, agents, projections, providers, and websocket streams consistently;
  - fill route gaps before adding frontend-only workarounds.
- [ ] MCP work should extend the existing server story.
  - discovery;
  - auto-registration of capabilities;
  - tests across at least one MCP-compatible client path.
- [ ] Package commands should be staged against a real registry or manifest format.
  - install
  - remove
  - search
  - publish
  - market browser
- [ ] CLI DX items should prefer existing platform hooks.
  - shell init;
  - NO_COLOR/CLICOLOR;
  - command timing;
  - richer `--version`;
  - shell completions.
- [ ] Web-surface work outside this repo must point to `/Users/will/dev/nunchi/nunchi-dashboard`, but backend route/schema work belongs here.

## Relevant current files

- `crates/roko-serve/src/routes/mod.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-cli/src/config_cmd.rs`
- `crates/roko-cli/src/deployment.rs`

## Verification checklist

- [ ] Any new API surface has tests in `crates/roko-serve/tests/` or route-level unit tests.
- [ ] MCP integration works against a concrete sample flow.
- [ ] Shell completions and color behavior work without breaking existing output contracts.

## Acceptance criteria

- Backend parity comes before frontend-specific polish.
- MCP/package features are grounded in real manifests and routes.
- DX work improves operator ergonomics without creating new fragmented workflows.
