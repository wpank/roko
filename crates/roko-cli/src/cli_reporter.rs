//! Structured CLI reporter — a thin facade over [`CliOutput`] that adds a
//! `table` method for consistent tabular output.
//!
//! [`CliReporter`] is the single entry point for all human-readable CLI
//! output. Every command that produces output should obtain one reporter and
//! call its methods instead of mixing `println!`, `eprintln!`, and
//! `tracing::*` in the command handler itself.
//!
//! # Design
//!
//! - Delegates status lines (`success`, `warn`, `error`, etc.) to the
//!   existing [`CliOutput`] / [`output_format`] primitives so formatting
//!   stays consistent across all call sites.
//! - Adds [`CliReporter::table`] which computes column widths and prints a
//!   fixed-width ASCII table to stdout.
//! - Respects the `--quiet` flag: non-error methods are silenced when
//!   `quiet` is `true`, matching the contract of the underlying [`CliOutput`].
//! - Respects the `--json` flag: [`CliReporter::table_json`] emits the same
//!   data as a JSON array of objects so that scripted callers can consume
//!   tabular data without parsing fixed-width text.

use crate::cli_output::CliOutput;
use crate::output_format;
use serde_json::Value;

/// Centralised output formatter for all `roko` CLI commands.
///
/// Obtain an instance via [`CliReporter::new`] or the shorthand
/// [`CliReporter::from_flags`], then call the appropriate method depending on
/// whether you need a status line, a table, or JSON output.
pub struct CliReporter {
    out: CliOutput,
    /// Mirrors the `quiet` flag stored inside `out` so we can read it back
    /// without adding a public accessor to [`CliOutput`].
    quiet: bool,
    /// When `true`, `table` writes nothing and callers should use
    /// `table_json` or `table_or_json` instead.
    json: bool,
}

impl CliReporter {
    /// Create a reporter from explicit `quiet` and `json` flags.
    pub fn new(quiet: bool, json: bool) -> Self {
        Self {
            out: CliOutput::new(quiet),
            quiet,
            json,
        }
    }

    /// Convenience constructor that reads flags from a `bool` pair.
    ///
    /// ```rust,ignore
    /// let reporter = CliReporter::from_flags(cli.quiet, cli.json);
    /// ```
    pub fn from_flags(quiet: bool, json: bool) -> Self {
        Self::new(quiet, json)
    }

    // ── Status lines ─────────────────────────────────────────────────────────

    /// Print the intro line `◆  <title>` in bold. Suppressed when quiet.
    pub fn intro(&self, title: &str) {
        self.out.intro(title);
    }

    /// Print `✔  <msg>` in green. Suppressed when quiet.
    pub fn success(&self, msg: &str) {
        self.out.success(msg);
    }

    /// Print `⚠  <msg>` in yellow. Suppressed when quiet.
    pub fn warn(&self, msg: &str) {
        self.out.warning(msg);
    }

    /// Print `✖  <msg>` in red. Always shown, even when quiet.
    pub fn error(&self, msg: &str) {
        self.out.error(msg);
    }

    /// Print `│  <text>` in dim style as an informational note.
    /// Suppressed when quiet.
    pub fn note(&self, text: &str) {
        self.out.note(text);
    }

    /// Print a step line `◇  <label>  <value>`. Suppressed when quiet.
    pub fn step(&self, label: &str, value: &str) {
        self.out.step(label, value);
    }

    /// Print a continuation line `│  <text>`. Suppressed when quiet.
    pub fn bar(&self, text: &str) {
        self.out.bar(text);
    }

    /// Print `├  <text>`. Suppressed when quiet.
    pub fn branch(&self, text: &str) {
        self.out.branch(text);
    }

    /// Print `└  <text>`. Suppressed when quiet.
    pub fn end(&self, text: &str) {
        self.out.end(text);
    }

    /// Print an empty `│` line (visual spacer). Suppressed when quiet.
    pub fn divider(&self) {
        self.out.divider();
    }

    // ── Table output ─────────────────────────────────────────────────────────

    /// Print a fixed-width table to stdout.
    ///
    /// `headers` is a slice of column header labels.
    /// `rows` is a slice of rows; each row must have the same number of
    /// columns as `headers`.  Extra cells are silently discarded; missing
    /// cells are rendered as empty strings.
    ///
    /// When `quiet` is `true` the entire table (including the header) is
    /// suppressed.  When `json` is `true` the table is not rendered by this
    /// method — call [`table_json`](Self::table_json) instead.
    ///
    /// Column widths are computed as `max(header_len, max_cell_len)`.
    ///
    /// Example output:
    /// ```text
    /// ID               TITLE                                    PROGRESS     STATUS
    /// my-plan          Wire the gate pipeline                   3/3          complete
    /// fix-routing      Fix cascade router category awareness    1/2          running
    /// ```
    pub fn table(&self, headers: &[&str], rows: &[Vec<String>]) {
        if self.quiet || self.json {
            return;
        }
        self.print_table_inner(headers, rows);
    }

    /// Print a table only when not in JSON mode; otherwise emit it as JSON.
    ///
    /// This is the recommended method for commands that support both human and
    /// machine-readable output.  When `json` is active the table is serialised
    /// as `[{"HEADER": "cell", ...}, ...]`.
    pub fn table_or_json(&self, headers: &[&str], rows: &[Vec<String>]) {
        if self.json {
            let json_val = table_to_json(headers, rows);
            println!("{}", serde_json::to_string_pretty(&json_val).unwrap_or_default());
        } else {
            self.table(headers, rows);
        }
    }

    /// Emit the table as a JSON array of objects to stdout, regardless of the
    /// `quiet` flag.  The `json` flag does not gate this method — call it
    /// explicitly when you want JSON output.
    pub fn table_json(&self, headers: &[&str], rows: &[Vec<String>]) {
        let json_val = table_to_json(headers, rows);
        println!("{}", serde_json::to_string_pretty(&json_val).unwrap_or_default());
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn print_table_inner(&self, headers: &[&str], rows: &[Vec<String>]) {
        let ncols = headers.len();
        if ncols == 0 {
            return;
        }

        // Compute column widths.
        let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
        for row in rows {
            for (col, cell) in row.iter().enumerate() {
                if col < ncols {
                    widths[col] = widths[col].max(cell.len());
                }
            }
        }

        // Print header.
        let header_line: String = headers
            .iter()
            .zip(widths.iter())
            .map(|(h, &w)| format!("{h:<w$}"))
            .collect::<Vec<_>>()
            .join("  ");
        println!("{}", output_format::bold(&header_line));

        // Print rows.
        for row in rows {
            let row_line: String = (0..ncols)
                .map(|col| {
                    let cell = row.get(col).map(String::as_str).unwrap_or("");
                    let w = widths[col];
                    format!("{cell:<w$}")
                })
                .collect::<Vec<_>>()
                .join("  ");
            println!("{row_line}");
        }
    }
}

/// Convert a table to a JSON array of objects keyed by header name.
fn table_to_json(headers: &[&str], rows: &[Vec<String>]) -> Value {
    let objects: Vec<Value> = rows
        .iter()
        .map(|row| {
            let map: serde_json::Map<String, Value> = headers
                .iter()
                .enumerate()
                .map(|(i, &header)| {
                    let cell = row.get(i).cloned().unwrap_or_default();
                    (header.to_string(), Value::String(cell))
                })
                .collect();
            Value::Object(map)
        })
        .collect();
    Value::Array(objects)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ──────────────────────────────────────────────────────────

    #[test]
    fn new_smoke() {
        let _r = CliReporter::new(false, false);
    }

    #[test]
    fn from_flags_smoke() {
        let _r = CliReporter::from_flags(true, true);
    }

    // ── table_to_json ─────────────────────────────────────────────────────────

    #[test]
    fn table_to_json_basic() {
        let headers = &["ID", "STATUS"];
        let rows = vec![
            vec!["plan-a".to_string(), "complete".to_string()],
            vec!["plan-b".to_string(), "running".to_string()],
        ];
        let json = table_to_json(headers, &rows);
        let arr = json.as_array().expect("should be array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["ID"], Value::String("plan-a".to_string()));
        assert_eq!(arr[0]["STATUS"], Value::String("complete".to_string()));
        assert_eq!(arr[1]["ID"], Value::String("plan-b".to_string()));
    }

    #[test]
    fn table_to_json_empty_rows() {
        let headers = &["A", "B"];
        let json = table_to_json(headers, &[]);
        assert_eq!(json, Value::Array(vec![]));
    }

    #[test]
    fn table_to_json_short_row_pads_with_empty() {
        let headers = &["A", "B", "C"];
        let rows = vec![vec!["x".to_string()]]; // only 1 cell for 3 columns
        let json = table_to_json(headers, &rows);
        let arr = json.as_array().unwrap();
        assert_eq!(arr[0]["B"], Value::String("".to_string()));
        assert_eq!(arr[0]["C"], Value::String("".to_string()));
    }

    // ── Status method smoke tests (no-panics; quiet=true suppresses output) ──

    #[test]
    fn success_quiet_no_panic() {
        CliReporter::new(true, false).success("done");
    }

    #[test]
    fn warn_quiet_no_panic() {
        CliReporter::new(true, false).warn("heads up");
    }

    #[test]
    fn error_quiet_still_prints_no_panic() {
        // error() is not gated on quiet — must not panic.
        CliReporter::new(true, false).error("something broke");
    }

    #[test]
    fn note_step_bar_branch_end_divider_quiet_no_panic() {
        let r = CliReporter::new(true, false);
        r.intro("Test");
        r.note("a note");
        r.step("Key", "Val");
        r.bar("bar text");
        r.branch("branch text");
        r.end("done");
        r.divider();
    }

    // ── table() is suppressed in json+quiet mode ──────────────────────────────

    #[test]
    fn table_json_mode_does_not_panic() {
        let r = CliReporter::new(false, true);
        r.table(&["A", "B"], &[vec!["x".to_string(), "y".to_string()]]);
        // json=true suppresses table(); no output expected, no panic.
    }

    #[test]
    fn table_quiet_mode_does_not_panic() {
        let r = CliReporter::new(true, false);
        r.table(&["A", "B"], &[vec!["x".to_string(), "y".to_string()]]);
    }

    #[test]
    fn table_empty_headers_no_panic() {
        let r = CliReporter::new(false, false);
        r.table(&[], &[]);
    }

    #[test]
    fn table_or_json_verbose_no_panic() {
        let r = CliReporter::new(false, false);
        r.table_or_json(
            &["ID", "STATUS"],
            &[vec!["plan-a".to_string(), "complete".to_string()]],
        );
    }

    #[test]
    fn table_json_method_no_panic() {
        let r = CliReporter::new(false, false);
        r.table_json(
            &["ID", "STATUS"],
            &[vec!["plan-x".to_string(), "running".to_string()]],
        );
    }
}
