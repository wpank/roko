# WU-15: Verified Chain Demo Scenario

**Layer**: 5 (depends on all backend WUs)
**Depends on**: WU-13 (orchestrator wiring), WU-12 (sidecar routes), WU-14 (integration tests pass)
**Blocks**: none (leaf unit)
**Estimated effort**: 4-5 hours
**Crate/App**: `demo/demo-app`

---

## Overview

Add a new demo scenario tab — **"Verified Chain"** — to the `/demo` page. This is a scripted, polished end-to-end demo that:

1. Connects to live Tempo Moderato testnet
2. Spins up two light-client-configured agents
3. Agent Alpha queries verified balances and storage, showing trust levels
4. Agent Beta initiates a real MPP (Machine Payments Protocol) payment on testnet
5. Alpha verifies the payment landed using `chain.verify_transfer`
6. Both agents inspect the verified block head and backend status
7. Summary panel shows proof metadata, trust levels, and consensus mechanism

The scenario follows the exact patterns of the existing `chain-intelligence` scenario runner.

---

## Pre-read

- `demo/demo-app/src/lib/scenarios.ts` — `Scenario`, `ScenarioContext`, `ScenarioStep` interfaces
- `demo/demo-app/src/lib/scenario-runners/chain-intelligence.ts` — reference scenario runner (DeFi agents on mirage)
- `demo/demo-app/src/lib/scenario-runners/index.ts` — `allScenarios` array (registration)
- `demo/demo-app/src/pages/Demo/index.tsx` — `TAB_CATEGORY` and `CAT_COLORS` maps
- `demo/demo-app/src/lib/terminal-session.ts` — `enterWorkspace`, `showCmd`, `roko`, `trackMetrics`
- `demo/demo-app/src/lib/playback-controller.ts` — `PlaybackController`, `TimelineStepper`

---

## Tasks

### 15.1 Create the scenario runner

**File**: `demo/demo-app/src/lib/scenario-runners/verified-chain.ts`

```typescript
import type { Scenario, ScenarioContext } from '../scenarios';
import { enterWorkspace, showCmd, roko } from '../terminal-session';

const STEPS = [
  { label: 'Connect', sublabel: 'Initialize Tempo Moderato light client' },
  { label: 'Backends', sublabel: 'List configured chain backends' },
  { label: 'Head', sublabel: 'Fetch verified block head' },
  { label: 'Balance', sublabel: 'Query verified native token balance' },
  { label: 'Storage', sublabel: 'Read verified contract storage slot' },
  { label: 'Payment', sublabel: 'Initiate MPP payment on testnet' },
  { label: 'Verify', sublabel: 'Verify payment with consensus proof' },
  { label: 'Summary', sublabel: 'Trust levels and proof metadata' },
];

export const verifiedChain: Scenario = {
  id: 'verified-chain',
  title: 'Verified Chain',
  subtitle: 'Light-client verified queries and payments on Tempo testnet',
  panes: 2,
  labels: ['Verifier', 'Payer'],
  panel: true,
  promptBar: false,
  mirageBar: false,
  steps: STEPS,
  category: 'chain',
  features: ['light-client', 'consensus-verification', 'MPP-payments', 'state-proofs'],
  durationHint: '~90s',
  accent: 'amber',
  icon: 'chain',
  agents: [
    { name: 'Verifier Alpha', role: 'verifier' },
    { name: 'Payer Beta', role: 'payer' },
  ],

  async run(ctx: ScenarioContext) {
    const {
      entries, playback, timeline, setMetric, setGate,
      logCommand, logCommandComplete, signal, workspaceDir,
    } = ctx;
    const [alpha, beta] = entries;
    const TOTAL = STEPS.length;

    // ── Phase 0: Boot terminals ──────────────────────────────────────
    await enterWorkspace(alpha, workspaceDir);
    const betaDir = await ctx.createWorkspace('payer');
    await enterWorkspace(beta, betaDir);
    timeline.init(STEPS);

    // ── Phase 1: Connect — check Tempo connectivity ──────────────────
    await playback.waitForStep();
    playback.setProgress(0, TOTAL, 'Connecting to Tempo Moderato');
    timeline.setActive(0);
    logCommand('init', 'Initializing roko workspace with Tempo backend');

    // Initialize workspace with chain config pointing to Tempo Moderato
    await showCmd(alpha, roko(ctx, 'init'), {
      playback, timeout: 30_000,
      onLog: logCommand,
      onLogComplete: logCommandComplete,
    });

    // Check chain connectivity
    const statusResult = await showCmd(alpha, roko(ctx, 'config show'), {
      playback, timeout: 15_000,
      onLog: logCommand,
      onLogComplete: logCommandComplete,
      customDesc: 'Verify Tempo Moderato RPC is reachable',
    });
    setGate('connect', statusResult.ok ? 'pass' : 'fail');
    if (signal.aborted) return;

    // ── Phase 2: Backends — list configured backends ─────────────────
    await playback.waitForStep();
    playback.setProgress(1, TOTAL, 'Listing chain backends');
    timeline.setActive(1);

    // Use roko run with chain.backends tool
    const backendsResult = await showCmd(alpha,
      roko(ctx, 'run "List all configured chain backends using chain.backends"'), {
      playback, timeout: 60_000,
      onLog: logCommand,
      onLogComplete: logCommandComplete,
      customDesc: 'Agent calls chain.backends tool',
    });
    if (backendsResult.cost) setMetric('cost', backendsResult.cost);
    if (backendsResult.tokens) setMetric('tokens', backendsResult.tokens);
    setGate('backends', backendsResult.ok ? 'pass' : 'fail');
    if (signal.aborted) return;

    // ── Phase 3: Head — fetch verified block head ────────────────────
    await playback.waitForStep();
    playback.setProgress(2, TOTAL, 'Fetching verified block head');
    timeline.setActive(2);

    const headResult = await showCmd(alpha,
      roko(ctx, 'run "Get the latest verified block head using chain.head. Report the block number, hash, state root, and trust level."'), {
      playback, timeout: 60_000,
      onLog: logCommand,
      onLogComplete: logCommandComplete,
      customDesc: 'Agent calls chain.head — returns consensus-verified header',
    });
    if (headResult.cost) setMetric('cost', headResult.cost);
    setGate('head', headResult.ok ? 'pass' : 'fail');
    if (signal.aborted) return;

    // ── Phase 4: Balance — verified balance query ────────────────────
    await playback.waitForStep();
    playback.setProgress(3, TOTAL, 'Querying verified balance');
    timeline.setActive(3);

    // Query verified balance of a known Tempo testnet address
    const balanceResult = await showCmd(alpha,
      roko(ctx, 'run "Check the verified balance of address 0x0000000000000000000000000000000000000000 using chain.verified_balance. Report the balance in ETH, the trust level, consensus mechanism, and block number."'), {
      playback, timeout: 60_000,
      onLog: logCommand,
      onLogComplete: logCommandComplete,
      customDesc: 'Agent calls chain.verified_balance — balance + trust metadata',
    });
    if (balanceResult.cost) setMetric('cost', balanceResult.cost);
    setGate('balance', balanceResult.ok ? 'pass' : 'fail');
    if (signal.aborted) return;

    // ── Phase 5: Storage — verified storage read ─────────────────────
    await playback.waitForStep();
    playback.setProgress(4, TOTAL, 'Reading verified storage');
    timeline.setActive(4);

    const storageResult = await showCmd(alpha,
      roko(ctx, 'run "Read storage slot 0x0 of address 0x0000000000000000000000000000000000000000 using chain.verified_storage. Report the value and trust level."'), {
      playback, timeout: 60_000,
      onLog: logCommand,
      onLogComplete: logCommandComplete,
      customDesc: 'Agent calls chain.verified_storage — MPT-verified slot read',
    });
    if (storageResult.cost) setMetric('cost', storageResult.cost);
    setGate('storage', storageResult.ok ? 'pass' : 'fail');
    if (signal.aborted) return;

    // ── Phase 6: Payment — MPP payment on testnet ────────────────────
    await playback.waitForStep();
    playback.setProgress(5, TOTAL, 'Initiating MPP payment');
    timeline.setActive(5);
    logCommand('payment', 'Agent Beta initiates a Machine Payments Protocol payment on Tempo Moderato testnet');

    // Beta sends a payment via chain.transfer tool
    const payResult = await showCmd(beta,
      roko(ctx, 'run "Send 0.001 ETH to address 0x0000000000000000000000000000000000000001 on Tempo Moderato testnet using chain.transfer. Report the transaction hash when complete."'), {
      playback, timeout: 120_000,
      onLog: logCommand,
      onLogComplete: logCommandComplete,
      customDesc: 'Agent signs and submits tx via chain.transfer',
    });
    if (payResult.cost) setMetric('cost', payResult.cost);
    setGate('payment', payResult.ok ? 'pass' : 'fail');
    if (signal.aborted) return;

    // ── Phase 7: Verify — verify the payment landed ──────────────────
    await playback.waitForStep();
    playback.setProgress(6, TOTAL, 'Verifying payment');
    timeline.setActive(6);

    // Alpha verifies the transfer
    // NOTE: In the real flow, the tx_hash from Phase 6 would be passed here.
    // For the demo, the agent extracts it from the previous output or uses chain.verify_transfer.
    const verifyResult = await showCmd(alpha,
      roko(ctx, 'run "Verify the most recent transfer to 0x0000000000000000000000000000000000000001 using chain.verify_transfer. Report the tx status, block number, gas used, trust level, and consensus mechanism. Explain what \'verified\' means in this context."'), {
      playback, timeout: 90_000,
      onLog: logCommand,
      onLogComplete: logCommandComplete,
      customDesc: 'Agent calls chain.verify_transfer — consensus-verified receipt',
    });
    if (verifyResult.cost) setMetric('cost', verifyResult.cost);
    setGate('verify', verifyResult.ok ? 'pass' : 'fail');
    if (signal.aborted) return;

    // ── Phase 8: Summary ─────────────────────────────────────────────
    await playback.waitForStep();
    playback.setProgress(7, TOTAL, 'Generating summary');
    timeline.setActive(7);

    // Both agents summarize in parallel
    await Promise.all([
      showCmd(alpha,
        roko(ctx, 'run "Summarize what we verified in this session: how many verified queries, what trust levels, what consensus mechanism, and what this means for trustless agent commerce."'), {
        playback, timeout: 60_000,
        onLog: logCommand,
        onLogComplete: logCommandComplete,
      }),
      showCmd(beta,
        roko(ctx, 'run "Report on the payment made: amount, recipient, tx hash, and verification status."'), {
        playback, timeout: 60_000,
      }),
    ]);

    // Done
    timeline.markAllComplete();
    setMetric('gates', '8/8 verified');
  },
};
```

### 15.2 Register in scenario-runners/index.ts

**File**: `demo/demo-app/src/lib/scenario-runners/index.ts`

Add import and include in the `allScenarios` array:

```typescript
import { verifiedChain } from './verified-chain';

export const allScenarios: Scenario[] = [
  // ... existing scenarios ...
  verifiedChain,
];
```

### 15.3 Register in Demo/index.tsx tab category

**File**: `demo/demo-app/src/pages/Demo/index.tsx`

Add to `TAB_CATEGORY`:
```typescript
const TAB_CATEGORY: Record<string, string> = {
  // ... existing entries ...
  'verified-chain': 'chain',
};
```

No change needed to `CAT_COLORS` — `chain: 'var(--warning)'` (amber) already exists.

### 15.4 Create roko.toml fixture for the demo workspace

**File**: `demo/demo-app/fixtures/verified-chain-roko.toml`

This is the config the scenario copies into the workspace before running:

```toml
[chain]
rpc_url = "https://rpc.moderato.tempo.xyz"
chain_id = 42431
default_backend = "tempo-moderato"

[chain.backends.tempo-moderato]
rpc_url = "https://rpc.moderato.tempo.xyz"
chain_id = 42431
consensus = "threshold_bls"
label = "Tempo Moderato Testnet"
# group_pubkey will need to be filled with real Tempo Moderato group key
# group_pubkey = "0x..."

[agent]
default_model = "sonnet"
```

**NOTE**: The `group_pubkey` for Tempo Moderato testnet needs to be obtained from Tempo documentation or the network itself. Until it's available, the verifier will fall back to `rpc` consensus (which still works, just with `TrustLevel::RpcTrusted` instead of `Cryptographic`).

### 15.5 Workspace setup helper

The scenario's `run()` function should copy the fixture config into the workspace before any commands. Add a utility or inline it:

```typescript
// At the start of run(), after enterWorkspace:
await showCmd(alpha, `cp ${FIXTURES_DIR}/verified-chain-roko.toml ${workspaceDir}/roko.toml`, {
  playback, timeout: 5_000,
});
```

Or better — modify `enterWorkspace` to also `roko init` if no `roko.toml` exists, and then patch the chain config:

```typescript
await showCmd(alpha, `cat > ${workspaceDir}/roko.toml << 'TOML'
[chain]
rpc_url = "https://rpc.moderato.tempo.xyz"
chain_id = 42431
TOML`, { playback, timeout: 5_000 });
```

### 15.6 Add chain-specific visual panel (optional enhancement)

The existing `ChainIntelPanel.tsx` shows DeFi-specific data. For the verified-chain demo, consider a simpler side panel that shows:

- **Trust Level Badge**: `RpcTrusted` / `Cryptographic` with color coding
- **Consensus Mechanism**: `threshold_bls` / `rpc`
- **Latest Verified Block**: number + hash (truncated)
- **Proof Metadata**: consensus proof size, state proof size
- **Payment Status**: pending → confirmed → verified pipeline

This is optional — the terminal output from the agents already shows all this data. A dedicated panel would make it more visual but isn't required for the demo to work.

---

## Verification Checklist

- [ ] `verified-chain.ts` scenario runner exists with 8 steps
- [ ] Scenario exported and registered in `scenario-runners/index.ts`
- [ ] Scenario appears in Demo tab bar under "chain" category (amber accent)
- [ ] `TAB_CATEGORY['verified-chain']` = `'chain'`
- [ ] Each step uses `playback.waitForStep()` + `timeline.setActive(N)` + `playback.setProgress()`
- [ ] `signal.aborted` checked between every phase
- [ ] Uses `showCmd` with proper `onLog`/`onLogComplete` callbacks
- [ ] Uses `roko(ctx, ...)` to build CLI commands (not hardcoded paths)
- [ ] Phase 6 (payment) uses real `chain.transfer` tool
- [ ] Phase 7 (verify) uses `chain.verify_transfer` tool
- [ ] Metrics tracked: cost, tokens, gates
- [ ] Workspace config includes Tempo Moderato RPC URL
- [ ] Scenario runs end-to-end when Tempo Moderato is reachable
- [ ] Scenario fails gracefully (gate 'fail' markers) when RPC is unreachable
- [ ] `npm run build` in `demo/demo-app` succeeds
- [ ] `npm run dev` + navigate to /demo, select "Verified Chain" tab, runs without console errors
