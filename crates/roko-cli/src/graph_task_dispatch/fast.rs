//! FAST lane bounds on Graph task attempts (gap-4a6dcb). A FAST run's
//! dispatcher carries its [`FastAttemptBounds`]: each attempt gets the FAST
//! turn cap, at most 90 s and a patch-only system prompt section, its simple
//! cargo verify commands build in the FAST profile, and a failed verify never
//! runs `cargo fix`.

use super::*;
use crate::graph_execution::fast_lane::FastAttemptBounds;

impl GraphTaskDispatcher {
    /// Bound every attempt of this run as FAST mode does; `None`, the
    /// default, leaves attempts as configured.
    #[must_use]
    pub fn with_fast_bounds(mut self, fast: Option<FastAttemptBounds>) -> Self {
        self.fast = fast;
        self
    }

    /// Bound `request` as this run's FAST mode does, if it is a FAST run.
    pub(super) fn fast_bound(&self, request: &mut AgentDispatchRequest) {
        if let Some(fast) = &self.fast {
            fast.bound(request);
        }
    }

    /// [`Self::fast_bound`], taking and returning `request`.
    pub(super) fn fast_bounded(&self, mut request: AgentDispatchRequest) -> AgentDispatchRequest {
        self.fast_bound(&mut request);
        request
    }

    /// The command a verify step runs: in a FAST run a simple cargo command
    /// builds in the FAST profile; anything else runs as authored.
    pub(super) fn verify_command(&self, command: &str) -> String {
        self.fast
            .as_ref()
            .and_then(|fast| fast.verify_command(command))
            .unwrap_or_else(|| command.to_string())
    }

    /// Whether a failed verify may run `cargo fix`: as `[gates]
    /// cargo_fix_enabled` says, and never in a FAST run.
    pub(super) fn auto_fix_enabled(&self) -> bool {
        self.config.gates.cargo_fix_enabled && self.fast.is_none()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{batch_ctx, make_batch_dispatcher, make_spec};

    /// The `--max-turns` a recorded provider call asked for.
    fn max_turns_arg(args: &str) -> Option<&str> {
        let mut words = args.split_whitespace();
        words.find(|word| *word == "--max-turns")?;
        words.next()
    }

    /// gap-4a6dcb: a FAST dispatch asks for at most the FAST turn cap and
    /// tells the agent to patch and hand off; without FAST nothing changes.
    /// (The 90 s attempt cap is `capped` in `fast_lane`'s tests.)
    #[tokio::test]
    async fn graph_fast_mode_bounds_dispatch() {
        let fast = FastAttemptBounds {
            max_turns: 6,
            cargo_profile: None,
        };
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |_| {}).await;
        let dispatcher = dispatcher.with_fast_bounds(Some(fast));
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
            .await
            .expect("dispatch");
        let args = std::fs::read_to_string(temp.path().join("provider-args")).expect("args");
        assert_eq!(max_turns_arg(&args), Some("6"), "provider args: {args}");
        assert!(
            args.contains("FAST implementation mode"),
            "provider args: {args}"
        );

        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |_| {}).await;
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
            .await
            .expect("dispatch");
        let args = std::fs::read_to_string(temp.path().join("provider-args")).expect("args");
        assert_eq!(max_turns_arg(&args), Some("60"), "provider args: {args}");
        assert!(
            !args.contains("FAST implementation mode"),
            "provider args: {args}"
        );
    }

    /// gap-4a6dcb: in a FAST run a simple cargo verify command builds in the
    /// `dev-fast` profile, a composed one runs as authored, and a failed
    /// verify never runs `cargo fix`, whatever `[gates] cargo_fix_enabled`
    /// says.
    #[tokio::test]
    async fn graph_fast_mode_verify_uses_dev_fast_without_autofix() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, _) = make_batch_dispatcher(&temp, 0.01, |config| {
            config.gates.cargo_fix_enabled = true;
        })
        .await;
        assert!(dispatcher.auto_fix_enabled());
        assert_eq!(
            dispatcher.verify_command("cargo test -p demo"),
            "cargo test -p demo"
        );

        let fast = FastAttemptBounds {
            max_turns: 6,
            cargo_profile: Some("dev-fast"),
        };
        let dispatcher = dispatcher.with_fast_bounds(Some(fast));
        assert!(!dispatcher.auto_fix_enabled());
        assert_eq!(
            dispatcher.verify_command("cargo test -p demo"),
            "cargo test -p demo --profile dev-fast"
        );
        assert_eq!(
            dispatcher.verify_command("cargo test && echo done"),
            "cargo test && echo done"
        );
    }
}
