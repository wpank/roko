# S-config-7: roko config doctor — provenance + validation warnings

## Task
Extend `roko config doctor` (existing skeleton from C9) to display:
- Per-field provenance (where each setting came from).
- Validation warnings (results from S-config-1 validators).
- Local overrides table (reason / scope / expiry / source / ack-env state).

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-config-2 + S-config-6. Wave 3.

## Source plan
`tmp/subsystem-audits/implementation-plans/23-config-validation-pipeline.md` § Phase 5.

## Read first

```bash
rg 'fn config_doctor|/api/config/doctor|roko config doctor' crates/roko-cli/src/ -n
```

Locate the existing `doctor` command (likely in `commands/config_cmd.rs` or `config_cmd.rs`).

## Exact changes

### 1. Render provenance table

```rust
async fn cmd_config_doctor(workdir: &Path) -> anyhow::Result<()> {
    let validated = roko_core::config::load_config(workdir)?;
    let cfg = validated.config();

    println!("Config Provenance");
    println!("=================");
    let mut rows: Vec<(String, String)> = Vec::new();
    for (path, prov) in validated.provenance().iter() {
        let label = match &prov.source {
            ConfigSource::Default => "default".into(),
            ConfigSource::SharedFile(p) => format!("shared: {}", p.display()),
            ConfigSource::LocalFile(p) => format!("local:  {}", p.display()),
            ConfigSource::EnvVar(v) => format!("env:    ${v}"),
            ConfigSource::CliFlag(f) => format!("cli:    --{f}"),
        };
        rows.push((path.clone(), label));
    }
    rows.sort();
    for (path, source) in rows {
        println!("  {:<48} {}", path, source);
    }

    println!("\nValidation");
    println!("==========");
    // S-config-1 validators were already run by load_config; if any failed,
    // load_config would have erred. Run them again as warnings (none should
    // fail at this point).
    print_validation_status(cfg);

    println!("\nLocal Overrides");
    println!("===============");
    if validated.local_overrides().is_empty() {
        println!("  (none)");
    } else {
        for (i, o) in validated.local_overrides().iter().enumerate() {
            let ack_state = if std::env::var(&o.acknowledgement_env).is_ok() { "set" } else { "MISSING" };
            let expiry = o.expiry.map_or("never".into(), |e| e.to_rfc3339());
            println!(
                "  [{i}] reason={:?}  scope={:?}  expiry={expiry}  ack_env={}={}",
                o.reason, o.scope, o.acknowledgement_env, ack_state,
            );
        }
    }
    Ok(())
}
```

### 2. (Optional) HTTP version

`crates/roko-serve/src/routes/diagnosis.rs` (or wherever the doctor route lives): expose `GET /api/config/doctor` returning the same data as JSON. The route uses `state.validated_config()` from S-config-4.

### 3. Tests

Smoke test that runs `roko config doctor` and asserts non-zero output.

## Write Scope
- `crates/roko-cli/src/commands/config_cmd.rs` or `crates/roko-cli/src/config_cmd.rs`
- `crates/roko-serve/src/routes/diagnosis.rs` (optional, only if extending HTTP)

## Verify

```bash
rg 'Config Provenance|Local Overrides' crates/roko-cli/src/
# Expect: at least 2 hits

# Manual smoke
cargo run -p roko-cli -- config doctor 2>&1 | head -30
# Expect: rendered table with provenance + validation
```

## Do NOT

- Do NOT mutate the config from `doctor`. It's read-only.
- Do NOT print secrets (use `mask_secret_fields` semantics).
- Do NOT bundle with S-config-1..6.
- Do NOT include validation errors as warnings — at this point they would have failed `load_config`. Status is informational ("OK" / etc.).
