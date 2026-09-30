+++
id = "bug-88f99d"
kind = "bug"
title = "Docker image may bind roko serve to loopback: ROKO_BIND is not read by roko"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["deploy/docker"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "94a72dcfc"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["Dockerfile:59", "Dockerfile:70", "Dockerfile:123", "docker/start-railway.sh", "crates/roko-core/src/config/serve.rs::default_bind", "crates/roko-serve/src/lib.rs::resolve_bind_with_port_env"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "awk '/AS distroless/,/^FROM debian/' Dockerfile | grep -qE '\"--bind\", *\"0\\.0\\.0\\.0\"|ROKO_SERVER_BIND|/workspace/roko.toml' || grep -rq 'ROKO_BIND' crates/ --include='*.rs'"

[closed]
at = 2026-09-30
commit = "94a72dcfc"
by = "commit trailer"
evidence = "94a72dcfc: the distroless stage ships docker/roko.toml as /workspace/roko.toml (bind 0.0.0.0:6677, acknowledge_public_risk) with WORKDIR /workspace, and drops the unread ROKO_BIND/ROKO_PORT; the static [[verify]] passes. The image was not built or run."
+++

The Dockerfile sets `ENV ROKO_BIND=0.0.0.0` (`:59`, `:123`), but no Rust crate reads `ROKO_BIND`; only `docker/start-railway.sh` and `docker/roko.toml` mention it.
Unless the entrypoint translates it into serve's bind setting, `roko serve` listens on its loopback default and the published port is unreachable.
Confirm by running the image and probing the port; fix by using the setting serve actually reads and adding a container smoke test.

Verified 2026-09-28 (static check against 3d0ee4d02): No Rust code reads ROKO_BIND; serve binds from `--bind` (crates/roko-cli/src/commands/server.rs:194-195) or `[server].bind`, default "127.0.0.1" (crates/roko-core/src/config/serve.rs:490), and only PORT is read from env (crates/roko-serve/src/lib.rs:284, keeps configured bind). The distroless target (Dockerfile:48-70) copies only the binary and runs `roko serve` with ENV ROKO_BIND=0.0.0.0 (:59) and no --bind/roko.toml, so it listens on loopback; the Railway target (Dockerfile:123/143) is fine because docker/start-railway.sh:6,255 translates ROKO_BIND into `--bind`.

Re-checked 2026-09-29: unchanged. The body's line references have moved: the default bind is now crates/roko-core/src/config/serve.rs:609 (default_bind), the PORT handling is crates/roko-serve/src/lib.rs:348, and the --bind flag is crates/roko-cli/src/main.rs:874 and crates/roko-cli/src/commands/server.rs:214. The Railway target is still fine through docker/start-railway.sh.

## Notes

- 2026-09-30 (wk-childenv): Fixed in `94a72dcfc` on `work/bug-17f0e4` by using the setting serve reads
  (`[server] bind` in `/workspace/roko.toml`), as the runtime target does. Not done: the container smoke test. Building
  the image runs cargo and npm, which this stream could not; a CI job that runs the distroless image with a provider key
  and probes `:6677/health` would close that gap.
