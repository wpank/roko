# CTRL-10 execution-evidence convention evidence

## Assignment

- Task: `CTRL-10`, establish canonical evidence and independent-review conventions.
- Base SHA: `128dc950c1659b49b85dc7d052ee0ea0dbc7bb12`.
- Branch/worktree: `agent/CTRL-10-evidence-conventions` at
  `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-10`.
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`.
- Reserved write scope: `tmp/status-quo/execution-evidence/README.md` and this
  evidence record only.
- Non-goals: no historical-evidence rewrite or rename; no master, manifest, shared
  index, production, test, lockfile, integration, remote, or external mutation.

## Requirement and method

The master defines evidence and independent review, but the evidence directory did
not have one canonical local convention for immutable candidate identity, review
cycles, retained rejections, command/artifact reporting, generated-index hygiene,
integration addenda, or bounded terminal claims.

I read the complete 1,164-line master, all 31 pre-CTRL-10 evidence records (3,550
lines), their complete Git history/review chains, and the current integration graph.
The review corpus includes simple acceptance, multiple rejection/correction cycles,
nonterminal evidence, ledger-only reconciliation, already-present behavior,
integration conflict reconstruction, and post-merge closure.

The new `README.md` is prospective and supplementary. It preserves every historical
record unchanged and derives the convention from the master's contract plus the
strongest existing examples.

## Mechanical pre-change inventory

The exact pre-change file set comprised 31 tracked regular Markdown files, zero
symlinks, and zero non-Markdown files. SHA-256 of the sorted output of
`sha256sum tmp/status-quo/execution-evidence/*.md` was:

```text
59541bb3e117f7d4d58be0448468e2578e015e5dfdfcbb6715f40b7ca7fc187f
```

Classification is based on file role, explicit verdict, content, and introduction
commit rather than filename alone:

| Record | Role/current disposition | Introduction commit |
|---|---|---|
| `CTRL-01-IGNORED-CANONICAL-IMPORT.md` | implementation; integrated DONE scope | `4887c2e6177c7a40749c185b01edbb0666882c81` |
| `CTRL-01-IGNORED-CANONICAL-IMPORT-REVIEW.md` | review; ACCEPTED | `c19bd30160443759f96d8fef6149cc9b146a5bde` |
| `CTRL-01.md` | implementation; integrated DONE scope | `e888da882db27d2dcd3fa03de968174cedb51ec4` |
| `CTRL-01-REVIEW.md` | review; corrected candidate ACCEPTED, prior rejection retained in content/history | `2671605cc0654794c5eff4ceebfa066df32fbe1b` |
| `CTRL-03.md` | implementation; integrated DONE scope | `ace630cebebc0b00aadcb60e8b5af3414ccadf88` |
| `CTRL-03-REVIEW.md` | review; ACCEPTED | `1c0fd5cc0dd1c9857c5734c589283cbaaff0d6ad` |
| `CTRL-04.md` | implementation/evidence-only resolution; integrated DONE scope | `c0be145d077b3989e6644bd9f0ca49823ce4da85` |
| `CTRL-04-REVIEW.md` | review; ACCEPTED | `b4661477763fbf1721bfb47ca1f6580a29ab6e63` |
| `CTRL-05.md` | implementation/reconciliation; integrated DONE scope | `ce223dcd34b864474bcdc610cd9e60829d09f614` |
| `CTRL-05-REVIEW.md` | review; ACCEPTED | `e33d36abdad962a9a68a2149465ee9bab6e76ecf` |
| `CTRL-06.md` | implementation plus two corrections; integrated DONE scope | `ea018feedcbccca3a3d922d293721134e6c7e829` |
| `CTRL-06-REVIEW-F1-REJECTED.md` | review; historical REJECTED candidate | `35fd8b912b2d6daab94e57fc9e71e36a8d960b91` |
| `CTRL-06-REVIEW.md` | review; second REJECTED candidate | `d5671d1e9994bb002563879e4f049004f470b31e` |
| `CTRL-06-REVIEW-FINAL.md` | review; corrected candidate ACCEPTED | `595eac759a2fea5b7dc22c4de182a94574971d6e` |
| `CTRL-07.md` | implementation/correction; integrated DONE scope | `18973e221a5ce6f8f72366ca2d8815db21f85b7c` |
| `CTRL-07-REVIEW.md` | review; REJECTED candidate | `18d16f2250bf1de3a09422c025019454b72511d6` |
| `CTRL-07-REVIEW-FINAL.md` | review; corrected candidate ACCEPTED | `81d1af92b142ce512964b078ccb5bc1a417b8e2d` |
| `CTRL-07-LEDGER-RECONCILIATION.md` | integration ledger reconciliation | `950fa8bc95a2b92f90dc970d6038547a28feb9e4` |
| `CTRL-07-LEDGER-RECONCILIATION-REVIEW.md` | review; ACCEPTED | `91da0fea5604e7639928824c3ab8ab07c21832af` |
| `CTRL-14.md` | nonterminal proof promoted by later terminal evidence; integrated DONE scope | `ed5ab0fed4a820b814d59b398c5c989b5003cfdf` |
| `CTRL-14-REVIEW-NOT-READY.md` | review; evidence accurate, terminal scope explicitly not accepted | `fb632f521d8e59dbb110ea45f947f11aa00210e1` |
| `CTRL-14-COVERAGE-CORRECTION.md` | correction implementation | `ad908c5af80b276cd7c66c1aaccaace452e2759a` |
| `CTRL-14-COVERAGE-CORRECTION-REVIEW.md` | review; REJECTED candidate | `0953c666e16076df100da1861e78da75c1fa484b` |
| `CTRL-14-COVERAGE-CORRECTION-REVIEW-FINAL.md` | review; corrected candidate ACCEPTED | `1be9dc64392b6556e2044f18eab922bf3f6f8eb3` |
| `CTRL-14-REVIEW-FINAL.md` | terminal review; ACCEPTED | `64273a9c43a167b012822df244192e370d4fd073` |
| `SH01-T06A-C1-C2-CORRECTION.md` | reconstructed implementation; integrated bounded DONE scope | `51bb0a0e5d0f20bf358198d02e06ecd5cb711f16` |
| `SH01-T06A-C1-C2-CORRECTION-REVIEW-RENEWED.md` | renewed review; ACCEPTED | `58ee07f2b97ec1d08893cf5a510fad965b763d7d` |
| `SH01-T06B1-B2B1-CORRECTION.md` | implementation; integrated bounded DONE scope | `fa828276a53597abc0fa82f249b7f14ac96f5a0d` |
| `SH01-T06B1-B2B1-CORRECTION-REVIEW.md` | review; ACCEPTED | `feb5753f15155ff64365e8e0116c1ef3d2fb318a` |
| `SH01-T06B2A.md` | implementation; integrated bounded DONE scope | `c71eb14f1aaa78a13375273a0981ccf166ead637` |
| `SH01-T06B2A-REVIEW.md` | review; ACCEPTED | `69a22c723548fad5318d2875690b5374e452146f` |

Totals:

```text
implementation/reconciliation records: 13
review records:                      18
  accepted candidate/accuracy reviews: 14
  rejected candidate reviews:            4
tracked Markdown records:             31
```

The 14 accepted/accuracy count includes the explicitly bounded
`CTRL-14-REVIEW-NOT-READY.md`; that file accepts evidence accuracy while clearly
refusing terminal task acceptance. It is therefore not counted as a terminal
candidate acceptance.

## Compatibility assessment

Every pre-convention record remains compatible as historical evidence:

- all 31 files are tracked and have a resolvable introduction commit;
- every review binds itself to a candidate directly or through an explicit
  correction chain;
- the four current-tree rejection records remain present and their later accepted
  reviews explicitly disposition the findings;
- the legacy CTRL-01 rejection predates separate current-tree rejection records;
  rejected candidate `e13ec0a86680028f9d333962eb5d81193b5c4772` remains in Git and its
  findings/disposition are named by current implementation/review evidence. Future
  rejection cycles require their own retained review file;
- accepted reviews distinguish candidate acceptance from integration and DONE;
- implementation records name integration/post-merge proof and bound their final
  scope;
- generated-index side effects, environmental failures, and cleanup are recorded
  rather than silently erased;
- the SH01 deadline chain correctly refuses to transfer an old acceptance across a
  semantic merge conflict and obtains renewed review on reconstructed bytes.

No historical file needs a compatibility edit. The README supplies prospective
navigation and conventions without changing prior claims or chronology.

## Implementation

`README.md` now defines:

- immutable candidate/base/component identity and renewed-review rules;
- worker/reviewer separation and the no-self-review boundary;
- exact `ACCEPTED`, `REJECTED`, and `BLOCKED` semantics;
- chronological, append-only rejection and correction retention;
- command, result, provenance, warning, failure, and artifact reporting;
- generated-index isolation and source-hash hygiene;
- integration, conflict, ancestry, post-merge, and status reconciliation fields;
- bounded DONE/SUPERSEDED scope and historical-record compatibility.

## Verification

Observed before commit:

- Local Markdown-link traversal found 12 links in the evidence directory; all 12
  resolve to existing repository paths.
- All 33 unique full commit SHAs cited by the two new records resolve as commits.
- The assigned base contains 31 evidence Markdown files, and the working tree still
  contains those exact 31 historical files. A base-blob versus working-file SHA-256
  comparison passed for every file.
- `git diff --check` passed.
- Status contained only the two new assigned Markdown files. No generated index,
  `.roko` state, lockfile, production path, master edit, or unrelated artifact was
  present.
- The pre-change inventory checksum remained
  `59541bb3e117f7d4d58be0448468e2578e015e5dfdfcbb6715f40b7ca7fc187f`;
  no historical record was rewritten to create this convention.

## Review readiness

- Candidate: the atomic commit containing the README and this evidence; its exact
  SHA is recorded at reviewer handoff because a commit cannot contain its own ID.
- Reviewer focus: independently recount and classify all 31 historical records,
  verify every introduction/review-chain commit, inspect every new normative rule
  against the master, run link/path and scope checks, and confirm no historical
  record was modified.
- Integration/post-merge status: pending independent review and integration-owner
  action.
