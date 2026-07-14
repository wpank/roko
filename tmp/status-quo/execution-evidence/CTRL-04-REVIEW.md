# CTRL-04 independent review

Verdict: **ACCEPTED**

Reviewed candidate: `c0be145d077b3989e6644bd9f0ca49823ce4da85`

Candidate base: `0e6c4cd81938df9cc0f18638242402cb4e53dfab`

Current integration compatibility target: `fb5a47f6bf024a30bce4c6b345896e390b5684b8`

Review branch: `review/CTRL-04-c0be145d077b`

## Scope and primary evidence

I read the complete canonical master, the CTRL-01 canonical import evidence and
independent review, the CTRL-03 implementation evidence and independent review, all
55 backlog manifests, all 32 retained-root manifests, the candidate evidence, and
the complete candidate diff. I did not use the candidate worker's temporary files or
generated results.

The candidate is an evidence-only commit. Its parent-to-candidate diff adds exactly
`tmp/status-quo/execution-evidence/CTRL-04.md`; `git diff --check` passes. It changes
no master, manifest, index, production source, test, or status file.

The claimed prerequisite history is present and integrated before the candidate
base. CTRL-03 implementation/review/merge commits
`ace630cebebc0b00aadcb60e8b5af3414ccadf88`,
`1c0fd5cc0dd1c9857c5734c589283cbaaff0d6ad`, and
`4ae834b797fac4bf3be61714418388b2012e4206` are ancestors of the base. The CTRL-01
import/review/merge commits `699df4e0ea34bddabc4516695d28d1bf41328774`,
`c19bd30160443759f96d8fef6149cc9b146a5bde`, and
`01c00546bc57a485ff53553d0fe53006afa8ed42` are also ancestors.

## Independent manifest census

Python's standard `tomllib` independently parsed every task manifest and asserted
unique plan IDs, `meta.total == len([[task]])`, unique task IDs, and resolution of
every same-plan `depends_on` value:

```text
backlog manifests:              55
retained-root manifests:        32
union plan IDs:                  87 unique
all cross-plan references:      264
unresolved in the 87-plan union: 0
same-plan unresolved references: 0
all repository TOMLs parsed:    193, errors 0
```

All 264 cross-plan references occur in the backlog root. Exactly 11 target retained
plans, and the complete reference multiset is:

| Consumer task | Exact retained plan ID |
|---|---|
| `E04-security-perimeter/E04-T06` | `P16-safety-contracts` |
| `E04-security-perimeter/E04-T14` | `P22-acp-tool-permission` |
| `E07-learning-knowledge/E07-T09` | `P19-cascade-router-acp` |
| `E16-prd-self-hosting-gaps/E16-T1` | `P08-search-command-fix` |
| `E16-prd-self-hosting-gaps/E16-T2` | `P23-prd-pipeline-fix` |
| `E16-prd-self-hosting-gaps/E16-T2` | `P09-tool-alias-fix` |
| `E17-acp-completion/E17-T01` | `P22-acp-tool-permission` |
| `E17-acp-completion/E17-T02` | `P19-cascade-router-acp` |
| `E17-acp-completion/E17-T03` | `P25-mcp-acp-passthrough` |
| `E17-acp-completion/E17-T04` | `P22-acp-tool-permission` |
| `E17-acp-completion/E17-T05` | `P28-image-support` |

I rebuilt this multiset independently from immutable Git trees at the CTRL-03 merge
`4ae834b797fa`, candidate base `0e6c4cd81938`, candidate `c0be145d077b`, and current
integration `fb5a47f6bf02`. Each tree has the same exact 11 rows. No alias or dropped
dependency is concealed by the later import.

## Target identity and unfinished state

The eight target files match the sealed root, their named recovery-archive members,
and the hashes independently recorded by the CTRL-01 reviewer:

| Plan | Tasks/state | SHA-256 |
|---|---:|---|
| `P08-search-command-fix` | 4 ready | `e2406f0dbbf1ecc436d7c2de32faabdc0419a0e326b32d3a7efaf6ead2689991` |
| `P09-tool-alias-fix` | 3 ready | `d3ded0c373224b458920122e924463557e7b0c4f795593808d43b3559fb489a7` |
| `P16-safety-contracts` | 5 ready | `fc63e6addac2631d909d5f7c371b1f614c93c79f61e12bc48a5390b1974c7ce2` |
| `P19-cascade-router-acp` | 6 ready | `29f202968a6566fcf55824d5ed7aaf275ac71a18043f01aa8c5183d165fa8892` |
| `P22-acp-tool-permission` | 5 ready | `9ea7c6b3a1cc77b094ad2e1ca05ae1c18b488619920aa14a49ed4035ebaaea1a` |
| `P23-prd-pipeline-fix` | 6 ready | `5b465603f8115b1b10a7a28508ae7a227147558b735162cf26dedff16b9886cc` |
| `P25-mcp-acp-passthrough` | 4 ready | `821a0a3dc72405aef894e1c12617a885aea8a3e7d73dced58b4b1af0d946f0a5` |
| `P28-image-support` | 5 ready | `15baf1d1a198cabe00d681ddef950523d5ff718377cc2d0e21bda57fc25b8dc6` |

The recovery archive itself has SHA-256
`01c10b4565c1a897c92ced109c7f351fcb35513816860d094efa446da62c34e0`.
All eight plan metas are `ready`; their aggregate is 38 tasks, zero done, 38 ready.
Thus the candidate correctly distinguishes dependency-ID resolution from completion
or supersession of the target tasks.

## Independent strict validation

I exported the immutable candidate with `git archive` to a fresh disposable root and
used the integration-built validator with reported provenance `git d4749f9c7`. This
kept the validator's generated-index side effect away from the review worktree.

```text
plan validate --strict tmp/status-quo/backlog/plans: 0 diagnostics in 55 plans, exit 0
P08-search-command-fix:                         0 diagnostics in 1 plan, exit 0
P09-tool-alias-fix:                             0 diagnostics in 1 plan, exit 0
P16-safety-contracts:                           0 diagnostics in 1 plan, exit 0
P19-cascade-router-acp:                         0 diagnostics in 1 plan, exit 0
P22-acp-tool-permission:                        0 diagnostics in 1 plan, exit 0
P23-prd-pipeline-fix:                           0 diagnostics in 1 plan, exit 0
P25-mcp-acp-passthrough:                        0 diagnostics in 1 plan, exit 0
P28-image-support:                              0 diagnostics in 1 plan, exit 0
plan validate --strict plans:                  94 diagnostics in 32 plans, exit 1
```

The retained-root failure is transparently unrelated to CTRL-04 ID resolution. All
94 diagnostics are `PLAN_031`: 93 missing historical file prerequisites belong to
`architecture-core-queue`, and one missing file prerequisite belongs to superseded
`self-dev-ux`. None belongs to a CTRL-04 target, and no other diagnostic class is
present. This reproduces the candidate's classification without treating the whole
retained root as green.

The validator changed only the disposable export's generated index. The source
`plans/INDEX.md` remained unchanged and has sealed SHA-256
`7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.
Each target ID occurs once in that index with its correct ready count and once in the
primary implementation-order queue.

## Compatibility with current integration

The candidate and current integration share exact base
`0e6c4cd81938df9cc0f18638242402cb4e53dfab`. Their changed-path sets do not overlap,
and `git merge-tree --write-tree fb5a47f6b c0be145d0` exits zero without conflicts.
Directly parsing current integration `fb5a47f6b` reproduces 87 unique union IDs, the
same 11 retained-plan references, zero unresolved union IDs, the same eight target
hashes and 38-ready state, and the sealed index hash. The candidate therefore remains
factually accurate and cleanly integrable at the assigned current integration head.

## Decision

**ACCEPTED.** CTRL-04's 11 exact external plan references resolve against real,
byte-preserved retained manifests in the chosen 87-plan union. The evidence is
complete and accurately scopes the remaining 38 ready tasks, unrelated retained-root
file diagnostics, historical index, and later CTRL-15/CTRL-16 responsibilities. No
candidate correction is required; integration and post-merge status reconciliation
remain coordinator actions.
