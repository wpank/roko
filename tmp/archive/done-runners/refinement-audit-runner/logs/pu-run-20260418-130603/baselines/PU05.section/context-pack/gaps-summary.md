# Gap Inventory — 05 Learning

Concise gap list for agents working on learning parity batches.

## Focus Now

These are the gaps batch `05` should actively try to close:

### 1. Learned Context Is Thinner In Production Than In The Library — HIGH

- `MatchContext` supports files, tags, categories, and error signatures,
- `SkillQuery` supports richer filtering,
- the main production path mostly uses role-only matching.

### 2. Regression Detection Ignores The Slice Model It Already Has — HIGH

- `Baseline.slices` are computed,
- `RegressionAlert.slice` exists,
- production alerts are still overall-only,
- `iterations_increase` is dead.

### 3. Predictive Calibration Has A Split Contract — HIGH

- routing-log replay and predictive prompt/scoring consumers are real,
- direct `PredictionRecord::register/resolve` is unused,
- Brier / reliability / arithmetic-corrector pieces are absent.

### 4. Cost And Experiment Loops Are Real But Not Fully Operator-Friendly — MEDIUM

- budget pressure is mostly override logic,
- experiment winners affect router state,
- durable operator-facing artifacts are weak.

### 5. A Few Big Learning Modules Are Still Ambiguous Dead Code — MEDIUM

- `run_learning_subscriber` has tests but no runtime caller,
- `DriftDetector` has no runtime caller,
- later agents cannot tell whether these are intended surfaces or abandoned scaffolding.

## Defer From Batch 05

These are valid findings, but they should usually be documented and handed off:

- tiered storage / HDC cold archives -> later storage hardening
- DBSCAN clustering -> later analytics pass
- advanced routing research -> post-parity routing pass
- full predictive foraging engine -> later forecasting pass
- scorecards / constitutional safety / significance testing -> eval or governance pass
- ADAS / EvoSkills optimization -> research pass

## Working Rule

If a learning task requires:

- a new routing research algorithm,
- a new storage architecture,
- or a governance / constitutional safety framework,

then batch `05` should normally implement the smallest runtime contract and defer the rest.
