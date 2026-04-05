// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { Test } from "forge-std/Test.sol";
import { MockERC20 } from "../src/MockERC20.sol";
import { WorkerRegistry } from "../src/WorkerRegistry.sol";
import { BountyMarket } from "../src/BountyMarket.sol";
import { ConsortiumValidator } from "../src/ConsortiumValidator.sol";

contract ConsortiumValidatorTest is Test {
    MockERC20 internal token;
    WorkerRegistry internal workers;
    BountyMarket internal market;
    ConsortiumValidator internal consortium;

    address internal poster = address(0xBEEF);
    address internal worker = address(0xC0FFEE);
    address[5] internal validators;

    function setUp() public {
        token = new MockERC20("DAEJI", "DAEJI", 18);
        workers = new WorkerRegistry(address(token));
        market = new BountyMarket(address(token), address(workers));
        consortium = new ConsortiumValidator(address(workers), address(market));

        // Market + consortium need to update reputation/slash + resolve.
        workers.setAuthorized(address(market), true);
        workers.setAuthorized(address(consortium), true);
        market.setResolver(address(consortium));

        // Fund everyone; register worker + 5 validators.
        token.mint(poster, 10_000 ether);
        vm.prank(poster);
        token.approve(address(market), type(uint256).max);
        token.mint(worker, 10_000 ether);
        vm.prank(worker);
        token.approve(address(workers), type(uint256).max);
        vm.prank(worker);
        workers.register(1_000 ether);

        for (uint256 i = 0; i < 5; i++) {
            validators[i] = address(uint160(0xD000 + i));
            token.mint(validators[i], 10_000 ether);
            vm.prank(validators[i]);
            token.approve(address(workers), type(uint256).max);
            vm.prank(validators[i]);
            workers.register(1_000 ether);
        }
        // Drive each validator's reputation into Trusted territory.
        workers.setAuthorized(address(this), true);
        for (uint256 k = 0; k < 30; k++) {
            for (uint256 i = 0; i < 5; i++) {
                workers.updateReputation(validators[i], true);
            }
        }
        for (uint256 i = 0; i < 5; i++) {
            assertGe(uint8(workers.tier(validators[i])), uint8(WorkerRegistry.Tier.Trusted));
        }
    }

    function _postAndSubmit() internal returns (uint256 id) {
        vm.prank(poster);
        id = market.postJob(
            keccak256("spec"),
            100 ether,
            uint64(block.timestamp + 3600),
            uint8(WorkerRegistry.Tier.Standard)
        );
        market.assign(id, worker);
        vm.prank(worker);
        market.submit(id, keccak256("result"));
    }

    function test_assemble_picks_three_distinct_trusted() public {
        uint256 id = _postAndSubmit();
        vm.roll(block.number + 1); // ensure blockhash(block.number-1) != 0
        consortium.assembleCommittee(id);
        address[3] memory m = consortium.getMembers(id);
        assertTrue(m[0] != m[1] && m[1] != m[2] && m[0] != m[2]);
        // All members must be in the validator set.
        for (uint256 i = 0; i < 3; i++) {
            bool found;
            for (uint256 j = 0; j < 5; j++) {
                if (m[i] == validators[j]) { found = true; break; }
            }
            assertTrue(found, "member not in validator set");
        }
    }

    function test_two_approves_accept_job() public {
        uint256 id = _postAndSubmit();
        vm.roll(block.number + 1);
        consortium.assembleCommittee(id);
        address[3] memory m = consortium.getMembers(id);
        vm.prank(m[0]);
        consortium.vote(id, true);
        vm.prank(m[1]);
        consortium.vote(id, true);
        assertEq(uint8(market.stateOf(id)), uint8(BountyMarket.State.Terminal));
        (, , bool tallied) = consortium.voteCounts(id);
        assertTrue(tallied);
    }

    function test_two_rejects_rejects_job() public {
        uint256 id = _postAndSubmit();
        vm.roll(block.number + 1);
        consortium.assembleCommittee(id);
        address[3] memory m = consortium.getMembers(id);
        vm.prank(m[0]);
        consortium.vote(id, false);
        vm.prank(m[1]);
        consortium.vote(id, false);
        assertEq(uint8(market.stateOf(id)), uint8(BountyMarket.State.Terminal));
    }

    function test_non_member_cannot_vote() public {
        uint256 id = _postAndSubmit();
        vm.roll(block.number + 1);
        consortium.assembleCommittee(id);
        vm.prank(address(0xABCD));
        vm.expectRevert(ConsortiumValidator.NotAMember.selector);
        consortium.vote(id, true);
    }

    function test_double_vote_reverts() public {
        uint256 id = _postAndSubmit();
        vm.roll(block.number + 1);
        consortium.assembleCommittee(id);
        address[3] memory m = consortium.getMembers(id);
        vm.prank(m[0]);
        consortium.vote(id, true);
        vm.prank(m[0]);
        vm.expectRevert(ConsortiumValidator.AlreadyVoted.selector);
        consortium.vote(id, false);
    }
}
