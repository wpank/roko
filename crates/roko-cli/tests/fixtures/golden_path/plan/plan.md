# Plan: golden-path

The golden path's acceptance fixture (gap-f30b8e; backlog 3115). Nine tasks
extend the seed in `../seed/`: one Python, one TypeScript, two Rust tasks that
each touch one crate, a cross-crate pair (alpha gains `initials`, then beta's
`badge` uses it) and three mechanical tasks.

Written by hand from this spec, not by the frontier planner, and frozen in
backlog task 3115 before any solution or replay patch existed. Every task
carries a planner-written `[task.accept]` test, and every one of them fails on
the seed (`golden_path_fixture_is_red_on_the_seed`).

## The work

- T1, Python: `textutil.slugify(text)` turns text into a URL slug.
- T2, TypeScript: `camelCase(text)` joins words in camel case.
- T3, Rust (alpha): `alpha::word_count(text)` counts whitespace-separated words.
- T4, Rust (beta): `beta::farewell(name)` says goodbye, shouted.
- T5, Rust (alpha): `alpha::initials(name)` takes the first letter of each word.
- T6, Rust (beta, after T5): `beta::badge(name)` leads a name with its initials.
- T7, mechanical (alpha): `alpha::VERSION` is the crate's version.
- T8, mechanical (Python): `textutil.word_list(text)` splits text into words.
- T9, mechanical (TypeScript): `shout(text)` upper-cases text.

## Order

Tasks that write the same file run one after the other: T1 then T8 (Python),
T2 then T9 (TypeScript), T3 then T5 then T7 (alpha), and T4 then T6 (beta);
T6 also waits for T5, whose `initials` it uses. T1, T2, T3 and T4 can run
side by side.

## The whole-plan check

Once every task passed, the integrated result must pass `cargo fmt --check`,
clippy with `-D warnings`, `cargo test --workspace` (the acceptance tests
included), and the Python and TypeScript suites (`[meta] verify`).

## Changed after the freeze (2026-10-18)

The plan was frozen in backlog 3115. On 2026-10-18 four of its acceptance
tests gained a case each, and their counts in `tasks.toml` grew with them. No
task's description, files, order, tier or other checks changed.

- T5 (`accept/alpha_initials.rs`, count 2 to 3) and T6 (`accept/beta_badge.rs`,
  count 2 to 3) take a name whose first letter is not ASCII:
  `initials("émile zola")` is "ÉZ", and `badge("émile zola")` is
  "[ÉZ] émile zola".
- T1 (`accept/test_accept_slugify.py`, count 3 to 4) and T2
  (`accept/camel_case.test.ts`, count 3 to 4) check that `title_words` and
  `reverse`, which these tasks keep as they are, are still the seed's text,
  docstring and doc comment included. Both tests stay in the tree, so the
  whole-plan check holds T8 and T9, which edit the same files, to them too.

Why: in each of the three live runs of 2026-10-16 (gap-c013c5), the
independent review found an escape that no check saw (find-4003b9). T5
upper-cased initials ASCII-only, so `initials("émile zola")` was "éZ" though
the spec says "upper-cased", and T1 rewrote `title_words`' docstring though
the task says to keep the function as it is. Will chose to tighten the
fixture and rerun the three live runs (find-4003b9, option 2). The third
escape, a doc example switched off to get `cargo test` green, needed no new
test: the pre-verify screen rejects it now (gap-f9fa2c).

## Changed after the freeze (2026-10-19)

On 2026-10-19 T5's and T6's acceptance tests gained one more case each, and
their counts in `tasks.toml` grew with them (3 to 4). No task's description,
files, order, tier or other checks changed.

- `initials("ßeta omega")` is "SSO" (`accept/alpha_initials.rs`), and
  `badge("ßeta")` is "[SS] ßeta" (`accept/beta_badge.rs`). The upper case of
  "ß" is two letters, "SS", and T5's spec upper-cases each word's first
  character, so both letters stay.

Why: after the 2026-10-18 tightening, two of the three independent reviews of
the rerun still found a T5 escape that no check saw (find-8b6828). `initials`
kept only the first character of an upper case (`to_uppercase().next()`), so
`initials("ßeta")` was "S". The fixture's replay solution upper-cases in full
(`flat_map(char::to_uppercase)`). The fixture was tightened once more, as on
2026-10-18 (find-8b6828, option 2), before the next three live runs
(gap-f30b8e).
