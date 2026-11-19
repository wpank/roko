+++
id = "bug-7dc3ef"
kind = "bug"
title = "A read-only view writes on a fresh day: roko's daily file log creates .roko/roko.log.<date> at startup for every command, roko plan status included"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
source = "round v8 guide-v8's read-only check of roko-item10a on a clone"
discovered_from = "agent:guide-v8 (round v8)"
anchors = ["crates/roko-cli/src/main.rs"]
links = { depends_on = [], blocks = [], related = ["bug-399240", "bug-62760a"], supersedes = [], duplicate_of = "" }
+++

## Problem

On a fresh day, a read-only view writes: guide-v8 saw `roko plan status` create an empty `.roko/roko.log.2026-11-13`
in its clone of Undertow's workspace, on the day's first run. roko's file logging opens the day's log file at
startup, for every command, the views included. A view must never write (bug-399240's rule). Here it's one empty
file, and on a live workspace serve has usually made the day's file already.

## Why it matters

Goal tooling, small. A view that creates files in the workspace breaks the read-only rule a person relies on, for
example when they inspect a copy, or a checkout they don't own.

## Where

roko-cli's tracing setup (the daily file appender), which every command runs.

## Plan

Let views (`plan status`, `status`, `diagnose`, `learn` views, `show`) log to the file only when it already exists,
or not at all. Check with a snapshot of a fresh workspace copy.

## Done when

A view run on a workspace without the day's log file leaves it without one.
