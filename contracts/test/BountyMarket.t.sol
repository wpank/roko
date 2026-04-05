// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { Test } from "forge-std/Test.sol";
import { MockERC20 } from "../src/MockERC20.sol";
import { WorkerRegistry } from "../src/WorkerRegistry.sol";
import { BountyMarket } from "../src/BountyMarket.sol";

contract BountyMarketTest is Test {
    MockERC20 internal token;
    WorkerRegistry internal workers;
    BountyMarket internal market;

    address internal poster = address(0xBEEF);
    address internal worker = address(0xC0FFEE);

    function setUp() public {
        token = new MockERC20("DAEJI", "DAEJI", 18);
        workers = new WorkerRegistry(address(token));
        market = new BountyMarket(address(token), address(workers));
        workers.setAuthorized(address(market), true);

        // Fund + approve.
        token.mint(poster, 1_000_000 ether);
        token.mint(worker, 10_000 ether);
        vm.prank(poster);
        token.approve(address(market), type(uint256).max);
        vm.prank(worker);
        token.approve(address(workers), type(uint256).max);

        // Register worker at Standard tier (default reputation 0.5).
        vm.prank(worker);
        workers.register(1_000 ether);
    }

    function _postJob() internal returns (uint256) {
        vm.prank(poster);
        return market.postJob(
            keccak256("spec"),
            500 ether,
            uint64(block.timestamp + 3600),
            uint8(WorkerRegistry.Tier.Standard)
        );
    }

    function test_post_transitions_to_funded_and_pulls_bounty() public {
        uint256 balBefore = token.balanceOf(poster);
        uint256 id = _postJob();
        assertEq(uint8(market.stateOf(id)), uint8(BountyMarket.State.Funded));
        assertEq(token.balanceOf(poster), balBefore - 500 ether);
        assertEq(token.balanceOf(address(market)), 500 ether);
    }

    function test_assign_requires_tier() public {
        uint256 id = _postJob();
        // Unregistered address cannot be assigned.
        vm.expectRevert(BountyMarket.WorkerTierTooLow.selector);
        market.assign(id, address(0x1234));
        // Valid worker works.
        market.assign(id, worker);
        assertEq(uint8(market.stateOf(id)), uint8(BountyMarket.State.Assigned));
    }

    function test_full_accept_flow() public {
        uint256 id = _postJob();
        market.assign(id, worker);
        vm.prank(worker);
        market.submit(id, keccak256("result"));
        uint256 workerBalBefore = token.balanceOf(worker);
        market.resolve(id, true);
        assertEq(uint8(market.stateOf(id)), uint8(BountyMarket.State.Terminal));
        assertEq(token.balanceOf(worker), workerBalBefore + 500 ether);
    }

    function test_reject_flow_refunds_and_slashes() public {
        uint256 id = _postJob();
        market.assign(id, worker);
        vm.prank(worker);
        market.submit(id, keccak256("result"));
        uint256 posterBalBefore = token.balanceOf(poster);
        uint256 workerBondBefore = workers.getWorker(worker).bond;
        market.resolve(id, false);
        assertEq(token.balanceOf(poster), posterBalBefore + 500 ether);
        // 5% slash.
        assertEq(workers.getWorker(worker).bond, workerBondBefore - (workerBondBefore * 500) / 10_000);
    }

    function test_non_assigned_cannot_submit() public {
        uint256 id = _postJob();
        market.assign(id, worker);
        vm.prank(address(0xABCD));
        vm.expectRevert(BountyMarket.NotAuthorized.selector);
        market.submit(id, bytes32("x"));
    }

    function test_resolver_only_can_resolve() public {
        uint256 id = _postJob();
        market.assign(id, worker);
        vm.prank(worker);
        market.submit(id, bytes32("x"));
        vm.prank(address(0xABCD));
        vm.expectRevert(BountyMarket.NotAuthorized.selector);
        market.resolve(id, true);
    }

    function test_state_machine_rejects_out_of_order_calls() public {
        uint256 id = _postJob();
        vm.prank(worker);
        vm.expectRevert(BountyMarket.WrongState.selector);
        market.submit(id, bytes32("early"));
    }
}
