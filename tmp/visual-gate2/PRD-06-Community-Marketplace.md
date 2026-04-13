# PRD-06 — Community Marketplace: Publish, Discover, Fork, Compose

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-29
**Crate**: `roko-eval-community` (new)
**Prerequisites**: PRD-00 (System Overview), PRD-01 (Core Abstractions), PRD-03 (Criterion Library)

---

## 0. Scope

This document defines the community marketplace for evaluation artifacts: how users
publish, discover, install, fork, and compose criteria, profiles, evidence collectors,
judge rubrics, canary sets, and knowledge bundles as shared, reusable primitives.

The core thesis: **everything users create during evaluation is a shareable byproduct
of their workflow.** A user tunes a contrast criterion for their e-commerce domain.
That tuned criterion is one click from becoming a published artifact that every
e-commerce team can install. The marketplace makes the collective's evaluation
knowledge compound -- every user who publishes raises the floor for every user who
installs.

### The DAW Analogy

This PRD applies the DAW composability principle throughout:

| DAW Concept | Evaluation Equivalent | Marketplace Role |
|---|---|---|
| VST/AU Plugin | `Criterion` | Atomic evaluation unit, installable and composable |
| Plugin Preset | `Criterion` with domain config | Plugin tuned for a particular context |
| Rack / FX Chain | `Profile` | Composition of criteria, ordered and configured |
| Rack Preset | Published `Profile` | Shareable evaluation strategy with macros |
| Sample Pack | `CanarySet` | Reference material for calibration |
| Project Template | `KnowledgeBundle` + `Profile` | Domain knowledge + evaluation strategy |
| Plugin Store | Community Marketplace | Discovery, install, trust, attribution |
| "Save as Rack Preset" | Fork | Start from what works |

---

## 1. Design Philosophy

### 1.1 Sharing as Byproduct, Not Ceremony

The publish action is a continuation of the creation workflow, not a separate
process. When a user finishes tuning a criterion, the "Publish" button is right
there -- same page, same context, same session. No context switch.

Reference: Figma Community. A designer clicks "Publish to Community." The artifact
IS the deliverable -- no repackaging required.

### 1.2 Fork Is the Fundamental Operation

Fork is the most common action. Every installed artifact has a "Fork & Edit"
button. Forking creates a fully mutable copy preserving attribution.

Reference: Ableton's "Save as Rack Preset." Open a synth rack, tweak it, save as
your own. The new preset is yours to modify, but lineage is preserved:
"Based on Factory > Bass > Acid House."

### 1.3 Criteria Are Plugins, Profiles Are Presets

A `Criterion` is a self-contained evaluation unit. It declares evidence
requirements, runs independently, produces a single-dimension score with
grounded findings. It is the VST plugin of evaluation.

A `Profile` is a composition of criteria. It declares which criteria to run,
how to compose results (conjunctive hard + Pareto soft), and which parameters
are promoted as macros. It is the preset / FX chain.

### 1.4 Collective Knowledge Compounds

Every published criterion encodes evaluation expertise. A domain expert who
publishes `@alice/saas-a11y-strict` has encoded their understanding of SaaS
accessibility. When a hundred teams install it, collective quality rises
without each team independently discovering the same thresholds.

### 1.5 The Marketplace Is the Plugin Store

Discovery (search, browse, featured), trust (verified runs, install counts,
creator reputation), and logistics (install, update, dependency resolution).

Reference platforms:
- **Splice** -- Samples as composable primitives, preview before install.
- **Figma Community** -- Components as both design and deliverable.
- **Hugging Face Hub** -- Models with metadata, cards, usage stats, lineage.
- **crates.io / npm** -- Version pinning, semver, transitive dependencies.

---

## 2. Artifact Types

### 2.1 Criteria (Launch Phase 1)

The atomic evaluation unit.

**Package contents:**
- `criterion.toml` -- TOML definition file
- `script.{js,py,sh}` -- Optional evaluation script
- `model.toml` -- Optional LLM model configuration (for JudgePanel kind)
- `README.md` -- Description, usage examples, domain context
- `sample/` -- Optional sample artifacts for dry-run validation

**Properties:**
- Semver versioned (`1.0.0`, `1.2.3-beta.1`)
- Dependency-declared (evidence from other collectors, run-after ordering)
- Immutable once published (new version = new semver)
- Content-addressed (`.rokoeval` bundle hashed for integrity)

**Example: `@alice/ecommerce-checkout-contrast`**

```toml
[criterion]
name = "ecommerce-checkout-contrast"
version = "1.2.0"
kind = "deterministic"
description = "APCA contrast check tuned for e-commerce checkout UX"
license = "CC-BY-4.0"
tags = ["ecommerce", "a11y", "contrast", "checkout"]
domain = "web-ui"

[evidence]
required = ["dom", "computed_styles"]
optional = ["screenshot"]

[scoring]
severity = "hard"
range = [0.0, 1.0]

[params]
cta_min_contrast = { type = "float", default = 90.0, description = "Min APCA Lc for CTAs" }
label_min_contrast = { type = "float", default = 75.0, description = "Min APCA Lc for form labels" }
body_min_contrast = { type = "float", default = 60.0, description = "Min APCA Lc for body text" }

[author]
name = "Alice Chen"
namespace = "alice"
```

### 2.2 Profiles (Launch Phase 1)

A composition of criteria.

**Package contents:**
- `profile.toml` -- Composition of CriterionRefs
- `README.md` -- Description, audience, domain context
- `macros.toml` -- Optional macro definitions (promoted parameters)

**Profiles can contain slots** -- empty positions where consumers plug in their
own criteria. A slot declares type constraints (evidence kinds, severity) but
leaves the specific criterion to the consumer.

```toml
[profile]
name = "saas-landing-page"
version = "2.1.0"
description = "Startup-friendly evaluation: relaxed visual, strict a11y"
composition = "conjunctive_hard_pareto_soft"

[[criteria]]
ref = "roko/apca-contrast@^1.0"
severity = "hard"
params = { min_contrast = 60.0 }

[[criteria]]
ref = "roko/visual-quality-judge@^1.0"
severity = "soft"
params = { panel_size = 3 }

[[slots]]
name = "brand_check"
description = "Your brand-specific criterion"
severity = "soft"
required_evidence = ["dom", "computed_styles"]
optional = true

[macros]
target_score = { source = "profile", param = "min_pareto_score", type = "float", default = 0.7 }
strict_a11y = { source = "roko/wcag-violations", param = "level", type = "enum", options = ["A", "AA", "AAA"] }
```

### 2.3 Evidence Collectors (Launch Phase 2)

Custom evidence collection scripts.

**Package contents:**
- `collector.toml` -- Metadata and evidence kind declarations
- `collect.{js,py,sh}` -- Collection script
- `schema.json` -- JSON Schema for evidence output format

**Properties:**
- Sandboxed execution (restricted filesystem, declared network access)
- Declares which `EvidenceKind`s it produces
- Declares runtime requirements (Node, Python, browser)

### 2.4 Judge Rubrics (Launch Phase 2)

Rubric templates for LLM judge criteria.

**Package contents:**
- `rubric.toml` -- Dimensions, weights, scoring anchors
- `prompt.md` -- Judge prompt template with variable slots
- `calibration/` -- Optional calibration examples

**Properties:**
- Declares recommended and contraindicated model families
- Declares position-swap strategy (per PRD-04)
- Can reference knowledge bundles for context enrichment

**Example: `@diana/design-system-rubric`**

A rubric calibrated on Material Design 3 patterns. Evaluates: component usage,
spacing adherence, elevation consistency, typography scale, color system.

```toml
[rubric]
name = "design-system-rubric"
version = "1.3.0"
description = "Visual quality rubric calibrated on Material Design 3"

[[dimensions]]
name = "component_usage"
weight = 0.25
anchors = [
  { score = 1, description = "Components unrecognizable or misused" },
  { score = 5, description = "Recognizable with minor issues" },
  { score = 9, description = "Exemplary usage including edge cases" },
]

[judge]
recommended_families = ["claude", "gemini"]
position_swap = true
panel_size_min = 3

[knowledge]
bundles = ["roko/material-design-3-patterns"]
```

### 2.5 Canary Sets (Launch Phase 3)

Curated human-rated datasets for judge calibration.

**Package contents:**
- `canary.toml` -- Metadata, rater info, statistical properties
- `artifacts/` -- Rated artifacts (screenshots, HTML, diffs)
- `ratings.jsonl` -- Human ratings per artifact per dimension

**Properties:**
- Multiple human raters per artifact
- Krippendorff alpha metadata per dimension
- Content-addressed (hash-pinned, no silent replacement)
- Dimension-specific ratings

### 2.6 Knowledge Bundles (Launch Phase 3)

Curated evaluation knowledge entries.

**Package contents:**
- `bundle.toml` -- Metadata, entry manifest
- `entries/` -- Individual knowledge entries (Markdown or JSON)

**Properties:**
- Each entry is a standalone knowledge unit
- Entries declare attachment context (which criteria/dimensions they enrich)
- Provenance: source URL, author, date, confidence level

---

## 3. Registry Architecture

### 3.1 Namespace Model

All artifacts identified by fully-qualified reference:

```
<namespace>/<artifact-name>@<version>
```

| Namespace | Pattern | Governance |
|---|---|---|
| Official | `roko/` | Maintained by roko team |
| User | `@username/` | Personal namespace |
| Organization | `@orgname/` | Shared team namespace |

**Version specifiers:**

| Specifier | Meaning |
|---|---|
| `=1.2.3` | Exact version |
| `^1.2` | Compatible with 1.x.x |
| `~1.2` | Patch-level only: 1.2.x |
| `>=1.0, <2.0` | Explicit range |

### 3.2 Package Format

Published artifacts are `.rokoeval` files -- ZIP archives:

```
my-criterion-1.2.0.rokoeval
  +-- manifest.toml          # Package metadata, deps, checksums
  +-- criterion.toml          # Artifact definition
  +-- README.md
  +-- LICENSE
  +-- script.js               # Optional
  +-- sample/
  +-- checksums.blake3        # BLAKE3 hashes
```

**manifest.toml:**

```toml
[package]
name = "ecommerce-checkout-contrast"
namespace = "alice"
version = "1.2.0"
type = "criterion"
roko_version_min = "0.9.0"

[signature]
algorithm = "ed25519"
public_key = "base64-encoded-pubkey"
signature = "base64-encoded-sig"

[dependencies]
"roko/apca-contrast" = "^1.0"

[checksums]
root = "blake3-hash-of-all-file-hashes"
```

### 3.3 Registry API

REST API at `https://registry.roko.dev/v1`. Bearer token auth for publish,
anonymous read for public artifacts.

**Core endpoints:**

| Method | Path | Description |
|---|---|---|
| `POST` | `/artifacts` | Publish a `.rokoeval` bundle |
| `GET` | `/search` | Fuzzy search with type/domain/tag filters |
| `GET` | `/artifacts/:ns/:name` | Get artifact metadata |
| `GET` | `/artifacts/:ns/:name/:version/download` | Download bundle |
| `POST` | `/artifacts/:ns/:name/:version/fork` | Fork to user's namespace |
| `GET` | `/artifacts/:ns/:name/versions` | List versions |
| `DELETE` | `/artifacts/:ns/:name/:version` | Yank version |
| `POST` | `/artifacts/:ns/:name/thumbsup` | Rate (requires verified install) |
| `POST` | `/artifacts/:ns/:name/comments` | Comment |

### 3.4 Dependency Resolution

**Dependency types:**
1. **Evidence dependency**: criterion needs evidence from another collector.
2. **Criterion reference**: profile references criteria by CriterionRef.
3. **Knowledge attachment**: rubric/criterion references knowledge bundles.

**Resolution algorithm:**
1. Expand all transitive dependencies.
2. Unify version ranges (intersect).
3. Resolve to concrete versions (highest matching stable).
4. Verify compatibility (roko_version_min).
5. Produce `eval.lock` file (pinned, committed to version control).

**Conflict resolution rules:**
1. Exact pins (`=`) win over ranges.
2. Lock file pins win over range resolution.
3. Local overrides (`.roko/eval/overrides.toml`) win over everything.
4. Incompatible ranges -> error with conflict chain diagnostic.

---

## 4. Trust and Reputation System

### 4.1 Verified Runs

The strongest trust signal: "this criterion has been run N times by M teams
with an average pass rate of P%." Verified runs are reported anonymously by
the evaluation framework when the user opts in.

```rust
/// File: crates/roko-eval-community/src/trust.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedRunReport {
    /// Anonymous hash of the team/workspace.
    pub team_hash: String,
    /// Criterion + version that was run.
    pub artifact_ref: String,
    /// Number of evaluations run.
    pub run_count: u32,
    /// Aggregate pass rate.
    pub pass_rate: f64,
    /// When this report was submitted.
    pub reported_at: String,
}
```

### 4.2 Creator Reputation Score

Reputation is earned through artifact quality, not social signals:

| Signal | Weight | Description |
|---|---|---|
| Install count | 0.20 | How many users installed |
| Active runs (30d) | 0.25 | How many times artifacts are actually used |
| Fork count | 0.15 | How many times artifacts are forked |
| Canary correlation | 0.25 | How well judge rubrics correlate with human ratings |
| Comment sentiment | 0.10 | Positive/negative comment ratio |
| Yank rate | 0.05 | Lower is better (fewer broken versions) |

Reputation is per-creator (not per-artifact) and displayed alongside artifacts
in search results.

### 4.3 Trust Badges

| Badge | Criteria | Display |
|---|---|---|
| Verified | >= 100 verified runs from >= 5 teams | Green checkmark |
| Battle-tested | >= 1,000 runs, pass rate > 90% | Shield icon |
| Calibrated | Rubric Spearman rho >= 0.7 vs canary set | Star icon |
| Official | Published by the roko team | Blue badge |

### 4.4 Security Review Pipeline

Artifacts containing scripts undergo automated security review:

1. **Static analysis**: no `eval()`, no credential patterns, no undeclared
   network access.
2. **Sandbox test**: run script in restricted container, verify it only
   accesses declared resources.
3. **Dependency audit**: check script dependencies against known
   vulnerability databases.

Artifacts that pass security review receive a "Reviewed" badge. Artifacts
with scripts that fail review are blocked from publishing.

---

## 5. Publishing Flow

### 5.1 CLI Flow

```bash
# Publish a criterion
roko eval publish ./my-criterion/

# Publish with version bump
roko eval publish ./my-criterion/ --bump minor

# Dry-run: validate without publishing
roko eval publish ./my-criterion/ --dry-run
```

**Publish pipeline (in order):**

1. Schema validation (TOML parses, required fields present)
2. Dependency check (referenced artifacts exist at compatible versions)
3. Dry-run evaluation (run on sample artifacts, verify valid output)
4. Security scan (no credential patterns, no undeclared network)
5. License compatibility (GPL cannot reference non-GPL)
6. Bundle (create `.rokoeval` with BLAKE3 checksums)
7. Sign (ed25519 key)
8. Upload

### 5.2 Dashboard Flow

Visual publishing from any criterion or profile in the Evals Library page.
"Publish" button runs the same validation pipeline with inline results.

### 5.3 Validation Checks

| Check | Required? | Blocks Publish? |
|---|---|---|
| Schema validation | Yes | Yes |
| Description >= 50 chars | Yes | Yes |
| At least one tag | Yes | Yes |
| Evidence declaration | Yes (criteria) | Yes |
| Severity declaration | Yes (criteria) | Yes |
| Dependency resolution | Yes | Yes |
| Dry-run evaluation | Yes | Yes |
| Security scan | Yes | Yes |
| License compatibility | Yes (profiles) | Yes |
| Version uniqueness | Yes | Yes |
| Signature validity | Yes | Yes |
| Sample quality | No | Warning |
| README quality | No | Warning |

---

## 6. Discovery and Installation

### 6.1 CLI Flow

```bash
# Search
roko eval search "ecommerce contrast"
roko eval search "a11y" --type criterion
roko eval search --tag saas --tag a11y --sort installs

# Install
roko eval install @alice/ecommerce-checkout-contrast
roko eval install @alice/ecommerce-checkout-contrast@1.1.0
roko eval install @bob/saas-landing-page  # Profile: installs all criteria

# Update
roko eval update
roko eval update @alice/ecommerce-checkout-contrast

# Fork
roko eval fork @alice/ecommerce-checkout-contrast
roko eval fork @alice/ecommerce-checkout-contrast --name strict-checkout

# List installed
roko eval list
roko eval list --type criterion --check-updates

# Uninstall
roko eval uninstall @alice/ecommerce-checkout-contrast
```

### 6.2 Search Ranking: Frecency

Default search ranking uses frecency (frequency + recency):

```
frecency_score = installs_30d * 2.0 + active_runs_30d * 3.0
               + fork_count * 1.5 + thumbsup_count * 1.0
               + recency_bonus(published_at)
```

This prioritizes actively-used artifacts over once-popular but stale ones.

### 6.3 Local Installation Layout

```
.roko/eval/
  installed/                    # Downloaded artifacts
    alice/
      ecommerce-checkout-contrast/
        1.2.0/
          criterion.toml
          script.js
          ...
  local/                        # Forked/local artifacts
    strict-checkout-contrast/
      criterion.toml
      ...
  eval.lock                     # Pinned dependency versions
  overrides.toml                # Local parameter overrides
```

---

## 7. Versioning and Composition

### 7.1 Semantic Versioning

All artifacts follow semver. Breaking changes require major version bump.

| Change Type | Version Bump | Example |
|---|---|---|
| Bug fix in scoring logic | Patch (1.0.0 -> 1.0.1) | Fix false positive |
| New optional parameter | Minor (1.0.0 -> 1.1.0) | Add `cta_selector` param |
| Changed evidence requirements | Major (1.0.0 -> 2.0.0) | Now requires screenshots |
| Changed scoring range | Major | [0,1] -> [0,100] |
| Removed parameter | Major | Remove `body_min_contrast` |

### 7.2 Composition Strategies

Profiles declare how to compose their criteria results:

| Strategy | Behavior |
|---|---|
| `conjunctive_hard_pareto_soft` | All hard criteria must pass; soft criteria aggregated via Pareto |
| `conjunctive_all` | All criteria must pass (no soft/hard distinction) |
| `voting_majority` | Majority of criteria must pass |
| `weighted_sum` | Weighted sum of scores, threshold for pass/fail |
| `fallback_chain` | Try criteria in order, use first non-error result |

The default is `conjunctive_hard_pareto_soft` (PRD-01 Section 3).

### 7.3 Profile Inheritance

Profiles can extend other profiles, inheriting criteria and overriding parameters:

```toml
[profile]
name = "strict-saas-landing"
extends = "@bob/saas-landing-page@^2.0"

# Override a criterion's parameter
[[criteria]]
ref = "roko/apca-contrast@^1.0"
params = { min_contrast = 90.0 }  # Stricter than base profile's 60.0

# Add a new criterion (not in base profile)
[[criteria]]
ref = "@alice/ecommerce-checkout-contrast@^1.2"
severity = "hard"

# Fill a slot from the base profile
[[slot_fills]]
slot = "brand_check"
ref = "@myteam/brand-tokens-check@^1.0"
```

### 7.4 Composition Safety

When composing artifacts from multiple sources:

1. **No circular dependencies**: the resolver detects and rejects cycles.
2. **Evidence compatibility**: all criteria in a profile must have their
   evidence requirements satisfiable by the available collectors.
3. **License compatibility**: profile license must be compatible with all
   referenced criteria licenses.
4. **Version compatibility**: `roko_version_min` across all artifacts must
   be satisfiable by the current roko version.

---

## 8. Fork Lineage and Attribution

### 8.1 Fork Chain

Every forked artifact records its full lineage:

```rust
/// File: crates/roko-eval-community/src/lineage.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkLineage {
    /// Immediate parent: the artifact this was forked from.
    pub parent: Option<ArtifactRef>,
    /// Full chain of ancestors, oldest first.
    pub chain: Vec<ArtifactRef>,
    /// Changes made relative to the parent.
    pub changes: Vec<ForkChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkChange {
    pub change_type: ForkChangeType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForkChangeType {
    ParameterChanged { param: String, old: String, new: String },
    CriterionAdded { criterion: String },
    CriterionRemoved { criterion: String },
    EvidenceChanged,
    ScoringChanged,
    RubricDimensionAdded { dimension: String },
    RubricDimensionRemoved { dimension: String },
    Other,
}
```

### 8.2 Attribution Display

Fork lineage is displayed in the artifact card:

```
@you/strict-ecommerce-contrast v1.0.0
  Forked from @alice/ecommerce-checkout-contrast v1.2.0
  Changes: +cta_min_contrast (75 -> 95), +payment_selector override
```

### 8.3 Upstream Notifications

When a forked-from artifact releases a new version, forkers receive a
notification suggesting they review the upstream changes. This is opt-in
and does not auto-update the fork.

---

## 9. Flywheel Integration

### 9.1 Evaluation Traces Feed the Marketplace

When a user runs an installed criterion, the evaluation trace (PRD-05) includes
the criterion's artifact reference. This enables:

- **Verified run counting**: anonymous run reports build trust signals.
- **Pass rate tracking**: marketplace shows aggregate pass rates.
- **Canary correlation**: rubrics are tested against shared canary sets.

### 9.2 Pattern Library -> Published Criteria

The nightly pattern extraction (PRD-05, Step 4) discovers reusable patterns.
When a pattern has high support count and consistently high scores, the system
can suggest publishing it as a criterion:

```
Pattern "pricing-table-3-tier" has been extracted 47 times with avg score 0.89.
Suggest publishing as a criterion? [y/N]
```

### 9.3 Canary Sets Calibrate Community Rubrics

Published canary sets serve as calibration benchmarks for community rubrics.
A rubric's trust score depends on its correlation with the highest-quality
available canary set:

```rust
/// File: crates/roko-eval-community/src/calibration.rs
pub struct RubricCalibrationResult {
    pub rubric_ref: ArtifactRef,
    pub canary_ref: ArtifactRef,
    pub spearman_rho: f64,
    pub agreement_rate: f64,
    pub calibrated_at: String,
    pub badge_earned: Option<TrustBadge>,
}
```

### 9.4 Collective Learning

When many users run the same criterion across different domains, the aggregate
data reveals domain-specific patterns:

- "This criterion has a 95% pass rate on SaaS dashboards but only 62% on
  e-commerce checkout flows." -> suggests domain-specific forks.
- "This rubric's correlation with human ratings is 0.82 for landing pages
  but 0.54 for documentation sites." -> suggests rubric is domain-limited.

This information is surfaced in the artifact's marketplace page.

---

## 10. Integration with Existing Crates

### 10.1 roko-learn Integration

The marketplace integrates with the existing learning subsystem:

| `roko-learn` Component | Integration |
|---|---|
| `ExperimentStore` | Published rubric variants become experiments |
| `CascadeRouter` | Criteria performance feeds model routing |
| `PlaybookStore` | Published patterns become playbook entries |
| `FeedbackService` | Criterion outcomes feed knowledge scoring |
| `EpisodeLogger` | Installed criterion refs recorded in episodes |

### 10.2 roko-neuro Integration

Published knowledge bundles are ingested into the neuro store:

```rust
/// File: crates/roko-eval-community/src/neuro_bridge.rs
///
/// Ingest a published knowledge bundle into the local neuro store.
/// Each entry becomes a KnowledgeEntry with Kind::ExternalBundle
/// and tier Transient (must prove itself to be promoted).
pub async fn ingest_knowledge_bundle(
    bundle: &KnowledgeBundle,
    neuro_store: &KnowledgeStore,
) -> Result<u32, NeuroError> {
    let mut ingested = 0;
    for entry in &bundle.entries {
        let knowledge = KnowledgeEntry {
            content: entry.content.clone(),
            kind: KnowledgeKind::ExternalBundle,
            tier: KnowledgeTier::Transient,
            source: format!("bundle:{}", bundle.artifact_ref),
            confidence: 0.6, // Start moderate, prove through use
            tags: entry.tags.clone(),
            ..Default::default()
        };
        neuro_store.ingest(knowledge)?;
        ingested += 1;
    }
    Ok(ingested)
}
```

### 10.3 roko-gate Integration

Installed criteria are available as gates in the pipeline:

```rust
/// File: crates/roko-eval-community/src/gate_bridge.rs
///
/// Convert an installed community criterion into a Verify impl
/// that can be pushed into the existing GatePipeline.
pub fn criterion_to_gate(
    criterion: &InstalledCriterion,
    evidence_bag: &EvidenceBag,
) -> Result<Box<dyn Verify>, EvalError> {
    match criterion.kind {
        CriterionKind::Deterministic => {
            Ok(Box::new(DeterministicCriterionGate::new(criterion)))
        }
        CriterionKind::Script => {
            Ok(Box::new(ScriptCriterionGate::new(criterion)))
        }
        CriterionKind::JudgePanel => {
            Ok(Box::new(JudgePanelCriterionGate::new(criterion)))
        }
    }
}
```

### 10.4 roko-serve Integration

The marketplace is exposed through the HTTP control plane at
`crates/roko-serve/src/routes/`. New routes:

| Route | Method | Description |
|---|---|---|
| `/api/eval/search` | GET | Search installed + community artifacts |
| `/api/eval/install` | POST | Install an artifact |
| `/api/eval/uninstall` | POST | Uninstall an artifact |
| `/api/eval/publish` | POST | Publish a local artifact |
| `/api/eval/fork` | POST | Fork an artifact |
| `/api/eval/installed` | GET | List installed artifacts |
| `/api/eval/updates` | GET | Check for available updates |

---

## 11. Offline and Private Modes

### 11.1 Air-Gapped Deployment

For organizations that cannot access the public registry:

1. **Private registry**: self-hosted registry API behind corporate firewall.
   Same API contract, different base URL.
2. **Bundle import**: manually download `.rokoeval` files and install from
   local path: `roko eval install ./path/to/artifact.rokoeval`.
3. **Mirror mode**: periodically sync from public registry to private mirror.

### 11.2 Organization-Scoped Sharing

Organizations can share artifacts within their namespace without publishing
to the public registry:

```toml
# roko.toml
[eval.registry]
url = "https://registry.acme-corp.internal/v1"
fallback_url = "https://registry.roko.dev/v1"
```

Artifacts in the `@acme-corp/` namespace are only visible to authenticated
members of the organization.

### 11.3 Local-Only Development

During development, criteria can be authored and tested locally without
ever publishing:

```bash
# Create a new criterion from template
roko eval new criterion my-criterion

# Test locally
roko eval test ./my-criterion/ --sample ./test-artifacts/

# Use in a local profile
# reference via relative path instead of registry ref
[[criteria]]
ref = "local:./my-criterion"
severity = "soft"
```

---

## 12. Governance and Curation

### 12.1 Official Criteria Curation

The `roko/` namespace is curated by the roko team. Criteria in this namespace:

- Are maintained and tested with every roko release.
- Have comprehensive documentation and sample artifacts.
- Are calibrated against canary sets with known Krippendorff alpha.
- Follow a strict deprecation policy (minimum 6 months notice).

### 12.2 Community Curation: Featured Lists

The roko team curates featured lists that highlight high-quality community
artifacts:

- **"Best of" by domain**: Best criteria for e-commerce, SaaS, mobile, etc.
- **"Getting Started" bundles**: Recommended profiles for common use cases.
- **"Weekly picks"**: Newly published artifacts reviewed by the team.

Featured artifacts receive additional visibility in search results and the
dashboard Community tab.

### 12.3 Deprecation and Yanking

**Deprecation**: a published version is marked as deprecated. Existing installs
continue working, but new installs receive a warning. The deprecation message
suggests a replacement.

**Yanking**: a published version is removed from new installations. Existing
installs continue working (they already have the bundle locally). Yanking is
for security issues or legal problems, not quality issues.

```bash
# Deprecate a version
roko eval deprecate @alice/old-criterion@1.0.0 --message "Use v2.0+ instead"

# Yank a version (security issue)
roko eval yank @alice/old-criterion@1.0.0 --reason "Security vulnerability in script"
```

---

## 13. Implementation Plan

### Phase 1: Core Registry + CLI (Weeks 1-4)

| File | What |
|---|---|
| `crates/roko-eval-community/src/lib.rs` | Crate root, re-exports |
| `crates/roko-eval-community/src/registry.rs` | Registry client (publish, search, download) |
| `crates/roko-eval-community/src/package.rs` | `.rokoeval` bundle creation and extraction |
| `crates/roko-eval-community/src/namespace.rs` | Namespace resolution, version specifiers |
| `crates/roko-eval-community/src/resolver.rs` | Dependency resolution, lock file |
| `crates/roko-eval-community/src/install.rs` | Local installation management |
| `crates/roko-eval-community/src/signature.rs` | Ed25519 signing and verification |

### Phase 2: Trust + Fork (Weeks 4-6)

| File | What |
|---|---|
| `crates/roko-eval-community/src/trust.rs` | Verified runs, reputation, badges |
| `crates/roko-eval-community/src/lineage.rs` | Fork chain, attribution, change tracking |
| `crates/roko-eval-community/src/security.rs` | Script security scanning |
| `crates/roko-eval-community/src/calibration.rs` | Rubric calibration against canary sets |

### Phase 3: Integration (Weeks 6-9)

| File | What |
|---|---|
| `crates/roko-eval-community/src/neuro_bridge.rs` | Knowledge bundle -> neuro store |
| `crates/roko-eval-community/src/gate_bridge.rs` | Criterion -> Verify impl |
| `crates/roko-eval-community/src/learn_bridge.rs` | Integration with roko-learn subsystem |
| `crates/roko-cli/src/commands/eval.rs` | CLI subcommands (search, install, publish, fork) |
| `crates/roko-serve/src/routes/eval.rs` | HTTP routes for marketplace |

### Phase 4: Governance + Dashboard (Weeks 9-12)

| File | What |
|---|---|
| `crates/roko-eval-community/src/curation.rs` | Featured lists, deprecation |
| `crates/roko-eval-community/src/private.rs` | Private registry, org-scoped sharing |
| `demo/demo-app/src/pages/EvalsLibrary.tsx` | Dashboard Community tab |
| `demo/demo-app/src/pages/ArtifactDetail.tsx` | Artifact detail page |

### Phase 5: Registry Backend (Weeks 12-16)

| Component | Technology | Notes |
|---|---|---|
| API server | Axum (Rust) | Same stack as roko-serve |
| Storage | S3-compatible | `.rokoeval` bundles |
| Database | PostgreSQL | Metadata, search index, reputation |
| Search | Meilisearch | Full-text search with typo tolerance |
| CDN | Cloudflare R2 | Bundle distribution |

---

## 14. Open Questions

1. **Monetization**: should the marketplace be free for all, freemium (free
   criteria, paid profiles), or marketplace-fee (take a cut of paid artifacts)?

2. **Quality floor**: should there be a minimum quality bar for publishing,
   or allow everything and let trust signals sort it out?

3. **Cross-framework**: should `.rokoeval` bundles be usable outside roko?
   Could criteria be consumed by other evaluation frameworks?

4. **Telemetry privacy**: verified run reports are anonymous, but could
   aggregate patterns reveal sensitive information about a team's workflow?

5. **Forked artifact divergence**: when a fork diverges significantly from
   its parent, should the lineage chain still be displayed? At what point
   does a fork become a new independent artifact?

6. **Registry federation**: should organizations be able to run private
   registries that federate with the public registry, or is the private
   registry model (Section 11.1) sufficient?
