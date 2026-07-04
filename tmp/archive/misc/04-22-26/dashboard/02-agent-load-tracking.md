# Task 2: Track in-flight jobs per agent

## Objective

The matchmaking ranking formula needs to know how many jobs each agent currently has active
(`jobsInFlight`). Add a method to count in-flight jobs for a given agent by scanning the
persisted job files, and expose it on `AppState`.

## Design decision

There are two approaches:
- **A) In-memory counter on DiscoveredAgent** — fast but drifts if jobs are modified externally.
- **B) Scan `.roko/jobs/*.json` on demand** — always accurate, slightly slower.

**Use approach B** for the first pass. The matchmaking endpoint is a read-only query that
runs infrequently (once per quote request). Scanning a handful of JSON files is fine. If
performance becomes an issue later, add a cache with a short TTL using the existing
`AppState::cached_json` / `put_cached_json` mechanism.

## Files to modify

| File | What to change |
|---|---|
| `crates/roko-serve/src/routes/jobs.rs` | Extract `jobs_dir` and `load_all_jobs` as pub helpers, add `count_agent_inflight` function |

## Detailed changes

### 1. `crates/roko-serve/src/routes/jobs.rs` — add pub helper functions

The `jobs_dir` function (line 490) and `JobRecord` struct (line 66) are currently private.
Add a new public function near the bottom of the file (before the `non_empty_or_default`
helper at line 682):

```rust
/// Count jobs currently in-flight (assigned or in_progress) for a given agent.
///
/// Scans `.roko/jobs/*.json` on disk. Returns 0 if the jobs directory doesn't exist.
pub async fn count_agent_inflight_jobs(workdir: &Path, agent_id: &str) -> u32 {
    let dir = jobs_dir(workdir);
    if !dir.is_dir() {
        return 0;
    }
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(e) => e,
        Err(_) => return 0,
    };
    let mut count = 0u32;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let data = match tokio::fs::read_to_string(&path).await {
            Ok(d) => d,
            Err(_) => continue,
        };
        let job = match JobRecord::from_path(&path, &data) {
            Ok(j) => j,
            Err(_) => continue,
        };
        let status = normalise_status(&job.status);
        if (status == "assigned" || status == "in_progress") && job.assigned_to == agent_id {
            count += 1;
        }
    }
    count
}
```

Note: `JobRecord::from_path` is already `fn from_path(path: &Path, data: &str) -> Result<Self, ApiError>`
at line 100. It's private (`fn`, not `pub fn`), which is fine since the new function lives in
the same module.

### 2. Make the function accessible to the match endpoint

The matchmaking endpoint (Task 3) will live in the same `jobs.rs` file, so no visibility
change is needed. The function signature takes `workdir: &Path` so it can be called as:

```rust
let inflight = count_agent_inflight_jobs(&state.workdir, &agent.agent_id).await;
```

## Verification

### Compile check
```bash
cargo build -p roko-serve
```

### Existing tests must pass
```bash
cargo test -p roko-serve
```

### Unit test

Add a test to the bottom of `jobs.rs` (there are no existing test modules in this file,
so create one):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn count_inflight_empty_dir() {
        let dir = tempdir().expect("tempdir");
        let count = count_agent_inflight_jobs(dir.path(), "agent-1").await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn count_inflight_counts_assigned_and_in_progress() {
        let dir = tempdir().expect("tempdir");
        let jobs = dir.path().join(".roko").join("jobs");
        tokio::fs::create_dir_all(&jobs).await.unwrap();

        // assigned to our agent
        let j1 = JobRecord {
            id: "j1".into(),
            status: "assigned".into(),
            assigned_to: "agent-1".into(),
            ..Default::default()
        };
        tokio::fs::write(
            jobs.join("j1.json"),
            serde_json::to_string(&j1).unwrap(),
        ).await.unwrap();

        // in_progress for our agent
        let j2 = JobRecord {
            id: "j2".into(),
            status: "in_progress".into(),
            assigned_to: "agent-1".into(),
            ..Default::default()
        };
        tokio::fs::write(
            jobs.join("j2.json"),
            serde_json::to_string(&j2).unwrap(),
        ).await.unwrap();

        // completed — should NOT count
        let j3 = JobRecord {
            id: "j3".into(),
            status: "completed".into(),
            assigned_to: "agent-1".into(),
            ..Default::default()
        };
        tokio::fs::write(
            jobs.join("j3.json"),
            serde_json::to_string(&j3).unwrap(),
        ).await.unwrap();

        // in_progress but different agent — should NOT count
        let j4 = JobRecord {
            id: "j4".into(),
            status: "in_progress".into(),
            assigned_to: "agent-2".into(),
            ..Default::default()
        };
        tokio::fs::write(
            jobs.join("j4.json"),
            serde_json::to_string(&j4).unwrap(),
        ).await.unwrap();

        let count = count_agent_inflight_jobs(dir.path(), "agent-1").await;
        assert_eq!(count, 2);
    }
}
```

**Note:** `JobRecord` needs `Default` derive for the test. It already has `#[serde(default)]`
on all fields, so adding `#[derive(Default)]` to the struct (line 65) is safe. Check that it
doesn't already have it — if not, add it:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct JobRecord {
```

(It currently has `#[derive(Debug, Clone, Serialize, Deserialize)]` — add `Default`.)

### Run the new test
```bash
cargo test -p roko-serve count_inflight
```

## What NOT to do

- Do NOT add an in-memory counter that needs to be kept in sync with disk state.
- Do NOT make `JobRecord` public — it stays module-private.
- Do NOT add new routes for this — it's an internal helper consumed by the match endpoint.
