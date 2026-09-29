+++
id = "gap-3c430e"
kind = "gap"
title = "Specs S01, S05, S06 and S08 disagree on token classes, the audit hash, a decision-point name and a budget-line name"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["cybernetic-harness/specs"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:46 wk-bench-tree; 14:50 wk-rp-appx findings 1-2; 15:04)"
anchors = ["tmp/cybernetic-harness/specs/S01-instrumentation.md", "tmp/cybernetic-harness/specs/S05-deep-audits.md", "tmp/cybernetic-harness/specs/S06-ultrastable-controller.md", "tmp/cybernetic-harness/specs/S08-benchmark-suite.md", "benchmarks/viabilitybench/schema/run-record.schema.json"]
lane = "paper"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["gap-528762", "gap-b605cf", "gap-33d54b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -qi 'disjoint' tmp/cybernetic-harness/specs/S01-instrumentation.md && grep -q 'harness_policy' tmp/cybernetic-harness/specs/S01-instrumentation.md && ! grep -q '"record_hash":"b3:' tmp/cybernetic-harness/specs/S01-instrumentation.md && test -f tmp/cybernetic-harness/specs/S08-benchmark-suite.md && ! grep -q -e '--line B1\]' tmp/cybernetic-harness/specs/S08-benchmark-suite.md'''
+++

## Problem

Four places where the v1.1 specs contradict each other or the merged benchmark schema:

1. **Token classes (S01 §4.4).** S01 defines the cost fields and prices `cache_read` separately, but never says that
   the token classes are disjoint: that `tokens_in` counts uncached input only, and cache reads, cache writes and
   reasoning tokens are counted apart. Its usage example (`S01-instrumentation.md:310`) also has no cache-write
   fields. The merged run-record schema says the classes are disjoint and carries `tokens_cache_write_5m` and
   `tokens_cache_write_1h` (`benchmarks/viabilitybench/schema/run-record.schema.json:76-84`), and the price snapshot
   prices each class separately. Providers differ here: an OpenAI-style `prompt_tokens` includes cached tokens.
2. **Audit hash (S01 §5.7 against S05).** S05 specifies a SHA-256 chain for the audit ledger (`prev_hash`,
   `record_hash`; `S05-deep-audits.md:315`) and a sha256 key commitment (`:149`), and checklist row S05.4 tests "the
   SHA-256 ledger chain". S01 §5.7's AuditRecord examples use BLAKE3 `b3:` prefixes for `record_id`, `prev_hash` and
   `record_hash`. S01 §5.7 says the battery and estimators are S05's, so S05's SHA-256 should win. The `b3:` prefix
   probably came from the config hash (D32), which is a different hash.
3. **Decision point name (S01 against S06).** S01's `decision_point` list (`S01:183`, `:284`) names S06's decision
   `params`, but S06 logs `decision_point = "harness_policy"` (`S06-ultrastable-controller.md:314`, T9 at `:374`).
4. **Budget line (S08 §5.7).** The example command passes `--line B1` (`S08-benchmark-suite.md:395`). S09.E2 defines
   the lines as BL0–BL11.

## Why it matters

gap-528762 (S01.P0-0, the attempt records library) is implementing S01's records now. The benchmark driver and ledger
(gap-28ebea, gap-33d54b) read S08 and S09. A cost computed on overlapping token classes double-charges cached input,
and a verifier written from S01's example would reject S05's ledger. The research paper's App. E describes these
formats as well. Part of epic spec-b7303f.

## Where

The four spec files in `tmp/cybernetic-harness/specs/`, at the lines above. The schema and
`config/prices/2026-09-28.toml` are already consistent and need no change.

## Current state

Checked on 2026-09-29 against the specs in MAIN and `4c0326dfc`: all four are as described.

## Plan

1. **S01 §4.4:** state that the classes are disjoint, map each provider's usage fields onto them (Anthropic
   `input_tokens` excludes cache reads and writes; OpenAI-style `prompt_tokens` includes cached tokens, so subtract
   them), and add the two cache-write fields to the §5.5 usage example.
2. **S01 §5.7:** use `sha256:` in the AuditRecord examples, as S05 does.
3. **S01 §4.5 and §5.x:** replace `params` with `harness_policy` in the decision-point lists.
4. **S08 §5.7:** `--line BL1`.
5. Add a v1.2 line to each changed spec's change log, and tell the owners of gap-528762 and gap-33d54b.

## Done when

- [ ] The four inconsistencies are fixed in the specs, each with a change-log line.
- [ ] The `[[verify]]` command passes.

## Notes

- The specs are untracked, so edit them in place in the main checkout.
- Don't change `checklist.json`.
- gap-b605cf fixes the matching v1 names in the research paper's `FIGURES-TABLES.md`: the cost sources in T9 and the
  budget lines.
