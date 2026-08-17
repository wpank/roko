//! Spore job marketplace with escrow and 3 hiring models (CHAIN-04).
//!
//! Implements the full job lifecycle state machine:
//! POSTED -> ASSIGNED -> IN_PROGRESS -> SUBMITTED -> SETTLED / DISPUTED / EXPIRED
//!
//! Three hiring models:
//! 1. RandomVRF -- power-of-two-choices (Sparrow, O(log log N) max load)
//! 2. BlindAuction -- commit-reveal Vickrey second-price with reputation adjustment
//! 3. DirectHire -- 1.5x premium, restricted to Protocol/Sovereign tier
//!
//! Escrow handles deposit/release/dispute/refund with 4-level dispute resolution.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::identity_economy_markets::{BountySpec, SparrowBid};
use crate::phase2::{
    DisputeLevel, DisputeOutcome, DisputeResolution, HiringModel, PassportTier, SporeJobPosting,
    u256,
};

// ---------------------------------------------------------------------------
// Job lifecycle
// ---------------------------------------------------------------------------

/// Job lifecycle states per the Spore spec (doc 10).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    /// Job posted and awaiting assignment.
    #[default]
    Posted,
    /// Agent assigned; work not yet started.
    Assigned,
    /// Agent is actively working.
    InProgress,
    /// Result submitted; awaiting settlement.
    Submitted,
    /// Settled successfully; escrow released.
    Settled,
    /// Under dispute; escrow locked.
    Disputed,
    /// Deadline passed without valid delivery.
    Expired,
}

/// A tracked job in the marketplace.
#[derive(Clone, Debug, PartialEq)]
pub struct MarketplaceJob {
    /// Job identifier.
    pub job_id: [u8; 32],
    /// Current lifecycle state.
    pub state: JobState,
    /// Poster passport ID.
    pub poster_passport_id: u256,
    /// Assigned agent passport ID (set on assignment).
    pub assigned_agent: Option<u256>,
    /// Budget deposited into escrow.
    pub budget: u256,
    /// Deadline block for delivery.
    pub deadline_block: u64,
    /// Hiring model used.
    pub hiring_model: HiringModel,
    /// Minimum reputation required for the job.
    pub min_reputation: f64,
    /// Minimum passport tier required.
    pub min_tier: PassportTier,
    /// Domain for the job.
    pub domain: String,
    /// Required capabilities bitmask.
    pub required_capabilities: u64,
    /// Result hash submitted by the agent.
    pub result_hash: Option<[u8; 32]>,
    /// Quality score from gate validation.
    pub quality_score: Option<f64>,
    /// Block at which job was posted.
    pub posted_at_block: u64,
    /// Payment determined by the hiring model.
    pub payment: Option<f64>,
}

// ---------------------------------------------------------------------------
// Escrow
// ---------------------------------------------------------------------------

/// Escrow entry tracking funds locked for a job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowEntry {
    /// Job identifier.
    pub job_id: [u8; 32],
    /// Depositor (poster) passport ID.
    pub depositor: u256,
    /// Amount held in escrow.
    pub amount: u256,
    /// Recipient (assigned agent) passport ID.
    pub recipient: Option<u256>,
    /// Whether the escrow has been released.
    pub released: bool,
    /// Whether the escrow is disputed.
    pub disputed: bool,
}

/// Configuration for the marketplace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketplaceConfig {
    /// Direct-hire premium multiplier (default 1.5).
    pub direct_hire_premium: f64,
    /// Platform fee fraction (default 0.02 = 2%).
    pub platform_fee_fraction: f64,
    /// Default challenge window in blocks for disputes.
    pub dispute_challenge_window: u64,
    /// Optimistic settlement window in blocks (72h at ~12s/block).
    pub optimistic_window_blocks: u64,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            direct_hire_premium: 1.5,
            platform_fee_fraction: 0.02,
            dispute_challenge_window: 100,
            optimistic_window_blocks: 21600, // ~72h at 12s blocks
        }
    }
}

/// Marketplace take-rate bands expressed in integer cents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TakeRateSchedule {
    /// Lifetime revenue that is exempt from platform take, in cents.
    pub free_band_cents: u64,
    /// Fraction charged only on revenue above the free band.
    pub above_band_rate: f64,
}

impl Default for TakeRateSchedule {
    fn default() -> Self {
        Self {
            free_band_cents: 100_000_000,
            above_band_rate: 0.12,
        }
    }
}

impl TakeRateSchedule {
    /// Compute the take on a new receipt. Revenue below the lifetime free band
    /// is never charged retroactively.
    #[must_use]
    pub fn compute_take_rate(&self, lifetime_revenue_cents: u64, new_revenue_cents: u64) -> u64 {
        let before = lifetime_revenue_cents.saturating_sub(self.free_band_cents);
        let after = lifetime_revenue_cents
            .saturating_add(new_revenue_cents)
            .saturating_sub(self.free_band_cents);
        let newly_taxable_cents = after.saturating_sub(before);
        let basis_points = if self.above_band_rate.is_finite() {
            (self.above_band_rate.clamp(0.0, 1.0) * 10_000.0).round() as u128
        } else {
            0
        };
        let take = u128::from(newly_taxable_cents).saturating_mul(basis_points) / 10_000;
        u64::try_from(take).unwrap_or(u64::MAX)
    }
}

/// Per-creator lifetime marketplace revenue totals, in integer cents.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatorRevenue {
    /// Gross marketplace revenue received across the creator's lifetime.
    pub lifetime_revenue_cents: u64,
    /// Cumulative platform take charged to the creator.
    pub take_paid_cents: u64,
    /// Gross lifetime revenue minus cumulative platform take.
    pub net_revenue_cents: u64,
}

impl CreatorRevenue {
    /// Record a receipt and return the take charged on that receipt.
    pub fn record_revenue(&mut self, new_revenue_cents: u64, schedule: &TakeRateSchedule) -> u64 {
        let take = schedule.compute_take_rate(self.lifetime_revenue_cents, new_revenue_cents);
        self.lifetime_revenue_cents = self
            .lifetime_revenue_cents
            .saturating_add(new_revenue_cents);
        self.take_paid_cents = self.take_paid_cents.saturating_add(take);
        self.net_revenue_cents = self
            .lifetime_revenue_cents
            .saturating_sub(self.take_paid_cents);
        take
    }
}

/// One parent-to-child edge in an artifact fork chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkChainEntry {
    /// Immediate parent artifact reference.
    pub original_ref: String,
    /// Child artifact reference created by this fork.
    pub fork_ref: String,
    /// Passport ID of the forking creator.
    pub forked_by: u256,
    /// Block or monotonic ledger position at which the fork was recorded.
    pub forked_at: u64,
    /// Creator-authored summary of changes relative to the parent.
    pub changes_summary: String,
}

/// The Spore marketplace: manages jobs, escrow, and dispute resolution.
#[derive(Debug, Clone)]
pub struct Marketplace {
    /// Configuration.
    pub config: MarketplaceConfig,
    /// Active jobs by ID.
    jobs: HashMap<[u8; 32], MarketplaceJob>,
    /// Escrow entries by job ID.
    escrow: HashMap<[u8; 32], EscrowEntry>,
    /// Active disputes by job ID.
    disputes: HashMap<[u8; 32], DisputeResolution>,
    /// Current block number.
    current_block: u64,
    /// Collected bids per job (for auction model).
    bids: HashMap<[u8; 32], Vec<SparrowBid>>,
    /// Fork lineage keyed by the child artifact reference.
    fork_chains: HashMap<String, ForkChainEntry>,
}

impl Default for Marketplace {
    fn default() -> Self {
        Self::empty(MarketplaceConfig::default())
    }
}

impl Marketplace {
    fn empty(config: MarketplaceConfig) -> Self {
        Self {
            config,
            jobs: HashMap::new(),
            escrow: HashMap::new(),
            disputes: HashMap::new(),
            current_block: 0,
            bids: HashMap::new(),
            fork_chains: HashMap::new(),
        }
    }

    /// Create a new marketplace.
    #[must_use]
    pub fn new(config: MarketplaceConfig) -> Self {
        Self::empty(config)
    }

    /// Set the current block number.
    pub fn set_block(&mut self, block: u64) {
        self.current_block = block;
    }

    /// Current block number.
    #[must_use]
    pub fn current_block(&self) -> u64 {
        self.current_block
    }

    /// Number of tracked jobs.
    #[must_use]
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }

    /// Get a job by ID.
    #[must_use]
    pub fn get_job(&self, job_id: &[u8; 32]) -> Option<&MarketplaceJob> {
        self.jobs.get(job_id)
    }

    /// Get an escrow entry by job ID.
    #[must_use]
    pub fn get_escrow(&self, job_id: &[u8; 32]) -> Option<&EscrowEntry> {
        self.escrow.get(job_id)
    }

    /// Get a dispute by job ID.
    #[must_use]
    pub fn get_dispute(&self, job_id: &[u8; 32]) -> Option<&DisputeResolution> {
        self.disputes.get(job_id)
    }

    /// Record a new artifact fork while rejecting ambiguous or cyclic lineage.
    pub fn fork_artifact(
        &mut self,
        original_ref: impl Into<String>,
        new_ref: impl Into<String>,
        forker: u256,
        block: u64,
        summary: impl Into<String>,
    ) -> Result<ForkChainEntry, MarketplaceError> {
        let original_ref = original_ref.into();
        let new_ref = new_ref.into();
        if original_ref.trim().is_empty()
            || new_ref.trim().is_empty()
            || original_ref == new_ref
            || self.fork_chains.contains_key(&new_ref)
        {
            return Err(MarketplaceError::InvalidForkLineage);
        }

        let mut ancestor = original_ref.as_str();
        let mut visited = std::collections::HashSet::new();
        loop {
            if ancestor == new_ref {
                return Err(MarketplaceError::InvalidForkLineage);
            }
            if !visited.insert(ancestor) {
                return Err(MarketplaceError::InvalidForkLineage);
            }
            let Some(entry) = self.fork_chains.get(ancestor) else {
                break;
            };
            ancestor = &entry.original_ref;
        }

        let entry = ForkChainEntry {
            original_ref,
            fork_ref: new_ref.clone(),
            forked_by: forker,
            forked_at: block,
            changes_summary: summary.into(),
        };
        self.fork_chains.insert(new_ref, entry.clone());
        Ok(entry)
    }

    /// Return a root-to-leaf list of parent edges for an artifact reference.
    #[must_use]
    pub fn fork_chain(&self, artifact_ref: &str) -> Vec<ForkChainEntry> {
        let mut chain = Vec::new();
        let mut current = artifact_ref;
        let mut visited = std::collections::HashSet::new();
        while visited.insert(current) {
            let Some(entry) = self.fork_chains.get(current) else {
                break;
            };
            chain.push(entry.clone());
            current = &entry.original_ref;
        }
        chain.reverse();
        chain
    }

    // -----------------------------------------------------------------------
    // Job creation
    // -----------------------------------------------------------------------

    /// Post a new job to the marketplace. Deposits budget into escrow.
    ///
    /// # Errors
    ///
    /// Returns an error if a job with the same ID already exists.
    pub fn create_job(&mut self, posting: &SporeJobPosting) -> Result<(), MarketplaceError> {
        if self.jobs.contains_key(&posting.job_id) {
            return Err(MarketplaceError::DuplicateJob);
        }

        let job = MarketplaceJob {
            job_id: posting.job_id,
            state: JobState::Posted,
            poster_passport_id: posting.poster_passport_id,
            assigned_agent: None,
            budget: posting.budget,
            deadline_block: posting.deadline_block,
            hiring_model: posting.hiring_model.clone(),
            min_reputation: posting.min_reputation,
            min_tier: posting.min_tier,
            domain: posting.domain.clone(),
            required_capabilities: posting.required_capabilities,
            result_hash: None,
            quality_score: None,
            posted_at_block: self.current_block,
            payment: None,
        };

        // Deposit into escrow.
        let escrow = EscrowEntry {
            job_id: posting.job_id,
            depositor: posting.poster_passport_id,
            amount: posting.budget,
            recipient: None,
            released: false,
            disputed: false,
        };

        self.jobs.insert(posting.job_id, job);
        self.escrow.insert(posting.job_id, escrow);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Bid submission (for auction model)
    // -----------------------------------------------------------------------

    /// Submit a bid for a posted job (used by BlindAuction hiring model).
    ///
    /// # Errors
    ///
    /// Returns an error if the job doesn't exist or isn't in Posted state.
    pub fn submit_bid(
        &mut self,
        job_id: &[u8; 32],
        bid: SparrowBid,
    ) -> Result<(), MarketplaceError> {
        let job = self.jobs.get(job_id).ok_or(MarketplaceError::NotFound)?;

        if job.state != JobState::Posted {
            return Err(MarketplaceError::InvalidState {
                expected: "Posted".to_string(),
                actual: format!("{:?}", job.state),
            });
        }

        self.bids.entry(*job_id).or_default().push(bid);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Assignment (3 hiring models)
    // -----------------------------------------------------------------------

    /// Assign a job using the RandomVRF model (power-of-two-choices).
    ///
    /// Picks two candidates from the pool and selects the one with higher
    /// reputation. The payment equals the full budget.
    ///
    /// # Errors
    ///
    /// Returns an error if the job doesn't exist, isn't Posted, or the
    /// pool is empty.
    pub fn assign_random_vrf(
        &mut self,
        job_id: &[u8; 32],
        pool: &[SparrowBid],
    ) -> Result<AssignmentResult, MarketplaceError> {
        let job = self.jobs.get(job_id).ok_or(MarketplaceError::NotFound)?;
        if job.state != JobState::Posted {
            return Err(MarketplaceError::InvalidState {
                expected: "Posted".to_string(),
                actual: format!("{:?}", job.state),
            });
        }
        if pool.is_empty() {
            return Err(MarketplaceError::EmptyPool);
        }

        let bounty = self.job_to_bounty(job);
        let decision = crate::identity_economy_markets::dispatch_random_vrf(pool, &bounty)
            .ok_or(MarketplaceError::EmptyPool)?;

        self.finalize_assignment(job_id, decision.winner, decision.payment)
    }

    /// Assign a job using the BlindAuction model (Vickrey second-price).
    ///
    /// Uses previously submitted bids. The winner pays the second-best
    /// reputation-adjusted score (truthful bidding).
    ///
    /// # Errors
    ///
    /// Returns an error if the job doesn't exist, isn't Posted, or there
    /// are no bids.
    pub fn assign_blind_auction(
        &mut self,
        job_id: &[u8; 32],
    ) -> Result<AssignmentResult, MarketplaceError> {
        let job = self.jobs.get(job_id).ok_or(MarketplaceError::NotFound)?;
        if job.state != JobState::Posted {
            return Err(MarketplaceError::InvalidState {
                expected: "Posted".to_string(),
                actual: format!("{:?}", job.state),
            });
        }

        let bids = self.bids.get(job_id).ok_or(MarketplaceError::EmptyPool)?;
        if bids.is_empty() {
            return Err(MarketplaceError::EmptyPool);
        }

        let bounty = self.job_to_bounty(job);
        let decision = crate::identity_economy_markets::dispatch_blind_auction(bids, &bounty)
            .ok_or(MarketplaceError::EmptyPool)?;

        self.finalize_assignment(job_id, decision.winner, decision.payment)
    }

    /// Assign a job using DirectHire model.
    ///
    /// The poster specifies the target agent directly. A 1.5x premium
    /// is applied. Restricted to Protocol/Sovereign tier agents.
    ///
    /// # Errors
    ///
    /// Returns an error if the job doesn't exist, isn't Posted, or the
    /// agent tier is below Sovereign.
    pub fn assign_direct_hire(
        &mut self,
        job_id: &[u8; 32],
        target_passport_id: u256,
        agent_tier: PassportTier,
        repeat_count: u32,
    ) -> Result<AssignmentResult, MarketplaceError> {
        let job = self.jobs.get(job_id).ok_or(MarketplaceError::NotFound)?;
        if job.state != JobState::Posted {
            return Err(MarketplaceError::InvalidState {
                expected: "Posted".to_string(),
                actual: format!("{:?}", job.state),
            });
        }

        // Direct hire restricted to Protocol/Sovereign tier
        if !agent_tier.has_privilege(PassportTier::Sovereign) {
            return Err(MarketplaceError::InsufficientTier {
                required: PassportTier::Sovereign,
                actual: agent_tier,
            });
        }

        let base_fee = job.budget as f64 * self.config.direct_hire_premium;
        let decision = crate::identity_economy_markets::dispatch_direct_hire(
            target_passport_id,
            base_fee,
            repeat_count,
        );

        self.finalize_assignment(job_id, decision.winner, decision.payment)
    }

    // -----------------------------------------------------------------------
    // Job progression
    // -----------------------------------------------------------------------

    /// Mark a job as in-progress (agent starts work).
    ///
    /// # Errors
    ///
    /// Returns an error if the job doesn't exist or isn't Assigned.
    pub fn start_work(&mut self, job_id: &[u8; 32]) -> Result<(), MarketplaceError> {
        let job = self
            .jobs
            .get_mut(job_id)
            .ok_or(MarketplaceError::NotFound)?;
        if job.state != JobState::Assigned {
            return Err(MarketplaceError::InvalidState {
                expected: "Assigned".to_string(),
                actual: format!("{:?}", job.state),
            });
        }
        job.state = JobState::InProgress;
        Ok(())
    }

    /// Submit a result for a job.
    ///
    /// # Errors
    ///
    /// Returns an error if the job isn't InProgress.
    pub fn submit_result(
        &mut self,
        job_id: &[u8; 32],
        result_hash: [u8; 32],
        quality_score: f64,
    ) -> Result<(), MarketplaceError> {
        let job = self
            .jobs
            .get_mut(job_id)
            .ok_or(MarketplaceError::NotFound)?;
        if job.state != JobState::InProgress {
            return Err(MarketplaceError::InvalidState {
                expected: "InProgress".to_string(),
                actual: format!("{:?}", job.state),
            });
        }
        job.result_hash = Some(result_hash);
        job.quality_score = Some(quality_score);
        job.state = JobState::Submitted;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Settlement
    // -----------------------------------------------------------------------

    /// Settle a job: release escrow to the assigned agent.
    ///
    /// # Errors
    ///
    /// Returns an error if the job isn't Submitted.
    pub fn settle_job(&mut self, job_id: &[u8; 32]) -> Result<SettlementResult, MarketplaceError> {
        let job = self.jobs.get(job_id).ok_or(MarketplaceError::NotFound)?;
        if job.state != JobState::Submitted {
            return Err(MarketplaceError::InvalidState {
                expected: "Submitted".to_string(),
                actual: format!("{:?}", job.state),
            });
        }

        let agent = job
            .assigned_agent
            .ok_or(MarketplaceError::NoAssignedAgent)?;
        let domain = job.domain.clone();
        let quality_score = job
            .quality_score
            .filter(|score| score.is_finite())
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let payment = job.payment.unwrap_or(job.budget as f64);
        let platform_fee = (payment * self.config.platform_fee_fraction) as u256;
        let agent_payment = (payment as u256).saturating_sub(platform_fee);

        // Release escrow
        if let Some(escrow) = self.escrow.get_mut(job_id) {
            escrow.released = true;
        }

        let job = self
            .jobs
            .get_mut(job_id)
            .ok_or(MarketplaceError::NotFound)?;
        job.state = JobState::Settled;

        Ok(SettlementResult {
            agent_payment,
            platform_fee,
            agent_passport_id: agent,
            quality_score,
            reputation_effects: vec![ReputationEffect {
                passport_id: agent,
                domain,
                delta: quality_score,
            }],
        })
    }

    // -----------------------------------------------------------------------
    // Expiration
    // -----------------------------------------------------------------------

    /// Expire a job past its deadline. Refunds escrow to the poster.
    ///
    /// # Errors
    ///
    /// Returns an error if the job doesn't exist or deadline hasn't passed.
    pub fn expire_job(&mut self, job_id: &[u8; 32]) -> Result<ExpirationResult, MarketplaceError> {
        let job = self.jobs.get(job_id).ok_or(MarketplaceError::NotFound)?;

        if self.current_block < job.deadline_block {
            return Err(MarketplaceError::DeadlineNotReached {
                current: self.current_block,
                deadline: job.deadline_block,
            });
        }

        if job.state == JobState::Settled || job.state == JobState::Expired {
            return Err(MarketplaceError::InvalidState {
                expected: "not Settled/Expired".to_string(),
                actual: format!("{:?}", job.state),
            });
        }

        let refund = job.budget;
        let poster = job.poster_passport_id;
        let assigned_agent = job.assigned_agent;
        let domain = job.domain.clone();

        // Refund escrow
        if let Some(escrow) = self.escrow.get_mut(job_id) {
            escrow.released = true;
        }

        let job = self
            .jobs
            .get_mut(job_id)
            .ok_or(MarketplaceError::NotFound)?;
        job.state = JobState::Expired;

        Ok(ExpirationResult {
            refund,
            poster_passport_id: poster,
            reputation_effects: assigned_agent
                .map(|passport_id| ReputationEffect {
                    passport_id,
                    domain,
                    delta: -0.5,
                })
                .into_iter()
                .collect(),
        })
    }

    // -----------------------------------------------------------------------
    // Disputes (4-level escalation)
    // -----------------------------------------------------------------------

    /// Open a dispute on a submitted job.
    ///
    /// Starts at BondEscalation round 1. The escrow is locked until
    /// the dispute resolves.
    ///
    /// # Errors
    ///
    /// Returns an error if the job isn't Submitted.
    pub fn open_dispute(
        &mut self,
        job_id: &[u8; 32],
        challenger: u256,
        challenger_bond: u256,
    ) -> Result<(), MarketplaceError> {
        let job = self
            .jobs
            .get_mut(job_id)
            .ok_or(MarketplaceError::NotFound)?;
        if job.state != JobState::Submitted {
            return Err(MarketplaceError::InvalidState {
                expected: "Submitted".to_string(),
                actual: format!("{:?}", job.state),
            });
        }

        let defender = job
            .assigned_agent
            .ok_or(MarketplaceError::NoAssignedAgent)?;

        let dispute = DisputeResolution {
            entry_hash: job.result_hash.unwrap_or([0; 32]),
            challenger,
            defender,
            current_level: DisputeLevel::BondEscalation { round: 1 },
            challenger_bond,
            defender_bond: 0,
            jury: None,
            deadline_block: self.current_block + self.config.dispute_challenge_window,
        };

        if let Some(escrow) = self.escrow.get_mut(job_id) {
            escrow.disputed = true;
        }

        job.state = JobState::Disputed;
        self.disputes.insert(*job_id, dispute);

        Ok(())
    }

    /// Escalate a dispute to the next level.
    ///
    /// # Errors
    ///
    /// Returns an error if no active dispute exists.
    pub fn escalate_dispute(
        &mut self,
        job_id: &[u8; 32],
    ) -> Result<&DisputeLevel, MarketplaceError> {
        let dispute = self
            .disputes
            .get_mut(job_id)
            .ok_or(MarketplaceError::NoDispute)?;

        dispute.current_level = match &dispute.current_level {
            DisputeLevel::BondEscalation { round } => {
                if *round >= 3 {
                    DisputeLevel::PeerJury {
                        votes_for: 0,
                        votes_against: 0,
                    }
                } else {
                    DisputeLevel::BondEscalation { round: round + 1 }
                }
            }
            DisputeLevel::PeerJury { .. } => DisputeLevel::GovernanceVote {
                proposal_id: *job_id,
            },
            level => {
                return Err(MarketplaceError::InvalidState {
                    expected: "escalatable dispute level".to_string(),
                    actual: format!("{:?}", level),
                });
            }
        };

        dispute.deadline_block = self.current_block + self.config.dispute_challenge_window;
        Ok(&dispute.current_level)
    }

    /// Resolve a dispute with a final outcome.
    ///
    /// # Errors
    ///
    /// Returns an error if no active dispute exists.
    pub fn resolve_dispute(
        &mut self,
        job_id: &[u8; 32],
        winner: u256,
        outcome: DisputeOutcome,
    ) -> Result<DisputeSettlement, MarketplaceError> {
        let dispute = self
            .disputes
            .get_mut(job_id)
            .ok_or(MarketplaceError::NoDispute)?;

        dispute.current_level = DisputeLevel::Resolved {
            winner,
            outcome: outcome.clone(),
        };

        let job = self.jobs.get(job_id).ok_or(MarketplaceError::NotFound)?;

        let (escrow_to, refund_amount) = match &outcome {
            DisputeOutcome::EntryUpheld => {
                // Agent wins: release to agent.
                (job.assigned_agent.unwrap_or(0), job.budget)
            }
            DisputeOutcome::EntryRemoved | DisputeOutcome::EntryAmended { .. } => {
                // Challenger wins: refund to poster.
                (job.poster_passport_id, job.budget)
            }
        };

        if let Some(escrow) = self.escrow.get_mut(job_id) {
            escrow.released = true;
            escrow.disputed = false;
        }

        Ok(DisputeSettlement {
            winner,
            outcome,
            escrow_recipient: escrow_to,
            amount: refund_amount,
            challenger_bond_returned: dispute.challenger_bond,
            defender_bond_returned: dispute.defender_bond,
        })
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Convert a job to a BountySpec for dispatch functions.
    fn job_to_bounty(&self, job: &MarketplaceJob) -> BountySpec {
        BountySpec {
            job_id: job.job_id,
            title: String::new(),
            description: String::new(),
            required_capabilities: job.required_capabilities,
            required_domain: job.domain.clone(),
            min_reputation: job.min_reputation,
            max_budget_usdc: job.budget as u64,
            deadline: job.deadline_block,
            hiring_model: job.hiring_model.clone(),
            evaluation_criteria: Vec::new(),
            quality_threshold: 0.0,
            poster_passport_id: job.poster_passport_id,
        }
    }

    /// Finalize an assignment: transition job state and update escrow.
    fn finalize_assignment(
        &mut self,
        job_id: &[u8; 32],
        winner: u256,
        payment: f64,
    ) -> Result<AssignmentResult, MarketplaceError> {
        let job = self
            .jobs
            .get_mut(job_id)
            .ok_or(MarketplaceError::NotFound)?;
        job.state = JobState::Assigned;
        job.assigned_agent = Some(winner);
        job.payment = Some(payment);

        if let Some(escrow) = self.escrow.get_mut(job_id) {
            escrow.recipient = Some(winner);
        }

        Ok(AssignmentResult {
            agent_passport_id: winner,
            payment,
            hiring_model: job.hiring_model.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Result of assigning an agent to a job.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentResult {
    /// Assigned agent passport ID.
    pub agent_passport_id: u256,
    /// Determined payment.
    pub payment: f64,
    /// Hiring model used.
    pub hiring_model: HiringModel,
}

/// Result of settling a job.
#[derive(Debug, Clone, PartialEq)]
pub struct SettlementResult {
    /// Payment to the agent (after platform fee).
    pub agent_payment: u256,
    /// Platform fee collected.
    pub platform_fee: u256,
    /// Agent who received the payment.
    pub agent_passport_id: u256,
    /// Quality score of the submission.
    pub quality_score: f64,
    /// Caller-applied reputation effects produced by settlement.
    pub reputation_effects: Vec<ReputationEffect>,
}

/// A marketplace outcome to apply to the reputation registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReputationEffect {
    /// Passport receiving the effect.
    pub passport_id: u256,
    /// Marketplace job domain.
    pub domain: String,
    /// Signed reputation delta.
    pub delta: f64,
}

/// Result of expiring a job.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpirationResult {
    /// Refunded amount.
    pub refund: u256,
    /// Poster who receives the refund.
    pub poster_passport_id: u256,
    /// Assigned agent's timeout penalty, or empty when never assigned.
    pub reputation_effects: Vec<ReputationEffect>,
}

/// Result of resolving a dispute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeSettlement {
    /// Winner of the dispute.
    pub winner: u256,
    /// Outcome category.
    pub outcome: DisputeOutcome,
    /// Recipient of the escrow.
    pub escrow_recipient: u256,
    /// Amount transferred.
    pub amount: u256,
    /// Challenger bond returned.
    pub challenger_bond_returned: u256,
    /// Defender bond returned.
    pub defender_bond_returned: u256,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from the marketplace.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MarketplaceError {
    /// Job not found.
    #[error("job not found")]
    NotFound,
    /// Duplicate job ID.
    #[error("job with this ID already exists")]
    DuplicateJob,
    /// Invalid state transition.
    #[error("invalid state: expected {expected}, got {actual}")]
    InvalidState {
        /// Expected state.
        expected: String,
        /// Actual state.
        actual: String,
    },
    /// Empty candidate pool.
    #[error("no candidates in the pool")]
    EmptyPool,
    /// No agent assigned to this job.
    #[error("no agent assigned to this job")]
    NoAssignedAgent,
    /// Deadline not reached.
    #[error("deadline not reached: current block {current}, deadline {deadline}")]
    DeadlineNotReached {
        /// Current block.
        current: u64,
        /// Deadline block.
        deadline: u64,
    },
    /// Insufficient passport tier for direct hire.
    #[error("insufficient tier: required {required:?}, got {actual:?}")]
    InsufficientTier {
        /// Required tier.
        required: PassportTier,
        /// Actual tier.
        actual: PassportTier,
    },
    /// No active dispute.
    #[error("no active dispute for this job")]
    NoDispute,
    /// Artifact lineage is empty, duplicated, self-referential, or cyclic.
    #[error("invalid artifact fork lineage")]
    InvalidForkLineage,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::identity_economy_identity::Signature;
    use crate::identity_economy_markets::SparrowBid;

    fn test_posting() -> SporeJobPosting {
        SporeJobPosting {
            job_id: [1; 32],
            domain: "coding".to_string(),
            required_capabilities: 0b11,
            budget: 10_000,
            deadline_block: 500,
            hiring_model: HiringModel::RandomVRF,
            min_reputation: 0.5,
            min_tier: PassportTier::Worker,
            description_cid: "QmTest".to_string(),
            poster_passport_id: 100,
            direct_hire_target: None,
            max_agents: 1,
        }
    }

    fn test_bid(passport: u256, price: u64, rep: f64) -> SparrowBid {
        SparrowBid {
            bidder_passport_id: passport,
            bounty_id: [1; 32],
            price_usdc: price,
            estimated_time: 3600,
            capability_proof: 0b11,
            reputation_snapshot: rep,
            signature: Signature::default(),
        }
    }

    #[test]
    fn take_rate_is_free_below_band_and_non_retroactive_above_it() {
        let schedule = TakeRateSchedule::default();
        assert_eq!(schedule.compute_take_rate(0, 99_900_000), 0);
        assert_eq!(schedule.compute_take_rate(99_900_000, 100_000), 0);
        assert_eq!(schedule.compute_take_rate(100_000_000, 100), 12);
        assert_eq!(schedule.compute_take_rate(99_999_950, 150), 12);

        let mut revenue = CreatorRevenue::default();
        assert_eq!(revenue.record_revenue(100_000_000, &schedule), 0);
        assert_eq!(revenue.record_revenue(100, &schedule), 12);
        assert_eq!(revenue.lifetime_revenue_cents, 100_000_100);
        assert_eq!(revenue.take_paid_cents, 12);
        assert_eq!(revenue.net_revenue_cents, 100_000_088);
    }

    #[test]
    fn fork_chain_returns_three_ancestors_in_root_to_leaf_order() {
        let mut market = Marketplace::default();
        market.fork_artifact("A", "B", 2, 10, "A to B").unwrap();
        market.fork_artifact("B", "C", 3, 20, "B to C").unwrap();
        market.fork_artifact("C", "D", 4, 30, "C to D").unwrap();

        let chain = market.fork_chain("D");
        assert_eq!(
            chain
                .iter()
                .map(|entry| entry.original_ref.as_str())
                .collect::<Vec<_>>(),
            vec!["A", "B", "C"]
        );
        assert_eq!(chain.last().map(|entry| entry.fork_ref.as_str()), Some("D"));
    }

    #[test]
    fn fork_lineage_rejects_duplicates_self_forks_and_cycles() {
        let mut market = Marketplace::default();
        market.fork_artifact("A", "B", 2, 10, "A to B").unwrap();
        assert_eq!(
            market.fork_artifact("A", "B", 3, 20, "duplicate"),
            Err(MarketplaceError::InvalidForkLineage)
        );
        assert_eq!(
            market.fork_artifact("A", "A", 3, 20, "self"),
            Err(MarketplaceError::InvalidForkLineage)
        );
        assert_eq!(
            market.fork_artifact("B", "A", 3, 20, "cycle"),
            Err(MarketplaceError::InvalidForkLineage)
        );
    }

    #[test]
    fn create_job_deposits_escrow() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();

        market.create_job(&posting).unwrap();

        let job = market.get_job(&posting.job_id).unwrap();
        assert_eq!(job.state, JobState::Posted);
        assert_eq!(job.budget, 10_000);

        let escrow = market.get_escrow(&posting.job_id).unwrap();
        assert_eq!(escrow.amount, 10_000);
        assert!(!escrow.released);
    }

    #[test]
    fn duplicate_job_rejected() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();

        market.create_job(&posting).unwrap();
        let err = market.create_job(&posting).unwrap_err();
        assert!(matches!(err, MarketplaceError::DuplicateJob));
    }

    #[test]
    fn random_vrf_assignment() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();
        market.create_job(&posting).unwrap();

        let pool = vec![
            test_bid(10, 100, 0.9),
            test_bid(20, 100, 0.7),
            test_bid(30, 100, 0.8),
        ];

        let result = market.assign_random_vrf(&posting.job_id, &pool).unwrap();
        assert!(
            pool.iter()
                .any(|b| b.bidder_passport_id == result.agent_passport_id)
        );
        assert!(matches!(result.hiring_model, HiringModel::RandomVRF));

        let job = market.get_job(&posting.job_id).unwrap();
        assert_eq!(job.state, JobState::Assigned);
        assert!(job.assigned_agent.is_some());
    }

    #[test]
    fn blind_auction_assignment() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();
        market.create_job(&posting).unwrap();

        market
            .submit_bid(&posting.job_id, test_bid(10, 100, 0.9))
            .unwrap();
        market
            .submit_bid(&posting.job_id, test_bid(20, 120, 0.8))
            .unwrap();

        let result = market.assign_blind_auction(&posting.job_id).unwrap();
        assert_eq!(result.agent_passport_id, 10); // Higher rep wins
        assert!(result.payment > 0.0);
    }

    #[test]
    fn direct_hire_restricted_to_high_tier() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();
        market.create_job(&posting).unwrap();

        // Worker tier: rejected
        let err = market
            .assign_direct_hire(&posting.job_id, 42, PassportTier::Worker, 0)
            .unwrap_err();
        assert!(matches!(err, MarketplaceError::InsufficientTier { .. }));

        // Sovereign tier: accepted
        let result = market
            .assign_direct_hire(&posting.job_id, 42, PassportTier::Sovereign, 0)
            .unwrap();
        assert_eq!(result.agent_passport_id, 42);
        assert!(result.payment > 10_000.0); // 1.5x premium
    }

    #[test]
    fn full_lifecycle_post_to_settled() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();
        market.create_job(&posting).unwrap();

        // Assign
        let pool = vec![test_bid(42, 100, 0.9)];
        market.assign_random_vrf(&posting.job_id, &pool).unwrap();

        // Start work
        market.start_work(&posting.job_id).unwrap();
        assert_eq!(
            market.get_job(&posting.job_id).unwrap().state,
            JobState::InProgress
        );

        // Submit result
        market
            .submit_result(&posting.job_id, [2; 32], 0.85)
            .unwrap();
        assert_eq!(
            market.get_job(&posting.job_id).unwrap().state,
            JobState::Submitted
        );

        // Settle
        let settlement = market.settle_job(&posting.job_id).unwrap();
        assert_eq!(settlement.agent_passport_id, 42);
        assert!(settlement.agent_payment > 0);
        assert_eq!(
            settlement.reputation_effects,
            vec![ReputationEffect {
                passport_id: 42,
                domain: "coding".to_string(),
                delta: 0.85,
            }]
        );
        assert_eq!(
            market.get_job(&posting.job_id).unwrap().state,
            JobState::Settled
        );
        assert!(market.get_escrow(&posting.job_id).unwrap().released);
    }

    #[test]
    fn expire_job_refunds_escrow() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();
        market.create_job(&posting).unwrap();

        market.set_block(600); // Past deadline of 500
        let result = market.expire_job(&posting.job_id).unwrap();
        assert_eq!(result.refund, 10_000);
        assert_eq!(result.poster_passport_id, 100);
        assert!(result.reputation_effects.is_empty());
        assert_eq!(
            market.get_job(&posting.job_id).unwrap().state,
            JobState::Expired
        );
    }

    #[test]
    fn assigned_job_expiry_emits_exact_negative_reputation_effect() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();
        market.create_job(&posting).unwrap();
        market
            .assign_random_vrf(&posting.job_id, &[test_bid(42, 100, 0.9)])
            .unwrap();
        market.set_block(posting.deadline_block);

        let result = market.expire_job(&posting.job_id).unwrap();
        assert_eq!(
            result.reputation_effects,
            vec![ReputationEffect {
                passport_id: 42,
                domain: "coding".to_string(),
                delta: -0.5,
            }]
        );
    }

    #[test]
    fn expire_before_deadline_fails() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();
        market.create_job(&posting).unwrap();

        market.set_block(100);
        let err = market.expire_job(&posting.job_id).unwrap_err();
        assert!(matches!(err, MarketplaceError::DeadlineNotReached { .. }));
    }

    #[test]
    fn dispute_escalation_through_4_levels() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();
        market.create_job(&posting).unwrap();

        let pool = vec![test_bid(42, 100, 0.9)];
        market.assign_random_vrf(&posting.job_id, &pool).unwrap();
        market.start_work(&posting.job_id).unwrap();
        market
            .submit_result(&posting.job_id, [2; 32], 0.85)
            .unwrap();

        // Open dispute at BondEscalation round 1
        market.open_dispute(&posting.job_id, 99, 500).unwrap();
        let dispute = market.get_dispute(&posting.job_id).unwrap();
        assert!(matches!(
            dispute.current_level,
            DisputeLevel::BondEscalation { round: 1 }
        ));

        // Escalate to round 2
        let level = market.escalate_dispute(&posting.job_id).unwrap();
        assert!(matches!(level, DisputeLevel::BondEscalation { round: 2 }));

        // Escalate to round 3
        market.escalate_dispute(&posting.job_id).unwrap();

        // Escalate to PeerJury
        let level = market.escalate_dispute(&posting.job_id).unwrap();
        assert!(matches!(level, DisputeLevel::PeerJury { .. }));

        // Escalate to GovernanceVote
        let level = market.escalate_dispute(&posting.job_id).unwrap();
        assert!(matches!(level, DisputeLevel::GovernanceVote { .. }));
    }

    #[test]
    fn dispute_resolution_upholds_entry() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();
        market.create_job(&posting).unwrap();

        let pool = vec![test_bid(42, 100, 0.9)];
        market.assign_random_vrf(&posting.job_id, &pool).unwrap();
        market.start_work(&posting.job_id).unwrap();
        market
            .submit_result(&posting.job_id, [2; 32], 0.85)
            .unwrap();
        market.open_dispute(&posting.job_id, 99, 500).unwrap();

        // Resolve in agent's favor
        let settlement = market
            .resolve_dispute(&posting.job_id, 42, DisputeOutcome::EntryUpheld)
            .unwrap();
        assert_eq!(settlement.winner, 42);
        assert_eq!(settlement.escrow_recipient, 42);
        assert_eq!(settlement.amount, 10_000);
    }

    #[test]
    fn dispute_resolution_removes_entry() {
        let mut market = Marketplace::new(MarketplaceConfig::default());
        let posting = test_posting();
        market.create_job(&posting).unwrap();

        let pool = vec![test_bid(42, 100, 0.9)];
        market.assign_random_vrf(&posting.job_id, &pool).unwrap();
        market.start_work(&posting.job_id).unwrap();
        market
            .submit_result(&posting.job_id, [2; 32], 0.85)
            .unwrap();
        market.open_dispute(&posting.job_id, 99, 500).unwrap();

        // Resolve in challenger's favor
        let settlement = market
            .resolve_dispute(&posting.job_id, 99, DisputeOutcome::EntryRemoved)
            .unwrap();
        assert_eq!(settlement.winner, 99);
        assert_eq!(settlement.escrow_recipient, 100); // Poster gets refund
    }
}
