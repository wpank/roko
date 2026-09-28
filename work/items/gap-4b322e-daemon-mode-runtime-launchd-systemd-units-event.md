+++
id = "gap-4b322e"
kind = "gap"
title = "Daemon mode runtime: launchd/systemd units, event loop, file-watch/cron subscriptions, multi-repo"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/daemon"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/32-deployment/daemon-launchd-macos.md:277"
discovered_from = "audit:docs/v3/depth/32-deployment/daemon-launchd-macos.md:277"
anchors = ["roko daemon install", "roko config subscriptions"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
v3 depth docs: launchd plist generation untested e2e, systemd unit generation not implemented, notify/cron not connected to the daemon event loop, subscription runner/IPC unwired, multi-repo coordination blocked on daemon. v2 25-DEPLOYMENT instead claims systemd IMPLEMENTED.

Imported without verification from:
- `docs/v3/depth/32-deployment/daemon-launchd-macos.md:277`
- `docs/v3/depth/32-deployment/daemon-systemd-linux.md:304`
- `docs/v3/depth/32-deployment/subscription-configuration.md:290`
- `docs/v3/depth/32-deployment/multi-repo-coordination.md:255`
- `docs/v2/25-DEPLOYMENT.md:5`

How to verify: Run `roko daemon install --help`/`status`; grep daemon code for notify/cron usage; resolve the v2 vs v3 contradiction.
