+++
id = "bug-88f99d"
kind = "bug"
title = "Docker image may bind roko serve to loopback: ROKO_BIND is not read by roko"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["deploy/docker"]
created = 2026-09-28
updated = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["Dockerfile:59", "Dockerfile:123", "docker/start-railway.sh"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

The Dockerfile sets `ENV ROKO_BIND=0.0.0.0` (`:59`, `:123`), but no Rust crate reads `ROKO_BIND`; only `docker/start-railway.sh` and `docker/roko.toml` mention it.
Unless the entrypoint translates it into serve's bind setting, `roko serve` listens on its loopback default and the published port is unreachable.
Confirm by running the image and probing the port; fix by using the setting serve actually reads and adding a container smoke test.
