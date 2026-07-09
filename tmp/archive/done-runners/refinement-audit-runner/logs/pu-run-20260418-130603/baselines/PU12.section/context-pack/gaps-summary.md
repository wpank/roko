# Gap Inventory — 12 Interfaces

## Focus Now

### 1. Doc 17 Still Undersells The Topic — HIGH

- it still says `Scaffold`,
- the topic has substantial shipping CLI/TUI/server/sidecar code.

### 2. CLI Truth Surface Needs Harder Corrections — HIGH

- `roko new` appears absent,
- standalone `roko explain` appears absent,
- later agents should not inherit fuzzy “partial” language here.

### 3. Port / Default Drift Is Real — HIGH

- `roko serve` defaults `9090` in code,
- chat and READMEs still point at `6677`,
- this should be explicit in the batch plan.

### 4. Server Docs Are Stronger Than Their Banners, But Some Endpoint Claims Need Narrower Language — MEDIUM

- route stacks and sidecar features ship,
- some detailed endpoint inventories were not verified end-to-end.

### 5. Frontier Halo Needs Harder Boundaries — MEDIUM

- Spectre
- portal
- A2UI
- sonification
- UX innovation
- IDE extension work

## Working Rule

If a batch starts requiring new frontend, visualization, audio, or IDE
runtime code, that is usually the wrong batch.
