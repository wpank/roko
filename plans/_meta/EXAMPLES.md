# Runnable plan examples

The example plans moved to `plans/demos/` on 2026-09-29. Their catalogue, run
commands, portal instructions and recorded costs are in
[`plans/demos/README.md`](../demos/README.md).

Three things from the earlier version of this page no longer hold:

- `roko plan run` does not read `--config`, so `--config plans/<demo>/roko.toml`
  had no effect. Set `ROKO_CONFIG` to run with a demo's config.
- `--rerun-existing` and `roko plan prune-attempts` are not part of the current
  CLI.
- `demo-release-readiness` had moved to `plans/archive/`; it is now
  `plans/demos/demo-release-readiness`.
