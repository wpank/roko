# 15 — Marketplace and Sharing

> Publish, discover, install, fork, rate, and attribute Blocks, Graphs, Racks, and Knowledge Bundles in a community marketplace.

**Source**: wf-12 (Marketplace & Sharing), updated to unified vocabulary with UsageLens integration and creator analytics.

---

## 1. Overview

Every byproduct of using Roko is a shareable, composable primitive. The marketplace turns local Blocks, Graphs, Racks, and Knowledge Bundles into community artifacts — the DAW preset / Splice sample / Figma Community pattern applied to agent orchestration.

Design principles:
- **Composition over isolation** — artifacts reference each other; forking preserves lineage.
- **Trust by evidence** — Verified Run badges, gate pass rates, and community validation replace trust by authority.
- **Capability transparency** — every artifact's permission requirements are visible before install.
- **Observation built in** — UsageLens tracks usage metrics for every published artifact; creators see analytics.

---

## 2. Publishable Artifact Types

Nine artifact kinds, in launch order (smallest / lowest-risk first):

| # | Kind | Risk | Forking | Notes |
|---|---|---|---|---|
| 1 | **Snippet** | None (paste into editor) | Copy | Saved sub-graph fragments |
| 2 | **Prompt Preset** | None (system prompt + role + model) | Copy | Smallest LLM artifact |
| 3 | **Block** (composition tier) | None (pure composition) | Copy | No new execution surface |
| 4 | **Block** (script / WASM tier) | Sandboxed execution | Copy | Requires capability disclosure |
| 5 | **Graph** | Composition of Blocks | Copy | Full state graph with nodes, edges, policy |
| 6 | **Rack** | Graph + Macros + Slots | Copy | Parameterized Graph |
| 7 | **Trigger Binding** | Event config + bound Graph | Copy | Trigger + Graph pairing |
| 8 | **Space Template** | Workspace skeleton | Copy | Capabilities, models, deploy targets |
| 9 | **Knowledge Bundle** | Curated knowledge Signals | Reference + Fork | Are.na-style composition by reference |

v1: Snippets, Prompt Presets, Blocks, Graphs. v1.1: Racks, Trigger Bindings. v1.2: Space Templates, Knowledge Bundles.

---

## 3. Identity and Attribution

### 3.1 Publisher identity

Marketplace identity uses GitHub OAuth for baseline reputation. Optional Google OAuth as alternate. Email-only accounts cannot publish.

Each artifact carries `publisher: "@github_handle"`. Anonymous publishing is not supported in v1.

### 3.2 Artifact identity

Marketplace artifact references use `@publisher/name@version`:

```
@my-org/markdown-classify@1.2.3
@nunchi/doc-ingest@1.0.0
@wpank/strict-pr-review@2.0.0-rc.1
```

Pre-1.0 versions allowed. Pre-release tags (`-rc`, `-alpha`, `-beta`) supported.

### 3.3 Lineage

Every fork records lineage. Lineage is preserved through `Signal.source` (the same content-addressed lineage that Signals carry):

```rust
pub struct ArtifactLineage {
    pub forked_from: Option<ArtifactRef>,
    pub composed_from: Vec<ArtifactRef>,    // when composing N artifacts
    pub forked_at: DateTime<Utc>,
}
```

Lineage is publicly visible: the marketplace renders a fork-chain on every artifact page:

```
Forked from @alice/code-review -> @bob/strict-review -> your version
```

### 3.4 License

Default: **CC BY 4.0** for all artifact kinds. Other supported licenses: CC BY-SA 4.0, CC0 1.0, MIT, Apache-2.0. License is required at publish.

Knowledge Bundles default to CC BY-SA 4.0 (share-alike preserves attribution chains).

---

## 4. Publish Flow

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
  Indexing...
Published as @wpank/doc-ingest@1.0.0
  https://market.roko.dev/@wpank/doc-ingest
```

Publish is also available via:
- Dashboard: "Publish" button in the visual editor
- TUI: `p` from F2 Graphs
- Meta-Graph: `graph-publish` (the meta-Graph from [doc-13](13-BUILTIN-BLOCK-CATALOG.md))

---

## 5. Browse and Discovery

### 5.1 Dashboard surface (`/work/marketplace`)

Faceted browse with three primary tabs:
- **Featured** — editorial picks; refreshed weekly by the Nunchi team
- **Trending** — install velocity over the last 7 days
- **All** — full search

Filters:
- Kind (snippet, preset, block, graph, rack, trigger, template, bundle)
- Category (Authoring, Verification, Research, Execution, Deploy, Operations, etc.)
- Tags
- Capabilities required ("what does this need permission to do?")
- Verified Run badge
- License
- Publisher
- Recency

### 5.2 Per-artifact page

```
@wpank/doc-ingest@1.0.0

Ingest a directory of markdown into PRDs, plans, and tasks.

Publisher: @wpank   License: CC-BY-4.0   Updated: 12d ago
Installs: 247   Active runs (30d): 1,820   Forks: 14

Lineage: original (no parent)

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

### 5.3 Preview ("Tinker Mode")

In-place preview is the single biggest unlock. Marketplace artifacts have a "Preview" button that runs against the artifact's bundled sample input in a sandboxed worker, returns within ~30s, displays the output in-page. No install required.

For Graphs that take >30s, the preview shows a recorded "last successful run" trace as a fallback. For Snippets and Prompt Presets, preview is instant.

---

## 6. Install Flow

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

## 7. Version Resolution

### 7.1 Semver

All artifacts use semver. Pre-release supported. The marketplace surfaces the highest stable version by default; pre-releases via "Show pre-releases" toggle.

### 7.2 Lockfile

`<workspace>/.roko/marketplace.lock` pins exact versions and checksums for installed artifacts. Ensures reproducible installs across machines.

### 7.3 Yanking

A publisher can yank a version (mark broken / vulnerable / wrong). Yanked versions remain installable for users who already pinned to them but are not shown in browse and emit a warning on install.

### 7.4 Deprecation

A publisher can deprecate an artifact (replaced by another, no longer maintained). Deprecation surfaces a banner suggesting the replacement:

```toml
[deprecation]
since       = "2026-06-01"
reason      = "Replaced by improved engine"
replacement = "@wpank/doc-ingest-v2@^1"
```

### 7.5 Vulnerability disclosure

Published artifacts that depend on a vulnerable Block receive an automatic banner. Maintainers receive a notification. Consumers see a warning on install / run. The marketplace runs a CVE feed against Block signatures.

---

## 8. Forking

### 8.1 Local fork

```
$ roko market fork @wpank/doc-ingest@1.0.0 my-doc-ingest
Forking @wpank/doc-ingest@1.0.0...
  Local name: my-doc-ingest
  Lineage:    @wpank/doc-ingest@1.0.0
  Block pins relaxed: ^1 -> ^1 (no change; pinned ones unpinned)
  Files written to: .roko/graphs/my-doc-ingest.toml
Forked. Edit with: roko graph edit my-doc-ingest
```

The fork keeps a `forked_from` reference. When later published, the marketplace renders the chain.

### 8.2 Fork chain visualization

```
@alice/code-review@1.0.0
   -> @bob/strict-code-review@2.0.0      (changed: model=opus, strictness=high)
        -> @carol/security-review@1.5.0  (changed: focus_areas=security)
             -> @wpank/strict-security@2.1.0  (you are here)
```

Click any node to view that version. Diff between any two versions via the graph-compare Block.

### 8.3 Composition by reference (vs fork)

For Knowledge Bundles, prompts, and other small artifacts, **composition by reference** is preferred over fork:

```toml
[[graph.knowledge_bundle]]
ref = "@nunchi/safety-knowledge@^1"
```

The bundle is referenced, not copied. Updates to the upstream bundle reach the consumer (with a compatibility check). Forking is still possible if the consumer wants to diverge.

---

## 9. Sandbox and Verified Run Badges

### 9.1 WASM-only marketplace Blocks

Blocks published to the marketplace must be in the **Composition** or **WASM** tier (not Script unless the publisher is verified). Native Rust Blocks are not directly publishable; they may live in trusted source repos for advanced users but not appear in public browse.

This is the security boundary: marketplace = sandboxed.

### 9.2 Verified Run

A Block / Graph earns a "Verified Run" badge when:
- It bundles at least one fixture (sample input + expected output shape)
- Marketplace CI runs the artifact against fixtures every release
- All fixtures pass

The badge is displayed prominently and filterable in browse.

### 9.3 Capability disclosure

Capabilities are computed for the entire dependency closure: a Graph's "required capabilities" include those of every Block it transitively uses. The artifact page displays both the immediate-required set and the transitive set.

---

## 10. Rating and Trust

### 10.1 Quality signals (Bayesian-weighted)

- **Install count** (lifetime, 30d, 7d)
- **Active runs** (last 30d) — strongest quality signal (Splice retention pattern)
- **Fork count** — a fork is a positive signal (someone found it useful enough to extend)
- **Comment quality** (length, threaded depth)
- **Rating** — thumbs-up / thumbs-down by verified installers only (accounts that have run the artifact at least once)

Aggregate "would recommend" percentage shown on artifacts with N>=10 installer ratings; below that, only raw install / fork counts.

### 10.2 Gate pass rates

For Graphs that include Verify Blocks, the marketplace tracks aggregate gate pass rates from community runs. A Graph whose gates pass 95% of the time surfaces higher than one at 60%. This is quality signal derived from the verification system itself.

### 10.3 Community validation

Published Graphs with strong pass rates and active runs earn "Community Validated" status. This is distinct from Verified Run (CI-level) — Community Validated means real users are running it successfully in production.

### 10.4 Editorial "Featured"

A weekly editorial pass selects up to 5 artifacts as Featured. Editorial uses curatorial judgment, not algorithm. Selection criteria (transparently documented):
- Useful in real Graphs
- Well-documented (README, fixtures, screenshots)
- Verified Run badge
- Sensible capability disclosures (no over-grants)

### 10.5 Anti-spam

- New-account publishing throttle: 1 publish/day for first 30 days; 5/day thereafter for unverified; 50/day for verified-by-Nunchi accounts
- Static analysis on WASM Blocks (banned imports, fuel limits, memory limits)
- LLM-based duplicate detection on prompts and configs; near-duplicates surface a "similar to @x/y" link
- One-click "Report" on any artifact; reports go to Nunchi moderation
- Featured / unfeatured / suspended states transparently logged on each artifact page

### 10.6 Reputation

Per-publisher reputation is a function of:
- Sum of install counts on their artifacts
- Sum of active runs
- Comment helpfulness ratings
- Editorial badge counts
- Time since first publish (Sybil-resistance via age)

Reputation is shown on publisher pages and serves as a cheap discovery filter ("show me artifacts by publishers with reputation >= 50").

---

## 11. Creator Analytics via UsageLens

The UsageLens ([doc-09 Telemetry](09-TELEMETRY.md)) powers marketplace creator analytics. For every published artifact, creators see:

### 11.1 Metrics dashboard

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

### 11.2 Trend signals

UsageLens feeds TrendLens and AnomalyLens:
- Install velocity (TrendLens slope drives "Trending" ranking)
- Error rate anomalies (AnomalyLens alerts creator when error rate spikes)
- Cost drift (TrendLens detects if upstream model price changes affect artifact cost)

### 11.3 Per-version analytics

Each published version tracks independent metrics. Creators see version-over-version comparisons to understand the impact of changes.

---

## 12. Monetization (v2+)

v1 is fully free. No paid artifacts. Monetization is a v2+ concern, tied to:
- Enterprise tier subscriptions
- Splice-style usage royalty (per-install or per-run small payouts to creators)
- Avoiding the GPT Store rev-share trap (rev-share without strong reputation infrastructure is dangerous)

---

## 13. Backend Service

`roko-marketplace` (service):
- **Storage**: S3 / Tigris for artifact bundles; Postgres for metadata; Redis for trending counts
- **Auth**: GitHub OAuth + JWT for sessions
- **API**: REST `/api/v1/artifacts`, `/publishers`, `/comments`, `/installs`, etc.
- **CI**: GitHub Actions runner that re-runs published fixtures on every release; emits `verified_run` records
- **CDN**: artifact bundles served via CloudFront / Bunny; checksums verified client-side
- **Mirroring**: optional self-hosted marketplace for orgs (`@my-org/...` namespace on a private endpoint)

The dashboard's marketplace tab is a thin client over this service.

---

## 14. CLI Surface

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
```

---

## 15. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `roko market publish` validates capabilities, signs, uploads, indexes | End-to-end test against staging marketplace |
| `roko market install` resolves semver, downloads, verifies checksum, prompts on capability gaps | Install + capability-gap test |
| WASM Blocks execute under fuel + memory limits; over-limit Blocks fail closed | Resource-limit test |
| Browse facets work: filter by kind, category, tag, capability, license | Faceted query test |
| Verified Run CI re-runs fixtures and emits badge state | CI integration test |
| Fork chain renders correctly with all ancestors clickable | Visual + DB test |
| Lineage walk works across the marketplace API | API test |
| Yanked versions warn on install but install for already-pinned | Yank test |
| Anti-spam throttling: new account cannot publish > 1/day for first 30 days | Account-limit test |
| UsageLens metrics appear on creator analytics page within 1h of event | Metrics pipeline test |
| TrendLens drives "Trending" sort correctly | Trending algorithm test |
