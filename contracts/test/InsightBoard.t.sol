// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { Test } from "forge-std/Test.sol";
import { MockERC20 } from "../src/MockERC20.sol";
import { InsightBoard } from "../src/InsightBoard.sol";

contract InsightBoardTest is Test {
    MockERC20 internal token;
    InsightBoard internal board;
    address internal alice = address(0xA11CE);
    address internal bob = address(0xB0B);
    address internal carol = address(0xCAA0);

    function setUp() public {
        token = new MockERC20("DAEJI", "DAEJI", 18);
        board = new InsightBoard(address(token));
        // Treasury funds the board so `claim` can succeed.
        token.mint(address(board), 1_000_000 ether);
    }

    function _post(address who, bytes32 hash) internal returns (uint256) {
        vm.prank(who);
        return board.post(hash, "ipfs://abc");
    }

    function test_post_and_query() public {
        uint256 id = _post(alice, bytes32("h1"));
        InsightBoard.Insight memory ins = board.getInsight(id);
        assertEq(ins.poster, alice);
        assertEq(ins.contentHash, bytes32("h1"));
        assertEq(ins.pheromone, 0);
    }

    function test_confirm_increments_pheromone_and_credits_earnings() public {
        uint256 id = _post(alice, bytes32("h1"));
        vm.prank(bob);
        board.confirm(id);
        vm.prank(carol);
        board.confirm(id);
        assertEq(board.getInsight(id).pheromone, 2);
        assertEq(board.earningsOf(alice), 2 ether);
    }

    function test_cannot_self_confirm() public {
        uint256 id = _post(alice, bytes32("h1"));
        vm.prank(alice);
        vm.expectRevert(InsightBoard.SelfConfirm.selector);
        board.confirm(id);
    }

    function test_cannot_double_confirm() public {
        uint256 id = _post(alice, bytes32("h1"));
        vm.prank(bob);
        board.confirm(id);
        vm.prank(bob);
        vm.expectRevert(InsightBoard.AlreadyConfirmed.selector);
        board.confirm(id);
    }

    function test_claim_transfers_and_resets() public {
        uint256 id = _post(alice, bytes32("h1"));
        vm.prank(bob);
        board.confirm(id);
        uint256 balBefore = token.balanceOf(alice);
        vm.prank(alice);
        uint256 claimed = board.claim();
        assertEq(claimed, 1 ether);
        assertEq(token.balanceOf(alice), balBefore + 1 ether);
        assertEq(board.earningsOf(alice), 0);
    }

    function test_claim_without_earnings_reverts() public {
        vm.prank(alice);
        vm.expectRevert(InsightBoard.NothingToClaim.selector);
        board.claim();
    }
}
