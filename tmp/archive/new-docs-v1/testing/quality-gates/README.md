# Quality Gates

> Four quality gates define what must pass at each stage of the development lifecycle: pre-commit, PR merge, pre-release, and post-deploy.

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | Gate | Blocking? |
|---|---|---|---|
| 01 | [pre-commit.md](01-pre-commit.md) | Pre-commit | Yes (developer) |
| 02 | [pr-checks.md](02-pr-checks.md) | PR merge | Yes (CI enforced) |
| 03 | [pre-release.md](03-pre-release.md) | Release tag | Yes (CI enforced) |
| 04 | [post-deploy.md](04-post-deploy.md) | Post-deploy observability | Alert (not blocking) |

---

## Quality Gate Summary

| Gate | Tests run | Coverage check | Performance check |
|---|---|---|---|
| Pre-commit | unit + property (fast) | No | No |
| PR merge | unit + property (full) + integration + regression | Yes (80% floor) | No |
| Pre-release | All of above + E2E | Yes (80% floor) | Yes |
| Post-deploy | Smoke tests + observability | No | No |

---

## See also

- [../tools-and-harness/04-ci-integration.md](../tools-and-harness/04-ci-integration.md) — CI pipeline that runs these gates
- [../tiers/README.md](../tiers/README.md) — test tiers used in each gate
