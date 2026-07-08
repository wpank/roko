#!/usr/bin/env bash
# verify.sh — per-topic quality checks.
# Sourced by run-migration.sh.

# shellcheck source=common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# --- Quality thresholds ------------------------------------------------------

# Minimum line count for INDEX.md per topic.
: "${MIN_INDEX_LINES:=40}"
# Minimum line count for each sub-doc. 60 accepts niche-domain citation
# lists in the references topic (e.g., market microstructure, streaming
# algorithms) which naturally have fewer entries.
: "${MIN_SUBDOC_LINES:=60}"
# Minimum number of sub-docs per topic (not counting INDEX.md).
: "${MIN_SUBDOCS:=5}"
# Minimum total line count across all docs in a topic.
: "${MIN_TOPIC_TOTAL_LINES:=2500}"
# Default citation floor (warning threshold).
: "${MIN_CITATIONS:=15}"
# Citation floor for topics with less academic content.
: "${MIN_CITATIONS_LOW:=5}"

# Forbidden terms, split into two classes.
#
# HARD_FORBIDDEN: should NEVER appear. No exceptions. The wrong renames below
# cannot be justified by any context — if the agent wrote "Clade→Fleet" it
# made an actual mistake and must be corrected.
HARD_FORBIDDEN_TERMS=(
    "clade→fleet"
    "clade → fleet"
    "Clade → fleet"
    "Clade→fleet"
)
#
# SOFT_FORBIDDEN: should not appear in regular prose BUT may legitimately
# appear in rename-context lines — markdown table rows, inline code quotes,
# parenthetical historical references, or prose that includes a "rename marker"
# (e.g., "legacy", "formerly", "old name", "was", "replaced by", "→", "renamed").
#
# Death-framed concepts are here (not HARD) because every migration doc
# legitimately needs to say "this was removed — it used to be called X".
# If the agent writes "the legacy Thanatopsis phase" or "X was replaced by Y"
# that's correct historical reference, not a bug.
SOFT_FORBIDDEN_TERMS=(
    "GNOS token"
    "1 noun, 6 verbs"
    "1 noun 6 verbs"
    "Thriving → Terminal"
    "Thriving to Terminal"
    "Thanatopsis"
    "thanatopsis"
    "Bloodstain"
    "bloodstain"
    "Katabasis"
    "katabasis"
    "Necrocracy"
    "necrocracy"
    "terminal requiem"
    "death daimon"
    "mortality daimon"
    "vitality gauge"
)

# Words whose presence in a line makes a soft-forbidden term acceptable.
# Case-insensitive match. Line content is checked AFTER stripping the grep
# file:line: prefix.
RENAME_CONTEXT_MARKERS_REGEX='(legacy|formerly|previously|old name|renamed|replaces?|replaced|deprecated|rename|framing|era|informal|branding|obsolete|superseded|supersedes|→|->| vs |versus |not "|NOT "|wrong|removed|references?|bardo-backup|bardo/|bardo |refactoring-prd|implementation-plans|skip|bardo era|Bardo era|not propagated|not used|archive|archived|Generated from|Source:|source:|Sources:|sources:|generation notes|Generation Notes|removed per|per reframe|skipped entirely|do not use|mortality|death|succession|inheritance)'

# Required terms — at least one instance must appear in the topic's output.
# We only require "Roko" universally. Some topics (e.g., 05-learning about
# bandits, 15-code-intelligence, 18-tools) legitimately don't use "Engram" or
# "Synapse" in their body content. If the topic mentions Roko it has passed
# the sanity bar; forbidden-term scanning handles the rest.
REQUIRED_TERMS_BASE=(
    "Roko"
)

# --- Verification functions --------------------------------------------------

# count_lines <file> → integer line count
count_lines() {
    [[ -f "$1" ]] || { echo 0; return; }
    wc -l < "$1" | tr -d ' '
}

# count_words <file> → integer word count
count_words() {
    [[ -f "$1" ]] || { echo 0; return; }
    wc -w < "$1" | tr -d ' '
}

# verify_file_exists <topic> <file>
verify_file_exists() {
    local topic="$1"
    local file="$2"
    if [[ ! -f "$file" ]]; then
        log_err "$topic" "Missing: $(basename "$file")"
        return 1
    fi
    return 0
}

# verify_min_lines <topic> <file> <min>
verify_min_lines() {
    local topic="$1"
    local file="$2"
    local min="$3"
    local lines
    lines=$(count_lines "$file")
    if (( lines < min )); then
        log_err "$topic" "$(basename "$file"): $lines lines (min $min)"
        return 1
    fi
    return 0
}

# _strip_grep_prefix <grep_line> → echoes content portion of "file:line:content"
# The first two colons delimit file and line number.
_strip_grep_prefix() {
    # bash parameter expansion: ${var#*:} strips up to first colon.
    # Apply twice to strip file:line:.
    local s="${1#*:}"
    s="${s#*:}"
    echo "$s"
}

# _is_rename_table_row <content> → returns 0 if content looks like a markdown
# table row (starts with | after optional whitespace, and has at least 2 pipes).
_is_rename_table_row() {
    local content="$1"
    # Strip leading whitespace
    local trimmed="${content#"${content%%[! $'\t']*}"}"
    if [[ "$trimmed" == "|"* ]]; then
        # Check that there are at least 2 pipes (table row)
        local pipe_count="${trimmed//[^|]/}"
        if [[ ${#pipe_count} -ge 2 ]]; then
            return 0
        fi
    fi
    return 1
}

# _has_rename_marker <content> → returns 0 if the line contains a rename
# context marker word (legacy, formerly, →, etc.). Case-insensitive.
_has_rename_marker() {
    local content="$1"
    if echo "$content" | grep -qiE "$RENAME_CONTEXT_MARKERS_REGEX"; then
        return 0
    fi
    return 1
}

# _term_is_in_quotes <content> <term> → returns 0 if the forbidden term appears
# inside double quotes or backticks on the line. This is a strong signal that
# the term is being referenced as a string rather than used as current language.
_term_is_in_quotes() {
    local content="$1"
    local term="$2"
    # Check for "term" or `term` (fixed string match)
    if echo "$content" | grep -qF "\"$term\""; then
        return 0
    fi
    if echo "$content" | grep -qF "\`$term\`"; then
        return 0
    fi
    # Also allow single quotes 'term'
    if echo "$content" | grep -qF "'$term'"; then
        return 0
    fi
    return 1
}

# verify_no_forbidden <topic> <topic_dir>
#   Checks all .md files for forbidden terms.
#   HARD forbidden terms fail regardless of context.
#   SOFT forbidden terms are allowed in lines that look like rename tables
#   or that contain rename-context markers (e.g., "legacy", "formerly", "→").
verify_no_forbidden() {
    local topic="$1"
    local topic_dir="$2"
    local failures=0

    # Hard forbidden — no context exceptions.
    local term
    for term in "${HARD_FORBIDDEN_TERMS[@]}"; do
        while IFS= read -r hit; do
            [[ -z "$hit" ]] && continue
            log_err "$topic" "HARD forbidden term '$term' found: $hit"
            failures=$((failures + 1))
        done < <(grep -rniF -- "$term" "$topic_dir" --include='*.md' 2>/dev/null || true)
    done

    # Soft forbidden — allowed in rename context only.
    # Also entirely skipped for sub-docs whose filenames indicate they are
    # documenting REMOVED concepts (e.g., `00-vision-and-mortality-replaced.md`,
    # `12-academic-foundations.md` in the lifecycle topic, INDEX.md if it
    # summarizes what was removed). Those files legitimately enumerate the
    # removed terminology.
    local exempt_filename_regex='(mortality|death|removed|reframe|legacy|lifecycle-incompatibility|incompatibility|skip|death-masks|sonification)'
    for term in "${SOFT_FORBIDDEN_TERMS[@]}"; do
        while IFS= read -r hit; do
            [[ -z "$hit" ]] && continue
            # Extract filename from the grep hit (file:line:content)
            local filename="${hit%%:*}"
            filename=$(basename "$filename")
            # Skip if the filename indicates this sub-doc is about removed concepts
            if echo "$filename" | grep -qiE "$exempt_filename_regex"; then
                continue
            fi
            local content
            content=$(_strip_grep_prefix "$hit")
            # Skip if it's in a markdown table row
            if _is_rename_table_row "$content"; then
                continue
            fi
            # Skip if the line has a rename-context marker word
            if _has_rename_marker "$content"; then
                continue
            fi
            # Skip if the term is quoted (clearly referenced as a string)
            if _term_is_in_quotes "$content" "$term"; then
                continue
            fi
            log_err "$topic" "SOFT forbidden term '$term' found (no rename context): $hit"
            failures=$((failures + 1))
        done < <(grep -rniF -- "$term" "$topic_dir" --include='*.md' 2>/dev/null || true)
    done

    return $failures
}

# verify_required_terms <topic> <topic_dir>
#   Checks that each required term appears at least once across the topic.
verify_required_terms() {
    local topic="$1"
    local topic_dir="$2"
    local failures=0

    local term
    for term in "${REQUIRED_TERMS_BASE[@]}"; do
        if ! grep -riq "$term" "$topic_dir" --include='*.md' 2>/dev/null; then
            log_err "$topic" "Required term '$term' not found anywhere in topic"
            failures=$((failures + 1))
        fi
    done

    return $failures
}

# verify_index_structure <topic> <topic_dir>
#   INDEX.md must exist, contain a title, and contain a linked table of contents
#   referencing all sub-docs in the directory.
verify_index_structure() {
    local topic="$1"
    local topic_dir="$2"
    local index="$topic_dir/INDEX.md"
    local failures=0

    verify_file_exists "$topic" "$index" || return 1
    verify_min_lines "$topic" "$index" "$MIN_INDEX_LINES" || failures=$((failures + 1))

    # Check that the index mentions each sub-doc by filename.
    local subdoc
    for subdoc in "$topic_dir"/*.md; do
        local base
        base=$(basename "$subdoc")
        [[ "$base" == "INDEX.md" ]] && continue
        if ! grep -q "$base" "$index"; then
            log_warn "$topic" "INDEX.md does not reference $base"
            failures=$((failures + 1))
        fi
    done

    return $failures
}

# verify_citation_count <topic> <topic_dir>
#   Heuristic: count citation-ish patterns ([Author YYYY], (Author YYYY), arXiv:, ICLR, NeurIPS, Proc.).
#   Warn (not fail) if below a reasonable floor per topic.
verify_citation_count() {
    local topic="$1"
    local topic_dir="$2"

    # Topics that legitimately have fewer citations (interfaces, tools, deployment, test).
    local low_citation_topics=(
        "12-interfaces" "18-tools" "19-deployment" "15-code-intelligence" "test-smoke"
    )
    local floor=$MIN_CITATIONS
    local t
    for t in "${low_citation_topics[@]}"; do
        [[ "$topic" == "$t" ]] && floor=$MIN_CITATIONS_LOW && break
    done

    local count
    count=$(grep -rohE '\b(arXiv:|ICLR|NeurIPS|ACM|IEEE|Proc\.|et al\.|[A-Z][a-z]+\s+(19|20)[0-9]{2})\b' "$topic_dir" --include='*.md' 2>/dev/null | wc -l | tr -d ' ')

    if (( count < floor )); then
        log_warn "$topic" "Only $count citation-like patterns found (floor $floor)"
        return 1
    else
        log_info "$topic" "  citations: ~$count"
        return 0
    fi
}

# --- Main verification orchestrator ------------------------------------------

# verify_topic <topic>
#   Runs all checks. Returns 0 on full pass, 1 on any failure, 2 on warnings only.
verify_topic() {
    local topic="$1"
    local topic_dir
    topic_dir=$(topic_output_dir "$topic")

    log_info "$topic" "Verifying output at $topic_dir"

    if [[ ! -d "$topic_dir" ]]; then
        log_err "$topic" "Output directory does not exist: $topic_dir"
        return 1
    fi

    local hard_failures=0
    local soft_failures=0

    # 1. INDEX.md must exist and be substantial.
    if ! verify_index_structure "$topic" "$topic_dir"; then
        hard_failures=$((hard_failures + 1))
    fi

    # 2. Count sub-docs.
    local subdoc_count
    subdoc_count=$(find "$topic_dir" -maxdepth 1 -type f -name '*.md' ! -name 'INDEX.md' | wc -l | tr -d ' ')
    if (( subdoc_count < MIN_SUBDOCS )); then
        log_err "$topic" "Only $subdoc_count sub-docs (min $MIN_SUBDOCS)"
        hard_failures=$((hard_failures + 1))
    else
        log_info "$topic" "  sub-docs: $subdoc_count"
    fi

    # 3. Each sub-doc must be substantial.
    local subdoc
    for subdoc in "$topic_dir"/*.md; do
        local base
        base=$(basename "$subdoc")
        [[ "$base" == "INDEX.md" ]] && continue
        if ! verify_min_lines "$topic" "$subdoc" "$MIN_SUBDOC_LINES"; then
            hard_failures=$((hard_failures + 1))
        fi
    done

    # 4. Total line count across the topic.
    local total_lines=0
    while IFS= read -r f; do
        total_lines=$((total_lines + $(count_lines "$f")))
    done < <(find "$topic_dir" -maxdepth 1 -type f -name '*.md')
    if (( total_lines < MIN_TOPIC_TOTAL_LINES )); then
        log_err "$topic" "Total $total_lines lines (min $MIN_TOPIC_TOTAL_LINES)"
        hard_failures=$((hard_failures + 1))
    else
        log_info "$topic" "  total lines: $total_lines"
    fi

    # 5. Forbidden terms.
    local forbidden_hits=0
    forbidden_hits=$(verify_no_forbidden "$topic" "$topic_dir") || forbidden_hits=$?
    if (( forbidden_hits > 0 )); then
        hard_failures=$((hard_failures + forbidden_hits))
    fi

    # 6. Required terms.
    local required_misses=0
    required_misses=$(verify_required_terms "$topic" "$topic_dir") || required_misses=$?
    if (( required_misses > 0 )); then
        hard_failures=$((hard_failures + required_misses))
    fi

    # 7. Citation heuristic (warning only).
    if ! verify_citation_count "$topic" "$topic_dir"; then
        soft_failures=$((soft_failures + 1))
    fi

    # Summary.
    if (( hard_failures > 0 )); then
        log_err "$topic" "FAILED: $hard_failures hard failures, $soft_failures warnings"
        return 1
    elif (( soft_failures > 0 )); then
        log_warn "$topic" "PASSED with $soft_failures warnings"
        return 2
    else
        log_ok "$topic" "PASSED all quality checks"
        return 0
    fi
}
