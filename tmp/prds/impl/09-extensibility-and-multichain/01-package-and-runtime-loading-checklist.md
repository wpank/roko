# Package System And Runtime Loading Checklist

## Scope

Use this file for package manifests, lockfiles, installation/resolution, QuickJS or Pi compatibility, and composed profile loading.

## Implementation checklist

- [ ] Define the package manifest before writing installer code.
  - package id/version;
  - source type;
  - files/assets;
  - permissions/capabilities;
  - runtime compatibility.
- [ ] Add lockfile and storage layout semantics next.
  - deterministic install dir;
  - integrity hash;
  - conflict resolution.
- [ ] Reuse or extend current plugin surfaces where possible.
  - do not introduce a second incompatible native extension vocabulary.
- [ ] Only add QuickJS/Pi compatibility when there is one concrete package to load through it.
- [ ] Compose multi-domain profiles through the existing config/profile path.
  - parse combined profiles;
  - deduplicate extensions;
  - resolve conflicts deterministically.

## Relevant file touchpoints

- `crates/roko-plugin/`
- `crates/roko-cli/src/config.rs`
- `crates/roko-cli/src/main.rs`
- `docs/18-tools/14-plugin-sdk.md`
- `docs/19-deployment/08-subscription-configuration.md`

## Verification checklist

- [ ] Install/uninstall leaves a deterministic filesystem state.
- [ ] A composed profile can be loaded and resolved in tests.
- [ ] Version/integrity mismatches fail with actionable errors.

## Acceptance criteria

- Package installation is manifest-driven and reproducible.
- Composed profiles reuse existing runtime profile semantics.
- Any JS/QuickJS bridge exists to serve a real package-loading need.
