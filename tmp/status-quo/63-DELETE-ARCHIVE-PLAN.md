# Delete And Archive Plan

This plan identifies what can be removed, archived, or quarantined after proof gates pass. It does not authorize deletion by itself.

## Rules

- Never delete a path solely because docs say it is stale.
- First mark as `supported`, `compat`, `experimental`, `archive`, or `delete-candidate`.
- Add a grep proof and a runtime proof for each deletion.
- Archive docs before code when docs are the only stale artifact.
- Feature-gate stubs before deleting if they may still be useful as design references.

## Candidate Table

| Area | Candidate | Recommendation | Required proof |
|---|---|---|---|
| Demos | `demo/demo-web` | Archive | README and CI no longer reference it; current demo-app covers the scenario. |
| Demos | `demo/demo-old` | Archive | Scenario scripts either ported to `demo/demo-resources` or marked historical. |
| Demos | `demo/demo-app/dist`, `demo/demo-app/node_modules`, `demo/demo-app/test-results`, Playwright reports | Ignore/generated cleanup | Confirm generated assets are not intended source. |
| Contracts | `contracts/out`, `contracts/cache`, generated ABI/build artifacts | Ignore/generated cleanup | Foundry can regenerate and source/tests do not depend on committed output. |
| Repo metadata | `.DS_Store` and other OS/editor residue | Delete candidate | `rg`/find proves no intentional source content. |
| Docker | `docker/gateway.Dockerfile` placeholder | Archive or label unsupported | `roko-gateway` is intentionally absent or implemented. |
| CI | `tui-parity-dry-run` references to missing tmp scripts | Repair or delete workflow | Referenced scripts are restored, moved to tracked tools, or workflow removed. |
| Runtime | `roko-cli/src/orchestrate.rs` legacy-only branches | Shrink/quarantine | Runner v2/Graph parity tests cover ported behavior. |
| Runtime | `roko-orchestrator` executor path | Keep or mark legacy | Decide if service factory remains shared runtime dependency. |
| Graph | `NoopCell`/`PassthroughCell` defaults on production paths | Remove from production registry | Graph smoke test proves real task/gate cells. |
| Dreams | `roko-dreams/src/phase2/*` | Feature-gate | Default build no longer advertises Phase 2 as live. |
| Daimon | `roko-daimon/src/phase2_stubs.rs` | Feature-gate | Public exports are behind feature or replaced by real implementation. |
| Runtime/state | Stale `roko-core` StateHub copy if unexported | Delete candidate | `rg "roko_core::.*StateHub|state_hub"` proves no public consumer. |
| Runtime/state | `roko-core/src/pulse_bus.rs` if superseded by runtime/event bus | Delete/quarantine candidate | Public API grep and event contract decision prove it is not canonical. |
| Runtime/layers | `roko-runtime -> roko-gate` dependency | Refactor candidate | Runtime gate interactions pass through traits/contracts. |
| CLI/docs | `surface_inventory.rs` stale command catalog | Regenerate or quarantine | Generated inventory matches Clap top-level/nested command tree. |
| Docs | Old root claims in `CLAUDE.md` | Update, not archive | Matches current execution/safety/API facts. |
| Tmp | Scratch migration logs | Archive | Source ranking marks them historical and no status-quo docs cite them as authority. |
| API | Compat route aliases | Deprecate, then remove | Frontend route manifest no longer calls alias. |
| Config | Legacy config aliases | Warn, migrate, then remove | `roko config migrate` converts old files and tests cover old examples. |

## Tmp Archive Policy

Keep:

- `tmp/status-quo`
- Authoritative newest design sources identified in [17-TMP-SOURCE-RANKING.md](17-TMP-SOURCE-RANKING.md)
- Fixtures used by tests
- Migration runners still referenced by CI or plans

Archive:

- Date-stamped scratch folders superseded by current status docs.
- Generated one-off audit outputs after their findings are copied into this pack.
- Old UI mockups not linked by current demo work.

Delete only after archive:

- Build outputs inside tmp.
- Duplicate generated reports with no unique source content.
- Broken symlinks or empty directories.

## Code Deletion Checklist

- [ ] `rg` proves no runtime references.
- [ ] Tests prove equivalent behavior on replacement path.
- [ ] Docs updated before or with deletion.
- [ ] Migration/compat note added when user data is affected.
- [ ] Release notes mention the removed path.
