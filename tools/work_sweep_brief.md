# Work-item sweep brief

You are checking whether open work items in roko's work graph are still true at the current HEAD.
Repo: /Users/will/dev/nunchi/roko/roko (Rust workspace + apps/portal Next.js).
Item format and rules: work/README.md (read the "Item format" and "Status and closure rules" sections once).

## Hard rules (other sessions may be building in this same tree)

- READ-ONLY. Do not edit, create, move or delete any file except your own verdict file (path given in your task).
- Do NOT run `cargo`, `rustc`, `npm`, `pnpm`, `next`, or any `roko`/`target/*/roko` binary (they take the shared
  build lock or rewrite .roko state). Static checks only: `grep`/`rg`, `sed -n`, `cat`/`head`, `find`, `ls`,
  `wc`, `python3` for read-only parsing, and read-only git (`git log`, `git show`, `git diff`, `git blame`, `git grep`).
- No git write commands of any kind (no add/commit/stash/checkout/reset/restore/switch).
- Do not spawn sub-agents.
- Budget: about 4-6 tool calls per item, 90 tool calls total. Prefer one targeted `grep -n -A8`/`sed -n 'a,bp'`
  over reading whole files. Batch several greps into one Bash call where you can.

## What changed

Each batch row comes from `python3 tools/work.py drift --json`: `id`, `title`, `path` (the item file), `head` (the
commit to check against), `since` (the commit or date of the item's last check), and the drift signals:
`gone` (anchors whose file or symbol no longer exists), `touched` (commits since the last check that changed an
anchored file: sha, date, subject, path), `claimed` (commits whose message says they close the item),
`mentioned` (commits that mention it), `plan_passed` (plan tasks declaring `closes` that passed), `stale`.
See what changed with `git log --oneline <since>..HEAD` (or `--since=<date>` when `since` is a date) and
`git show <sha> -- <file>`. Uncommitted edits in the main checkout (`git status --short`) belong to other sessions:
a fix that exists only there is `in_progress`, not `done`.

## Procedure for each item

1. Read the item file fully (`path` in your batch row) and its drift signals. Run the static part of its
   [[verify]] command (everything before the first cargo/npm step) to see where it stands.
2. Check the claim against the code at HEAD. Look at the anchors, but remember a fix often lands in a
   different file (a caller, a new module), so grep for the relevant symbols/behaviour workspace-wide
   (`grep -rn ... crates/ apps/ --include=*.rs` etc., exclude target/ and node_modules/).
3. If it looks fixed, find the commit: `git log -S'<new code>' --oneline -- <file>` or `git log -L` or
   `git blame -L a,b <file>`. Record the short sha. If the fix is only in the working tree, say so.
4. Decide a verdict (below). Be conservative: `done` needs concrete evidence that the described problem is gone,
   i.e. the exact code path now behaves as the item's fix criteria require. If the item lists several defects and
   only some are fixed, that is `partial`. If you cannot tell without running tests, that is `unclear`
   (say which command would settle it).
5. Judge the item's [[verify]] command: is it sound (passes only when fixed, fails while the problem exists)?
   Some grep for the PROBLEM pattern and therefore "pass" while the bug is present - flag those.
   See work/README.md, "Verify commands".
   Suggest a sound replacement, static where possible (grep/sed for the fix's code, or a named cargo test that
   exists or that the fix must add). If the item has no [[verify]], suggest one when you can.
6. If anchors moved (line numbers shifted, symbol renamed, file moved/split), give the corrected anchor list
   (`path::symbol` preferred; `path:line` allowed).

## Verdicts

- `done`       the described problem no longer exists at HEAD (committed code). Needs `commit` + `evidence` with file:line.
- `in_progress` the fix exists only as uncommitted working-tree edits.
- `open`       still true as described (possibly with moved anchors).
- `partial`    some of it is fixed, some remains; `note` must say exactly what remains.
- `superseded` the premise no longer applies (code deleted/redesigned so the problem cannot occur, or the item is
               replaced by a newer item - give `replaced_by`). Not the same as done: nothing fixed it, it went away.
- `duplicate`  same problem as another item (open or closed) - give `duplicate_of` (the OLDER id wins; check
               `created`). Search: `grep -rln '<keyword>' work/items work/parked`.
- `unclear`    cannot be settled statically; say what would settle it.

## Output: one JSON object per line (JSONL), appended to your verdict file AS SOON AS each item is decided

Write with a heredoc append, e.g.
  cat >> VERDICT_FILE <<'JSON'
  {"id": "...", ...}
  JSON
Keep each object on ONE line. Fields:
  id (string), verdict (one of the above), confidence ("high"|"medium"|"low"),
  evidence (string: 1-3 sentences with file:line and what the code does now),
  commit (short sha or null), replaced_by (id or null), duplicate_of (id or null),
  note (string or null: text to append to the item body as a dated update - required for partial/open-with-changes;
        plain factual sentences, no markdown headers),
  anchors (list of strings or null: corrected anchors if they changed),
  verify_sound (true|false|null if none), verify_suggestion (string command or null),
  severity_suggestion (null or "p0".."p3" only if clearly miscalibrated, with reason in evidence),
  goal_suggestion (only when the item has NO goal: one of release|core|hermes|visibility|learning|tooling|features|later|park)

Goals (for goal_suggestion): the keys in work/goals.toml, plus later = keep open with no goal, park = not worth planning.

When all items are written, reply with ONLY a short summary: counts per verdict, and one line per `done`,
`superseded`, `duplicate` and `partial` item (id: why). No other prose.
