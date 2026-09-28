+++
id = "bug-88f99d"
kind = "bug"
title = "Docker image may bind roko serve to loopback: ROKO_BIND is not read by roko"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["deploy/docker"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["Dockerfile:59", "Dockerfile:123", "docker/start-railway.sh", "Dockerfile:59 (distroless target only)"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

The Dockerfile sets `ENV ROKO_BIND=0.0.0.0` (`:59`, `:123`), but no Rust crate reads `ROKO_BIND`; only `docker/start-railway.sh` and `docker/roko.toml` mention it.
Unless the entrypoint translates it into serve's bind setting, `roko serve` listens on its loopback default and the published port is unreachable.
Confirm by running the image and probing the port; fix by using the setting serve actually reads and adding a container smoke test.

Verified 2026-09-28 (static check against 3d0ee4d02): No Rust code reads ROKO_BIND; serve binds from `--bind` (crates/roko-cli/src/commands/server.rs:194-195) or `[server].bind`, default "127.0.0.1" (crates/roko-core/src/config/serve.rs:490), and only PORT is read from env (crates/roko-serve/src/lib.rs:284, keeps configured bind). The distroless target (Dockerfile:48-70) copies only the binary and runs `roko serve` with ENV ROKO_BIND=0.0.0.0 (:59) and no --bind/roko.toml, so it listens on loopback; the Railway target (Dockerfile:123/143) is fine because docker/start-railway.sh:6,255 translates ROKO_BIND into `--bind`.
