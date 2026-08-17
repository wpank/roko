//! Pure bounded transforms over upstream feed values.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use parking_lot::RwLock;
use serde_json::Value;
use tokio::sync::broadcast;

use crate::error::{Result, RokoError};
use crate::feed_cell::FeedPulse;
use crate::{Body, Pulse};

/// Pure transformation from the latest upstream values to an output value.
pub type FeedTransform = Arc<dyn Fn(&HashMap<String, Value>) -> Result<Value> + Send + Sync>;

/// A derived feed with bounded input history and latest-per-topic state.
pub struct DerivedFeedCell {
    id: String,
    input_topics: HashSet<String>,
    latest: RwLock<HashMap<String, Value>>,
    history: RwLock<VecDeque<FeedPulse>>,
    window: usize,
    transform: FeedTransform,
    output: broadcast::Sender<FeedPulse>,
    sequence: AtomicU64,
    connected: AtomicBool,
}

impl std::fmt::Debug for DerivedFeedCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DerivedFeedCell")
            .field("id", &self.id)
            .field("input_topics", &self.input_topics)
            .field("window", &self.window)
            .finish_non_exhaustive()
    }
}

impl DerivedFeedCell {
    /// Construct a derived feed. `window` is clamped to at least one event.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        input_topics: impl IntoIterator<Item = String>,
        window: usize,
        transform: FeedTransform,
    ) -> Self {
        let (output, _) = broadcast::channel(256);
        Self {
            id: id.into(),
            input_topics: input_topics.into_iter().collect(),
            latest: RwLock::new(HashMap::new()),
            history: RwLock::new(VecDeque::with_capacity(window.max(1))),
            window: window.max(1),
            transform,
            output,
            sequence: AtomicU64::new(0),
            connected: AtomicBool::new(false),
        }
    }

    /// Mark upstream Bus subscriptions active. Subscription ownership stays
    /// with the runtime because the kernel [`Bus`](crate::Bus) deliberately
    /// leaves receiver mechanics backend-specific.
    pub fn connect(&self) {
        self.connected.store(true, Ordering::Release);
    }

    /// Subscribe to transformed outputs.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<FeedPulse> {
        self.output.subscribe()
    }

    /// Accept an upstream pulse, update bounded state, and emit a result once
    /// every required topic has produced at least one value.
    pub fn ingest(&self, pulse: FeedPulse) -> Result<Option<FeedPulse>> {
        if !self.connected.load(Ordering::Acquire) {
            return Err(RokoError::invalid("derived feed is disconnected"));
        }
        if !self.input_topics.contains(&pulse.topic) {
            return Err(RokoError::invalid(format!(
                "unexpected upstream topic: {}",
                pulse.topic
            )));
        }
        self.latest
            .write()
            .insert(pulse.topic.clone(), pulse.payload.clone());
        let mut history = self.history.write();
        history.push_back(pulse);
        while history.len() > self.window {
            history.pop_front();
        }
        drop(history);

        let latest = self.latest.read();
        if !self
            .input_topics
            .iter()
            .all(|topic| latest.contains_key(topic))
        {
            return Ok(None);
        }
        let payload = (self.transform)(&latest)?;
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let derived = FeedPulse::new(
            &self.id,
            format!("feed:{}:data", self.id),
            sequence,
            payload,
        );
        let _ = self.output.send(derived.clone());
        Ok(Some(derived))
    }

    /// Consume an event delivered by a canonical Bus subscription.
    pub fn ingest_bus_pulse(&self, pulse: &Pulse) -> Result<Option<FeedPulse>> {
        let payload = match &pulse.body {
            Body::Json(value) => value.clone(),
            _ => {
                return Err(RokoError::invalid(
                    "derived feed Bus input must have a JSON body",
                ));
            }
        };
        let topic = pulse.topic.to_string();
        let feed_id = topic
            .strip_prefix("feed:")
            .and_then(|value| value.strip_suffix(":data"))
            .unwrap_or(&topic);
        self.ingest(FeedPulse {
            feed_id: feed_id.to_string(),
            topic,
            sequence: pulse.seq,
            payload,
            emitted_at_ms: pulse.created_at_ms,
            metadata: pulse.tags.clone(),
        })
    }

    /// Current bounded event count.
    #[must_use]
    pub fn buffered_len(&self) -> usize {
        self.history.read().len()
    }

    /// Whether all upstream topics have produced at least one value.
    #[must_use]
    pub fn healthy(&self) -> bool {
        let latest = self.latest.read();
        self.input_topics
            .iter()
            .all(|topic| latest.contains_key(topic))
    }

    /// Clear subscriptions' cached data on disconnect.
    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::Release);
        self.latest.write().clear();
        self.history.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn waits_for_all_inputs_and_bounds_history() {
        let derived = DerivedFeedCell::new(
            "average",
            ["left".to_string(), "right".to_string()],
            2,
            Arc::new(|values| {
                let left = values["left"].as_f64().unwrap();
                let right = values["right"].as_f64().unwrap();
                Ok(json!((left + right) / 2.0))
            }),
        );
        derived.connect();
        assert!(
            derived
                .ingest(FeedPulse::new("a", "left", 0, json!(2)))
                .unwrap()
                .is_none()
        );
        let output = derived
            .ingest(FeedPulse::new("b", "right", 0, json!(4)))
            .unwrap()
            .unwrap();
        assert_eq!(output.payload, json!(3.0));
        derived
            .ingest(FeedPulse::new("a", "left", 1, json!(6)))
            .unwrap();
        assert_eq!(derived.buffered_len(), 2);
        assert!(derived.healthy());
        derived.disconnect();
        assert!(!derived.healthy());
    }
}
