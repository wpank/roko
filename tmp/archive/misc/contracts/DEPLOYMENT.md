# Nunchi Agents Stack — Mirage Devnet Deployment

**Network**: mirage-devnet.up.railway.app (Chain ID: 1)
**Redeployed**: 2026-04-24 (contracts redeployed via Anvil account #0)
**Deployer**: `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266`
**Source**: [Nunchi-trade/contracts-core](https://github.com/Nunchi-trade/contracts-core) `packages/agents/`
**Deploy script**: `DeployMirage.s.sol` (via `forge script`)
**Broadcast artifact**: `contracts-core/broadcast/DeployMirage.s.sol/1/run-latest.json`

## RPC Endpoint

```
https://mirage-devnet.up.railway.app
```

## Railway Project

| Field | Value |
|---|---|
| Project Name | Mirage-rs |
| Project ID | `70abb6dd-a379-4979-840e-32aba529a65a` |
| Service Name | roko-agent |
| Service ID | `08e4cc7b-8a77-47d6-8b37-45f8c52a5960` |
| Environment | production |
| Volume Mount | `/workspace/.roko` |
| Dockerfile | `docker/mirage-demo.Dockerfile` |
| Health Check | `/relay/health` (300s timeout) |
| Restart Policy | ON_FAILURE (max 3 retries) |

## Mirage Configuration

| Setting | Value |
|---|---|
| Block Interval | **50ms** (`MIRAGE_BLOCK_INTERVAL_MS=50`) |
| Chain ID | 1 |
| Upstream Fork | None (standalone devnet) |
| Snapshot Interval | 15s |
| State Dir | `/workspace/.roko/state` |
| Chain Extensions | HDC, Knowledge, Stigmergy enabled |
| HNSW Threshold | 100,000 |
| Persistence | Enabled (snapshots to Railway volume) |

### Pruning (fixed 2026-04-23)

Mirage now prunes transactions and receipts when blocks are evicted. Previously only
blocks were pruned, leaving orphaned txs/receipts that grew unbounded and caused
`Memory capacity exceeded` errors from jsonrpsee serialization.

- **Max retained blocks**: 1,000
- **Pruning trigger**: `commit_local_transaction()` → `prune_old_blocks()`
- **Snapshot restore**: `apply_fork_snapshot()` calls `prune_old_blocks()` after restoring
- **Files changed**: `apps/mirage-rs/src/fork.rs`, `apps/mirage-rs/src/persist.rs`
- **Escape hatch**: `MIRAGE_NO_PERSIST=1` env var skips snapshot restore entirely

### EVM Forking Notes

Forking ETH mainnet requires a paid RPC provider with archive access. Free public RPCs
(Llama, Ankr, Cloudflare) either rate-limit aggressively or don't serve historical state
(`eth_getBalance` on pinned blocks fails with "header not found"). The previous Alchemy
API key (`LQ4XyD7-bqX01fy6`) is expired ("Must be authenticated!").

To enable forking, set `ETH_RPC_URL` on Railway to a valid archive RPC:
```bash
railway variables set "ETH_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/<YOUR_KEY>"
railway redeploy -y
```

## Deployer Accounts

### Current Deployer (2026-04-24)

| Field | Value |
|---|---|
| Address | `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` |
| Private Key | `0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80` |
| Balance | ~1 ETH (prefunded by mirage-rs genesis) |

> Standard Anvil/Hardhat account #0. Deployed via `DeployMirage.s.sol` with `--slow --legacy`
> flags through a local Python proxy (DNS resolution workaround).

### Legacy Deployer (2026-04-23, addresses no longer valid)

| Field | Value |
|---|---|
| Address | `0x19E7E376E7C213B7E7e7e46cc70A5dD086DAff2A` |
| Private Key | `0x1111111111111111111111111111111111111111111111111111111111111111` |

> Previous deployer with fresh nonce. Addresses invalid after Mirage restart cleared state.

## Contract Addresses

### Core Contracts

| # | Contract | Address | Tx Hash |
|---|---|---|---|
| 1 | RoleRegistry | `0x322813Fd9A801c5507c9de605d63CEA4f2CE6c44` | — |
| 2 | MockERC20 (DAEJI) | `0xa85233C63b9Ee964Add6F2cffe00Fd84eb32338f` | — |
| 3 | AgentRegistry | `0x4A679253410272dd5232B3Ff7cF5dbB88f295319` | — |
| 4 | WorkerRegistry | `0x7a2088a1bFc9d81c55368AE168C2C02570cB814F` | — |
| 5 | BountyMarket | `0x09635F643e140090A9A8Dcd712eD6285858ceBef` | — |
| 6 | ConsortiumValidator | `0xc5a5C42992dECbae36851359345FE25997F5C42d` | — |
| 7 | JobTypeRegistry | `0x67d269191c92Caf3cD7723F116c85e6E9bf55933` | — |

### Auxiliary Contracts

| # | Contract | Address | Tx Hash |
|---|---|---|---|
| 8 | InsightBoard | `0xE6E340D132b5f46d1e472DebcD681B2aBc16e57E` | — |
| 9 | FeeDistributor | `0xc3e53F4d16Ae77Db1c982e75a937B9f60FE63690` | — |
| 10 | DisputeResolver | `0x84eA74d481Ee0A5332c457a4d796187F6Ba67fEB` | — |
| 11 | CompletionProof | `0x9E545E3C0baAB3E08CdfD552C960A1050f373042` | — |
| 12 | NotificationRegistry | `0xa82fF9aFd8f496c3d6ac40E2a0F282E47488CFc9` | — |
| 13 | ISFRMinimal | `0x1613beB3B2C4f22Ee086B2b38C1476A3cE7f78E8` | — |

### ERC-8004 Identity (Korai Passport) — Built into Mirage Binary

These are bootstrapped on every Mirage startup, not from the deploy script:

| # | Contract | Address |
|---|---|---|
| — | IdentityRegistry | `0x8004A818BFB912233c491871b3d84c89A494BD9e` |
| — | ReputationRegistry | `0x8004A818Bfb912233c491871B3D84c89A494Bd9F` |
| — | ValidationRegistry | `0x8004a818bfb912233c491871B3D84C89A494Bda0` |

> Init bytecode compiled into the mirage-rs binary via `include_str!()` from
> `apps/mirage-rs/static/erc8004/*.init.hex`. Deployed via `EvmExecutor::transact()`
> at startup before the RPC server starts. Bootstrap deployer prefunded with 10 ETH.

### ERC-8183 Job Wrappers

| # | Contract | Address | Tx Hash |
|---|---|---|---|
| 14 | PerpsLiquidatorJob | `0x851356ae760d987E095750cCeb3bC6014560891C` | — |
| 15 | OracleUpdaterJob | `0xf5059a5D33d5853360D16C683c16e67980206f36` | — |
| 16 | FundingRateKeeperJob | `0x95401dc811bb5740090279Ba06cfA8fcF6113778` | — |

## Post-Deploy Wiring

| Action | Status |
|---|---|
| MANAGER_ROLE granted to deployer | Done |
| OPERATOR_ROLE granted to BountyMarket | Done |
| OPERATOR_ROLE granted to ConsortiumValidator | Done |
| BountyMarket resolver set to ConsortiumValidator | Done |
| JobTypeRegistry seeded: `perps-liquidate` | Done |
| JobTypeRegistry seeded: `oracle-update` | Done |
| JobTypeRegistry seeded: `funding-window` | Done |

## Role Hashes

| Role | Hash |
|---|---|
| MANAGER_ROLE | `0x241ecf16d79d0f8dbfb92cbc07fe17840425976cf0667f022fe9877caa831b08` |
| OPERATOR_ROLE | `0x97667070c54ef182b0f5858b034beac1b6f3089aa2d3188bb1e8929f4fa9b929` |

## Job Type Hashes

| Job Type | Hash | Min Tier | Min Bounty | Max Deadline |
|---|---|---|---|---|
| perps-liquidate | `keccak256("perps-liquidate")` | 3 (Trusted) | 10 ETH | 1 hour |
| oracle-update | `keccak256("oracle-update")` | 3 (Trusted) | 1 ETH | 1 hour |
| funding-window | `keccak256("funding-window")` | 3 (Trusted) | 5 ETH | 1 hour |

## DAEJI Token

| Field | Value |
|---|---|
| Address | `0xa85233C63b9Ee964Add6F2cffe00Fd84eb32338f` |
| Name | DAEJI |
| Symbol | DAEJI |
| Decimals | 18 |

Mint tokens:
```bash
cast send 0xa85233C63b9Ee964Add6F2cffe00Fd84eb32338f \
  "mint(address,uint256)" <recipient> <amount_wei> \
  --rpc-url https://mirage-devnet.up.railway.app \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
```

## Quick Interaction Guide

```bash
RPC="https://mirage-devnet.up.railway.app"
PK="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

# Register an agent
cast send 0x4A679253410272dd5232B3Ff7cF5dbB88f295319 \
  "register(string,bytes32,string)" "my-agent" 0x0 "ipfs://..." \
  --rpc-url $RPC --private-key $PK

# Check agent count
cast call 0x4A679253410272dd5232B3Ff7cF5dbB88f295319 \
  "count()(uint256)" --rpc-url $RPC

# Fund any address with ETH
curl -X POST $RPC -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"anvil_setBalance","params":["<address>","0x21E19E0C9BAB2400000"],"id":1}'

# Check block number
cast block-number --rpc-url $RPC

# Check auto-miner status (should be ~50ms blocks)
cast block latest --rpc-url $RPC
```

## Deployment Method

Contracts deployed via `forge script DeployMirage` through a local Node.js proxy to
bypass DNS resolution issues with `mirage-devnet.up.railway.app`. The proxy forwards
to Railway's IP (`151.101.2.15`) with proper SNI/Host headers.

```bash
# Start proxy (if DNS is flaky)
node -e "
const http = require('http'), https = require('https');
http.createServer((req, res) => {
  let body = [];
  req.on('data', c => body.push(c));
  req.on('end', () => {
    const data = Buffer.concat(body);
    const p = https.request({
      hostname: '151.101.2.15', port: 443, path: req.url, method: req.method,
      headers: {...req.headers, host: 'mirage-devnet.up.railway.app'},
      servername: 'mirage-devnet.up.railway.app',
    }, r => { res.writeHead(r.statusCode, r.headers); r.pipe(res); });
    p.on('error', e => { res.writeHead(502); res.end(e.message); });
    p.end(data);
  });
}).listen(19548, '127.0.0.1', () => console.log('proxy :19548'));
" &

# Deploy
cd /Users/will/dev/nunchi/contracts-core
FOUNDRY_PROFILE=agents \
DEPLOYER_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
forge script DeployMirage --rpc-url http://127.0.0.1:19548 --broadcast --slow --legacy
```

## Redeployment Notes

Contracts must be redeployed after a Mirage restart that clears state (e.g., `--no-persist`
or volume wipe). The ERC-8004 contracts are always re-bootstrapped at fixed addresses by
the mirage-rs binary itself.

**Important**: Use a fresh deployer account with nonce 0 when redeploying. The 50ms
auto-miner advances nonces on prefunded accounts between simulation and broadcast,
causing `EOA nonce changed unexpectedly` errors with stale accounts.

## Exchange Contracts (packages/exchange)

Deployed via Cannon (`cannonfile.test.toml`) on 2026-04-24. Test cannonfile deploys
modules + routers directly (no TransparentUpgradeableProxy wrapping). ClearingHouse
and MarketRegistry are NOT initialized — this is the module infrastructure only.

### ClearingHouse

| # | Contract | Address |
|---|---|---|
| 1 | AccountModule | `0x5FbDB2315678afecb367f032d93F642f64180aa3` |
| 2 | ConfigurationModule | `0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512` |
| 3 | PositionModule | `0x2279B7A0a67DB372996a5FaB50D91eAA73d2eBe6` |
| 4 | LiquidationModule | `0x0165878A594ca255338adfa4d48449f69242Eb8F` |
| 5 | FeeModule | `0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9` |
| 6 | ViewModule | `0xB7f8BC63BbcaD18155201308C8f3540b07f84F5e` |
| 7 | SpotModule | `0x610178dA211FEF7D417bC0e6FeD39F05609AD788` |
| 8 | **ClearingHouseRouter** | `0xa51c1fc2f0d1a1b8494ed1fe312d7c3a78ed91c0` |

### MarketRegistry

| # | Contract | Address |
|---|---|---|
| 9 | MarketConfigModule | `0xa513E6E4b8f2a923D98304ec87F64353C4D5C853` |
| 10 | FundingWindowModule | `0x5FC8d32690cc91D4c39d9d3abcBD16989F875707` |
| 11 | DPNLWindowModule | `0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0` |
| 12 | FeeConfigModule | `0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9` |
| 13 | SpotMarketConfigModule | `0x8A791620dd6260079BF849Dc5567aDC3F2FdC318` |
| 14 | **MarketRegistryRouter** | `0x0dcd1bf9a1b36ce34237eeafef220932846bcd82` |

### Exchange Deploy Command

```bash
cd contracts-core
FOUNDRY_PROFILE=exchange npx cannon build packages/exchange/cannonfile.test.toml \
  --rpc-url http://127.0.0.1:8555 \
  --chain-id 1 \
  --private-key $DEPLOYER_PK \
  --skip-compile --gas-price 1 --wipe --quiet
```

### Not Yet Deployed (exchange)

| Component | Why |
|---|---|
| MockERC20 (USDC) | Needed for ClearingHouse initialization |
| MockPythOracle / MockStorkOracle | Needed for OracleManager |
| OracleManager | Needed for ClearingHouse initialization |
| AccountNFT | Needed for ConfigurationModule.initialize |
| PerpOrderbook (per-market) | Needs ClearingHouse + MarketRegistry initialized first |

To fully initialize the exchange, deploy the production cannonfile with mock
settings and run the Phase 2-5 scripts (see exchange CLAUDE.md for details).

## Runtime Contract Discovery

Mirage-rs exposes `GET /api/deployment` for runtime address discovery.
After deploying contracts, register them:

```bash
./scripts/register-deployment.sh  # reads forge broadcast, POSTs to mirage
```

Dashboard and roko-serve fetch addresses at startup via `GET /api/deployment`
instead of hardcoding them. ERC-8004 addresses are auto-seeded by mirage.

## Packages NOT Deployed

| Package | Reason |
|---|---|
| `nhype` | Liquid staking for HyperEVM — requires CREATE2 vanity salts + validator infrastructure |
| `vaults` | Genesis vaults — requires external DeFi integrations (AAVE, LayerZero) |
| `sy-tokens` | Pendle SY wrapper — requires existing Pendle deployment + ProxyAdmin |

These packages target production chains (ETH mainnet, Arbitrum, HyperEVM) and have
dependencies on live infrastructure that doesn't exist on the mirage devnet.

## Railway CLI Cheatsheet

```bash
# Check status
railway status

# View logs
railway logs -n 50
railway logs --build --latest -n 20    # build logs

# Set env vars
railway variables set "KEY=VALUE"
railway variables list

# Redeploy (reuses existing image)
railway redeploy -y

# Deploy from local code (rebuilds)
railway up --detach -m "description"

# Manage volume
railway volume list
```

## Incident Log

### 2026-04-24: Contracts missing after Mirage restart

**Symptom**: Dashboard showed 0 agents, empty Knowledge Layer, Stigmergy Field, and Agent
Collective panels. `eth_getCode` returned `0x` for all previously deployed addresses.

**Root cause**: Mirage restarted (new Railway build) and the volume snapshot did not preserve
the previous deployment state. All Foundry-deployed contracts were wiped. The ERC-8004 contracts
(IdentityRegistry, ReputationRegistry, ValidationRegistry) were bootstrapped at their fixed
addresses by the mirage-rs binary, but the agents stack contracts were gone.

**Fix**: Redeployed all 16 contracts via `DeployMirage.s.sol` using Anvil account #0
(`0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266`) with `--slow --legacy` flags through a
local Python proxy (DNS resolution workaround). First attempt with default batch mode
failed with "Transaction dropped from the mempool" — the `--slow` flag serializes tx
submission and `--legacy` avoids EIP-1559 gas pricing issues.

**Lesson**: Always use `--slow --legacy` when deploying to mirage through a proxy. Keep
`tmp/contracts/addresses.md` updated after every redeploy.

### 2026-04-23: Memory capacity exceeded (OOM)

**Symptom**: Every RPC request returned `Error serializing response: Error("Memory capacity exceeded")`.
Mirage was alive but completely unresponsive. Health check kept failing.

**Root cause**: `prune_old_blocks()` only evicted blocks from `blocks_by_number` and
`blocks_by_hash`, but left orphaned transactions and receipts in their respective HashMaps.
After 223K+ blocks at 1 block/sec, the tx/receipt maps grew so large that jsonrpsee couldn't
serialize any response.

**Fix**: Extended `prune_old_blocks()` to also remove transactions and receipts belonging
to pruned blocks. Added `prune_old_blocks()` call after snapshot restore to clean up
pre-fix snapshots. Added `MIRAGE_NO_PERSIST=1` env var escape hatch.

**Files**: `apps/mirage-rs/src/fork.rs:1137-1146`, `apps/mirage-rs/src/persist.rs:166-185`

### 2026-04-23: Alchemy API keys expired

All Alchemy API keys found in the codebase are inactive:
- `LQ4XyD7-bqX01fy6` (eth-mainnet) → "Must be authenticated!"
- `L8lRc4P8NvKni000Tv1v7` (base-mainnet) → "App is inactive"

Mirage was switched to standalone mode (no fork). A paid archive RPC is needed for
mainnet forking.
