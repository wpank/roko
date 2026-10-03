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
