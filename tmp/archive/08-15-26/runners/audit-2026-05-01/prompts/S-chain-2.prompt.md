# S-chain-2: Add foundry broadcast artifacts to .gitignore

## Task
Add `contracts/broadcast/**/run-*.json` (and similar) to `.gitignore`. These foundry deploy artifacts shouldn't be committed.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/34-chain-deploy-cleanup.md` § CD-4.

## Read first

```bash
ls contracts/broadcast/
git check-ignore contracts/broadcast/Deploy.s.sol/1/run-1777492613906.json
```

If `git check-ignore` returns nothing, the file isn't ignored. Per the git status at the start of this audit, `run-*.json` files are untracked, suggesting `.gitignore` doesn't currently catch them.

## Exact changes

### 1. Update `.gitignore`

Add (or extend an existing `# Foundry` block):

```gitignore
# Foundry deploy artifacts (per-run; do not commit)
contracts/broadcast/**/run-*.json
contracts/broadcast/**/run-latest.json
contracts/cache/
contracts/out/
```

### 2. Confirm

```bash
git check-ignore -v contracts/broadcast/Deploy.s.sol/1/run-latest.json
# Expect: a non-empty line (the file is now ignored)
```

### 3. Don't `git rm` already-tracked artifacts

```bash
git ls-files contracts/broadcast/
```

If the broadcast artifacts are **already tracked** (committed previously), this batch does NOT delete them. That's a separate cleanup; the `.gitignore` update only stops new artifacts from being committed.

If the user wants to also remove tracked artifacts, that's a separate batch (`S-chain-3` or follow-up): `git rm --cached -r contracts/broadcast/**/run-*.json`.

## Write Scope
- `.gitignore`

## Verify

```bash
grep -E 'broadcast.*run-' .gitignore
# Expect: at least 1 hit

git check-ignore contracts/broadcast/Deploy.s.sol/1/run-latest.json
# Expect: a path printed
```

## Do NOT

- Do NOT `git rm --cached` artifacts in this batch.
- Do NOT add `contracts/` entirely to gitignore (the .sol sources stay).
- Do NOT bundle with S-chain-1.
- Do NOT add deploy.config.toml or chain.toml to gitignore (those are real config).
