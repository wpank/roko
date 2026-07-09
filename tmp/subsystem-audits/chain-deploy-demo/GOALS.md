# Chain, Deploy & Demo: Goals

## End State

Deployment is one-command. Demo scenarios are runnable benchmarks. Chain integration (Phase 2+) provides witness anchoring for compliance/audit trails.

## Key Properties

- **One-command deploy**: `roko deploy railway|fly|docker` works end-to-end with minimal config.
- **Demo as benchmarks**: Demo scenarios double as regression benchmarks (tournament mode).
- **Chain witness anchoring**: Episode hashes anchored on-chain for tamper-proof audit trail (Phase 2+).
- **Daemon lifecycle**: Clean start/stop/status/logs with launchd/systemd integration.

## What Exists Today

- roko-chain: 20K LOC (16K+ dormant: reputation, marketplace, KORAI token, x402, identity, chain gates)
- Railway API integration (923 LOC in `roko-serve/src/deploy/railway_api.rs`)
- Fly.io deployment (`cmd_deploy_fly` in `roko-cli/src/commands/server.rs`)
- Daemon IPC (start/stop/status/logs/install)
- 5 demo scenarios with tournament benchmarks (separate `roko-demo` binary; not in `roko` CLI)
- Docker deployment support

## Gap

- Chain runtime integration deferred to Phase 2+ (needs blockchain backend)
- 16K+ LOC dormant chain code (reputation, marketplace, token economy, chain gates)
- Demo scenarios not automated as CI benchmarks
- Daemon not wired to launchd/systemd install

---

## Sources

- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/` (all files)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/deploy/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/server.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-demo/src/`
