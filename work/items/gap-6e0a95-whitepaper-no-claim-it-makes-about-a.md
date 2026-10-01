+++
id = "gap-6e0a95"
kind = "gap"
title = "Whitepaper: no claim it makes about a cited work has been checked against that work's full text"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["docs/whitepaper"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "4984c436a"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (coordinator, after gap-11845c found unsupported claims in 62% of the docs/v3 works read in full)"
anchors = ["docs/whitepaper/10-related-work.md", "docs/whitepaper/references.bib", "docs/whitepaper/data/"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-11845c", "gap-1f72ac", "gap-08d9b2", "gap-8117a8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json;d=json.load(open('docs/whitepaper/data/citation-content-audit.json'));assert d['complete'] is True and len(d['works'])>=40\" && python3 tools/paperlint.py --strict docs/whitepaper/0*.md docs/whitepaper/10-related-work.md"
+++

## Problem

The whitepaper cites 44 works (27 in §10). The companion audit and the citation checker verified their metadata (titles, authors, ids, years), but nobody has read the cited papers against what the whitepaper says about them. In docs/v3, agent-written descriptions turned out unsupported for 62% of the works read in full (gap-11845c), and the whitepaper was drafted the same way.

## Why it matters

The whitepaper is about to be tagged (gap-8117a8). A misdescribed related work is the kind of error a reader checks first.

## Plan

1. For every work the whitepaper cites, read the full text (arXiv HTML or the publisher's version) and check each sentence that describes it: method, results, numbers, positioning.
2. Fix what the paper doesn't support, with a section reference where useful, and keep the whitepaper's claims about Roko unchanged.
3. Record each work, its verdict (supported, corrected, removed), the sentence(s) checked and the evidence in `docs/whitepaper/data/citation-content-audit.json` with `"complete": true`.
4. `paperlint --strict` must still pass, and the status matrix must not change.

## Done when

- [x] Every cited work is checked and recorded.
- [x] The `[[verify]]` command passes.

## Notes

2026-10-01 (wk-rp-cite). The whitepaper cites 50 keys, which is every entry in references.bib, not 44. Each citing
sentence was compared with the work's full text:
- arXiv HTML for the 39 arXiv works;
- the vendor documentation page for the 8 product citations;
- the 1960 edition of Ashby on archive.org;
- the authors' PDF of Brun et al.

Filieri et al. could only be checked against its abstract: HAL, Lund, Imperial and CiteSeerX refused automated
download. The claim concerns only the paper's scope.

Result: 45 supported, 5 corrected, 0 removed (10%, all minor). The whitepaper's numbers all match their sources:
METR's 4-6x, Minions' 97.9% at 5.7x, SpecBench's 28 points per tenfold, Brun's 16% and 7%, Scrouting's 266 tasks
and its no-router tie, UTBoost's 345. The corrections:
- **§10:** "Self-improving harnesses converge on one guard: test each edit on held-out and anchor tasks, and commit only
  what passes, with rollback" credited Roko's own guard (§5) to Kang, Tayebati and Xia. They share a weaker guard: an
  edit persists only if it improves without regressing beyond a margin on protected or previously passing tasks.
  Tayebati says it does not measure held-out transfer, RRSI uses held-out sets only for evaluation, and none describes
  rollback. "Roko is designed to adopt the guard" is unchanged.
- **§6:** the `opusplan` sentence cited only the advisor page, which names opusplan in passing. It now cites the
  model-configuration page as well, and describes the advisor tool.
- **§6:** Factory's router "escalates when one struggles" is not on its page. Reworded to the page: it "reserves
  stronger models for work that needs deeper reasoning".

paperlint --strict is clean on the 11 files, and the status matrix is untouched. Verdicts, the sentences checked and
the evidence for every work are in `docs/whitepaper/data/citation-content-audit.json`.

