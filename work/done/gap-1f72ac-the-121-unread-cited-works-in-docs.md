+++
id = "gap-1f72ac"
kind = "gap"
title = "The ~121 unread cited works in docs/v3 likely carry unsupported claims at the audited 62% rate: banner the references, cut unchecked annotations to bare citations"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["docs/v3", "docs/v1", "tools/docs_integrity"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "90a39c516"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's gap-11845c result: 28 of 45 audited works corrected, 62.2%)"
anchors = ["docs/v3/REFERENCES.md", "tools/docs_integrity/content_audit.json", "tools/docs_integrity/check_citation_errata.py"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-11845c", "gap-0d996a", "gap-785a4f", "gap-fc5d3d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json;d=json.load(open('tools/docs_integrity/content_audit.json'));assert d.get('unchecked_annotations_cut') is True\" && grep -q 'not been checked against' docs/v3/REFERENCES.md && python3 tools/docs_integrity/check_citation_errata.py --prose"

[closed]
at = 2026-10-01
commit = "90a39c516"
evidence = "content_audit.json: unchecked_annotations_cut=true (1,771 annotations cut: 1,132 v3 reference entries, 450 v1, 189 chapter reference items); banner in REFERENCES.md; 23 evidence works read (7 corrected); check_citation_errata.py --prose clean (905 files, 306 works); 13 tests pass"
+++

## Problem

gap-11845c read the 45 most-described cited works in full: 28 (62.2%) had a claim their paper does not support, 16 (35.6%) a major one, and the rate did not fall with rank (12/15, 8/14, 8/16 by band). The other ~121 cited arXiv works were not read, and their descriptions in docs/v3 and docs/v1 are likely wrong at a similar rate (wk-rp-cite).

## Why it matters

Release: the docs are public, and the papers point readers at the repository. Reading every remaining work in full costs far more than the descriptions are worth.

## Plan

1. Add a short banner to `docs/v3/REFERENCES.md` and the docs/v1 reference list saying which works' descriptions were checked against the papers (the `content_audit.json` list) and that the rest have not been checked against the papers.
2. Script the cut: for every cited work not in `content_audit.json`, reduce one-line annotations to the bare citation and drop numbers attributed to it. Keep the citation itself (title, authors, id, year).
3. Keep full reads only for works the docs cite as evidence for a design decision. List them in the item notes, and read them in a follow-up if there are any.
4. Record `"unchecked_annotations_cut": true` and the counts in `content_audit.json`. The checker stays clean with `--prose`.

## Done when

- [x] Unchecked annotations are bare citations, the banner is in place, and the audit file records the cut.
- [x] The `[[verify]]` command passes.

## Notes

2026-10-01 (wk-rp-cite). The cut, the banners and the follow-up reads are recorded in
`tools/docs_integrity/content_audit.json` under `unchecked_annotations_cut`, `annotation_cut`,
`evidence_candidates` and `evidence_reads`.

- **The cut.** A work counts as checked when it appears in content_audit.json: the 45 audited works plus 4 earlier full
  reads, 49 in all. Every other cited work keeps its citation (authors, year, title, venue and identifier) and loses
  its annotation and the numbers in it. Venue and award notes and "(See NN.)" pointers stay. A script
  (`cut.py`, in the session scratch) handles three formats:
  - docs/v3/REFERENCES.md and depth/39-references: 1,316 entries. 136 checked entries keep their annotations,
    1,132 were cut (1,126 annotation lines removed) and 48 were already bare.
  - docs/v1/21-references: 489 entries. 30 kept, 450 cut (their `*Grounds:*` lines) and 9 already bare.
  - Reference sections in other chapters, v3 and v1: 810 citation items. 78 kept and 189 cut. The other 543
    were left as they are; most are bare, and label-first forms such as `**Author Year** -- Title (Venue)` are not
    touched.
  - The cut does not reach in-text descriptions in chapter prose. The REFERENCES.md banner says they are unchecked.
- **Banners.**
  - docs/v3/REFERENCES.md names the 49 checked works and says the rest "have not been checked against the papers".
  - A shorter note heads each depth/39-references file and each docs/v1/21-references file.
  - docs/v1/21-references/INDEX.md has its own banner.
- **Checked works' reference items, reviewed by hand (26 fixes).** These lists were outside gap-11845c's scan. The
  fixes:
  - Invented titles: GRASP ("Grounding Retrieval-Augmented Skill and Planning for Medical Agents"), SkillZip, ReSkill
    and Meta-Harness ("Optimizing Agent Scaffolds").
  - Wrong authors: DGM as "Lange, R., Fu, Y." and "Sakana AI et al.".
  - "MDL compression for bounded skill libraries", a claim gap-11845c had already corrected elsewhere.
  - In docs/v1:
    - HAL's 21,730 rollouts still credited to the agentic-AI taxonomy paper;
    - the "CoALA 9-step" claims;
    - a FrugalGPT citation carrying Meta-Harness's ID;
    - Reflexion cited as arXiv:2308.xxxxx.
  - The new titles, authors and phrases are in citation_errata.json (306 works).
- **Evidence for design decisions (step 3).** I scanned docs/v3 prose outside the reference lists for unchecked arXiv
  works cited with an evidence or design verb (validates, grounds, demonstrates, shows, motivates, based on, ...). The
  scan found 30 works.
  - 7 carry no claim about the paper: 2202.09722, 2308.07870, 2405.10467, 2406.16008, 2210.03629, 2511.14650,
    2602.14038.
  - I read the other 23 in full: 16 supported and 7 corrected (30.4%), 4 of them major (17.4%).
  - The major ones:
    - WSCL's "38% reduction in catastrophic forgetting", which appeared in 12 places across v3 and v1 and is not in
      the paper;
    - Governed Shared Memory's "three categories", where the paper lists four other failure modes;
    - xMemory's principle, misread;
    - PACE described as "continuous verification scores at 15% of the compute cost", where the paper reports less
      than 1/100 of the cost and a different mechanism.
  - The minor ones are PathHD, Phasor Agents, and "agent optimizers fail to compound", which one of the three
    optimizers did.
- **Rates by sample.** The rate is lower than gap-11845c's 62% because these works were checked only on one or two
  evidence sentences each, not on full descriptions.
- **Classic works.** Classic works cited as grounding are listed in `evidence_candidates.classic_works`, for example
  Conant & Ashby 1970, Ashby 1956, Woolley et al. 2010, McClelland et al. 1995, Bower 1981 and Kahneman 2011. They are
  books and journal papers, not on arXiv, and were not read.
- **Left as is.** Venue and award claims the paper text cannot confirm (ICML Spotlight, WWW, FSE, CAIS Best Paper,
  IEEE 2024).

