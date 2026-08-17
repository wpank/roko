# 15 — Marketplace and Sharing

> Publish, discover, install, fork, rate, and attribute Cells, Graphs, Racks, and Knowledge Bundles in a community marketplace. Composition over isolation. Trust by evidence. DAW composability: Criteria as plugins, Profiles as presets, fork as fundamental.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal lineage, content addressing), [02-CELL](02-CELL.md) (Cell protocol, Verify), [04-SPECIALIZATIONS](04-SPECIALIZATIONS.md) (Rack, Slot, Macro), [09-TELEMETRY](09-TELEMETRY.md) (UsageLens), [14-CONFIG-AND-AUTHORING](14-CONFIG-AND-AUTHORING.md) (5-tier SPI, TOML schemas), [17-SECURITY-MODEL](17-SECURITY-MODEL.md) (capability intersection), [18-ON-CHAIN-REGISTRIES](18-ON-CHAIN-REGISTRIES.md) (ERC-8004 attribution)

---

## 1. Overview

Every byproduct of using Roko is a shareable, composable primitive. The marketplace turns local Cells, Graphs, Racks, and Knowledge Bundles into community artifacts -- the DAW preset / Splice sample / Figma Community pattern applied to agent orchestration.

Design principles:
- **Composition over isolation** -- artifacts reference each other; forking preserves lineage.
- **Trust by evidence** -- Verified Run badges, gate pass rates, and community validation replace trust by authority.
- **Capability transparency** -- every artifact's permission requirements are visible before install.
- **Observation built in** -- UsageLens ([doc-09](09-TELEMETRY.md)) tracks usage metrics for every published artifact; creators see analytics.
- **Creator-first economics** -- transparent take-rates, creators own customer relationships, all metrics published.
- **Fork as fundamental** -- forking is not a failure of the original; it is the primary mechanism for adaptation.

---

## 2. Marketplace Economics

The marketplace exists because transparent economics solve the problem that killed the GPT Store: opaque revenue sharing without reputation infrastructure destroys creator trust. Every metric is public. Every fee is documented.

### 2.1 Take-rate structure

| Revenue band | Take-rate | Precedent |
|---|---|---|
| First $1M lifetime creator revenue | **0%** | Shopify ($0 until you succeed) |
| Above $1M lifetime | **12-15%** | Unreal Engine (5% after $1M) |

The 0% band is per-creator, lifetime. A creator who earns $999,999 pays nothing. At $1,000,001, they pay 12-15% on the $1 above the threshold -- not retroactively on the first $1M.

Rationale: the GPT Store's opaque rev-share produced median creator earnings of <$100/quarter (confirmed by multiple creator reports). The failure mode is not the percentage -- it is the opacity and the lack of creator control. Shopify's zero-take-rate-until-you-succeed pattern bootstraps supply. Unreal's graduated model sustains the platform at scale.

### 2.2 Revenue model comparison

| Platform | Take-rate | Creator owns customer? | Metrics public? | Outcome |
|---|---|---|---|---|
| **GPT Store** | Opaque (OpenAI sets rev share) | No | No | Median <$100/quarter |
| **npm** | 0% (free distribution) | N/A (OSS) | Download counts only | Sustainability crisis |
| **Unreal Marketplace** | 12% | Partial | Yes (sales, ratings) | Healthy creator economy |
| **Unity Asset Store** | 30% | No | Partial | Creator complaints |
| **Roko Marketplace** | 0% to $1M, 12-15% above | **Yes** | **All metrics public** | Target: sustainable creator economy |

### 2.3 Published metrics (mandatory)

Every published artifact surfaces these metrics publicly. Creators cannot hide them. Consumers cannot be misled.

| Metric | Visibility | Update frequency |
|---|---|---|
| Installs (lifetime, 30d, 7d) | Public | Hourly |
| Active runs (30d) | Public | Hourly |
| Fork count (lifetime) | Public | Hourly |
| Gate pass rates (30d) | Public | Hourly |
| Mean cost per run (30d) | Public | Daily |
| Mean duration per run (30d) | Public | Daily |
| Error rate (30d) | Public | Daily |
| Revenue (creator only) | Creator-only | Real-time |

### 2.4 Creator owns the customer

Creators can:
- Export their installer list (email/wallet with opt-in consent).
- Send release notes and changelogs directly to installers.
- Offer paid tiers (free/pro/team) with feature gating via Macros ([doc-14](14-CONFIG-AND-AUTHORING.md)).
- Run their own support channels (linked from the artifact page).

The marketplace is a distribution channel, not a walled garden. If a creator wants to take their artifacts elsewhere, they can.

### 2.5 On-chain attribution via ERC-8004

Fork chains and provenance are anchored on-chain via ERC-8004 ([doc-18](18-ON-CHAIN-REGISTRIES.md)):

- **Fork chain provenance**: when an artifact is forked, the fork relationship is recorded as an ERC-8004 attestation. The chain of `forked_from` references forms a Merkle DAG with cryptographic provenance.
- **Reputation flows upstream**: when a fork earns reputation (installs, runs, gate pass rates), a portion flows upstream to the original author. Attribution is economic, not just cosmetic.
- **ZK-attested quality**: an artifact's gate pass rate can be ZK-attested (proving the rate without revealing individual run data) and anchored in its ERC-8004 record. Third parties verify quality claims without trusting the marketplace backend.

### 2.6 Anti-GPT-Store lessons

The marketplace design deliberately avoids patterns that caused the GPT Store to underperform:

| GPT Store failure | Roko marketplace response |
|---|---|
| Opaque revenue sharing | Published take-rate schedule, 0% to $1M |
| No creator analytics | Full UsageLens analytics for every artifact |
| No forking | Fork as fundamental operation |
| Platform owns the customer | Creator owns the customer (export, direct messaging) |
| No quality signal beyond star ratings | Gate pass rates, active runs, Verified Run badges |
| Discoverability by editorial fiat | Algorithmic trending + editorial curation + search |
| No composability | Artifacts compose via Slots, Macros, sub-Graph references |
| Invite-only revenue sharing | All creators eligible from day one |

---

## 3. DAW Composability

The marketplace is modeled on DAW (Digital Audio Workstation) plugin ecosystems: Criteria are plugins, Profiles are presets, fork is the remix.

### 3.1 Criteria as plugins

A Verify Cell ([doc-02](02-CELL.md), [doc-13](13-BUILTIN-BLOCK-CATALOG.md)) is a plugin that evaluates one dimension of quality. Just as a DAW audio plugin slots into any channel strip, a Criterion Cell slots into any Verify pipeline:

- Criteria are individually publishable marketplace artifacts.
- A Verify Graph is a composition of Criterion Cells -- a "rack" of quality checks.
- Users install individual Criteria from the marketplace and wire them into their own verification pipelines.
- Custom Criteria are just Cells that implement the Verify protocol. No special machinery.

### 3.2 Profiles as presets

A **Profile** is a Rack ([doc-04](04-SPECIALIZATIONS.md)) that computes a Verdict: a pre-wired composition of Criteria with tuned Macros. Just as a DAW preset is a saved configuration of plugin parameters:

- Profiles are publishable marketplace artifacts.
- A Profile bundles: which Criteria to run, in what order, with what thresholds, with what weights for soft criteria.
- Users install a Profile, adjust its Macros (thresholds, strictness, model selection), and use it as their verification pipeline.
- "Strict code review," "fast prototype check," and "security audit" are all Profiles over the same Criteria library.

### 3.3 Fork as fundamental

Forking is not a negative signal (someone copied your work). It is the primary adaptation mechanism:

- Every marketplace artifact is forkable with one command.
- Fork preserves lineage (visible chain on the artifact page).
- Fork chains are first-class marketplace navigation (browse a fork tree, diff between any two nodes).
- On-chain attribution via ERC-8004 means forks that succeed generate reputation for the original author.

### 3.4 Composability hierarchy

```
Criterion (plugin)
  -> Profile (preset: N Criteria + tuned Macros)
    -> Rack (parameterized Graph with Slots + Macros)
      -> Graph (full state graph of Cells)
        -> Space Template (skeleton: Graph + Triggers + capabilities + models)
```

Each level composes the level below. The marketplace supports publishing and forking at every level.

---

## 4. Publishable Artifact Types

11 artifact kinds, in launch order (smallest / lowest-risk first):

| # | Kind | Risk | Forking | Notes |
|---|---|---|---|---|
| 1 | **Snippet** | None (paste into editor) | Copy | Saved sub-graph fragments |
| 2 | **Prompt Preset** | None (system prompt + role + model) | Copy | Smallest LLM artifact. Tier 1 SPI. |
| 3 | **Criterion** | None (single Verify Cell) | Copy | Individual quality check plugin |
| 4 | **Cell** (composition tier) | None (pure composition) | Copy | No new execution surface |
| 5 | **Cell** (script / WASM tier) | Sandboxed execution | Copy | Requires capability disclosure |
| 6 | **Profile** | Composition of Criteria | Copy | Verification preset (DAW preset). Also: full cognitive posture ([doc-14](14-CONFIG-AND-AUTHORING.md)). |
| 7 | **Graph** | Composition of Cells | Copy | Full state graph with nodes, edges, policy |
| 8 | **Rack** | Graph + Macros + Slots | Copy | Parameterized Graph |
| 9 | **Trigger Binding** | Event config + bound Graph | Copy | Trigger + Graph pairing |
| 10 | **Space Template** | Workspace skeleton | Copy | Capabilities, models, deploy targets |
| 11 | **Knowledge Bundle** | Curated knowledge Signals | Reference + Fork | Are.na-style composition by reference |

v1: Snippets, Prompt Presets, Criteria, Cells, Profiles, Graphs. v1.1: Racks, Trigger Bindings. v1.2: Space Templates, Knowledge Bundles.

---

## 5. Identity and Attribution

### 5.1 Publisher identity

Marketplace identity uses GitHub OAuth for baseline reputation. Optional Google OAuth as alternate. Email-only accounts cannot publish.

Each artifact carries `publisher: "@github_handle"`. Anonymous publishing is not supported in v1.

### 5.2 Artifact identity

Marketplace artifact references use `@publisher/name@version`:

```
@my-org/markdown-classify@1.2.3
@nunchi/doc-ingest@1.0.0
@wpank/strict-pr-review@2.0.0-rc.1
```

### 5.3 Lineage

Every fork records lineage through `Signal.source` (the same content-addressed lineage that Signals carry, [doc-01](01-SIGNAL.md)):

```rust
pub struct ArtifactLineage {
    pub forked_from: Option<ArtifactRef>,
    pub composed_from: Vec<ArtifactRef>,     // when composing N artifacts
    pub forked_at: DateTime<Utc>,
    pub onchain_attestation: Option<ERC8004Ref>,  // on-chain provenance anchor
}
```

Lineage is publicly visible. On-chain lineage (via ERC-8004) provides cryptographic proof that cannot be disputed.

### 5.4 License

Default: **CC BY 4.0** for all artifact kinds. Other supported: CC BY-SA 4.0, CC0 1.0, MIT, Apache-2.0. License is required at publish. Knowledge Bundles default to CC BY-SA 4.0 (share-alike preserves attribution chains).

---

## 6. Publish Flow

```
$ roko market publish doc-ingest
Validating doc-ingest@1.0.0...
  Schema valid
  All references resolve
  Capabilities declared: FsRead, FsWrite, Llm, Net
  License: CC-BY-4.0
  No screenshot bundle (recommended for visibility)
  No fixture for "Verified Run" badge

Capabilities your artifact requires (consumers will see this):
  FsRead              "any path"   (recommend: restrict to specific patterns)
  FsWrite             "any path"   (recommend: restrict)
  Llm                 "any provider"
  Net                 "*"          (recommend: list specific domains)
[continue anyway / restrict capabilities / abort]

Visibility:  ( ) public   ( ) org-only   (*) private (publish later)
Tags:        doc, ingest, authoring
README:      .roko/graphs/doc-ingest.README.md  (auto-detected)
Sample input: .roko/graphs/doc-ingest.fixtures/  (auto-detected, 2 fixtures)

Publish? [Y/n]
Publishing...
  Bundling artifact (graph.toml + block deps + README + fixtures)
  Computing checksum: blake3:abc...
  Uploading to marketplace.roko.dev
  Anchoring lineage to ERC-8004 (if creator opted in)
  Indexing...
Published as @wpank/doc-ingest@1.0.0
  https://market.roko.dev/@wpank/doc-ingest
```

Publish is also available via Dashboard ("Publish" button in the visual editor, [doc-16](16-SURFACES.md)), TUI (`p` from F2 Graphs), and as a meta-Graph.

---

## 7. Browse and Discovery

### 7.1 Faceted browse

Three primary tabs:
- **Featured** -- editorial picks; refreshed weekly
- **Trending** -- install velocity over the last 7 days (driven by TrendLens slope on UsageLens data)
- **All** -- full search

Filters: Kind, Category, Tags, Capabilities required, Verified Run badge, Gate pass rate (minimum), License, Publisher, Recency, Revenue band (free, paid-with-free-tier, paid-only).

### 7.2 Preview ("Tinker Mode")

In-place preview is the single biggest unlock. Marketplace artifacts have a "Preview" button that runs against the artifact's bundled sample input in a sandboxed worker, returns within ~30s, displays the output in-page. No install required.

For Graphs that take >30s, the preview shows a recorded "last successful run" trace as a fallback. For Snippets and Prompt Presets, preview is instant.

---

## 8. Install Flow with Capability Disclosure

```
$ roko market install @wpank/doc-ingest@^1
Resolving @wpank/doc-ingest@^1 -> 1.0.0...
Inspecting capabilities:
  This Graph requires:
    FsRead    any path     -> granted by Space
    FsWrite   any path     -> granted by Space
    Llm                    -> granted by Space
    Net        api.perplexity.ai, arxiv.org -> Space grants Net.* (covered)
  All capabilities covered. Continue? [Y/n]
Downloading bundle...  (124 KB)
  Verifying checksum: blake3:abc...
  Verifying signature (publisher: @wpank)
  Verifying on-chain lineage (if available)
Installing Cell dependencies (8 blocks)...
  fs-walk@1.0.4
  markdown-classify@1.0.0
  ...
Registering with Space...
Installed as @wpank/doc-ingest@1.0.0
  Run with: roko run @wpank/doc-ingest --input source_dir=...
```

If a capability is not covered, the install pauses for explicit grant:

```
This artifact requires:
  Shell  ["cargo", "git", "rustc"]
Space currently grants: Shell = false
Grant `Shell ["cargo", "git", "rustc"]` to this Space? (y/N/configure)
```

Granting writes to `workspace.toml` capabilities ([doc-14](14-CONFIG-AND-AUTHORING.md)); the user can revoke later.

---

## 9. Version Resolution, Lockfile, Yanking, Deprecation

### Semver

All artifacts use semver. Pre-release supported. The marketplace surfaces the highest stable version by default; pre-releases via "Show pre-releases" toggle.

### Lockfile

`<workspace>/.roko/marketplace.lock` pins exact versions and checksums for installed artifacts. Ensures reproducible installs across machines.

### Yanking

A publisher can yank a version (mark broken / vulnerable / wrong). Yanked versions remain installable for users who already pinned to them but are not shown in browse and emit a warning on install.

### Deprecation

A publisher can deprecate an artifact (replaced by another, no longer maintained). Deprecation surfaces a banner suggesting the replacement:

```toml
[deprecation]
since       = "2026-06-01"
reason      = "Replaced by improved engine"
replacement = "@wpank/doc-ingest-v2@^1"
```

### Vulnerability disclosure

Published artifacts that depend on a vulnerable Cell receive an automatic banner. Maintainers receive a notification. Consumers see a warning on install/run.

---

## 10. Forking with Lineage Chain Visualization

### Local fork

```
$ roko market fork @wpank/doc-ingest@1.0.0 my-doc-ingest
Forking @wpank/doc-ingest@1.0.0...
  Local name: my-doc-ingest
  Lineage:    @wpank/doc-ingest@1.0.0
  Cell pins relaxed: ^1 -> ^1 (no change; pinned ones unpinned)
  Files written to: .roko/graphs/my-doc-ingest.toml
Forked. Edit with: roko graph edit my-doc-ingest
```

### Fork chain visualization

```
@alice/code-review@1.0.0
   -> @bob/strict-code-review@2.0.0      (changed: model=opus, strictness=high)
        -> @carol/security-review@1.5.0  (changed: focus_areas=security)
             -> @wpank/strict-security@2.1.0  (you are here)
```

Click any node to view that version. Diff between any two versions via the graph-compare Cell. On-chain lineage (ERC-8004) provides cryptographic proof of the chain.

### Composition by reference (vs fork)

For Knowledge Bundles, prompts, and other small artifacts, **composition by reference** is preferred over fork:

```toml
[[graph.knowledge_bundle]]
ref = "@nunchi/safety-knowledge@^1"
```

The bundle is referenced, not copied. Updates to the upstream bundle reach the consumer (with a compatibility check). Forking is still possible if the consumer wants to diverge.

---

## 11. Rating and Trust

### Bayesian quality signals

- **Install count** (lifetime, 30d, 7d)
- **Active runs** (last 30d) -- strongest quality signal (Splice retention pattern)
- **Fork count** -- a fork is a positive signal (someone found it useful enough to extend)
- **Comment quality** (length, threaded depth)
- **Rating** -- thumbs-up / thumbs-down by verified installers only (accounts that have run the artifact at least once)

Aggregate "would recommend" shown on artifacts with N>=10 ratings; below that, only raw counts.

### Gate pass rates

For Graphs that include Verify Cells ([doc-13](13-BUILTIN-BLOCK-CATALOG.md)), the marketplace tracks aggregate gate pass rates from community runs. A Graph at 95% surfaces higher than one at 60%. Quality signal derived from the verification system itself.

### Community validation

Strong pass rates + active runs earn "Community Validated" status. Distinct from Verified Run (CI-level) -- Community Validated means real users running it successfully.

### On-chain reputation via ERC-8004

Publisher reputation is also reflected on-chain ([doc-18](18-ON-CHAIN-REGISTRIES.md)):
- TraceRank reputation from verified work.
- Fork-chain attestations showing provenance graph.
- ZK-attested gate pass rates for cryptographic quality verification.

Neither on-chain nor off-chain metrics can be faked independently.

### Editorial "Featured"

Weekly editorial pass selects up to 5 artifacts. Selection criteria (transparently documented): useful in real Graphs, well-documented, Verified Run badge, sensible capability disclosures.

### Creator analytics via UsageLens

The UsageLens ([doc-09](09-TELEMETRY.md)) powers marketplace creator analytics:

| Metric | Description | Period |
|---|---|---|
| Installs | New installs | 7d, 30d, lifetime |
| Active runs | Distinct runs | 7d, 30d |
| Forks | New forks created | 7d, 30d, lifetime |
| Error rate | % of runs that failed | 7d, 30d |
| Avg cost | Mean USD cost per run | 30d |
| Avg duration | Mean wall-clock per run | 30d |
| Gate pass rate | Verify pass % | 30d |
| Retention | % of installers who ran more than once | 30d |
| Revenue | Creator revenue (take-rate applied) | 7d, 30d, lifetime |
| Upstream attribution | Revenue from downstream forks | 30d, lifetime |

UsageLens feeds TrendLens and AnomalyLens: install velocity drives "Trending" ranking, error rate anomalies alert creators on spikes, cost drift detects upstream model price changes.

---

## 12. Anti-Spam and Safety

- New-account publishing throttle: 1 publish/day for first 30 days; 5/day thereafter for unverified; 50/day for verified accounts
- Static analysis on WASM Cells (banned imports, fuel limits, memory limits)
- LLM-based duplicate detection on prompts and configs; near-duplicates surface a "similar to @x/y" link
- One-click "Report" on any artifact; reports go to moderation
- Featured / unfeatured / suspended states transparently logged
- Marketplace Cells must be Composition or WASM tier (not Script unless verified publisher). Native Rust not directly publishable. Security boundary: marketplace = sandboxed ([doc-17](17-SECURITY-MODEL.md)).

---

## 13. CLI Surface

```
roko market browse [--query <q>] [--tag <t>] [--kind <k>] [--featured]
roko market show <ref>
roko market install <ref>
roko market uninstall <ref>
roko market upgrade [<ref>]                  # all installed if no ref
roko market list-installed
roko market fork <ref> [<new-name>]
roko market publish <local-name>
roko market unpublish <ref>                  # only your own; soft-delete (yank)
roko market deprecate <ref> [--replacement <ref>]
roko market mirror <url>                     # add alternate marketplace endpoint
roko market verify <ref>                     # explicit checksum + signature check
roko market analytics <ref>                  # creator analytics for your published artifact
roko market revenue [--period 30d]           # creator revenue summary
```

---

## 14. Backend Service

`roko-marketplace` (service):
- **Storage**: S3 / Tigris for artifact bundles; Postgres for metadata; Redis for trending counts
- **Auth**: GitHub OAuth + JWT for sessions
- **API**: REST `/api/v1/artifacts`, `/publishers`, `/comments`, `/installs`
- **CI**: GitHub Actions runner that re-runs published fixtures every release; emits `verified_run` records
- **CDN**: artifact bundles served via CloudFront / Bunny; checksums verified client-side
- **On-chain**: ERC-8004 lineage attestation service (optional, creator opt-in)
- **Mirroring**: optional self-hosted marketplace for orgs (`@my-org/...` namespace on private endpoint)

---

## 15. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `roko market publish` validates capabilities, signs, uploads, indexes | End-to-end test against staging |
| `roko market install` resolves semver, downloads, verifies checksum, prompts on capability gaps | Install + capability-gap test |
| WASM Cells execute under fuel + memory limits; over-limit Cells fail closed | Resource-limit test |
| Browse facets work: filter by kind, category, tag, capability, license, gate pass rate | Faceted query test |
| Verified Run CI re-runs fixtures and emits badge state | CI integration test |
| Fork chain renders correctly with all ancestors clickable | Visual + DB test |
| Lineage walk works across the marketplace API | API test |
| Yanked versions warn on install but install for already-pinned | Yank test |
| Anti-spam throttling: new account cannot publish > 1/day for first 30 days | Account-limit test |
| UsageLens metrics appear on creator analytics page within 1h of event | Metrics pipeline test |
| TrendLens drives "Trending" sort correctly | Trending algorithm test |
| 0% take-rate applied for creator revenue under $1M lifetime | Revenue calculation test |
| 12-15% take-rate applied above $1M threshold, non-retroactive | Revenue calculation test |
| All mandatory metrics published | Metrics completeness test |
| On-chain lineage attestation anchored in ERC-8004 for opted-in creators | Chain attestation test |
| Fork generates upstream reputation credit for original author | Attribution flow test |
| Criterion artifacts installable as Verify pipeline plugins | Round-trip: install, wire into Profile, run Verify |
| Profile artifacts apply as verification presets with adjustable Macros | Round-trip: install, adjust, run |
| Creator can export installer list with opt-in consent | Export + consent test |
| 11 artifact types each publish and install correctly | Per-type round-trip test |
| Preview ("Tinker Mode") runs against fixtures within 30s | Latency test |
