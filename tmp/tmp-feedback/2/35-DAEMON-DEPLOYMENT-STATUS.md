# Daemon & Deployment: Mostly Complete, Docker Push Missing

## Problem

The deployment story is more complete than expected. Daemon, Railway, and Docker all work
with minor gaps.

## Status

| Component | Status | Notes |
|-----------|--------|-------|
| Daemon (launchd) | **Working** | `roko daemon install` creates launchd plist on macOS |
| Daemon (systemd) | **Working** | `roko daemon install` creates systemd unit on Linux |
| Daemon lifecycle | **Working** | start/stop/status/logs all functional |
| Railway deploy | **Working** | `roko deploy railway` — real Dockerfile + railway.toml |
| Fly.io deploy | **Working** | `roko deploy fly` — fly.toml generation |
| Docker deploy | **Partial** | Builds image but missing `docker push` step |
| Worker mode | **Working** | `roko worker` for deployed instances |

## Docker Push Gap

**File:** `crates/roko-cli/src/commands/deploy.rs`

```rust
pub async fn cmd_docker(opts: &DockerOpts) -> Result<()> {
    // Builds the Docker image
    let status = Command::new("docker")
        .args(&["build", "-t", &tag, "."])
        .status()
        .await?;

    // Missing: push step
    // Should have:
    // if opts.push {
    //     Command::new("docker")
    //         .args(&["push", &tag])
    //         .status()
    //         .await?;
    // }

    Ok(())
}
```

The `--push` flag is defined in the CLI args but not wired to the push command.

## Fix

### Fix 1: Wire docker push (~5 min)

**File:** `crates/roko-cli/src/commands/deploy.rs`

```rust
if opts.push {
    println!("Pushing image: {tag}");
    let status = Command::new("docker")
        .args(&["push", &tag])
        .status()
        .await?;
    if !status.success() {
        anyhow::bail!("Docker push failed");
    }
}
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/commands/deploy.rs` | Wire `--push` flag to docker push |

## Priority

**P2** — Deployment works for the primary targets (Railway, daemon). Docker push is a
nice-to-have for custom deployments.
