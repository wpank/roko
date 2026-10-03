//! Prompt composition strategy selection.
//!
//! Every strategy allocates the prompt budget the density-greedy way: `Auto`,
//! `WeightedSum` and `Vcg` resolve to `DensityGreedy`. The VCG auction and
//! the learning bidders it would have warmed up on are retired (backlog
//! 4218): no run ever registered a bidder, so `Auto` never left
//! density-greedy. The variants stay so that configs naming them still
//! parse.

use serde::{Deserialize, Serialize};

/// Strategy for allocating prompt token budget across candidate sections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionStrategy {
    /// The default: density-greedy allocation.
    #[default]
    Auto,
    /// Deterministic greedy allocation by score density.
    DensityGreedy,
    /// Backward-compatible alias for density-greedy allocation.
    WeightedSum,
    /// The retired VCG allocation, which now runs density-greedy.
    Vcg,
}

impl CompositionStrategy {
    /// The strategy the composer runs: density-greedy, whatever was asked.
    #[must_use]
    pub const fn resolve(self) -> Self {
        Self::DensityGreedy
    }

    /// Whether this strategy names the density-greedy path itself.
    #[must_use]
    pub const fn is_density_greedy(self) -> bool {
        matches!(self, Self::DensityGreedy | Self::WeightedSum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 4218: every strategy, `Vcg` included, resolves to density-greedy.
    #[test]
    fn every_strategy_resolves_to_density_greedy() {
        for strategy in [
            CompositionStrategy::Auto,
            CompositionStrategy::DensityGreedy,
            CompositionStrategy::WeightedSum,
            CompositionStrategy::Vcg,
        ] {
            assert_eq!(
                strategy.resolve(),
                CompositionStrategy::DensityGreedy,
                "{strategy:?}"
            );
        }
    }

    #[test]
    fn is_density_greedy_covers_both_aliases() {
        assert!(CompositionStrategy::DensityGreedy.is_density_greedy());
        assert!(CompositionStrategy::WeightedSum.is_density_greedy());
        assert!(!CompositionStrategy::Vcg.is_density_greedy());
        assert!(!CompositionStrategy::Auto.is_density_greedy());
    }
}
