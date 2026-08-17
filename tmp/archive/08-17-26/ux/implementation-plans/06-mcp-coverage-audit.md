# 06 — MCP Coverage Audit (`roko-mcp-*`)

> **Source plan**: `tmp/ux/ux-followup/05-partially-wired-subsystems.md`
> item 34.
>
> **Status as of 2026-05-01**: Five MCP server crates exist
> (`roko-mcp-code`, `-github`, `-slack`, `-scripts`, `-stdio`). Only
> `roko-mcp-code` is documented as "wired" in CLAUDE.md. The other four
> have no integration test, no documented dispatch path in
> `roko.toml.mcp_config`, and no ship-gate evidence. PR #13 advertises
> broad MCP coverage; the catalogue calls this "ship-gate for any release
> that advertises broad MCP coverage."
>
> **Effort**: 2-3 days audit + 1-3 days per crate to finish or document
> deprecation. Total 5-12 days depending on outcome.
>
> **Risk**: Low for the audit; medium if a crate is judged
> not-shippable and we have to remove it from advertised features.

---

## What this plan accomplishes

A clear, shipped status per MCP crate. After this plan, every
`roko-mcp-*` crate is in one of four buckets:

1. **Shipped, default-on**: integrated end-to-end, documented in
   `CLAUDE.md`, smoke-tested in CI.
2. **Shipped, opt-in**: works but requires explicit `roko.toml`
   configuration. Documented as opt-in.
3. **Deprecated**: removed from the workspace and from advertised
   features. README has a redirect.
4. **In progress**: explicit gap list; no shipped claim made.

No more "compiles, listed under Key crates as partial".

## Why this matters

External users see "Roko supports GitHub / Slack / scripts / stdio MCP"
in CLAUDE.md and the PR body. If the crate doesn't actually run in the
default workflow, this is a false claim and a loaded foot-gun (someone
files an issue, we have to triage from scratch).

---

## Required reading

```
crates/roko-mcp-code/                      (the ship-gate reference)
crates/roko-mcp-github/                    (~2643 LOC, partial)
crates/roko-mcp-slack/                     (~920 LOC, partial)
crates/roko-mcp-scripts/                   (~767 LOC, partial)
crates/roko-mcp-stdio/                     (~246 LOC, lib.rs only — likely a transport)
crates/roko-core/src/mcp/                  (any MCP traits the crates implement)
crates/roko-cli/src/dispatch/              (dispatch routing — does it touch MCP?)
crates/roko-agent/src/dispatcher/mod.rs    (MCP dispatch)
roko.toml + roko.toml.example              (mcp_config keys)
docs/v2/MCP-INTEGRATION.md                 (if present)
CLAUDE.md (Key crates table, MCP rows)
tmp/ux/ux-followup/05-partially-wired-subsystems.md (item 34)
```

---

## Deliverables

### Audit (2-3 days)

For each of the four crates besides `roko-mcp-code`:

1. **Symbol audit**: list public types, traits, transports, and tools.
2. **Call-site audit**: grep for non-test imports of each public symbol.
   No call site outside the crate's own tests = the crate is
   self-contained orphan.
3. **Wire-up audit**: trace from `roko-cli` (or `roko-serve`) startup to
   each crate's entry point. Document the path in `docs/v2/MCP-WIRING.md`.
4. **Behaviour audit**: spawn each MCP server (likely under
   `cargo run -p <crate> -- --stdio`), send the standard MCP handshake,
   request tool listing, exercise one tool. Capture the transcript.
5. **Decision**: rank into the four buckets.

### Decision matrix template

Save as `docs/v2/MCP-AUDIT.md`:

| Crate | LOC | External call sites | Handshake works | Tool listing works | One tool exercise | Bucket | Owner | Notes |
|-------|-----|---------------------|-----------------|--------------------|-------------------|--------|-------|-------|
| roko-mcp-code | … | many | yes | yes | yes | shipped-default | … | reference |
| roko-mcp-github | 2643 | ? | ? | ? | ? | ? | ? | … |
| roko-mcp-slack | 920 | ? | ? | ? | ? | ? | ? | … |
| roko-mcp-scripts | 767 | ? | ? | ? | ? | ? | ? | … |
| roko-mcp-stdio | 246 | ? | ? | ? | ? | ? | transport-only? | … |

### Implementation per bucket

**Bucket 1 — Shipped, default-on**:
- Add the crate to `roko.toml.example` `mcp_config`.
- Add an integration test under `crates/<crate>/tests/integration.rs`
  that spawns the server, sends the handshake, and asserts at least one
  tool is callable. Run it in CI.
- Update CLAUDE.md row to "Wired".
- Document in `docs/v2/MCP-INTEGRATION.md` how to enable.

**Bucket 2 — Shipped, opt-in**:
- Same as bucket 1 but `roko.toml.example` keeps the crate commented
  out with "uncomment to enable" guidance.
- CI test runs but is gated behind a feature flag or env knob.

**Bucket 3 — Deprecated**:
- Move to `bardo-backup/` or open a deletion PR. Don't accumulate
  archive crates inside the workspace.
- Update CLAUDE.md to remove the crate from the Key crates table.
- Add a redirect note in `docs/v2/MCP-INTEGRATION.md` pointing at the
  replacement (or noting "no replacement").

**Bucket 4 — In progress**:
- File a tracking issue.
- Mark the crate in `Cargo.toml` workspace members as `# WIP — see issue #N`.
- CLAUDE.md row reads "Phase 2+ (in progress)".
- No more advertised "wired" claim until the matrix turns green.

### Cleanup

- `roko-mcp-stdio` is suspected of being a transport library, not an MCP
  server. Confirm by reading `lib.rs`. If so, document that explicitly
  (no MCP server claim) and reuse it in the other crates' integration
  tests.
- Reduce the per-crate `Cargo.toml` description fields to be honest. Don't
  say "production-ready" if the matrix says otherwise.

---

## Step-by-step

### Step 1 — Generate the matrix (1 day)

For each crate (`github`, `slack`, `scripts`, `stdio`):

```bash
crate=roko-mcp-github
echo "=== $crate ==="
echo "## Public API"
rg -t rust '^pub (struct|fn|trait|enum)' crates/$crate/src/

echo "## External call sites"
rg --type rust -l "$(echo $crate | tr - _)::" crates/ apps/ \
   | grep -v "crates/$crate/"
```

Record the matrix row.

### Step 2 — Behaviour smoke per crate (1 day)

For an MCP server with stdio transport:

```bash
# In one terminal:
cargo run -p roko-mcp-github -- --stdio < /dev/null
# Send the MCP handshake from another terminal:
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}' \
  | cargo run -p roko-mcp-github -- --stdio
# Tool listing:
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | …
# Tool call (use the first listed tool):
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"<X>"}}' | …
```

Capture the transcripts under
`tmp/ux/implementation-plans/fixtures/mcp-<crate>-handshake.txt`.

### Step 3 — Decide bucket per crate (half day)

Bring the matrix to the team. The audit author has signal but not the
final call (especially for slack/github which have business
implications). Update the matrix with the decision.

### Step 4 — Per-crate implementation (1-3 days each, parallelisable)

Apply the bucket-specific work above. Each crate is its own commit so
rollback is granular.

### Step 5 — Documentation (half day)

- `docs/v2/MCP-INTEGRATION.md`: rewrite the "what's wired" table.
- `CLAUDE.md`: update the Key crates rows.
- `tmp/ux/ux-followup/05-partially-wired-subsystems.md`: mark item 34
  with a "Closed YYYY-MM-DD" header and link to this plan.

---

## Anti-patterns to avoid

- **Don't bucket every crate as "shipped-default"** to make the matrix
  green. The audit must be honest. False green now means false
  outage signal later.
- **Don't write smoke tests that only assert "the server starts".** The
  bug is "tool calls hang forever in production"; tests must drive a
  call that exercises the actual logic.
- **Don't widen `mcp_config` schema "for future use".** Add only what's
  needed to enable each shipped crate. Schema growth without a
  consumer is dead weight.
- **Don't keep the deprecated crates as workspace members.** They will
  drift. Either move to `bardo-backup/` or delete from the workspace.
- **Don't conflate `roko-mcp-stdio` with a server.** `stdio` is most
  likely the transport adapter consumed by the others. Confirm before
  bucketing.

## Done when

1. `docs/v2/MCP-AUDIT.md` exists with one row per crate, all cells
   filled.
2. `docs/v2/MCP-INTEGRATION.md` lists exactly the shipped crates and
   their config knobs.
3. CLAUDE.md "Key crates" rows for the MCP family match
   `MCP-AUDIT.md`.
4. Each shipped crate has at least one integration test in
   `crates/<crate>/tests/`.
5. `tmp/ux/ux-followup/05-partially-wired-subsystems.md` item 34 closed.
6. `cargo test --workspace` passes including the new MCP integration
   tests.
