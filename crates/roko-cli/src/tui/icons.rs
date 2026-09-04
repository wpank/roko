//! Centralized unicode icon constants for the TUI.
//!
//! Every status icon, separator, and progress character used across views and
//! widgets is defined here so the visual language stays consistent.

// ── Status indicators ────────────────────────────────────────────────────

/// Success / done / passed.
pub const SUCCESS: &str = "\u{2713}"; // checkmark

/// Failure / error / blocked.
pub const FAILURE: &str = "\u{2717}"; // X mark

/// Active / running.
pub const ACTIVE: &str = "\u{25b6}"; // play triangle

/// Pending / waiting / idle.
pub const PENDING: &str = "\u{25cb}"; // open circle

/// Warning / degraded.
pub const WARNING: &str = "\u{26a0}"; // warning triangle

// ── Separators ───────────────────────────────────────────────────────────

/// Text separator between items in a line (use with surrounding spaces).
pub const SEP_DOT: &str = "\u{00b7}"; // middle dot

/// Column / section separator.
pub const SEP_BAR: &str = "\u{2502}"; // box-drawing vertical

/// Horizontal rule character.
pub const HRULE: char = '\u{2500}'; // box-drawing horizontal

/// Truncation indicator.
pub const ELLIPSIS: &str = "\u{2026}"; // horizontal ellipsis

/// Missing / not-applicable value placeholder.
pub const EM_DASH: &str = "\u{2014}"; // em dash

// ── Progress bar characters ──────────────────────────────────────────────

/// Filled portion of a progress bar.
pub const BAR_FILLED: &str = "\u{2588}"; // full block

/// Empty portion of a progress bar.
pub const BAR_EMPTY: &str = "\u{2500}"; // horizontal line (thin, clean)

/// Filled block as char (for push operations).
pub const BAR_FILLED_CHAR: char = '\u{2588}';

/// Empty block as char (for push operations).
pub const BAR_EMPTY_CHAR: char = '\u{2500}';

// ── Navigation ───────────────────────────────────────────────────────────

/// Collapsed tree / right-pointing indicator.
pub const COLLAPSED: &str = "\u{25b8}"; // small right triangle

/// Expanded tree / down-pointing indicator.
pub const EXPANDED: &str = "\u{25be}"; // small down triangle

/// Scroll-up indicator.
pub const SCROLL_UP: &str = "\u{25b2}"; // up triangle

/// Scroll-down indicator.
pub const SCROLL_DOWN: &str = "\u{25bc}"; // down triangle

// ── Heartbeat / alive indicators ─────────────────────────────────────────

/// Filled circle (heartbeat on / active).
pub const CIRCLE_FILLED: &str = "\u{25cf}";

/// Open circle (heartbeat off / idle).
pub const CIRCLE_OPEN: &str = "\u{25cb}";

// ── Error category icons (from error_digest) ─────────────────────────────

/// Gate failure.
pub const ERR_GATE: &str = "\u{2717}"; // X mark (unified with FAILURE)

/// Compile error.
pub const ERR_COMPILE: &str = "\u{2692}"; // hammer and pick

/// Agent error.
pub const ERR_AGENT: &str = "\u{26a0}"; // warning triangle

/// Preflight error.
pub const ERR_PREFLIGHT: &str = "\u{2691}"; // flag

/// Runtime error.
pub const ERR_RUNTIME: &str = "\u{26a1}"; // lightning

// ── Misc ─────────────────────────────────────────────────────────────────

/// Stopwatch / elapsed time marker.
pub const STOPWATCH: &str = "\u{23F1}";

/// Right arrow (transition / suggestion).
pub const ARROW_RIGHT: &str = "\u{2192}";

/// Up arrow (network upload / trend up).
pub const ARROW_UP: &str = "\u{2191}";

/// Down arrow (network download / trend down).
pub const ARROW_DOWN: &str = "\u{2193}";
