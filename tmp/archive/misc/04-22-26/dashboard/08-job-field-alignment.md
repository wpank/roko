# Task 08: Job Field Alignment

**Priority**: P1
**Crate**: `roko-serve`
**File**: `crates/roko-serve/src/routes/jobs.rs`

## Problem

The dashboard's `Job` type uses different field names and state values than roko-serve.

### Field name mismatches

**Dashboard expects** (`nunchi-dashboard/src/types/api.ts`):
```typescript
type Job = {
  id: string;
  title: string;
  description: string;
  job_type: "research" | "coding_task" | "review" | "documentation" | "testing";
  state: string;          // ← "state"
  posted_by: string;
  assigned_to: string | null;
  created_at: string;
  updated_at: string;
  submission: { ... } | null;
  evaluation: { ... } | null;
  metadata: Record<string, unknown>;
  reward?: number;
  required_capabilities?: string[];
  deadline?: string | null;
}
```

**Roko-serve returns:**
```json
{
  "id": "...",
  "title": "...",
  "description": "...",
  "job_type": "other",
  "status": "open",         // ← "status" (not "state")
  "posted_by": "...",
  "assigned_to": "...",
  "priority": "high",
  "created_at": "...",
  "updated_at": "...",
  "tags": [...],
  "reward": "100",           // ← string (not number)
  "plan_id": "...",
  "auto_execute": false,
  "submission": null,
  "evaluation": null
}
```

**Key mismatches:**

1. **`status` vs `state`**: Dashboard reads `job.state`, roko-serve returns `job.status`.
   This means the dashboard sees `undefined` for every job's state.

2. **`reward` type**: Roko-serve returns `"100"` (string), dashboard expects `100` (number).
   This may cause display issues or NaN in calculations.

3. **Missing `metadata` field**: Dashboard expects `metadata: Record<string, unknown>`.
   If roko-serve doesn't include it, the dashboard may error on `Object.keys(job.metadata)`.

4. **Missing `required_capabilities`**: Dashboard may render this in job detail views.

5. **`job_type` values**: Roko-serve uses `"other"` which isn't in the dashboard's expected
   set. Dashboard expects: `"research"`, `"coding_task"`, `"review"`, `"documentation"`,
   `"testing"`. Mismatched types may render as "Unknown" or break filters.

### State value alignment

**Dashboard expected states:**
```
"open" | "assigned" | "in_progress" | "submitted" | "evaluated" | "cancelled" | "expired"
```

**Roko-serve states** (check the Job enum in `roko-core` or `roko-serve`):
May use different names (e.g., `"completed"` instead of `"evaluated"`, `"pending"` instead
of `"open"`). Audit the actual enum values.

### Submission and evaluation nested shapes

**Dashboard expects `submission`:**
```json
{
  "agent_id": "string",
  "result_summary": "string",
  "artifacts": ["string"],
  "gate_results": [{ "gate": "string", "passed": true, "detail": "string" }],
  "submitted_at": "string"
}
```

**Dashboard expects `evaluation`:**
```json
{
  "evaluator": "string",
  "accepted": true,
  "score": 0.95,
  "feedback": "string",
  "evaluated_at": "string"
}
```

Verify these nested objects match. The dashboard accesses these fields in job detail views.

## Implementation

### Step 1: Rename `status` → `state` in serialization

Find the Job serialization struct. Add a serde rename:

```rust
#[derive(Serialize)]
struct JobResponse {
    // ...
    #[serde(rename = "state")]
    status: String,
    // ...
}
```

Or rename the field itself if it makes sense across the codebase. The `#[serde(rename)]`
approach is safest — it changes only the JSON output without touching internal code.

### Step 2: Serialize `reward` as number

If `reward` is stored as a string (e.g., from TOML/JSON), parse it to a number before
serializing:

```rust
#[serde(serialize_with = "serialize_reward")]
reward: Option<String>,
```

Or change the field type to `Option<f64>` / `Option<u64>` and parse on load.

### Step 3: Add missing fields with defaults

Ensure these fields exist in the response, even if empty:

```rust
metadata: serde_json::Value,           // default: {}
required_capabilities: Vec<String>,     // default: []
deadline: Option<String>,               // default: null
```

### Step 4: Align job_type values

Audit what `job_type` values roko-serve produces. Map any non-standard values to the
dashboard's expected set:

| Roko-serve | Dashboard | Action |
|------------|-----------|--------|
| `"other"` | not in set | Map to closest match or keep (dashboard should handle gracefully) |
| `"implementation"` | `"coding_task"` | Rename |
| `"research"` | `"research"` | OK |
| `"review"` | `"review"` | OK |

Add `#[serde(rename)]` or a mapping function.

### Step 5: Align state values

Map internal state names to dashboard-expected names:

| Internal | Dashboard | Action |
|----------|-----------|--------|
| `"pending"` | `"open"` | Rename |
| `"completed"` | `"evaluated"` | Rename |
| `"running"` | `"in_progress"` | Rename |
| Others | Check | Map as needed |

### Step 6: Verify submission/evaluation shapes

Read the Job struct's `submission` and `evaluation` fields. Ensure nested field names match
the dashboard's expected shape. Key fields to verify:

- `submission.gate_results` — dashboard expects array of `{gate, passed, detail}`, not
  a `Record<string, boolean>` (though it handles both).
- `evaluation.accepted` — dashboard uses this, not a separate `passed` field.

## Files to modify

| File | Change |
|------|--------|
| `crates/roko-serve/src/routes/jobs.rs` | Serialization adjustments |
| `crates/roko-core/src/` (job types) | May need to adjust core job type if rename affects it |

## Verification

### Automated

```bash
cargo build -p roko-serve
cargo test -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
```

### Manual

```bash
cargo run -p roko-cli -- serve &

# Create a job
RESP=$(curl -s -X POST http://127.0.0.1:6677/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{"title": "Test job", "description": "Testing field alignment", "job_type": "research"}')

echo "$RESP" | jq .
JOB_ID=$(echo "$RESP" | jq -r '.id')

# Verify field names
echo "$RESP" | jq 'has("state")'
# MUST be true

echo "$RESP" | jq 'has("status")'
# MUST be false (or if both exist, "state" must be the canonical one)

# Verify state value
echo "$RESP" | jq -r '.state'
# MUST be "open" (not "pending" or other)

# Verify reward type (if present)
echo "$RESP" | jq '.reward | type'
# MUST be "number" or "null" — NOT "string"

# Verify metadata exists
echo "$RESP" | jq 'has("metadata")'
# MUST be true

# List jobs
JOBS=$(curl -s http://127.0.0.1:6677/api/jobs)
echo "$JOBS" | jq '.[0] | keys'
# MUST include: id, title, description, job_type, state, posted_by, created_at, updated_at
```

### Manual — job lifecycle

```bash
# Assign
curl -s -X POST "http://127.0.0.1:6677/api/jobs/${JOB_ID}/assign" \
  -H 'Content-Type: application/json' \
  -d '{"agent_id": "test-agent"}'

curl -s "http://127.0.0.1:6677/api/jobs/${JOB_ID}" | jq '.state'
# MUST be "assigned"

# Start
curl -s -X POST "http://127.0.0.1:6677/api/jobs/${JOB_ID}/start"

curl -s "http://127.0.0.1:6677/api/jobs/${JOB_ID}" | jq '.state'
# MUST be "in_progress"

# Submit
curl -s -X POST "http://127.0.0.1:6677/api/jobs/${JOB_ID}/submit" \
  -H 'Content-Type: application/json' \
  -d '{"agent_id": "test-agent", "result_summary": "Done", "artifacts": []}'

curl -s "http://127.0.0.1:6677/api/jobs/${JOB_ID}" | jq '.state'
# MUST be "submitted"

# Evaluate
curl -s -X POST "http://127.0.0.1:6677/api/jobs/${JOB_ID}/evaluate" \
  -H 'Content-Type: application/json' \
  -d '{"accepted": true, "score": 0.9, "feedback": "Good work"}'

curl -s "http://127.0.0.1:6677/api/jobs/${JOB_ID}" | jq '.state'
# MUST be "evaluated"
```

## Acceptance criteria

- [ ] Job responses use `state` (not `status`) for the state field
- [ ] State values match dashboard expectations: `open`, `assigned`, `in_progress`, `submitted`, `evaluated`, `cancelled`, `expired`
- [ ] `reward` serializes as a number (not string)
- [ ] `metadata` field always present (default `{}`)
- [ ] `required_capabilities` field always present (default `[]`)
- [ ] `deadline` field present (nullable)
- [ ] `submission` and `evaluation` nested shapes match dashboard types
- [ ] Job lifecycle transitions produce correct state values
- [ ] All existing tests still pass
- [ ] No new clippy warnings
