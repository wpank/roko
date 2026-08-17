//! Local DeFi product primitives and deterministic lifecycle registries.
//!
//! These registries model product behavior for simulation and testing. They do
//! not submit transactions or imply that the Phase 2 HTTP surface is live.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::phase2::u256;

const RATE_SCALE: u256 = 1_000_000_000;
const COLLATERAL_RATIO_BPS: u256 = 10_000;
const BPS_SCALE: u256 = 10_000;
const BLOCKS_PER_YEAR: f64 = 2_628_000.0;
const INDEX_WEIGHT_EPSILON: f64 = 0.01;
const MAX_VALUATION_HISTORY: usize = 1_000;

/// Supported knowledge-economy financial products.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstrumentKind {
    /// Collateralized reputation bond.
    Bond,
    /// Option whose underlying is a reputation score.
    ReputationOption,
    /// Contract for future delivery of knowledge.
    KnowledgeFuture,
    /// Policy covering a named operational event.
    InsurancePolicy,
    /// Weighted index over agent-economy observations.
    SyntheticIndex,
}

impl InstrumentKind {
    /// Stable prefix used when projecting an instrument into a market feed.
    #[must_use]
    pub const fn to_market_prefix(self) -> &'static str {
        match self {
            Self::Bond => "bond",
            Self::ReputationOption => "option",
            Self::KnowledgeFuture => "future",
            Self::InsurancePolicy => "insurance",
            Self::SyntheticIndex => "index",
        }
    }
}

/// Lifecycle state shared by all product instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstrumentState {
    /// Issued and usable.
    Active,
    /// Reached its contractual expiry and awaits settlement.
    Matured,
    /// Failed its repayment or delivery obligation.
    Defaulted,
    /// Completed and collateral was released or paid.
    Settled,
    /// Cancelled before completion.
    Cancelled,
}

/// A DeFi instrument and its immutable issuance terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instrument {
    /// Deterministic instrument identifier.
    pub id: [u8; 32],
    /// Product family.
    pub kind: InstrumentKind,
    /// Issuing passport or agent identifier.
    pub issuer_id: u256,
    /// Currently locked USDC collateral in smallest units.
    pub collateral_usdc: u256,
    /// Block at issuance.
    pub created_at_block: u64,
    /// Contractual expiry block.
    pub expires_at_block: u64,
    /// Current lifecycle state.
    pub state: InstrumentState,
    /// Product-specific issuance terms.
    pub parameters: InstrumentParams,
}

/// Product-specific terms carried by an [`Instrument`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "parameters", rename_all = "snake_case")]
pub enum InstrumentParams {
    /// Reputation bond terms.
    Bond(BondParams),
    /// Reputation option terms.
    ReputationOption(OptionParams),
    /// Knowledge future terms.
    KnowledgeFuture(FutureParams),
    /// Insurance policy terms.
    InsurancePolicy(InsuranceParams),
    /// Synthetic index terms.
    SyntheticIndex(IndexParams),
}

/// Reputation bond issuance terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BondParams {
    /// Principal due at maturity in smallest USDC units.
    pub face_value: u256,
    /// Decimal coupon paid on each scheduled payment.
    pub coupon_rate: f64,
    /// Blocks between coupon payments.
    pub payment_frequency_blocks: u64,
}

/// Reputation option terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionParams {
    /// Reputation domain used as the underlying.
    pub underlying_domain: String,
    /// Exercise score.
    pub strike_score: f64,
    /// Premium in smallest USDC units.
    pub premium: u256,
    /// Whether the option is a call rather than a put.
    pub is_call: bool,
}

/// Knowledge future delivery terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FutureParams {
    /// Human-readable knowledge deliverable.
    pub knowledge_spec: String,
    /// Optional target hyperdimensional vector.
    pub target_hdc: Option<Vec<f64>>,
    /// Delivery deadline.
    pub delivery_block: u64,
    /// Minimum accepted quality in `[0, 1]`.
    pub min_quality: f64,
}

/// Insurance policy terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsuranceParams {
    /// Canonical description of the covered event.
    pub covered_event: String,
    /// Maximum payout in smallest USDC units.
    pub payout_amount: u256,
    /// Decimal premium fraction of the payout.
    pub premium_rate: f64,
    /// Amount retained by the claimant before coverage applies.
    pub deductible: u256,
}

/// Synthetic index terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexParams {
    /// Weighted components.
    pub components: Vec<IndexComponent>,
    /// Minimum blocks between rebalances.
    pub rebalance_freq_blocks: u64,
}

/// One weighted synthetic-index component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexComponent {
    /// Unique component/domain key.
    pub domain: String,
    /// Non-negative normalized weight.
    pub weight: f64,
    /// Provider-neutral observation source.
    pub source: IndexSource,
}

/// Provider-neutral source for an index observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexSource {
    /// Local reputation registry.
    ReputationRegistry,
    /// Named arena leaderboard.
    ArenaLeaderboard(String),
    /// Named external or local market-rate provider.
    MarketRate(String),
    /// Caller-defined source.
    Custom(String),
}

/// Provider-neutral observation emitted by a DeFi registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefiRateEffect {
    /// Stable market key.
    pub market_id: String,
    /// Observed decimal rate or index value.
    pub rate: f64,
    /// Instrument that produced the observation.
    pub source_instrument_id: [u8; 32],
    /// Product family of the source.
    pub source_kind: InstrumentKind,
    /// Observation confidence in `[0, 1]`.
    pub confidence: f64,
    /// Block associated with the observation.
    pub block: u64,
}

/// A recorded coupon transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CouponPayment {
    /// Block at which the transfer occurred.
    pub block_number: u64,
    /// Amount transferred in smallest USDC units.
    pub amount: u256,
    /// Whether the scheduled coupon was paid.
    pub paid: bool,
}

/// Bond lifecycle failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BondError {
    /// Bond identifier does not exist.
    #[error("bond not found")]
    NotFound,
    /// Issuance collateral is below the configured ratio.
    #[error("insufficient collateral")]
    InsufficientCollateral,
    /// A maturity-only transition was attempted too early.
    #[error("bond has not matured")]
    BondNotMatured,
    /// The bond is already settled.
    #[error("bond is already settled")]
    AlreadySettled,
    /// The lifecycle state does not permit the operation.
    #[error("invalid bond state")]
    InvalidState,
    /// The current scheduled coupon was already paid.
    #[error("coupon already paid")]
    CouponAlreadyPaid,
    /// No coupon is due at the current block.
    #[error("coupon is not due")]
    CouponNotDue,
    /// Terms contain a zero, non-finite, negative, or out-of-range value.
    #[error("invalid bond terms")]
    InvalidTerms,
    /// Integer arithmetic would overflow.
    #[error("bond arithmetic overflow")]
    ArithmeticOverflow,
    /// Repayment is below face value.
    #[error("repayment is below face value")]
    InsufficientRepayment,
}

/// In-memory reputation bond lifecycle registry.
#[derive(Debug, Clone, Default)]
pub struct BondRegistry {
    /// Instruments keyed by ID.
    pub bonds: HashMap<[u8; 32], Instrument>,
    /// Completed coupon payments keyed by bond ID.
    pub coupon_payments: HashMap<[u8; 32], Vec<CouponPayment>>,
    /// Deterministic simulated block clock.
    pub current_block: u64,
    issuer_domains: HashMap<u256, String>,
    next_sequence: u64,
}

impl BondRegistry {
    /// Advance the simulated block clock; rewind requests are ignored.
    pub fn set_current_block(&mut self, block: u64) {
        self.current_block = self.current_block.max(block);
    }

    /// Associate an issuer with a canonical market domain.
    pub fn set_issuer_domain(
        &mut self,
        issuer_id: u256,
        domain: impl Into<String>,
    ) -> Result<(), BondError> {
        let domain = canonical_key(&domain.into()).ok_or(BondError::InvalidTerms)?;
        self.issuer_domains.insert(issuer_id, domain);
        Ok(())
    }

    /// Issue a fully collateralized reputation bond.
    pub fn issue_bond(
        &mut self,
        issuer_id: u256,
        face_value: u256,
        coupon_rate: f64,
        payment_frequency: u64,
        collateral: u256,
        duration_blocks: u64,
    ) -> Result<Instrument, BondError> {
        if face_value == 0 || payment_frequency == 0 || duration_blocks == 0 {
            return Err(BondError::InvalidTerms);
        }
        rate_units(coupon_rate).map_err(|_| BondError::InvalidTerms)?;
        let required = mul_div_ceil(face_value, COLLATERAL_RATIO_BPS, BPS_SCALE)
            .ok_or(BondError::ArithmeticOverflow)?;
        if collateral < required {
            return Err(BondError::InsufficientCollateral);
        }
        let expires_at_block = self
            .current_block
            .checked_add(duration_blocks)
            .ok_or(BondError::ArithmeticOverflow)?;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(BondError::ArithmeticOverflow)?;
        let id = make_id("bond", issuer_id, self.current_block, self.next_sequence);
        let instrument = Instrument {
            id,
            kind: InstrumentKind::Bond,
            issuer_id,
            collateral_usdc: collateral,
            created_at_block: self.current_block,
            expires_at_block,
            state: InstrumentState::Active,
            parameters: InstrumentParams::Bond(BondParams {
                face_value,
                coupon_rate,
                payment_frequency_blocks: payment_frequency,
            }),
        };
        self.bonds.insert(id, instrument.clone());
        self.coupon_payments.insert(id, Vec::new());
        Ok(instrument)
    }

    /// Pay the next due coupon exactly once.
    pub fn pay_coupon(&mut self, bond_id: [u8; 32]) -> Result<CouponPayment, BondError> {
        let bond = self.bonds.get(&bond_id).ok_or(BondError::NotFound)?;
        if bond.state == InstrumentState::Settled {
            return Err(BondError::AlreadySettled);
        }
        if bond.state != InstrumentState::Active {
            return Err(BondError::InvalidState);
        }
        let params = bond_params(bond)?;
        let payments = self
            .coupon_payments
            .get(&bond_id)
            .ok_or(BondError::NotFound)?;
        let last_payment_block = payments
            .last()
            .map_or(bond.created_at_block, |payment| payment.block_number);
        if last_payment_block == self.current_block && !payments.is_empty() {
            return Err(BondError::CouponAlreadyPaid);
        }
        let due_block = last_payment_block
            .checked_add(params.payment_frequency_blocks)
            .ok_or(BondError::ArithmeticOverflow)?;
        if self.current_block >= bond.expires_at_block {
            return Err(BondError::InvalidState);
        }
        if self.current_block < due_block {
            return Err(BondError::CouponNotDue);
        }
        let amount = apply_decimal_rate(params.face_value, params.coupon_rate)
            .map_err(|_| BondError::ArithmeticOverflow)?;
        let payment = CouponPayment {
            block_number: self.current_block,
            amount,
            paid: true,
        };
        self.coupon_payments
            .get_mut(&bond_id)
            .ok_or(BondError::NotFound)?
            .push(payment.clone());
        Ok(payment)
    }

    /// Transition an active bond to matured once expiry is reached.
    pub fn mature_bond(&mut self, bond_id: [u8; 32]) -> Result<(), BondError> {
        let bond = self.bonds.get_mut(&bond_id).ok_or(BondError::NotFound)?;
        if bond.state == InstrumentState::Settled {
            return Err(BondError::AlreadySettled);
        }
        if bond.state != InstrumentState::Active {
            return Err(BondError::InvalidState);
        }
        if self.current_block < bond.expires_at_block {
            return Err(BondError::BondNotMatured);
        }
        bond.state = InstrumentState::Matured;
        Ok(())
    }

    /// Settle a matured bond and return released collateral.
    pub fn settle_bond(
        &mut self,
        bond_id: [u8; 32],
        repayment_amount: u256,
    ) -> Result<u256, BondError> {
        let bond = self.bonds.get_mut(&bond_id).ok_or(BondError::NotFound)?;
        if bond.state == InstrumentState::Settled {
            return Err(BondError::AlreadySettled);
        }
        if bond.state != InstrumentState::Matured {
            return Err(BondError::InvalidState);
        }
        let face_value = match &bond.parameters {
            InstrumentParams::Bond(params) => params.face_value,
            _ => return Err(BondError::InvalidState),
        };
        if repayment_amount < face_value {
            return Err(BondError::InsufficientRepayment);
        }
        let released = std::mem::take(&mut bond.collateral_usdc);
        bond.state = InstrumentState::Settled;
        Ok(released)
    }

    /// Mark an unpaid matured bond as defaulted and return seized collateral.
    pub fn default_bond(&mut self, bond_id: [u8; 32]) -> Result<u256, BondError> {
        let bond = self.bonds.get_mut(&bond_id).ok_or(BondError::NotFound)?;
        if bond.state == InstrumentState::Settled {
            return Err(BondError::AlreadySettled);
        }
        if !matches!(
            bond.state,
            InstrumentState::Active | InstrumentState::Matured
        ) {
            return Err(BondError::InvalidState);
        }
        if self.current_block < bond.expires_at_block {
            return Err(BondError::BondNotMatured);
        }
        let seized = std::mem::take(&mut bond.collateral_usdc);
        bond.state = InstrumentState::Defaulted;
        Ok(seized)
    }

    /// Project active bond coupons into provider-neutral market observations.
    #[must_use]
    pub fn bond_rate_effects(&self) -> Vec<DefiRateEffect> {
        let mut effects: Vec<_> = self
            .bonds
            .values()
            .filter(|bond| bond.state == InstrumentState::Active)
            .filter_map(|bond| {
                let params = match &bond.parameters {
                    InstrumentParams::Bond(params) => params,
                    _ => return None,
                };
                let domain = self
                    .issuer_domains
                    .get(&bond.issuer_id)
                    .map_or("general", String::as_str);
                let ratio = ratio_as_f64(bond.collateral_usdc, params.face_value);
                Some(DefiRateEffect {
                    market_id: format!("bond.{domain}.coupon_rate"),
                    rate: params.coupon_rate,
                    source_instrument_id: bond.id,
                    source_kind: InstrumentKind::Bond,
                    confidence: (ratio / 2.0).clamp(0.0, 1.0),
                    block: self.current_block,
                })
            })
            .collect();
        effects.sort_by_key(|effect| effect.source_instrument_id);
        effects
    }
}

fn bond_params(bond: &Instrument) -> Result<&BondParams, BondError> {
    match &bond.parameters {
        InstrumentParams::Bond(params) => Ok(params),
        _ => Err(BondError::InvalidState),
    }
}

/// Vanilla Black-Scholes pricer for reputation-score options.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReputationOptionPricer;

/// First-order option sensitivities.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct OptionGreeks {
    /// Sensitivity to underlying score.
    pub delta: f64,
    /// Rate of change of delta.
    pub gamma: f64,
    /// Per-year time decay.
    pub theta: f64,
    /// Sensitivity to a one-unit volatility change.
    pub vega: f64,
}

impl ReputationOptionPricer {
    /// Price a European call; malformed inputs deterministically return zero.
    #[must_use]
    pub fn price_call(
        current_score: f64,
        strike_score: f64,
        time_to_expiry_blocks: u64,
        volatility: f64,
        risk_free_rate: f64,
    ) -> f64 {
        price_option(
            true,
            current_score,
            strike_score,
            time_to_expiry_blocks,
            volatility,
            risk_free_rate,
        )
    }

    /// Price a European put; malformed inputs deterministically return zero.
    #[must_use]
    pub fn price_put(
        current_score: f64,
        strike_score: f64,
        time_to_expiry_blocks: u64,
        volatility: f64,
        risk_free_rate: f64,
    ) -> f64 {
        price_option(
            false,
            current_score,
            strike_score,
            time_to_expiry_blocks,
            volatility,
            risk_free_rate,
        )
    }

    /// Estimate annualized sample volatility from block-stamped log returns.
    #[must_use]
    pub fn estimate_volatility(score_history: &[(u64, f64)]) -> f64 {
        if score_history.len() < 3 {
            return 0.0;
        }
        let mut observations = score_history.to_vec();
        observations.sort_by_key(|(block, _)| *block);
        if observations
            .iter()
            .any(|(_, score)| !score.is_finite() || *score <= 0.0)
        {
            return 0.0;
        }
        let mut returns = Vec::with_capacity(observations.len() - 1);
        let mut total_gap = 0_u64;
        for pair in observations.windows(2) {
            let gap = pair[1].0.saturating_sub(pair[0].0);
            if gap == 0 {
                return 0.0;
            }
            total_gap = match total_gap.checked_add(gap) {
                Some(value) => value,
                None => return 0.0,
            };
            returns.push((pair[1].1 / pair[0].1).ln());
        }
        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (returns.len() - 1) as f64;
        let average_gap = total_gap as f64 / returns.len() as f64;
        let result = variance.sqrt() * (BLOCKS_PER_YEAR / average_gap).sqrt();
        if result.is_finite() { result } else { 0.0 }
    }

    /// Compute call-option Greeks; malformed and degenerate inputs remain finite.
    #[must_use]
    pub fn compute_greeks(
        current_score: f64,
        strike_score: f64,
        time_to_expiry_blocks: u64,
        volatility: f64,
        risk_free_rate: f64,
    ) -> OptionGreeks {
        if !valid_option_inputs(current_score, strike_score, volatility, risk_free_rate) {
            return OptionGreeks::default();
        }
        let time = time_to_expiry_blocks as f64 / BLOCKS_PER_YEAR;
        if time == 0.0 {
            return OptionGreeks {
                delta: if current_score > strike_score {
                    1.0
                } else {
                    0.0
                },
                ..OptionGreeks::default()
            };
        }
        let discounted_strike = strike_score * (-risk_free_rate * time).exp();
        if volatility == 0.0 {
            return OptionGreeks {
                delta: if current_score > discounted_strike {
                    1.0
                } else {
                    0.0
                },
                ..OptionGreeks::default()
            };
        }
        let sqrt_time = time.sqrt();
        let d1 = ((current_score / strike_score).ln()
            + (risk_free_rate + volatility.powi(2) / 2.0) * time)
            / (volatility * sqrt_time);
        let d2 = d1 - volatility * sqrt_time;
        let density = normal_pdf(d1);
        if !d1.is_finite() || !d2.is_finite() || !density.is_finite() {
            return OptionGreeks::default();
        }
        let greeks = OptionGreeks {
            delta: normal_cdf(d1),
            gamma: density / (current_score * volatility * sqrt_time),
            theta: -(current_score * density * volatility) / (2.0 * sqrt_time)
                - risk_free_rate * strike_score * (-risk_free_rate * time).exp() * normal_cdf(d2),
            vega: current_score * density * sqrt_time,
        };
        OptionGreeks {
            delta: finite_or_zero(greeks.delta),
            gamma: finite_or_zero(greeks.gamma),
            theta: finite_or_zero(greeks.theta),
            vega: finite_or_zero(greeks.vega),
        }
    }
}

fn price_option(
    is_call: bool,
    current_score: f64,
    strike_score: f64,
    blocks: u64,
    volatility: f64,
    risk_free_rate: f64,
) -> f64 {
    if !valid_option_inputs(current_score, strike_score, volatility, risk_free_rate) {
        return 0.0;
    }
    let time = blocks as f64 / BLOCKS_PER_YEAR;
    if time == 0.0 {
        return if is_call {
            (current_score - strike_score).max(0.0)
        } else {
            (strike_score - current_score).max(0.0)
        };
    }
    let discounted_strike = strike_score * (-risk_free_rate * time).exp();
    if volatility == 0.0 {
        let value = if is_call {
            (current_score - discounted_strike).max(0.0)
        } else {
            (discounted_strike - current_score).max(0.0)
        };
        return finite_or_zero(value);
    }
    let sqrt_time = time.sqrt();
    let d1 = ((current_score / strike_score).ln()
        + (risk_free_rate + volatility.powi(2) / 2.0) * time)
        / (volatility * sqrt_time);
    let d2 = d1 - volatility * sqrt_time;
    let value = if is_call {
        current_score * normal_cdf(d1) - discounted_strike * normal_cdf(d2)
    } else {
        discounted_strike * normal_cdf(-d2) - current_score * normal_cdf(-d1)
    };
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn valid_option_inputs(score: f64, strike: f64, volatility: f64, rate: f64) -> bool {
    score.is_finite()
        && score > 0.0
        && strike.is_finite()
        && strike > 0.0
        && volatility.is_finite()
        && volatility >= 0.0
        && rate.is_finite()
}

// Abramowitz-Stegun 7.1.26 normal CDF approximation.
fn normal_cdf(value: f64) -> f64 {
    let x = value.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * x);
    let polynomial = t
        * (0.319_381_530
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    let approximation = 1.0 - normal_pdf(x) * polynomial;
    if value >= 0.0 {
        approximation
    } else {
        1.0 - approximation
    }
}

fn normal_pdf(value: f64) -> f64 {
    (-value.powi(2) / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

/// Insurance claim lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimState {
    /// Submitted by an insured buyer.
    Filed,
    /// Explicitly placed under review.
    UnderReview,
    /// Approved for payout.
    Approved,
    /// Rejected by the reviewer.
    Rejected,
    /// Paid and terminal.
    Paid,
}

/// Claim filed against an insurance policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsuranceClaim {
    /// Stable claim identifier.
    pub id: [u8; 32],
    /// Covered policy.
    pub policy_id: [u8; 32],
    /// Insured buyer filing the claim.
    pub claimant_id: u256,
    /// Claimed event.
    pub event_description: String,
    /// Optional immutable evidence digest.
    pub evidence_hash: Option<[u8; 32]>,
    /// Current review state.
    pub state: ClaimState,
    /// Net payout once paid.
    pub payout: Option<u256>,
    /// Filing block.
    pub claimed_at_block: u64,
}

/// Insurance lifecycle failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InsuranceError {
    /// Policy or claim identifier does not exist.
    #[error("policy or claim not found")]
    NotFound,
    /// Policy is expired or no longer active.
    #[error("policy is not active")]
    PolicyNotActive,
    /// This claimant or policy already has a claim/payout.
    #[error("claim already exists")]
    AlreadyClaimed,
    /// Supplied premium is below the contractual minimum.
    #[error("insufficient premium")]
    InsufficientPremium,
    /// Claimed event differs from the covered event.
    #[error("event is not covered")]
    EventNotCovered,
    /// Claim or policy state does not permit the operation.
    #[error("invalid insurance state")]
    InvalidState,
    /// Issuer collateral cannot cover the contractual payout.
    #[error("insufficient policy collateral")]
    InsufficientCollateral,
    /// Caller did not purchase this policy.
    #[error("claimant did not purchase this policy")]
    NotInsured,
    /// Terms contain an invalid value.
    #[error("invalid insurance terms")]
    InvalidTerms,
    /// Integer arithmetic would overflow.
    #[error("insurance arithmetic overflow")]
    ArithmeticOverflow,
    /// Buyer already purchased this policy.
    #[error("policy already purchased by buyer")]
    AlreadyPurchased,
}

/// In-memory insurance policy and claim registry.
#[derive(Debug, Clone, Default)]
pub struct InsuranceRegistry {
    /// Policy instruments keyed by ID.
    pub policies: HashMap<[u8; 32], Instrument>,
    /// Claims grouped by policy ID.
    pub claims: HashMap<[u8; 32], Vec<InsuranceClaim>>,
    /// Deterministic simulated block clock.
    pub current_block: u64,
    /// Residual collateral released back to issuers when policies settle.
    pub released_collateral: HashMap<[u8; 32], u256>,
    purchasers: HashMap<[u8; 32], HashSet<u256>>,
    next_sequence: u64,
}

impl InsuranceRegistry {
    /// Advance the simulated block clock; rewind requests are ignored.
    pub fn set_current_block(&mut self, block: u64) {
        self.current_block = self.current_block.max(block);
    }

    /// Create a fully collateralized policy for the default issuer ID zero.
    pub fn create_policy(
        &mut self,
        params: InsuranceParams,
        collateral: u256,
        duration_blocks: u64,
    ) -> Result<Instrument, InsuranceError> {
        self.create_policy_for(0, params, collateral, duration_blocks)
    }

    /// Create a fully collateralized policy for an explicit issuer.
    pub fn create_policy_for(
        &mut self,
        issuer_id: u256,
        params: InsuranceParams,
        collateral: u256,
        duration_blocks: u64,
    ) -> Result<Instrument, InsuranceError> {
        if canonical_key(&params.covered_event).is_none()
            || params.payout_amount == 0
            || params.deductible > params.payout_amount
            || duration_blocks == 0
            || rate_units(params.premium_rate).is_err()
        {
            return Err(InsuranceError::InvalidTerms);
        }
        if collateral < params.payout_amount {
            return Err(InsuranceError::InsufficientCollateral);
        }
        let expires_at_block = self
            .current_block
            .checked_add(duration_blocks)
            .ok_or(InsuranceError::ArithmeticOverflow)?;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(InsuranceError::ArithmeticOverflow)?;
        let id = make_id(
            "insurance",
            issuer_id,
            self.current_block,
            self.next_sequence,
        );
        let policy = Instrument {
            id,
            kind: InstrumentKind::InsurancePolicy,
            issuer_id,
            collateral_usdc: collateral,
            created_at_block: self.current_block,
            expires_at_block,
            state: InstrumentState::Active,
            parameters: InstrumentParams::InsurancePolicy(params),
        };
        self.policies.insert(id, policy.clone());
        self.claims.insert(id, Vec::new());
        self.purchasers.insert(id, HashSet::new());
        Ok(policy)
    }

    /// Purchase active coverage by paying at least the fixed-point premium.
    pub fn purchase_policy(
        &mut self,
        policy_id: [u8; 32],
        buyer_id: u256,
        premium: u256,
    ) -> Result<(), InsuranceError> {
        let policy = self
            .policies
            .get(&policy_id)
            .ok_or(InsuranceError::NotFound)?;
        self.ensure_policy_active(policy)?;
        let params = insurance_params(policy)?;
        let required = apply_decimal_rate(params.payout_amount, params.premium_rate)
            .map_err(|_| InsuranceError::ArithmeticOverflow)?;
        if premium < required {
            return Err(InsuranceError::InsufficientPremium);
        }
        let purchasers = self
            .purchasers
            .get_mut(&policy_id)
            .ok_or(InsuranceError::NotFound)?;
        if !purchasers.insert(buyer_id) {
            return Err(InsuranceError::AlreadyPurchased);
        }
        Ok(())
    }

    /// File one claim per insured buyer against the exact covered event.
    pub fn file_claim(
        &mut self,
        policy_id: [u8; 32],
        claimant_id: u256,
        event: impl Into<String>,
        evidence_hash: Option<[u8; 32]>,
    ) -> Result<InsuranceClaim, InsuranceError> {
        let event = event.into();
        let policy = self
            .policies
            .get(&policy_id)
            .ok_or(InsuranceError::NotFound)?;
        self.ensure_policy_active(policy)?;
        let covered_event = insurance_params(policy)?.covered_event.clone();
        if !self
            .purchasers
            .get(&policy_id)
            .is_some_and(|buyers| buyers.contains(&claimant_id))
        {
            return Err(InsuranceError::NotInsured);
        }
        if canonical_key(&event) != canonical_key(&covered_event) {
            return Err(InsuranceError::EventNotCovered);
        }
        if self
            .claims
            .get(&policy_id)
            .is_some_and(|claims| claims.iter().any(|claim| claim.claimant_id == claimant_id))
        {
            return Err(InsuranceError::AlreadyClaimed);
        }
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(InsuranceError::ArithmeticOverflow)?;
        let id = make_id("claim", claimant_id, self.current_block, self.next_sequence);
        let claim = InsuranceClaim {
            id,
            policy_id,
            claimant_id,
            event_description: event,
            evidence_hash,
            state: ClaimState::Filed,
            payout: None,
            claimed_at_block: self.current_block,
        };
        self.claims
            .get_mut(&policy_id)
            .ok_or(InsuranceError::NotFound)?
            .push(claim.clone());
        Ok(claim)
    }

    /// Move a filed claim into explicit review.
    pub fn review_claim(&mut self, claim_id: [u8; 32]) -> Result<(), InsuranceError> {
        let claim = self.find_claim_mut(claim_id)?;
        if claim.state != ClaimState::Filed {
            return Err(InsuranceError::InvalidState);
        }
        claim.state = ClaimState::UnderReview;
        Ok(())
    }

    /// Approve a claim after it has entered review.
    pub fn approve_claim(&mut self, claim_id: [u8; 32]) -> Result<(), InsuranceError> {
        let claim = self.find_claim_mut(claim_id)?;
        if claim.state != ClaimState::UnderReview {
            return Err(InsuranceError::InvalidState);
        }
        claim.state = ClaimState::Approved;
        Ok(())
    }

    /// Reject a claim after it has entered review.
    pub fn reject_claim(&mut self, claim_id: [u8; 32]) -> Result<(), InsuranceError> {
        let claim = self.find_claim_mut(claim_id)?;
        if claim.state != ClaimState::UnderReview {
            return Err(InsuranceError::InvalidState);
        }
        claim.state = ClaimState::Rejected;
        Ok(())
    }

    /// Pay an approved claim, deducting the contractual deductible exactly once.
    pub fn pay_claim(&mut self, claim_id: [u8; 32]) -> Result<u256, InsuranceError> {
        let (policy_id, state) = self
            .claims
            .values()
            .flat_map(|claims| claims.iter())
            .find(|claim| claim.id == claim_id)
            .map(|claim| (claim.policy_id, claim.state))
            .ok_or(InsuranceError::NotFound)?;
        if state != ClaimState::Approved {
            return Err(InsuranceError::InvalidState);
        }
        if self
            .claims
            .get(&policy_id)
            .is_some_and(|claims| claims.iter().any(|claim| claim.state == ClaimState::Paid))
        {
            return Err(InsuranceError::AlreadyClaimed);
        }
        let policy = self
            .policies
            .get_mut(&policy_id)
            .ok_or(InsuranceError::NotFound)?;
        if policy.state != InstrumentState::Active {
            return Err(InsuranceError::PolicyNotActive);
        }
        let params = match &policy.parameters {
            InstrumentParams::InsurancePolicy(params) => params,
            _ => return Err(InsuranceError::InvalidState),
        };
        let payout = params
            .payout_amount
            .checked_sub(params.deductible)
            .ok_or(InsuranceError::InvalidTerms)?;
        if policy.collateral_usdc < payout {
            return Err(InsuranceError::InsufficientCollateral);
        }
        let locked_collateral = std::mem::take(&mut policy.collateral_usdc);
        let released = locked_collateral - payout;
        policy.state = InstrumentState::Settled;
        self.released_collateral.insert(policy_id, released);
        let claim = self.find_claim_mut(claim_id)?;
        claim.state = ClaimState::Paid;
        claim.payout = Some(payout);
        Ok(payout)
    }

    fn ensure_policy_active(&self, policy: &Instrument) -> Result<(), InsuranceError> {
        if policy.state != InstrumentState::Active || self.current_block >= policy.expires_at_block
        {
            return Err(InsuranceError::PolicyNotActive);
        }
        Ok(())
    }

    fn find_claim_mut(
        &mut self,
        claim_id: [u8; 32],
    ) -> Result<&mut InsuranceClaim, InsuranceError> {
        self.claims
            .values_mut()
            .flat_map(|claims| claims.iter_mut())
            .find(|claim| claim.id == claim_id)
            .ok_or(InsuranceError::NotFound)
    }
}

fn insurance_params(policy: &Instrument) -> Result<&InsuranceParams, InsuranceError> {
    match &policy.parameters {
        InstrumentParams::InsurancePolicy(params) => Ok(params),
        _ => Err(InsuranceError::InvalidState),
    }
}

/// One computed synthetic-index value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexValuation {
    /// Index instrument identifier.
    pub index_id: [u8; 32],
    /// Exact domain/value inputs in declared component order.
    pub components_values: Vec<(String, f64)>,
    /// Weighted component sum.
    pub weighted_value: f64,
    /// Computation block.
    pub computed_at_block: u64,
}

/// Synthetic-index lifecycle failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IndexError {
    /// Index identifier does not exist.
    #[error("index not found")]
    NotFound,
    /// Weights are invalid or do not sum to one.
    #[error("index weights are not normalized")]
    WeightsNotNormalized,
    /// Rebalance cadence has not elapsed.
    #[error("index rebalance is too early")]
    RebalanceTooEarly,
    /// Instrument lifecycle state does not permit the operation.
    #[error("invalid index state")]
    InvalidState,
    /// Component domains or supplied values do not match exactly.
    #[error("index component values do not match declared domains")]
    ComponentMismatch,
    /// A domain is empty or duplicated.
    #[error("invalid index domain")]
    InvalidDomain,
    /// Integer arithmetic would overflow.
    #[error("index arithmetic overflow")]
    ArithmeticOverflow,
}

/// In-memory synthetic-index registry with bounded valuation history.
#[derive(Debug, Clone, Default)]
pub struct SyntheticIndexRegistry {
    /// Index instruments keyed by ID.
    pub indices: HashMap<[u8; 32], Instrument>,
    /// Bounded valuation histories keyed by index ID.
    pub valuations: HashMap<[u8; 32], Vec<IndexValuation>>,
    /// Deterministic simulated block clock.
    pub current_block: u64,
    last_rebalances: HashMap<[u8; 32], u64>,
    next_sequence: u64,
}

impl SyntheticIndexRegistry {
    /// Advance the simulated block clock; rewind requests are ignored.
    pub fn set_current_block(&mut self, block: u64) {
        self.current_block = self.current_block.max(block);
    }

    /// Create an active normalized synthetic index.
    pub fn create_index(
        &mut self,
        components: Vec<IndexComponent>,
        rebalance_freq: u64,
        collateral: u256,
    ) -> Result<Instrument, IndexError> {
        if rebalance_freq == 0 {
            return Err(IndexError::RebalanceTooEarly);
        }
        validate_components(&components)?;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(IndexError::ArithmeticOverflow)?;
        let id = make_id("index", 0, self.current_block, self.next_sequence);
        let instrument = Instrument {
            id,
            kind: InstrumentKind::SyntheticIndex,
            issuer_id: 0,
            collateral_usdc: collateral,
            created_at_block: self.current_block,
            expires_at_block: u64::MAX,
            state: InstrumentState::Active,
            parameters: InstrumentParams::SyntheticIndex(IndexParams {
                components,
                rebalance_freq_blocks: rebalance_freq,
            }),
        };
        self.indices.insert(id, instrument.clone());
        self.valuations.insert(id, Vec::new());
        self.last_rebalances.insert(id, self.current_block);
        Ok(instrument)
    }

    /// Compute a valuation from an exact one-to-one set of component values.
    pub fn compute_valuation(
        &mut self,
        index_id: [u8; 32],
        component_values: &[(String, f64)],
    ) -> Result<IndexValuation, IndexError> {
        let instrument = self.indices.get(&index_id).ok_or(IndexError::NotFound)?;
        if instrument.state != InstrumentState::Active {
            return Err(IndexError::InvalidState);
        }
        let params = index_params(instrument)?;
        if component_values.len() != params.components.len()
            || component_values.iter().any(|(_, value)| !value.is_finite())
        {
            return Err(IndexError::ComponentMismatch);
        }
        let mut supplied = HashMap::with_capacity(component_values.len());
        for (domain, value) in component_values {
            if supplied.insert(domain.as_str(), *value).is_some() {
                return Err(IndexError::ComponentMismatch);
            }
        }
        let mut ordered = Vec::with_capacity(params.components.len());
        let mut weighted_value = 0.0;
        for component in &params.components {
            let value = supplied
                .get(component.domain.as_str())
                .copied()
                .ok_or(IndexError::ComponentMismatch)?;
            ordered.push((component.domain.clone(), value));
            weighted_value += component.weight * value;
        }
        if !weighted_value.is_finite() {
            return Err(IndexError::ComponentMismatch);
        }
        let valuation = IndexValuation {
            index_id,
            components_values: ordered,
            weighted_value,
            computed_at_block: self.current_block,
        };
        let history = self
            .valuations
            .get_mut(&index_id)
            .ok_or(IndexError::NotFound)?;
        if history.len() == MAX_VALUATION_HISTORY {
            history.remove(0);
        }
        history.push(valuation.clone());
        Ok(valuation)
    }

    /// Rebalance existing domains after the declared cadence elapses.
    pub fn rebalance(
        &mut self,
        index_id: [u8; 32],
        new_weights: Vec<(String, f64)>,
    ) -> Result<(), IndexError> {
        let instrument = self
            .indices
            .get_mut(&index_id)
            .ok_or(IndexError::NotFound)?;
        if instrument.state != InstrumentState::Active {
            return Err(IndexError::InvalidState);
        }
        let params = match &mut instrument.parameters {
            InstrumentParams::SyntheticIndex(params) => params,
            _ => return Err(IndexError::InvalidState),
        };
        let last = self
            .last_rebalances
            .get(&index_id)
            .copied()
            .ok_or(IndexError::NotFound)?;
        let due = last
            .checked_add(params.rebalance_freq_blocks)
            .ok_or(IndexError::ArithmeticOverflow)?;
        if self.current_block < due {
            return Err(IndexError::RebalanceTooEarly);
        }
        if new_weights.len() != params.components.len() {
            return Err(IndexError::WeightsNotNormalized);
        }
        let mut weights = HashMap::with_capacity(new_weights.len());
        for (domain, weight) in new_weights {
            if !weight.is_finite() || weight < 0.0 || weights.insert(domain, weight).is_some() {
                return Err(IndexError::WeightsNotNormalized);
            }
        }
        if (weights.values().sum::<f64>() - 1.0).abs() > INDEX_WEIGHT_EPSILON {
            return Err(IndexError::WeightsNotNormalized);
        }
        let mut validated_weights = Vec::with_capacity(params.components.len());
        for component in &params.components {
            validated_weights.push(
                weights
                    .remove(&component.domain)
                    .ok_or(IndexError::ComponentMismatch)?,
            );
        }
        if !weights.is_empty() {
            return Err(IndexError::ComponentMismatch);
        }
        for (component, weight) in params.components.iter_mut().zip(validated_weights) {
            component.weight = weight;
        }
        self.last_rebalances.insert(index_id, self.current_block);
        Ok(())
    }

    /// Return the most recent `n` valuations in chronological order.
    #[must_use]
    pub fn get_valuation_history(&self, index_id: [u8; 32], n: usize) -> Vec<IndexValuation> {
        self.valuations
            .get(&index_id)
            .map_or_else(Vec::new, |history| {
                history[history.len().saturating_sub(n)..].to_vec()
            })
    }

    /// Project each active index's latest value into a market observation.
    #[must_use]
    pub fn index_rate_effects(&self) -> Vec<DefiRateEffect> {
        let mut effects: Vec<_> = self
            .indices
            .values()
            .filter(|instrument| instrument.state == InstrumentState::Active)
            .filter_map(|instrument| {
                let valuation = self.valuations.get(&instrument.id)?.last()?;
                Some(DefiRateEffect {
                    market_id: format!("index.{}.weighted_value", hex_id(&instrument.id)),
                    rate: valuation.weighted_value,
                    source_instrument_id: instrument.id,
                    source_kind: InstrumentKind::SyntheticIndex,
                    confidence: 1.0,
                    block: valuation.computed_at_block,
                })
            })
            .collect();
        effects.sort_by_key(|effect| effect.source_instrument_id);
        effects
    }
}

fn index_params(instrument: &Instrument) -> Result<&IndexParams, IndexError> {
    match &instrument.parameters {
        InstrumentParams::SyntheticIndex(params) => Ok(params),
        _ => Err(IndexError::InvalidState),
    }
}

fn validate_components(components: &[IndexComponent]) -> Result<(), IndexError> {
    if components.is_empty()
        || components
            .iter()
            .any(|component| !component.weight.is_finite() || component.weight < 0.0)
        || (components
            .iter()
            .map(|component| component.weight)
            .sum::<f64>()
            - 1.0)
            .abs()
            > INDEX_WEIGHT_EPSILON
    {
        return Err(IndexError::WeightsNotNormalized);
    }
    let mut domains = HashSet::with_capacity(components.len());
    for component in components {
        let domain = canonical_key(&component.domain).ok_or(IndexError::InvalidDomain)?;
        if domain != component.domain || !domains.insert(domain) {
            return Err(IndexError::InvalidDomain);
        }
    }
    Ok(())
}

fn canonical_key(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_lowercase().replace(' ', "-");
    if normalized.is_empty()
        || !normalized
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return None;
    }
    Some(normalized)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InvalidRate;

fn rate_units(rate: f64) -> Result<u256, InvalidRate> {
    if !rate.is_finite() || !(0.0..=1.0).contains(&rate) {
        return Err(InvalidRate);
    }
    let scaled = (rate * RATE_SCALE as f64).round();
    if !(0.0..=RATE_SCALE as f64).contains(&scaled) {
        return Err(InvalidRate);
    }
    Ok(scaled as u256)
}

fn apply_decimal_rate(amount: u256, rate: f64) -> Result<u256, InvalidRate> {
    let units = rate_units(rate)?;
    mul_div_ceil(amount, units, RATE_SCALE).ok_or(InvalidRate)
}

fn mul_div_ceil(value: u256, numerator: u256, denominator: u256) -> Option<u256> {
    if denominator == 0 {
        return None;
    }
    // Split before multiplying. All current callers guarantee
    // `numerator <= denominator`, so both products fit whenever the exact
    // quotient fits, including at `u128::MAX`.
    if numerator > denominator {
        return None;
    }
    let whole = (value / denominator).checked_mul(numerator)?;
    let remainder_product = (value % denominator).checked_mul(numerator)?;
    let fractional = remainder_product / denominator;
    let round_up = u256::from(remainder_product % denominator != 0);
    whole.checked_add(fractional)?.checked_add(round_up)
}

fn ratio_as_f64(numerator: u256, denominator: u256) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let ratio = numerator as f64 / denominator as f64;
    if ratio.is_finite() { ratio } else { 0.0 }
}

fn make_id(namespace: &str, issuer: u256, block: u64, sequence: u64) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"roko:defi:v1\0");
    hasher.update(namespace.as_bytes());
    hasher.update(&issuer.to_be_bytes());
    hasher.update(&block.to_be_bytes());
    hasher.update(&sequence.to_be_bytes());
    *hasher.finalize().as_bytes()
}

fn hex_id(id: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(64);
    for byte in id {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn components() -> Vec<IndexComponent> {
        vec![
            IndexComponent {
                domain: "coding".into(),
                weight: 0.6,
                source: IndexSource::ReputationRegistry,
            },
            IndexComponent {
                domain: "security".into(),
                weight: 0.4,
                source: IndexSource::MarketRate("security-risk".into()),
            },
        ]
    }

    #[test]
    fn bond_issue_mature_and_settle_releases_exact_collateral() {
        let mut registry = BondRegistry::default();
        registry.set_current_block(10);
        let bond = registry.issue_bond(7, 1_000, 0.05, 10, 1_500, 100).unwrap();
        assert_eq!(
            registry.mature_bond(bond.id),
            Err(BondError::BondNotMatured)
        );
        registry.set_current_block(110);
        registry.mature_bond(bond.id).unwrap();
        assert_eq!(
            registry.settle_bond(bond.id, 999),
            Err(BondError::InsufficientRepayment)
        );
        assert_eq!(registry.settle_bond(bond.id, 1_000).unwrap(), 1_500);
        assert_eq!(registry.bonds[&bond.id].state, InstrumentState::Settled);
        assert_eq!(registry.bonds[&bond.id].collateral_usdc, 0);
    }

    #[test]
    fn bond_coupon_is_fixed_point_scheduled_and_not_double_paid() {
        let mut registry = BondRegistry::default();
        let bond = registry
            .issue_bond(1, 10_001, 0.025, 5, 10_001, 20)
            .unwrap();
        assert_eq!(registry.pay_coupon(bond.id), Err(BondError::CouponNotDue));
        registry.set_current_block(5);
        assert_eq!(registry.pay_coupon(bond.id).unwrap().amount, 251);
        assert_eq!(
            registry.pay_coupon(bond.id),
            Err(BondError::CouponAlreadyPaid)
        );
        registry.set_current_block(10);
        assert_eq!(registry.pay_coupon(bond.id).unwrap().amount, 251);
    }

    #[test]
    fn late_coupon_cannot_catch_up_multiple_times_in_one_block() {
        let mut registry = BondRegistry::default();
        let bond = registry.issue_bond(1, 100, 0.1, 5, 100, 30).unwrap();
        registry.set_current_block(15);
        registry.pay_coupon(bond.id).unwrap();
        assert_eq!(
            registry.pay_coupon(bond.id),
            Err(BondError::CouponAlreadyPaid)
        );
        registry.set_current_block(19);
        assert_eq!(registry.pay_coupon(bond.id), Err(BondError::CouponNotDue));
        registry.set_current_block(20);
        registry.pay_coupon(bond.id).unwrap();
    }

    #[test]
    fn bond_default_seizes_collateral_only_after_expiry() {
        let mut registry = BondRegistry::default();
        let bond = registry.issue_bond(1, 500, 0.01, 10, 800, 20).unwrap();
        assert_eq!(
            registry.default_bond(bond.id),
            Err(BondError::BondNotMatured)
        );
        registry.set_current_block(20);
        assert_eq!(registry.default_bond(bond.id).unwrap(), 800);
        assert_eq!(registry.bonds[&bond.id].state, InstrumentState::Defaulted);
        assert_eq!(registry.default_bond(bond.id), Err(BondError::InvalidState));
    }

    #[test]
    fn bond_handles_full_range_amounts_and_rejects_invalid_rate() {
        let mut registry = BondRegistry::default();
        let largest = registry
            .issue_bond(1, u128::MAX, 0.1, 1, u128::MAX, 1)
            .unwrap();
        registry.set_current_block(1);
        registry.mature_bond(largest.id).unwrap();
        assert_eq!(
            registry.settle_bond(largest.id, u128::MAX).unwrap(),
            u128::MAX
        );
        assert_eq!(apply_decimal_rate(u128::MAX, 1.0).unwrap(), u128::MAX);
        assert_eq!(
            apply_decimal_rate(u128::MAX, 0.5).unwrap(),
            u128::MAX / 2 + 1
        );
        assert_eq!(
            registry.issue_bond(1, 10, f64::NAN, 1, 10, 1),
            Err(BondError::InvalidTerms)
        );
        registry.set_current_block(u64::MAX);
        assert_eq!(
            registry.issue_bond(1, 10, 0.1, 1, 10, 1),
            Err(BondError::ArithmeticOverflow)
        );
    }

    #[test]
    fn option_prices_obey_put_call_parity() {
        let blocks = 1_314_000;
        let call = ReputationOptionPricer::price_call(0.8, 0.7, blocks, 0.25, 0.03);
        let put = ReputationOptionPricer::price_put(0.8, 0.7, blocks, 0.25, 0.03);
        let expected = 0.8 - 0.7 * (-0.03_f64 * 0.5).exp();
        assert!((call - put - expected).abs() < 1e-8);
        assert!(call > 0.0);
    }

    #[test]
    fn option_zero_volatility_and_expiry_are_finite() {
        let call = ReputationOptionPricer::price_call(0.8, 0.7, 100, 0.0, 0.02);
        assert!(call.is_finite() && call > 0.0);
        assert_eq!(
            ReputationOptionPricer::price_put(0.8, 0.7, 0, 0.0, 0.02),
            0.0
        );
        let greeks = ReputationOptionPricer::compute_greeks(0.8, 0.7, 0, 0.0, 0.02);
        assert_eq!(greeks.delta, 1.0);
        assert!(greeks.gamma.is_finite());
    }

    #[test]
    fn option_invalid_inputs_and_short_history_do_not_produce_nan() {
        assert_eq!(
            ReputationOptionPricer::price_call(f64::NAN, 0.5, 10, 0.2, 0.0),
            0.0
        );
        assert_eq!(
            ReputationOptionPricer::estimate_volatility(&[(1, 0.5), (2, 0.6)]),
            0.0
        );
        assert!(
            ReputationOptionPricer::estimate_volatility(&[
                (1, 0.5),
                (101, 0.55),
                (201, 0.51),
                (301, 0.58),
            ]) > 0.0
        );
    }

    #[test]
    fn option_greeks_remain_finite_for_extreme_finite_inputs() {
        let greeks = ReputationOptionPricer::compute_greeks(
            f64::MAX,
            f64::MIN_POSITIVE,
            1,
            f64::MIN_POSITIVE,
            f64::MAX,
        );
        assert!(greeks.delta.is_finite());
        assert!(greeks.gamma.is_finite());
        assert!(greeks.theta.is_finite());
        assert!(greeks.vega.is_finite());
        for value in [
            ReputationOptionPricer::price_call(1.0, 1.0, 2_628_000, 0.0, -1_000.0),
            ReputationOptionPricer::price_put(1.0, 1.0, 2_628_000, 0.0, -1_000.0),
            ReputationOptionPricer::price_call(
                f64::MAX,
                f64::MIN_POSITIVE,
                1,
                f64::MIN_POSITIVE,
                f64::MAX,
            ),
            ReputationOptionPricer::price_put(
                f64::MAX,
                f64::MIN_POSITIVE,
                1,
                f64::MIN_POSITIVE,
                f64::MAX,
            ),
        ] {
            assert!(value.is_finite());
        }
    }

    #[test]
    fn insurance_purchase_claim_approval_and_payment_are_bound_to_buyer() {
        let mut registry = InsuranceRegistry::default();
        let policy = registry
            .create_policy_for(
                9,
                InsuranceParams {
                    covered_event: "model-outage".into(),
                    payout_amount: 10_000,
                    premium_rate: 0.02,
                    deductible: 500,
                },
                10_000,
                100,
            )
            .unwrap();
        assert_eq!(
            registry.purchase_policy(policy.id, 4, 199),
            Err(InsuranceError::InsufficientPremium)
        );
        registry.purchase_policy(policy.id, 4, 200).unwrap();
        assert_eq!(
            registry.file_claim(policy.id, 5, "model-outage", None),
            Err(InsuranceError::NotInsured)
        );
        let claim = registry
            .file_claim(policy.id, 4, "MODEL-OUTAGE", Some([3; 32]))
            .unwrap();
        registry.review_claim(claim.id).unwrap();
        registry.approve_claim(claim.id).unwrap();
        assert_eq!(registry.pay_claim(claim.id).unwrap(), 9_500);
        assert_eq!(
            registry.policies[&policy.id].state,
            InstrumentState::Settled
        );
        assert_eq!(registry.policies[&policy.id].collateral_usdc, 0);
        assert_eq!(registry.released_collateral[&policy.id], 500);
    }

    #[test]
    fn insurance_rejects_duplicate_and_uncovered_claims() {
        let mut registry = InsuranceRegistry::default();
        let policy = registry
            .create_policy(
                InsuranceParams {
                    covered_event: "gate-failure".into(),
                    payout_amount: 100,
                    premium_rate: 0.1,
                    deductible: 0,
                },
                100,
                10,
            )
            .unwrap();
        registry.purchase_policy(policy.id, 2, 10).unwrap();
        assert_eq!(
            registry.file_claim(policy.id, 2, "budget-overrun", None),
            Err(InsuranceError::EventNotCovered)
        );
        let claim = registry
            .file_claim(policy.id, 2, "gate-failure", None)
            .unwrap();
        assert_eq!(
            registry.file_claim(policy.id, 2, "gate-failure", None),
            Err(InsuranceError::AlreadyClaimed)
        );
        assert_eq!(
            registry.approve_claim(claim.id),
            Err(InsuranceError::InvalidState)
        );
        assert_eq!(
            registry.reject_claim(claim.id),
            Err(InsuranceError::InvalidState)
        );
        registry.review_claim(claim.id).unwrap();
        registry.reject_claim(claim.id).unwrap();
    }

    #[test]
    fn expired_policy_rejects_purchase_and_claim() {
        let mut registry = InsuranceRegistry::default();
        let policy = registry
            .create_policy(
                InsuranceParams {
                    covered_event: "outage".into(),
                    payout_amount: 10,
                    premium_rate: 0.1,
                    deductible: 0,
                },
                10,
                2,
            )
            .unwrap();
        registry.purchase_policy(policy.id, 3, 1).unwrap();
        registry.set_current_block(2);
        assert_eq!(
            registry.file_claim(policy.id, 3, "outage", None),
            Err(InsuranceError::PolicyNotActive)
        );
    }

    #[test]
    fn index_create_value_rebalance_and_history() {
        let mut registry = SyntheticIndexRegistry::default();
        let index = registry.create_index(components(), 10, 50).unwrap();
        let valuation = registry
            .compute_valuation(
                index.id,
                &[("security".into(), 0.5), ("coding".into(), 0.9)],
            )
            .unwrap();
        assert!((valuation.weighted_value - 0.74).abs() < 1e-12);
        assert_eq!(
            registry.rebalance(
                index.id,
                vec![("coding".into(), 0.5), ("security".into(), 0.5)]
            ),
            Err(IndexError::RebalanceTooEarly)
        );
        registry.set_current_block(10);
        registry
            .rebalance(
                index.id,
                vec![("coding".into(), 0.5), ("security".into(), 0.5)],
            )
            .unwrap();
        assert_eq!(registry.get_valuation_history(index.id, 1), vec![valuation]);
    }

    #[test]
    fn index_rejects_bad_weights_and_mismatched_values() {
        let mut registry = SyntheticIndexRegistry::default();
        let mut invalid = components();
        invalid[0].weight = f64::NAN;
        assert_eq!(
            registry.create_index(invalid, 10, 0),
            Err(IndexError::WeightsNotNormalized)
        );
        let index = registry.create_index(components(), 10, 0).unwrap();
        assert_eq!(
            registry.compute_valuation(index.id, &[("coding".into(), 1.0)]),
            Err(IndexError::ComponentMismatch)
        );
        assert_eq!(
            registry.compute_valuation(index.id, &[("coding".into(), 1.0), ("coding".into(), 0.5)]),
            Err(IndexError::ComponentMismatch)
        );
    }

    #[test]
    fn failed_index_rebalance_is_atomic() {
        let mut registry = SyntheticIndexRegistry::default();
        let index = registry.create_index(components(), 10, 0).unwrap();
        registry.set_current_block(10);
        assert_eq!(
            registry.rebalance(
                index.id,
                vec![("coding".into(), 0.25), ("unknown".into(), 0.75)]
            ),
            Err(IndexError::ComponentMismatch)
        );
        let params = index_params(&registry.indices[&index.id]).unwrap();
        assert_eq!(params.components[0].weight, 0.6);
        assert_eq!(params.components[1].weight, 0.4);
    }

    #[test]
    fn bond_effects_are_provider_neutral_and_collateral_weighted() {
        let mut registry = BondRegistry::default();
        registry.set_issuer_domain(8, "security").unwrap();
        let bond = registry.issue_bond(8, 100, 0.08, 5, 200, 20).unwrap();
        let effects = registry.bond_rate_effects();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].market_id, "bond.security.coupon_rate");
        assert_eq!(effects[0].source_instrument_id, bond.id);
        assert_eq!(effects[0].confidence, 1.0);
    }

    #[test]
    fn index_effect_uses_latest_valuation_and_stable_id() {
        let mut registry = SyntheticIndexRegistry::default();
        let index = registry.create_index(components(), 10, 0).unwrap();
        registry
            .compute_valuation(
                index.id,
                &[("coding".into(), 0.8), ("security".into(), 0.7)],
            )
            .unwrap();
        registry.set_current_block(4);
        registry
            .compute_valuation(
                index.id,
                &[("coding".into(), 0.9), ("security".into(), 0.8)],
            )
            .unwrap();
        let effects = registry.index_rate_effects();
        assert_eq!(effects.len(), 1);
        assert!(effects[0].market_id.starts_with("index."));
        assert!(effects[0].market_id.ends_with(".weighted_value"));
        assert!((effects[0].rate - 0.86).abs() < 1e-12);
        assert_eq!(effects[0].block, 4);
    }

    #[test]
    fn instrument_types_round_trip_through_json() {
        let instrument = Instrument {
            id: [9; 32],
            kind: InstrumentKind::KnowledgeFuture,
            issuer_id: 12,
            collateral_usdc: 30,
            created_at_block: 1,
            expires_at_block: 9,
            state: InstrumentState::Active,
            parameters: InstrumentParams::KnowledgeFuture(FutureParams {
                knowledge_spec: "proof".into(),
                target_hdc: Some(vec![0.1, 0.2]),
                delivery_block: 9,
                min_quality: 0.8,
            }),
        };
        let encoded = serde_json::to_string(&instrument).unwrap();
        assert_eq!(
            serde_json::from_str::<Instrument>(&encoded).unwrap(),
            instrument
        );
        assert_eq!(
            InstrumentKind::InsurancePolicy.to_market_prefix(),
            "insurance"
        );
    }
}
