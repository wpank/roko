// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/// @title WorkerRegistry — stake bonds + EMA reputation + tiers + decay.
/// @notice Implements the Korai-spec reputation model:
///         R_new = α · O + (1 − α) · R_old    with α = 0.2 (as 200_000 in WAD6)
///         Tiers by threshold (Probation < 350_000 ≤ Standard < 550_000 ≤ Trusted < 800_000 ≤ Elite).
///         Decay: halves toward 0.5 every 30 days of inactivity (applied lazily on access).
contract WorkerRegistry {
    /// @dev Fixed-point scale (6 decimals). `R = 500_000` means reputation 0.5.
    uint256 public constant SCALE = 1_000_000;
    /// @dev EMA alpha = 0.2.
    uint256 public constant ALPHA_NUM = 200_000;
    /// @dev Minimum bond to register as a worker.
    uint256 public constant MIN_BOND = 1_000 ether;
    /// @dev Decay halving period (seconds). 30 days per Korai spec.
    uint256 public constant DECAY_PERIOD = 30 days;

    enum Tier { Unregistered, Probation, Standard, Trusted, Elite }

    /// @dev Slashing reason codes — keep in sync with BountyMarket.
    uint8 public constant SLASH_MISSED_DEADLINE = 1; //  1%
    uint8 public constant SLASH_QUALITY_REJECT = 2;  //  5%
    uint8 public constant SLASH_ABANDONMENT = 3;     // 10%

    struct Worker {
        uint256 bond;
        uint256 reputation;    // scaled by SCALE
        uint64 jobsCompleted;
        uint64 jobsSlashed;
        uint64 lastUpdated;    // timestamp of last reputation update (for decay)
        bool exists;
    }

    IERC20 public immutable stakeToken;
    address public owner;
    /// @notice Addresses allowed to call `updateReputation`/`slash` (BountyMarket,
    ///         ConsortiumValidator, etc.). Set via `setAuthorized`.
    mapping(address => bool) public authorized;
    mapping(address => Worker) private _workers;
    address[] private _registered;

    event WorkerRegistered(address indexed worker, uint256 bond);
    event BondIncreased(address indexed worker, uint256 amount, uint256 newBond);
    event BondDecreased(address indexed worker, uint256 amount, uint256 newBond);
    event ReputationUpdated(address indexed worker, uint256 oldRep, uint256 newRep, bool outcome);
    event WorkerSlashed(address indexed worker, uint8 reasonCode, uint256 amount, uint256 newBond);
    event AuthorizedSet(address indexed caller, bool allowed);

    error InsufficientBond();
    error AlreadyRegistered();
    error NotRegistered();
    error NotAuthorized();
    error BelowMinBond();

    constructor(address stakeToken_) {
        stakeToken = IERC20(stakeToken_);
        owner = msg.sender;
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotAuthorized();
        _;
    }

    modifier onlyAuthorized() {
        if (!authorized[msg.sender]) revert NotAuthorized();
        _;
    }

    function setAuthorized(address caller, bool allowed) external onlyOwner {
        authorized[caller] = allowed;
        emit AuthorizedSet(caller, allowed);
    }

    /// @notice Bond `amount` tokens and register as a worker. Starting reputation = 0.5.
    function register(uint256 amount) external {
        if (_workers[msg.sender].exists) revert AlreadyRegistered();
        if (amount < MIN_BOND) revert InsufficientBond();
        _pull(msg.sender, amount);
        _workers[msg.sender] = Worker({
            bond: amount,
            reputation: SCALE / 2, // 0.5
            jobsCompleted: 0,
            jobsSlashed: 0,
            lastUpdated: uint64(block.timestamp),
            exists: true
        });
        _registered.push(msg.sender);
        emit WorkerRegistered(msg.sender, amount);
    }

    function bond(uint256 amount) external {
        if (!_workers[msg.sender].exists) revert NotRegistered();
        _pull(msg.sender, amount);
        _workers[msg.sender].bond += amount;
        emit BondIncreased(msg.sender, amount, _workers[msg.sender].bond);
    }

    function unbond(uint256 amount) external {
        Worker storage w = _workers[msg.sender];
        if (!w.exists) revert NotRegistered();
        if (w.bond < amount || w.bond - amount < MIN_BOND) revert BelowMinBond();
        w.bond -= amount;
        bool ok = stakeToken.transfer(msg.sender, amount);
        require(ok, "transfer failed");
        emit BondDecreased(msg.sender, amount, w.bond);
    }

    /// @notice Called by BountyMarket/ConsortiumValidator on job outcome.
    ///         `outcome == true` → success, increments jobsCompleted + raises EMA.
    ///         `outcome == false` → failure, raises jobsSlashed + lowers EMA (no bond change here).
    function updateReputation(address worker, bool outcome) external onlyAuthorized {
        Worker storage w = _workers[worker];
        if (!w.exists) revert NotRegistered();
        _applyDecay(w);
        uint256 old = w.reputation;
        uint256 observation = outcome ? SCALE : 0;
        // R_new = α·O + (1-α)·R_old
        w.reputation = (ALPHA_NUM * observation + (SCALE - ALPHA_NUM) * old) / SCALE;
        if (outcome) {
            w.jobsCompleted += 1;
        } else {
            w.jobsSlashed += 1;
        }
        w.lastUpdated = uint64(block.timestamp);
        emit ReputationUpdated(worker, old, w.reputation, outcome);
    }

    /// @notice Slash a worker's bond by `bpsAmount` basis points of current bond.
    function slash(address worker, uint8 reasonCode, uint256 bpsAmount) external onlyAuthorized {
        Worker storage w = _workers[worker];
        if (!w.exists) revert NotRegistered();
        uint256 amount = (w.bond * bpsAmount) / 10_000;
        if (amount > w.bond) amount = w.bond;
        w.bond -= amount;
        // Slashed tokens stay in this contract (to be redirected by owner later).
        emit WorkerSlashed(worker, reasonCode, amount, w.bond);
    }

    /// @notice Idempotent: apply time-based decay to reputation (halves toward 0.5 every 30d).
    ///         Anyone may call; safe for external keepers to poke before reads.
    function decay(address worker) external {
        Worker storage w = _workers[worker];
        if (!w.exists) revert NotRegistered();
        _applyDecay(w);
    }

    /* ------------------------------- views ---------------------------------- */

    function tier(address worker) external view returns (Tier) {
        Worker storage w = _workers[worker];
        if (!w.exists) return Tier.Unregistered;
        uint256 r = _effectiveReputation(w);
        if (r < 350_000) return Tier.Probation;
        if (r < 550_000) return Tier.Standard;
        if (r < 800_000) return Tier.Trusted;
        return Tier.Elite;
    }

    function canAccept(address worker, Tier minTier) external view returns (bool) {
        Worker storage w = _workers[worker];
        if (!w.exists) return false;
        uint256 r = _effectiveReputation(w);
        Tier t;
        if (r < 350_000) t = Tier.Probation;
        else if (r < 550_000) t = Tier.Standard;
        else if (r < 800_000) t = Tier.Trusted;
        else t = Tier.Elite;
        return uint8(t) >= uint8(minTier);
    }

    function getWorker(address worker) external view returns (Worker memory) {
        return _workers[worker];
    }

    function reputationOf(address worker) external view returns (uint256) {
        Worker storage w = _workers[worker];
        if (!w.exists) return 0;
        return _effectiveReputation(w);
    }

    function registeredCount() external view returns (uint256) {
        return _registered.length;
    }

    function registeredAt(uint256 index) external view returns (address) {
        return _registered[index];
    }

    /* ----------------------------- internal --------------------------------- */

    function _pull(address from, uint256 amount) internal {
        bool ok = stakeToken.transferFrom(from, address(this), amount);
        require(ok, "transferFrom failed");
    }

    function _applyDecay(Worker storage w) internal {
        uint64 nowTs = uint64(block.timestamp);
        if (nowTs <= w.lastUpdated) return;
        uint256 elapsed = nowTs - w.lastUpdated;
        uint256 halvings = elapsed / DECAY_PERIOD;
        if (halvings == 0) return;
        uint256 mid = SCALE / 2;
        uint256 r = w.reputation;
        // Move r halfway toward mid per halving (cap at 64 halvings to avoid gas blowup).
        if (halvings > 64) halvings = 64;
        for (uint256 i = 0; i < halvings; i++) {
            if (r > mid) r = mid + (r - mid) / 2;
            else r = mid - (mid - r) / 2;
        }
        w.reputation = r;
        w.lastUpdated = nowTs;
    }

    function _effectiveReputation(Worker storage w) internal view returns (uint256) {
        uint64 nowTs = uint64(block.timestamp);
        if (nowTs <= w.lastUpdated) return w.reputation;
        uint256 elapsed = nowTs - w.lastUpdated;
        uint256 halvings = elapsed / DECAY_PERIOD;
        if (halvings == 0) return w.reputation;
        if (halvings > 64) halvings = 64;
        uint256 mid = SCALE / 2;
        uint256 r = w.reputation;
        for (uint256 i = 0; i < halvings; i++) {
            if (r > mid) r = mid + (r - mid) / 2;
            else r = mid - (mid - r) / 2;
        }
        return r;
    }
}
