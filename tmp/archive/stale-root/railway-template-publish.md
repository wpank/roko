# Railway Template Publish Fields

Copy everything between the ``` blocks for each template.

---

## 1. Roko (Control Plane)

**Short Description:** `Self-building agent orchestration control plane`

**Category:** `Other`

**Template Overview:**

```
# Deploy and Host Roko

Roko is an orchestration control plane for AI agents that build themselves. It exposes ~85 REST API routes, WebSocket event streaming, and a learning feedback loop. Deploy this template to get a fully functional Roko instance that reads PRDs, generates plans, dispatches agents, validates with gates, and learns from results.

## About Hosting

Roko runs as a single container serving HTTP on the configured PORT (default 3000). It persists learning state, episodes, and plans to `/workspace/.roko`. Attach a volume at that path for durability across redeploys. The health endpoint at `GET /api/health` returns status, version, uptime, active plans/agents/runs, and provider health.

### Environment Variables

- `PORT` (default: 3000) — HTTP listen port (Railway sets this automatically)
- `RUST_LOG` (default: info) — log verbosity (debug, info, warn, error)

## Common Use Cases

- Orchestrate multi-agent coding workflows with automatic plan generation and gate validation
- Serve as the API backend for the Nunchi dashboard (connect via `VITE_ROKO_API_URL`)
- Run self-improving AI agent pipelines where cascade routing, prompt experiments, and adaptive gate thresholds persist across runs
- Generate implementation plans from PRDs and execute them with coordinated agent fleets

## Dependencies for Roko Hosting

- No external service dependencies — Roko runs fully standalone
- Optionally connect a Mirage instance for on-chain agent coordination
- Optionally connect Roko Workers for distributed task execution

### Deployment Dependencies

- Volume mounted at `/workspace/.roko` for persistent state (learning data, episodes, plans, knowledge store)
- No databases or external caches required — all state is file-based

## Why Deploy

- **Self-hosting agents**: Run your own agent orchestration platform without depending on third-party SaaS
- **Dashboard backend**: Powers the Nunchi dashboard with real-time agent status, plan progress, and learning analytics
- **Learning loop**: Agents improve over time — the system learns which models work best for which tasks, tunes gate thresholds, and runs prompt A/B experiments automatically
- **Full API surface**: ~85 REST endpoints cover agents, plans, PRDs, gates, episodes, signals, knowledge, learning, inference, and more
```

---

## 2. Mirage-RS

**Short Description:** `Local EVM fork with AI agent relay and chain APIs`

**Category:** `Other`

**Template Overview:**

```
# Deploy and Host Mirage

Mirage is a local EVM fork node with a built-in AI agent relay and chain extension APIs. It simulates a blockchain environment where AI agents can deploy contracts, post insights, deposit pheromones, and coordinate via on-chain stigmergy — all without real gas costs.

## About Hosting

Mirage runs as a single container serving JSON-RPC and REST APIs on port 8545. An internal agent relay runs on loopback and is exposed via same-origin `/relay/*` routes. Chain state is persisted to `/workspace/.roko/state` via periodic snapshots. The health endpoint at `GET /relay/health` returns "ok" when ready.

### Environment Variables

- `PORT` (default: 8545) — HTTP/JSON-RPC listen port
- `MIRAGE_BLOCK_INTERVAL_MS` (default: 1000) — block production speed in milliseconds
- `MIRAGE_SNAPSHOT_INTERVAL_SECS` (default: 15) — how often chain state is persisted to disk
- `RUST_LOG` (default: info) — log verbosity

## Common Use Cases

- Run a simulated EVM devnet for AI agent trading, coordination, and strategy testing
- Provide on-chain knowledge graphs and pheromone fields for agent swarm intelligence
- Serve as the chain backend for the Nunchi dashboard's chain views and agent registry
- Test smart contract interactions in a zero-cost sandbox with pre-deployed registries

## Dependencies for Mirage Hosting

- No external dependencies — Mirage is fully self-contained
- Optionally pair with a Roko control plane for full agent orchestration
- Optionally connect the Nunchi dashboard for visual monitoring

### Deployment Dependencies

- Volume mounted at `/workspace/.roko` for persistent chain state and snapshots
- No databases or external services required

## Why Deploy

- **Agent devnet**: Give AI agents a blockchain to transact on without real gas costs or mainnet risk
- **Zero config**: Boots with pre-deployed ERC-8004 identity, reputation, and validation registry contracts — ready to use immediately
- **Dashboard integration**: Powers the Nunchi dashboard's chain views, knowledge graph, pheromone field, and agent topology
- **Built-in relay**: Same-origin agent relay means no CORS issues — agents and frontends connect to a single endpoint
```

---

## 3. Roko Worker

**Short Description:** `Stateless AI agent worker for task execution`

**Category:** `Other`

**Template Overview:**

```
# Deploy and Host Roko Worker

Roko Worker is a stateless agent execution container. It receives task requests over HTTP, runs them through the Roko universal loop (compose prompt, dispatch to Claude, validate with gates), and reports results back to the control plane. Scale horizontally by deploying multiple workers.

## About Hosting

Workers are stateless with no persistent storage needed. Each worker reads its agent template from the `ROKO_TEMPLATE_JSON` environment variable (base64-encoded JSON) and listens for task submissions. The health endpoint at `GET /health` returns status, template name, and uptime.

### Environment Variables

- `ANTHROPIC_API_KEY` (required) — your Anthropic API key for Claude agent execution
- `ROKO_CONTROL_PLANE_URL` (required) — URL of your Roko control plane instance for result callbacks
- `ROKO_TEMPLATE_JSON` (required) — base64-encoded agent template JSON defining the worker's behavior
- `PORT` (default: 8080) — HTTP listen port
- `RUST_LOG` (default: info) — log verbosity

## Common Use Cases

- Execute AI coding tasks dispatched from the Roko control plane
- Run code-implementer agents that clone repos, execute plans, commit changes, and open PRs
- Scale agent fleet capacity by deploying multiple workers behind a load balancer
- Process one-off agent tasks via the `POST /task` endpoint

## Dependencies for Roko Worker Hosting

- A running Roko control plane instance (for task dispatch and result callbacks)
- An Anthropic API key with access to Claude (for agent execution)
- A base64-encoded agent template defining the worker's role and prompt

### Deployment Dependencies

- No persistent storage required — workers are fully stateless
- No databases or volumes needed
- Workers include Node.js, Python, and Claude Code CLI for agent tool execution

## Why Deploy

- **Horizontal scaling**: Spin up as many workers as you need — each handles tasks independently
- **Code agents**: Workers can clone repositories, generate code, run tests, commit changes, and open pull requests autonomously
- **Stateless design**: No storage management — deploy, use, tear down without cleanup
- **Rich tooling**: Pre-installed with Node.js, Python, and Claude Code CLI so agents have the tools they need at runtime
```
