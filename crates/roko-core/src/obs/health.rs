//! Liveness / readiness probes (§40.10 + §40.11 + §40.12).
//!
//! Liveness answers "is this process alive?" (always `Ok` unless
//! catastrophic), readiness answers "can this process serve traffic?"
//! (depends on upstream deps). A failing readiness check carries a
//! structured [`DegradedReason`] so operators can see *why*.
//!
//! Degraded-mode reporting (§40.12): `json_snapshot` returns
//! `{"status":"degraded","degraded":true,"reasons":[…]}` when any probe
//! fails — the embedding HTTP layer returns 200 with that body, rather
//! than hiding the failures behind a 5xx.

use std::fmt;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Liveness verdict. `Ok` = healthy, `Degraded` = running but impaired,
/// `Unhealthy` = should be restarted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Fully healthy.
    Ok,
    /// Running but impaired.
    Degraded,
    /// Should be restarted by the orchestrator.
    Unhealthy,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ok => "ok",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        })
    }
}

/// Readiness verdict. `Ready` = can accept traffic, `NotReady` = should
/// be removed from the load-balancer pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessStatus {
    /// All upstream dependencies reachable.
    Ready,
    /// At least one upstream dependency is unreachable.
    NotReady,
}

impl fmt::Display for ReadinessStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ready => "ready",
            Self::NotReady => "not_ready",
        })
    }
}

/// Structured reason explaining why a probe flagged itself as degraded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DegradedReason {
    /// Component name, e.g. `"mirage-rs"`, `"llm-provider:anthropic"`.
    pub component: String,
    /// Human-readable explanation, e.g. `"connection refused"`.
    pub message: String,
}

impl DegradedReason {
    /// Construct a new `DegradedReason`.
    pub fn new(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for DegradedReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.component, self.message)
    }
}

/// A single readiness check. Implementors are registered with a
/// [`ProbeRegistry`] and invoked whenever readiness is queried.
pub trait Probe: Send + Sync {
    /// Unique name of this probe (for logs / metrics).
    fn name(&self) -> &str;
    /// `Ok(())` when the dependency is healthy, `Err` with a structured
    /// reason when it is not.
    fn check(&self) -> Result<(), DegradedReason>;
}

/// A thread-safe registry of [`Probe`]s. Liveness always returns `Ok`
/// unless no probes are registered (in which case it is still `Ok`).
///
/// Readiness walks every probe and aggregates failures into a list of
/// [`DegradedReason`]s. Any failure demotes the overall status to
/// `NotReady`.
pub struct ProbeRegistry {
    probes: RwLock<Vec<Arc<dyn Probe>>>,
}

impl ProbeRegistry {
    /// Construct an empty probe registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            probes: RwLock::new(Vec::new()),
        }
    }

    /// Register a new probe.
    pub fn register(&self, probe: Arc<dyn Probe>) {
        self.probes.write().push(probe);
    }

    /// Number of registered probes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.probes.read().len()
    }

    /// True if no probes are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.probes.read().is_empty()
    }

    /// Liveness status. Always `Ok` — if the process can answer, it is
    /// alive. Kubernetes-style liveness probes use this to decide whether
    /// to restart the process; we only return non-`Ok` in catastrophic
    /// cases that this registry cannot currently detect (e.g. deadlocked
    /// runtime), but the shape is here for callers that want to set it
    /// externally.
    #[must_use]
    pub const fn liveness(&self) -> HealthStatus {
        HealthStatus::Ok
    }

    /// Readiness status + list of degraded reasons. `Ready` iff every
    /// probe returned `Ok(())`.
    #[must_use]
    pub fn readiness(&self) -> (ReadinessStatus, Vec<DegradedReason>) {
        let mut reasons = Vec::new();
        {
            let probes = self.probes.read();
            for probe in probes.iter() {
                if let Err(reason) = probe.check() {
                    reasons.push(reason);
                }
            }
        }
        let status = if reasons.is_empty() {
            ReadinessStatus::Ready
        } else {
            ReadinessStatus::NotReady
        };
        (status, reasons)
    }

    /// Snapshot as JSON matching §40.12:
    /// `{"status":"ok"|"degraded","degraded":bool,"reasons":[{component,message},…]}`.
    #[must_use]
    pub fn json_snapshot(&self) -> serde_json::Value {
        let (_, reasons) = self.readiness();
        let degraded = !reasons.is_empty();
        serde_json::json!({
            "status": if degraded { "degraded" } else { "ok" },
            "degraded": degraded,
            "reasons": reasons,
        })
    }
}

impl Default for ProbeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Built-in probes ─────────────────────────────────────────────────

/// Probe that always returns `Ok(())`. Useful for liveness wiring and
/// as a placeholder while real probes are implemented.
#[derive(Debug, Clone)]
pub struct AlwaysUpProbe {
    name: String,
}

impl AlwaysUpProbe {
    /// Construct an always-healthy probe with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Probe for AlwaysUpProbe {
    fn name(&self) -> &str {
        &self.name
    }
    fn check(&self) -> Result<(), DegradedReason> {
        Ok(())
    }
}

/// Ad-hoc probe backed by a boxed closure. Handy for wiring a call-site's
/// readiness check without defining a new type.
pub struct NamedProbe {
    /// Probe name (used in logs/metrics).
    pub name: String,
    /// Closure returning the probe verdict.
    pub check_fn: Box<dyn Fn() -> Result<(), DegradedReason> + Send + Sync>,
}

impl NamedProbe {
    /// Wrap a closure as a `Probe`.
    pub fn new(
        name: impl Into<String>,
        check_fn: impl Fn() -> Result<(), DegradedReason> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            check_fn: Box::new(check_fn),
        }
    }
}

impl fmt::Debug for NamedProbe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The `check_fn` field is a closure with no useful Debug output;
        // emitting just the name keeps the impl stable.
        f.debug_struct("NamedProbe")
            .field("name", &self.name)
            .field("check_fn", &"<closure>")
            .finish()
    }
}

impl Probe for NamedProbe {
    fn name(&self) -> &str {
        &self.name
    }
    fn check(&self) -> Result<(), DegradedReason> {
        (self.check_fn)()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_status_display_matches() {
        assert_eq!(HealthStatus::Ok.to_string(), "ok");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
    }

    #[test]
    fn readiness_status_display_matches() {
        assert_eq!(ReadinessStatus::Ready.to_string(), "ready");
        assert_eq!(ReadinessStatus::NotReady.to_string(), "not_ready");
    }

    #[test]
    fn empty_registry_is_ready() {
        let reg = ProbeRegistry::new();
        let (status, reasons) = reg.readiness();
        assert_eq!(status, ReadinessStatus::Ready);
        assert!(reasons.is_empty());
    }

    #[test]
    fn one_failing_probe_makes_not_ready() {
        let reg = ProbeRegistry::new();
        reg.register(Arc::new(NamedProbe::new("db", || {
            Err(DegradedReason::new("db", "connection refused"))
        })));
        let (status, reasons) = reg.readiness();
        assert_eq!(status, ReadinessStatus::NotReady);
        assert_eq!(reasons.len(), 1);
        assert_eq!(reasons[0].component, "db");
        assert_eq!(reasons[0].message, "connection refused");
    }

    #[test]
    fn mixed_probes_report_only_failures() {
        let reg = ProbeRegistry::new();
        reg.register(Arc::new(AlwaysUpProbe::new("up-1")));
        reg.register(Arc::new(NamedProbe::new("mirage", || {
            Err(DegradedReason::new("mirage-rs", "http 503"))
        })));
        reg.register(Arc::new(AlwaysUpProbe::new("up-2")));
        let (status, reasons) = reg.readiness();
        assert_eq!(status, ReadinessStatus::NotReady);
        assert_eq!(reasons.len(), 1);
        assert_eq!(reasons[0].component, "mirage-rs");
    }

    #[test]
    fn liveness_is_ok_even_with_failing_probes() {
        let reg = ProbeRegistry::new();
        reg.register(Arc::new(NamedProbe::new("x", || {
            Err(DegradedReason::new("x", "down"))
        })));
        assert_eq!(reg.liveness(), HealthStatus::Ok);
    }

    #[test]
    fn json_snapshot_ok_when_no_reasons() {
        let reg = ProbeRegistry::new();
        reg.register(Arc::new(AlwaysUpProbe::new("a")));
        let v = reg.json_snapshot();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["degraded"], false);
        assert_eq!(v["reasons"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn json_snapshot_degraded_when_probe_fails() {
        let reg = ProbeRegistry::new();
        reg.register(Arc::new(NamedProbe::new("chain", || {
            Err(DegradedReason::new("chain", "rpc timeout"))
        })));
        let v = reg.json_snapshot();
        assert_eq!(v["status"], "degraded");
        assert_eq!(v["degraded"], true);
        let reasons = v["reasons"].as_array().unwrap();
        assert_eq!(reasons.len(), 1);
        assert_eq!(reasons[0]["component"], "chain");
        assert_eq!(reasons[0]["message"], "rpc timeout");
    }

    #[test]
    fn degraded_reason_display() {
        let r = DegradedReason::new("db", "conn refused");
        assert_eq!(r.to_string(), "db: conn refused");
    }

    #[test]
    fn probe_registry_len_tracks_registrations() {
        let reg = ProbeRegistry::new();
        assert!(reg.is_empty());
        reg.register(Arc::new(AlwaysUpProbe::new("a")));
        reg.register(Arc::new(AlwaysUpProbe::new("b")));
        assert_eq!(reg.len(), 2);
        assert!(!reg.is_empty());
    }
}
