# Daemon and Deployment Issues

## High

### Port mismatch between Dockerfile and fly.toml
- `docker/roko.Dockerfile:45`: `ENV PORT=3000`.
- `fly.toml:7-9`: `internal_port = 3000`.
- Server default: `6677`. Embedded `FLY_TOML_TEMPLATE` (server.rs:755): `internal_port = 6677`.
- If `PORT=3000` set but not consumed by `roko serve`, health checks probe wrong port.
- Dockerfile creates `/workspace/.roko` but embedded template mounts at `/data/.roko`.

### SIGTERM not handled in `roko serve`
- `roko-serve/lib.rs:509-521`: Only `tokio::signal::ctrl_c()`. Docker/systemd send SIGTERM.
- Container waits full grace period (10s) then SIGKILL. State never flushed.
- Contrast: daemon.rs correctly combines SIGTERM + SIGINT.

### Daemon binds `0.0.0.0` ignoring `[server].bind`
- `daemon.rs:1349`: `format!("0.0.0.0:{port}")`. Bypasses intentional `127.0.0.1` default and `validate_bind_safety`.

### Docker deploy skips `docker push`
- `commands/server.rs:427-444`: Builds and tags image, returns success. Image only local. Downstream pulls get stale/missing image.

## Medium

### `roko deploy fly` overwrites fly.toml with hardcoded template
- `commands/server.rs:487-492`: Unconditional overwrite with static template. Destroys operator customizations (regions, app name, scaling).

### Daemon PID file not cleaned on abnormal exit
- `daemon.rs:302-313`: SIGKILL/crash → `daemon.json` persists with `state: "running"`. `roko daemon status` reports running until next start attempt.

### Daemon log files overwritten on each spawn
- `daemon.rs:755-758`: `StdFile::create` truncates. Post-mortem debugging impossible after crash+restart.

### `degraded` health returns HTTP 200
- `routes/status/health.rs:83-96`: Platform health checks never detect degraded state. Traffic keeps routing to unhealthy machine.

### Retention disabled when cold storage disabled
- `roko-serve/lib.rs:2096-2102`: `apply_retention` only runs inside cold archival timer. No cold storage → no retention at all. Files grow forever.
