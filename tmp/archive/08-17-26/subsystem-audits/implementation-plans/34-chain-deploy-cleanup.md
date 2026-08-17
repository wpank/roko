# 34 — Chain / Deploy / Demo Cleanup

`roko-chain` (~14K of 42K LOC) has dormant code; deploy is fully wired;
the demo is standalone benchmark harness work. Plan 26 covers demo
truth migration; this plan covers chain cleanup and deploy
follow-ups.

Source: subsystem-audits/chain-deploy-demo/AUDIT.md.

---

## Plan

### [ ] CD-1: Inventory dormant chain code

```bash
rg -l 'pub fn|pub async fn' crates/roko-chain/src/ \
  | xargs -I {} sh -c 'echo "=== {} ==="; rg "use roko_chain::|crate::" {} | head -3'
```

Identify which `roko-chain` modules have callers in production code.

Likely dormant (per audit):

- `marketplace.rs` — 2K LOC, only test callers; uses `dispatch_direct`
  which is being deprecated.
- `identity_economy_markets.rs` — similar.
- Various `pricing_*` modules — speculative pricing schemes.

### [ ] CD-2: Quarantine or delete dormant chain modules

Same procedure as T2-17 (plan 12):

1. Confirm zero non-test callers.
2. Delete the module.
3. Build clean.

If the module is "we plan to ship this," feature-gate behind
`chain-experimental` and document the plan.

### [ ] CD-3: Verify deploy paths

The audit calls deploy "fully wired" (Railway/Docker/daemon). Verify:

```bash
cargo run -p roko-cli -- deploy --dry-run
```

Should plan a Railway deployment without errors. If recent changes
broke this, file a separate task.

### [ ] CD-4: Clean up `contracts/` build artifacts

```bash
ls contracts/broadcast/Deploy.s.sol/1/
```

There are recent `run-*.json` artifacts from foundry runs. Add to
`.gitignore` if they shouldn't be committed:

```
contracts/broadcast/**/run-*.json
```

(Confirm with `git check-ignore` before committing the change.)

---

**Estimated effort**: 6-10 hours, mostly inventory + deletion.
