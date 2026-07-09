# 27 — CI Fitness Checks

`scripts/roko-fitness-checks.sh` and `scripts/docs-status-check.sh` exist
as **inventory** scripts. They list current findings; they don't fail CI
on new findings. To prevent regression, they must become **no-new-violations**
gates with reviewed allowlists.

Source: doc 35 § Enforcement and CI; doc 41 priority 18.

---

## Today's State (verified 2026-05-01)

- `scripts/roko-fitness-checks.sh` lists raw provider HTTP sites,
  dangerous permission defaults, oversized functions, etc.
- `scripts/docs-status-check.sh` lists stale "resolved/completed" claims
  in audit docs.
- Neither is run in CI. Neither has an allowlist with owner / reason /
  expiry.

---

## Anti-Patterns

1. **No "fix the broken check by adding to allowlist" without review.**
   Allowlist entries require owner + reason + expiry.
2. **No "broaden the regex to silence findings."** Findings are real;
   either fix or allowlist with rationale.
3. **No CI gate without a way to override** for emergency hotfixes.
   Provide an `EMERGENCY_BYPASS=1` env that requires a paper trail.
4. **No relying on fitness alone.** Fitness catches what the regex catches.
   Real review still required.

---

## Plan

### Phase 1: Define the allowlist format

**File**: `scripts/fitness/allowlist.toml` (new)

```toml
# Each entry: a finding the script would otherwise flag, marked as
# acknowledged with a reason and expiry.
#
# Required fields per entry: file, pattern, reason, owner, expires.
# Optional: linked_issue, migration_plan.
#
# An entry is valid only if `expires` is in the future.

[[raw_provider_http]]
file = "crates/roko-cli/src/dispatch_direct.rs"
pattern = "reqwest::Client"
reason = "Legacy direct dispatch path; behind feature gate `legacy-direct-dispatch`. See plan 22 for removal."
owner = "@will"
expires = "2026-08-01"
linked_issue = "T5-37"
migration_plan = "tmp/subsystem-audits/implementation-plans/22-dispatch-streaming-completion.md"

[[raw_provider_http]]
file = "crates/roko-serve/src/routes/connectors.rs"
pattern = "reqwest::Client"
reason = "Non-LLM external API calls (GitHub, Linear). Not subject to ModelCallService."
owner = "@will"
expires = "2027-01-01"

[[oversized_function]]
file = "crates/roko-cli/src/orchestrate.rs"
function = "dispatch_agent_with"
lines = 2059
max_lines = 200
reason = "Tracked in T5-35; extraction in progress. See plan 20."
owner = "@will"
expires = "2026-09-01"
linked_issue = "T5-35"
```

### Phase 2: Update `roko-fitness-checks.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

ALLOWLIST="scripts/fitness/allowlist.toml"
MODE="${1:-check}"   # check | inventory | update

inventory_raw_provider_http() {
    rg -n 'reqwest::Client::(new|builder)' crates/ -g '*.rs' \
        | rg -v '^crates/roko-cli/src/dispatch_direct\.rs:' \
        || true
}

inventory_dangerous_perms() {
    rg -n 'dangerously_skip_permissions\s*=\s*true' \
        --type rust --type toml \
        crates/ roko.toml \
        | rg -v '^crates/roko-core/src/config/validation\.rs:' \
        || true
}

inventory_oversized_functions() {
    # Use a Rust helper or awk: list functions > N lines
    cargo run --quiet -p roko-tooling --bin function-sizes 2>/dev/null \
        | awk '$2 > 200 { print }'
}

check_against_allowlist() {
    local kind="$1"
    local findings_file="$2"
    if [ ! -s "$findings_file" ]; then
        echo "[fitness] $kind: 0 findings"
        return 0
    fi
    # Parse allowlist; for each finding, check if it's allowlisted with non-expired entry.
    cargo run --quiet -p roko-tooling --bin allowlist-check -- \
        --kind "$kind" --findings "$findings_file" --allowlist "$ALLOWLIST"
    return $?
}

main() {
    local exit_code=0
    local tmp=$(mktemp -d)

    inventory_raw_provider_http > "$tmp/raw_http.txt"
    inventory_dangerous_perms > "$tmp/dangerous.txt"
    inventory_oversized_functions > "$tmp/oversized.txt"

    if [ "$MODE" = "inventory" ]; then
        cat "$tmp"/*.txt
        return 0
    fi

    check_against_allowlist "raw_provider_http" "$tmp/raw_http.txt" || exit_code=1
    check_against_allowlist "dangerous_perms" "$tmp/dangerous.txt" || exit_code=1
    check_against_allowlist "oversized_function" "$tmp/oversized.txt" || exit_code=1

    rm -rf "$tmp"
    exit $exit_code
}

main "$@"
```

### Phase 3: Implement the allowlist checker

**Crate**: `crates/roko-tooling`

```rust
// src/bin/allowlist_check.rs

#[derive(Debug, Deserialize)]
struct Allowlist {
    raw_provider_http: Vec<AllowlistEntry>,
    dangerous_perms: Vec<AllowlistEntry>,
    oversized_function: Vec<OversizedFunctionEntry>,
}

#[derive(Debug, Deserialize)]
struct AllowlistEntry {
    file: String,
    pattern: String,
    reason: String,
    owner: String,
    expires: chrono::DateTime<chrono::Utc>,
    linked_issue: Option<String>,
    migration_plan: Option<String>,
}

fn main() -> Result<()> {
    let args = parse_args();
    let allowlist: Allowlist = toml::from_str(&fs::read_to_string(&args.allowlist)?)?;
    let findings: Vec<String> = fs::read_to_string(&args.findings)?.lines().map(String::from).collect();

    let entries = match args.kind.as_str() {
        "raw_provider_http" => &allowlist.raw_provider_http,
        "dangerous_perms" => &allowlist.dangerous_perms,
        // ...
    };

    let now = chrono::Utc::now();
    let mut new_violations = Vec::new();
    for finding in &findings {
        let allowed = entries.iter().any(|e| {
            finding.contains(&e.file) && finding.contains(&e.pattern) && e.expires > now
        });
        if !allowed {
            new_violations.push(finding.clone());
        }
    }

    let mut expired = Vec::new();
    for entry in entries {
        if entry.expires <= now {
            expired.push(format!("{} (owner: {}, expired {})", entry.file, entry.owner, entry.expires));
        }
    }

    if !new_violations.is_empty() {
        eprintln!("FAIL: new violations not in allowlist:");
        for v in &new_violations { eprintln!("  {}", v); }
        std::process::exit(1);
    }
    if !expired.is_empty() {
        eprintln!("FAIL: allowlist entries expired:");
        for e in &expired { eprintln!("  {}", e); }
        std::process::exit(1);
    }
    Ok(())
}
```

### Phase 4: Add to GitHub Actions / CI

**File**: `.github/workflows/fitness.yml` (new)

```yaml
name: Fitness Checks

on:
  pull_request:
  push:
    branches: [main]

jobs:
  fitness:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: bash scripts/roko-fitness-checks.sh check
      - run: bash scripts/docs-status-check.sh check
```

### Phase 5: Initialize the allowlist with current findings

Run `bash scripts/roko-fitness-checks.sh inventory` and add each
finding to `scripts/fitness/allowlist.toml` with:

- A real owner (@will or someone else committing to fix it).
- A real reason (link to a plan in this folder).
- A reasonable expiry (3-6 months out, aligned with the migration plan).

Then commit. The CI check should pass on the current main.

### Phase 6: Promote `docs-status-check.sh`

Same pattern as fitness: inventory mode, check mode, allowlist, owner,
expiry. The check fails if a doc claims `[x] resolved` but no commit
references the linked task ID.

---

## Anti-Patterns Specific To CI Promotion

1. **Don't allowlist a category by regex.** Each entry must be a
   specific file/pattern pair.
2. **Don't extend `expires` without re-reviewing.** Extending an expired
   entry requires the owner to confirm the migration plan is still
   accurate.
3. **Don't add the allowlist to `.gitignore`.** It's source-controlled.
4. **Don't run `EMERGENCY_BYPASS=1` without a paper trail.** Include an
   issue link in the commit message.

---

## Combined Verification

```bash
# Inventory mode (current findings)
bash scripts/roko-fitness-checks.sh inventory

# Check mode (must pass on clean main)
bash scripts/roko-fitness-checks.sh check

# Add a fake violation; check fails
echo 'let _ = reqwest::Client::new();' >> crates/roko-runtime/src/lib.rs
bash scripts/roko-fitness-checks.sh check
# Should exit 1 with the new violation listed

git checkout crates/roko-runtime/src/lib.rs

# Allowlist roundtrip
cargo run -p roko-tooling --bin allowlist-check -- \
    --kind raw_provider_http \
    --findings /tmp/findings.txt \
    --allowlist scripts/fitness/allowlist.toml
```

---

## Status

- [ ] Phase 1 — Allowlist format + initial entries
- [ ] Phase 2 — Update fitness script for check mode
- [ ] Phase 3 — Implement allowlist checker in roko-tooling
- [ ] Phase 4 — Add to CI workflow
- [ ] Phase 5 — Initialize allowlist with current findings
- [ ] Phase 6 — Promote docs-status-check similarly

**Estimated total effort**: 8-12 hours.
