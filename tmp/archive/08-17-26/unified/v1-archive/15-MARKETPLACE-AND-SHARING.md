# 15 — Marketplace and Sharing

> Publish, discover, install, fork, rate, and attribute Blocks, Graphs, Racks, and Knowledge Bundles in a community marketplace. DAW composability: Criteria as plugins, Profiles as presets, fork as fundamental.

**Source**: wf-12 (Marketplace & Sharing), visual-gate2 PRD-06 (DAW composability), updated to unified vocabulary with UsageLens integration, creator analytics, marketplace economics, and on-chain attribution.

---

## 1. Overview

Every byproduct of using Roko is a shareable, composable primitive. The marketplace turns local Blocks, Graphs, Racks, and Knowledge Bundles into community artifacts — the DAW preset / Splice sample / Figma Community pattern applied to agent orchestration.

Design principles:
- **Composition over isolation** — artifacts reference each other; forking preserves lineage.
- **Trust by evidence** — Verified Run badges, gate pass rates, and community validation replace trust by authority.
- **Capability transparency** — every artifact's permission requirements are visible before install.
- **Observation built in** — UsageLens tracks usage metrics for every published artifact; creators see analytics.
- **Creator-first economics** — transparent take-rates, creators own customer relationships, all metrics published.
- **Fork as fundamental** — forking is not a failure of the original; it is the primary mechanism for adaptation.

---

## 2. Marketplace Economics

The marketplace exists because transparent economics solve the problem that killed the GPT Store: opaque revenue sharing without reputation infrastructure destroys creator trust. Every metric is public. Every fee is documented.

### 2.1 Take-rate structure

| Revenue band | Take-rate | Precedent |
|---|---|---|
| First $1M lifetime creator revenue | **0%** | Shopify ($0 until you succeed) |
| Above $1M lifetime | **12-15%** | Unreal Engine (5% after $1M) |

The 0% band is per-creator, lifetime. A creator who earns $999,999 pays nothing. At $1,000,001, they pay 12-15% on the $1 above the threshold (not retroactively on the first $1M).

Rationale: the GPT Store's opaque rev-share produced median creator earnings of <$100/quarter. The failure mode is not the percentage — it is the opacity and the lack of creator control. Shopify's zero-take-rate-until-you-succeed pattern bootstraps supply. Unreal's graduated model sustains the platform at scale.

### 2.2 Revenue model comparison

| Platform | Take-rate | Creator owns customer? | Metrics public? | Outcome |
|---|---|---|---|---|
| **GPT Store** | Opaque (OpenAI sets rev share) | No | No | Median <$100/quarter |
| **npm** | 0% (free distribution) | N/A (OSS) | Download counts only | Sustainability crisis |
| **Unreal Marketplace** | 12% | Partial | Yes (sales, ratings) | Healthy creator economy |
| **Unity Asset Store** | 30% | No | Partial | Creator complaints |
| **Roko Marketplace** | 0% to $1M, 12-15% above | Yes | All metrics public | -- |

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
- Offer paid tiers (free/pro/team) with feature gating via Macros.
- Run their own support channels (linked from the artifact page).

The marketplace is a distribution channel, not a walled garden. If a creator wants to take their artifacts elsewhere, they can.

### 2.5 On-chain attribution via ERC-8004

Fork chains and provenance are anchored on-chain via ERC-8004 (see [doc-18](18-ON-CHAIN-REGISTRIES.md)):

- **Fork chain provenance**: when an artifact is forked, the fork relationship is recorded as an ERC-8004 attestation. The chain of `forked_from` references forms a Merkle DAG with cryptographic provenance.
- **Reputation flows upstream**: when a fork earns reputation (installs, runs, gate pass rates), a portion flows upstream to the original author. Attribution is not just cosmetic — it is economic.
- **ZK-attested quality**: an artifact's gate pass rate can be ZK-attested (proving the rate without revealing individual run data) and anchored in its ERC-8004 record. Third parties can verify quality claims without trusting the marketplace backend.

### 2.6 Anti-GPT-Store lessons

The marketplace design deliberately avoids patterns that caused the GPT Store to underperform:

| GPT Store failure | Roko marketplace response |
|---|---|
| Opaque revenue sharing | Published take-rate schedule, 0% to $1M |
| No creator analytics | Full UsageLens analytics for every artifact |
| No forking | Fork as fundamental operation |
| Platform owns the customer | Creator owns the customer (export, direct messaging) |
| No quality signal beyond ratings | Gate pass rates, active runs, Verified Run badges |
| Discoverability by editorial fiat | Algorithmic trending + editorial curation + search |
| No composability | Artifacts compose via Slots, Macros, and sub-Graph references |

---

## 3. DAW Composability

The marketplace is modeled on DAW (Digital Audio Workstation) plugin ecosystems: Criteria are plugins, Profiles are presets, fork is the remix.

### 3.1 Criteria as plugins

A Verify Block (see [doc-02](02-BLOCK.md)) is a plugin that evaluates one dimension of quality. Just as a DAW audio plugin slots into any channel strip, a Criterion Block slots into any Verify pipeline:

- Criteria are individually publishable marketplace artifacts.
- A Verify Graph is a composition of Criterion Blocks — a "rack" of quality checks.
- Users install individual Criteria from the marketplace and wire them into their own verification pipelines.
- Custom Criteria are just Blocks that implement the Verify protocol. No special machinery.

### 3.2 Profiles as presets

A **Profile** is a Rack (see [doc-04](04-SPECIALIZATIONS.md)) that computes a Verdict: a pre-wired composition of Criteria with tuned Macros. Just as a DAW preset is a saved configuration of plugin parameters:

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
      -> Graph (full state graph of Blocks)
        -> Space Template (skeleton: Graph + Triggers + capabilities + models)
```

Each level composes the level below. The marketplace supports publishing and forking at every level.

---

## 4. Publishable Artifact Types

Nine artifact kinds, in launch order (smallest / lowest-risk first):

| # | Kind | Risk | Forking | Notes |
|---|---|---|---|---|
| 1 | **Snippet** | None (paste into editor) | Copy | Saved sub-graph fragments |
| 2 | **Prompt Preset** | None (system prompt + role + model) | Copy | Smallest LLM artifact |
| 3 | **Criterion** | None (single Verify Block) | Copy | Individual quality check plugin |
| 4 | **Block** (composition tier) | None (pure composition) | Copy | No new execution surface |
| 5 | **Block** (script / WASM tier) | Sandboxed execution | Copy | Requires capability disclosure |
| 6 | **Profile** | Composition of Criteria | Copy | Verification preset (DAW preset) |
| 7 | **Graph** | Composition of Blocks | Copy | Full state graph with nodes, edges, policy |
| 8 | **Rack** | Graph + Macros + Slots | Copy | Parameterized Graph |
| 9 | **Trigger Binding** | Event config + bound Graph | Copy | Trigger + Graph pairing |
| 10 | **Space Template** | Workspace skeleton | Copy | Capabilities, models, deploy targets |
| 11 | **Knowledge Bundle** | Curated knowledge Signals | Reference + Fork | Are.na-style composition by reference |

v1: Snippets, Prompt Presets, Criteria, Blocks, Profiles, Graphs. v1.1: Racks, Trigger Bindings. v1.2: Space Templates, Knowledge Bundles.

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

Pre-1.0 versions allowed. Pre-release tags (`-rc`, `-alpha`, `-beta`) supported.

### 5.3 Lineage

Every fork records lineage. Lineage is preserved through `Signal.source` (the same content-addressed lineage that Signals carry):

```rust
pub struct ArtifactLineage {
    pub forked_from: Option<ArtifactRef>,
    pub composed_from: Vec<ArtifactRef>,    // when composing N artifacts
    pub forked_at: DateTime<Utc>,
    pub onchain_attestation: Option<ERC8004Ref>, // on-chain provenance anchor
}
```

Lineage is publicly visible: the marketplace renders a fork-chain on every artifact page:

```
Forked from @alice/code-review -> @bob/strict-review -> your version
```

On-chain lineage (via ERC-8004) provides cryptographic proof that cannot be disputed.

### 5.4 License

Default: **CC BY 4.0** for all artifact kinds. Other supported licenses: CC BY-SA 4.0, CC0 1.0, MIT, Apache-2.0. License is required at publish.

Knowledge Bundles default to CC BY-SA 4.0 (share-alike preserves attribution chains).

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

Publish is also available via:
- Dashboard: "Publish" button in the visual editor
- TUI: `p` from F2 Graphs
- Meta-Graph: `graph-publish` (the meta-Graph from [doc-13](13-BUILTIN-BLOCK-CATALOG.md))

---

## 7. Browse and Discovery

### 7.1 Dashboard surface (`/work/marketplace`)

Faceted browse with three primary tabs:
- **Featured** — editorial picks; refreshed weekly by the Nunchi team
- **Trending** — install velocity over the last 7 days
- **All** — full search

Filters:
- Kind (snippet, preset, criterion, block, profile, graph, rack, trigger, template, bundle)
- Category (Authoring, Verification, Research, Execution, Deploy, Operations, etc.)
- Tags
- Capabilities required ("what does this need permission to do?")
- Verified Run badge
- Gate pass rate (minimum threshold filter)
- License
- Publisher
- Recency
- Revenue band (free, paid-with-free-tier, paid-only)

### 7.2 Per-artifact page

```
@wpank/doc-ingest@1.0.0

Ingest a directory of markdown into PRDs, plans, and tasks.

Publisher: @wpank   License: CC-BY-4.0   Updated: 12d ago
Installs: 247   Active runs (30d): 1,820   Forks: 14
Gate pass rate (30d): 94.2%   Avg cost: $0.42   Avg duration: 1m 14s
Revenue: creator-only

Lineage: original (no parent)
On-chain: ERC-8004 attestation 0xabc...

Capabilities required:
  FsRead              any path
  FsWrite             any path
  Llm
  Net                 api.perplexity.ai, arxiv.org

Macros:
  enable_audit         bool   default true
  enable_web_research  bool   default true
  ...

Slots:
  researcher           any web-research Block    default: perplexity-search

Block dependencies:
  fs-walk@^1, markdown-classify@^1, doc-cluster@^1, prd-synthesize@^1,
  prd-audit@^1, prd-plan@^1, knowledge-ingest@^1, ...

Sample input:  Run preview against fixture ->
                Sample output: 3 PRDs created, 1m 14s, $0.42

Versions:
  1.0.0    12d ago
  0.9.0    18d ago

Comments (8):
  @alice  "Works great on docs/ but timed out on a 200-file dir."  reply
  @bob    "Fork: @bob/doc-ingest-with-roman-numeral-headings"      reply

[Install]  [Fork & Edit]  [Preview]  [Source]  [Report]
```

### 7.3 Preview ("Tinker Mode")

In-place preview is the single biggest unlock. Marketplace artifacts have a "Preview" button that runs against the artifact's bundled sample input in a sandboxed worker, returns within ~30s, displays the output in-page. No install required.

For Graphs that take >30s, the preview shows a recorded "last successful run" trace as a fallback. For Snippets and Prompt Presets, preview is instant.

---

## 8. Install Flow

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
Installing Block dependencies (8 blocks)...
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

Granting writes to `workspace.toml` capabilities; the user can revoke later.

---

## 9. Version Resolution

### 9.1 Semver

All artifacts use semver. Pre-release supported. The marketplace surfaces the highest stable version by default; pre-releases via "Show pre-releases" toggle.

### 9.2 Lockfile

`<workspace>/.roko/marketplace.lock` pins exact versions and checksums for installed artifacts. Ensures reproducible installs across machines.

### 9.3 Yanking

A publisher can yank a version (mark broken / vulnerable / wrong). Yanked versions remain installable for users who already pinned to them but are not shown in browse and emit a warning on install.

### 9.4 Deprecation

A publisher can deprecate an artifact (replaced by another, no longer maintained). Deprecation surfaces a banner suggesting the replacement:

```toml
[deprecation]
since       = "2026-06-01"
reason      = "Replaced by improved engine"
replacement = "@wpank/doc-ingest-v2@^1"
```

### 9.5 Vulnerability disclosure

Published artifacts that depend on a vulnerable Block receive an automatic banner. Maintainers receive a notification. Consumers see a warning on install / run. The marketplace runs a CVE feed against Block signatures.

---

## 10. Forking

### 10.1 Local fork

```
$ roko market fork @wpank/doc-ingest@1.0.0 my-doc-ingest
Forking @wpank/doc-ingest@1.0.0...
  Local name: my-doc-ingest
  Lineage:    @wpank/doc-ingest@1.0.0
  Block pins relaxed: ^1 -> ^1 (no change; pinned ones unpinned)
  Files written to: .roko/graphs/my-doc-ingest.toml
Forked. Edit with: roko graph edit my-doc-ingest
```

The fork keeps a `forked_from` reference. When later published, the marketplace renders the chain. On-chain attribution via ERC-8004 ensures the original author receives reputation credit.

### 10.2 Fork chain visualization

```
@alice/code-review@1.0.0
   -> @bob/strict-code-review@2.0.0      (changed: model=opus, strictness=high)
        -> @carol/security-review@1.5.0  (changed: focus_areas=security)
             -> @wpank/strict-security@2.1.0  (you are here)
```

Click any node to view that version. Diff between any two versions via the graph-compare Block.

### 10.3 Composition by reference (vs fork)

For Knowledge Bundles, prompts, and other small artifacts, **composition by reference** is preferred over fork:

```toml
[[graph.knowledge_bundle]]
ref = "@nunchi/safety-knowledge@^1"
```

The bundle is referenced, not copied. Updates to the upstream bundle reach the consumer (with a compatibility check). Forking is still possible if the consumer wants to diverge.

---

## 11. Sandbox and Verified Run Badges

### 11.1 WASM-only marketplace Blocks

Blocks published to the marketplace must be in the **Composition** or **WASM** tier (not Script unless the publisher is verified). Native Rust Blocks are not directly publishable; they may live in trusted source repos for advanced users but not appear in public browse.

This is the security boundary: marketplace = sandboxed. See [doc-17](17-SECURITY-MODEL.md) for the full sandboxing model.

### 11.2 Verified Run

A Block / Graph earns a "Verified Run" badge when:
- It bundles at least one fixture (sample input + expected output shape)
- Marketplace CI runs the artifact against fixtures every release
- All fixtures pass

The badge is displayed prominently and filterable in browse.

### 11.3 Capability disclosure

Capabilities are computed for the entire dependency closure: a Graph's "required capabilities" include those of every Block it transitively uses. The artifact page displays both the immediate-required set and the transitive set.

---

## 12. Rating and Trust

### 12.1 Quality signals (Bayesian-weighted)

- **Install count** (lifetime, 30d, 7d)
- **Active runs** (last 30d) — strongest quality signal (Splice retention pattern)
- **Fork count** — a fork is a positive signal (someone found it useful enough to extend)
- **Comment quality** (length, threaded depth)
- **Rating** — thumbs-up / thumbs-down by verified installers only (accounts that have run the artifact at least once)

Aggregate "would recommend" percentage shown on artifacts with N>=10 installer ratings; below that, only raw install / fork counts.

### 12.2 Gate pass rates

For Graphs that include Verify Blocks, the marketplace tracks aggregate gate pass rates from community runs. A Graph whose gates pass 95% of the time surfaces higher than one at 60%. This is quality signal derived from the verification system itself.

### 12.3 Community validation

Published Graphs with strong pass rates and active runs earn "Community Validated" status. This is distinct from Verified Run (CI-level) — Community Validated means real users are running it successfully in production.

### 12.4 On-chain reputation via ERC-8004

Publisher reputation is also reflected on-chain:
- TraceRank reputation from verified work (see [doc-18](18-ON-CHAIN-REGISTRIES.md)).
- Fork-chain attestations showing the provenance graph.
- ZK-attested gate pass rates for cryptographic quality verification.

The marketplace displays on-chain reputation alongside off-chain metrics. Neither can be faked independently.

### 12.5 Editorial "Featured"

A weekly editorial pass selects up to 5 artifacts as Featured. Editorial uses curatorial judgment, not algorithm. Selection criteria (transparently documented):
- Useful in real Graphs
- Well-documented (README, fixtures, screenshots)
- Verified Run badge
- Sensible capability disclosures (no over-grants)

### 12.6 Anti-spam

- New-account publishing throttle: 1 publish/day for first 30 days; 5/day thereafter for unverified; 50/day for verified-by-Nunchi accounts
- Static analysis on WASM Blocks (banned imports, fuel limits, memory limits)
- LLM-based duplicate detection on prompts and configs; near-duplicates surface a "similar to @x/y" link
- One-click "Report" on any artifact; reports go to Nunchi moderation
- Featured / unfeatured / suspended states transparently logged on each artifact page

### 12.7 Reputation

Per-publisher reputation is a function of:
- Sum of install counts on their artifacts
- Sum of active runs
- Comment helpfulness ratings
- Editorial badge counts
- Time since first publish (Sybil-resistance via age)
- On-chain TraceRank score (if available)

Reputation is shown on publisher pages and serves as a cheap discovery filter ("show me artifacts by publishers with reputation >= 50").

---

## 13. Creator Analytics via UsageLens

The UsageLens ([doc-09 Telemetry](09-TELEMETRY.md)) powers marketplace creator analytics. For every published artifact, creators see:

### 13.1 Metrics dashboard

| Metric | Description | Period |
|---|---|---|
| **Installs** | New installs | 7d, 30d, lifetime |
| **Active runs** | Distinct runs of the artifact | 7d, 30d |
| **Forks** | New forks created | 7d, 30d, lifetime |
| **Error rate** | Percentage of runs that failed | 7d, 30d |
| **Avg cost** | Mean USD cost per run | 30d |
| **Avg duration** | Mean wall-clock per run | 30d |
| **Gate pass rate** | Verify Block pass percentage (if applicable) | 30d |
| **Retention** | % of installers who ran it more than once | 30d |
| **Revenue** | Creator revenue (take-rate applied) | 7d, 30d, lifetime |
| **Upstream attribution** | Revenue flowing from downstream forks | 30d, lifetime |

### 13.2 Trend signals

UsageLens feeds TrendLens and AnomalyLens:
- Install velocity (TrendLens slope drives "Trending" ranking)
- Error rate anomalies (AnomalyLens alerts creator when error rate spikes)
- Cost drift (TrendLens detects if upstream model price changes affect artifact cost)

### 13.3 Per-version analytics

Each published version tracks independent metrics. Creators see version-over-version comparisons to understand the impact of changes.

---

## 14. Backend Service

`roko-marketplace` (service):
- **Storage**: S3 / Tigris for artifact bundles; Postgres for metadata; Redis for trending counts
- **Auth**: GitHub OAuth + JWT for sessions
- **API**: REST `/api/v1/artifacts`, `/publishers`, `/comments`, `/installs`, etc.
- **CI**: GitHub Actions runner that re-runs published fixtures on every release; emits `verified_run` records
- **CDN**: artifact bundles served via CloudFront / Bunny; checksums verified client-side
- **On-chain**: ERC-8004 lineage attestation service (optional, creator opt-in)
- **Mirroring**: optional self-hosted marketplace for orgs (`@my-org/...` namespace on a private endpoint)

The dashboard's marketplace tab is a thin client over this service.

---

## 15. CLI Surface

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

## 16. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `roko market publish` validates capabilities, signs, uploads, indexes | End-to-end test against staging marketplace |
| `roko market install` resolves semver, downloads, verifies checksum, prompts on capability gaps | Install + capability-gap test |
| WASM Blocks execute under fuel + memory limits; over-limit Blocks fail closed | Resource-limit test |
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
| All mandatory metrics (installs, runs, forks, gate pass, cost, duration, error rate) published | Metrics completeness test |
| On-chain lineage attestation anchored in ERC-8004 for opted-in creators | Chain attestation test |
| Fork generates upstream reputation credit for original author | Attribution flow test |
| Criterion artifacts installable as Verify pipeline plugins | Round-trip test: install Criterion, wire into Profile, run Verify |
| Profile artifacts apply as verification presets with adjustable Macros | Round-trip test: install Profile, adjust thresholds, run Verify |
| Creator can export installer list with opt-in consent | Export + consent test |
