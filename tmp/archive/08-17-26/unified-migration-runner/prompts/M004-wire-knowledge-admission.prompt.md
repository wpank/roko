# M004 — Wire KnowledgeAdmissionController

## Objective
The KnowledgeAdmissionController (or similar knowledge-gating logic) exists in roko-neuro
but is not called from the main orchestration loop or dispatch path. Wire it so that
knowledge signals are validated before being persisted.

## Scope
- Crates: `roko-neuro`, `roko-cli`
- Files: relevant neuro source, `crates/roko-cli/src/orchestrate.rs`
- Phase ref: 01-PHASE-0-PREP.md §0.1

## Steps
1. Find the admission controller:
   `grep -rn 'AdmissionController\|admission\|knowledge.*gate\|knowledge.*filter' crates/roko-neuro/src/ --include='*.rs'`

2. Find where knowledge signals are written (persisted to neuro store):
   `grep -rn 'neuro.*put\|knowledge.*write\|knowledge.*store\|neuro.*store' crates/roko-cli/src/orchestrate.rs`

3. If an admission controller exists:
   - Import it in orchestrate.rs
   - Call it before knowledge persistence: `if admission.check(&signal).is_ok() { store.put(signal) }`
   - Log when admission rejects a signal

4. If no concrete admission controller exists:
   - Check if there's a quality gate or score threshold that could serve:
     `grep -rn 'score.*threshold\|quality.*gate\|min_score' crates/roko-neuro/src/ --include='*.rs'`
   - If found, wire that instead
   - If nothing exists, add a minimal check: reject signals with `score.confidence < 0.1`

## Verification
```bash
cargo check -p roko-neuro -p roko-cli
cargo clippy -p roko-neuro -p roko-cli --no-deps -- -D warnings
```

## What NOT to do
- Do NOT build a full admission controller — wire what exists or add minimal check
- Do NOT modify the neuro store API
