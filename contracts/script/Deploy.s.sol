// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { Script } from "forge-std/Script.sol";
import { console2 } from "forge-std/console2.sol";

import { MockERC20 } from "../src/MockERC20.sol";
import { AgentRegistry } from "../src/AgentRegistry.sol";
import { WorkerRegistry } from "../src/WorkerRegistry.sol";
import { BountyMarket } from "../src/BountyMarket.sol";
import { ConsortiumValidator } from "../src/ConsortiumValidator.sol";
import { InsightBoard } from "../src/InsightBoard.sol";
import { RoleRegistry } from "../src/RoleRegistry.sol";
import { ISFROracle } from "../src/ISFROracle.sol";
import { ISFRBountyPool } from "../src/ISFRBountyPool.sol";
import { IdentityRegistry } from "../src/IdentityRegistry.sol";
import { ReputationRegistry } from "../src/ReputationRegistry.sol";
import { ValidationRegistry } from "../src/ValidationRegistry.sol";
import { FeeDistributor } from "../src/FeeDistributor.sol";

/// @title Deploy — canonical full-suite reference deployer for all 13 roko contracts.
/// @notice Deploys every authored contract with correct constructor wiring.
///         Not used by `roko-demo` directly (it uses an alloy-based deployer
///         to sidestep forge's mempool-watcher model against mirage-rs) or by
///         `isfr_bootstrap` (runtime subset path). Both are documented as subset
///         deployers; this script is the canonical reference that covers all 13.
contract Deploy is Script {
    struct Deployed {
        // --- original 6 ---
        address mockErc20;
        address agentRegistry;
        address workerRegistry;
        address bountyMarket;
        address consortiumValidator;
        address insightBoard;
        // --- ISFR set (3) ---
        address roleRegistry;
        address isfrOracle;
        address isfrBountyPool;
        // --- ERC-8004 trio (3) ---
        address identityRegistry;
        address reputationRegistry;
        address validationRegistry;
        // --- Fee distribution (1) ---
        address feeDistributor;
    }

    function run() external returns (Deployed memory d) {
        vm.startBroadcast();

        // ── Original 6 contracts ──────────────────────────────────────
        MockERC20 daeji = new MockERC20("DAEJI", "DAEJI", 18);
        AgentRegistry agentReg = new AgentRegistry();
        WorkerRegistry workerReg = new WorkerRegistry(address(daeji));
        BountyMarket market = new BountyMarket(address(daeji), address(workerReg));
        ConsortiumValidator consortium = new ConsortiumValidator(address(workerReg), address(market));
        InsightBoard board = new InsightBoard(address(daeji));

        // Post-deploy wiring: authorize market + consortium to update reputation,
        // delegate market's `resolve` to the consortium.
        workerReg.setAuthorized(address(market), true);
        workerReg.setAuthorized(address(consortium), true);
        market.setResolver(address(consortium));

        // ── ISFR set (3) ──────────────────────────────────────────────
        // Deploy order follows isfr_bootstrap.rs: RoleRegistry → ISFROracle → ISFRBountyPool
        RoleRegistry roleReg = new RoleRegistry(msg.sender);
        ISFROracle oracle = new ISFROracle(address(roleReg), address(0));
        ISFRBountyPool bountyPool = new ISFRBountyPool(address(roleReg), address(daeji), 1 ether);

        // Post-deploy wiring per isfr_bootstrap.rs:
        // - oracle.setBountyPool(bounty_pool)
        // - roleRegistry.grantRole(ORACLE_ROLE, oracle)
        // - roleRegistry.grantRole(KEEPER_ROLE, deployer)
        oracle.setBountyPool(address(bountyPool));
        roleReg.grantRole(bountyPool.ORACLE_ROLE(), address(oracle));
        roleReg.grantRole(oracle.KEEPER_ROLE(), msg.sender);

        // ── ERC-8004 trio (3) ─────────────────────────────────────────
        // IdentityRegistry first (ReputationRegistry + ValidationRegistry depend on it).
        IdentityRegistry identityReg = new IdentityRegistry(address(0), address(daeji));
        ReputationRegistry reputationReg = new ReputationRegistry(address(identityReg), address(0));
        ValidationRegistry validationReg = new ValidationRegistry(address(identityReg), address(0));

        // ── Fee distribution (1) ──────────────────────────────────────
        // Treasury is the deployer; operators can reassign post-deploy.
        FeeDistributor feeDist = new FeeDistributor(address(daeji), msg.sender);

        vm.stopBroadcast();

        d = Deployed({
            mockErc20: address(daeji),
            agentRegistry: address(agentReg),
            workerRegistry: address(workerReg),
            bountyMarket: address(market),
            consortiumValidator: address(consortium),
            insightBoard: address(board),
            roleRegistry: address(roleReg),
            isfrOracle: address(oracle),
            isfrBountyPool: address(bountyPool),
            identityRegistry: address(identityReg),
            reputationRegistry: address(reputationReg),
            validationRegistry: address(validationReg),
            feeDistributor: address(feeDist)
        });

        console2.log("MockERC20           ", d.mockErc20);
        console2.log("AgentRegistry       ", d.agentRegistry);
        console2.log("WorkerRegistry      ", d.workerRegistry);
        console2.log("BountyMarket        ", d.bountyMarket);
        console2.log("ConsortiumValidator ", d.consortiumValidator);
        console2.log("InsightBoard        ", d.insightBoard);
        console2.log("RoleRegistry        ", d.roleRegistry);
        console2.log("ISFROracle          ", d.isfrOracle);
        console2.log("ISFRBountyPool      ", d.isfrBountyPool);
        console2.log("IdentityRegistry    ", d.identityRegistry);
        console2.log("ReputationRegistry  ", d.reputationRegistry);
        console2.log("ValidationRegistry  ", d.validationRegistry);
        console2.log("FeeDistributor      ", d.feeDistributor);
    }
}
