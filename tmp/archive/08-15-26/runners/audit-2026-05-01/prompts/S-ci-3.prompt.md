# S-ci-3: Fitness GitHub Actions workflow

## Task
Add `.github/workflows/fitness.yml` that runs `roko-fitness-checks.sh check` and `docs-status-check.sh check` on PR + push to main.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-ci-1, S-ci-2. Wave 3.

## Source plan
`tmp/subsystem-audits/implementation-plans/27-ci-fitness-checks.md` § Phase 4.

## Exact changes

### `.github/workflows/fitness.yml` (new)

```yaml
name: Fitness Checks

on:
  pull_request:
  push:
    branches: [main, wp-arch2]

jobs:
  fitness:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: Build allowlist-check
        run: cargo build --quiet -p roko-tooling --bin allowlist-check
      - name: Run roko-fitness-checks
        run: bash scripts/roko-fitness-checks.sh check
      - name: Run docs-status-check
        run: bash scripts/docs-status-check.sh check
        # Failure here is informational until S-ci-4 lands; allow to continue
        continue-on-error: true
```

(After S-ci-4 lands, drop `continue-on-error` from `docs-status-check`.)

## Write Scope
- `.github/workflows/fitness.yml` (new)

## Verify

```bash
ls .github/workflows/fitness.yml

# Lint the workflow if actionlint is available locally
actionlint .github/workflows/fitness.yml 2>&1 || echo "actionlint not installed; skip"
```

## Do NOT

- Do NOT bundle with S-ci-1/2/4.
- Do NOT add `EMERGENCY_BYPASS=1` env handling unless explicitly requested. The default is "fail = block."
- Do NOT skip the `setup-rust-toolchain` action; the binary needs `cargo` to build.
- Do NOT enable `continue-on-error` on the main fitness check.
