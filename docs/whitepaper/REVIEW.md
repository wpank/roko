# Whitepaper review

- **Reviewer:** wk-wp-review, a Claude agent that drafted no section and had no drafting context (gap-8d2c79).
- **Read at:** `9947af4d2`, with the status matrix pinned at `a17d4dadd` (71 rows; `status_matrix.py --check`
  passes).
- **Fixes:** `f173fa9e1` (section text), `97c0c22e1` (`references.bib`), `7df1354b5` (frozen S09 excerpt and §8's
  footnotes), `d8cd83b23` (this directory's `README.md`), `856e52cac` (trims after the sync) and `5c144601f` (§5.2's
  loop count, §0–§1 footnote rows). `bb3a42650` added this file and moved §0–§10 to `Status: reviewed`; `45829814d`
  moved the appendix to `reviewed`, with the coordinator's approval, by changing only `[matrix] status` in
  `data/mechanisms.toml`.
- **Sync:** `87e0eca96` merged the coordinator branch at `7c556bc0a`. That brought in the §9 roadmap follow-up
  (gap-c19902), B5's frozen loop table and paperlint's update (gap-af0b57). `06ee34e6f` then merged `48d35a67f`, with
  the figures (gap-d1d92c). The review read that text too (findings 25 and 26).

## How the read was done

- **Scope:** §0–§10, the README, the appendix and `evidence/`, read as one document for an engineering lead deciding
  how to run agent work unattended.
- **Tags:** every `TAG@a17d4dadd` in §0–§10 checked against its row or vision claim in `data/mechanisms.toml`.
- **Numbers:** checked against the frozen files (`shasum -a 256 -c SHA256SUMS` passes) and the work items the
  footnotes cite. Recomputed where the text gives a method: the plan counts at `a17d4dadd` and the line counts at
  `1f4481133`.
- **Literature:** eleven results checked against the papers' abstracts or text (finding 21).
- **Code:** identifiers and paths through `paperlint --strict`. The code facts of §3 and §4 were checked by hand at
  HEAD: the 8,000-character PRD cut, the five-file limit, "the fewest cohesive tasks", the 11-node subgraph,
  `SkipFailed`, the `agent/*/<task>` branch lookup and the default `predict`.
- **Work items:** all 76 ids cited in §0–§10 exist. Their states were checked wherever the text depends on one.
- **Length:** `paperlint --report`.

## Findings

| # | Where | Finding | Disposition |
|---|---|---|---|
| 1 | §7.3 | Quoted "about 20 subagents" and "about 25 engine defects" from B7's TL;DR, which gives no source for either count; §1 had already dropped them | Fixed in `f173fa9e1`; the README's IN6 row in `d8cd83b23` |
| 2 | §0, §1, §4, §6, §9 | Footnotes pointed at §7 ("§7 cites the frozen copy") or at items, not at the frozen files | Fixed in `f173fa9e1`: each cites its `evidence/` file with the sha256 prefix |
| 3 | §1, §7.3 | Said gap-263de5 would replace the operator estimate. gap-263de5 is done: it built the harvester but committed no harvest, so the figures wait for gap-ccb87e | Fixed in `f173fa9e1` |
| 4 | §0 | Gave Roko's $174.87 "excluding the supervising sessions" without their size, so a reader of the abstract alone would overrate what Roko did unsupervised. §1 and §7 give the estimate | Fixed in `f173fa9e1`: one clause (16–20×) and the W12 footnote |
| 5 | §3.1, §4.6 | Said that provider CLIs get an allowlisted environment and no longer inherit Roko's keys. Matrix row IS5: gates start from an allowlist, but provider CLIs only have the key variables they recognise scrubbed. §9.2 had this right | Fixed in `f173fa9e1` |
| 6 | §4.9 | Said "every portal plan was green" while the product was unusable. CASE-006 concerns plans 05 to 08 | Fixed in `f173fa9e1` |
| 7 | §4, footnote on plans | The reproduction command listed 104 files, because two `_harness` scripts match. Recomputed with the pathspec `'plans/*tasks.toml'`: 132 plans, 102 with `max_parallel = 1`, 25 of those with tasks at the same depth. The text's figures hold | Fixed in `f173fa9e1` |
| 8 | §5.4 | Said affect and offline consolidation were "both decided by q-6b7cca", but q-6b7cca is an open question | Fixed in `f173fa9e1`: "both pending q-6b7cca" |
| 9 | §8 | The falsifier bar (0.90 and 0.30), the expected level (3, with Claude Code ahead at 5) and the 48-task probe come from S09 v1.1. S09 is gitignored and no tracked item carries these figures; the footnotes cited items that only refer to the bar. The text matches the spec | Fixed in `7df1354b5`: the excerpt is frozen with its hash and provenance, and the footnotes cite it |
| 10 | `references.bib` | The dedupe left comments with other entries' identifiers on yang2024sweagent, filieri2015software and paul2026context, plus a dangling comment from §1's FrugalGPT copy. FrugalGPT's journal field repeated the year, and one Devin page had two keys. The five kept entries the brief names match the verified bibliography, and no field holds an internal path | Fixed in `97c0c22e1`; §10 now cites the Devin page by §6's key (`f173fa9e1`) |
| 11 | README | N7, FE8 and the ROLLUP source row described the 11:14:31 rollup (3/41, 121 notes), and IN6 carried the unsourced counts. N12 was unresolved. N4 said "different window", where gap-cdd5f4 found the same data counted in another unit and denominator. GP14 had the tldr's plan counts. Six rows still showed keys in backticks that are now in the bib | Fixed in `d8cd83b23` |
| 12 | §0–§10 | 7,250 words, 1.12× the 6,500 budget. The cuts remove repeated detail: §1.1 repeated §2's results, §3.2 repeated §5.2's table, §9.2 repeated §7, and §10 repeated §5.4; §5.2 had a commit-by-commit narrative; there were also §6's intro, one §4.11 sentence and one §8.3 clause. After the first sync the total was 7,153 | Fixed in `f173fa9e1` and `856e52cac`. With the figures merged, the total is 7,123 words, 1.10×, and every file is within 0.5–1.3× of its budget |
| 13 | §1.3 | Said "an operator stepped in 39 times". The rollup's 39 interventions include users; §7.3 says "people" | Fixed in `f173fa9e1` |
| 14 | §9.1 | Called "planning" WIRED while §4.1 tags authoring PARTIAL. The WIRED rows are plan generation and validation (AU1, AU2) | Fixed in `f173fa9e1` |
| 15 | §10 | Wrote opusplan as plain text, where §6 marks it as code | Fixed in `f173fa9e1` |
| 16 | §3, §4, §9 | The sections cite merge commits (`1d923e377`, `33e107da1`); the matrix cites the commits they merged (`dc99a9e81`, and `763596768`, `189a14e65`, `ebf274ada`, `fb87e3738`) | Accepted: each merge contains its row's commit, so both references are right |
| 17 | §2 | At 1.18×, the most over budget | Accepted: every sentence carries a sourced result or its design response, and the paper as a whole is within 10% |
| 18 | §4.10, §4.12 | Say that no epic covers the review step. Matrix row SS6 names gap-25065c, which imports the research checklist and does not cover it | Accepted: the text is right. Reported to the matrix owner (row SS6) |
| 19 | paperlint | The number rule takes a footnote as a source only if it names a commit, a work item, a `[@key]` or a rollup or snapshot id. A frozen file cited with its sha256 does not count, so two footnotes also name gap-29a64e, the item that froze their files | Accepted; a possible refinement for the tool |
| 20 | paperlint | `--strict docs/whitepaper/*.md` includes this directory's `README.md`, which fails by design: it has no status header, and it lists the banned words and the bare tag vocabulary. The verify runs the directory form, which reads only section files | Accepted |
| 21 | Literature | Checked against the papers: kim2025towards (six benchmarks, +80.8% and −70.0%), rajput2026cheap (0.12–0.55, about 0.05, 46%, 3% against 32%), zhao2026specbench (30 tasks, 28 points per tenfold), bhola2026scrouting (266 tasks, a fifth of the cost, the ablation tie), wang2026compound, wang2026rethinking (Terminal-Bench 2.1), lin2026evopath, edwards2026askorassume (69.4%), narayan2025minions (97.9% at 5.7×), prasad2024adapt (up to 33%), brun2011proactive (5,355 merges, 16% textual, 1% build and 6% test) | Accepted: all match |
| 22 | Numbers | Checked against the frozen files and the code: B7's TL;DR and Corrections, CASE-001 and CASE-005 to CASE-007, the rollup's totals and day rows ($159.36, $33.56, $172.80), W12's F2, §9's tag counts (21, 23, 2, 7, 7, 10 and 1 of 71) and §9.4's line counts (64,066 of 1,052,880 at `1f4481133`) | Accepted: all match |
| 23 | Tags | Every tag in §0–§10 matches its matrix row or vision claim, and §4.12's step tags match the rows it lists | Accepted |
| 24 | Figure 3 | §9's link to the appendix resolves at this commit. The SVG path belongs to gap-d1d92c, whose verify checks it, and `paperlint --strict` checks the link again after the swap | Accepted; recheck after gap-d1d92c merges |
| 25 | §9.3, `evidence/` | Text from the first sync: §9.3's epic numbers match §4.12; its new sentence on the four rows with no item (EX8, IS6, SS3, DM2) matches the matrix's own warnings; B5's frozen file is named against the date convention, and the evidence README records that exception and hashes the file | Accepted: all consistent |
| 26 | §5.2, §9 | B5's frozen re-check at `98ee1418f` counts Runner-v2's 16 loops as two wired, five partial, seven orphaned, one broken and one built but unwired. §5.2 said only "most" and "several", and §9 gives no count | Fixed in `5c144601f`: §5.2 states the count and cites the frozen file; §9 agrees because it gives no count |

## For the author

- **The abstract** now gives the supervising sessions' estimated cost (finding 4). Drop the clause if the abstract
  should stay on Roko alone; §1.3 carries the same fact.
- **FrugalGPT and RouteLLM** are cited at TMLR 2024 and ICLR 2025, as the programme's verified bibliography records
  them. refcheck confirmed only their arXiv ids.
- **Whether this file goes into the PDF** is gap-8117a8's call.

## Reported, not changed here

- **Matrix row SS6** names gap-25065c, an import task, as the item that would change its tag; no filed item covers
  the approval hold or the per-task diff.
- **Matrix rows EX8, IS6, SS3 and DM2** have a verdict but no item. `status_matrix.py` already warns about them, and
  §9.3 now names them.

Verdict: accept
