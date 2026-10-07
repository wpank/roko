+++
id = "gap-963433"
kind = "gap"
title = "Companion audit E4 canary: the live planted-marker run (task 9518 option a) is still open"
status = "open"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["companion-audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK96 gap-a6dab7)"
discovered_from = "gap-a6dab7 (backlog task 9518, option a)"
anchors = ["work/done/gap-a6dab7-pk96-papers-figure-and-table-scripts-for.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^## Notes/,$p' work/items/gap-963433-*.md work/done/gap-963433-*.md 2>/dev/null | grep -qw 'E4-live-canary-verified'"
+++

## Problem

The companion audit's E4 knowledge-injection canary (backlog task 9518, delivered under `work/done/gap-a6dab7-*`)
has two options: (a) a live planted-marker run requiring a `cargo build` of roko at the audit tag, and (b) a static
trace. Only (b) was implemented (commit `d0c46a686`, result "injected"); gap-a6dab7's own Progress notes say
verbatim: "option (a), the planted-marker run, needs a cargo build at the tag and is still open." Because
gap-a6dab7 is now closed (`status = "done"`, its own `[[verify]]` only required option (b)), this remaining
sub-task has no open tracker entry and will not resurface via `NOW.md` or `tools/work.py next`.

## Why it matters

Option (b) is a static trace through the code/config at the audit tag; option (a) is the stronger, empirical
confirmation (an actual planted marker surviving into a real run's output under a real build). Until it runs, the
companion audit's E4 finding rests on inference rather than a reproduced observation, which matters for anyone
re-auditing the companion report's claims (`tmp/cybernetic-harness/companion-audit/`, epic spec-6afe5c).

## Where

The real deliverable (`e4_opportunities.py`, `research/E4-CANARY.md`) lives entirely under
`tmp/cybernetic-harness/companion-audit/`, which is gitignored in this repo (`git ls-files` returns nothing there;
`.gitignore:69 /tmp/*` has no carve-out for `tmp/cybernetic-harness/`) — there is no tracked source file specific
to this task to anchor on. "A cargo build at the audit tag" means building the `roko` binary (e.g. `cargo build -p
roko-cli` checked out at the audit tag, such as `a43288b5f` used elsewhere in this wave) and running a real plan
through it with a planted knowledge marker, then checking the marker's fate in the real output — distinct from the
Rust `roko-gate::audit` module itself (S05 task 4), which does not exist yet (`crates/roko-gate/src/audit/` is not
present at HEAD) and is not required for this canary.

## Current state

Option (b) (static trace) passed its verify as part of gap-a6dab7. Option (a) (live run) has not been attempted;
no blocker is recorded beyond needing the build step.

## Plan

1. Check out (or build from) the audit tag, `cargo build -p roko-cli` (or whatever binary the companion audit's
   harness invokes) in an isolated worktree.
2. Run the planted-marker scenario `research/E4-CANARY.md` describes against that real build.
3. Record the live result in `research/E4-CANARY.md` (or a dated sibling) alongside the existing static-trace
   result, noting any divergence between the two.
4. Since that evidence file is gitignored, also add one tracked line to this item's own `## Notes`, containing the
   word `E4-live-canary-verified` followed by the outcome and the commit/tag built — no other tracked artifact
   exists for this task, and the `[[verify]]` command below looks for exactly that word after the `## Notes`
   heading (it deliberately does not scan the sections above, so this Plan can name the word without the check
   passing vacuously).

## Done when

- A live, cargo-built planted-marker run has produced a recorded result (not just the static trace).
- A `## Notes` line contains `E4-live-canary-verified` plus the outcome (see Plan step 4).
- The `[[verify]]` command passes.

## Notes

- This item's anchor is the already-closed work item that first surfaced the gap
  (`work/done/gap-a6dab7-pk96-papers-figure-and-table-scripts-for.md`), since no tracked source file is specific to
  this task. If a future task adds a tracked script or fixture for the live canary, re-anchor this item there.
- Do not touch `docs/whitepaper/*` or `tmp/cybernetic-harness/paper/*` while doing this — out of scope, owned by
  another session. `tmp/cybernetic-harness/companion-audit/**` itself is in scope (a different, allowed directory).
- 2026-10-02 (filer-grpD): filed from backlog wave reports (PK96 gap-a6dab7, backlog task 9518 option (a)); the
  "Figure 2 subsystem x level heatmap (Appendix A)" half of the same report line was investigated separately and
  not filed here — see the filer's final report (it turned out to name whitepaper content, not a companion-audit
  deliverable, and docs/whitepaper/tmp/cybernetic-harness/paper are excluded from this filing pass).
