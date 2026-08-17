# S-ci-2: Implement allowlist-check binary in roko-tooling

## Task
Add `crates/roko-tooling/src/bin/allowlist_check.rs` that compares fitness findings to the allowlist. Fails on new findings or expired entries.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-ci-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/27-ci-fitness-checks.md` § Phase 3.

## Exact changes

### 1. `crates/roko-tooling/src/bin/allowlist_check.rs`

```rust
use std::fs;
use std::path::PathBuf;
use clap::Parser;
use serde::Deserialize;

#[derive(Parser)]
struct Args {
    /// Kind: raw_provider_http | dangerous_perms | oversized_function
    #[arg(long)]
    kind: String,
    /// File with one finding per line (file:line:pattern)
    #[arg(long)]
    findings: PathBuf,
    /// Path to allowlist.toml
    #[arg(long)]
    allowlist: PathBuf,
}

#[derive(Deserialize, Default)]
struct Allowlist {
    #[serde(default)]
    raw_provider_http: Vec<Entry>,
    #[serde(default)]
    dangerous_perms: Vec<Entry>,
    #[serde(default)]
    oversized_function: Vec<OversizedEntry>,
}

#[derive(Deserialize)]
struct Entry {
    file: String,
    pattern: String,
    reason: String,
    owner: String,
    expires: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
struct OversizedEntry {
    file: String,
    function: String,
    lines: u32,
    max_lines: u32,
    reason: String,
    owner: String,
    expires: chrono::DateTime<chrono::Utc>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let raw = fs::read_to_string(&args.allowlist)?;
    let allowlist: Allowlist = toml::from_str(&raw)?;
    let findings: Vec<String> = fs::read_to_string(&args.findings)?
        .lines().map(String::from).filter(|l| !l.trim().is_empty()).collect();

    let now = chrono::Utc::now();
    let mut new_violations = Vec::new();
    let mut expired = Vec::new();

    let entries: Vec<&dyn EntryView> = match args.kind.as_str() {
        "raw_provider_http" => allowlist.raw_provider_http.iter().map(|e| e as &dyn EntryView).collect(),
        "dangerous_perms" => allowlist.dangerous_perms.iter().map(|e| e as &dyn EntryView).collect(),
        "oversized_function" => allowlist.oversized_function.iter().map(|e| e as &dyn EntryView).collect(),
        _ => anyhow::bail!("unknown kind: {}", args.kind),
    };

    for finding in &findings {
        let allowed = entries.iter().any(|e| e.matches(finding) && e.expires() > now);
        if !allowed {
            new_violations.push(finding.clone());
        }
    }
    for e in &entries {
        if e.expires() <= now {
            expired.push(format!("{} (owner: {}, expired {})", e.label(), e.owner(), e.expires()));
        }
    }

    if !new_violations.is_empty() {
        eprintln!("FAIL: new violations not in allowlist (kind={}):", args.kind);
        for v in &new_violations { eprintln!("  {v}"); }
        std::process::exit(1);
    }
    if !expired.is_empty() {
        eprintln!("FAIL: allowlist entries expired (kind={}):", args.kind);
        for e in &expired { eprintln!("  {e}"); }
        std::process::exit(1);
    }
    println!("OK: {} findings, all allowlisted (kind={})", findings.len(), args.kind);
    Ok(())
}

trait EntryView {
    fn matches(&self, finding: &str) -> bool;
    fn expires(&self) -> chrono::DateTime<chrono::Utc>;
    fn owner(&self) -> &str;
    fn label(&self) -> String;
}

impl EntryView for Entry {
    fn matches(&self, f: &str) -> bool { f.contains(&self.file) && f.contains(&self.pattern) }
    fn expires(&self) -> chrono::DateTime<chrono::Utc> { self.expires }
    fn owner(&self) -> &str { &self.owner }
    fn label(&self) -> String { format!("{}: {}", self.file, self.pattern) }
}

impl EntryView for OversizedEntry {
    fn matches(&self, f: &str) -> bool { f.contains(&self.file) && f.contains(&self.function) }
    fn expires(&self) -> chrono::DateTime<chrono::Utc> { self.expires }
    fn owner(&self) -> &str { &self.owner }
    fn label(&self) -> String { format!("{}::{}", self.file, self.function) }
}
```

### 2. Update `crates/roko-tooling/Cargo.toml`

```toml
[[bin]]
name = "allowlist-check"
path = "src/bin/allowlist_check.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
```

(Use whatever versions the workspace already standardizes on; check `Cargo.toml` workspace deps first.)

### 3. Update `scripts/roko-fitness-checks.sh` to use the checker in check mode

```bash
if [ "$MODE" = "check" ]; then
    cargo run --quiet -p roko-tooling --bin allowlist-check -- \
        --kind raw_provider_http --findings "$tmp/raw_http.txt" --allowlist "$ALLOWLIST" \
        || exit_code=1
    cargo run --quiet -p roko-tooling --bin allowlist-check -- \
        --kind dangerous_perms --findings "$tmp/dangerous.txt" --allowlist "$ALLOWLIST" \
        || exit_code=1
    cargo run --quiet -p roko-tooling --bin allowlist-check -- \
        --kind oversized_function --findings "$tmp/oversized.txt" --allowlist "$ALLOWLIST" \
        || exit_code=1
fi
```

## Write Scope
- `crates/roko-tooling/Cargo.toml`
- `crates/roko-tooling/src/bin/allowlist_check.rs` (new)
- `scripts/roko-fitness-checks.sh`

## Verify

```bash
cargo run -p roko-tooling --bin allowlist-check -- --kind raw_provider_http \
    --findings /dev/null --allowlist scripts/fitness/allowlist.toml
# Expect: "OK: 0 findings, all allowlisted"

# Try with a fake violation
echo 'crates/foo.rs:1:reqwest::Client::new' > /tmp/findings.txt
cargo run -p roko-tooling --bin allowlist-check -- --kind raw_provider_http \
    --findings /tmp/findings.txt --allowlist scripts/fitness/allowlist.toml
# Expect: exit 1, "FAIL: new violations"
```

## Do NOT

- Do NOT make `allowlist_check` a library — it's a binary.
- Do NOT bundle with S-ci-1/3/4.
- Do NOT silently pass when the allowlist is missing — error.
- Do NOT include findings older than the configured window. Findings are point-in-time; allowlist entries control the time dimension.
