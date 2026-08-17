//! Local peer-discovery table used by a future gossip transport.
//!
//! This module intentionally performs no networking. It validates and stores
//! announcements, supports capability discovery, and expires stale peers so a
//! relay/libp2p adapter can remain a thin transport layer.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::phase2::u256;

/// A peer advertisement stored in the local discovery table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    /// ERC-8004 passport ID.
    pub passport_id: u256,
    /// Advertised service or relay endpoints.
    pub endpoints: Vec<String>,
    /// Capability bitmask from `agent_registry`.
    pub capabilities: u64,
    /// Unix timestamp of the newest authenticated announcement.
    pub last_seen: u64,
    /// Optional HDC fingerprint for private capability matching.
    pub hdc_fingerprint: Option<Vec<u64>>,
}

/// Transport-neutral gossip messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GossipMessage {
    /// Advertise or refresh a peer.
    Announce {
        /// Peer advertisement.
        peer: PeerInfo,
    },
    /// Query for peers matching any requested capability bit.
    Query {
        /// Capability bitmask.
        capability: u64,
    },
    /// Respond with matching peer records.
    Response {
        /// Matching peers.
        peers: Vec<PeerInfo>,
    },
    /// Refresh the liveness timestamp for a known identity.
    Heartbeat {
        /// Sending passport.
        passport_id: u256,
        /// Authenticated sender timestamp.
        timestamp: u64,
    },
}

/// Outcome of registering a peer announcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterPeerOutcome {
    /// The passport was not previously known.
    Inserted,
    /// A newer or equal timestamp replaced the existing record.
    Updated,
    /// A stale out-of-order announcement was ignored.
    IgnoredStale,
}

/// In-memory peer table with deterministic queries and TTL expiry.
#[derive(Debug, Clone)]
pub struct PeerRegistry {
    peers: HashMap<u256, PeerInfo>,
    /// Seconds after `last_seen` before a peer expires.
    pub ttl_secs: u64,
}

impl PeerRegistry {
    /// Create an empty peer table.
    #[must_use]
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            peers: HashMap::new(),
            ttl_secs,
        }
    }

    /// Insert or update an announcement.
    ///
    /// Out-of-order announcements cannot roll a live record back to stale
    /// endpoints or capabilities.
    pub fn register_peer(&mut self, info: PeerInfo) -> RegisterPeerOutcome {
        match self.peers.get(&info.passport_id) {
            None => {
                self.peers.insert(info.passport_id, info);
                RegisterPeerOutcome::Inserted
            }
            Some(existing) if info.last_seen < existing.last_seen => {
                RegisterPeerOutcome::IgnoredStale
            }
            Some(_) => {
                self.peers.insert(info.passport_id, info);
                RegisterPeerOutcome::Updated
            }
        }
    }

    /// Apply a heartbeat only when it does not move time backwards.
    #[must_use]
    pub fn heartbeat(&mut self, passport_id: u256, timestamp: u64) -> bool {
        let Some(peer) = self.peers.get_mut(&passport_id) else {
            return false;
        };
        if timestamp < peer.last_seen {
            return false;
        }
        peer.last_seen = timestamp;
        true
    }

    /// Return peers matching at least one requested capability bit.
    #[must_use]
    pub fn query_peers_by_capability(&self, capability: u64) -> Vec<PeerInfo> {
        if capability == 0 {
            return Vec::new();
        }
        let mut peers: Vec<_> = self
            .peers
            .values()
            .filter(|peer| peer.capabilities & capability != 0)
            .cloned()
            .collect();
        peers.sort_by_key(|peer| peer.passport_id);
        peers
    }

    /// Remove peers whose age is strictly greater than the configured TTL.
    ///
    /// Returns removed passport IDs in deterministic order.
    pub fn expire_stale(&mut self, now: u64) -> Vec<u256> {
        let mut expired: Vec<_> = self
            .peers
            .iter()
            .filter_map(|(passport_id, peer)| {
                (now.saturating_sub(peer.last_seen) > self.ttl_secs).then_some(*passport_id)
            })
            .collect();
        expired.sort_unstable();
        for passport_id in &expired {
            self.peers.remove(passport_id);
        }
        expired
    }

    /// Fetch a peer by passport ID.
    #[must_use]
    pub fn get(&self, passport_id: u256) -> Option<&PeerInfo> {
        self.peers.get(&passport_id)
    }

    /// Number of live table entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    /// Whether the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }
}

impl Default for PeerRegistry {
    fn default() -> Self {
        Self::new(300)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_registry::{CAP_INFERENCE, CAP_SECURITY, CAP_TRADING};

    fn peer(passport_id: u256, capabilities: u64, last_seen: u64) -> PeerInfo {
        PeerInfo {
            passport_id,
            endpoints: vec![format!("https://peer-{passport_id}.example")],
            capabilities,
            last_seen,
            hdc_fingerprint: None,
        }
    }

    #[test]
    fn register_query_and_expiry_are_deterministic() {
        let mut registry = PeerRegistry::new(60);
        assert_eq!(
            registry.register_peer(peer(2, CAP_TRADING, 100)),
            RegisterPeerOutcome::Inserted
        );
        registry.register_peer(peer(1, CAP_INFERENCE | CAP_SECURITY, 100));

        let matches = registry.query_peers_by_capability(CAP_INFERENCE | CAP_TRADING);
        assert_eq!(
            matches
                .iter()
                .map(|peer| peer.passport_id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert!(registry.expire_stale(160).is_empty());
        assert_eq!(registry.expire_stale(161), vec![1, 2]);
        assert!(registry.is_empty());
    }

    #[test]
    fn stale_announcement_and_heartbeat_cannot_rewind_peer() {
        let mut registry = PeerRegistry::new(60);
        registry.register_peer(peer(1, CAP_INFERENCE, 100));
        assert_eq!(
            registry.register_peer(peer(1, CAP_TRADING, 99)),
            RegisterPeerOutcome::IgnoredStale
        );
        assert_eq!(registry.get(1).unwrap().capabilities, CAP_INFERENCE);
        assert!(!registry.heartbeat(1, 90));
        assert!(registry.heartbeat(1, 120));
        assert_eq!(registry.get(1).unwrap().last_seen, 120);
        assert!(!registry.heartbeat(999, 120));
    }

    #[test]
    fn zero_capability_query_never_matches_everyone() {
        let mut registry = PeerRegistry::default();
        registry.register_peer(peer(1, CAP_INFERENCE, 1));
        assert!(registry.query_peers_by_capability(0).is_empty());
    }

    #[test]
    fn gossip_message_round_trips_json() {
        let message = GossipMessage::Heartbeat {
            passport_id: 42,
            timestamp: 100,
        };
        let json = serde_json::to_string(&message).unwrap();
        assert_eq!(
            serde_json::from_str::<GossipMessage>(&json).unwrap(),
            message
        );
    }
}
