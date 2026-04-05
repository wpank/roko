// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/// @title InsightBoard — on-chain knowledge registry with pheromone curation.
/// @notice Thin wrapper over mirage-rs's `0xA01` InsightEntry precompile. Agents
///         post short content-hashed insights; others `confirm` them, which
///         increments pheromone weight and credits the poster with rewards.
contract InsightBoard {
    struct Insight {
        address poster;
        bytes32 contentHash;
        string uri;
        uint64 postedAt;
        uint64 pheromone;    // count of confirmations
    }

    IERC20 public immutable rewardToken;
    /// @notice Tokens credited per confirmation.
    uint256 public constant REWARD_PER_CONFIRM = 1 ether;

    uint256 public nextInsightId;
    mapping(uint256 => Insight) private _insights;
    mapping(uint256 => mapping(address => bool)) public confirmed;
    mapping(address => uint256) public earningsOf;

    event InsightPosted(uint256 indexed id, address indexed poster, bytes32 contentHash, string uri);
    event InsightConfirmed(uint256 indexed id, address indexed confirmer, uint64 pheromone);
    event EarningsClaimed(address indexed poster, uint256 amount);

    error AlreadyConfirmed();
    error SelfConfirm();
    error NothingToClaim();
    error UnknownInsight();

    constructor(address rewardToken_) {
        rewardToken = IERC20(rewardToken_);
    }

    function post(bytes32 contentHash, string calldata uri) external returns (uint256 id) {
        id = nextInsightId++;
        _insights[id] = Insight({
            poster: msg.sender,
            contentHash: contentHash,
            uri: uri,
            postedAt: uint64(block.timestamp),
            pheromone: 0
        });
        emit InsightPosted(id, msg.sender, contentHash, uri);
    }

    function confirm(uint256 id) external {
        Insight storage i = _insights[id];
        if (i.poster == address(0)) revert UnknownInsight();
        if (i.poster == msg.sender) revert SelfConfirm();
        if (confirmed[id][msg.sender]) revert AlreadyConfirmed();
        confirmed[id][msg.sender] = true;
        i.pheromone += 1;
        earningsOf[i.poster] += REWARD_PER_CONFIRM;
        emit InsightConfirmed(id, msg.sender, i.pheromone);
    }

    function claim() external returns (uint256 amount) {
        amount = earningsOf[msg.sender];
        if (amount == 0) revert NothingToClaim();
        earningsOf[msg.sender] = 0;
        bool ok = rewardToken.transfer(msg.sender, amount);
        require(ok, "transfer failed");
        emit EarningsClaimed(msg.sender, amount);
    }

    function getInsight(uint256 id) external view returns (Insight memory) {
        Insight memory i = _insights[id];
        if (i.poster == address(0)) revert UnknownInsight();
        return i;
    }
}
