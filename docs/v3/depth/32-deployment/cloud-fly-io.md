# 32-deployment/07 -- Cloud Deployment: Fly.io

> Firecracker microVMs with persistent volumes, auto-stop/start, private
> networking, per-service fly.toml, deploy scripts, and cost estimates.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `deploy/fly/`, `deploy/scripts/`

---

## 1. Architecture on Fly.io

Each service runs as a Firecracker microVM (called a "machine"). Services
in the same Fly organization communicate over a private IPv6 network
(6PN) using `.internal` DNS names. The CLI orchestrator reaches the API
server at `http://roko-serve.internal:6677`. This traffic stays on Fly's
private network.

```
               Fly.io Organization
  HTTPS -----> roko-serve (6677) <---> roko-cli (6677)
  (public)          |                       |
                    | .internal             | volume: /data
                    v
               roko-console (3000)
               (Caddy proxy)
```

---

## 2. fly.toml: roko-serve

```toml
app = "roko-serve"
primary_region = "iad"

[build]
  dockerfile = "docker/roko-serve.Dockerfile"

[env]
  RUST_LOG = "roko_serve=info"

[http_service]
  internal_port = 6677
  force_https = true
  auto_stop_machines = "stop"
  auto_start_machines = true
  min_machines_running = 0

  [http_service.concurrency]
    type = "requests"
    hard_limit = 500
    soft_limit = 250

[[http_service.checks]]
  grace_period = "10s"
  interval = "30s"
  method = "GET"
  path = "/readyz"
  timeout = "5s"

[[vm]]
  size = "shared-cpu-2x"
  memory = "1gb"
```

### Key Settings

- `auto_stop_machines = "stop"` with `min_machines_running = 0`: machine
  stops when idle. No CPU/RAM charges while stopped. Starts in ~2-3s on
  request arrival.
- `[mounts]` (for stateful services): persistent Fly volume for `.roko/`
  state. Survives machine stops and restarts.

---

## 3. fly.toml: roko-cli (Orchestrator)

```toml
app = "roko-cli"
primary_region = "iad"

[build]
  dockerfile = "docker/roko-cli-full.Dockerfile"

[env]
  RUST_LOG = "roko=info"

[mounts]
  source = "roko_data"
  destination = "/data"

[http_service]
  internal_port = 6677
  force_https = true
  auto_stop_machines = "stop"
  auto_start_machines = true
  min_machines_running = 0

[[vm]]
  size = "shared-cpu-2x"
  memory = "2gb"
```

Memory is 2GB because the code index (tree-sitter + HDC + symbol graph)
can grow for large codebases.

---

## 4. Deploy Scripts

### fly-deploy.sh

```bash
#!/bin/bash
# deploy/scripts/fly-deploy.sh
# Usage:
#   ./deploy/scripts/fly-deploy.sh all
#   ./deploy/scripts/fly-deploy.sh roko-serve

set -euo pipefail

deploy_service() {
    local service="$1"
    local fly_dir="$REPO_ROOT/deploy/fly/$service"
    local app_name
    app_name=$(grep '^app = ' "$fly_dir/fly.toml" | sed 's/app = "\(.*\)"/\1/')

    # Create app if needed
    if ! fly apps list --json | jq -e ".[] | select(.Name == \"$app_name\")" >/dev/null 2>&1; then
        fly apps create "$app_name" --org personal
    fi

    # Create volume if needed
    if grep -q '\[mounts\]' "$fly_dir/fly.toml"; then
        # ... volume creation logic ...
    fi

    fly deploy --config "$fly_dir/fly.toml" --app "$app_name" --remote-only
}
```

### fly-secrets.sh

Reads from `.env` or environment variables, sets secrets per service:

```bash
fly secrets set ANTHROPIC_API_KEY="${ANTHROPIC_API_KEY}" --app roko-serve
```

Secrets become environment variables inside the machine. They never appear
in fly.toml, Docker images, or logs.

### fly-status.sh

Checks health of all deployed services by calling `/readyz`.

---

## 5. Secret Management on Fly.io

Provider API keys live as Fly secrets, encrypted at rest:

```
deploy command
  -> reads --anthropic-key or $ANTHROPIC_API_KEY
  -> fly secrets set ANTHROPIC_API_KEY=<value> --app roko-cli
  -> injected as env vars at machine startup
```

On redeploy, the script checks existing secrets via
`fly secrets list --app ... --json` and only prompts for missing ones.

---

## 6. Cost Estimates

With auto-stop enabled (prices as of early 2026):

| Service | VM Size | Memory | Est. Cost |
|---------|---------|--------|-----------|
| roko-serve | shared-cpu-2x | 1GB | ~$2-5/mo |
| roko-cli | shared-cpu-2x | 2GB | ~$3-7/mo |
| console | shared-cpu-1x | 256MB | ~$1-2/mo |
| Volumes (1GB) | -- | -- | ~$0.15/mo |

Personal deployment with intermittent use: $5-15/month total. For
always-on production, set `min_machines_running = 1` to eliminate cold
starts at continuous cost.

---

## 7. Custom Domains

```bash
fly certs create roko-serve api.example.com
# Add CNAME: api.example.com -> roko-serve.fly.dev
# Certificate provisions automatically via Let's Encrypt
```

No ACME DNS records, no cert files, no renewal cron.

---

## 8. Console Service: Web Terminal

A lightweight Caddy reverse proxy forwards WebSocket connections to each
service's ttyd instance over Fly's internal network. A static HTML page
with xterm.js renders the terminal in the browser.

```
https://roko-console.fly.dev/
  +-- [Tab: Roko CLI]   -> Live TUI in browser
  +-- [Tab: Roko Serve] -> Live server logs in browser
```

---

## 9. Deployment as a Gate Step

Plans can include deployment as a verification step:

```markdown
### Verification
- Deploy to staging: `roko deploy --target fly --env staging`
- Health check passes within 30 seconds
- Smoke test: `/readyz` returns 200
```

If the deploy fails or health check does not pass, the plan does not
advance.

---

## 10. Directory Structure

```
deploy/
  fly/
    roko-cli/fly.toml
    roko-serve/fly.toml
    console/fly.toml
  console/
    Caddyfile
    static/index.html
  scripts/
    fly-deploy.sh
    fly-secrets.sh
    fly-status.sh
    fly-logs.sh
```

---

## 11. Implementation Status

> **Implementation status:** Fly.io deployment is designed but not
> configured. The `roko deploy fly` CLI subcommand exists. fly.toml
> configs, deploy scripts, and the console service are specified. The
> preferred cloud port is 6677 (matching roko-serve default).
