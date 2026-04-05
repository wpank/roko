// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { Test } from "forge-std/Test.sol";
import { MockERC20 } from "../src/MockERC20.sol";

contract MockERC20Test is Test {
    MockERC20 internal token;

    function setUp() public {
        token = new MockERC20("DAEJI", "DAEJI", 18);
    }

    function test_metadata() public view {
        assertEq(token.name(), "DAEJI");
        assertEq(token.symbol(), "DAEJI");
        assertEq(token.decimals(), 18);
    }

    function test_mint_to_anyone() public {
        token.mint(address(0xBEEF), 100 ether);
        assertEq(token.balanceOf(address(0xBEEF)), 100 ether);
        assertEq(token.totalSupply(), 100 ether);
    }

    function testFuzz_mint_cumulative(uint96 a, uint96 b) public {
        token.mint(address(this), a);
        token.mint(address(this), b);
        assertEq(token.balanceOf(address(this)), uint256(a) + uint256(b));
    }
}
