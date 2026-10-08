Status: reviewed · budget none · owner gap-8d2c79

> **Note, 2026-10-02.** The whitepaper was rewritten on 2026-10-02 with a new framing: a design paper about
> Roko's feedback loops, without status tags or the status appendix. The review below applies to the earlier
> text, which git history keeps; the rewritten text needs its own independent review. A same-day editing pass applied
> a fact-check of the rewrite against the code and brought each section within its word budget; that pass is an edit,
> not an independent review.

# Whitepaper review

- **Reviewer:** wk-wp-review, a Claude agent that drafted no section and had no drafting context (gap-8d2c79).
- **Read at:** `9947af4d2`, with the status matrix pinned at `a17d4dadd` (71 rows; `tools/status_matrix.py --check`
  passes).
- **Fixes:** `f173fa9e1` (section text), `97c0c22e1` (`references.bib`), `7df1354b5` (frozen S09 excerpt and §8's
  footnotes), `d8cd83b23` (this directory's `README.md`), `856e52cac` (trims after the sync) and `5c144601f` (§5.2's
  loop count, §0–§1 footnote rows), `fbfdedb5b` and `1852c5d5b` (the README, now publication-clean), `eee9c3dd5`
  (the matrix's next-item ids for EX8, SS3 and DM2) and `c2f4772e8` (§4 and §9 name the items filed since). `bb3a42650` added this file and moved §0–§10 to `Status: reviewed`; `45829814d`
  moved the appendix to `reviewed`, with the coordinator's approval, by changing only `[matrix] status` in
  `data/mechanisms.toml`.
- **Sync:** `87e0eca96` merged the coordinator branch at `7c556bc0a`. That brought in the §9 roadmap follow-up
  (gap-c19902), B5's frozen loop table and paperlint's update (gap-af0b57). `06ee34e6f` then merged `48d35a67f`, with
  the figures (gap-d1d92c), and `d64ba4434` merged `80fa2171b`. The review read that text too (findings 25, 26
  and 28).

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

| # | Where | Finding[^r-sources] | Disposition |
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
| 12 | §0–§10 | 7,250 words, 1.12× the 6,500 budget. The cuts remove repeated detail: §1.1 repeated §2's results, §3.2 repeated §5.2's table, §9.2 repeated §7, and §10 repeated §5.4; §5.2 had a commit-by-commit narrative; there were also §6's intro, one §4.11 sentence and one §8.3 clause. After the first sync the total was 7,153 | Fixed in `f173fa9e1` and `856e52cac`. With the figures merged, the total is 7,135 words, 1.10×, and every file is within 0.5–1.3× of its budget |
| 13 | §1.3 | Said "an operator stepped in 39 times". The rollup's 39 interventions include users; §7.3 says "people" | Fixed in `f173fa9e1` |
| 14 | §9.1 | Called "planning" WIRED@a17d4dadd while §4.1 tags authoring PARTIAL@a17d4dadd. The rows tagged WIRED@a17d4dadd are plan generation and validation (AU1, AU2) | Fixed in `f173fa9e1` |
| 15 | §10 | Wrote opusplan as plain text, where §6 marks it as code | Fixed in `f173fa9e1` |
| 16 | §3, §4, §9 | The sections cite merge commits (`1d923e377`, `33e107da1`); the matrix cites the commits they merged (`dc99a9e81`, and `763596768`, `189a14e65`, `ebf274ada`, `fb87e3738`) | Accepted: each merge contains its row's commit, so both references are right |
| 17 | §2 | At 1.18×, the most over budget | Accepted: every sentence carries a sourced result or its design response, and the paper as a whole is within 10% |
| 18 | §4.10, §4.12 | Said that no epic covers the review step. Matrix row SS6 names gap-25065c, which imports the research checklist and does not cover it. gap-0d64d5 (the approval hold with a per-task diff, under E6) was filed after the pin | Fixed in `c2f4772e8`: §4.10 and §4.12 name gap-0d64d5. The matrix keeps gap-25065c, because `tools/status_matrix.py` checks each item at the pinned commit, where gap-0d64d5 does not yet exist |
| 19 | paperlint | The number rule takes a footnote as a source only if it names a commit, a work item, a `[@key]` or a rollup or snapshot id. A frozen file cited with its sha256 does not count, so two footnotes also name gap-29a64e, the item that froze their files | Accepted; a possible refinement for the tool |
| 20 | paperlint | `--strict docs/whitepaper/*.md` includes this directory's `README.md`, which fails by design: it has no status header, and it lists the banned words and the bare tag vocabulary. The verify runs the directory form, which reads only section files | Accepted |
| 21 | Literature | Checked against the papers: kim2025towards (six benchmarks, +80.8% and −70.0%), rajput2026cheap (0.12–0.55, about 0.05, 46%, 3% against 32%), zhao2026specbench (30 tasks, 28 points per tenfold), bhola2026scrouting (266 tasks, a fifth of the cost, the ablation tie), wang2026compound, wang2026rethinking (Terminal-Bench 2.1), lin2026evopath, edwards2026askorassume (69.4%), narayan2025minions (97.9% at 5.7×), prasad2024adapt (up to 33%), brun2011proactive (5,355 merges, 16% textual, 1% build and 6% test) | Accepted: all match |
| 22 | Numbers | Checked against the frozen files and the code: B7's TL;DR and Corrections, CASE-001 and CASE-005 to CASE-007, the rollup's totals and day rows ($159.36, $33.56, $172.80), W12's F2, §9's tag counts (21, 23, 2, 7, 7, 10 and 1 of 71) and §9.4's line counts (64,066 of 1,052,880 at `1f4481133`) | Accepted: all match |
| 23 | Tags | Every tag in §0–§10 matches its matrix row or vision claim, and §4.12's step tags match the rows it lists | Accepted |
| 24 | Figure 3 | §9's link to the appendix resolves at this commit. The SVG path belongs to gap-d1d92c, whose verify checks it, and `paperlint --strict` checks the link again after the swap | Accepted; recheck after gap-d1d92c merges |
| 25 | §9.3, `evidence/` | Text from the first sync: §9.3's epic numbers match §4.12; its new sentence on the four rows with no item (EX8, IS6, SS3, DM2) matches the matrix's own warnings; B5's frozen file is named against the date convention, and the evidence README records that exception and hashes the file | Accepted: all consistent |
| 26 | §5.2, §9 | B5's frozen re-check at `98ee1418f` counts Runner-v2's 16 loops as two wired, five partial, seven orphaned, one broken and one built but unwired. §5.2 said only "most" and "several", and §9 gives no count | Fixed in `5c144601f`: §5.2 states the count and cites the frozen file; §9 agrees because it gives no count |
| 27 | README | The public README held the writers' working ledger: each section's claims with gitignored sources, the shared numbers N1–N13, `tmp/` paths, bare status tags, and banned words that flagged themselves. `paperlint --strict` found 78 problems | Fixed in `fbfdedb5b`: the ledger is now an untracked working file. The README keeps the title block, thesis, outline with budgets and reading order, and the conventions; paperlint still reads its budgets and banned words. With a status header on the README and on this file, `paperlint --strict docs/whitepaper/*.md` passes on all 14 files |
| 28 | §4.6, §9.2, §3.2 | After the pin, the coordinator branch merged fixes for the git guard and key-file access (bug-7de5df, bug-a66941, `0728a2817`) and attempt records (gap-528762). Their cargo checks are pending. The text describes these mechanisms as at `a17d4dadd`, as the whole paper does | Accepted: the paper is pinned at one commit. Rows IS4, IS5, LM1 and RC6 are for the next matrix refresh |

[^r-sources]: Each row names its own sources: the frozen files in `evidence/`, the matrix pinned at `a17d4dadd`, the
    work items and commits it cites, and for finding 21 the papers by their bibliography keys.

## For the author

- **The abstract** now gives the supervising sessions' estimated cost (finding 4). Drop the clause if the abstract
  should stay on Roko alone; §1.3 carries the same fact.
- **FrugalGPT and RouteLLM** are cited at TMLR 2024 and ICLR 2025, as the programme's verified bibliography records
  them. refcheck confirmed only their arXiv ids.
- **Whether this file goes into the PDF** is gap-8117a8's call.

## Reported, not changed here

- **Matrix rows SS6 and IS6** could not take their new items, gap-0d64d5 and gap-8f8544. `tools/status_matrix.py`
  checks items at the pinned commit, and both were filed after it. SS6 still names gap-25065c, an import task; IS6
  names none. EX8, SS3 and DM2 took theirs in `eee9c3dd5`. Both remaining rows can move at the next refresh.
- **Rows IS4, IS5, LM1 and RC6** may move at that refresh (finding 28).

Verdict: accept

## Re-read after the audit re-pin

- **Reader:** w3-pk95, a Claude agent that wrote none of the re-pinned text and had no drafting context (backlog task
  9504, gap-d4a1c1). Another agent wrote task 9503's sections.
- **Read at:** `79ba9911f`, where the whitepaper is as `ec12c7513` left it, with the matrix pinned at `a43288b5f` (71
  rows; `tools/status_matrix.py --check` passes, with the warnings of finding 39).
- **Scope:** every sentence of §0, §1, §3, §4, §5, §6, §8, §9 and the README that changed after the first read: the
  re-pins at `ed0c33bd5` (`12a8d7793`), at `41228d7b2` (`9a37ad4d4`) and at `a43288b5f` (`29996ca33`, `ec12c7513`),
  read in the current text; the 13 rows `9d059cc72` moved, with its claims and figures; and the two files `ac644b6d7`
  froze. §2 and §7 have not changed since the first read. §10 and §6.3's sentences on cited works changed through the
  prior-art pass (gap-65ed57) and the citation content audit (`34a924d2f`), not through a re-pin, and were not
  re-read.

### How the re-read was done

- **Tags:** each `TAG@a43288b5f` in the changed text against its row or claim in `data/mechanisms.toml`, and every
  row and claim tag against the audit's own matrix at `a43288b5f`, which the re-pin followed. All 71 rows and 10
  claims agree, and so do §9.1's counts.
- **Lowered rows:** the notes of EX4, IS5, QA7, RC1, RC3, RC4, RC5 and SS3 against the frozen run and its root causes.
- **Numbers:** each number in the changed text against the frozen run, its root causes, B7 and the field rollup;
  §4.5's plan counts recomputed at `a43288b5f` with the footnote's command and `tomllib`.
- **Code at the pin:** the retry budget (`TaskRetryBudgets`), plan discovery's skip of `plans/archive/` (`load_plans`)
  and the efficiency record's lack of a cost source (`AgentEfficiencyEvent`).
- **Items and commits:** every commit the changed text cites is in the pin's history, and every item it cites exists.
  Where the text names an item as the work that would change a tag, its tasks were read and its state checked at
  `79ba9911f`.
- **Frozen evidence:** `shasum -a 256 -c SHA256SUMS` passes, no absolute path remains, and each 2026-10-02 file
  differs from its gitignored source only by the rewrites its provenance row lists.
- **Lint:** `paperlint --strict --require-status reviewed docs/whitepaper/*.md` passes on all 14 files.

### Findings

| # | Where | Finding[^rr-sources] | Disposition |
|---|---|---|---|
| 29 | §0, §1.3, §4.12, §8.5 and §9.1; the README's thesis; claim V3's blocker, printed in the appendix | The run of 2026-10-02 is called Roko's first live run on cheap models ("a first", "the first", "Roko's first", "after the first live run"); the README calls it "the one live run on cheap models", and V3 says "One live run on real cheap models exists". The paper's own frozen evidence records earlier ones. B7's TL;DR, and the Findings row "Cheap models on mechanical tasks" that the footnotes of §0 and §1 cite: 142 attempts on gpt-oss-120b, 132 of them on demo, test and bench tasks and 10 on self-development, half of those 10 verified. The field rollup's "By run": plan runs on gpt-oss-120b on 09-18, 09-21 and 09-22. All of them predate the tier ladder (`a13873ad2` and `ce12e86d8`, 2026-09-30). What the run of 2026-10-02 adds is the ladder's first live use on cheap models, as the text before this re-pin put it ("no real run has used the cheap rungs yet"); gap-f30b8e, a fixture plan through the ladder on real cheap models, is still open | Must change before the tag: "the ladder's first live run on cheap models" (one word more; §0 has three to spare), no "the one" in the README, and in V3 "One live run of the ladder on real cheap models exists" |
| 30 | §0, §1.3, §4.12 and §9.1 | "Needed six operator interventions". The frozen run's TL;DR and its "Operator interventions" table count six, and say four count against unattended running: three rounds of moving the isolation files aside, the last with a ladder change, then a second ladder change. The fifth passed `USER` to the Claude CLI, a harness artefact that "does not count against roko"; the sixth was the designed merge between plans. The text charges all six to Roko | Must change, in the sentences of finding 29: four interventions, the other two named in the footnotes, which keeps §0 within budget |
| 31 | §3.1; row RC1's note, printed in the appendix | §3.1: "keeps one field per stream chunk, so GLM-4.7's answers arrived blank"; RC1: "because". The frozen evidence confirms the parser's behaviour but not that it made the answers blank. The run, "What broke" 2: the parser makes up a `stop` finish reason, "so the failure looks like an empty success". The root causes, TL;DR and §1: which payload was lost (a tool call, a length or content-filter finish, or reasoning with no answer) "stays open until someone captures the raw stream", and in the last case GLM gave no answer at all. The capture is task 1112 of gap-625195, still open | Must change, in both places: for example "keeps one field per stream chunk and makes up a stop reason, so GLM-4.7's failed turns arrived as blank successes" |
| 32 | §3.3, the `.roko/learn/gate-thresholds.json` row | The caveat "Sets retry budgets since `99adacd6d`" rests on QA7, which the re-pin lowered, and was not rewritten. At the pin `TaskRetryBudgets` raises the budget of every task the ladder routes to at least what climbing takes, five in the run and the top of the thresholds' default range (`default_max_retries`), so there the thresholds change nothing; §3.2 says the ladder overrode them | Should change: "Sets retry budgets since `99adacd6d`, but the ladder raises them (QA7)" |
| 33 | §4.5 | Recomputed at `a43288b5f`: 132 plans, 102 with `max_parallel = 1`, 25 of those with tasks at the same depth; 79 of the 102 and 24 of the 25 are under `plans/archive/`, which `load_plans` skips, as the footnote says. The body still reads "102 of the 132 tracked plans set it to 1, 25 of them with tasks that could run together": of the plans discovery finds, one, `plans/qa-workflow-validation/tasks.toml`, is held to 1 with tasks that could run together | Should change: the archive caveat in the body ("all but one of them archived") |
| 34 | §1.2, §3.2 | The README asks each mechanism that is not WIRED to name the item that would change its tag. §1.2 cites gap-625195 and gap-e00238 for adapters, failover, the circuit breaker, budgets and output screening, but budgets are in neither: they are gap-997366 (task 3102) and gap-f548c1 (task 2101). §3.2's paragraph names no item for RC3 (gap-e00238), RC5 (as above) or QA7 (none is filed; §5.2 points it at spec-6ac537). The Sensors row cites gap-f548c1 for "no decision log", but decision records are gap-cc5051, gap-f61823 and gap-2b5d37 | Should change |
| 35 | §4.7, §5.2 | "Since `4c0e5646e` every surface counts only verified passes" (§5.2: "neither does any surface"): `4c0e5646e` brought the dashboard and the TUI, and `601997dd1` the serve routes, the runtime adapter and the portal. Row QA2 cites both, and both precede the pin | Should change: cite both |
| 36 | §4.8 | "The budget gate history sets (`99adacd6d`) is PARTIAL@a43288b5f" reads as if "budget gate history" were one noun; the claim is right | Should change: "the retry budget that gate history sets" |
| 37 | §9.3 | "Four goals follow in order" lists E2 (spec-e9d7ec) under Truth and E13 (spec-f2463d) under Proof as work to come, but both closed before the pin, E2 in `a43288b5f` itself and E13 in `d7070e368`; §4.12 and §8.5 already treat E2 as done. Five of the six Golden-path epics had closed by the pin, which "most of them merged" understates. E4 (spec-b7303f) closed after the pin | Should change: mark the closed epics |
| 38 | §3.1; §9.1, V10's row | gap-198c9c, cited for pause (SS5) and for V10, closed in wave 1 (`f37936858`), after 9503 was written: its tasks 1207 and 1208 make pause hold. At the pin pause is as row SS5 says, so the text holds as a snapshot, as in finding 28 | For gap-08d9b2's final re-pin: move SS5 if the fix holds there, and cite what is still open |
| 39 | Matrix | `tools/status_matrix.py --check` warns that eight rows have a fix, wire or build verdict but no item: RC1, RC3, RC6, LM3, SS3, SS5, SS6 and DM2. The items the text cites for them (gap-625195, gap-e00238, gap-f548c1, gap-843aef, gap-ce1d11) were filed after the pin, where the tool looks; LM3 could name gap-644040, open at the pin. RC1, RC3 and RC5 take the verdict fix where the audit's matrix says keep, though `9d059cc72` says the verdicts follow the audit's; fix suits their live defects. QA7 (PARTIAL, keep) has no item | For the final re-pin |
| 40 | §0–§10 | 7,755 words, 1.19× the 6,500 decided in dec-2cd76a, against 7,135 (1.10×) at the first read; the re-pins added 620 words, 238 of them in `29996ca33`. §0, §3, §4 and §9 sit at 1.27–1.29× their budgets, 3 to 15 words under strict lint's limit, so the changes above must be close to word-neutral there. Finding 17 accepted §2's 1.18× because the whole was within 10%; it no longer is | For the author: trim before the tag, or accept 1.19× |
| 41 | Tags, numbers, figures | Every tag in the changed text matches its row or claim at the pin. QA7, RC3, RC4, RC5 and SS3 say what the live run showed, and so does RC1 but for finding 31; EX4 and IS5, lowered on the static re-check, rest on the code and the canaries, and EX4 says no live run has used it. The run's numbers in the text match the frozen files: about $1.2–1.4, ten tasks verified, seven on gpt-oss-120b, failover down to the cheap rung four times, a rate-limited call retried about 1 s later, three knowledge entries per prompt, retries raised to five. Figures 1 and 2 and their text versions carry the matrix's marks, and the re-pin changed only §6's tags | Accepted |
| 42 | `evidence/` | The two 2026-10-02 files match their sources but for the rewrites their rows list. The run's copy still names the audit's own README, a gitignored file, as its row records | Accepted |

[^rr-sources]: Each row names its sources: the live run, frozen as `evidence/2026-10-02-live-cheap-model-run.md`
    (sha256 `813172c96b88`), and its root causes, as `evidence/2026-10-02-live-defect-root-causes.md` (sha256
    `5df7d5221539`); B7, as `evidence/2026-09-29-b7-real-run-evidence.md` (sha256 `799b6a2b6184`); the field rollup,
    as `evidence/2026-09-29-field-rollup.md` (sha256 `7bade1532a6d`); the matrix pinned at `a43288b5f`; the commits
    and work items it cites; and for finding 40, the word counts of `paperlint --report` applied to the files at
    `f5a6ac64b` and at `79ba9911f`.

### Checks that read this file

The first read's verdict line, above, stands for the text read at `9947af4d2`. The checks of task 9504 and of the
whitepaper epic (spec-ce1484) grep this file for `^Verdict: accept`, which that line matches, so they cannot see the
verdict below.

Findings 29 to 31 must change before gap-8117a8 tags the whitepaper, and a reader should then check those sentences
again. Findings 32 to 37 can go in the same pass, 38 and 39 wait for gap-08d9b2's final re-pin, and 40 is the
author's call.

Verdict: revise
