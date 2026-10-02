//! The move catalog and the requisite-variety matrix (S06 §4.3, §4.9).
//!
//! [`CATALOG`] lists every single-notch move M1's search may make, each
//! with its class and its main effects on the essential variables. A lower
//! verification floor (B3) or a lower audit boost (B7) is not a search
//! move, so the catalog has neither: only rollback and relaxation undo
//! them, through the `SafetyBox`.
//!
//! [`candidates`] directs a breach to the moves that can counter it, from
//! the breached EVs and the auxiliary signals alone: the controller never
//! sees which disturbance is running (8119 keeps that hidden).
//! [`REQUISITE_VARIETY`] is S06's matrix of disturbances against their
//! counter-moves, kept as data for tests, the replay's scoring and the
//! operator's display. It also says up front which disturbances M1 cannot
//! regulate (`flaky_verify`, every provider slowed), so the controller
//! holds instead of thrashing.

use roko_core::config::harness_params::{Block, HarnessParams, Knob, KnobKind, Step};
use serde::{Deserialize, Serialize};

use super::ev::Ev;

/// A move's class (S06 §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveClass {
    /// Free inside the box.
    Free,
    /// Lowers cost: the audit rate on the task class doubles after it
    /// (§4.6.5, 8127).
    CostReducing,
    /// Adds checks or audits; M1 withdraws only what it added.
    AddOnly,
    /// Only ever lowers its knob (B8's task budget share), and is
    /// audit-coupled like a cost-reducing move.
    DecreaseOnly,
}

/// A move's main effect on one EV, as S06 §4.3 lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// No main effect.
    None,
    /// Raises the EV.
    Raises,
    /// Lowers the EV.
    Lowers,
    /// Either way, depending on the task mix.
    Varies,
}

/// One single-notch move on one knob family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Move {
    /// The knob family.
    pub knob: KnobKind,
    /// The notch it moves to.
    pub direction: Step,
    /// Its class.
    pub class: MoveClass,
    /// Its main effects on E1, E2, E3 and E4, in [`Ev::ALL`] order.
    pub effects: [Effect; 4],
}

impl Move {
    /// The block the move changes.
    #[must_use]
    pub const fn block(&self) -> Block {
        self.knob.block()
    }

    /// The move's main effect on `ev`.
    #[must_use]
    pub const fn effect(&self, ev: Ev) -> Effect {
        let index = match ev {
            Ev::PassRate => 0,
            Ev::UsdPerVerifiedSuccess => 1,
            Ev::FalseGreen => 2,
            Ev::LatencyP90S => 3,
        };
        self.effects[index]
    }

    /// Whether the audit rate on the task class doubles after the move:
    /// cap↓, floor↓, retry↓, turn↓, promise↑, budget↓ or a section switched
    /// off (§4.6.5).
    #[must_use]
    pub const fn audit_coupled(&self) -> bool {
        matches!(self.class, MoveClass::CostReducing | MoveClass::DecreaseOnly)
    }

    /// Whether the move lowers verification: a lower B3 floor or B7 boost.
    /// No catalog move does.
    #[must_use]
    pub fn lowers_verification(&self) -> bool {
        matches!(self.knob, KnobKind::ExtraRungs | KnobKind::AuditBoost)
            && self.direction == Step::Down
    }

    /// The knobs of `theta` the move may turn: one per tier for the B1
    /// rungs, the first-choice provider for `provider_order` (an outage
    /// demotes it one place), and the one knob otherwise.
    #[must_use]
    pub fn knobs(&self, theta: &HarnessParams) -> Vec<Knob> {
        match self.knob {
            KnobKind::TierFloor => theta
                .tier_floor
                .keys()
                .map(|&tier| Knob::TierFloor(tier))
                .collect(),
            KnobKind::TierCap => theta
                .tier_cap
                .keys()
                .map(|&tier| Knob::TierCap(tier))
                .collect(),
            KnobKind::ProviderOrder => theta
                .provider_order
                .first()
                .map(|&first| vec![Knob::ProviderRank(first)])
                .unwrap_or_default(),
            kind => Knob::SCALARS
                .into_iter()
                .filter(|knob| knob.kind() == kind)
                .collect(),
        }
    }
}

const fn entry(
    knob: KnobKind,
    direction: Step,
    class: MoveClass,
    effects: [Effect; 4],
) -> Move {
    Move {
        knob,
        direction,
        class,
        effects,
    }
}

const NO: Effect = Effect::None;
const UP: Effect = Effect::Raises;
const DOWN: Effect = Effect::Lowers;
const EITHER: Effect = Effect::Varies;

/// Every move M1's search may make (S06 §4.3). Effects are on
/// [E1 pass rate, E2 $/verified success, E3 false green, E4 p90 latency].
pub const CATALOG: [Move; 20] = [
    entry(KnobKind::TierFloor, Step::Up, MoveClass::Free, [UP, UP, DOWN, NO]),
    entry(KnobKind::TierFloor, Step::Down, MoveClass::CostReducing, [DOWN, DOWN, UP, NO]),
    entry(KnobKind::TierCap, Step::Up, MoveClass::Free, [UP, UP, NO, NO]),
    entry(KnobKind::TierCap, Step::Down, MoveClass::CostReducing, [DOWN, DOWN, NO, NO]),
    entry(KnobKind::ProviderOrder, Step::Up, MoveClass::Free, [UP, NO, NO, DOWN]),
    entry(KnobKind::RetryDelta, Step::Up, MoveClass::Free, [UP, UP, NO, UP]),
    entry(KnobKind::RetryDelta, Step::Down, MoveClass::CostReducing, [DOWN, DOWN, NO, DOWN]),
    entry(KnobKind::TurnCapMult, Step::Up, MoveClass::Free, [UP, UP, NO, UP]),
    entry(KnobKind::TurnCapMult, Step::Down, MoveClass::CostReducing, [DOWN, DOWN, NO, DOWN]),
    entry(KnobKind::ExtraRungs, Step::Up, MoveClass::AddOnly, [NO, UP, DOWN, UP]),
    entry(KnobKind::ErrorPatternsK, Step::Up, MoveClass::Free, [EITHER, UP, NO, NO]),
    entry(KnobKind::ErrorPatternsK, Step::Down, MoveClass::Free, [EITHER, DOWN, NO, NO]),
    entry(KnobKind::KnowledgeSection, Step::Up, MoveClass::Free, [EITHER, UP, NO, NO]),
    entry(KnobKind::KnowledgeSection, Step::Down, MoveClass::CostReducing, [EITHER, DOWN, NO, NO]),
    entry(KnobKind::MaxParallel, Step::Up, MoveClass::Free, [NO, NO, NO, EITHER]),
    entry(KnobKind::MaxParallel, Step::Down, MoveClass::Free, [NO, NO, NO, EITHER]),
    entry(KnobKind::PromiseMin, Step::Up, MoveClass::CostReducing, [NO, DOWN, NO, DOWN]),
    entry(KnobKind::PromiseConsecutive, Step::Up, MoveClass::CostReducing, [NO, DOWN, NO, DOWN]),
    entry(KnobKind::AuditBoost, Step::Up, MoveClass::AddOnly, [NO, UP, DOWN, NO]),
    entry(KnobKind::TaskBudgetScale, Step::Up, MoveClass::DecreaseOnly, [DOWN, DOWN, NO, NO]),
];

/// The catalog's move on `knob` toward `direction`, if the search may make
/// it.
#[must_use]
pub fn catalog_move(knob: KnobKind, direction: Step) -> Option<Move> {
    CATALOG
        .into_iter()
        .find(|entry| entry.knob == knob && entry.direction == direction)
}

/// What the controller knows when it picks a move: which EVs breached, and
/// the auxiliary signals that only rank moves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Signature {
    /// E1 is below its bound.
    pub pass_rate_low: bool,
    /// E2 is above its bound.
    pub cost_high: bool,
    /// E3 is above its bound.
    pub false_green_high: bool,
    /// E4 is above its bound.
    pub latency_high: bool,
    /// Auxiliary: infrastructure-blamed attempts are up.
    pub provider_errors: bool,
    /// Auxiliary: attempts are ending on the task budget.
    pub budget_exhausted: bool,
    /// Auxiliary: attempts per resolution are up.
    pub retries_up: bool,
}

impl Signature {
    /// The signature of the `breached` EVs, with no auxiliary signal.
    #[must_use]
    pub fn of(breached: &[Ev]) -> Self {
        Self {
            pass_rate_low: breached.contains(&Ev::PassRate),
            cost_high: breached.contains(&Ev::UsdPerVerifiedSuccess),
            false_green_high: breached.contains(&Ev::FalseGreen),
            latency_high: breached.contains(&Ev::LatencyP90S),
            ..Self::default()
        }
    }
}

/// The moves directed at `signature`, most fitting first, each one notch of
/// one knob in one block. None lowers verification.
///
/// - Provider errors with E4 or E1 breached: reroute (demote the
///   first-choice provider, fewer slots).
/// - E4 alone: fewer slots, a lower turn cap, an earlier abandon.
/// - E2 with no capability gap (or with attempts ending on the budget):
///   cut cost.
/// - E1 otherwise: strengthen (floor↑, retries↑, verify floor↑, fresher
///   patterns, the knowledge section, more turns).
/// - E3: deepen verification and audits, and a stronger floor.
#[must_use]
pub fn candidates(signature: &Signature) -> Vec<Move> {
    let mut picks: Vec<(KnobKind, Step)> = Vec::new();
    let outage =
        signature.provider_errors && (signature.latency_high || signature.pass_rate_low);
    if outage {
        picks.extend([
            (KnobKind::ProviderOrder, Step::Up),
            (KnobKind::MaxParallel, Step::Down),
        ]);
    } else if signature.latency_high {
        picks.extend([
            (KnobKind::MaxParallel, Step::Down),
            (KnobKind::TurnCapMult, Step::Down),
            (KnobKind::PromiseMin, Step::Up),
        ]);
    }
    let cost_pressure =
        signature.cost_high && (signature.budget_exhausted || !signature.pass_rate_low);
    if cost_pressure {
        picks.extend([
            (KnobKind::TierCap, Step::Down),
            (KnobKind::RetryDelta, Step::Down),
            (KnobKind::TurnCapMult, Step::Down),
            (KnobKind::PromiseMin, Step::Up),
            (KnobKind::PromiseConsecutive, Step::Up),
            (KnobKind::KnowledgeSection, Step::Down),
            (KnobKind::TaskBudgetScale, Step::Up),
            (KnobKind::ProviderOrder, Step::Up),
        ]);
    } else if signature.pass_rate_low && !outage {
        picks.extend([
            (KnobKind::TierFloor, Step::Up),
            (KnobKind::RetryDelta, Step::Up),
            (KnobKind::ExtraRungs, Step::Up),
            (KnobKind::ErrorPatternsK, Step::Up),
            (KnobKind::KnowledgeSection, Step::Up),
            (KnobKind::TurnCapMult, Step::Up),
        ]);
    }
    if signature.false_green_high {
        picks.extend([
            (KnobKind::ExtraRungs, Step::Up),
            (KnobKind::AuditBoost, Step::Up),
            (KnobKind::TierFloor, Step::Up),
        ]);
    }
    let mut moves: Vec<Move> = Vec::new();
    for (knob, direction) in picks {
        if let Some(found) = catalog_move(knob, direction)
            && !moves.contains(&found)
        {
            moves.push(found);
        }
    }
    moves
}

// ── The requisite-variety matrix ──────────────────────────────────────

/// A disturbance S06 names (§4.9). The first six are `KINDS` in
/// `benchmarks/viabilitybench/driver/disturb.py`; `price_shock` is
/// optional and outside H6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisturbanceKind {
    /// An HTTP fault on one provider (or, as the unregulable case, every
    /// provider slowed).
    ProviderFault,
    /// A tier's model replaced by a weaker one.
    ModelSwap,
    /// The task sampler switches to the hard pool.
    HarderMix,
    /// The per-task budget ceiling halved.
    BudgetCut,
    /// The repository's conventions change under the tasks.
    ConventionFlip,
    /// The visible verify step fails at random.
    FlakyVerify,
    /// One model's prices multiplied.
    PriceShock,
}

impl DisturbanceKind {
    /// `disturb.py`'s `KINDS`, in its order.
    pub const DISTURB_PY: [Self; 6] = [
        Self::ProviderFault,
        Self::ModelSwap,
        Self::HarderMix,
        Self::BudgetCut,
        Self::ConventionFlip,
        Self::FlakyVerify,
    ];

    /// The regulable kinds of H6's primary endpoint (C4).
    pub const REGULABLE: [Self; 5] = [
        Self::ProviderFault,
        Self::ModelSwap,
        Self::HarderMix,
        Self::BudgetCut,
        Self::ConventionFlip,
    ];

    /// The kind's name, as `disturb.py` and the records write it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ProviderFault => "provider_fault",
            Self::ModelSwap => "model_swap",
            Self::HarderMix => "harder_mix",
            Self::BudgetCut => "budget_cut",
            Self::ConventionFlip => "convention_flip",
            Self::FlakyVerify => "flaky_verify",
            Self::PriceShock => "price_shock",
        }
    }
}

/// What M1 should end with under a disturbance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expected {
    /// A counter-move exists in the box: regain viability.
    Regulate,
    /// Nothing in the box fixes it: hold and alert within N_max changes,
    /// never by lowering verification (C5).
    Hold,
}

/// One row of the requisite-variety matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RequisiteRow {
    /// The disturbance.
    pub kind: DisturbanceKind,
    /// Every provider slowed: `provider_fault`'s unregulable form.
    pub all_providers: bool,
    /// The breach signature it produces (S06 §4.9, "Expected effect").
    pub signature: Signature,
    /// The moves that can counter it.
    pub counters: &'static [(KnobKind, Step)],
    /// What M1 should end with.
    pub expected: Expected,
}

impl RequisiteRow {
    /// Whether `candidate` is one of the row's counter-moves.
    #[must_use]
    pub fn counters_with(&self, candidate: &Move) -> bool {
        self.counters
            .contains(&(candidate.knob, candidate.direction))
    }
}

const fn signature(
    pass_rate_low: bool,
    cost_high: bool,
    latency_high: bool,
    provider_errors: bool,
) -> Signature {
    Signature {
        pass_rate_low,
        cost_high,
        false_green_high: false,
        latency_high,
        provider_errors,
        budget_exhausted: false,
        retries_up: false,
    }
}

/// S06's requisite-variety matrix (§4.9).
pub const REQUISITE_VARIETY: [RequisiteRow; 8] = [
    RequisiteRow {
        kind: DisturbanceKind::ProviderFault,
        all_providers: false,
        signature: signature(true, false, true, true),
        counters: &[
            (KnobKind::ProviderOrder, Step::Up),
            (KnobKind::MaxParallel, Step::Down),
        ],
        expected: Expected::Regulate,
    },
    RequisiteRow {
        kind: DisturbanceKind::ProviderFault,
        all_providers: true,
        signature: signature(false, false, true, false),
        counters: &[],
        expected: Expected::Hold,
    },
    RequisiteRow {
        kind: DisturbanceKind::ModelSwap,
        all_providers: false,
        signature: signature(true, false, false, false),
        counters: &[
            (KnobKind::TierFloor, Step::Up),
            (KnobKind::RetryDelta, Step::Up),
            (KnobKind::TurnCapMult, Step::Up),
            (KnobKind::ExtraRungs, Step::Up),
            (KnobKind::ErrorPatternsK, Step::Up),
            (KnobKind::KnowledgeSection, Step::Up),
        ],
        expected: Expected::Regulate,
    },
    RequisiteRow {
        kind: DisturbanceKind::ConventionFlip,
        all_providers: false,
        signature: signature(true, false, false, false),
        counters: &[
            (KnobKind::ErrorPatternsK, Step::Up),
            (KnobKind::KnowledgeSection, Step::Up),
            (KnobKind::ExtraRungs, Step::Up),
            (KnobKind::RetryDelta, Step::Up),
            (KnobKind::TurnCapMult, Step::Up),
        ],
        expected: Expected::Regulate,
    },
    RequisiteRow {
        kind: DisturbanceKind::BudgetCut,
        all_providers: false,
        signature: Signature {
            budget_exhausted: true,
            ..signature(true, true, false, false)
        },
        counters: &[
            (KnobKind::TierCap, Step::Down),
            (KnobKind::RetryDelta, Step::Down),
            (KnobKind::TurnCapMult, Step::Down),
            (KnobKind::PromiseMin, Step::Up),
            (KnobKind::PromiseConsecutive, Step::Up),
            (KnobKind::KnowledgeSection, Step::Down),
            (KnobKind::TaskBudgetScale, Step::Up),
        ],
        expected: Expected::Regulate,
    },
    RequisiteRow {
        kind: DisturbanceKind::HarderMix,
        all_providers: false,
        signature: signature(true, true, false, false),
        counters: &[
            (KnobKind::TierFloor, Step::Up),
            (KnobKind::RetryDelta, Step::Up),
            (KnobKind::TurnCapMult, Step::Up),
            (KnobKind::ExtraRungs, Step::Up),
        ],
        expected: Expected::Regulate,
    },
    RequisiteRow {
        kind: DisturbanceKind::FlakyVerify,
        all_providers: false,
        signature: Signature {
            retries_up: true,
            ..signature(true, false, false, false)
        },
        counters: &[(KnobKind::RetryDelta, Step::Up)],
        expected: Expected::Hold,
    },
    RequisiteRow {
        kind: DisturbanceKind::PriceShock,
        all_providers: false,
        signature: signature(false, true, false, false),
        counters: &[
            (KnobKind::TierCap, Step::Down),
            (KnobKind::ProviderOrder, Step::Up),
        ],
        expected: Expected::Regulate,
    },
];

#[cfg(test)]
mod tests {
    use roko_core::config::RokoConfig;
    use roko_core::config::harness_params::HarnessLadders;
    use roko_core::task::TaskTier;

    use super::*;

    #[test]
    fn first_move_lies_in_requisite_variety_row() {
        for row in &REQUISITE_VARIETY {
            let moves = candidates(&row.signature);
            // Every candidate is one catalog move, and none lowers
            // verification.
            for candidate in &moves {
                assert_eq!(
                    catalog_move(candidate.knob, candidate.direction),
                    Some(*candidate)
                );
                assert!(!candidate.lowers_verification(), "{candidate:?}");
            }
            let shared = REQUISITE_VARIETY
                .iter()
                .filter(|other| other.signature == row.signature)
                .count();
            match row.expected {
                Expected::Regulate => {
                    // The directed candidates meet the row; with a
                    // signature no other disturbance shares, the first one
                    // already lies in it. Model swap and convention flip
                    // both show only E1↓, so the controller's priors, not
                    // the order, choose between their moves.
                    assert!(
                        moves.iter().any(|candidate| row.counters_with(candidate)),
                        "{:?}: {moves:?}",
                        row.kind
                    );
                    if shared == 1 {
                        let first = moves.first().expect("a directed move");
                        assert!(row.counters_with(first), "{:?}: {first:?}", row.kind);
                    }
                }
                Expected::Hold => {
                    // Nothing regulates it: at most spending (B2) can absorb
                    // it, never a lower bar.
                    assert!(
                        row.counters
                            .iter()
                            .all(|&(knob, direction)| knob.block() == Block::B2
                                && direction == Step::Up),
                        "{:?}",
                        row.kind
                    );
                }
            }
        }

        // The model-swap case of S06 A1: E1↓ with E4 flat directs B1
        // floor↑ first, then B2↑ and B3↑.
        let first_three: Vec<(KnobKind, Step)> = candidates(&Signature::of(&[Ev::PassRate]))
            .iter()
            .take(3)
            .map(|candidate| (candidate.knob, candidate.direction))
            .collect();
        assert_eq!(
            first_three,
            [
                (KnobKind::TierFloor, Step::Up),
                (KnobKind::RetryDelta, Step::Up),
                (KnobKind::ExtraRungs, Step::Up)
            ]
        );

        // Every regulable kind of H6 has a row that regulates it, and the
        // canonical names are disturb.py's.
        for kind in DisturbanceKind::REGULABLE {
            assert!(REQUISITE_VARIETY.iter().any(|row| row.kind == kind
                && !row.all_providers
                && row.expected == Expected::Regulate));
        }
        let names: Vec<&str> = DisturbanceKind::DISTURB_PY
            .iter()
            .map(|kind| kind.name())
            .collect();
        assert_eq!(
            names,
            [
                "provider_fault",
                "model_swap",
                "harder_mix",
                "budget_cut",
                "convention_flip",
                "flaky_verify"
            ]
        );
    }

    #[test]
    fn catalog_classes_match_s06() {
        // No catalog move lowers verification.
        assert!(CATALOG.iter().all(|entry| !entry.lowers_verification()));
        // The audit-coupled moves are §4.6.5's: cap↓, floor↓, retry↓,
        // turn↓, promise↑, budget↓ and a section switched off; each lowers
        // E2.
        let coupled: Vec<(KnobKind, Step)> = CATALOG
            .iter()
            .filter(|entry| entry.audit_coupled())
            .map(|entry| (entry.knob, entry.direction))
            .collect();
        assert_eq!(
            coupled,
            [
                (KnobKind::TierFloor, Step::Down),
                (KnobKind::TierCap, Step::Down),
                (KnobKind::RetryDelta, Step::Down),
                (KnobKind::TurnCapMult, Step::Down),
                (KnobKind::KnowledgeSection, Step::Down),
                (KnobKind::PromiseMin, Step::Up),
                (KnobKind::PromiseConsecutive, Step::Up),
                (KnobKind::TaskBudgetScale, Step::Up)
            ]
        );
        assert!(
            CATALOG
                .iter()
                .filter(|entry| entry.audit_coupled())
                .all(|entry| entry.effect(Ev::UsdPerVerifiedSuccess) == Effect::Lowers)
        );
        // B3 and B7 only add.
        for knob in [KnobKind::ExtraRungs, KnobKind::AuditBoost] {
            assert!(catalog_move(knob, Step::Down).is_none());
            assert_eq!(
                catalog_move(knob, Step::Up).map(|entry| entry.class),
                Some(MoveClass::AddOnly)
            );
        }

        // A move names the knobs of θ it may turn.
        let mut config = RokoConfig::default();
        config.providers.insert(
            "alpha".to_string(),
            roko_core::config::ProviderConfig::default(),
        );
        let theta0 = HarnessParams::baseline(&config);
        let floor = catalog_move(KnobKind::TierFloor, Step::Up).expect("floor up");
        assert_eq!(floor.knobs(&theta0).len(), TaskTier::ALL.len());
        let reroute = catalog_move(KnobKind::ProviderOrder, Step::Up).expect("reroute");
        assert_eq!(reroute.knobs(&theta0), [Knob::ProviderRank(0)]);
        let retry = catalog_move(KnobKind::RetryDelta, Step::Up).expect("retry up");
        assert_eq!(retry.knobs(&theta0), [Knob::RetryDelta]);
        let ladders = HarnessLadders::from_config(&config);
        for entry in CATALOG {
            for knob in entry.knobs(&theta0) {
                assert_eq!(knob.block(), entry.block());
                // From θ₀ a move either takes one notch or meets its ladder's end.
                let _ = theta0.step(knob, entry.direction, &ladders);
            }
        }
    }
}
