# S-ci-4: Promote docs-status-check to no-stale-claim gate

## Task
Add `scripts/fitness/docs-allowlist.toml` and update `docs-status-check.sh` to fail on docs that claim "[x] resolved" or similar without a matching commit reference.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-ci-2. Wave 3.

## Source plan
`tmp/subsystem-audits/implementation-plans/27-ci-fitness-checks.md` § Phase 6.

## Exact changes

### 1. `scripts/fitness/docs-allowlist.toml` (new)

```toml
# audit-2026-05-01 docs allowlist
#
# Each entry: a doc that claims a state but the verifier can't (or
# shouldn't) check it programmatically.

[[stale_claim_acceptable]]
file = "tmp/subsystem-audits/INDEX.md"
reason = "Top-level summary; status updates lag intentionally."
owner = "@will"
expires = "2027-01-01"

[[stale_claim_acceptable]]
file = "tmp/subsystem-audits/05-01/35-current-state-checklist.md"
reason = "Snapshot at audit time; sister doc 41 (consolidated backlog) is the live tracker."
owner = "@will"
expires = "2026-12-01"
```

### 2. Update `scripts/docs-status-check.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
ALLOWLIST="scripts/fitness/docs-allowlist.toml"
MODE="${1:-check}"

# Inventory: for each doc that contains "[x] resolved" / similar,
# emit "filename:line: claim". Filter against allowlist.

inventory_stale_claims() {
    rg -n '^- \[x\] ~~T[0-9]+-[0-9]+~~|^.*\[x\].*resolved\b' tmp/subsystem-audits/ \
      | head -200
}

if [ "$MODE" = "inventory" ]; then
    inventory_stale_claims
    exit 0
fi

# Check mode: hand findings to allowlist-check.
findings=$(mktemp)
inventory_stale_claims > "$findings"

cargo run --quiet -p roko-tooling --bin allowlist-check -- \
    --kind stale_claim_acceptable --findings "$findings" --allowlist "$ALLOWLIST" || {
    rm -f "$findings"
    exit 1
}
rm -f "$findings"
```

The allowlist-check binary needs to know about `stale_claim_acceptable`. Extend the `Allowlist` struct + match arm in `crates/roko-tooling/src/bin/allowlist_check.rs`:

```rust
#[derive(Deserialize)]
struct StaleClaimEntry {
    file: String,
    reason: String,
    owner: String,
    expires: chrono::DateTime<chrono::Utc>,
}

// In Allowlist:
#[serde(default)]
stale_claim_acceptable: Vec<StaleClaimEntry>,

// In match:
"stale_claim_acceptable" => allowlist.stale_claim_acceptable.iter().map(|e| e as &dyn EntryView).collect(),
```

`StaleClaimEntry::matches` checks `finding.contains(&self.file)`.

### 3. Update fitness GitHub Actions to drop `continue-on-error`

In `.github/workflows/fitness.yml` (added in S-ci-3):

```yaml
- name: Run docs-status-check
  run: bash scripts/docs-status-check.sh check
  # No continue-on-error; this is now blocking.
```

## Write Scope
- `scripts/fitness/docs-allowlist.toml` (new)
- `scripts/docs-status-check.sh`
- `crates/roko-tooling/src/bin/allowlist_check.rs`
- `.github/workflows/fitness.yml`

## Verify

```bash
ls scripts/fitness/docs-allowlist.toml

bash scripts/docs-status-check.sh inventory | head -10
bash scripts/docs-status-check.sh check
# Either passes (allowlist covers all current claims) or fails with specific docs
```

## Do NOT

- Do NOT add a generic "all stale claims OK" entry. Each file is explicit.
- Do NOT bundle with S-ci-1/2/3.
- Do NOT relax the regex to "find more than just `[x] resolved`" — start narrow; expand later.
