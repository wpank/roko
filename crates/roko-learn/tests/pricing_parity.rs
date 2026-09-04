//! QW-7: Assert that shared model slugs between `PRICING_TABLE`
//! (cost_projection.rs, per-1K rates) and `CostTable::with_defaults()`
//! (cost_table.rs / model_registry.rs, per-1M rates) agree within 1%.
//!
//! The projection table is private, so we derive its effective per-1M rates
//! from `project_task_cost_with_output` with known token counts and compare
//! them against the canonical `BUILTIN_PRICING` rows surfaced by
//! `CostTable::with_defaults()`.

#![allow(missing_docs)]

use std::collections::HashMap;

use roko_learn::cost_projection::project_task_cost_with_output;
use roko_learn::cost_table::CostTable;

/// Fallback rates from cost_projection.rs (per-1K).  A slug that returns
/// these exact rates is NOT in the projection table.
const FALLBACK_INPUT_PER_K: f64 = 0.003;
const FALLBACK_OUTPUT_PER_K: f64 = 0.015;

/// Derive the effective per-1M input and output rates for `slug` by calling
/// `project_task_cost_with_output` with 1M input / 1M output tokens.
///
/// Returns `None` when the slug falls back to default Sonnet rates (meaning
/// it is not present in the projection table).
fn projection_rates_per_m(slug: &str) -> Option<(f64, f64)> {
    // Use a two-call approach to isolate input and output rates.
    let input_only = project_task_cost_with_output(1_000_000, 0, slug);
    let output_only = project_task_cost_with_output(0, 1_000_000, slug);

    let input_per_k = input_only.estimated_cost_usd / 1_000.0;
    let output_per_k = output_only.estimated_cost_usd / 1_000.0;

    // Check if both rates match the fallback exactly (not in the table).
    if (input_per_k - FALLBACK_INPUT_PER_K).abs() < 1e-12
        && (output_per_k - FALLBACK_OUTPUT_PER_K).abs() < 1e-12
    {
        return None;
    }

    // Convert per-1K to per-1M.
    Some((input_per_k * 1_000.0, output_per_k * 1_000.0))
}

#[test]
fn pricing_table_parity_within_one_percent() {
    let defaults = CostTable {
        models: HashMap::new(),
    }
    .with_defaults();

    let mut checked = 0u32;
    let mut divergences: Vec<String> = Vec::new();

    for (slug, cost_table_pricing) in &defaults.models {
        let Some((proj_input_per_m, proj_output_per_m)) = projection_rates_per_m(slug) else {
            // Slug is not in the projection table — skip.
            continue;
        };

        checked += 1;

        let input_ratio = if cost_table_pricing.input_per_m.abs() < 1e-12 {
            if proj_input_per_m.abs() < 1e-12 {
                0.0 // both zero
            } else {
                f64::INFINITY
            }
        } else {
            ((proj_input_per_m - cost_table_pricing.input_per_m) / cost_table_pricing.input_per_m)
                .abs()
        };

        let output_ratio = if cost_table_pricing.output_per_m.abs() < 1e-12 {
            if proj_output_per_m.abs() < 1e-12 {
                0.0 // both zero
            } else {
                f64::INFINITY
            }
        } else {
            ((proj_output_per_m - cost_table_pricing.output_per_m)
                / cost_table_pricing.output_per_m)
                .abs()
        };

        if input_ratio > 0.01 || output_ratio > 0.01 {
            divergences.push(format!(
                "  {slug}: projection input/M=${proj_input_per_m:.4} output/M=${proj_output_per_m:.4} \
                 vs cost_table input/M=${:.4} output/M=${:.4} (input delta {:.2}%, output delta {:.2}%)",
                cost_table_pricing.input_per_m,
                cost_table_pricing.output_per_m,
                input_ratio * 100.0,
                output_ratio * 100.0,
            ));
        }
    }

    assert!(
        checked > 0,
        "No shared slugs found between PRICING_TABLE and CostTable::with_defaults(). \
         This likely means the probe mechanism is broken."
    );

    assert!(
        divergences.is_empty(),
        "Pricing divergence (>1%) between PRICING_TABLE and CostTable::with_defaults() \
         for {n}/{checked} shared slugs:\n{divs}",
        n = divergences.len(),
        divs = divergences.join("\n"),
    );
}
