# M001 — Baseline verification snapshot

## Objective
Record the current workspace state before migration begins: test count, pass rate,
clippy warnings, and binary size. This creates a reference point for detecting regressions.

## Scope
- Crates: entire workspace
- Files: create `tmp/unified-migration-runner/baseline.json`
- Phase ref: 01-PHASE-0-PREP.md §0.4

## Steps
1. Run `cargo test --workspace -- --list 2>/dev/null | grep -c ':: test'` to count tests.
2. Run `cargo test --workspace 2>&1` and capture pass/fail/ignore counts.
3. Run `cargo clippy --workspace --no-deps -- -D warnings 2>&1 | tail -5` for lint status.
4. Run `cargo build -p roko-cli --release 2>/dev/null && ls -la target/release/roko` for binary size.
5. Write results to `tmp/unified-migration-runner/baseline.json`:
   ```json
   {
     "timestamp": "<ISO-8601>",
     "git_sha": "<HEAD short sha>",
     "test_count": <N>,
     "tests_passed": <N>,
     "tests_failed": <N>,
     "tests_ignored": <N>,
     "clippy_clean": true|false,
     "binary_size_bytes": <N>
   }
   ```
6. Commit the baseline file.

## Verification
```bash
test -f tmp/unified-migration-runner/baseline.json
python3 -c "import json; json.load(open('tmp/unified-migration-runner/baseline.json'))"
cargo check --workspace
```

## What NOT to do
- Do NOT fix any existing failures — just record them
- Do NOT modify any source code — this is measurement only
