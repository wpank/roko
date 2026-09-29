# 32-deployment/08 -- Cloud Deployment: Railway

> Railway project setup, service deployment via GraphQL API, secret
> management, health checks, and the deprecation of --with-mirage.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-cli/src/commands/deploy.rs`

---

## 1. Overview

Railway deployment creates a Railway project with roko-serve as the
control plane, configures secrets, and deploys from a Docker image or
Nixpack build. The `roko deploy railway` CLI subcommand orchestrates
the full flow via Railway's public GraphQL API.

Unlike Fly.io (Firecracker microVMs with fly.toml), Railway uses a
project-service-environment model with Nixpacks auto-detection or
explicit Dockerfiles.

---

## 2. The `roko deploy railway` Command

```
roko deploy railway [OPTIONS]

Options:
  --workers <N>       Deploy N agent worker services (default: 0)
  --region <REGION>   Railway region (default: us-west1)
  --env <NAME>        Environment name (default: production)
```

### Deprecated: --with-mirage

The `--with-mirage` flag previously deployed an additional mirage-rs
chain relay service alongside roko-serve. This flag is **deprecated** as
of 2026-09. The mirage-rs service has been decoupled from the core
deployment workflow. To deploy mirage-rs separately, use the standalone
Docker image or a dedicated Railway service.

---

## 3. Deployment Flow

```
roko deploy railway
  1. Authenticate with Railway (RAILWAY_TOKEN or browser login)
  2. Create project if it does not exist
  3. Create roko-serve service
  4. Set secrets (ANTHROPIC_API_KEY, etc.)
  5. Configure health check (/readyz on port 6677)
  6. Trigger deployment from Dockerfile
  7. Wait for health check to pass
  8. Print public URL
```

### Railway GraphQL API

The deploy command uses Railway's GraphQL endpoint directly rather than
shelling out to the Railway CLI. This avoids an external tool dependency:

```rust
async fn create_service(
    client: &reqwest::Client,
    token: &str,
    project_id: &str,
    name: &str,
) -> Result<String> {
    let query = r#"
        mutation($projectId: String!, $name: String!) {
            serviceCreate(input: {
                projectId: $projectId,
                name: $name
            }) { id }
        }
    "#;
    // ... execute GraphQL mutation ...
}
```

---

## 4. Project Structure on Railway

```
Railway Project: roko-workspace
  +-- Service: roko-serve
  |     Port: 6677
  |     Health: /readyz
  |     Secrets: ANTHROPIC_API_KEY, RUST_LOG
  |
  +-- Service: worker-1 (optional, via --workers)
  |     Template: agent worker
  |     Reads tasks from roko-serve
  |
  +-- Service: worker-2 (optional)
        Same template
```

### Service Configuration

| Setting | Value |
|---------|-------|
| Port | 6677 |
| Health check path | `/readyz` |
| Health check interval | 30s |
| Restart policy | On failure |
| Build | Dockerfile (`docker/roko-serve.Dockerfile`) |

---

## 5. Secret Management

Secrets are set via the Railway GraphQL API and injected as environment
variables at runtime:

```rust
async fn set_secrets(
    client: &reqwest::Client,
    token: &str,
    service_id: &str,
    env_id: &str,
    secrets: &HashMap<String, String>,
) -> Result<()> {
    let query = r#"
        mutation($input: VariableCollectionUpsertInput!) {
            variableCollectionUpsert(input: $input)
        }
    "#;
    // ... set each secret as a Railway variable ...
}
```

The deploy command reads secrets from:

1. CLI flags (`--anthropic-key`)
2. Environment variables (`$ANTHROPIC_API_KEY`)
3. Interactive prompt (if neither is set)

Secrets never appear in logs, Railway service config files, or Docker
image layers.

---

## 6. Worker Services

The `--workers <N>` option deploys N agent worker services from a
template. Workers connect to roko-serve over Railway's private network
and pull tasks from the control plane:

```
Worker startup:
  1. Read ROKO_SERVE_URL from environment
  2. Register with roko-serve via POST /v1/workers/register
  3. Poll for tasks via GET /v1/workers/tasks
  4. Execute tasks, report results
```

Workers use the same Docker image as roko-serve but run with the
`roko worker` entrypoint instead of `roko serve`.

---

## 7. Regions

Railway supports multiple regions. The deploy command defaults to
`us-west1` but accepts any supported region:

| Region | Location |
|--------|----------|
| `us-west1` | US West (default) |
| `us-east1` | US East |
| `eu-west1` | Europe West |
| `asia-east1` | Asia East |

For latency-sensitive deployments, pick the region closest to your LLM
provider endpoints (Anthropic, OpenAI).

---

## 8. Deployment Lifecycle

### First Deploy

```bash
$ roko deploy railway

[1/7] Authenticating with Railway...
[2/7] Creating project "roko-workspace"...
[3/7] Creating service "roko-serve"...
[4/7] Setting secrets (3 variables)...
[5/7] Configuring health check (/readyz)...
[6/7] Triggering deployment...
[7/7] Waiting for health check...

Deployed successfully.
  URL:     https://roko-workspace-production.up.railway.app
  Service: roko-serve
  Status:  healthy
```

### Redeployment

Subsequent `roko deploy railway` calls detect the existing project and
service, update secrets if needed, and trigger a new deployment.

### Teardown

```bash
roko deploy railway --teardown
```

Removes the Railway project and all associated services.

---

## 9. Differences from Fly.io

| Aspect | Railway | Fly.io |
|--------|---------|--------|
| Runtime | Container (Docker) | Firecracker microVM |
| Config format | GraphQL API / Dashboard | fly.toml |
| Auto-stop | Paid feature | Built-in |
| Private networking | Project-scoped | Organization-scoped (6PN) |
| Volumes | Mounted volumes | Fly Volumes |
| Deploy CLI | `roko deploy railway` | `roko deploy fly` |

Both targets produce the same observable behavior: roko-serve on port
6677 with health checks on `/readyz`. The deployment interface is
designed as a trait; adding a target means implementing `deploy()`,
`teardown()`, `status()`, and `logs()`.

---

## 10. Cost Estimates

Railway pricing with usage-based billing:

| Service | Memory | vCPU | Est. Cost |
|---------|--------|------|-----------|
| roko-serve | 1GB | 1 vCPU | ~$5-10/mo |
| Each worker | 512MB | 0.5 vCPU | ~$3-5/mo |
| Network egress | -- | -- | Included |

Railway charges per-second for compute. Idle services with auto-sleep
(paid plans) reduce costs significantly.

---

## 11. Implementation Status

> **Implementation status:** The `roko deploy railway` subcommand exists
> in the CLI with the Railway GraphQL API integration. The `--with-mirage`
> flag is deprecated. Worker template deployment is designed but depends
> on the worker registration API in roko-serve. The deployment trait
> interface is shared with the Fly.io target.
