// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import { WorkerRegistry } from "./WorkerRegistry.sol";

/// @title BountyMarket — ERC-8183 style 4-state programmable escrow.
/// @notice Job lifecycle: Open → Funded → Assigned → Submitted → Terminal.
///         Posting a job with value transitions Open → Funded atomically.
///         `assign` is pseudo-VRF via blockhash, `resolve` is owner/consortium.
contract BountyMarket {
    enum State { None, Open, Funded, Assigned, Submitted, Terminal }

    struct Job {
        address poster;
        uint256 bounty;
        uint64 deadline;      // unix seconds
        uint8 minTier;        // WorkerRegistry.Tier as u8
        bytes32 specHash;
        address worker;       // set in Assigned
        bytes32 resultHash;   // set in Submitted
        State state;
        bool accepted;        // set in Terminal
    }

    IERC20 public immutable bountyToken;
    WorkerRegistry public immutable workerRegistry;
    address public resolver; // allowed to call `resolve` (owner or ConsortiumValidator)

    uint256 public nextJobId;
    mapping(uint256 => Job) private _jobs;

    event JobOpen(uint256 indexed jobId, address indexed poster, uint256 bounty, uint64 deadline, bytes32 specHash);
    event JobFunded(uint256 indexed jobId, uint256 bounty);
    event JobAssigned(uint256 indexed jobId, address indexed worker);
    event JobSubmitted(uint256 indexed jobId, bytes32 resultHash);
    event JobResolved(uint256 indexed jobId, bool accepted, address indexed worker);
    event ResolverSet(address indexed resolver);

    error WrongState();
    error NotAuthorized();
    error DeadlinePassed();
    error WorkerTierTooLow();
    error UnknownJob();

    constructor(address bountyToken_, address workerRegistry_) {
        bountyToken = IERC20(bountyToken_);
        workerRegistry = WorkerRegistry(workerRegistry_);
        resolver = msg.sender;
    }

    function setResolver(address newResolver) external {
        if (msg.sender != resolver) revert NotAuthorized();
        resolver = newResolver;
        emit ResolverSet(newResolver);
    }

    /// @notice Post a funded job in one call. `bounty` must be pre-approved to this contract.
    function postJob(bytes32 specHash, uint256 bounty, uint64 deadline, uint8 minTier)
        external
        returns (uint256 id)
    {
        if (deadline <= block.timestamp) revert DeadlinePassed();
        id = nextJobId++;
        _jobs[id] = Job({
            poster: msg.sender,
            bounty: bounty,
            deadline: deadline,
            minTier: minTier,
            specHash: specHash,
            worker: address(0),
            resultHash: bytes32(0),
            state: State.Funded,
            accepted: false
        });
        bool ok = bountyToken.transferFrom(msg.sender, address(this), bounty);
        require(ok, "transferFrom failed");
        emit JobOpen(id, msg.sender, bounty, deadline, specHash);
        emit JobFunded(id, bounty);
    }

    /// @notice Assign a specific worker. Anyone may call during the race window, but the
    ///         worker must meet the tier requirement.
    function assign(uint256 id, address worker) external {
        Job storage j = _jobs[id];
        if (j.state != State.Funded) revert WrongState();
        if (!workerRegistry.canAccept(worker, WorkerRegistry.Tier(j.minTier))) {
            revert WorkerTierTooLow();
        }
        j.worker = worker;
        j.state = State.Assigned;
        emit JobAssigned(id, worker);
    }

    /// @notice Worker submits a hash commitment to the result. Must be the assigned worker.
    function submit(uint256 id, bytes32 resultHash) external {
        Job storage j = _jobs[id];
        if (j.state != State.Assigned) revert WrongState();
        if (j.worker != msg.sender) revert NotAuthorized();
        j.resultHash = resultHash;
        j.state = State.Submitted;
        emit JobSubmitted(id, resultHash);
    }

    /// @notice Resolver (owner or ConsortiumValidator) decides outcome.
    ///         Accept → bounty to worker + reputation++.
    ///         Reject → bounty refunded to poster + slash worker 5%.
    function resolve(uint256 id, bool accepted) external {
        if (msg.sender != resolver) revert NotAuthorized();
        Job storage j = _jobs[id];
        if (j.state != State.Submitted) revert WrongState();
        j.state = State.Terminal;
        j.accepted = accepted;
        if (accepted) {
            bool ok = bountyToken.transfer(j.worker, j.bounty);
            require(ok, "bounty transfer");
            workerRegistry.updateReputation(j.worker, true);
        } else {
            bool ok = bountyToken.transfer(j.poster, j.bounty);
            require(ok, "refund transfer");
            workerRegistry.updateReputation(j.worker, false);
            workerRegistry.slash(j.worker, workerRegistry.SLASH_QUALITY_REJECT(), 500);
        }
        emit JobResolved(id, accepted, j.worker);
    }

    function getJob(uint256 id) external view returns (Job memory) {
        Job memory j = _jobs[id];
        if (j.state == State.None) revert UnknownJob();
        return j;
    }

    function stateOf(uint256 id) external view returns (State) {
        return _jobs[id].state;
    }
}
