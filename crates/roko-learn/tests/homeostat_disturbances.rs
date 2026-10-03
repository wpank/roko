//! M1's disturbance acceptance suite at $0 (S06 §1 C1, C2, C5 and C7):
//! every canonical disturbance, from 8119's kind list, against the
//! controller on 8115's synthetic streams and 8117's full-information
//! table. Only this harness knows the kind; the controller sees
//! resolutions.
//!
//! - C1: the detectors confirm the step of each regulable kind with a
//!   median delay of at most 10 resolutions over 50 seeds.
//! - C2 (shadow, guided moves only: an Ashby step is random by design): the
//!   first move of the episode the step opens lies in the kind's
//!   requisite-variety row, and no locked knob is proposed. Two kinds need
//!   more than the catalog-sign prior. Convention flip shows model swap's
//!   breach (E1 low, nothing else), so its first move can only lie in the
//!   union of the two rows until M3's priors tell them apart (8121); and a
//!   budget cut's signature (E2 high, attempts ending on the budget) forms
//!   only as its successes fall, after E1 confirms, so its row is checked
//!   over the whole episode.
//! - C5 (on mode): `flaky_verify` and every provider slowed end in HOLD
//!   within N_max = 6 changes, with no verification-reducing move.
//! - C7 (on mode): after a budget cut, E2 comes back in bounds with E1 and
//!   E3 not breached for `recover_window` resolutions, or the controller
//!   holds and alerts.

use std::collections::BTreeMap;

use roko_core::config::harness_params::{HarnessLadders, HarnessParams, Knob, Step};
use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};
use roko_core::disturbance::DisturbanceKind as GroundTruthKind;
use roko_learn::homeostasis::catalog::{DisturbanceKind, REQUISITE_VARIETY, RequisiteRow};
use roko_learn::homeostasis::controller::{
    ChangeProposal, Controller, ControllerEvent, EpisodeOutcome,
};
use roko_learn::homeostasis::detect::{DetectorBank, DetectorTuning, Observation};
use roko_learn::homeostasis::policy::ChangeKind;
use roko_learn::homeostasis::replay::{OutcomeTable, SyntheticTable};
use roko_learn::homeostasis::streams::{IN_CONTROL, StepKind, SyntheticStream, replay_theta0};
use serde_json::Value;

/// The last in-control position of every stream.
const ONSET: u64 = 20;

/// The kind of `name` in 8119's list, as a synthetic step.
fn step(kind: GroundTruthKind) -> StepKind {
    StepKind::parse(kind.name()).expect("every canonical kind has a synthetic stream")
}

fn every_provider() -> StepKind {
    StepKind::parse(StepKind::ALL_PROVIDERS).expect("provider_fault on every provider")
}

fn row(step: StepKind) -> &'static RequisiteRow {
    let rows: &'static [RequisiteRow] = &REQUISITE_VARIETY;
    rows.iter()
        .find(|row| row.kind == step.kind && row.all_providers == step.all_providers)
        .expect("every kind has a requisite-variety row")
}

/// Resolutions from the onset to the detectors' first confirmed breach; a
/// confirmation before the onset restarts them, as an applied change would.
fn detection_delay(step: StepKind, seed: u64) -> u64 {
    let stream = SyntheticStream {
        step,
        onset: ONSET,
        seed,
    };
    let policy = stream.policy();
    let tuning = DetectorTuning::default();
    let mut bank = DetectorBank::new(IN_CONTROL, &policy.ev, &tuning);
    for position in 1..=ONSET + 60 {
        let observation = Observation::of(&stream.resolution(position));
        if bank.observe(&observation, None).confirmed.is_empty() {
            continue;
        }
        if position <= ONSET {
            bank.reset_all();
        } else {
            return position - ONSET;
        }
    }
    999
}

/// A controller over `step`'s stream, and the table it reads.
fn setup(
    step: StepKind,
    seed: u64,
    mode: HomeostasisMode,
    random: f64,
) -> (Controller, SyntheticTable) {
    let (theta0, ladders) = replay_theta0();
    let stream = SyntheticStream {
        step,
        onset: ONSET,
        seed,
    };
    let settings = HomeostasisConfig {
        mode,
        random_step_prob: random,
        ..HomeostasisConfig::default()
    };
    let controller = Controller::new(
        &settings,
        stream.policy(),
        theta0.clone(),
        ladders.clone(),
        IN_CONTROL,
        seed,
    );
    (controller, SyntheticTable::new(stream, ONSET + 200, theta0, ladders))
}

/// Run the controller over `length` positions: in `on` mode each position
/// is the table's outcome under the controller's θ, in shadow under θ₀.
/// Returns each event with its position and whether anything was breached
/// after it.
fn drive(
    controller: &mut Controller,
    table: &SyntheticTable,
    length: u64,
) -> (Vec<(u64, ControllerEvent)>, Vec<bool>) {
    let theta0 = controller.state().theta0.clone();
    let mut events = Vec::new();
    let mut calm = Vec::new();
    for position in 1..=length {
        let theta = if controller.mode() == HomeostasisMode::On {
            controller.theta().clone()
        } else {
            theta0.clone()
        };
        let resolution = table.outcome(position, &theta);
        for event in controller.on_resolution(&resolution) {
            events.push((position, event));
        }
        calm.push(controller.breached().is_empty());
    }
    (events, calm)
}

/// The events of the first episode after the onset, with the position it
/// starts at: from the first episode opened after the onset, or else from
/// the first move of one still open from before it, to its close or the end.
fn first_episode(events: &[(u64, ControllerEvent)]) -> Option<(u64, Vec<ControllerEvent>)> {
    let opened = events.iter().position(|(position, event)| {
        *position > ONSET && matches!(event, ControllerEvent::EpisodeOpen { .. })
    });
    let ongoing = || {
        events
            .iter()
            .position(|(position, event)| *position > ONSET && is_search(event))
    };
    let start = opened.or_else(ongoing)?;
    let mut episode = Vec::new();
    for (_, event) in &events[start..] {
        episode.push(event.clone());
        if matches!(event, ControllerEvent::EpisodeClose { .. }) {
            break;
        }
    }
    Some((events[start].0, episode))
}

fn is_search(event: &ControllerEvent) -> bool {
    matches!(event, ControllerEvent::Change(change) if change.kind == ChangeKind::Search)
}

/// The search moves among `events`.
fn moves(events: &[ControllerEvent]) -> Vec<ChangeProposal> {
    events
        .iter()
        .filter_map(|event| match event {
            ControllerEvent::Change(change) if change.kind == ChangeKind::Search => {
                Some(change.as_ref().clone())
            }
            _ => None,
        })
        .collect()
}

/// The direction of a move along its knob's ladder.
fn direction(change: &ChangeProposal, ladders: &HarnessLadders) -> Step {
    let ladder = HarnessParams::ladder(change.knob, ladders);
    let at = |value: &Value| ladder.iter().position(|notch| notch == value);
    if at(&change.to) > at(&change.from) {
        Step::Up
    } else {
        Step::Down
    }
}

fn in_row(change: &ChangeProposal, row: &RequisiteRow, ladders: &HarnessLadders) -> bool {
    let direction = direction(change, ladders);
    row.counters.contains(&(change.knob.kind(), direction))
}

/// No knob is moved after its search moves changed direction twice.
fn no_locked_knob(changes: &[ChangeProposal], ladders: &HarnessLadders) -> bool {
    let mut history: BTreeMap<Knob, Vec<Step>> = BTreeMap::new();
    for change in changes {
        let steps = history.entry(change.knob).or_default();
        let flips = steps.windows(2).filter(|pair| pair[0] != pair[1]).count();
        if flips >= 2 {
            return false;
        }
        steps.push(direction(change, ladders));
    }
    true
}

#[test]
fn canonical_disturbances_meet_c1_c2_c5_c7() {
    let (_, ladders) = replay_theta0();
    let regulable: Vec<StepKind> = GroundTruthKind::DISTURB_PY
        .into_iter()
        .filter(|&kind| kind != GroundTruthKind::FlakyVerify)
        .map(step)
        .collect();
    assert_eq!(regulable.len(), 5);

    // C1: median confirmed-breach delay of at most 10 over 50 seeds.
    for &kind in &regulable {
        let mut delays: Vec<u64> = (1..=50).map(|seed| detection_delay(kind, seed)).collect();
        delays.sort_unstable();
        let median = (delays[24] + delays[25]) as f64 / 2.0;
        assert!(median <= 10.0, "{}: median delay {median}", kind.name());
    }

    // C2: the first move of the episode the step opens, guided moves only,
    // in shadow mode.
    let swap_row = row(StepKind::of(DisturbanceKind::ModelSwap));
    for &kind in &regulable {
        let kind_row = row(kind);
        for seed in 1..=20 {
            let (mut controller, table) = setup(kind, seed, HomeostasisMode::Shadow, 0.0);
            let (events, _) = drive(&mut controller, &table, ONSET + 100);
            let (_, episode) = first_episode(&events)
                .unwrap_or_else(|| panic!("{} seed {seed}: no episode", kind.name()));
            let proposed = moves(&episode);
            let first = proposed.first().expect("the episode moves");
            assert!(no_locked_knob(&proposed, &ladders), "{}: {proposed:?}", kind.name());
            let placed = match kind.kind {
                DisturbanceKind::ConventionFlip => {
                    in_row(first, kind_row, &ladders) || in_row(first, swap_row, &ladders)
                }
                DisturbanceKind::BudgetCut => proposed
                    .iter()
                    .any(|change| in_row(change, kind_row, &ladders)),
                _ => in_row(first, kind_row, &ladders),
            };
            assert!(
                placed,
                "{} seed {seed}: {:?}",
                kind.name(),
                proposed
                    .iter()
                    .map(|change| (change.knob, direction(change, &ladders)))
                    .collect::<Vec<_>>()
            );
        }
    }

    // C5: the unregulable kinds HOLD within N_max changes, and no move
    // lowers verification.
    let unregulable = [step(GroundTruthKind::FlakyVerify), every_provider()];
    for kind in unregulable {
        for seed in 1..=10 {
            let (mut controller, table) = setup(kind, seed, HomeostasisMode::On, 0.2);
            let (events, _) = drive(&mut controller, &table, ONSET + 120);
            let (_, episode) = first_episode(&events)
                .unwrap_or_else(|| panic!("{} seed {seed}: no episode", kind.name()));
            let held = episode.iter().any(|event| {
                matches!(
                    event,
                    ControllerEvent::EpisodeClose {
                        outcome: EpisodeOutcome::Hold,
                        changes,
                        ..
                    } if *changes <= 6
                )
            });
            assert!(held, "{} seed {seed}: {episode:?}", kind.name());
            for change in moves(&episode) {
                assert!(change.verdict.passed(), "{change:?}");
                let lowers = matches!(change.knob, Knob::ExtraRungs | Knob::AuditBoost)
                    && direction(&change, &ladders) == Step::Down;
                assert!(!lowers, "{} lowered verification: {change:?}", kind.name());
            }
        }
    }

    // C7: after a budget cut, a calm stretch (E2 back in bounds, E1 and E3
    // not breached) of recover_window resolutions follows the first move,
    // or the controller holds.
    let cut = step(GroundTruthKind::BudgetCut);
    let recover = HomeostasisConfig::default().recover_window as usize;
    for seed in 1..=10 {
        let (mut controller, table) = setup(cut, seed, HomeostasisMode::On, 0.2);
        let (events, calm) = drive(&mut controller, &table, ONSET + 200);
        let (opened, episode) = first_episode(&events).expect("the budget cut opens an episode");
        let held = episode
            .iter()
            .any(|event| matches!(event, ControllerEvent::Hold { .. }));
        let after = &calm[usize::try_from(opened).unwrap_or(usize::MAX).min(calm.len())..];
        let mut streak = 0;
        let settled = after.iter().any(|&quiet| {
            streak = if quiet { streak + 1 } else { 0 };
            streak >= recover
        });
        assert!(held || settled, "budget_cut seed {seed}: {episode:?}");
    }
}
