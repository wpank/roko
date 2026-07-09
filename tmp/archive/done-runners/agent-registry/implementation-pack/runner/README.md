# Agent Registry Runner

This runner executes the implementation-pack batches with Codex in a separate
git worktree.

## Defaults

- model: `gpt-5.4`
- Codex fast mode: prefers local `--profile fast`, otherwise falls back to
  `model_reasoning_effort=high`
- separate worktree under `.roko/worktrees/`
- commit after each successful batch
- retry failed batches (`--retries 2` means 1 initial attempt + 2 retries)
- remove ephemeral Rust build artifacts after each successful commit

## Usage

```bash
bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh
```

Common options:

```bash
bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh --list
bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh --only AR01,AR02
bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh --continue last
bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh --dry-run --only AR01
```

## Output

Logs are written under:

- `tmp/agent-registry/implementation-pack/runner/logs/<run-id>/`

The runner keeps:

- the worktree
- the branch
- the logs
- per-attempt prompt snapshots, last-message files, and logs under the run log dir

The runner removes after successful batches:

- per-batch `CARGO_TARGET_DIR` temp directories
- `target/`
- `.cargo-target/`

inside the worktree, if present.

The runner also fails early if required tools are missing for the selected
batches, if `git user.name` / `git user.email` are unset for commit-bearing
runs, or if `timeout`/`gtimeout` is unavailable for real Codex execution.
