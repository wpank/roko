# PRD-07 — Doc-Ingest Worked Example

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Prerequisites**: PRD-00 through PRD-06

---

## 0. Scope

This document walks the `doc-ingest` Workflow end-to-end as a concrete demonstration of the workflow subsystem. The example is the originating use case: ingesting `/Users/will/dev/nunchi/nunchi-dashboard/tmp/ux-refresh/` (89 markdown files in 10 sections) into the `nunchi-dashboard` workspace, producing PRDs, plans, and tasks that roko can execute.

This PRD is the integration test specification. If everything in this document executes successfully, the Tier 0 + Tier 1 catalog is complete enough to self-host doc ingestion.

---

## 1. The Source Material

```
/Users/will/dev/nunchi/nunchi-dashboard/tmp/ux-refresh/
├── README.md
├── 00-foundation/        (6 files: principles, glossary, personas, thesis, source-map, index)
├── 10-design-system/     (10 files: tokens, typography, glass, motion, ...)
├── 20-navigation/
├── 30-decoupling/
├── 40-pages/
├── 50-authoring/
├── 60-marketplace/       (7 files: browse, publish, install, ratings, attribution, backend)
├── 70-cross-cutting/
├── 80-tasks/             (multiple files in pre-shaped task format)
└── 99-references/
```

Plus the cross-cutting context:

```
/Users/will/dev/nunchi/nunchi-dashboard/tmp/ux-refresh-context/
├── doc-1-current-state-audit.md
├── doc-2a-will-workflow.md
├── doc-2b-session-replay.md
├── doc-3-optimal-redesign.md
├── doc-4-context-transfer.md
└── uxresearch.md
```

`ux-refresh-context/` is *philosophy* — it should be loaded as a Knowledge Bundle and included as context in every PRD synthesis, not ingested as work items.

---

## 2. The Setup

### 2.1 Open the workspace

```bash
$ cd /Users/will/dev/nunchi/nunchi-dashboard
$ roko workspace new . --template web-app
Created workspace 'nunchi-dashboard' at /Users/will/dev/nunchi/nunchi-dashboard
Registered in ~/.roko/workspaces.json
Active workspace: nunchi-dashboard
```

This creates `workspace.toml`, a minimal `.roko/` skeleton, and registers the workspace.

### 2.2 Install or load the doc-ingest Workflow

`doc-ingest` ships built-in (Tier 1), no install needed. Verify:

```bash
$ roko workflow show doc-ingest
doc-ingest@1.0.0  (builtin)
description: Ingest a directory of markdown into PRDs, plans, and tasks
macros:
  enable_audit            bool   default: true
  enable_web_research     bool   default: true
  max_refine_iterations   int    default: 2     range: 1..=5
  synthesizer_model       model  default: claude-opus-4-7
  cluster_granularity     enum   default: auto  variants: auto|section|file
  budget_usd              money  default: 5.00
slots:
  researcher              optional, defaults to perplexity-search@^1
input.required:
  source_dir              string
  context_bundle          optional path
output:
  created_prds            array
  created_plans           array
  audit_report            string
```

### 2.3 Load the context as a Knowledge Bundle

```bash
$ roko knowledge-ingest \
    --kind context-bundle \
    --tags "nunchi,philosophy,redesign" \
    --source /Users/will/dev/nunchi/nunchi-dashboard/tmp/ux-refresh-context/
Bundle id: kb_nunchi_philosophy_2026_04_25
Entries: 6 docs, 1929 lines, ~24k tokens
Available to Workflows via slot.context_bundle = "kb_nunchi_philosophy_2026_04_25"
```

---

## 3. The One-Line Run

```bash
$ roko run doc-ingest \
    --input source_dir=tmp/ux-refresh/ \
    --input context_bundle=kb_nunchi_philosophy_2026_04_25 \
    --macro enable_web_research=true \
    --macro budget_usd=10.00
```

Or equivalently as a workflow input file:

```bash
$ cat > /tmp/ingest.toml <<EOF
[input]
source_dir      = "tmp/ux-refresh/"
context_bundle  = "kb_nunchi_philosophy_2026_04_25"

[macros]
enable_web_research    = true
budget_usd             = 10.00
EOF

$ roko run doc-ingest --from-file /tmp/ingest.toml
```

The CLI prints:

```
Workflow: doc-ingest@1.0.0 (resolved from registry)
Run id: wf_01HGZK7B9XVJ4P8TRYM3N8DSWE
Estimated cost: $4.20  (range: $2.50–$8.00)
Estimated time: 8m 30s
Capabilities required: fs.read, fs.write, llm, net (api.perplexity.ai, arxiv.org)
This workspace grants: ✓ all
Continue? [Y/n]
```

After confirmation, the engine begins streaming events.

---

## 4. The State Graph

The state graph for `doc-ingest`:

```
                          ┌──────────┐
                          │   walk   │  fs.read, walks tmp/ux-refresh/
                          └────┬─────┘
                               │  files: [{path, mime, size, ...}, ...]
                          ┌────▼─────┐
                          │ segment  │  splits each file by markdown heading
                          └────┬─────┘
                               │  segments: [{text, source: {path, lines}, ...}, ...]
                          ┌────▼─────┐
                          │ classify │  llm: tags each segment (context|task|spec|reference|meta)
                          └────┬─────┘
                               │  classifications: [{seg_idx, kind, confidence}, ...]
                          ┌────▼─────┐
                          │  cluster │  groups segments into PRD-sized clusters
                          └────┬─────┘
                               │  clusters: [{name, members[], dominant_kind}, ...]
                          ┌────▼─────┐ FanOut over clusters (parallelism: 4)
                          │  fan-out │
                          └────┬─────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
       ┌──────▼──────┐  ┌──────▼──────┐ ...
       │ synthesize  │  │ synthesize  │
       └──────┬──────┘  └──────┬──────┘
              │  prd_md         │  prd_md
       ┌──────▼──────┐  ┌──────▼──────┐
       │   enrich    │  │   enrich    │  (gated on enable_web_research macro)
       └──────┬──────┘  └──────┬──────┘
              │                │
       ┌──────▼──────┐  ┌──────▼──────┐
       │   audit     │  │   audit     │  prd-audit module
       └──────┬──────┘  └──────┬──────┘
              │                │
       ┌──────▼──────┐  ┌──────▼──────┐
       │ refine-loop │  │ refine-loop │  loop until audit findings clear, max 2 iters
       └──────┬──────┘  └──────┬──────┘
              │                │
       ┌──────▼──────┐  ┌──────▼──────┐
       │    plan     │  │    plan     │  prd-plan module → tasks.toml
       └──────┬──────┘  └──────┬──────┘
              │                │
              └────────┬───────┘
                       │  FanIn (concat artifacts)
                ┌──────▼──────┐
                │   persist   │  artifact-persist → .roko/prd/, .roko/plans/
                └──────┬──────┘
                       │
                ┌──────▼──────┐
                │    index    │  knowledge-ingest of new PRDs
                └──────┬──────┘
                       │
                ┌──────▼──────┐
                │   report    │  produces audit_report and final summary
                └─────────────┘
```

The `80-tasks/` subtree is detected during classify (every segment is `kind = task` with high confidence) and routed through a shorter sub-pipeline that bypasses synthesize / enrich / audit / refine and goes directly to plan-passthrough → persist.

---

## 5. Per-Pass Walkthrough

### 5.1 walk

`fs-walk` Module. Capabilities: `fs.read`. Inputs: `{source_dir, patterns, ignore}`. Output: `{files: [{path, mime, size, mtime, content_hash}]}`.

For incremental runs (`incremental = true`), compares each file's content_hash against the previous run's snapshot stored at `.roko/runs/<latest-doc-ingest>/files-manifest.json`. Only changed files are forwarded.

Output for the example dir: 89 files in 10 categories.

### 5.2 segment

`markdown-segment` Module. Capabilities: none. Splits each file by markdown headings (default depth 2: H1 + H2). Each segment carries provenance: `{path, start_line, end_line, content_hash, heading_path}`.

For mixed-content docs (e.g., a doc with both context paragraphs and a checklist), segmentation creates one segment per heading-bounded chunk. The classifier (next pass) then tags each chunk independently.

For the example: 89 files → ~340 segments.

### 5.3 classify

`markdown-classify` Module. Capabilities: `llm`. Tags each segment with kind (`context | spec | task | reference | meta`) and confidence. Uses a fast model (`scribe` role; defaults to haiku-class model).

The Module batches segments per file to share context window, calls the LLM once per file, parses the structured output. Includes the context bundle as system prompt context so classification is informed by the philosophy docs.

For the example: ~340 segments classified. `80-tasks/*.md` segments classify almost entirely as `kind = task`. `00-foundation/*.md` classify as `kind = context | spec`. Marketplace/personas/principles classify as `kind = spec`.

### 5.4 cluster

`doc-cluster` Module. Capabilities: `llm`. With `cluster_granularity = "auto"`, the module decides cluster boundaries based on cohesion. Default heuristic: one cluster per top-level section directory, but the LLM may split a section if topics are sufficiently distinct.

For the example: 10 sections → likely 11–14 clusters (a couple of sections split). Each cluster has a `name`, `description`, `members[]` (segment indices), `dominant_kind`, and `expected_artifact_kind` (PRD vs task-bundle).

The `80-tasks/` cluster is marked `expected_artifact_kind = task-bundle` and routed to the passthrough subpipeline.

### 5.5 fan-out

`FanOut` engine node. `over: clusters`, `max_parallelism: 4`. Spawns parallel subgraph executions per cluster. Each child run carries a `cluster` variable in scope.

### 5.6 synthesize

`prd-synthesize` Module per cluster. Capabilities: `llm`. Inputs: cluster's segments + cluster description + context bundle. Output: a PRD markdown document with frontmatter (name, status, related, tags) and structured sections matching the workspace's PRD template.

Uses the `strategist` role; model selected by cascade router (defaults to `claude-opus-4-7`).

The PRD references its source segments inline: `(see: 40-pages.md L120-145)`. Provenance is preserved.

### 5.7 enrich (conditional)

`prd-enrich` Module via the `researcher` slot. Capabilities: `llm`, `net`. Default filling: `perplexity-search`.

For each PRD, the module:
1. Reads the synthesized PRD.
2. Identifies claims that would benefit from external grounding (assertions, recommendations, comparisons).
3. Queries the researcher slot's filling for each.
4. Inserts citations inline; appends a "References" section.

If `enable_web_research = false`, this node is skipped via edge condition.

### 5.8 audit

`prd-audit` Module per PRD. Capabilities: `llm`. Reads the PRD, looks for: contradictions, vague language, missing acceptance criteria, unsupported claims, broken cross-references, citation hallucinations.

Output: `findings: [{severity, location, message, suggestion}]`. Severity is `low | medium | high | blocker`.

### 5.9 refine-loop

`Loop` engine node. Body: `synthesize` (re-synthesize given audit findings as additional context). `until`: `audit.findings.severity_max < 'high'`. `max_iterations: 2` (from macro).

If after 2 iterations findings are still high-severity, the run continues but flags the PRD as `needs-human-review` in the final report.

### 5.10 plan

`prd-plan` Module per PRD. Capabilities: `llm`. Reads the published PRD, generates a `tasks.toml` plan with task list, dependencies, files-affected, acceptance-criteria. The `80-tasks/` passthrough subpipeline uses a different module (`tasks-passthrough`) that just transforms the pre-shaped task markdown into `tasks.toml` directly.

### 5.11 fan-in

`FanIn` engine node with `MergeStrategy::Concat`. Collects all child branches' outputs into the main thread.

### 5.12 persist

`artifact-persist` Module. Capabilities: `fs.write`. Writes:
- PRDs to `<workspace>/.roko/prd/<slug>.md`
- Plans to `<workspace>/.roko/plans/<slug>/tasks.toml` and `<workspace>/.roko/plans/<slug>/plan.md`
- Manifest of changes to `<workspace>/.roko/runs/<run-id>/output.json`

Each artifact is content-addressed and registered in the artifact store with full lineage back to source segments.

### 5.13 index

`knowledge-ingest` Module. Capabilities: `knowledge.write`. Adds each new PRD as a knowledge entry with HDC fingerprint for resonance/lineage queries.

### 5.14 report

`run-report` Module. Reads run state, produces a markdown summary: clusters created, PRDs synthesized, audit findings histogram, citation counts, total cost, total time, any human-review flags. Writes to `<workspace>/.roko/runs/<run-id>/REPORT.md`.

---

## 6. Live Output

During execution, the CLI streams progress in a TUI-style status bar (alternative views in PRD-08/09):

```
doc-ingest [wf_01HGZK7B9XVJ4P8TRYM3N8DSWE]
[██████████░░░░░░░░░░░░░░░░░░] 37%   8/22 nodes  $1.84/$10.00  4m 12s elapsed

walk            ✓  89 files, 0 changed (skipped)              0.1s
segment         ✓  340 segments                                0.4s
classify        ✓  340 segments tagged                         12.3s   $0.31
cluster         ✓  12 clusters                                 4.1s    $0.18
synthesize  →   ⠋  cluster 7/12 in flight                      ~       $1.20
enrich      →   ⠦  cluster 4/12 in flight                      ~       $0.15
audit       □   pending
refine-loop □   pending
plan        □   pending
persist     □   pending
index       □   pending
report      □   pending

Press 'd' to detach (continue in daemon), 'c' to cancel, 'p' to pause, '?' for keys.
```

The dashboard (PRD-10) renders this as a state-graph view with live node statuses, an artifact panel populating in real time, and a cost/budget gauge.

---

## 7. Idempotency & Re-Running

The user updates `40-pages.md` and re-runs:

```bash
$ roko run doc-ingest --input source_dir=tmp/ux-refresh/
Workflow: doc-ingest@1.0.0
Run id: wf_01HGZP2XR5VQK3M7TNDB6FA8WT
Resolved incremental against: wf_01HGZK7B9XVJ4P8TRYM3N8DSWE
  changed: 1 file (40-pages.md)
  affected clusters: 1 ("40-pages")
  affected PRDs: 1 (.roko/prd/40-pages.md)
Estimated cost: $0.30  (range: $0.20–$0.45)
Continue? [Y/n]
```

Only the affected cluster reruns. The other 11 clusters are short-circuited (output reused from the previous run). The PRD's lineage records both runs.

---

## 8. Audit-Triggered Refine Demonstration

Suppose `synthesize` produced a PRD claiming "the marketplace launches with 50 pre-built workflows." `audit` flags this as severity-high (no source supports it). `refine-loop` re-runs `synthesize` with the audit finding as added context: "Audit found: claim '50 pre-built workflows' has no source. Verify or remove." The synthesizer either grounds the claim (citation found) or removes it.

After max 2 iterations, if still flagged, the PRD is persisted with a `needs-human-review: yes` frontmatter field, and the final report lists it.

---

## 9. Human-in-Loop Demonstration

Suppose `cluster` is uncertain about whether to split `60-marketplace` into one or two clusters. With workspace's `human_input_default = "human"`, the engine pauses and emits:

```
[Workflow paused — human input requested]
Run: wf_01HGZK7B...
Node: cluster
Question: Should '60-marketplace' (7 docs, mixed authoring/install/attribution topics) be:
  1) one PRD (default)
  2) split into 'marketplace-publishing' and 'marketplace-discovery'
Respond: roko run respond wf_01HGZK7B... --node cluster --input '{"choice":2}'
Or respond via dashboard at https://localhost:6677/runs/wf_01HGZK7B...
Timeout: 10 minutes (default to choice 1)
```

If the user answers via dashboard or CLI within 10 minutes, the run proceeds with the chosen branching. If not, the timeout strategy applies.

---

## 10. Artifact Lineage Verification

After the run:

```bash
$ roko artifact lineage art_a3f4d2c1
art_a3f4d2c1   markdown   "PRD-40-pages-redesign.md"
  produced by  prd-synthesize@1.0.0  in run wf_01HGZK...  node synthesize-cluster-7
  sources:
    art_92bc7e1f  cluster description
    art_ac88f1d3  segment-bundle for cluster 7
      sources:
        tmp/ux-refresh/40-pages/00-overview.md  L1-220
        tmp/ux-refresh/40-pages/01-pulse.md     L1-185
        tmp/ux-refresh/40-pages/02-fleet.md     L1-310
        ...
  citations:
    [refactoring-ui.com] cited in §3.2
    [arxiv:2024.01234]   cited in §5.1
```

Lineage is fully queryable; every claim in the PRD traces to either a source segment, a citation, or an LLM generation tagged with model + prompt hash.

---

## 11. Trigger-Driven Re-Ingest

Setting up auto-reingest:

```bash
$ roko trigger create reingest-on-change \
    --kind folder-watch \
    --path tmp/ux-refresh \
    --workflow doc-ingest \
    --debounce-ms 30000 \
    --binding-input source_dir=tmp/ux-refresh \
    --binding-input incremental=true
```

Now editing any file in `tmp/ux-refresh/` triggers an incremental re-ingest 30s later. Multiple rapid edits debounce to a single run.

---

## 12. Output Layout

After a successful run:

```
<workspace>/.roko/
├── prd/
│   ├── 00-foundation.md
│   ├── 10-design-system.md
│   ├── ...
│   └── 99-references.md
├── plans/
│   ├── 00-foundation/
│   │   ├── plan.md
│   │   └── tasks.toml
│   ├── 10-design-system/
│   │   ├── plan.md
│   │   └── tasks.toml
│   └── 80-tasks/
│       └── tasks.toml         (passthrough; no plan.md)
├── artifacts/                 (content-addressed)
├── runs/
│   └── wf_01HGZK7B.../
│       ├── snapshot.json
│       ├── input.json
│       ├── output.json
│       ├── events.jsonl
│       └── REPORT.md
├── episodes.jsonl             (appended)
└── trigger-events.jsonl
```

---

## 13. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `roko run doc-ingest --input source_dir=tmp/ux-refresh/` runs end-to-end and produces PRDs in `.roko/prd/` and plans in `.roko/plans/`. | Filesystem check after run. |
| Each PRD has frontmatter listing its source segments with file + line range. | YAML frontmatter parse. |
| Re-running with `incremental = true` after touching one file processes only the affected cluster. | Cost / time reduction measured. |
| `enable_web_research = false` skips the enrich node; resulting PRDs have no citations section. | Macro toggling test. |
| Audit findings of severity `high` trigger refine-loop; loop terminates at `max_refine_iterations`. | Synthetic test with deliberately flawed input. |
| Mixed-content documents are segmented and classified at sub-doc granularity. | Inspect classifications.json for a known mixed file. |
| 80-tasks/ subtree bypasses synthesize/audit/refine and produces tasks.toml directly. | Run; verify tasks.toml exists and synthesize was not called for that cluster. |
| Lineage walk on any produced PRD returns the source segments. | `roko artifact lineage <id>`. |
| Cancellation mid-run preserves snapshot; `--resume <run-id>` continues from the snapshot. | Kill at synthesize stage; resume; verify completion. |
| Human-input nodes pause the run, accept input via CLI or dashboard, and resume correctly. | Multi-channel test. |

---

## 14. Open Questions

- Should the doc-ingest workflow auto-create a Knowledge Bundle from the source dir's "context" segments (those classified as `kind=context`)? This would automate what step 2.3 does manually. Probably yes — make it a macro `auto_context_bundle = true`.
- Should there be a `dry_run` macro that produces the cluster plan + cost estimate without actually synthesizing? Yes — add to v1.
- How are conflicts handled when the same source dir is ingested into two different workspaces? Each workspace gets independent PRDs; the `share_with` knowledge config (PRD-01 §9) determines whether they share knowledge.
- What if a PRD synthesis output is *worse* than a previously generated one (e.g., a re-ingest produces a regression)? Add a `prd-compare` Module in `prd-audit` that compares against the previous version and flags regressions; require human-approve to overwrite.
