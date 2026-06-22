# S-learn-C: Episode JSONL schema_version field + reader migration

## Task
Add `schema_version: u32` to the `Episode` record. Add a reader that handles both current and previous schema versions (forward-compatible).

## Runner Context
Runner audit-2026-05-01, group S. Depends on T4-33. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/25-learning-feedback-completion.md` § Phase C.

## Why
T4-33 rotates the JSONL. To safely consume rotated archives across schema changes, each line carries a version marker.

## Exact changes

### 1. Add field

```rust
// crates/roko-cli/src/runtime_feedback/episodes.rs (or wherever Episode is defined)

const EPISODE_SCHEMA_VERSION: u32 = 1;   // bump when changing

#[derive(Serialize, Deserialize)]
pub struct Episode {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub episode_id: String,
    // ... existing fields
}

fn default_schema_version() -> u32 { 1 }
```

When writing, set `schema_version: EPISODE_SCHEMA_VERSION`.

### 2. Reader

```rust
pub fn read_episodes(path: &Path) -> impl Iterator<Item = Result<Episode, ReadError>> + '_ {
    BufReader::new(File::open(path).unwrap_or_else(|_| {
        // empty reader if file missing
        File::open("/dev/null").unwrap()
    }))
    .lines()
    .filter_map(|line| {
        let l = match line {
            Ok(l) => l,
            Err(e) => return Some(Err(ReadError::Io(e))),
        };
        if l.trim().is_empty() { return None; }
        // Try current version
        match serde_json::from_str::<Episode>(&l) {
            Ok(ep) => Some(Ok(ep)),
            Err(_) => {
                // Try previous version (V0): no schema_version field
                match serde_json::from_str::<EpisodeV0>(&l) {
                    Ok(legacy) => Some(Ok(legacy.into())),
                    Err(e) => Some(Err(ReadError::ParseError(e, l))),
                }
            }
        }
    })
}
```

Define `EpisodeV0` (without `schema_version`) and `From<EpisodeV0> for Episode`.

### 3. Tests

```rust
#[test]
fn read_handles_current_and_v0() {
    let v0_line = serde_json::to_string(&EpisodeV0 { episode_id: "x".into(), ... }).unwrap();
    let v1_line = serde_json::to_string(&Episode { schema_version: 1, episode_id: "y".into(), ... }).unwrap();
    let dir = tempdir().unwrap();
    let path = dir.path().join("ep.jsonl");
    std::fs::write(&path, format!("{v0_line}\n{v1_line}\n")).unwrap();
    let eps: Vec<_> = read_episodes(&path).collect::<Result<_, _>>().unwrap();
    assert_eq!(eps.len(), 2);
    assert_eq!(eps[0].schema_version, 1);  // upgraded from V0
    assert_eq!(eps[1].schema_version, 1);
}
```

## Write Scope
- `crates/roko-cli/src/runtime_feedback/episodes.rs`
- `crates/roko-learn/src/episode_logger.rs` (only if it owns Episode definition)

## Verify

```bash
rg 'schema_version|EPISODE_SCHEMA_VERSION' crates/roko-cli/src/runtime_feedback/episodes.rs crates/roko-learn/src/episode_logger.rs
# Expect: at least 4 hits
```

## Do NOT

- Do NOT add `schema_version` to other JSONL types in this batch (knowledge candidates etc.). One type per batch.
- Do NOT bump `EPISODE_SCHEMA_VERSION` past 1 unless you also change the schema.
- Do NOT bundle with other S-learn batches.
