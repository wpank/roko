# Self-heal verify commands misuse Cargo filters

- Severity: high
- Area: task authoring / validation

Several self-heal tasks use commands such as:

```sh
cargo test -p roko-cli runner::state runner::projection
```

Cargo accepts only one positional `TESTNAME` and exits with `unexpected argument 'runner::projection'`. Strict plan validation checks TOML structure but not executable command syntax, so these plans validate and dry-run successfully while containing guaranteed gate failures.

Split each filter into separate `cargo test` commands or use one shared substring. Add plan validation that parses common Cargo command shapes or executes declared verification in a safe `--no-run`/lint mode before starting a self-host run.

