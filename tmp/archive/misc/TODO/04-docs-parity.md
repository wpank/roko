# docs-parity/, docs-parity-meta/, docs-parity2/ — Parity Systems

**Status**: All DONE — completed artifacts, no remaining action

## docs-parity/ — Post-Audit Verification (13 Batches)

**Directory**: `tmp/docs-parity/`
**Generated**: 2026-04-18
**Files**: 201 files across batches 00-12
**Accuracy**: 95%+ (all spot-checks verified)

A docs verification framework tracking ~387 items:
- 44% "keep" (docs match code)
- 17% "DONE"
- 17% "planned" (future work)
- 12% "DEFERRED" (Phase 2+)
- 10% "PARTIAL"

**Verdict**: Trustworthy source of truth. No hallucinated implementations found. Code anchors (file paths, line numbers) verified correct.

### Source Files

- Per-batch parity analysis: `tmp/docs-parity/{00..12}/`
- Context packs: `tmp/docs-parity/{batch}/context-pack/`
- Source indexes: `tmp/docs-parity/{batch}/SOURCE-INDEX.md`

---

## docs-parity-meta/ — Batch Generation System

**Directory**: `tmp/docs-parity-meta/`
**Purpose**: Shell pipeline that generated `docs-parity2/`
**Files**: `generate.sh` + `lib/` (6 modules) + `templates/` (3 templates)

Scans docs, scans crates, renders prompts for 21 batches. Reusable if you want to re-run parity against updated docs.

### Source Files

- Generator: `tmp/docs-parity-meta/generate.sh`
- Section map (21 batches): `tmp/docs-parity-meta/lib/section-map.sh`
- Prompt templates: `tmp/docs-parity-meta/templates/*.tmpl`

---

## docs-parity2/ — Full Code-Generation Run (21 Batches)

**Directory**: `tmp/docs-parity2/`
**Executed**: 2026-04-18 21:13:37 UTC
**Result**: 21/21 batches SUCCESS (100%)
**Model**: gpt-5.4, high reasoning
**All commits merged to main.**

| Batch | Section | Crates | Status |
|-------|---------|--------|--------|
| DP00 | Architecture | roko-core | DONE |
| DP01 | Orchestration | roko-orchestrator, roko-cli | DONE |
| DP02 | Agents | roko-agent | DONE |
| DP03 | Composition | roko-compose | DONE |
| DP04 | Verification | roko-gate | DONE |
| DP05 | Learning | roko-learn | DONE |
| DP06 | Neuro | roko-neuro, roko-primitives | DONE |
| DP07 | Conductor | roko-conductor | DONE |
| DP08 | Chain | roko-chain | DONE (stubs) |
| DP09 | Daimon | roko-daimon | DONE (stubs) |
| DP10 | Dreams | roko-dreams | DONE (stubs) |
| DP11 | Safety | roko-agent | DONE |
| DP12 | Interfaces | roko-cli, roko-serve, roko-agent-server | DONE |
| DP13 | Coordination | roko-orchestrator | DONE |
| DP14 | Identity-Economy | roko-chain | DONE (stubs) |
| DP15 | Code-Intelligence | roko-index, roko-mcp-code, roko-lang-* | DONE |
| DP16 | Heartbeat | roko-runtime | DONE |
| DP17 | Lifecycle | roko-agent, roko-runtime | DONE |
| DP18 | Tools | roko-std, roko-agent | DONE |
| DP19 | Deployment | roko-cli | DONE |
| DP20 | Technical Analysis | cross-cutting | DONE |

### Source Files

- Runner: `tmp/docs-parity2/run-docs-parity2.sh`
- Manifest: `tmp/docs-parity2/BATCHES.md`
- Context pack: `tmp/docs-parity2/context-pack/`
- Logs: `tmp/docs-parity2/logs/run-20260418-211337/`
- Per-batch prompts: `tmp/docs-parity2/logs/run-20260418-211337/prompts/`
