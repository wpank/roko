//! impact command handler.

use crate::*;

pub(crate) async fn cmd_impact(
    cli: &Cli,
    base: &str,
    files: &[String],
    json: bool,
    workdir: Option<PathBuf>,
) -> Result<i32> {
    use roko_cli::runner::impact_analysis;
    use roko_core::config::GatesConfig;

    let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let config = GatesConfig::default();
    let report = impact_analysis::analyze_against(&wd, base, files, &config).await;

    if json {
        let affected_packages = {
            let mut all: std::collections::BTreeSet<String> =
                report.producer_packages.iter().cloned().collect();
            all.extend(report.reverse_dependents.iter().cloned());
            all.into_iter().collect::<Vec<_>>()
        };
        let targets: Vec<_> = report.targets.iter().map(|t| t.check_command()).collect();
        let output = serde_json::json!({
            "changed_files": report.changed_files,
            "producer_packages": report.producer_packages,
            "reverse_dependents": report.reverse_dependents,
            "affected_packages": affected_packages,
            "high_impact": report.high_impact,
            "high_impact_reasons": report.high_impact_reasons,
            "confidence": report.confidence.to_string(),
            "analysis_ms": report.analysis_ms,
            "fallback_reason": report.fallback_reason,
            "targets": targets,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else {
        if let Some(reason) = &report.fallback_reason {
            println!("Fallback to full verification: {reason}");
            println!();
        }

        if report.changed_files.is_empty() {
            println!("No changes detected.");
            return Ok(EXIT_SUCCESS);
        }

        println!("Changed files ({}):", report.changed_files.len());
        for file in &report.changed_files {
            println!("  {file}");
        }
        println!();

        if !report.producer_packages.is_empty() {
            println!("Producer crates ({}):", report.producer_packages.len());
            for pkg in &report.producer_packages {
                println!("  {pkg}");
            }
            println!();
        }

        if !report.reverse_dependents.is_empty() {
            println!("Reverse dependents ({}):", report.reverse_dependents.len());
            for pkg in &report.reverse_dependents {
                println!("  {pkg}");
            }
            println!();
        }

        // Combined affected set for cargo test -p
        let mut affected: std::collections::BTreeSet<String> =
            report.producer_packages.iter().cloned().collect();
        affected.extend(report.reverse_dependents.iter().cloned());
        if !affected.is_empty() {
            println!("All affected crates ({}):", affected.len());
            for pkg in &affected {
                println!("  {pkg}");
            }
            println!();
            // Print a ready-to-use cargo test command
            let pkg_args: Vec<String> = affected
                .iter()
                .flat_map(|p| vec!["-p".to_string(), p.clone()])
                .collect();
            println!("cargo test {}", pkg_args.join(" "));
        }

        if report.high_impact {
            println!();
            println!("HIGH IMPACT:");
            for reason in &report.high_impact_reasons {
                println!("  - {reason}");
            }
        }

        if !report.targets.is_empty() {
            println!();
            println!("Focused check commands ({}):", report.targets.len());
            for cmd in report.focused_commands() {
                println!("  {cmd}");
            }
        }

        println!();
        println!(
            "Confidence: {} | Analysis: {}ms",
            report.confidence, report.analysis_ms
        );
    }

    Ok(EXIT_SUCCESS)
}
