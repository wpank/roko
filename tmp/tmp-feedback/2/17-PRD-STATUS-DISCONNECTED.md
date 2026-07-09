# /prd-status Shows Disconnected Data

## Problem

`/prd-status` shows:
```
═══ PRD Coverage Report ═══

PRD                                 Status       Plans  Tasks  Done
───                                 ──────       ─────  ─────  ────
dry-run-flag                        published    —      —      —
costs                               draft        —      —      —
cursor-composer-backend             draft        —      —      —
self-developing-workflow            unknown      —      —      —
test-quick                          draft        —      —      —

Plans: 11  Tasks: 136  Done: 19  Coverage: 14%
```

Issues:
1. **Per-PRD plans/tasks/done are always `—`** — no plan linkage logic exists
2. **Global totals are disconnected** — shows Plans: 11, Tasks: 136 at the bottom but
   none are attributed to any PRD
3. **`self-developing-workflow` shows "unknown" status** — missing YAML frontmatter
4. **14% coverage is misleading** — it's global done/total across all plans, not PRD coverage
5. **No slugs shown** — user can't act on the data (same issue as #14, but not yet fixed here)

## Root Cause

`prd.rs:795-801` — the per-PRD row is hardcoded dashes:

```rust
for path in &all_prds {
    let entry = read_prd_entry(path);
    println!(
        "{:<35} {:<12} {:<6} {:<6} {:<8}",
        entry.slug, entry.status, "—", "—", "—"   // ← always dashes
    );
}
```

There's no code to match plans to PRDs. The `[meta]` section of `tasks.toml` has a
`plan` field (e.g. `plan = "dry-run-flag"`) which happens to match the PRD slug, but
`cmd_status` never does this lookup.

### The data that exists but isn't connected:

```
.roko/prd/published/dry-run-flag.md   ← PRD slug: "dry-run-flag"
plans/dry-run-flag/tasks.toml         ← meta.plan = "dry-run-flag"
```

These match by name convention, but the code doesn't check. Meanwhile:

```
plans/P06-process-management/tasks.toml   ← no matching PRD
plans/self-dev-ux/tasks.toml              ← no matching PRD
plans/architecture-core-queue/tasks.toml  ← no matching PRD
```

11 plans exist, but only 1 (dry-run-flag) has a matching PRD slug. The other 10 are
plans created independently without the PRD workflow.

### Additional issue: "unknown" status

`self-developing-workflow.md` likely has no YAML frontmatter or a missing `status` field,
so `PrdMeta::parse` returns `None` and `read_prd_entry` falls back to status "unknown".

## What the Report Should Look Like

```
═══ PRD Coverage Report ═══

PRD                              Status     Plans  Tasks  Done
dry-run-flag                     published  1      10     0/10 (0%)
costs                            draft      0      —      —
cursor-composer-backend          draft      0      —      —
self-developing-workflow         draft      0      —      —
test-quick                       draft      0      —      —

Linked:    1/5 PRDs have plans
Unlinked:  10 plans without PRDs (P06-process-management, P07-autofix-retry, ...)
Global:    11 plans, 136 tasks, 19 done (14%)
```

## Fix

### Fix 1: Link plans to PRDs by slug match (~20 min)

**File:** `crates/roko-cli/src/prd.rs:752-819`

```rust
pub fn cmd_status(workdir: &Path, plans_dir: Option<&Path>) -> Result<()> {
    ensure_dirs(workdir)?;

    let all_prds: Vec<PathBuf> = list_md_files(&published_dir(workdir))
        .into_iter()
        .chain(list_md_files(&drafts_dir(workdir)))
        .collect();

    let plans_root = plans_dir.map_or_else(|| workspace_plans_dir(workdir), Path::to_path_buf);

    // Build a map: plan_name → (task_count, done_count)
    let mut plan_stats: std::collections::HashMap<String, (u32, u32)> =
        std::collections::HashMap::new();
    let mut total_plans = 0u32;
    let mut total_tasks = 0u32;
    let mut total_done = 0u32;
    if plans_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&plans_root) {
            for entry in entries.flatten() {
                let toml_path = entry.path().join("tasks.toml");
                if toml_path.exists() {
                    total_plans += 1;
                    let content = std::fs::read_to_string(&toml_path).unwrap_or_default();
                    let tasks = content.matches("status = ").count() as u32;
                    let done = content.matches("status = \"done\"").count() as u32;
                    total_tasks += tasks;
                    total_done += done;

                    let plan_name = entry.file_name().to_string_lossy().to_string();
                    plan_stats.insert(plan_name, (tasks, done));
                }
            }
        }
    }

    println!("═══ PRD Coverage Report ═══");
    println!();
    println!("{:<30} {:<12} {:<6} {:<12}", "PRD", "Status", "Plans", "Progress");
    println!("{:<30} {:<12} {:<6} {:<12}", "───", "──────", "─────", "────────");

    let mut linked_count = 0u32;
    let mut unlinked_plans: Vec<String> = plan_stats.keys().cloned().collect();

    for path in &all_prds {
        let entry = read_prd_entry(path);
        if let Some((tasks, done)) = plan_stats.get(&entry.slug) {
            linked_count += 1;
            unlinked_plans.retain(|p| p != &entry.slug);
            println!(
                "{:<30} {:<12} {:<6} {}/{} ({:.0}%)",
                entry.slug, entry.status, 1, done, tasks,
                if *tasks > 0 { f64::from(*done) / f64::from(*tasks) * 100.0 } else { 0.0 }
            );
        } else {
            println!("{:<30} {:<12} {:<6} —", entry.slug, entry.status, 0);
        }
    }

    if all_prds.is_empty() {
        println!("  (no PRDs yet)");
    }

    println!();
    println!("Linked: {linked_count}/{} PRDs have plans", all_prds.len());
    if !unlinked_plans.is_empty() {
        unlinked_plans.sort();
        println!("Unlinked: {} plans without PRDs ({})",
            unlinked_plans.len(),
            unlinked_plans.iter().take(5).cloned().collect::<Vec<_>>().join(", "),
        );
    }
    println!(
        "Global: {} plans, {} tasks, {} done ({:.0}%)",
        total_plans, total_tasks, total_done,
        if total_tasks > 0 { f64::from(total_done) / f64::from(total_tasks) * 100.0 } else { 0.0 }
    );

    Ok(())
}
```

### Fix 2: Fix "unknown" status for drafts (~5 min)

**File:** `crates/roko-cli/src/prd.rs:625-647`

In `read_prd_entry`, when `PrdMeta::parse` returns `None`, default status to "draft" if
the file is in the drafts directory, "published" if in published:

```rust
fn read_prd_entry(path: &Path) -> PrdEntry {
    let slug = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let content = std::fs::read_to_string(path).unwrap_or_default();
    if let Some(meta) = PrdMeta::parse(&content) {
        PrdEntry { slug, title: meta.title, status: meta.status, coverage: meta.coverage }
    } else {
        // Infer status from directory location.
        let status = if path.to_string_lossy().contains("/published/") {
            "published"
        } else if path.to_string_lossy().contains("/drafts/") {
            "draft"
        } else {
            "unknown"
        };
        PrdEntry { slug: slug.clone(), title: slug, status: status.into(), coverage: 0.0 }
    }
}
```

### Fix 3: Add `source_prd` to TaskMeta (optional, longer term)

**File:** `crates/roko-cli/src/task_parser.rs`

Add an optional `source_prd` field so plans explicitly declare their PRD:
```rust
pub struct TaskMeta {
    pub plan: String,
    #[serde(default)]
    pub source_prd: Option<String>,  // ← PRD slug this plan was generated from
    // ...
}
```

When `roko prd plan <slug>` generates a plan, it should set `source_prd = "<slug>"` in
the meta. This makes the linkage explicit instead of relying on name matching.

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/prd.rs:752-819` | Link plans to PRDs by slug, show real stats |
| `crates/roko-cli/src/prd.rs:625-647` | Infer status from directory for missing frontmatter |
| `crates/roko-cli/src/task_parser.rs:22` | (Optional) Add `source_prd` field to `TaskMeta` |

## Priority

**P2** — The command runs but produces useless output. The global totals at the bottom are
correct but the per-PRD breakdown is always empty. Combined with the `/prd-draft` issues
(#16) and the tool alias bug (#15), the entire PRD pipeline is non-functional for tracking
purposes. The slug-matching fix is straightforward and would make the status report useful
immediately.
