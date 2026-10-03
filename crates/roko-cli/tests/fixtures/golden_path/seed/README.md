# Golden-path seed

The seed repository of roko's golden-path acceptance test
(`crates/roko-cli/tests/golden_path_acceptance.rs`). The plan in `../plan/`
extends it; every check of that plan fails here until its work is done.

- `crates/alpha`, `crates/beta`: a two-crate Cargo workspace (`beta` uses
  `alpha`). `cargo test --workspace`
- `py/`: a Python module. `python3 -m unittest discover -s py`
- `ts/`: a TypeScript module, run by Node 22.6 or later with type stripping.
  `node --experimental-strip-types --test ts/*.test.ts`
