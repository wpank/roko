# Agent and gate Cargo environments diverge

- Severity: high
- Area: verification / build performance
- Reproduced: 2026-07-11, task `SH01-T02`

The agent ran the declared `roko-cli` tests, then the gate immediately ran the identical command. The gate still spent 189 seconds rebuilding most workspace crates. The worktree `target` directory grew to roughly 13 GB.

Process inspection showed that the gate inherits `CARGO_INCREMENTAL=0` and `CARGO_BUILD_JOBS=2`, while the Codex provider child inherits neither. Cargo produced different `roko_cli` artifact hashes and profile fingerprints for the two executions, so the agent and gate populate separate build universes. Cache reuse works within each environment but not across the agent-to-gate boundary.

Provider children and gates must use one canonical build environment: target directory, incremental setting, jobs, rustflags, wrappers, and profile. Log a stable build-environment fingerprint at both dispatches and add an integration test that an agent verify followed by its gate reuses the same Cargo artifacts.
