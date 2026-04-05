// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { Test } from "forge-std/Test.sol";
import { MockERC20 } from "../src/MockERC20.sol";
import { WorkerRegistry } from "../src/WorkerRegistry.sol";

contract WorkerRegistryTest is Test {
    MockERC20 internal token;
    WorkerRegistry internal reg;
    address internal alice = address(0xA11CE);
    address internal bob = address(0xB0B);
    address internal authorizedCaller = address(0xCAFE);

    function setUp() public {
        token = new MockERC20("DAEJI", "DAEJI", 18);
        reg = new WorkerRegistry(address(token));
        reg.setAuthorized(authorizedCaller, true);

        // Seed Alice + Bob with tokens and approvals.
        token.mint(alice, 100_000 ether);
        token.mint(bob, 100_000 ether);
        vm.prank(alice);
        token.approve(address(reg), type(uint256).max);
        vm.prank(bob);
        token.approve(address(reg), type(uint256).max);
    }

    function _register(address who, uint256 amount) internal {
        vm.prank(who);
        reg.register(amount);
    }

    function test_register_requires_min_bond() public {
        vm.prank(alice);
        vm.expectRevert(WorkerRegistry.InsufficientBond.selector);
        reg.register(999 ether);
    }

    function test_register_and_starting_reputation_is_half() public {
        _register(alice, 1_000 ether);
        WorkerRegistry.Worker memory w = reg.getWorker(alice);
        assertTrue(w.exists);
        assertEq(w.bond, 1_000 ether);
        assertEq(w.reputation, reg.SCALE() / 2);
        assertEq(uint8(reg.tier(alice)), uint8(WorkerRegistry.Tier.Standard));
    }

    function test_ema_updates_monotonically_on_all_success() public {
        _register(alice, 1_000 ether);
        uint256 prev = reg.reputationOf(alice);
        for (uint256 i = 0; i < 10; i++) {
            vm.prank(authorizedCaller);
            reg.updateReputation(alice, true);
            uint256 next = reg.reputationOf(alice);
            assertGt(next, prev);
            prev = next;
        }
        // After 10 successes reputation pushes into Trusted territory.
        assertGt(reg.reputationOf(alice), 800_000);
    }

    function test_ema_bounded_in_zero_to_scale() public {
        _register(alice, 1_000 ether);
        // Hammer successes then failures; reputation must stay ∈ [0, SCALE].
        for (uint256 i = 0; i < 50; i++) {
            vm.prank(authorizedCaller);
            reg.updateReputation(alice, i % 2 == 0);
            uint256 r = reg.reputationOf(alice);
            assertLe(r, reg.SCALE());
        }
    }

    function test_slash_reduces_bond_by_bps() public {
        _register(alice, 10_000 ether);
        uint8 reasonCode = reg.SLASH_QUALITY_REJECT();
        vm.prank(authorizedCaller);
        reg.slash(alice, reasonCode, 500); // 5%
        assertEq(reg.getWorker(alice).bond, 9_500 ether);
    }

    function test_unauthorized_update_reverts() public {
        _register(alice, 1_000 ether);
        vm.prank(bob);
        vm.expectRevert(WorkerRegistry.NotAuthorized.selector);
        reg.updateReputation(alice, true);
    }

    function test_unbond_cannot_drop_below_min() public {
        _register(alice, 2_000 ether);
        vm.prank(alice);
        vm.expectRevert(WorkerRegistry.BelowMinBond.selector);
        reg.unbond(1_500 ether);
    }

    function test_unbond_success() public {
        _register(alice, 3_000 ether);
        uint256 before = token.balanceOf(alice);
        vm.prank(alice);
        reg.unbond(1_000 ether);
        assertEq(token.balanceOf(alice), before + 1_000 ether);
        assertEq(reg.getWorker(alice).bond, 2_000 ether);
    }

    function test_decay_halves_toward_half() public {
        _register(alice, 1_000 ether);
        // Push rep high.
        for (uint256 i = 0; i < 20; i++) {
            vm.prank(authorizedCaller);
            reg.updateReputation(alice, true);
        }
        uint256 high = reg.reputationOf(alice);
        assertGt(high, 700_000);

        // Skip forward 30 days; reputation should decay halfway toward 0.5.
        vm.warp(block.timestamp + 30 days);
        uint256 mid = reg.SCALE() / 2;
        uint256 expected = mid + (high - mid) / 2;
        uint256 actual = reg.reputationOf(alice);
        // allow ±1 for integer truncation
        assertApproxEqAbs(actual, expected, 1);
    }

    function test_tier_thresholds() public {
        _register(alice, 1_000 ether);
        // Standard by default.
        assertEq(uint8(reg.tier(alice)), uint8(WorkerRegistry.Tier.Standard));
        // Drive to Elite.
        for (uint256 i = 0; i < 40; i++) {
            vm.prank(authorizedCaller);
            reg.updateReputation(alice, true);
        }
        assertEq(uint8(reg.tier(alice)), uint8(WorkerRegistry.Tier.Elite));
    }
}
