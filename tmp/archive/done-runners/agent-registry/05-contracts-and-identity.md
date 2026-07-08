# 05 — Contracts and Identity

This doc defines the **target** ERC-8004 surface for the discovery design.
If current code disagrees with this doc, the code should be brought into sync in
the implementation phase.

## Deployment strategy on mirage

The intended behavior is:

1. if mirage forks a chain with ERC-8004 already deployed, reuse it
2. otherwise deploy the target contracts at boot

That gives one conceptual model for:

- local dev
- Railway devnet
- eventual real chain integration

## Target identity contract

The main contract is `IdentityRegistry`.

### Target interface

```solidity
struct PassportData {
    uint64  capabilityList;
    uint8   tier;
    bytes32 systemPromptHash;
    bytes32 teeAttestation;
    uint256 registeredBlock;
    string  agentCardUri;
}

mapping(uint256 => PassportData) public passports;
mapping(address => uint256) public ownerToPassportId;

function register(
    address agent,
    uint64 capabilityList,
    uint8 tier,
    bytes32 systemPromptHash,
    string calldata agentCardUri
) external returns (uint256 passportId);

function updateAgentCardUri(
    uint256 passportId,
    string calldata newUri
) external;

function registeredCount() external view returns (uint256);
function registeredAt(uint256 index) external view returns (uint256 passportId);
function hasCapability(uint256 passportId, uint8 bitIndex) external view returns (bool);
```

### Important ABI note

The target method signature is:

`updateAgentCardUri(uint256,string)`

Current helper code under
`crates/roko-agent-server/src/registration.rs` still encodes an outdated
signature. The docs should treat this contract ABI as authoritative and the code
work should update the helper accordingly.

## Companion registries

For the scope of discovery and messaging:

- `ReputationRegistry` can ship as a stub
- `ValidationRegistry` can ship as a stub

They exist now for address determinism and future compatibility, not because the
dashboard needs them today.

## Capability bitmask

The dashboard filter for this protocol is:

- bit 15 = `CAP_ROKO`

That gives a cheap first-pass filter before reading the card.

## Agent Card

The Agent Card is the transport-facing metadata document pointed to by
`agentCardUri`.

### Required MVP fields

```json
{
  "name": "roko-alpha-prod",
  "capabilities": ["messaging", "predictions"],
  "endpoints": {
    "rest": "https://agent.example.com",
    "relay": "wss://mirage.example.com/relay/ws/dashboard?agent=roko-alpha-prod"
  },
  "domain_tags": ["roko"],
  "version": "0.1.0"
}
```

### Optional extension fields

These are valid but not required for the first pass:

- `owner`
- `payment`
- `websocket`
- `a2a`
- `mcp`
- descriptive metadata

The docs should not force every one of these fields into the MVP.

## Agent Card hosting choices

| URI shape | Host | Typical use |
|---|---|---|
| `https://agent.example/.well-known/agent-card.json` | agent | public direct-connect agent |
| `https://mirage.example/relay/cards/{agent_id}` | relay | relay-first or wallet-free agent |
| `data:application/json;base64,...` | inline | tiny fallback only |

## Card publication rules

### Public direct agent

- serve card from the agent itself
- publish direct `rest` endpoint
- update `agentCardUri` on-chain

### Relay-first agent

- push card to relay on hello
- relay hosts it at `/relay/cards/{id}`
- card advertises relay endpoint

### Wallet-free agent

- same as relay-first agent
- no chain write is required

## Wallet-optional policy

Wallet-free agents are a first-class production path.

### Wallet-free agents may

- connect to relay
- be listed by the dashboard
- receive and answer messages
- run indefinitely

### Wallet-free agents may not

- own an 8004 passport unless they later gain signing capability
- participate in on-chain passport-based economics without a wallet

That is acceptable and intentional.

## Registration flows

### Wallet-holding flow

1. operator obtains or creates a passport
2. agent starts with `--passport-id`, chain RPC, and wallet config
3. agent builds a card
4. agent publishes the card URI
5. agent calls `updateAgentCardUri(passportId, uri)`
6. dashboard sees the passport and fetches the card

### Wallet-free flow

1. operator starts `roko agent serve --relay-url ...`
2. agent sends hello to relay
3. relay hosts the card at `/relay/cards/{agent_id}`
4. dashboard sees the relay entry and fetches the card

Neither flow is a toy version of the other.

## Discovery merge model

Dashboard-side merged type:

```ts
type Agent = {
  agentId: string;
  owner?: string;
  passportId?: bigint;
  tier?: number;
  endpoints?: {
    rest?: string;
    relay?: string;
  };
  online: boolean;
};
```

Load rules:

1. enumerate on-chain passports
2. filter by `CAP_ROKO`
3. fetch each card
4. read `GET /relay/agents`
5. merge by `agent_id`
6. use relay presence as current online signal when available

## System invariants

After the migration, all of these should be true:

- `mirage-rs::chain::AgentRegistry` is no longer the production identity source
- `IdentityRegistry` is the durable source of wallet-backed identity
- relay is the current source of relay-backed presence
- Agent Cards contain the transport metadata
- no agent is forced to have a wallet to participate
- `roko-serve` is not on the discovery or messaging hot path
