# PRD-12 — Marketplace & Sharing

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Surface**: Dashboard Marketplace pages + `roko market` CLI + `roko-marketplace` backend service
**Prerequisites**: PRD-00 through PRD-11

---

## 0. Scope

This PRD defines how Workflows, Modules, Triggers, Profiles, Snippets, and Workspace Templates are published, discovered, installed, forked, rated, and attributed in a community marketplace. The goal is to turn every byproduct of using roko into a shareable composable primitive — the DAW preset / Splice sample / Figma Community pattern applied to AI agent orchestration.

This builds on the marketplace research in `uxresearch.md` Topic 5 (community ecosystems and shareable artifacts) and aligns with the visual-gate2 PRD-06 community model.

---

## 1. Publishable Artifact Types

Five artifact kinds, in launch order (smallest / lowest-risk first):

| # | Kind | Why first | Forking semantics |
|---|---|---|---|
| 1 | **Snippet** | Smallest, no execution risk; pasted into editor | Copy-by-fork |
| 2 | **Prompt Preset** | System-prompt + role + model recommendation; smallest LLM artifact | Copy-by-fork |
| 3 | **Module** (composition tier) | Pure composition of other modules; no new execution surface | Copy-by-fork |
| 4 | **Module** (script / WASM tier) | Includes executable code; sandboxing required | Copy-by-fork |
| 5 | **Workflow** | Highest-value; full state graph with macros / slots | Copy-by-fork |
| 6 | **Trigger Recipe** | Trigger config + bound Workflow | Copy-by-fork |
| 7 | **Profile** (visual-gate2) | Evaluation strategy | Copy-by-fork |
| 8 | **Workspace Template** | Whole workspace skeleton | Copy-by-fork |
| 9 | **Knowledge Bundle** | Curated knowledge entries | Reference (Are.na) + fork |

Snippets and prompt presets ship in v1; Modules and Workflows in v1.1; Profiles, Trigger Recipes, Workspace Templates, Knowledge Bundles in v1.2.

---

## 2. Identity & Attribution

### 2.1 Publisher Identity

Marketplace identity uses GitHub OAuth for baseline reputation (per research §Topic 5: anti-spam requires linkable identity). Optional Google OAuth as alternate. Email-only accounts cannot publish.

Each artifact carries `publisher: "@github_handle"`. Anonymous publishing is not supported in v1.

### 2.2 Artifact Identity

Marketplace artifact references use `@publisher/name@version`:

```
@my-org/markdown-classify@1.2.3
@nunchi/doc-ingest@1.0.0
@wpank/strict-pr-review@2.0.0-rc.1
```

Pre-1.0 versions allowed. Pre-release tags (`-rc`, `-alpha`, `-beta`) supported.

### 2.3 Lineage

Every fork records lineage:

```rust
pub struct Lineage {
    pub forked_from: Option<ArtifactRef>,
    pub composed_from: Vec<ArtifactRef>,    // when composing N artifacts
    pub forked_at:   DateTime<Utc>,
}
```

Lineage is publicly visible per artifact: the marketplace renders a fork-chain ("Forked from @alice/code-review → @bob/strict-review → your version") on every artifact page.

### 2.4 License

Default: **CC BY 4.0** for all artifact kinds. Other supported licenses on publish: CC BY-SA 4.0, CC0 1.0, MIT, Apache-2.0. The license is required at publish; it's set on the artifact record and surfaced on every artifact page.

Knowledge Bundles default to CC BY-SA 4.0 (share-alike preserves attribution chains).

---

## 3. Publish Flow

```
$ roko market publish doc-ingest
Validating doc-ingest@1.0.0...
  ✓ Schema valid
  ✓ All references resolve
  ✓ Capabilities declared: fs.read, fs.write, llm, net
  ✓ License: CC-BY-4.0
  ⚠ No screenshot bundle (recommended for visibility)
  ⚠ No fixture for "Verified Run" badge

Capabilities your artifact requires (consumers will see this):
  • fs.read              "any path"   (recommend: restrict to specific patterns)
  • fs.write             "any path"   (recommend: restrict)
  • llm                  "any provider"
  • net                  "*"          (recommend: list specific domains)
[continue anyway / restrict capabilities / abort]

Visibility:  ( ) public   ( ) my-org-only   (•) private (publish later)
Tags:        doc, ingest, authoring
Description: Ingest a directory of markdown into PRDs, plans, and tasks
README:      .roko/workflows/doc-ingest.README.md  (auto-detected)
Sample input: .roko/workflows/doc-ingest.fixtures/  (auto-detected, 2 fixtures)

Publish? [Y/n]
Publishing...
  Bundling artifact (workflow.toml + module deps + README + fixtures)
  Computing checksum: blake3:abc...
  Uploading to marketplace.roko.dev
  Indexing...
✓ Published as @wpank/doc-ingest@1.0.0
  https://market.roko.dev/@wpank/doc-ingest
```

Publish is also available via:
- Dashboard: "Publish" button in the editor (PRD-11 §1).
- TUI: `p` from F2 Workflows.
- `workflow-publish` Workflow (the meta-workflow from PRD-06 §13).

---

## 4. Browse & Discovery

### 4.1 Dashboard Surface (`/work/marketplace`)

Faceted browse with three primary tabs:
- **Featured** — editorial picks; refreshed weekly by the Nunchi team.
- **Trending** — install velocity over the last 7 days.
- **All** — full search.

Filters:
- Kind (snippet / preset / module / workflow / trigger / profile / template / bundle).
- Category (per PRD-06 categories).
- Tags.
- Capabilities required (the "what does this need permission to do?" filter).
- Verified Run badge (artifacts with passing automated test fixtures).
- License.
- Publisher.
- Recency.

### 4.2 Per-Artifact Page

```
@wpank/doc-ingest@1.0.0
─────────────────────────────────────────
Ingest a directory of markdown into PRDs, plans, and tasks.

Publisher: @wpank · License: CC-BY-4.0 · Updated: 12d ago
Installs: 247 · Active runs (30d): 1,820 · Forks: 14

Lineage: original (no parent)

Capabilities required:
  • fs.read              any path
  • fs.write             any path
  • llm
  • net                  api.perplexity.ai, arxiv.org

Macros:
  enable_audit         bool   default true
  enable_web_research  bool   default true
  ...

Slots:
  researcher           any web-research module    default: perplexity-search

Module dependencies:
  fs-walk@^1, markdown-classify@^1, doc-cluster@^1, prd-synthesize@^1,
  prd-audit@^1, prd-plan@^1, knowledge-ingest@^1, ...

Sample input:  Run preview against fixture →
                Sample output: 3 PRDs created, 1m 14s, $0.42

Versions:
  1.0.0    12d ago
  0.9.0    18d ago

Comments (8):
  @alice  "Works great on docs/ but timed out on a 200-file dir."  reply
  @bob    "Fork: @bob/doc-ingest-with-roman-numeral-headings"      reply
  ...

[Install]  [Fork & Edit]  [Preview]  [Source]  [Report]
```

### 4.3 Preview ("Tinker Mode")

Per `uxresearch.md` Topic 5: in-place preview is the single biggest unlock. Marketplace artifacts have a "Preview" button that runs against the artifact's bundled sample input in a sandboxed worker, returns within ~30s, displays the output in-page. No install required.

For Workflows that take >30s to run, the preview shows a recorded "last successful run" trace as a fallback.

For Snippets and Prompt Presets, preview is instant (no execution).

---

## 5. Install Flow

```
$ roko market install @wpank/doc-ingest@^1
Resolving @wpank/doc-ingest@^1 → 1.0.0...
Inspecting capabilities:
  This workflow requires:
    fs.read    any path     → granted by workspace
    fs.write   any path     → granted by workspace
    llm                     → granted by workspace
    net        api.perplexity.ai, arxiv.org → workspace grants net.* (covered)
  All capabilities covered. Continue? [Y/n]
Downloading bundle...  (124 KB)
  Verifying checksum: blake3:abc... ✓
  Verifying signature (publisher: @wpank): ✓
Installing module dependencies (8 modules)...
  ✓ fs-walk@1.0.4
  ✓ markdown-classify@1.0.0
  ...
Registering with workspace...
✓ Installed as @wpank/doc-ingest@1.0.0
  Run with: roko run @wpank/doc-ingest --input source_dir=...
```

If a capability is **not** covered, the install pauses for explicit grant:

```
This artifact requires:
  shell  ["cargo", "git", "rustc"]
Workspace currently grants: shell = false
Grant `shell ["cargo", "git", "rustc"]` to this workspace? (y/N/configure)
```

Granting writes to `workspace.toml` `[workspace.capabilities]` with the granular declaration; the user can revoke later.

The dashboard install flow has the same checks rendered as a series of capability cards with toggles, then a "Continue install" CTA.

---

## 6. Sandbox & Verified Run Badges

### 6.1 WASM-Only Marketplace Modules

Modules published to the marketplace must be in the **Composition** or **WASM** tier (not Script unless the publisher is verified). Native Rust modules are not directly publishable; they may live in trusted source repos and be installed via `git+https://...` references for advanced users, but not appear in the public marketplace browse.

This is the security boundary: marketplace = sandboxed.

### 6.2 Verified Run

A Module / Workflow earns a "Verified Run" badge when:
- It bundles ≥1 fixture (sample input + expected output shape).
- Marketplace CI runs the artifact against fixtures every release.
- All fixtures pass.

The badge is displayed prominently. Filter by it on browse. It's the "this thing demonstrably works" signal.

### 6.3 Capability Disclosure

Capabilities are computed for the entire dependency closure: a Workflow's "required capabilities" include those of every Module it transitively uses. The artifact page displays both the immediate-required set and the transitive set.

---

## 7. Rating & Trust

### 7.1 Signals (Bayesian-weighted)

- **Install count** (lifetime, 30d, 7d).
- **Active runs** (last 30d) — strongest quality signal per `uxresearch.md` (Splice retention pattern).
- **Fork count** — a fork is a positive signal (someone found it useful enough to extend).
- **Comment quality** (length, threaded depth).
- **Rating** — thumbs-up / thumbs-down by **verified installers only** (accounts that have run the artifact ≥1 time).

Aggregate "would recommend" percentage shown on artifacts with N≥10 installer ratings; below that, only raw install / fork counts.

### 7.2 Editorial "Featured"

A weekly editorial pass selects up to 5 artifacts as Featured. Editorial uses curatorial judgment, not algorithm. The Featured tier is the antidote to algorithmic gaming.

Selection criteria (transparently documented):
- Useful in real workflows.
- Well-documented (README, fixtures, screenshots).
- Verified Run badge.
- Sensible capability disclosures (no over-grants).

### 7.3 Anti-Spam

Per `uxresearch.md` Topic 5 mitigations:
- New-account publishing throttle: 1 publish/day for the first 30 days; 5/day thereafter for unverified, 50/day for verified-by-Nunchi accounts.
- Static analysis on WASM modules (banned imports, fuel limits, memory limits).
- LLM-based duplicate detection on prompts and configs; near-duplicates surface a "similar to @x/y" link instead of competing in browse.
- One-click "Report" on any artifact; reports go to Nunchi moderation; abuse triggers account flags.
- Featured / unfeatured / suspended states transparently logged on each artifact page.

### 7.4 Reputation

Per-publisher reputation is a function of:
- Sum of install counts on their artifacts.
- Sum of active runs.
- Comment helpfulness ratings.
- Editorial badge counts.
- Time since first publish (Sybil-resistance via age).

Reputation is shown on publisher pages. It's a cheap discovery filter ("show me artifacts by publishers with reputation ≥ 50").

---

## 8. Forking

### 8.1 Local Fork

`roko market fork @wpank/doc-ingest@1.0.0 my-doc-ingest` creates a local mutable copy.

```
$ roko market fork @wpank/doc-ingest@1.0.0 my-doc-ingest
Forking @wpank/doc-ingest@1.0.0...
  Local name: my-doc-ingest
  Lineage:    @wpank/doc-ingest@1.0.0
  Module pins relaxed: ^1 → ^1 (no change for ^1; pinned ones unpinned)
  Files written to: .roko/workflows/my-doc-ingest.toml
✓ Forked. Edit with: roko workflow edit my-doc-ingest
```

The fork keeps a `forked_from` reference. When the fork is later published, the marketplace renders the chain.

### 8.2 Fork Chain Visualization

```
@alice/code-review@1.0.0
   └─ @bob/strict-code-review@2.0.0      (changed: model=opus, strictness=high)
        └─ @carol/security-review@1.5.0  (changed: focus_areas=security)
             └─ @wpank/strict-security@2.1.0  (you are here)
                  └─ ...
```

Click any node to view that version. Diff between any two versions via `workflow-compare` (per PRD-06 §13).

### 8.3 Composition (vs Fork)

For Knowledge Bundles, prompts, and other small artifacts, **composition by reference** (Are.na pattern) is preferred over fork. A Workflow may declare:

```toml
[[workflow.knowledge_bundle]]
ref = "@nunchi/safety-knowledge@^1"
```

The bundle is referenced, not copied. Updates to the upstream bundle reach the consumer (with a compatibility check). Forking is still possible if the consumer wants to diverge.

---

## 9. Versioning & Deprecation

### 9.1 Semver

All artifacts use semver. Pre-release supported. The marketplace surfaces the highest stable version by default; pre-releases via "Show pre-releases" toggle.

### 9.2 Yanking

A publisher can yank a version (mark broken / vulnerable / wrong). Yanked versions remain installable for users who already pinned to them but aren't shown in browse and emit a warning on install.

### 9.3 Deprecation

A publisher can deprecate an artifact (replaced by another, no longer maintained). Deprecation surfaces a banner on the artifact page suggesting the replacement.

```toml
[deprecation]
since   = "2026-06-01"
reason  = "Replaced by improved engine"
replacement = "@wpank/doc-ingest-v2@^1"
```

### 9.4 Vulnerability Disclosure

Published artifacts that depend on a vulnerable Module receive an automatic banner. Maintainers receive a notification. Consumers see a warning on install / run. The marketplace runs a CVE feed against module signatures.

---

## 10. Monetization (Out of Scope for v1)

v1 is fully free; no paid artifacts. Per `uxresearch.md` Topic 5: "start the marketplace with editorial Featured + verified-installer ratings + composition-by-reference." Monetization is a v2+ concern, ideally tied to enterprise tier subscriptions or a Splice-style usage royalty (per-install or per-run small payouts to creators), avoiding the GPT Store rev-share trap.

---

## 11. Backend Service

`roko-marketplace` (new crate / service):
- **Storage**: S3 / Tigris for artifact bundles; Postgres for metadata; Redis for trending counts.
- **Auth**: GitHub OAuth + JWT for sessions.
- **API**: REST `/api/v1/artifacts`, `/publishers`, `/comments`, `/installs`, etc.
- **CI**: GitHub Actions runner that re-runs published fixtures on every release; emits `verified_run` records.
- **CDN**: artifact bundles served via CloudFront / Bunny; checksums verified client-side.
- **Mirroring**: optional self-host marketplace for orgs that want their own ("@my-org/..." namespace on a private endpoint).

The dashboard's marketplace tab is a thin client over this service.

---

## 12. CLI Surface

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
roko market mirror <url>                     # add an alternate marketplace endpoint
roko market verify <ref>                     # explicit checksum + signature check
```

---

## 13. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `roko market publish` validates capabilities, signs, uploads, indexes. | End-to-end test against a staging marketplace. |
| `roko market install` resolves semver, downloads bundle, verifies checksum, prompts on capability gaps. | Install + capability-gap test. |
| WASM modules execute under fuel + memory limits; over-limit modules fail closed. | Resource-limit test. |
| Browse facets work: filter by kind, category, tag, capability, license. | Faceted query test. |
| Verified Run CI re-runs fixtures and emits the badge state. | CI integration test. |
| Fork chain renders correctly with all ancestors clickable. | Visual + DB test. |
| Lineage walk works across the marketplace API. | API test. |
| Yanked versions warn on install but install for already-pinned. | Yank test. |
| Anti-spam throttling: new account cannot publish > 1/day for first 30 days. | Account-limit test. |
| Editorial "Featured" toggle surfaces artifacts on the marketplace home. | Featured workflow test. |

---

## 14. Open Questions

- Should there be a "marketplace mirror" model where an org runs their own marketplace and federates with public? Likely yes for enterprise; specify in v1.1.
- Should installs be reproducible (lockfile equivalent)? Yes — `<workspace>/.roko/marketplace.lock` pins exact versions and checksums for installed artifacts.
- Should artifacts be signed by publishers (Sigstore, age, GPG)? Sigstore is the right answer; integrate with publisher identity.
- Should there be private "org-only" marketplaces with shared namespaces? Yes; a `@my-org/` prefix routes to a configurable endpoint.
- Should there be revenue sharing in v1? No. Keep it free; revisit when enterprise/usage data motivates it. Per Topic 5 research: rev-share without strong reputation infrastructure (the GPT Store trap) is dangerous to launch with.
- How are artifacts rendered when they require disabled capabilities at preview time? The preview pane stubs the capability with a sandbox response and warns "This preview ran with stubbed capability X."
