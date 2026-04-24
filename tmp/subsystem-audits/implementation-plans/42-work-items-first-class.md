# 42 — Work Items as First-Class Objects

Doc 42 idea B: today, "work" is scattered across PRDs, plans, tasks,
episodes, executor snapshots, and signals — each with its own format.
A `WorkItem` wraps everything coherently.

Depends on plan 40 (5 verbs) and ideally plan 41 (universal session).

---

## Today's State

To resume a plan, the user has to know:
- the plan directory path
- `--resume .roko/state/executor.json`
- whether the plan was created from a PRD (different path)
- whether the executor snapshot is stale
- which git branch the work happened on

There's no single "name this thing I'm doing" abstraction.

---

## Anti-Patterns

1. **Don't replace PRDs/plans/tasks.** Wrap them.
2. **Don't add a new persistence root.** Work items live under
   `.roko/work/<id>/`.
3. **Don't create a name collision with `roko-runtime`'s run ledger.**
   A work item *contains* a run ledger; it's not the same thing.
4. **Don't auto-create work items for trivial work** (a single
   `roko do "fix typo"`). Threshold: medium formality and above.

---

## Plan

### Phase 1: Define `WorkItem` and persistence

**File**: `crates/roko-core/src/work_item.rs` (referenced from plan 40
WF-6).

```rust
pub struct WorkItem {
    pub id: String,                        // human-readable, e.g. "auth-redesign"
    pub status: WorkStatus,
    pub created: chrono::DateTime<chrono::Utc>,
    pub prompt: String,
    pub formality: Formality,

    pub prd: Option<PathBuf>,              // .roko/work/<id>/prd.md
    pub plan_dir: Option<PathBuf>,         // .roko/work/<id>/plan/
    pub run_ledger: Option<PathBuf>,       // .roko/work/<id>/ledger.jsonl
    pub git_branch: Option<String>,
    pub cost: CostSummary,
}

impl WorkItem {
    pub fn directory(workdir: &Path, id: &str) -> PathBuf {
        workdir.join(".roko").join("work").join(id)
    }

    pub async fn create(workdir: &Path, prompt: String, formality: Formality) -> Result<Self> {
        let id = generate_id(&prompt);
        let dir = Self::directory(workdir, &id);
        tokio::fs::create_dir_all(&dir).await?;
        let item = Self { id, status: WorkStatus::Created, /* ... */ };
        item.save(workdir).await?;
        Ok(item)
    }

    pub async fn save(&self, workdir: &Path) -> Result<()> {
        let path = Self::directory(workdir, &self.id).join("work_item.json");
        let json = serde_json::to_string_pretty(self)?;
        crate::persistence::atomic_write(&path, json.as_bytes()).await?;
        Ok(())
    }

    pub async fn list(workdir: &Path) -> Result<Vec<Self>> {
        let dir = workdir.join(".roko").join("work");
        // ...
    }
}
```

### Phase 2: Wire `roko do` to create work items (medium+ formality)

```rust
async fn cmd_do(prompt: String, opts: DoOpts) -> Result<()> {
    let formality = intent::classify(&prompt, &workspace);
    let work_item = match formality {
        Formality::Trivial => None,    // no work item for tiny work
        Formality::Small | Formality::Medium | Formality::Large => {
            Some(WorkItem::create(&workdir, prompt.clone(), formality).await?)
        }
    };

    // Dispatch through existing planner / runner
    // The work item directory becomes the plan_dir.
    if let Some(item) = &work_item {
        run_plan(&item.plan_dir.as_ref().unwrap(), &item.run_ledger.as_ref().unwrap()).await?;
    } else {
        run_one_shot(&prompt).await?;
    }
    Ok(())
}
```

### Phase 3: `roko show`

Lists work items with status/cost/duration:

```rust
async fn cmd_show(target: Option<String>) -> Result<()> {
    if let Some(id) = target {
        let item = WorkItem::load(&workdir, &id).await?;
        print_work_item_detail(&item);
    } else {
        let items = WorkItem::list(&workdir).await?;
        print_work_item_table(&items);
    }
    Ok(())
}
```

### Phase 4: Resume by name

```rust
async fn cmd_do_continue(id: &str) -> Result<()> {
    let item = WorkItem::load(&workdir, id).await?;
    if !matches!(item.status, WorkStatus::Paused | WorkStatus::Running) {
        return Err("not paused / running".into());
    }
    resume_plan(item.plan_dir.unwrap(), item.run_ledger.unwrap()).await?;
    Ok(())
}
```

`roko do --continue auth-redesign` becomes the canonical resume.

### Phase 5: Surface in TUI

The TUI's "Work" tab (per plan 40 / doc 42 idea D) lists work items
and shows their detail.

---

## Status

- [ ] Phase 1 — `WorkItem` type + persistence
- [ ] Phase 2 — `roko do` creates work items
- [ ] Phase 3 — `roko show` lists / shows detail
- [ ] Phase 4 — Resume by name
- [ ] Phase 5 — TUI surface

**Estimated effort**: 20-30 hours.

**Order of dependencies**: plan 40 (WF-1..WF-6) must land first.
