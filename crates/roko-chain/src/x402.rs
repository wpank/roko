//! HTTP 402 micropayment protocol (CHAIN-08).
//!
//! Implements the x402 pay-per-request protocol:
//! 1. Client sends request to agent's HTTP endpoint
//! 2. Agent responds with `402 Payment Required` + `X-Payment-Request` header
//! 3. Client signs an ERC-3009 `transferWithAuthorization` (gasless transfer)
//! 4. Client includes signed authorization in `X-Payment-Authorization` on retry
//! 5. Agent verifies authorization and serves response
//!
//! State channels reduce gas to 2 tx per session via signed balance proofs.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::phase2::{
    Address, AgentPaymentChannel, ChannelParty, ChannelState, DisputeLevel, DisputeOutcome,
    DisputeResolution, u256,
};

// ---------------------------------------------------------------------------
// Payment Request / Response
// ---------------------------------------------------------------------------

/// Payment request sent in the `X-Payment-Request` header (HTTP 402).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentRequest {
    /// Recipient address (agent's payment address).
    pub recipient: Address,
    /// Amount required in KORAI base units.
    pub amount: u256,
    /// Token contract address (KORAI).
    pub token: Address,
    /// Request nonce for replay protection.
    pub nonce: u256,
    /// Block deadline for payment validity.
    pub deadline: u64,
    /// Human-readable reason for the payment.
    pub reason: String,
}

/// ERC-3009 authorization (gasless transfer) sent in `X-Payment-Authorization`.
///
/// `transferWithAuthorization(from, to, value, validAfter, validBefore, nonce, v, r, s)`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentAuthorization {
    /// Payer address (signer of the authorization).
    pub from: Address,
    /// Payee address (agent receiving the payment).
    pub to: Address,
    /// Amount authorized for transfer.
    pub value: u256,
    /// Earliest valid timestamp.
    pub valid_after: u64,
    /// Latest valid timestamp.
    pub valid_before: u64,
    /// Authorization nonce.
    pub nonce: u256,
    /// ECDSA signature v component.
    pub v: u8,
    /// ECDSA signature r component.
    pub r: [u8; 32],
    /// ECDSA signature s component.
    pub s: [u8; 32],
}

/// Verification result for a payment authorization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    /// Authorization is valid.
    Valid,
    /// Authorization expired.
    Expired,
    /// Nonce already used.
    NonceReused,
    /// Amount mismatch.
    AmountMismatch {
        /// Requested amount.
        requested: u256,
        /// Authorized amount.
        authorized: u256,
    },
    /// Recipient mismatch.
    RecipientMismatch,
    /// Signature invalid (stub -- real verification would use ecrecover).
    InvalidSignature,
}

// ---------------------------------------------------------------------------
// Settlement batching
// ---------------------------------------------------------------------------

/// In-memory queue of verified x402 authorizations awaiting batch submission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementBatch {
    /// Verified authorization/request pairs in arrival order.
    pub authorizations: Vec<(PaymentAuthorization, PaymentRequest)>,
    /// Unix timestamp at which the current batch began accumulating.
    pub created_at: u64,
    /// Maximum number of authorizations retained by this queue.
    pub max_batch_size: usize,
    /// Maximum age of a non-empty batch before settlement.
    pub batch_timeout_secs: u64,
}

impl SettlementBatch {
    /// Canonical authorization-count trigger from the payment specification.
    pub const SETTLEMENT_SIZE_TRIGGER: usize = 100;

    /// Create an empty settlement batch.
    #[must_use]
    pub fn new(max_size: usize, timeout_secs: u64) -> Self {
        Self {
            authorizations: Vec::new(),
            created_at: unix_timestamp_secs(),
            max_batch_size: max_size,
            batch_timeout_secs: timeout_secs,
        }
    }

    /// Queue one authorization, returning `false` when the batch is full.
    pub fn add_authorization(
        &mut self,
        auth: PaymentAuthorization,
        request: PaymentRequest,
    ) -> bool {
        if self.authorizations.len() >= self.max_batch_size {
            return false;
        }
        self.authorizations.push((auth, request));
        true
    }

    /// Whether a non-empty batch reached 100 authorizations or its age limit.
    #[must_use]
    pub fn should_settle(&self, current_timestamp: u64) -> bool {
        self.authorizations.len() >= Self::SETTLEMENT_SIZE_TRIGGER
            || (!self.authorizations.is_empty()
                && current_timestamp.saturating_sub(self.created_at) >= self.batch_timeout_secs)
    }

    /// Drain all queued authorizations and begin a fresh batch window.
    pub fn drain(&mut self) -> Vec<(PaymentAuthorization, PaymentRequest)> {
        self.created_at = unix_timestamp_secs();
        std::mem::take(&mut self.authorizations)
    }

    /// Number of queued authorizations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.authorizations.len()
    }

    /// Whether no authorizations are currently queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.authorizations.is_empty()
    }
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ---------------------------------------------------------------------------
// Metered Payment Protocol (MPP)
// ---------------------------------------------------------------------------

/// Lifecycle state of one metered payment session.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MppSessionState {
    /// Usage may still be metered.
    #[default]
    Active,
    /// Final usage has been settled.
    Closed,
    /// The session crossed its deadline and awaits final settlement.
    Expired,
    /// Settlement is paused pending dispute resolution.
    Disputed,
}

/// One session authorized once and metered until close or expiry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MppSession {
    /// Stable session identifier.
    pub session_id: [u8; 32],
    /// Account paying for metered usage.
    pub subscriber: Address,
    /// Account providing the metered service.
    pub provider: Address,
    /// Single ERC-3009 authorization covering this session.
    pub authorization: PaymentAuthorization,
    /// Accumulated metered units.
    pub usage_units: u64,
    /// KORAI base units charged per usage unit.
    pub rate_per_unit: u256,
    /// Maximum session charge authorized by the subscriber.
    pub max_cost: u256,
    /// Unix timestamp when the session began.
    pub started_at: u64,
    /// Unix timestamp after which the session expires.
    pub deadline: u64,
    /// Current lifecycle state.
    pub state: MppSessionState,
}

/// Final accounting result for an MPP session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MppSettlement {
    /// Settled session identifier.
    pub session_id: [u8; 32],
    /// Account that paid for the session.
    pub subscriber: Address,
    /// Account that provided the service.
    pub provider: Address,
    /// Final metered unit count.
    pub usage_units: u64,
    /// Final KORAI amount owed.
    pub amount: u256,
}

/// In-memory owner of active and terminal MPP sessions.
#[derive(Clone, Debug, Default)]
pub struct MppSessionManager {
    sessions: HashMap<[u8; 32], MppSession>,
}

impl MppSessionManager {
    /// Create an empty session manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new active session.
    pub fn open_session(&mut self, session: MppSession) -> Result<(), MppError> {
        if self.sessions.contains_key(&session.session_id) {
            return Err(MppError::DuplicateSession);
        }
        if session.state != MppSessionState::Active {
            return Err(MppError::SessionNotActive);
        }
        self.sessions.insert(session.session_id, session);
        Ok(())
    }

    /// Read a session by id.
    #[must_use]
    pub fn get_session(&self, session_id: &[u8; 32]) -> Option<&MppSession> {
        self.sessions.get(session_id)
    }

    /// Increment metered usage without crossing the authorized maximum cost.
    pub fn meter_usage(&mut self, session_id: &[u8; 32], units: u64) -> Result<(), MppError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or(MppError::SessionNotFound)?;
        if session.state != MppSessionState::Active {
            return Err(MppError::SessionNotActive);
        }
        let usage_units = session
            .usage_units
            .checked_add(units)
            .ok_or(MppError::UsageOverflow)?;
        let amount = u256::from(usage_units)
            .checked_mul(session.rate_per_unit)
            .ok_or(MppError::UsageOverflow)?;
        if amount > session.max_cost {
            return Err(MppError::MaxCostExceeded {
                attempted: amount,
                maximum: session.max_cost,
            });
        }
        session.usage_units = usage_units;
        Ok(())
    }

    /// Compute the final amount and close an active or expired session.
    pub fn settle(&mut self, session_id: &[u8; 32]) -> Result<MppSettlement, MppError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or(MppError::SessionNotFound)?;
        if !matches!(
            session.state,
            MppSessionState::Active | MppSessionState::Expired
        ) {
            return Err(MppError::SessionNotSettleable);
        }
        let amount = u256::from(session.usage_units)
            .checked_mul(session.rate_per_unit)
            .ok_or(MppError::UsageOverflow)?;
        if amount > session.max_cost {
            return Err(MppError::MaxCostExceeded {
                attempted: amount,
                maximum: session.max_cost,
            });
        }
        session.state = MppSessionState::Closed;
        Ok(MppSettlement {
            session_id: session.session_id,
            subscriber: session.subscriber.clone(),
            provider: session.provider.clone(),
            usage_units: session.usage_units,
            amount,
        })
    }

    /// Whether a session is already expired or has crossed its deadline.
    #[must_use]
    pub fn is_expired(&self, session_id: &[u8; 32], current_timestamp: u64) -> bool {
        self.sessions.get(session_id).is_some_and(|session| {
            session.state == MppSessionState::Expired || current_timestamp > session.deadline
        })
    }

    /// Mark every active session past its deadline as expired.
    pub fn expire_stale(&mut self, current_timestamp: u64) -> usize {
        let mut expired = 0;
        for session in self.sessions.values_mut() {
            if session.state == MppSessionState::Active && current_timestamp > session.deadline {
                session.state = MppSessionState::Expired;
                expired += 1;
            }
        }
        expired
    }
}

/// Accounting-layer errors for MPP sessions.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MppError {
    /// No session exists for the supplied id.
    #[error("session not found")]
    SessionNotFound,
    /// A session already exists for the supplied id.
    #[error("session id already exists")]
    DuplicateSession,
    /// The requested operation requires an active session.
    #[error("session is not active")]
    SessionNotActive,
    /// The session lifecycle does not permit settlement.
    #[error("session is not settleable")]
    SessionNotSettleable,
    /// The usage counter or computed amount overflowed.
    #[error("metered usage overflow")]
    UsageOverflow,
    /// The proposed accumulated usage exceeds the session spending cap.
    #[error("session cost {attempted} exceeds maximum {maximum}")]
    MaxCostExceeded {
        /// Cost that would result from the requested usage.
        attempted: u256,
        /// Authorized maximum session cost.
        maximum: u256,
    },
}

// ---------------------------------------------------------------------------
// State Channels
// ---------------------------------------------------------------------------

/// Configuration for the x402 payment system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct X402Config {
    /// Default payment deadline in seconds from now.
    pub default_deadline_secs: u64,
    /// Challenge window for state channel disputes (in blocks).
    pub challenge_window_blocks: u64,
    /// Minimum deposit to open a state channel.
    pub min_channel_deposit: u256,
}

impl Default for X402Config {
    fn default() -> Self {
        Self {
            default_deadline_secs: 3600,
            challenge_window_blocks: 100,
            min_channel_deposit: 1000,
        }
    }
}

/// A state channel for streaming micropayments between two agents.
///
/// Reduces gas to 2 transactions per session (open + close) regardless
/// of how many requests are served.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateChannel {
    /// Channel identifier.
    pub channel_id: [u8; 32],
    /// Party A (typically the payer / client).
    pub party_a: Address,
    /// Party A passport ID.
    pub party_a_passport: u256,
    /// Party B (typically the payee / agent).
    pub party_b: Address,
    /// Party B passport ID.
    pub party_b_passport: u256,
    /// Total deposit from party A.
    pub deposit_a: u256,
    /// Total deposit from party B.
    pub deposit_b: u256,
    /// Current channel nonce (increments with each update).
    pub nonce: u64,
    /// Current balance allocated to party A.
    pub balance_a: u256,
    /// Current balance allocated to party B.
    pub balance_b: u256,
    /// Channel lifecycle state.
    pub state: ChannelLifecycle,
    /// Challenge window in blocks.
    pub challenge_window: u64,
    /// Block at which channel was opened.
    pub opened_at_block: u64,
}

/// State channel lifecycle states.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelLifecycle {
    /// Channel is open and active.
    #[default]
    Open,
    /// Close requested; in challenge period.
    Closing {
        /// Block at which close was requested.
        close_requested_at: u64,
    },
    /// Channel is fully closed and settled.
    Closed,
    /// Under dispute.
    Disputed,
}

/// A signed balance proof for off-chain state channel updates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceProof {
    /// Channel identifier.
    pub channel_id: [u8; 32],
    /// Channel nonce at time of signing.
    pub nonce: u64,
    /// Balance for party A.
    pub balance_a: u256,
    /// Balance for party B.
    pub balance_b: u256,
    /// Signature from party A.
    pub sig_a: Vec<u8>,
    /// Signature from party B.
    pub sig_b: Vec<u8>,
}

// ---------------------------------------------------------------------------
// X402 Protocol Manager
// ---------------------------------------------------------------------------

/// Manages the x402 payment protocol: payment requests, authorization
/// verification, and state channels.
#[derive(Debug, Clone)]
pub struct X402Manager {
    /// Configuration.
    pub config: X402Config,
    /// Used nonces for replay protection.
    used_nonces: HashMap<(Address, u256), bool>,
    /// Active state channels by channel ID.
    channels: HashMap<[u8; 32], StateChannel>,
    /// Current block number.
    current_block: u64,
    /// Current timestamp.
    current_timestamp: u64,
    /// Nonce counter for payment requests.
    next_nonce: u256,
    /// Verified authorizations waiting for batched on-chain settlement.
    pub settlement_batch: SettlementBatch,
}

impl Default for X402Manager {
    fn default() -> Self {
        Self {
            config: X402Config::default(),
            used_nonces: HashMap::new(),
            channels: HashMap::new(),
            current_block: 0,
            current_timestamp: 0,
            next_nonce: 1,
            settlement_batch: SettlementBatch::new(100, 600),
        }
    }
}

impl X402Manager {
    /// Create a new x402 manager.
    #[must_use]
    pub fn new(config: X402Config) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Set the current block and timestamp.
    pub fn set_time(&mut self, block: u64, timestamp: u64) {
        self.current_block = block;
        self.current_timestamp = timestamp;
    }

    // -----------------------------------------------------------------------
    // Payment Requests
    // -----------------------------------------------------------------------

    /// Create a payment request for a specific amount.
    ///
    /// This would be serialized into the `X-Payment-Request` header on an
    /// HTTP 402 response.
    pub fn create_payment_request(
        &mut self,
        recipient: Address,
        amount: u256,
        token: Address,
        reason: &str,
    ) -> PaymentRequest {
        let nonce = self.next_nonce;
        self.next_nonce += 1;

        PaymentRequest {
            recipient,
            amount,
            token,
            nonce,
            deadline: self.current_timestamp + self.config.default_deadline_secs,
            reason: reason.to_string(),
        }
    }

    /// Verify a payment authorization against an expected payment request.
    ///
    /// Checks:
    /// 1. Nonce not reused
    /// 2. Amount >= requested
    /// 3. Recipient matches
    /// 4. Timestamp within valid window
    #[must_use]
    pub fn verify_authorization(
        &self,
        request: &PaymentRequest,
        auth: &PaymentAuthorization,
    ) -> VerificationStatus {
        // Check nonce reuse
        let nonce_key = (auth.from.clone(), auth.nonce);
        if self.used_nonces.contains_key(&nonce_key) {
            return VerificationStatus::NonceReused;
        }

        // Check amount
        if auth.value < request.amount {
            return VerificationStatus::AmountMismatch {
                requested: request.amount,
                authorized: auth.value,
            };
        }

        // Check recipient
        if auth.to != request.recipient {
            return VerificationStatus::RecipientMismatch;
        }

        // Check timestamp validity
        if self.current_timestamp < auth.valid_after || self.current_timestamp > auth.valid_before {
            return VerificationStatus::Expired;
        }

        VerificationStatus::Valid
    }

    /// Record a nonce as used (after successful verification + submission).
    pub fn record_nonce(&mut self, from: &Address, nonce: u256) {
        self.used_nonces.insert((from.clone(), nonce), true);
    }

    /// Verify and queue an authorization for batched settlement.
    ///
    /// Returns `false` for invalid/replayed authorizations or a full batch.
    pub fn queue_for_settlement(
        &mut self,
        auth: PaymentAuthorization,
        request: PaymentRequest,
    ) -> bool {
        if self.verify_authorization(&request, &auth) != VerificationStatus::Valid {
            return false;
        }
        if self.settlement_batch.is_empty() {
            self.settlement_batch.created_at = self.current_timestamp;
        }
        if !self
            .settlement_batch
            .add_authorization(auth.clone(), request)
        {
            return false;
        }
        self.record_nonce(&auth.from, auth.nonce);
        true
    }

    // -----------------------------------------------------------------------
    // State Channels
    // -----------------------------------------------------------------------

    /// Open a new state channel between two parties.
    ///
    /// # Errors
    ///
    /// Returns an error if the deposit is below the minimum.
    pub fn open_channel(
        &mut self,
        channel_id: [u8; 32],
        party_a: Address,
        party_a_passport: u256,
        party_b: Address,
        party_b_passport: u256,
        deposit_a: u256,
        deposit_b: u256,
    ) -> Result<(), X402Error> {
        if deposit_a < self.config.min_channel_deposit {
            return Err(X402Error::InsufficientDeposit {
                provided: deposit_a,
                required: self.config.min_channel_deposit,
            });
        }

        let channel = StateChannel {
            channel_id,
            party_a,
            party_a_passport,
            party_b,
            party_b_passport,
            deposit_a,
            deposit_b,
            nonce: 0,
            balance_a: deposit_a,
            balance_b: deposit_b,
            state: ChannelLifecycle::Open,
            challenge_window: self.config.challenge_window_blocks,
            opened_at_block: self.current_block,
        };

        self.channels.insert(channel_id, channel);
        Ok(())
    }

    /// Update a state channel with a new signed balance proof.
    ///
    /// Both parties must agree (signatures are present but not cryptographically
    /// verified in this stub). The nonce must be strictly increasing.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel doesn't exist, is not open, the nonce
    /// is not increasing, or the balances don't sum to the total deposit.
    pub fn update_channel(&mut self, proof: &BalanceProof) -> Result<(), X402Error> {
        let channel = self
            .channels
            .get_mut(&proof.channel_id)
            .ok_or(X402Error::ChannelNotFound)?;

        if channel.state != ChannelLifecycle::Open {
            return Err(X402Error::ChannelNotOpen);
        }

        if proof.nonce <= channel.nonce {
            return Err(X402Error::InvalidNonce {
                current: channel.nonce,
                provided: proof.nonce,
            });
        }

        // Conservation check: sum of balances must equal total deposits
        let total_deposit = channel.deposit_a + channel.deposit_b;
        let total_balance = proof.balance_a + proof.balance_b;
        if total_balance != total_deposit {
            return Err(X402Error::BalanceMismatch {
                total_deposit,
                total_balance,
            });
        }

        channel.nonce = proof.nonce;
        channel.balance_a = proof.balance_a;
        channel.balance_b = proof.balance_b;

        Ok(())
    }

    /// Request to close a state channel. Starts the challenge period.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel doesn't exist or is not open.
    pub fn request_close(&mut self, channel_id: &[u8; 32]) -> Result<u64, X402Error> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or(X402Error::ChannelNotFound)?;

        if channel.state != ChannelLifecycle::Open {
            return Err(X402Error::ChannelNotOpen);
        }

        let close_deadline = self.current_block + channel.challenge_window;
        channel.state = ChannelLifecycle::Closing {
            close_requested_at: self.current_block,
        };

        Ok(close_deadline)
    }

    /// Finalize channel close after the challenge period.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is not in Closing state or the
    /// challenge period hasn't passed.
    pub fn finalize_close(
        &mut self,
        channel_id: &[u8; 32],
    ) -> Result<ChannelSettlement, X402Error> {
        let channel = self
            .channels
            .get(channel_id)
            .ok_or(X402Error::ChannelNotFound)?;

        let close_requested_at = match channel.state {
            ChannelLifecycle::Closing { close_requested_at } => close_requested_at,
            _ => return Err(X402Error::InvalidState("not in Closing state".to_string())),
        };

        if self.current_block < close_requested_at + channel.challenge_window {
            return Err(X402Error::ChallengePeriodActive {
                remaining: (close_requested_at + channel.challenge_window) - self.current_block,
            });
        }

        let settlement = ChannelSettlement {
            channel_id: *channel_id,
            final_balance_a: channel.balance_a,
            final_balance_b: channel.balance_b,
            nonce: channel.nonce,
        };

        let channel = self.channels.get_mut(channel_id).unwrap();
        channel.state = ChannelLifecycle::Closed;

        Ok(settlement)
    }

    /// Get a state channel by ID.
    #[must_use]
    pub fn get_channel(&self, channel_id: &[u8; 32]) -> Option<&StateChannel> {
        self.channels.get(channel_id)
    }

    /// Number of active channels.
    #[must_use]
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Convert a state channel to the phase2 `AgentPaymentChannel` type.
    #[must_use]
    pub fn to_phase2_channel(channel: &StateChannel) -> AgentPaymentChannel {
        AgentPaymentChannel {
            channel_id: channel.channel_id,
            agent_a: ChannelParty {
                passport_id: channel.party_a_passport,
                address: channel.party_a.clone(),
            },
            agent_b: ChannelParty {
                passport_id: channel.party_b_passport,
                address: channel.party_b.clone(),
            },
            deposit_a: channel.deposit_a,
            deposit_b: channel.deposit_b,
            state: ChannelState {
                nonce: channel.nonce,
                balance_a: channel.balance_a,
                balance_b: channel.balance_b,
                sig_a: [0u8; 64],
                sig_b: [0u8; 64],
            },
            challenge_window: channel.challenge_window,
        }
    }
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Settlement result when closing a state channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelSettlement {
    /// Channel identifier.
    pub channel_id: [u8; 32],
    /// Final balance for party A.
    pub final_balance_a: u256,
    /// Final balance for party B.
    pub final_balance_b: u256,
    /// Final nonce.
    pub nonce: u64,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from the x402 payment system.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum X402Error {
    /// Channel not found.
    #[error("channel not found")]
    ChannelNotFound,
    /// Channel not in open state.
    #[error("channel not open")]
    ChannelNotOpen,
    /// Invalid state.
    #[error("invalid state: {0}")]
    InvalidState(String),
    /// Invalid nonce.
    #[error("invalid nonce: current {current}, provided {provided}")]
    InvalidNonce {
        /// Current nonce.
        current: u64,
        /// Provided nonce.
        provided: u64,
    },
    /// Balance conservation violated.
    #[error("balance mismatch: deposit total {total_deposit}, balance total {total_balance}")]
    BalanceMismatch {
        /// Total deposit.
        total_deposit: u256,
        /// Total balance from proof.
        total_balance: u256,
    },
    /// Insufficient deposit.
    #[error("insufficient deposit: provided {provided}, required {required}")]
    InsufficientDeposit {
        /// Provided deposit.
        provided: u256,
        /// Required minimum deposit.
        required: u256,
    },
    /// Challenge period still active.
    #[error("challenge period active: {remaining} blocks remaining")]
    ChallengePeriodActive {
        /// Remaining blocks.
        remaining: u64,
    },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn manager() -> X402Manager {
        let mut m = X402Manager::new(X402Config {
            default_deadline_secs: 3600,
            challenge_window_blocks: 10,
            min_channel_deposit: 100,
        });
        m.set_time(100, 1_000_000);
        m
    }

    fn payment_pair(nonce: u256) -> (PaymentAuthorization, PaymentRequest) {
        let request = PaymentRequest {
            recipient: "0xAgent".to_string(),
            amount: 500,
            token: "0xKORAI".to_string(),
            nonce,
            deadline: 1_001_000,
            reason: "test".to_string(),
        };
        let auth = PaymentAuthorization {
            from: "0xClient".to_string(),
            to: request.recipient.clone(),
            value: request.amount,
            valid_after: 999_000,
            valid_before: 1_001_000,
            nonce,
            v: 27,
            r: [0u8; 32],
            s: [0u8; 32],
        };
        (auth, request)
    }

    fn mpp_session(id: u8) -> MppSession {
        let (authorization, _) = payment_pair(u256::from(id));
        MppSession {
            session_id: [id; 32],
            subscriber: "0xClient".to_string(),
            provider: "0xAgent".to_string(),
            authorization,
            usage_units: 0,
            rate_per_unit: 10,
            max_cost: 100,
            started_at: 1_000,
            deadline: 2_000,
            state: MppSessionState::Active,
        }
    }

    // -----------------------------------------------------------------------
    // Payment Request / Authorization
    // -----------------------------------------------------------------------

    #[test]
    fn create_payment_request_increments_nonce() {
        let mut mgr = manager();
        let req1 = mgr.create_payment_request(
            "0xAgent".to_string(),
            500,
            "0xKORAI".to_string(),
            "API access",
        );
        let req2 = mgr.create_payment_request(
            "0xAgent".to_string(),
            300,
            "0xKORAI".to_string(),
            "Data query",
        );

        assert_eq!(req1.nonce, 1);
        assert_eq!(req2.nonce, 2);
        assert_eq!(req1.amount, 500);
        assert_eq!(req2.amount, 300);
        assert_eq!(req1.deadline, 1_000_000 + 3600);
    }

    #[test]
    fn verify_valid_authorization() {
        let mgr = manager();

        let request = PaymentRequest {
            recipient: "0xAgent".to_string(),
            amount: 500,
            token: "0xKORAI".to_string(),
            nonce: 1,
            deadline: 1_001_000,
            reason: "test".to_string(),
        };

        let auth = PaymentAuthorization {
            from: "0xClient".to_string(),
            to: "0xAgent".to_string(),
            value: 500,
            valid_after: 999_000,
            valid_before: 1_001_000,
            nonce: 1,
            v: 27,
            r: [0u8; 32],
            s: [0u8; 32],
        };

        assert_eq!(
            mgr.verify_authorization(&request, &auth),
            VerificationStatus::Valid
        );
    }

    #[test]
    fn verify_rejects_amount_mismatch() {
        let mgr = manager();

        let request = PaymentRequest {
            recipient: "0xAgent".to_string(),
            amount: 500,
            token: "0xKORAI".to_string(),
            nonce: 1,
            deadline: 1_001_000,
            reason: "test".to_string(),
        };

        let auth = PaymentAuthorization {
            from: "0xClient".to_string(),
            to: "0xAgent".to_string(),
            value: 200, // Less than requested
            valid_after: 999_000,
            valid_before: 1_001_000,
            nonce: 1,
            v: 27,
            r: [0u8; 32],
            s: [0u8; 32],
        };

        assert!(matches!(
            mgr.verify_authorization(&request, &auth),
            VerificationStatus::AmountMismatch { .. }
        ));
    }

    #[test]
    fn verify_rejects_recipient_mismatch() {
        let mgr = manager();

        let request = PaymentRequest {
            recipient: "0xAgent".to_string(),
            amount: 500,
            token: "0xKORAI".to_string(),
            nonce: 1,
            deadline: 1_001_000,
            reason: "test".to_string(),
        };

        let auth = PaymentAuthorization {
            from: "0xClient".to_string(),
            to: "0xWrongAgent".to_string(), // Wrong recipient
            value: 500,
            valid_after: 999_000,
            valid_before: 1_001_000,
            nonce: 1,
            v: 27,
            r: [0u8; 32],
            s: [0u8; 32],
        };

        assert_eq!(
            mgr.verify_authorization(&request, &auth),
            VerificationStatus::RecipientMismatch
        );
    }

    #[test]
    fn verify_rejects_expired_authorization() {
        let mgr = manager();

        let request = PaymentRequest {
            recipient: "0xAgent".to_string(),
            amount: 500,
            token: "0xKORAI".to_string(),
            nonce: 1,
            deadline: 900_000,
            reason: "test".to_string(),
        };

        let auth = PaymentAuthorization {
            from: "0xClient".to_string(),
            to: "0xAgent".to_string(),
            value: 500,
            valid_after: 999_000,
            valid_before: 999_999, // Before current timestamp of 1_000_000
            nonce: 1,
            v: 27,
            r: [0u8; 32],
            s: [0u8; 32],
        };

        assert_eq!(
            mgr.verify_authorization(&request, &auth),
            VerificationStatus::Expired
        );
    }

    #[test]
    fn nonce_replay_prevention() {
        let mut mgr = manager();

        let request = PaymentRequest {
            recipient: "0xAgent".to_string(),
            amount: 500,
            token: "0xKORAI".to_string(),
            nonce: 1,
            deadline: 1_001_000,
            reason: "test".to_string(),
        };

        let auth = PaymentAuthorization {
            from: "0xClient".to_string(),
            to: "0xAgent".to_string(),
            value: 500,
            valid_after: 999_000,
            valid_before: 1_001_000,
            nonce: 1,
            v: 27,
            r: [0u8; 32],
            s: [0u8; 32],
        };

        // First verification: valid
        assert_eq!(
            mgr.verify_authorization(&request, &auth),
            VerificationStatus::Valid
        );

        // Record the nonce
        mgr.record_nonce(&auth.from, auth.nonce);

        // Second verification with same nonce: rejected
        assert_eq!(
            mgr.verify_authorization(&request, &auth),
            VerificationStatus::NonceReused
        );
    }

    #[test]
    fn settlement_batch_triggers_at_one_hundred_authorizations() {
        let mut batch = SettlementBatch::new(150, 600);
        batch.created_at = 1_000;
        for nonce in 0..99 {
            let (auth, request) = payment_pair(nonce);
            assert!(batch.add_authorization(auth, request));
        }
        assert!(!batch.should_settle(1_100));
        let (auth, request) = payment_pair(99);
        assert!(batch.add_authorization(auth, request));
        assert!(batch.should_settle(1_100));
    }

    #[test]
    fn settlement_batch_triggers_at_timeout_and_drain_resets_contents() {
        let mut batch = SettlementBatch::new(100, 600);
        batch.created_at = 1_000;
        let (auth, request) = payment_pair(1);
        assert!(batch.add_authorization(auth, request));
        assert!(!batch.should_settle(1_599));
        assert!(batch.should_settle(1_600));

        let drained = batch.drain();
        assert_eq!(drained.len(), 1);
        assert!(batch.is_empty());
        assert!(!batch.should_settle(u64::MAX));
    }

    #[test]
    fn settlement_batch_rejects_over_capacity_and_manager_replays() {
        let mut batch = SettlementBatch::new(1, 600);
        let (auth, request) = payment_pair(1);
        assert!(batch.add_authorization(auth, request));
        let (auth, request) = payment_pair(2);
        assert!(!batch.add_authorization(auth, request));

        let mut mgr = manager();
        let (auth, request) = payment_pair(7);
        assert!(mgr.queue_for_settlement(auth.clone(), request.clone()));
        assert!(!mgr.queue_for_settlement(auth, request));
        assert_eq!(mgr.settlement_batch.len(), 1);
    }

    #[test]
    fn mpp_meter_usage_and_settlement_compute_exact_amount() {
        let mut sessions = MppSessionManager::new();
        sessions.open_session(mpp_session(1)).unwrap();
        sessions.meter_usage(&[1; 32], 4).unwrap();
        sessions.meter_usage(&[1; 32], 2).unwrap();

        let settlement = sessions.settle(&[1; 32]).unwrap();
        assert_eq!(settlement.usage_units, 6);
        assert_eq!(settlement.amount, 60);
        assert_eq!(
            sessions.get_session(&[1; 32]).unwrap().state,
            MppSessionState::Closed
        );
        assert_eq!(
            sessions.meter_usage(&[1; 32], 1),
            Err(MppError::SessionNotActive)
        );
    }

    #[test]
    fn mpp_rejects_usage_over_max_without_mutating_session() {
        let mut sessions = MppSessionManager::new();
        sessions.open_session(mpp_session(2)).unwrap();

        assert_eq!(
            sessions.meter_usage(&[2; 32], 11),
            Err(MppError::MaxCostExceeded {
                attempted: 110,
                maximum: 100,
            })
        );
        assert_eq!(sessions.get_session(&[2; 32]).unwrap().usage_units, 0);
    }

    #[test]
    fn mpp_expiry_is_detected_marked_and_settleable() {
        let mut sessions = MppSessionManager::new();
        sessions.open_session(mpp_session(3)).unwrap();
        sessions.meter_usage(&[3; 32], 5).unwrap();

        assert!(!sessions.is_expired(&[3; 32], 2_000));
        assert!(sessions.is_expired(&[3; 32], 2_001));
        assert_eq!(sessions.expire_stale(2_001), 1);
        assert_eq!(
            sessions.get_session(&[3; 32]).unwrap().state,
            MppSessionState::Expired
        );
        assert_eq!(sessions.settle(&[3; 32]).unwrap().amount, 50);
    }

    #[test]
    fn mpp_rejects_duplicate_and_missing_sessions() {
        let mut sessions = MppSessionManager::new();
        sessions.open_session(mpp_session(4)).unwrap();
        assert_eq!(
            sessions.open_session(mpp_session(4)),
            Err(MppError::DuplicateSession)
        );
        assert_eq!(
            sessions.meter_usage(&[9; 32], 1),
            Err(MppError::SessionNotFound)
        );
    }

    // -----------------------------------------------------------------------
    // State Channels
    // -----------------------------------------------------------------------

    #[test]
    fn open_and_update_channel() {
        let mut mgr = manager();

        mgr.open_channel(
            [1u8; 32],
            "0xClient".to_string(),
            10,
            "0xAgent".to_string(),
            20,
            1000,
            0,
        )
        .unwrap();

        let channel = mgr.get_channel(&[1u8; 32]).unwrap();
        assert_eq!(channel.balance_a, 1000);
        assert_eq!(channel.balance_b, 0);
        assert_eq!(channel.nonce, 0);
        assert_eq!(channel.state, ChannelLifecycle::Open);

        // Update: transfer 100 from A to B
        let proof = BalanceProof {
            channel_id: [1u8; 32],
            nonce: 1,
            balance_a: 900,
            balance_b: 100,
            sig_a: vec![0u8; 64],
            sig_b: vec![0u8; 64],
        };
        mgr.update_channel(&proof).unwrap();

        let channel = mgr.get_channel(&[1u8; 32]).unwrap();
        assert_eq!(channel.balance_a, 900);
        assert_eq!(channel.balance_b, 100);
        assert_eq!(channel.nonce, 1);
    }

    #[test]
    fn channel_rejects_insufficient_deposit() {
        let mut mgr = manager();

        let err = mgr
            .open_channel(
                [1u8; 32],
                "0xClient".to_string(),
                10,
                "0xAgent".to_string(),
                20,
                50, // Below minimum of 100
                0,
            )
            .unwrap_err();
        assert!(matches!(err, X402Error::InsufficientDeposit { .. }));
    }

    #[test]
    fn channel_rejects_invalid_nonce() {
        let mut mgr = manager();
        mgr.open_channel(
            [1u8; 32],
            "0xClient".to_string(),
            10,
            "0xAgent".to_string(),
            20,
            1000,
            0,
        )
        .unwrap();

        let proof = BalanceProof {
            channel_id: [1u8; 32],
            nonce: 0, // Not increasing
            balance_a: 900,
            balance_b: 100,
            sig_a: vec![0u8; 64],
            sig_b: vec![0u8; 64],
        };
        let err = mgr.update_channel(&proof).unwrap_err();
        assert!(matches!(err, X402Error::InvalidNonce { .. }));
    }

    #[test]
    fn channel_rejects_balance_mismatch() {
        let mut mgr = manager();
        mgr.open_channel(
            [1u8; 32],
            "0xClient".to_string(),
            10,
            "0xAgent".to_string(),
            20,
            1000,
            0,
        )
        .unwrap();

        let proof = BalanceProof {
            channel_id: [1u8; 32],
            nonce: 1,
            balance_a: 900,
            balance_b: 200, // 900 + 200 != 1000
            sig_a: vec![0u8; 64],
            sig_b: vec![0u8; 64],
        };
        let err = mgr.update_channel(&proof).unwrap_err();
        assert!(matches!(err, X402Error::BalanceMismatch { .. }));
    }

    #[test]
    fn channel_close_lifecycle() {
        let mut mgr = manager();
        mgr.open_channel(
            [1u8; 32],
            "0xClient".to_string(),
            10,
            "0xAgent".to_string(),
            20,
            1000,
            0,
        )
        .unwrap();

        // Update balances
        let proof = BalanceProof {
            channel_id: [1u8; 32],
            nonce: 1,
            balance_a: 700,
            balance_b: 300,
            sig_a: vec![0u8; 64],
            sig_b: vec![0u8; 64],
        };
        mgr.update_channel(&proof).unwrap();

        // Request close
        let deadline = mgr.request_close(&[1u8; 32]).unwrap();
        assert_eq!(deadline, 100 + 10); // current_block + challenge_window

        // Can't finalize yet (challenge period active)
        let err = mgr.finalize_close(&[1u8; 32]).unwrap_err();
        assert!(matches!(err, X402Error::ChallengePeriodActive { .. }));

        // Advance past challenge period
        mgr.set_time(111, 2_000_000);

        // Finalize
        let settlement = mgr.finalize_close(&[1u8; 32]).unwrap();
        assert_eq!(settlement.final_balance_a, 700);
        assert_eq!(settlement.final_balance_b, 300);
        assert_eq!(settlement.nonce, 1);

        let channel = mgr.get_channel(&[1u8; 32]).unwrap();
        assert_eq!(channel.state, ChannelLifecycle::Closed);
    }

    #[test]
    fn to_phase2_channel_conversion() {
        let channel = StateChannel {
            channel_id: [1u8; 32],
            party_a: "0xA".to_string(),
            party_a_passport: 10,
            party_b: "0xB".to_string(),
            party_b_passport: 20,
            deposit_a: 1000,
            deposit_b: 500,
            nonce: 5,
            balance_a: 700,
            balance_b: 800,
            state: ChannelLifecycle::Open,
            challenge_window: 100,
            opened_at_block: 0,
        };

        let p2 = X402Manager::to_phase2_channel(&channel);
        assert_eq!(p2.channel_id, [1u8; 32]);
        assert_eq!(p2.agent_a.passport_id, 10);
        assert_eq!(p2.agent_b.passport_id, 20);
        assert_eq!(p2.deposit_a, 1000);
        assert_eq!(p2.deposit_b, 500);
        assert_eq!(p2.state.nonce, 5);
        assert_eq!(p2.state.balance_a, 700);
        assert_eq!(p2.state.balance_b, 800);
    }
}
