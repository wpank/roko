# C — Gossip And P2P Network

Verdict: `DEFERRED`

The chain-specific gossip and p2p network does not ship today. Topic `08`
should stop describing mesh topology, topics, peer scoring, or sybil
resistance as current system behavior.

## Current Parity Position

- No Korai gossip mesh should be described in present tense.
- No chain-specific p2p runtime should be described in present tense.
- Existing WebSocket or SSE interfaces elsewhere in the repo are not a
  substitute for the deferred gossip design.

## Deferred Items

- four-tier gossip architecture
- topic taxonomy
- peer scoring
- sybil resistance
- mesh membership logic

## Working Rule

If a section depends on libp2p, gossipsub, peer scoring, or a dedicated Korai
network layer, it belongs in future work, not in the shipped-parity story.
