# CTRL-03 independent review

## Assignment

- Role: independent reviewer
- Candidate: `ace630cebebc0b00aadcb60e8b5af3414ccadf88`
- Candidate parent/base: `e736f324bf6f1c6840d9d011c9189fe26e4cf052`
- Review branch: `review/CTRL-03-ace630cebebc`
- Authorized review write: this evidence file only

## Scope and method

I read the CTRL-03 requirement in the master checklist, the `depends_on_plan`
contract in the canonical task schema, every candidate manifest diff, and the
candidate implementation evidence. I independently parsed all 55 current backlog
manifests and the same 55 files from the candidate's parent with Python's standard
TOML parser. The review rebuilt the canonical ID set solely from each manifest's
`[meta].plan`; it did not reuse a mapping supplied by the implementer.

The candidate changes exactly six paths: five assigned manifests and its evidence
record. Manifest changes are limited to 17 `depends_on_plan` array lines (17 removed
and 17 added). No task ID, same-plan dependency, task status, file scope, context,
verification command, or other manifest field changed.

## Independent results

- Parsed backlog manifests: 55.
- Unique canonical `[meta].plan` IDs: 55.
- References absent from the canonical set before the candidate: 47.
- References absent from the canonical set after the candidate: 11.
- Changed tasks: 17 across exactly the five assigned manifests.
- Dependency values removed: 36.
- Dependency values added: 36.
- All 36 added values exactly match IDs in the independently rebuilt canonical set.
- External `P*` references before: 11.
- External `P*` references after: 11.
- The complete `(task_id, P-plan-id)` multiset is identical before and after.
- Every post-change unresolved reference is one of the 11 preserved external P-plan
  dependencies assigned to CTRL-04; there is no unresolved internal reference.

Changed-task distribution independently reproduced:

- `DOC-v1-cognition`: 7 tasks / 26 values.
- `E06-COMPOSE-UNIFY`: 3 tasks / 3 values.
- `E07-learning-knowledge`: 4 tasks / 4 values.
- `E10-FRONTEND-CONTRACT`: 2 tasks / 2 values.
- `E14-providers-tools`: 1 task / 1 value.

## Commands and gates

- Independent TOML census comparing candidate parent to candidate: exit 0; all
  assertions above passed, including `pre_unresolved=47`, `post_unresolved=11`,
  `refs_removed=36`, `refs_added=36`, and exact external multiset preservation.
- `/Users/will/dev/nunchi/roko/roko/target/debug/roko --version`: exit 0; sealed
  binary provenance `roko 0.1.0 ... git 1649c18b2`.
- `/Users/will/dev/nunchi/roko/roko/target/debug/roko plan validate --strict tmp/status-quo/self-heal/plans`:
  exit 0; `0 diagnostics in 6 plans`.
- The same binary validating all backlog plans: exit 1; the expected baseline
  `25 diagnostics in 55 plans`, unrelated to these dependency-string edits.
- The same binary validating each changed plan directory: DOC-v1, E07, E10, and E14
  exit 0 with zero diagnostics; E06 exits 1 with its one pre-existing missing future
  ADR diagnostic.
- `cargo fmt --all -- --check`: exit 0.
- `git diff --check e736f324bf6f1c6840d9d011c9189fe26e4cf052..ace630cebebc0b00aadcb60e8b5af3414ccadf88`:
  exit 0.
- Review worktree was clean before this evidence file was created.

## Verdict

`ACCEPTED`.

The candidate satisfies CTRL-03 exactly: all 36 noncanonical internal dependency
names become exact canonical plan IDs, all 11 external P-plan dependencies remain
byte-for-byte attributable to the same tasks, the diff is confined to its reserved
scope, and the evidence claims are reproducible. Integration and post-merge proof
remain coordinator responsibilities; this review does not mark CTRL-03 DONE.
