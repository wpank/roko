// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { WorkerRegistry } from "./WorkerRegistry.sol";
import { BountyMarket } from "./BountyMarket.sol";

/// @title ConsortiumValidator — 3-agent 2-of-3 validation committee.
/// @notice Committees are assembled from Trusted+ workers via a blockhash-seeded
///         deterministic pick. Each validator calls `vote(jobId, approve)` at most
///         once; when 2 APPROVE or 2 REJECT votes are reached, the committee
///         invokes `BountyMarket.resolve` with the majority decision.
contract ConsortiumValidator {
    WorkerRegistry public immutable workerRegistry;
    BountyMarket public immutable market;
    uint256 public constant COMMITTEE_SIZE = 3;

    struct Committee {
        address[3] members;
        uint8 approves;
        uint8 rejects;
        bool tallied;
        bool exists;
        mapping(address => bool) voted;
    }

    mapping(uint256 => Committee) private _committees;

    event CommitteeAssembled(uint256 indexed jobId, address a, address b, address c);
    event VoteCast(uint256 indexed jobId, address indexed validator, bool approve);
    event Tallied(uint256 indexed jobId, bool accepted);

    error AlreadyAssembled();
    error NotAssembled();
    error NotAMember();
    error AlreadyVoted();
    error AlreadyTallied();
    error InsufficientTrustedWorkers();

    constructor(address workerRegistry_, address market_) {
        workerRegistry = WorkerRegistry(workerRegistry_);
        market = BountyMarket(market_);
    }

    /// @notice Deterministically pick 3 Trusted+ workers for `jobId`. Anyone may call.
    function assembleCommittee(uint256 jobId) external {
        Committee storage c = _committees[jobId];
        if (c.exists) revert AlreadyAssembled();
        // Enumerate Trusted+ workers from the registry.
        uint256 total = workerRegistry.registeredCount();
        address[] memory candidates = new address[](total);
        uint256 ncand;
        for (uint256 i = 0; i < total; i++) {
            address w = workerRegistry.registeredAt(i);
            if (workerRegistry.canAccept(w, WorkerRegistry.Tier.Trusted)) {
                candidates[ncand++] = w;
            }
        }
        if (ncand < COMMITTEE_SIZE) revert InsufficientTrustedWorkers();
        // Fisher–Yates-ish: draw 3 distinct indices using blockhash as seed.
        bytes32 seed = blockhash(block.number - 1);
        if (seed == bytes32(0)) seed = bytes32(uint256(jobId + block.timestamp));
        address[3] memory chosen;
        uint256 taken;
        for (uint256 round = 0; round < 32 && taken < COMMITTEE_SIZE; round++) {
            uint256 idx = uint256(keccak256(abi.encode(seed, round, jobId))) % ncand;
            address pick = candidates[idx];
            if (pick == address(0)) continue;
            bool dup;
            for (uint256 k = 0; k < taken; k++) {
                if (chosen[k] == pick) { dup = true; break; }
            }
            if (dup) continue;
            chosen[taken++] = pick;
        }
        require(taken == COMMITTEE_SIZE, "assembly failed");
        c.exists = true;
        c.members = chosen;
        emit CommitteeAssembled(jobId, chosen[0], chosen[1], chosen[2]);
    }

    /// @notice A committee member casts their vote. Triggers tally on reaching a majority.
    function vote(uint256 jobId, bool approve) external {
        Committee storage c = _committees[jobId];
        if (!c.exists) revert NotAssembled();
        if (c.tallied) revert AlreadyTallied();
        if (c.voted[msg.sender]) revert AlreadyVoted();
        bool member = (
            c.members[0] == msg.sender
                || c.members[1] == msg.sender
                || c.members[2] == msg.sender
        );
        if (!member) revert NotAMember();
        c.voted[msg.sender] = true;
        if (approve) c.approves += 1;
        else c.rejects += 1;
        emit VoteCast(jobId, msg.sender, approve);
        // Tally on majority.
        if (c.approves >= 2 || c.rejects >= 2) {
            c.tallied = true;
            bool accepted = c.approves >= 2;
            market.resolve(jobId, accepted);
            emit Tallied(jobId, accepted);
        }
    }

    function getMembers(uint256 jobId) external view returns (address[3] memory) {
        return _committees[jobId].members;
    }

    function voteCounts(uint256 jobId) external view returns (uint8 approves, uint8 rejects, bool tallied) {
        Committee storage c = _committees[jobId];
        return (c.approves, c.rejects, c.tallied);
    }
}
