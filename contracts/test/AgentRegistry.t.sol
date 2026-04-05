// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { Test } from "forge-std/Test.sol";
import { AgentRegistry } from "../src/AgentRegistry.sol";

contract AgentRegistryTest is Test {
    AgentRegistry internal reg;
    address internal alice = address(0xA11CE);
    address internal bob = address(0xB0B);

    function setUp() public {
        reg = new AgentRegistry();
    }

    function test_register_and_query() public {
        bytes32 passport = keccak256("alice-passport");
        vm.prank(alice);
        reg.register("compute,storage", passport);

        AgentRegistry.Agent memory a = reg.getAgent(alice);
        assertTrue(a.exists);
        assertEq(a.capabilities, "compute,storage");
        assertEq(a.passportHash, passport);
        assertEq(reg.registeredCount(), 1);
        assertEq(reg.registeredAt(0), alice);
    }

    function test_double_register_reverts() public {
        vm.startPrank(alice);
        reg.register("c", bytes32("p"));
        vm.expectRevert(AgentRegistry.AlreadyRegistered.selector);
        reg.register("c2", bytes32("p2"));
        vm.stopPrank();
    }

    function test_heartbeat_liveness() public {
        vm.prank(alice);
        reg.register("c", bytes32("p"));

        assertTrue(reg.isActive(alice));
        vm.roll(block.number + reg.LIVENESS_WINDOW() + 1);
        assertFalse(reg.isActive(alice));

        vm.prank(alice);
        reg.heartbeat();
        assertTrue(reg.isActive(alice));
    }

    function test_heartbeat_without_register_reverts() public {
        vm.prank(bob);
        vm.expectRevert(AgentRegistry.NotRegistered.selector);
        reg.heartbeat();
    }

    function test_update_capabilities() public {
        vm.startPrank(alice);
        reg.register("a", bytes32("p"));
        reg.updateCapabilities("a,b,c");
        vm.stopPrank();
        assertEq(reg.getAgent(alice).capabilities, "a,b,c");
    }
}
