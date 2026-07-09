# M156 — Create Fly.io Deployment Scripts

## Objective
Create Fly.io deployment configuration and scripts for roko services. Generate `fly.toml` for each deployable service (roko-serve HTTP control plane, roko-agent-server sidecar). Include auto_stop_machines, health checks, volume mounts for persistent state, and secret management. Create a unified `deploy.sh` that deploys all services. Wire into `roko deploy fly` CLI command.

## Scope
- Crates: `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/deploy/` (deployment configs)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs` (wire deploy command)
- Depth doc: `tmp/unified-depth/14-deployment/` (cloud deployment)

## Steps
1. Read existing deploy directory:
   ```bash
   ls -la /Users/will/dev/nunchi/roko/roko/deploy/
   cat /Users/will/dev/nunchi/roko/roko/deploy/roko-agent/fly.toml 2>/dev/null
   cat /Users/will/dev/nunchi/roko/roko/deploy/mirage/fly.toml 2>/dev/null
   ```

2. Read existing deploy CLI command:
   ```bash
   grep -rn 'deploy\|Deploy\|fly\|Fly' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/ --include='*.rs' | head -15
   ```

3. Create `deploy/roko-serve/fly.toml`:
   ```toml
   app = "roko-serve"
   primary_region = "iad"

   [build]
   dockerfile = "../../Dockerfile.serve"

   [env]
   RUST_LOG = "info"
   ROKO_PORT = "8080"

   [http_service]
   internal_port = 8080
   force_https = true
   auto_stop_machines = true
   auto_start_machines = true
   min_machines_running = 0

   [[http_service.checks]]
   grace_period = "10s"
   interval = "30s"
   method = "GET"
   path = "/health"
   timeout = "5s"

   [mounts]
   source = "roko_data"
   destination = "/data/.roko"
   ```

4. Create `deploy/roko-agent-server/fly.toml` (similar structure, internal service).

5. Create `deploy/deploy.sh`:
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail

   SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

   echo "=== Deploying roko-serve ==="
   cd "$SCRIPT_DIR/roko-serve"
   fly deploy --ha=false

   echo "=== Deploying roko-agent-server ==="
   cd "$SCRIPT_DIR/roko-agent-server"
   fly deploy --ha=false

   echo "=== All services deployed ==="
   fly status --app roko-serve
   fly status --app roko-agent-server
   ```

6. Create minimal Dockerfiles:
   ```bash
   # Check if Dockerfiles exist
   ls /Users/will/dev/nunchi/roko/roko/Dockerfile* 2>/dev/null
   ```
   If not present, create `Dockerfile.serve`:
   ```dockerfile
   FROM rust:1.91-slim AS builder
   WORKDIR /app
   COPY . .
   RUN cargo build --release -p roko-serve

   FROM debian:bookworm-slim
   RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
   COPY --from=builder /app/target/release/roko-serve /usr/local/bin/
   ENTRYPOINT ["roko-serve"]
   ```

7. Wire secret management:
   ```bash
   # Document required secrets
   # ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.
   ```
   Add a `deploy/secrets.example` file listing required secrets without values.

8. Wire into CLI `roko deploy fly`:
   - Verify `fly` CLI is installed
   - Check if apps exist (create if not)
   - Run deploy.sh or equivalent logic
   - Report deployment status

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
# Verify fly.toml is valid TOML:
cat /Users/will/dev/nunchi/roko/roko/deploy/roko-serve/fly.toml | python3 -c "import sys,tomllib; tomllib.loads(sys.stdin.read())"
# Verify deploy.sh is executable:
bash -n /Users/will/dev/nunchi/roko/roko/deploy/deploy.sh
```

## What NOT to do
- Do NOT deploy anything — only create config files and scripts
- Do NOT hardcode API keys — use `fly secrets set` workflow
- Do NOT add Kubernetes/Helm — Fly.io only in this batch
- Do NOT create multi-region configs — single region (iad) for now
- Do NOT add CI/CD pipeline — that is separate infrastructure
