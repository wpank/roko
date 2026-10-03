//! The controller never reads the disturbance ground truth (S06 §4.9, A4):
//! no file of `roko_learn::homeostasis` imports `roko_core::disturbance` or
//! names the run's `disturbances.jsonl`, and the controller's kind list is
//! the ground truth's.

use std::path::Path;

use roko_core::disturbance::{DISTURBANCES_FILE, DisturbanceKind};
use roko_learn::homeostasis::catalog;

#[test]
fn controller_never_reads_ground_truth() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/homeostasis");
    let forbidden = [
        "roko_core::disturbance",
        DISTURBANCES_FILE,
        "DISTURBANCES_FILE",
        "GroundTruthWriter",
    ];
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("read the homeostasis module") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read a source file");
        for name in forbidden {
            assert!(!source.contains(name), "{} names {name}", path.display());
        }
        checked += 1;
    }
    assert!(checked >= 15, "only {checked} files checked");

    // One kind list: the controller's names are the ground truth's.
    let ground: Vec<&str> = DisturbanceKind::ALL
        .iter()
        .map(|kind| kind.name())
        .collect();
    let controller: Vec<&str> = catalog::DisturbanceKind::DISTURB_PY
        .iter()
        .chain([&catalog::DisturbanceKind::PriceShock])
        .map(|kind| kind.name())
        .collect();
    assert_eq!(controller, ground);
}
