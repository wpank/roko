# Contract Addresses — Mirage Devnet

Quick-reference address table for `mirage-devnet.up.railway.app`.

Last updated: 2026-04-24 (post-restart redeploy)

```
RPC_URL=https://mirage-devnet.up.railway.app
CHAIN_ID=1

# Deployer
DEPLOYER=0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
DEPLOYER_PK=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

# ═══════════════════════════════════════════════════
# packages/agents — Agent infrastructure (forge)
# ═══════════════════════════════════════════════════

# Core
ROLE_REGISTRY=0x959922bE3CAee4b8Cd9a407cc3ac1C251C2007B1
DAEJI_TOKEN=0x9A9f2CCfdE556A7E9Ff0848998Aa4a0CFD8863AE
AGENT_REGISTRY=0x68B1D87F95878fE05B998F19b66F4baba5De1aed
WORKER_REGISTRY=0x3Aa5ebB10DC797CAC828524e59A333d0A371443c
BOUNTY_MARKET=0xc6e7DF5E7b4f2A278906862b61205850344D4e7d
CONSORTIUM_VALIDATOR=0x59b670e9fA9D0A427751Af201D676719a970857b
JOB_TYPE_REGISTRY=0x4ed7c70F96B99c776995fB64377f0d4aB3B0e1C1

# Auxiliary
INSIGHT_BOARD=0x322813Fd9A801c5507c9de605d63CEA4f2CE6c44
FEE_DISTRIBUTOR=0xa85233C63b9Ee964Add6F2cffe00Fd84eb32338f
DISPUTE_RESOLVER=0x4A679253410272dd5232B3Ff7cF5dbB88f295319
COMPLETION_PROOF=0x7a2088a1bFc9d81c55368AE168C2C02570cB814F
NOTIFICATION_REGISTRY=0x09635F643e140090A9A8Dcd712eD6285858ceBef
ISFR_MINIMAL=0xc5a5C42992dECbae36851359345FE25997F5C42d

# ERC-8183 Job Wrappers
PERPS_LIQUIDATOR_JOB=0x67d269191c92Caf3cD7723F116c85e6E9bf55933
ORACLE_UPDATER_JOB=0xE6E340D132b5f46d1e472DebcD681B2aBc16e57E
FUNDING_RATE_KEEPER_JOB=0xc3e53F4d16Ae77Db1c982e75a937B9f60FE63690

# ═══════════════════════════════════════════════════
# ERC-8004 Identity — bootstrapped by mirage-rs binary
# ═══════════════════════════════════════════════════

IDENTITY_REGISTRY=0x8004A818BFB912233c491871b3d84c89A494BD9e
REPUTATION_REGISTRY=0x8004A818Bfb912233c491871B3D84c89A494Bd9F
VALIDATION_REGISTRY=0x8004a818bfb912233c491871B3D84C89A494Bda0

# ═══════════════════════════════════════════════════
# packages/exchange — Perps DEX (cannon)
# ═══════════════════════════════════════════════════

# ClearingHouse
ACCOUNT_MODULE=0x5FbDB2315678afecb367f032d93F642f64180aa3
CONFIGURATION_MODULE=0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
POSITION_MODULE=0x2279B7A0a67DB372996a5FaB50D91eAA73d2eBe6
LIQUIDATION_MODULE=0x0165878A594ca255338adfa4d48449f69242Eb8F
FEE_MODULE=0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9
VIEW_MODULE=0xB7f8BC63BbcaD18155201308C8f3540b07f84F5e
SPOT_MODULE=0x610178dA211FEF7D417bC0e6FeD39F05609AD788
CLEARING_HOUSE_ROUTER=0xa51c1fc2f0d1a1b8494ed1fe312d7c3a78ed91c0

# MarketRegistry
MARKET_CONFIG_MODULE=0xa513E6E4b8f2a923D98304ec87F64353C4D5C853
FUNDING_WINDOW_MODULE=0x5FC8d32690cc91D4c39d9d3abcBD16989F875707
DPNL_WINDOW_MODULE=0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0
FEE_CONFIG_MODULE=0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
SPOT_MARKET_CONFIG_MODULE=0x8A791620dd6260079BF849Dc5567aDC3F2FdC318
MARKET_REGISTRY_ROUTER=0x0dcd1bf9a1b36ce34237eeafef220932846bcd82

# ═══════════════════════════════════════════════════
# Runtime discovery: GET /api/deployment (33 total)
# Dashboard fetches from mirage at startup
# ═══════════════════════════════════════════════════
```

## Seeded Data

| Type | Count |
|---|---|
| Agents | 9 (researcher, trader, auditor, orchestrator, coder, analyst, sentinel, curator, test) |
| Knowledge entries | 21 (insight, heuristic, warning, causal_link, strategy_fragment) |
| Pheromones | 14 (wisdom, opportunity, threat at varied intensities) |
| Contracts registered | 33 (via GET /api/deployment) |

## Redeploy Process

```bash
# 1. Start DNS proxy (if needed)
python3 scripts/mirage-proxy.py &

# 2. Deploy exchange (cannon) — FIRST (uses lower nonces)
cd contracts-core
FOUNDRY_PROFILE=exchange npx cannon build packages/exchange/cannonfile.test.toml \
  --rpc-url http://127.0.0.1:8555 --chain-id 1 \
  --private-key $DEPLOYER_PK --skip-compile --gas-price 1 --wipe --quiet

# 3. Deploy agents (forge) — SECOND
FOUNDRY_PROFILE=agents DEPLOYER_PRIVATE_KEY=$DEPLOYER_PK \
  forge script DeployMirage --rpc-url http://127.0.0.1:8555 --broadcast --slow --legacy

# 4. Register with mirage
python3 scripts/register-deployment.py

# 5. Seed data
bash scripts/seed-mirage.sh http://127.0.0.1:8555
```
