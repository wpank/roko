# Chain Heartbeat Variant

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/02-chain-heartbeat-variant.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS3.3.
>
> Note: Chain-specific DeFi trading is deprecated, but this document preserves
> the general heartbeat parameterization pattern. The SIMULATE/VALIDATE injection
> into VERIFY is the reference pattern for any domain where actions are
> irreversible or financially consequential.

---

## 1. Why Irreversible Actions Need Extra Steps

In the coding domain, most actions are reversible. A bad commit can be reverted.
A broken test can be fixed. The cost of a mistake is time, not money.

In domains with irreversible actions, the consequences are permanent:

| Action | Reversibility | Cost of Mistake |
|---|---|---|
| Submit a swap transaction | Irreversible once mined | Gas + slippage + sandwich attack |
| Provide liquidity to a pool | Reversible with IL | Impermanent loss, fees, gas |
| Deploy a contract | Irreversible (mutable proxy aside) | Gas + deployment cost |
| Interact with a malicious contract | Irreversible | Complete wallet drain possible |
| Publish a signed document | Irreversible (reputation) | Reputational damage |

SIMULATE and VALIDATE provide a safety net before committing irreversible actions.
They are the equivalent of a dry-run compilation: verify the action will succeed
before executing it for real.

---

## 2. The General Pattern: Domain-Specific Gate Injection

The universal loop does not add extra steps. Instead, domain-specific agents inject
additional Gate implementations into the VERIFY phase:

```
Universal Loop          Domain Extension                   Synapse Trait
--------------          ----------------                   ------------
SENSE               1. OBSERVE (domain state)              Substrate.query()
ASSESS              2. SCORE + ROUTE                       Scorer.score() + Router.select()
COMPOSE             3. COMPOSE (context assembly)          Composer.compose()
ACT                 4. EXECUTE (domain action)             Agent.execute()
VERIFY              5. SIMULATE (pre-flight)               [domain-specific Gate]
                    6. VALIDATE (policy/limits)             Gate.verify()
                    7. VERIFY (ground truth)                Gate.verify()
PERSIST             8. PERSIST (store with lineage)         Substrate.put()
BROADCAST           9. BROADCAST (publish Pulses)           Bus.publish()
REACT              10. REACT (episode, predictions)         Policy.decide()
```

### 2.1 SIMULATE (Pre-flight)

Run the proposed action in a local simulation before executing it for real. For
chain agents, this uses an in-process EVM fork. For other irreversible domains,
this might be a sandbox environment, a staging deploy, or a model-based simulation.

What simulation checks:
- **Revert detection**: Will the action fail? If so, why?
- **Cost estimation**: What will the actual cost be? Is it within budget?
- **State change verification**: What side effects will occur? Are they expected?
- **Adversarial vulnerability**: Can the action be exploited?
- **Multi-step simulation**: For complex strategies, simulate the entire sequence.

When SIMULATE is skipped:
- T0 ticks (no action proposed)
- T1 ticks where the LLM recommends no action
- Read-only operations (queries, balance checks)

### 2.2 VALIDATE (Policy Enforcement)

Check the simulated action against safety constraints before execution:

- **Resource limits**: Maximum resource consumption per action.
- **Approved targets**: Only interact with whitelisted targets/protocols.
- **Tolerance thresholds**: Maximum acceptable deviation from expectations.
- **Rate limits**: Maximum actions per time period.
- **Capability tokens**: Typed, unforgeable authorization tokens.
- **Risk assessment**: Multi-layer risk evaluation.

If validation fails, the action is rejected and the agent receives structured
feedback about why. The failure feeds back into the Daimon (decreased Pleasure)
and the CascadeRouter (this strategy path failed).

---

## 3. Observer Mode (Reduced Heartbeat)

A named capability configuration where the agent observes without acting. This
is useful for monitoring, research, and insight generation:

```
OBSERVE  -> Deterministic probes (free, $0.00)
REFLECT  -> LLM analysis of anomalies / regime changes / hypothesis validation
PUBLISH  -> Push confirmed insights to marketplace
```

The observer variant:
- **Never reaches ACT**: No SIMULATE, no VALIDATE, no EXECUTE. Read-only tools.
- **Enhanced dreaming**: ~40% of runtime budget allocated to dreams.
- **Revenue model**: Earns by selling typed knowledge artifacts.

---

## 4. Comparison: Universal vs. Extended vs. Observer

| Step | Universal Loop | Extended (Irreversible) | Observer |
|---|---|---|---|
| SENSE | Substrate.query() | Domain state, probes | Probes only |
| ASSESS | Scorer.score() | Multi-factor scoring | Multi-factor scoring |
| COMPOSE | Composer.compose() | VCG auction | PromptComposer |
| ACT | Agent.execute() | Domain action | _(skip)_ |
| SIMULATE | _(skip)_ | Pre-flight simulation | _(skip)_ |
| VALIDATE | _(skip)_ | Policy enforcement | _(skip)_ |
| VERIFY | Gate.verify() | Domain receipt | _(skip)_ |
| PERSIST | Substrate.put() | On-chain + off-chain | Off-chain only |
| BROADCAST | Bus.publish() | Topic-addressed | Topic-addressed |
| REACT | Policy.decide() | Episode + prediction + C-Factor | Episode + publish |

---

## 5. References

- **Sumers et al. 2023** -- CoALA framework (arXiv:2309.02427).
- **Chen et al. 2023** -- FrugalGPT (arXiv:2305.05176).
- **Kahneman 2011** -- "Thinking, Fast and Slow". Dual-process theory.

---

## Cross-References

- `docs/v3/depth/29-heartbeat/coala-9-step-pipeline.md` -- CoALA foundation
- `docs/v3/depth/29-heartbeat/universal-loop-mapping.md` -- Universal Synapse loop
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
