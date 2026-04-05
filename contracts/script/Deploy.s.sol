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

/// @title Deploy — reference deployer for the roko demo suite.
/// @notice Not used by `roko-demo` directly (it uses an alloy-based deployer
///         to sidestep forge's mempool-watcher model against mirage-rs), but
///         kept here for operators who want a pure-forge path.
contract Deploy is Script {
    struct Deployed {
        address mockErc20;
        address agentRegistry;
        address workerRegistry;
        address bountyMarket;
        address consortiumValidator;
        address insightBoard;
    }

    function run() external returns (Deployed memory d) {
        vm.startBroadcast();

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

        vm.stopBroadcast();

        d = Deployed({
            mockErc20: address(daeji),
            agentRegistry: address(agentReg),
            workerRegistry: address(workerReg),
            bountyMarket: address(market),
            consortiumValidator: address(consortium),
            insightBoard: address(board)
        });

        console2.log("MockERC20           ", d.mockErc20);
        console2.log("AgentRegistry       ", d.agentRegistry);
        console2.log("WorkerRegistry      ", d.workerRegistry);
        console2.log("BountyMarket        ", d.bountyMarket);
        console2.log("ConsortiumValidator ", d.consortiumValidator);
        console2.log("InsightBoard        ", d.insightBoard);
    }
}
