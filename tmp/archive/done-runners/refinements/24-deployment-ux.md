# Deployment UX

> **TL;DR**: Roko should be deployable in five distinct shapes —
> laptop-local, single-server, container, clustered, and edge —
> with almost no code differences between them. The single Rust
> binary is the first shape; everything else is a configuration
> layer on top of it. This doc proposes deployment profiles,
> secret handling, state portability, and observability so
> moving between shapes is a one-day task, not a rewrite.

### For first-time readers

Recurring terms:

- **Substrate** — durable store (Engrams): SQLite on laptop,
  Postgres/object store at scale. Trait-level; swap with config.
- **Bus** — ephemeral stream (Pulses): in-memory by default,
  NATS/Redis/Kafka for clustered. Trait-level; swap with config.
- **Profile** — a bundle of defaults for a deployment shape
  (laptop / single-server / container / clustered / edge). One
  binary, five profiles.
- **State portability** — substrate + bus queues + config
  exported/imported as one signed archive. See §4.

## 1. The shapes

### 1.1 Laptop-local (developer mode)

- Single user, single machine, `./.roko` in the project directory.
- Models: local (Ollama/LM Studio) and/or cloud APIs with keys.
- No HTTP exposure unless explicitly `roko serve`.
- All plugins are local files.

This is the default and the most common. Works today. Improvements
focused on `23` (user UX) and the `roko init` flow.

### 1.2 Single-server (team shared)

- A small team shares one Roko instance on a box they control.
- `roko serve` exposed to LAN or VPN.
- State in a shared directory or a lightweight DB (SQLite or
  PostgreSQL).
- Plugin set curated for the team.

Key addition: **multi-user auth**. A minimal identity layer that
tags episodes, heuristics, and decisions with who-initiated-what.

### 1.3 Container (cloud-host)

- Docker image, runs on anyone's container runtime.
- Stateful volume mount.
- Environment-based configuration.
- Probes (liveness, readiness) wired to the control plane.

### 1.4 Clustered (scale-out)

- Multiple Roko instances behind a load balancer.
- Shared Substrate (Postgres or object-store backed).
- Shared Bus (NATS, Redis Pub/Sub, or Kafka).
- Sticky routing for long-lived sessions; stateless otherwise.

### 1.5 Edge (embedded, serverless, WASM)

- Minimal feature set.
- No persistence or read-only memory.
- Called per-request, returns, disappears.
- Targets: Cloudflare Workers, Deno Deploy, Lambda, WASM
  runtimes.

Each shape can be reached by the same binary with a different
config. The deployment experience is picking a shape, not
rebuilding.

## 2. Deployment profiles

A new config concept: `profile`.

```toml
# roko.toml
profile = "single-server"   # one of: laptop, single-server, container, clustered, edge

[profile.single-server]
listen      = "0.0.0.0:6677"
auth        = "basic"
substrate   = { kind = "sqlite", path = "/var/lib/roko/state.db" }
bus         = { kind = "in-memory" }
```

Profiles bundle defaults. A user pins a profile and overrides only
what they care about. Profiles are user-writable and shippable as
tier-2 plugins (`17` §2).

## 3. Secrets story

Today model API keys live in env vars or config files. For broader
deployment, we need:

- **Layered resolution**: env → config → OS keychain → secret store
  (Vault / AWS Secrets Manager / 1Password CLI).
- **Never-logged**: secrets tagged in config schemas so log
  sanitization is automatic.
- **Rotation-friendly**: the Roko process can pick up a new secret
  without restart (subscribe to secret-change events).
- **Per-role secrets**: the researcher might have a Perplexity
  key; the implementer doesn't. Role-scoped injection.

Proposed CLI:

```bash
roko secret set anthropic.api_key
roko secret get anthropic.api_key     # requires confirmation
roko secret list
roko secret rotate anthropic.api_key
```

Behind the scenes, backed by the OS keychain for laptop mode, by
`vault` or equivalent for server mode. Swappable via a `SecretStore`
trait.

## 4. State portability

The single most important cross-shape concern. Users need to:

- Move state from laptop to server without surgery.
- Back up state.
- Reset state (for a clean experiment).
- Split state (project A vs project B).

Proposed:

```bash
roko state export <file.tar.zst>   # serializes substrate, bus queues, config
roko state import <file.tar.zst>
roko state split --by project      # produces N archives
roko state merge <file1> <file2>   # merges archives with conflict policy
roko state gc --dry-run            # shows what demurrage would drop
```

State archives should be content-addressed, versioned, and signed
(for integrity). Users can audit what they're about to import. A
commons member can share "their heuristic state" as a signed
archive.

## 5. Observability is part of deployment

Every Roko shape needs:

- **Structured logs** (JSON) on stderr by default.
- **Prometheus-compatible metrics** on `/metrics`.
- **OpenTelemetry traces** with spans around each operator.
- **Health probes**: `/healthz` (liveness), `/readyz` (readiness).

And *domain-specific* metrics that aren't in existing dashboards:

- `roko.c_factor` (gauge)
- `roko.demurrage.balance_p50` / `_p95` (histogram)
- `roko.heuristic.calibration_brier_score` (gauge)
- `roko.gate.pass_rate` by rung (counter rate)
- `roko.bus.pulses_per_second` by topic (counter rate)
- `roko.substrate.query_latency_p99` by kind (histogram)

These are uniquely Roko's; every deployment exposes them; they
can be scraped by existing Prometheus/Grafana setups without
special integration.

## 6. Zero-downtime upgrades

For single-server and clustered shapes, the goal is rolling
upgrades without losing work.

- **Graceful shutdown**: SIGTERM causes Roko to stop accepting new
  tasks, finish running ones, checkpoint state, exit.
- **Warm restart**: a new process reads the checkpoint, resumes.
- **Cluster rolling**: load balancer drains one node, the replaced
  node rejoins, repeat.

Build on existing `--resume` support in the plan executor; extend
to cover agents mid-turn.

## 6.5 Metrics and log aggregation

`/metrics`, `/healthz`, and structured stderr are the low layer.
Most operators want them flowing into an existing stack. Supported
paths, none of which fork the binary:

- **Prometheus + Grafana**: the default. Ship a Grafana dashboard
  JSON under `packaging/dashboards/roko.json` covering c-factor,
  gate pass-rates, demurrage balance, pulse throughput.
- **OpenTelemetry**: `RUST_LOG` plus
  `OTEL_EXPORTER_OTLP_ENDPOINT=https://otel.example.com` turns on
  OTLP export for traces, metrics, and logs. No additional config
  needed — the operator spans from §5 are already OTel-native.
- **ELK / OpenSearch**: switch stderr to `--format=json` (ECS
  field names) and point Filebeat at it. Index mapping shipped
  under `packaging/elk/roko-ecs-template.json`.
- **Loki / Grafana Agent**: the same JSON works as-is; promtail
  labels picked up via `--log-labels tenant=${ROKO_TENANT}`.
- **Datadog / New Relic / Honeycomb**: OTLP path covers all three.
- **Audit sink**: every Custody record (`25-domain-specific-agents.md`
  §8.2) is emitted as a separate structured log line with a
  stable schema, so SIEM ingestion doesn't need to know anything
  about Roko internals.

Rules:

- Metric names are `roko.<subsystem>.<thing>`; no shape-specific
  names (no `roko.clustered.*` vs `roko.laptop.*`). The shape is
  a label, not a prefix.
- Labels are low-cardinality by default. Tenant and role go on
  metrics; episode-hash stays in traces only.
- Sampling: high-volume subsystems (Bus throughput, token streams)
  ship with a tail-based sampler on by default; per-subsystem
  overrides via `[observability.sampling]` in `roko.toml`.

See also `27-realtime-event-surface.md` §4 — the same cursor
format is reused for log-tail subscriptions, so `roko logs follow
--tenant acme` and a Grafana Loki query see identical events.

## 7. Platform-specific shapes

### 7.1 Kubernetes

Ship a Helm chart. Support StatefulSet for single-server, Deployment
+ HPA for clustered. Config via ConfigMap; secrets via K8s Secret;
volumes for state.

Opinionated values file targeting the common case:

```yaml
roko:
  replicas: 3
  profile: clustered
  persistence:
    size: 20Gi
    storageClass: fast-ssd
  secrets:
    anthropic:
      existingSecretKey: roko-anthropic-key
  ingress:
    enabled: true
    host: roko.internal.example.com
```

### 7.2 Docker Compose

For single-server teams without K8s. A ready-made compose file with
Roko + Postgres + NATS. Two commands to be running.

A multi-stage `Dockerfile` keeps the production image small,
reproducible, and free of the toolchain. Sketch:

```dockerfile
# --- build stage ---
FROM rust:1.91-slim AS build
WORKDIR /src
RUN apt-get update && apt-get install -y --no-install-recommends \
      pkg-config libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Cache deps: copy manifests first, build a skeleton.
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo fetch --locked

# Real build against a warm dep cache.
ARG ROKO_PROFILE=release
RUN cargo build --locked --profile ${ROKO_PROFILE} --bin roko && \
    strip target/${ROKO_PROFILE}/roko

# --- runtime stage ---
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app
COPY --from=build /src/target/release/roko /usr/local/bin/roko
COPY packaging/docker/roko.toml /etc/roko/roko.toml

ENV ROKO_CONFIG=/etc/roko/roko.toml \
    ROKO_STATE_DIR=/var/lib/roko \
    ROKO_LOG_FORMAT=json \
    RUST_LOG=info

VOLUME ["/var/lib/roko"]
EXPOSE 6677
USER nonroot:nonroot

HEALTHCHECK --interval=15s --timeout=3s --start-period=10s \
  CMD ["/usr/local/bin/roko", "probe", "readyz"]

ENTRYPOINT ["/usr/local/bin/roko"]
CMD ["serve", "--profile", "container"]
```

Notes:

- Builder is `rust:1.91-slim` to match the MSRV flagged in
  CLAUDE.md. CI pins the digest.
- Runtime is `distroless/cc` — no shell, no package manager, no
  setuid. ~40 MB image; drops to ~15 MB with musl static binary
  (`packaging/docker/Dockerfile.musl`).
- `/var/lib/roko` is the state volume; maps to the archive used
  by `roko state export` (§4). Backup/restore is cp-in, cp-out.
- `HEALTHCHECK` uses the same `/readyz` probe K8s does (§7.1), so
  Compose and Helm agree on health semantics.
- Secrets never bake into the image; they arrive via env (§3) or
  a mounted tmpfs.

### 7.3 systemd unit

For single-server teams on bare metal. A canonical unit file in
`packaging/systemd/` with restart policy, journald logging,
capability restrictions.

### 7.4 macOS launchd

For Roko running as a background service on a Mac.

### 7.5 WASM targets

For edge deployment, a `roko-wasm` binary compiled to wasm32-wasi
with the smallest possible feature set (no filesystem, no local
models, no plugin registry — just core + HTTP).

Each of these is a packaging artifact, not a code fork. Same Rust,
different front door.

## 8. Multi-tenancy

In single-server and clustered deployments, multiple users share
one Roko. Isolation requirements:

- **Memory isolation**: each tenant has their own Substrate scope;
  heuristics don't cross-contaminate unless explicitly shared.
- **Auth**: OIDC (Google Workspace, Microsoft, Okta) via a
  pluggable `Auth` trait. Plus API keys for machine users.
- **Quotas**: per-tenant token/dollar/episode budgets.
- **Role limits**: some roles or tools off by default for
  untrusted tenants.

Build on the existing role-auth in `roko-agent/src/safety/`. The
safety layer generalizes nicely from role → tenant × role.

### 8.1 Auth-header → tenant mapping

A concrete wiring that turns an inbound HTTP request into a
`TenantCtx` the kernel uses for substrate scoping and budget
enforcement. One `roko.toml` stanza, zero custom code:

```toml
[auth]
mode = "oidc"

[auth.oidc]
issuer        = "https://auth.example.com/"
audience      = "roko"
jwks_uri      = "https://auth.example.com/.well-known/jwks.json"
cache_ttl     = "10m"

# Claim-to-tenant projection. First matching rule wins.
[[auth.tenant_rules]]
# Canonical: an explicit tenant claim issued by the IdP.
claim   = "https://roko.dev/tenant"
tenant  = "${value}"
roles   = "${claims.roles[*]}"

[[auth.tenant_rules]]
# Google Workspace: hd claim is the hosted domain.
claim   = "hd"
tenant  = "workspace:${value}"
roles   = ["viewer"]       # default; overridden by group rules below

[[auth.tenant_rules]]
# Fallback: derive from email domain.
claim   = "email"
regex   = "^[^@]+@(?P<domain>.+)$"
tenant  = "email:${domain}"
roles   = ["viewer"]

[[auth.group_rules]]
# Map IdP groups to Roko roles.
group_claim = "groups"
mapping = {
  "roko-admins"      = "admin",
  "security-reviewers" = "reviewer",
  "engineering"      = "implementer",
}
```

Request flow:

1. Ingress validates `Authorization: Bearer <jwt>` against the
   JWKS (cached for `cache_ttl`).
2. The first `tenant_rule` that matches emits `TenantId`, e.g.
   `workspace:acme.com`. `group_rules` layer roles on top.
3. The `TenantCtx` is attached to the request and propagated as a
   span attribute (see §6.5) and as a prefix on every Substrate
   key: `tenant:workspace:acme.com/engram/<hash>`.
4. Quotas (§8) lookup uses `TenantId` as the aggregation key.

Machine users bypass OIDC via `Authorization: Bearer roko_pat_...`
(personal access tokens). PATs carry tenant and role at creation
time (`roko token create --tenant acme --role implementer`) and
live in the OS keychain / secret store (§3).

A header-only shortcut is supported for air-gapped deployments
behind a trusted reverse proxy:

```toml
[auth.headers]
tenant_header = "X-Roko-Tenant"
role_header   = "X-Roko-Role"
trust_chain   = ["10.0.0.0/8"]   # only accept from this CIDR
```

This path is off by default; enabling it explicitly is required
because it shifts trust to the upstream proxy.

## 9. The migration path for existing state

Users with `.roko/` directories from v1 need to keep their
episodes, plans, heuristics when upgrading to v2 (the
post-refinements kernel). Migration rules:

- **Automatic**: a version stamp in `.roko/meta` triggers
  migration on first run.
- **Non-destructive**: the old state is preserved as
  `.roko/.pre-v2/`.
- **Auditable**: `roko migrate --dry-run` shows what changes.
- **Reversible**: `roko migrate --rollback` if something goes
  wrong.

This is a 3-day project of careful work. It matters because
without it, every user has to choose between "stay on v1" and
"lose my accumulated state." Either is a bad choice for them.

## 10. Cost visibility

A Roko deployment is going to spend money on LLM APIs. Users need
clarity:

- **Live cost counter**: the TUI and Web UI show `$X.YZ spent
  this session`.
- **Budget caps**: `--budget 5` limits a run to $5 of API spend;
  the cascade router prefers cheaper models as budget depletes.
- **Per-task breakdown**: after a run, a table of cost per task,
  per agent, per model.
- **Historical**: `roko cost report --period 7d` for team-level
  understanding.

This is both a UX and a trust property. Users who can't see what
they're spending will either stop using the tool or feel
uncomfortable using it.

## 11. Air-gapped and on-prem realities

Some deployments cannot reach the public internet. For them:

- **Local models supported end-to-end** (already in `roko-agent`).
- **Plugin registry mirror**: an on-prem registry serving
  verified plugins.
- **Heuristic commons mirror**: imports from a curated,
  organization-internal commons rather than public.
- **Telemetry off by default**: no phoning home.

None of this requires a forked binary. Feature flags in the
profile.

## 12. The deployment UX pitch

"Single Rust binary, five shapes. Configuration is declarative.
State is portable. Secrets are layered. Observability is
standard-compatible. Upgrades are rolling. Multi-tenancy is
explicit. Air-gap works."

This is the DevOps side of the "world-class" claim. Deployment
can feel like a known-quantity infrastructure project (Postgres,
Redis, Nginx) rather than a novel-framework adoption risk. That
feeling is a moat of its own — it's what gets Roko approved by
platform teams rather than blocked by them.

## 13. What to ship first

Priority:

1. **Single-binary with profile config**. Already 80% there;
   needs the `profile` concept and a cleanup pass.
2. **`roko state export/import`**. Unlocks laptop-to-server
   migration. One week.
3. **Basic Docker image** (scratch + static musl binary).
   Two days.
4. **Docker Compose bundle** with Postgres + NATS. Three days.
5. **Helm chart**. One to two weeks.
6. **Secrets CLI and keychain integration**. One week.
7. **Cost visibility**. One week.

After these, Roko is deployable in all five shapes. Further
polish (WASM, air-gap, multi-tenancy maturation) is iterative.
