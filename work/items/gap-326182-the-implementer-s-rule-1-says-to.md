+++
id = "gap-326182"
kind = "gap"
title = "The implementer's rule 1 says to use read_file only for noted lines and undeclared task files, which taken literally forbids the tests and headers a try needs (phase 5.2's read six): say 'don't read again what the prompt shows; read the lines a note names, and anything else you need'"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-compose/templates"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round-v7 roko-ladder, gap-744bd0's measurement on phase 5.2 (run 13260e60), the lead's wording"
discovered_from = "agent:roko-ladder (round v7)"
anchors = ["crates/roko-compose/src/templates/implementer.rs::IMPLEMENTER_ROLE_IDENTITY"]
links = { depends_on = [], blocks = [], related = ["gap-744bd0", "gap-5db3d9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn implementer_may_read_what_the_prompt_lacks' crates/roko-compose/src && cargo test -p roko-compose implementer_may_read_what_the_prompt_lacks"
+++

## Problem

gap-744bd0 rewrote the implementer's rule 1 to: "If this prompt has a "Declared source context" section, work from
the files it holds: use `read_file` only for the lines a note there says are not shown, and for the files in the
task's `files` field that it does not hold."

Taken literally, "only" forbids reading the tests behind the task's checks and the headers of its sources. Phase
5.2's only try needed six such files before its first edit: three spec tests, presets.h, params.h and
params.cpp. It read them anyway; rule 2 says to check existing code. A model that obeys the letter could skip
them.

## Why it matters

The rule's point is to stop re-reading what the prompt shows, not to stop reading what it lacks. Missing context is
what costs turns now. That's the gap filed with this one: plans don't declare their checks' tests.

## Where

`crates/roko-compose/src/templates/implementer.rs::IMPLEMENTER_ROLE_IDENTITY`, rule 1 of "Before you write any code".

## Plan

1. Rule 1 becomes: "If this prompt has a "Declared source context" section, don't read again what it shows: read
   the lines a note there says are not shown, and anything else you need. Otherwise read the files listed in the
   task's `read_files` and `files` fields". That's the lead's wording (2026-10-26), with the conditional kept for
   prompts without the section.
2. Test `implementer_may_read_what_the_prompt_lacks`: the identity doesn't limit read_file to noted lines, tells the
   agent not to re-read what the prompt shows, and lets it read anything else it needs. gap-744bd0's test still
   passes, and the implementer snapshot follows.

## Done when

The `[[verify]]` passes.
