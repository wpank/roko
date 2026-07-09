# Source Doc Coverage Ledger

This ledger tracks how much of `docs/v1`, `docs/v2`, and `docs/v2-depth` has been converted into current-state status docs.

The generated full manifest is [80-SOURCE-DOC-MANIFEST.md](80-SOURCE-DOC-MANIFEST.md).

## Source Counts

| Source | Count | Status |
|---|---:|---|
| `docs/v1` | 417 md files | Richest design source, but many status claims are stale. |
| `docs/v2` | 34 md files | Higher-level migration narrative; useful but not current proof. |
| `docs/v2-depth` | 185 md files | Deep target-state design; coverage is uneven by directory. |
| Total | 636 md files | All must be treated as source material, not implementation truth. |

## Current Coverage Shape

| Source area | Current status-pack coverage | Gap |
|---|---|---|
| v1 architecture/orchestration/agents/composition/verification | Covered by `02`, `03`, `11`, `14`, `30`-`36`, `47`, `54`. | Per-doc claim status is not exhaustive. |
| v1 learning/neuro/dreams/daimon/safety/tools | Covered by `33`, `35`, `38`-`41`, `48`, `56`. | Tool catalog and safety scope matrix need route/ACP/MCP proof. |
| v1 chain/identity/economy/deployment | Covered by `42`, `58`, `77`. | Contract/deploy docs still need live/mock/local authority tags. |
| v1 heartbeat/lifecycle/coordination/references | Partly covered by event/state docs. | Lifecycle and heartbeat claims need stricter target/current labeling. |
| v2 top-level docs | Covered by `15`, `18`, `19`, `65`. | Root v2 docs should eventually be rewritten from the status pack. |
| v2-depth graph/execution/runtime/learning/memory | Covered by `31`, `36`, `37`, `40`, `60`, `76`. | Graph examples and target cells need proof tags. |
| v2-depth connectivity/security/surfaces/deployment | Covered by `59`, `66`, `70`, `75`, `77`. | Security and ops runbooks still need generated route/deploy manifests. |
| v2-depth marketplace/registries/arenas/builtin catalog | Thin coverage across `42`, `52`, `58`. | Needs explicit local-vs-chain-vs-plugin ownership decisions before implementation. |
| v2-depth research prompts | Covered by `78`. | Strategy, deck, and memo claims must be fenced from code-roadmap truth. |
| v1 reference bibliography | Covered by `79`. | Use as provenance/rationale, not as status or priority proof. |

## Under-Covered Source Folders

| Folder | Why it matters | Action |
|---|---|---|
| `docs/v1/21-references` | Contains appendices and reference material that can hide stale command/path claims. | Grep for commands, env vars, routes, and state paths before docs convergence. |
| `docs/v1/20-technical-analysis` | Large risk/architecture analysis corpus. | Use as rationale only; do not import status labels without code proof. |
| `docs/v1/14-identity-economy` | Overlaps with chain, ISFR, jobs, registries, and marketplace. | Map each claim to local JSON, Solidity contract, Mirage, or live chain. |
| `docs/v2-depth/RESEARCH-PROMPT*.md` | Contains strategic narrative, pitch, demo, and market framing across 14 files. | Keep as strategy input; extract only claims that can be tied to code, proofs, or dated sources. |
| `docs/v2-depth/02-block` | Block/pulse/event semantics may conflict with current StateHub/EventBus. | Fold into event-contract work in `76`. |
| `docs/v2-depth/04-specializations` | INDEX-only despite real role-template/domain-profile code. | Write a current-state depth doc or mark the section intentionally spec-only. |
| `docs/v2-depth/06-trigger-system` | INDEX-only despite plugin event sources and test-only `Trigger` protocol impls. | Reconcile shipped event-source triggers with the v2 Trigger protocol. |
| `docs/v2-depth/08-extension-system` | INDEX-only and links to missing unified docs while `roko-plugin` and `roko-acp` exist. | Repair links and write current extension/ACP status. |
| `docs/v2-depth/13-builtin-catalog` | Builtin tools must align with `roko-std`, MCP, ACP, and safety. | Add generated tool manifest before expanding providers. |
| `docs/v2-depth/15-marketplace` | Target-state marketplace claims overlap with local jobs/deploy. | Keep target-labeled until chain-backed job settlement is live. |
| `docs/v2-depth/19-arenas` | Sparse but conceptually broad coordination model. | Defer unless adopted into runtime contract. |

## Checklist

- [x] Generate a source-doc manifest with path, title, source era/status tag, and status-pack owner.
- [ ] Mark every source doc as `current`, `partially-current`, `target-state`, `stale`, or `archive`.
- [ ] Add owners for the 14 research prompts and 27 v1 reference docs before using them in roadmap work.
- [ ] Add stale-claim grep categories: commands, routes, env vars, `.roko` paths, crate names, feature flags, proof commands.
- [ ] Repair `docs/v2-depth/INDEX.md` counts and the missing `docs/unified/08-EXTENSION-SYSTEM.md` link.
- [ ] Make docs convergence update root docs from `tmp/status-quo`, not directly from v1/v2 labels.
- [ ] Keep v2-depth target-state docs, but add current-state banners where implementation lags.
