# B3: Incremental file watchers for TUI dashboard

## Context

**Repo:** `/Users/will/dev/nunchi/roko/roko`
**Branch:** `demo-backend`
**Language:** Rust (workspace with ~29 crates)
**Key crate paths:**
- CLI + orchestrator: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/`
- Core types: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/`
- HTTP server: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/`
- Agent dispatch: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/`

**Key files:**
- Orchestrator (20K lines): `crates/roko-cli/src/orchestrate.rs`
- CLI entry: `crates/roko-cli/src/main.rs`
- Server routes: `crates/roko-serve/src/routes/mod.rs`
- Server state: `crates/roko-serve/src/state.rs`
- Server events: `crates/roko-serve/src/events.rs`
- Server WS: `crates/roko-serve/src/routes/ws.rs`

**Architecture:**
- `roko-serve` is an axum HTTP server on port 6677 with ~85 REST routes + WebSocket
- `AppState` uses `tokio::sync::RwLock` — all lock ops are `.read().await` / `.write().await` (NOT `.unwrap()`)
- Event bus: `state.event_bus.publish(event)` — always present, no Option wrapping
- The TUI gets data two ways: (1) StateHub push via `watch<DashboardSnapshot>` channel, (2) file polling via `DashboardData::tick()` reading `.roko/` files

### Pre-commit (MANDATORY)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## What this task does

`DashboardData::tick()` currently does full re-reads of JSONL files on every tick when the file modification timestamp changes. For large JSONL files (episodes, efficiency, signals), this is O(N) per tick. An incremental cursor (`JsonlCursor`) already exists at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/jsonl_cursor.rs` and is already used by `SignalCursor` and `EpisodeCursor`.

This task extends incremental tailing to the efficiency and c-factor JSONL files, and adds comprehensive edge-case tests.

Small JSON config files (`gate-thresholds.json`, `cascade-router.json`, `experiments.json`) keep stamp-based full reads — they are small enough that incremental parsing provides no benefit.

---

## Current state

**Already incremental (no changes needed):**
- `signals/engrams.jsonl` — uses `SignalCursor` (see `self.signal_cursor.tick()`)
- `episodes.jsonl` — uses `EpisodeCursor` (see `self.episode_cursor.tick()`)
- task outputs — uses `TaskOutputCursors`

**Still full-reading on stamp change (targets for this task):**
- `.roko/learn/efficiency.jsonl` — `read_efficiency_events_sync()` re-reads the entire file
- `.roko/learn/c-factor.jsonl` — `load_latest_jsonl_value()` re-reads the entire file

**Small config files (keep as-is):**
- `experiments.json` — single JSON object, ~1 KB
- `gate-thresholds.json` — single JSON object, ~1 KB
- `cascade-router.json` — single JSON object, ~2 KB

---

## Files to modify

1. `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/jsonl_cursor.rs` — add `IncrementalTailer<T>`
2. `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard.rs` — replace full-read paths with tailers

---

## Steps

### Step 1 — Read the existing JsonlCursor

Before writing anything, read:
```
/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/jsonl_cursor.rs
```

`JsonlCursor` handles seek-based incremental reads, truncation detection (by comparing the current file size against the stored offset), and partial-line buffering. `IncrementalTailer<T>` wraps it and adds deserialization plus item accumulation. Build on top of `JsonlCursor` — do not duplicate its logic.

Key method to understand: `JsonlCursor::read_new_lines()` returns `Vec<String>` of complete lines appended since the last call. `JsonlCursor::offset()` returns the current byte offset; if this decreases between ticks, the file was truncated or rotated.

### Step 2 — Add `IncrementalTailer<T>` to jsonl_cursor.rs

Append the following block after the existing `JsonlCursor` impl, before the `#[cfg(test)]` block:

```rust
/// Generic incremental tailer for typed JSONL files.
///
/// Wraps a [`JsonlCursor`] and deserializes each new line into `T`.
/// Accumulates all parsed items, so the consumer always has the full
/// history without re-reading from the start.
///
/// # Truncation and rotation
///
/// If the underlying file is truncated (offset goes backward) or rotated
/// (replaced with a new file at the same path), the accumulated item list
/// is cleared and rebuilt from the new content.
///
/// # Malformed lines
///
/// Lines that fail to deserialize are silently skipped. This handles the
/// case where a concurrent writer has produced a partial line.
///
/// # Examples
///
/// ```no_run
/// use roko_cli::tui::jsonl_cursor::IncrementalTailer;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Event { kind: String }
///
/// let mut tailer = IncrementalTailer::<Event>::new("/tmp/events.jsonl");
/// if tailer.tick().unwrap_or(false) {
///     println!("New events: {}", tailer.items().len());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct IncrementalTailer<T> {
    cursor: JsonlCursor,
    items: Vec<T>,
    changed: bool,
}

impl<T: serde::de::DeserializeOwned> IncrementalTailer<T> {
    /// Create a tailer for `path`. The file does not need to exist yet.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            cursor: JsonlCursor::new(path),
            items: Vec::new(),
            changed: false,
        }
    }

    /// Read any newly appended lines, deserialize, and accumulate.
    ///
    /// Returns `true` if at least one new item was parsed.
    /// Returns `false` if there were no new lines (or all new lines
    /// were malformed).
    ///
    /// On file truncation or rotation (detected by the offset going
    /// backward), accumulated items are cleared before re-reading.
    pub fn tick(&mut self) -> std::io::Result<bool> {
        let prev_offset = self.cursor.offset();
        let new_lines = self.cursor.read_new_lines()?;

        // Truncation / rotation: offset went backward or reset to zero
        // after being non-zero. Clear accumulated items so the consumer
        // sees only data from the current version of the file.
        let current_offset = self.cursor.offset();
        if current_offset < prev_offset || (prev_offset > 0 && current_offset == 0) {
            self.items.clear();
        }

        if new_lines.is_empty() {
            self.changed = false;
            return Ok(false);
        }

        let mut added = false;
        for line in &new_lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(item) = serde_json::from_str::<T>(trimmed) {
                self.items.push(item);
                added = true;
            }
            // Silently skip lines that fail to parse — they may be partial
            // writes from a concurrent appender.
        }

        self.changed = added;
        Ok(added)
    }

    /// All accumulated items since the file was first opened (or last reset
    /// due to truncation / rotation).
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// `true` if the last call to [`tick`][Self::tick] added new items.
    #[must_use]
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// The most recently accumulated item, if any.
    #[must_use]
    pub fn latest(&self) -> Option<&T> {
        self.items.last()
    }
}
```

### Step 3 — Add tests for IncrementalTailer

Inside the existing `#[cfg(test)] mod tests` block in `jsonl_cursor.rs`, add these tests:

```rust
    // -----------------------------------------------------------------------
    // IncrementalTailer tests
    // -----------------------------------------------------------------------

    #[test]
    fn tailer_accumulates_typed_items() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Entry { value: u32 }

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("data.jsonl");
        fs::write(&path, "{\"value\":1}\n{\"value\":2}\n").expect("seed");

        let mut tailer = super::IncrementalTailer::<Entry>::new(&path);

        // First tick: read 2 items.
        assert!(tailer.tick().expect("first tick"));
        assert_eq!(tailer.items().len(), 2);
        assert_eq!(tailer.items()[0].value, 1);
        assert_eq!(tailer.items()[1].value, 2);
        assert!(tailer.changed());

        // Second tick: no new data.
        assert!(!tailer.tick().expect("idle tick"));
        assert_eq!(tailer.items().len(), 2);
        assert!(!tailer.changed());

        // Third tick: append one more line.
        append(&path, "{\"value\":3}\n");
        assert!(tailer.tick().expect("third tick"));
        assert_eq!(tailer.items().len(), 3);
        assert_eq!(tailer.latest(), Some(&Entry { value: 3 }));
    }

    #[test]
    fn tailer_handles_truncation() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Row { n: u32 }

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("data.jsonl");
        fs::write(&path, "{\"n\":1}\n{\"n\":2}\n").expect("seed");

        let mut tailer = super::IncrementalTailer::<Row>::new(&path);
        tailer.tick().expect("initial tick");
        assert_eq!(tailer.items().len(), 2);

        // Truncate by overwriting with shorter content.
        fs::write(&path, "{\"n\":99}\n").expect("truncate");
        tailer.tick().expect("post-truncation tick");
        assert_eq!(tailer.items().len(), 1, "old items must be cleared after truncation");
        assert_eq!(tailer.items()[0].n, 99);
    }

    #[test]
    fn tailer_handles_empty_file() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Item { x: i32 }

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("empty.jsonl");
        fs::write(&path, "").expect("create empty");

        let mut tailer = super::IncrementalTailer::<Item>::new(&path);
        let changed = tailer.tick().expect("tick on empty file");
        assert!(!changed);
        assert!(tailer.items().is_empty());
    }

    #[test]
    fn tailer_skips_malformed_lines() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Good { v: u32 }

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("mixed.jsonl");
        // Mix valid and invalid lines.
        fs::write(&path, "{\"v\":1}\nNOT JSON\n{\"v\":2}\n").expect("seed");

        let mut tailer = super::IncrementalTailer::<Good>::new(&path);
        tailer.tick().expect("tick with malformed lines");
        assert_eq!(tailer.items().len(), 2, "malformed lines must be silently skipped");
        assert_eq!(tailer.items()[0].v, 1);
        assert_eq!(tailer.items()[1].v, 2);
    }

    #[test]
    fn tailer_handles_file_not_yet_existing() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct E { n: u32 }

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("will-appear-later.jsonl");
        // File does not exist yet — tick should succeed gracefully.
        let mut tailer = super::IncrementalTailer::<E>::new(&path);
        let _ = tailer.tick(); // may return Ok(false) or Err — must not panic

        // File appears.
        fs::write(&path, "{\"n\":1}\n").expect("create file");
        assert!(tailer.tick().expect("tick after file appears"));
        assert_eq!(tailer.items().len(), 1);
    }

    #[test]
    fn tailer_handles_blank_lines() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct V { n: u32 }

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("blanks.jsonl");
        // File has blank lines interspersed.
        fs::write(&path, "{\"n\":1}\n\n{\"n\":2}\n\n").expect("seed");

        let mut tailer = super::IncrementalTailer::<V>::new(&path);
        tailer.tick().expect("tick");
        assert_eq!(tailer.items().len(), 2, "blank lines must be silently skipped");
    }
```

### Step 4 — Replace efficiency full-reads in dashboard.rs

Open `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard.rs`.

**Add import** near the existing cursor imports (around line 38):
```rust
use super::jsonl_cursor::IncrementalTailer;
```

**Replace the `efficiency_stamp` field** in `DashboardData`.

Find:
```rust
    /// Last observed efficiency file metadata.
    efficiency_stamp: FileStamp,
```

Replace with:
```rust
    /// Incremental cursor over `.roko/learn/efficiency.jsonl`.
    efficiency_tailer: IncrementalTailer<AgentEfficiencyEvent>,
```

**Replace the `cfactor_stamp` field** in `DashboardData`.

Find:
```rust
    /// Last observed C-Factor file metadata.
    cfactor_stamp: FileStamp,
```

Replace with:
```rust
    /// Incremental cursor over `.roko/learn/c-factor.jsonl`.
    cfactor_tailer: IncrementalTailer<CFactor>,
```

**Update `DashboardData::load_best_effort()`**.

Find the initializer lines that set `efficiency_stamp:` and `cfactor_stamp:` and replace them with tailer constructors:
```rust
efficiency_tailer: IncrementalTailer::new(&efficiency_path),
cfactor_tailer:    IncrementalTailer::new(&cfactor_path),
```

**Update the `tick()` method** — replace the efficiency block.

Find (approximately lines 591–598):
```rust
        let stamp = file_stamp(&efficiency_path);
        if stamp != self.efficiency_stamp {
            self.efficiency_stamp = stamp;
            self.efficiency_events = read_efficiency_events_sync(&efficiency_path);
            self.efficiency = load_efficiency_summary(&efficiency_path);
            self.efficiency_trend = load_efficiency_trend(&efficiency_path);
            generation_changed = true;
        }
```

Replace with:
```rust
        if self.efficiency_tailer.tick().unwrap_or(false) {
            self.efficiency_events = self.efficiency_tailer.items().to_vec();
            self.efficiency = summarize_efficiency_events(&self.efficiency_events);
            self.efficiency_trend = efficiency_trend(&self.efficiency_events, 24);
            generation_changed = true;
        }
```

Find the cfactor block (approximately lines 632–637):
```rust
        let stamp = file_stamp(&cfactor_path);
        if stamp != self.cfactor_stamp {
            self.cfactor_stamp = stamp;
            self.cfactor = load_latest_jsonl_value::<CFactor>(&cfactor_path);
            self.cfactor_trend = load_cfactor_trend(&cfactor_path);
            generation_changed = true;
        }
```

Replace with:
```rust
        if self.cfactor_tailer.tick().unwrap_or(false) {
            self.cfactor = self.cfactor_tailer.latest().cloned();
            self.cfactor_trend = cfactor_trend(self.cfactor_tailer.items(), 24);
            generation_changed = true;
        }
```

### Step 5 — Add `summarize_efficiency_events` helper if needed

Check whether `load_efficiency_summary()` reads from disk. If it does, you need an in-memory variant. Search:
```bash
grep -n "fn load_efficiency_summary\|fn summarize_efficiency" \
  crates/roko-cli/src/tui/dashboard.rs
```

If `load_efficiency_summary` takes a file path, add a helper that works from a slice:
```rust
/// Derive an [`EfficiencySummary`] from an in-memory event slice.
///
/// Used by the incremental tailer path to avoid re-reading the file.
fn summarize_efficiency_events(events: &[AgentEfficiencyEvent]) -> EfficiencySummary {
    // Delegate to EfficiencySummary::from_events if that method exists,
    // or replicate the aggregation logic from load_efficiency_summary.
    EfficiencySummary::from_events(events)
}
```

If `EfficiencySummary::from_events` does not exist, check whether the aggregation is done inline in `load_efficiency_summary` and move it into this helper.

### Step 6 — Remove `DashboardDataStamps` fields

If a `DashboardDataStamps` struct tracks per-file stamps for fingerprinting, remove the `efficiency` and `cfactor` fields from it (and from its `fingerprint()` or `hash()` method) since those files are now tracked by tailers.

Search:
```bash
grep -n "efficiency\|cfactor" \
  crates/roko-cli/src/tui/dashboard.rs | grep -i "stamp\|finger"
```

Remove any entries found.

---

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Compile
cargo check -p roko-cli 2>&1 | head -30

# Run the jsonl_cursor tests (includes new tailer tests)
cargo test -p roko-cli -- jsonl_cursor --nocapture

# Run dashboard tests
cargo test -p roko-cli -- dashboard --nocapture 2>&1 | tail -30

# Clippy
cargo clippy -p roko-cli --no-deps -- -D warnings 2>&1 | head -20

# Format
cargo +nightly fmt --all -- --check
```

Expected: all tests pass. The tailer tests confirm incremental accumulation, truncation handling, empty-file handling, malformed-line skipping, and pre-existence handling. The dashboard still compiles and renders in the TUI.
