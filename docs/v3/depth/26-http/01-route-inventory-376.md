# 26.01 -- Route Inventory (~376 Canonical Routes)

> Depth file for [26-HTTP.md](../../26-HTTP.md).

---

## Router Assembly

The central `build_router()` in `crates/roko-serve/src/routes/mod.rs` assembles the
complete API surface. Routes are nested under `/api` with a handful of top-level
unauthenticated endpoints.

### Top-Level (No Auth, No `/api` Prefix)

| Method | Path | Handler | Purpose |
|---|---|---|---|
| GET | `/health` | `top_level_health` | Liveness probe for load balancers |
| GET | `/ready` | `top_level_ready` | Readiness probe (returns 503 during shutdown) |
| GET | `/metrics` | `metrics_handler` | Prometheus scrape endpoint |

### Route Domain Modules

Each route group is defined in its own submodule under `routes/` and merged into
the api router via `Router::merge()` or `Router::nest()`:

```rust
let api = Router::new()
    .merge(openapi::routes())     // /openapi.json
    .merge(status::routes())      // /health, /session, /dashboard, /episodes, /signals
    .merge(jobs::routes())        // /jobs CRUD + match + execute
    .merge(heartbeats::routes())  // /heartbeats
    .merge(plans::routes())       // /plans CRUD + execute + status
    .merge(prds::routes())        // /prds CRUD + idea + draft + plan
    .merge(run::routes())         // /run (single prompt execution)
    .merge(runs::routes())        // /runs observability
    .merge(research::routes())    // /research endpoints
    .merge(subscriptions::routes())
    .merge(templates::routes())
    .merge(aggregator::routes())
    .merge(arenas::routes())      // /arenas lifecycle + attempts + settlement
    .merge(meta::routes())        // /meta-agents lineage
    .merge(agents::routes())      // /agents registration + lifecycle
    .merge(learning::routes())    // /learning/* cascade, experiments, etc.
    .merge(marketplace::routes()) // /marketplace publish/install
    .merge(defi::routes())        // /defi instruments (501 stubs)
    .merge(registries::routes())  // /registries passport + knowledge
    .merge(config::routes())      // /config CRUD + reload
    .merge(deployments::routes()) // /deployments CRUD + proxy
    .merge(diagnosis::routes())   // /diagnosis endpoints
    .merge(integrations::routes())
    .merge(projections::routes()) // /projections telemetry + lens
    .merge(neuro::routes())       // /neuro knowledge query
    .merge(dream::routes())       // /dream consolidation
    .merge(event_ingest::routes())// /events/ingest
    .merge(extensions::routes())  // /extensions metadata
    .merge(gateway::routes())     // /inference gateway
    .merge(chain::routes())       // /chain (feature-gated)
    .merge(connectors::routes())  // /connectors transport
    .merge(feeds::routes())       // /feeds CRUD + runtime
    .merge(recipes::routes())     // /recipes evaluation
    .merge(groups::routes())      // /groups membership + knowledge
    .merge(auth::routes())        // /auth audit + api-keys + tokens
    .merge(secrets::routes())     // /secrets CRUD
    .merge(vision_loop::routes()) // /vision-loop
    .merge(team::routes())        // /team RBAC
    .merge(bench::routes())       // /bench evaluation
    .merge(swe_bench::routes())   // /swe-bench
    .merge(triggers::routes())    // /triggers CRUD + fire
    .merge(workflows::routes())   // /workflows
    .merge(workspaces::routes())  // /workspaces
    .merge(history::routes())     // /history
    .merge(cache::routes())       // /cache
    .merge(doctor::routes())      // /doctor diagnostics
    .merge(safety::routes())      // /safety inspection
    .merge(affect::routes())      // /affect state
    .merge(shared_runs::auth_routes())
    .merge(webhooks::authenticated_routes())
    .nest("/providers", providers::router())
    .nest("/models", providers::models_router())
    .nest("/routing", providers::routing_router())
    .merge(sse::routes())         // /events, /sse
    .merge(rpc_proxy::routes())   // /rpc proxy
    .route("/workflow/events", get(workflow_sse_handler));
```

### Routes by Domain

#### Status and Health (~8 routes)

- `GET /api/health` -- Rich health with session counts and version
- `GET /api/session` -- Session status
- `GET /api/metrics` -- Metrics summary
- `GET /api/dashboard` -- Dashboard snapshot
- `GET /api/episodes` -- Episode listing
- `GET /api/signals` -- Signal listing
- `GET /api/operation/:id` -- Operation status by ID

#### Plans (~12 routes)

- `GET /api/plans` -- List plans
- `POST /api/plans` -- Create plan
- `GET /api/plans/:id` -- Get plan detail
- `POST /api/plans/:id/execute` -- Execute plan
- `GET /api/plans/:id/status` -- Execution status
- `POST /api/plans/generate` -- Generate plan from prompt

#### Run Execution + Observability (~14 routes)

- `POST /api/run` -- Single prompt execution (rate-limited: 30 req/min burst 10)
- `GET /api/run/status` -- Run status
- `GET /api/runs/:id` -- Run detail
- `GET /api/runs/:id/events` -- Run events
- `GET /api/runs/:id/events/stream` -- Run event SSE stream
- `GET /api/runs/:id/tasks` -- Run tasks
- `GET /api/runs/:id/attempts` -- Run attempts
- `GET /api/runs/:id/gates` -- Run gate results
- `GET /api/runs/:id/logs` -- Run logs
- `GET /api/runs/:id/metrics` -- Run metrics
- `GET /api/runs/:id/artifacts` -- Run artifacts
- `GET /api/runs/:id/screenshots` -- Run screenshots
- `GET /api/runs/:id/bundle` -- Complete evidence bundle

#### Agents (~16 routes)

- `GET /api/agents` -- List managed agents (rate-limited: 5 req/min burst 5)
- `POST /api/agents` -- Register agent
- `GET /api/agents/:id` -- Agent detail
- `DELETE /api/agents/:id` -- Stop agent
- `POST /api/agents/:id/observation` -- Lifecycle observation commit
- `GET /api/agents/:id/episodes` -- Agent episode history
- `GET /api/agents/:id/logs` -- Proxy agent logs
- `POST /api/agents/:id/message` -- Send message to agent
- `GET /api/agents/:id/token` -- Token status
- `POST /api/agents/:id/token` -- Issue/rotate token
- `GET /api/agents/topology` -- Agent topology
- `GET /api/agents/stats` -- Aggregate agent stats
- `GET /api/agents/:id/skills` -- Agent skill manifest
- `POST /api/agents/:id/heartbeat` -- Record heartbeat
- `GET /api/agents/:id/trace` -- Agent trace

#### Providers and Models (~8 routes)

- `GET /api/providers` -- List providers
- `GET /api/providers/:name/health` -- Provider health
- `POST /api/providers/:name/test` -- Test provider connectivity
- `GET /api/models` -- List configured models
- `GET /api/routing/explain` -- Explain routing decision

#### Learning and Feedback (~10 routes)

- `GET /api/learning/efficiency` -- Efficiency summary
- `GET /api/learning/cascade-router` -- Cascade router state
- `GET /api/learning/cascade` -- Cascade explanation
- `GET /api/learning/cost-tiers` -- Cost tier breakdown
- `GET /api/learning/experiments` -- Experiment listing
- `GET /api/learning/adaptive-thresholds` -- Adaptive threshold state
- `GET /api/learning/gate-thresholds` -- Gate threshold EMA values

#### PRDs (~8 routes)

- `GET /api/prds` -- List PRDs
- `POST /api/prds/idea` -- Post idea
- `GET /api/prds/:slug` -- Get PRD
- `POST /api/prds/:slug/draft` -- Draft PRD
- `POST /api/prds/:slug/promote` -- Promote PRD
- `POST /api/prds/:slug/plan` -- Generate plan from PRD

#### Groups (~16 routes)

- `GET /api/groups` -- List groups
- `POST /api/groups` -- Create group
- `GET /api/groups/:id` -- Get group
- `PUT /api/groups/:id` -- Update group
- `DELETE /api/groups/:id` -- Delete group
- `POST /api/groups/:id/invite` -- Invite agent
- `GET /api/groups/:id/invitations` -- List invitations
- `POST /api/invitations/:id/accept` -- Accept invitation
- `POST /api/invitations/:id/reject` -- Reject invitation
- `GET /api/groups/:id/members` -- List members
- `PUT /api/groups/:id/members/:mid` -- Update member role
- `DELETE /api/groups/:id/members/:mid` -- Remove member
- `GET /api/groups/:id/knowledge` -- Group knowledge
- `POST /api/groups/:id/knowledge` -- Publish group knowledge
- `GET /api/groups/:id/pheromones` -- List pheromones
- `POST /api/groups/:id/pheromones` -- Deposit pheromone
- `POST /api/groups/:id/messages` -- Publish group message
- `GET /api/groups/:id/events` -- Group events

#### Arenas (~10 routes)

- `GET /api/arenas` -- List arenas
- `POST /api/arenas` -- Create arena
- `GET /api/arenas/:id` -- Get arena
- `POST /api/arenas/:id/transition` -- Transition arena state
- `GET /api/arenas/:id/leaderboard` -- Arena leaderboard
- `GET /api/arenas/:id/attempts` -- List attempts
- `POST /api/arenas/:id/attempts` -- Start attempt
- `GET /api/arenas/:id/attempts/:aid` -- Get attempt
- `POST /api/arenas/:id/attempts/:aid/submit` -- Submit attempt
- `POST /api/arenas/:id/attempts/:aid/settle` -- Settle attempt

#### Registries (~14 routes)

- `GET /api/registries/passports` -- List passports
- `POST /api/registries/passports` -- Mint passport
- `GET /api/registries/passports/:id` -- Get passport
- `GET /api/registries/passports/:id/history` -- Passport history
- `POST /api/registries/passports/:id/transfer` -- Transfer passport
- `POST /api/registries/passports/:id/metadata` -- Update metadata
- `POST /api/registries/passports/:id/delegate` -- Add delegation
- `DELETE /api/registries/passports/:id/delegate/:did` -- Revoke delegation
- `GET /api/registries/knowledge` -- List registry knowledge
- `POST /api/registries/knowledge` -- Publish registry knowledge
- `GET /api/registries/knowledge/:id` -- Get knowledge entry
- `POST /api/registries/knowledge/:id/validate` -- Validate entry
- `POST /api/registries/knowledge/:id/challenge` -- Challenge entry
- `POST /api/registries/knowledge/:id/resolve` -- Resolve challenge
- `GET /api/registries/events` -- Registry event log
- `GET /api/registries/stats` -- Registry statistics
- `POST /api/registries/indexer/sync` -- Sync indexer
- `POST /api/registries/indexer/rebuild` -- Rebuild indexer

#### Additional Domain Routes

Each of these modules contributes additional routes:

- **Config** (~6): get, update, reload, validate, export, migrate
- **Deployments** (~6): list, get, logs, create, proxy, callback
- **Research** (~6): list, topic, enhance-prd, enhance-plan, enhance-tasks, analyze
- **Subscriptions** (~8): list, create, update, delete, enable, disable, relay status
- **Templates** (~6): list, create, get, delete, deploy
- **Feeds** (~12): list, create, get, delete, catalog, runtime, status, discover, search, health, start, stop
- **Recipes** (~5): list, save, get, delete, evaluate
- **Triggers** (~6): list, create, get, delete, fire, history
- **Secrets** (~4): list, set, delete, test
- **Projections** (~8): catalog, telemetry, stream, named projections, lens runtimes, statehub
- **Gateway** (~4): inference dispatch, batch, events (rate-limited: 30 req/min burst 10)
- **DeFi** (~8): structured 501 stubs for instruments, bonds, options, insurance, index
- **Meta-agents** (~6): list, propose, get, validate, morph, rollback, deactivate
- **Connectors** (~6): list, create, delete, health, restart, query, execute
- **Marketplace** (~4): list artifacts, publish, install, fork
- **Dreams** (~2): run, journal
- **Webhooks** (~4): GitHub, Slack, generic (public and authenticated variants)
- **Safety** (~2): inspection routes
- **Affect** (~2): daimon state routes
- **Cache** (~2): status, prune
- **Doctor** (~2): diagnostics
- **History** (~2): list, get

## Counting Methodology

The ~376 canonical route count is produced by `python3 tools/http_route_inventory.py`,
which scans all `routes()` functions for `.route()` calls, counts distinct
(method, path) pairs, and includes feature-gated variants. The ~421 total includes
legacy aliases (e.g., `/events` alongside `/sse`).

## Source

- `crates/roko-serve/src/routes/mod.rs` -- Router assembly
- `crates/roko-serve/src/routes/*.rs` -- Individual domain handlers
- `tools/http_route_inventory.py` -- Automated route counter
