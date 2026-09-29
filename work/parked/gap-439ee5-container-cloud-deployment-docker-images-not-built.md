+++
id = "gap-439ee5"
kind = "gap"
title = "Container & cloud deployment: Docker images not built, Fly.io not configured, Railway worker template pending"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/deploy"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/32-DEPLOYMENT.md:3"
discovered_from = "audit:docs/v3/32-DEPLOYMENT.md:3"
anchors = ["roko deploy docker", "roko deploy fly", "roko deploy railway"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Deployment docs: Docker images/Compose/CI are specified but not built (`roko deploy docker` only emits a bundle); Fly.io is designed but not configured; Railway worker template depends on a worker registration API in roko-serve.

Imported without verification from:
- `docs/v3/32-DEPLOYMENT.md:3`
- `docs/v3/depth/32-deployment/docker.md:306`
- `docs/v3/depth/32-deployment/cloud-fly-io.md:256`
- `docs/v3/depth/32-deployment/cloud-railway.md:253`
- `docs/v2/25-DEPLOYMENT.md:5`

How to verify: Run `roko deploy docker --help` and inspect output; check for Dockerfile/fly.toml/.github workflows building images.
