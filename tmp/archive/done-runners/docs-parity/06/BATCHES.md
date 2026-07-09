# Batch Execution Contract

This parity bundle is for **documentation refresh only** under
`tmp/docs-parity/06/`.

## Batch Posture

- Rewrite the docs to match shipped neuro/HDC reality.
- Do not propose large implementation programs inside these batch notes.
- Mark target-state concepts explicitly when there is no code.
- Keep each batch realistic for one Codex agent in about 90 minutes.

## Recommended Order

`N1 -> N2 -> N3 -> N4 -> N5 -> N6`

## Batch Overview

| Batch | Purpose | Write Scope | Verify Focus |
|-------|---------|-------------|--------------|
| `N1` | Refresh the overview and execution posture around the audit | `00-INDEX.md`, `BATCHES.md`, context-pack | `rg -n "HDC fingerprint|deferred|target-state" tmp/docs-parity/06` |
| `N2` | Rewrite knowledge / tier / decay parity around what ships today | `A-knowledge-types-tiers-decay.md` | `rg -n "demurrage|tier progression|Engram" tmp/docs-parity/06/A-knowledge-types-tiers-decay.md` |
| `N3` | Rewrite HDC parity around the real `HdcVector` and no-extra-crate story | `B-hdc-foundations-operations.md` | `rg -n "HdcVector|roko-hdc|Engram" tmp/docs-parity/06/B-hdc-foundations-operations.md` |
| `N4` | Refresh query / context docs and defer cross-domain transfer honestly | `C-query-crossdomain-context.md` | `rg -n "query_similar|Substrate|cross-domain|deferred" tmp/docs-parity/06/C-query-crossdomain-context.md` |
| `N5` | Refresh distillation, somatic, exchange, and backup scope | `D-distillation-progression.md`, `E-somatic-exchange-backup.md` | `rg -n "Distiller|TierProgression|Library of Babel|deferred" tmp/docs-parity/06/D-distillation-progression.md tmp/docs-parity/06/E-somatic-exchange-backup.md` |
| `N6` | Refresh frontier status, source anchors, audit log, and runner text | `F-status-frontier.md`, `SOURCE-INDEX.md`, `AUDIT-LOG.md`, `run-docs-parity.sh` | `bash -n tmp/docs-parity/06/run-docs-parity.sh` |

## Batch Details

### N1 — Overview And Context Pack

Owns the docs posture for PU06.

Deliverables:

- make HDC-on-Engram the top priority
- separate shipping vs partial vs deferred
- update context-pack notes for a docs-only pass

Out of scope:

- any code change under `crates/`
- any attempt to design a new roadmap

### N2 — Knowledge, Tiers, Decay

Deliverables:

- keep `KnowledgeEntry`, `KnowledgeKind`, and tier progression in present tense
- keep demurrage and worldview language in target-state tense
- point readers at Engram fingerprinting as the next bridge

### N3 — HDC Foundations

Deliverables:

- state that `HdcVector` already exists and works
- state that no separate `roko-hdc` crate is needed
- narrow HDC work to retrieval / clustering and Engram fingerprinting

### N4 — Query, Context, Cross-Domain

Deliverables:

- keep current neuro query surfaces in present tense
- note that `Substrate` has no `query_similar()`
- explicitly defer resonance / analogy / cross-domain transfer

### N5 — Distillation, Somatic, Exchange

Deliverables:

- describe `Distiller` and `TierProgression` as wired
- keep somatic retrieval bias as real
- move Library of Babel, exchange channels, backup / publish flows to deferred

### N6 — Frontier, Sources, Runner

Deliverables:

- update status/frontier language to audit reality
- simplify `SOURCE-INDEX.md` to the anchors that matter now
- append the PU06 refresh to the audit log
- update runner descriptions and shell verification text
