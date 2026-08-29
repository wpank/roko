//! Integration tests for example graph parsing.

#[test]
fn all_example_graphs_parse_successfully() {
    let examples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .join("examples")
        .join("graphs");

    let mut failed = Vec::new();
    for entry in std::fs::read_dir(&examples_dir).expect("examples/graphs/ must exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            if let Err(e) = roko_graph::loader::load_from_str(&content) {
                failed.push(format!("{}: {}", path.display(), e));
            }
        }
    }

    if !failed.is_empty() {
        panic!(
            "{} example graph(s) failed to parse:\n{}",
            failed.len(),
            failed.join("\n")
        );
    }
}
