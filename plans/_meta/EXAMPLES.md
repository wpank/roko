# Runnable plan examples

Build Roko once:

```sh
cargo build -p roko-cli --bin roko
```

Then run an example with the approval TUI:

```sh
./target/debug/roko plan run plans/demo-hello --config plans/demo-hello/roko.toml --approval --fresh
./target/debug/roko plan run plans/demo-multistage --config plans/demo-multistage/roko.toml --model glm-5-1 --approval --fresh
./target/debug/roko plan run plans/demo-incident-tabletop --config plans/demo-incident-tabletop/roko.toml --model glm-5-1 --approval --fresh
./target/debug/roko plan run plans/demo-parallel-integration --config plans/demo-parallel-integration/roko.toml --model glm-5-1 --approval --fresh
./target/debug/roko plan run plans/demo-release-readiness --config plans/demo-release-readiness/roko.toml --model glm-5-1 --approval --fresh
```

`demo-parallel-integration` runs two disjoint producer tasks concurrently, then
joins their accepted sibling outputs in a dependent aggregation task. Its
`max_parallel = 2` setting makes parallel activity visible in the TUI.

After an example has completed once, Roko normally verifies and reuses its
existing outputs. To deliberately dispatch the agents again for a TUI demo,
add `--rerun-existing` alongside `--fresh`:

```sh
./target/debug/roko plan run plans/demo-parallel-integration --config plans/demo-parallel-integration/roko.toml --model glm-5-1 --approval --fresh --rerun-existing
```

The existing files may produce no new Git diff when they already match the
contract, but both producer agents still dispatch concurrently.

Exercise failure and cross-process resume recovery with two commands:

```sh
./target/debug/roko plan run plans/demo-resume-recovery --config plans/demo-resume-recovery/roko.toml --model glm-5-1 --fresh --max-retries 0
ROKO_RESUME_DEMO_READY=1 ./target/debug/roko plan run plans/demo-resume-recovery --config plans/demo-resume-recovery/roko.toml --model glm-5-1
```

The first command is expected to stop at 1/2. The second must resume at stage
two and merge both `demo/resume-recovery/stage-one.txt` and `stage-two.txt`.

Use `--dry-run` before `--approval` to inspect task order without executing.
The larger demos use a lightweight base gate because their meaningful checks
validate CSV, JSON, Markdown, shell behavior, and cross-artifact traceability.

After a successful run, `roko plan prune-attempts` previews retained attempt
worktrees and branches without deleting them. Review that report before deciding
whether to run the separate explicit `--apply` form.
