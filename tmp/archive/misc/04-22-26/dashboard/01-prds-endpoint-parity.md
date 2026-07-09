# Task 01: PRD Endpoint Parity

**Priority**: P0
**Crate**: `roko-serve`
**File**: `crates/roko-serve/src/routes/prds.rs`

## Problem

The dashboard expects richer PRD data than roko-serve currently returns.

### GET /api/prds — missing fields

**What roko-serve returns now:**
```json
[
  { "slug": "prd-slug", "status": "draft" }
]
```

**What the dashboard expects** (defined in `nunchi-dashboard/src/services/rokoApi.ts`, type `Prd`):
```json
[
  {
    "slug": "prd-slug",
    "title": "Human-readable title",
    "status": "idea" | "draft" | "published",
    "section": "optional-section-name",
    "has_plan": true
  }
]
```

Missing fields: `title`, `section`, `has_plan`. The dashboard renders these in the PRDs tab
and the Atelier chat `IdeaCard` component.

### POST /api/prds/ideas — wrong response shape

**What roko-serve returns now:**
```json
{ "status": "appended" }
```

**What the dashboard expects:**
```json
{ "slug": "generated-slug" }
```

The dashboard uses the returned `slug` to chain commands: after `/idea`, it offers
`/draft <slug>` and `/publish <slug>`. Without a slug, the chain breaks.

### Status value: "idea" not supported

Roko-serve returns `"draft"` or `"published"`. The dashboard also uses `"idea"` as a
third status. Ideas are currently appended to `.roko/prd/ideas.md` as freetext, not
tracked as individual PRDs with slugs.

## Implementation

### Step 1: Give ideas individual identity

Currently `POST /api/prds/ideas` appends text to `.roko/prd/ideas.md`. Change it to:

1. Generate a slug from the text (e.g., `slugify(first 5 words)` + 4-char random suffix)
2. Create a file `.roko/prd/ideas/{slug}.md` with frontmatter:
   ```markdown
   ---
   title: <first line or first ~60 chars of text>
   status: idea
   created: <ISO timestamp>
   ---

   <full idea text>
   ```
3. Return `{ "slug": "<slug>" }` with status 201

**Alternative** (simpler, if you don't want to change the idea storage format): keep appending
to `ideas.md` but also return a synthetic slug. Store a mapping of slug → line offset in
the ideas file. This is fragile — prefer the file-per-idea approach.

### Step 2: Enrich GET /api/prds response

The handler at `crates/roko-serve/src/routes/prds.rs` currently reads draft and published
directories. Extend it:

1. Also scan `.roko/prd/ideas/` directory for idea files
2. For each PRD (idea, draft, published), parse frontmatter to extract `title`
3. Extract `section` from frontmatter if present (optional field)
4. Check whether a plan exists for this slug:
   - Scan `.roko/plans/` directory for any plan whose source PRD matches the slug
   - OR check if the PRD frontmatter has a `plan_id` field
   - Set `has_plan: true/false` accordingly
5. Return the enriched array

**Serialization struct:**
```rust
#[derive(Serialize)]
struct PrdSummary {
    slug: String,
    title: String,
    status: String,       // "idea" | "draft" | "published"
    section: Option<String>,
    has_plan: bool,
}
```

### Step 3: Verify draft/promote still work

The `/draft` and `/promote` endpoints should work with the new idea files. When drafting
from an idea:
1. Read `.roko/prd/ideas/{slug}.md`
2. Move/transform it into `.roko/prd/drafts/{slug}.md`
3. Update status in frontmatter to `draft`

Check `POST /api/prds/{slug}/draft` and `POST /api/prds/{slug}/promote` handlers to
confirm they handle idea → draft → published transitions.

## Files to modify

| File | Change |
|------|--------|
| `crates/roko-serve/src/routes/prds.rs` | All three changes above |
| `crates/roko-core/src/lib.rs` (or wherever `PrdStatus` is defined) | Add `Idea` variant if enum exists |

## Verification

### Automated

```bash
# Must compile
cargo build -p roko-serve

# Must pass tests
cargo test -p roko-serve

# Must pass clippy
cargo clippy -p roko-serve --no-deps -- -D warnings
```

### Manual — POST /api/prds/ideas

```bash
# Start serve
cargo run -p roko-cli -- serve &

# Create an idea
RESP=$(curl -s -X POST http://127.0.0.1:6677/api/prds/ideas \
  -H 'Content-Type: application/json' \
  -d '{"text": "Add real-time notifications for agent failures"}')

echo "$RESP"
# MUST contain: {"slug": "add-real-time-notif-xxxx"} (or similar)
# MUST NOT contain: {"status": "appended"}

SLUG=$(echo "$RESP" | jq -r '.slug')
[ -n "$SLUG" ] && echo "PASS: slug returned" || echo "FAIL: no slug"
```

### Manual — GET /api/prds

```bash
PRDS=$(curl -s http://127.0.0.1:6677/api/prds)

# Check first PRD has all required fields
echo "$PRDS" | jq '.[0] | keys'
# MUST include: slug, title, status, has_plan
# section is optional (may be null)

# Verify idea appears with status "idea"
echo "$PRDS" | jq '.[] | select(.status == "idea")'
# MUST return the idea we just created

# Verify has_plan is boolean
echo "$PRDS" | jq '.[0].has_plan | type'
# MUST be "boolean"
```

### Manual — draft chain

```bash
# Draft the idea
curl -s -X POST "http://127.0.0.1:6677/api/prds/${SLUG}/draft"

# Verify status changed
curl -s http://127.0.0.1:6677/api/prds | jq ".[] | select(.slug == \"${SLUG}\")"
# status MUST be "draft"

# Promote
curl -s -X POST "http://127.0.0.1:6677/api/prds/${SLUG}/promote"

# Verify
curl -s http://127.0.0.1:6677/api/prds | jq ".[] | select(.slug == \"${SLUG}\")"
# status MUST be "published"
```

## Acceptance criteria

- [ ] `POST /api/prds/ideas` returns `{ "slug": "<string>" }` with status 201
- [ ] `GET /api/prds` returns array where every element has: `slug`, `title`, `status`, `has_plan`
- [ ] Ideas appear in the list with `status: "idea"`
- [ ] `has_plan` is correctly `true` when a plan exists for the PRD, `false` otherwise
- [ ] `/draft` and `/promote` work for ideas (idea → draft → published lifecycle)
- [ ] All existing tests still pass
- [ ] No new clippy warnings
