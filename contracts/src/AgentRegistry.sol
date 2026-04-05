// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title AgentRegistry — minimal ERC-8004-compatible agent identity.
/// @notice Tracks agent address → capabilities + passport hash + last-heartbeat block.
///         A complementary on-chain counterpart to the `0xA09` precompile exposed by
///         mirage-rs (roko-chain stack). Agents call `register` once, then `heartbeat`
///         periodically; `isActive` returns true within a liveness window.
contract AgentRegistry {
    struct Agent {
        string capabilities;
        bytes32 passportHash;
        uint64 registeredAt;
        uint64 lastHeartbeat;
        bool exists;
    }

    /// @notice Blocks after last heartbeat before an agent is considered inactive.
    uint64 public constant LIVENESS_WINDOW = 200;

    mapping(address => Agent) private _agents;
    address[] private _registered;

    event AgentRegistered(address indexed agent, bytes32 passportHash, string capabilities);
    event AgentHeartbeat(address indexed agent, uint64 blockNumber);
    event AgentCapabilitiesUpdated(address indexed agent, string capabilities);

    error AlreadyRegistered();
    error NotRegistered();

    function register(string calldata capabilities, bytes32 passportHash) external {
        if (_agents[msg.sender].exists) revert AlreadyRegistered();
        _agents[msg.sender] = Agent({
            capabilities: capabilities,
            passportHash: passportHash,
            registeredAt: uint64(block.number),
            lastHeartbeat: uint64(block.number),
            exists: true
        });
        _registered.push(msg.sender);
        emit AgentRegistered(msg.sender, passportHash, capabilities);
    }

    function heartbeat() external {
        if (!_agents[msg.sender].exists) revert NotRegistered();
        _agents[msg.sender].lastHeartbeat = uint64(block.number);
        emit AgentHeartbeat(msg.sender, uint64(block.number));
    }

    function updateCapabilities(string calldata capabilities) external {
        if (!_agents[msg.sender].exists) revert NotRegistered();
        _agents[msg.sender].capabilities = capabilities;
        emit AgentCapabilitiesUpdated(msg.sender, capabilities);
    }

    function isActive(address agent) external view returns (bool) {
        Agent storage a = _agents[agent];
        if (!a.exists) return false;
        return block.number - a.lastHeartbeat <= LIVENESS_WINDOW;
    }

    function getAgent(address agent) external view returns (Agent memory) {
        return _agents[agent];
    }

    function registeredCount() external view returns (uint256) {
        return _registered.length;
    }

    function registeredAt(uint256 index) external view returns (address) {
        return _registered[index];
    }
}
