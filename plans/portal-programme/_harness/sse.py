#!/usr/bin/env python3
"""Assertions over a capture of `GET /api/events` (lines of `data: {json}`).

usage: sse.py FILE COMMAND ARGS

  has      TYPE [k=v ...]                       some event matches
  count    TYPE [k=v ...] --eq N | --ge N        number of matching events
  before   A [k=v ...] -- B [k=v ...]           first A precedes first B; both exist
  between  X [k=v ...] -- A [k=v ...] -- B [k=v ...]
           some X lies between the first A and the first B after it
  positive TYPE FIELD [k=v ...]                 some match has numeric FIELD > 0

`k=v` compares event[k] rendered as JSON (strings unquoted), so booleans are
`true`/`false`. Exit status 0 means the assertion holds; a reason is printed
otherwise.
"""
import json
import sys


def load(path):
    events = []
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if not line.startswith("data:"):
                continue
            try:
                event = json.loads(line[5:].strip())
            except ValueError:
                continue
            if isinstance(event, dict) and "type" in event:
                events.append(event)
    return events


def render(value):
    return value if isinstance(value, str) else json.dumps(value)


def matcher(spec):
    kind, filters = spec[0], [item.split("=", 1) for item in spec[1:]]

    def matches(event):
        return event.get("type") == kind and all(
            key in event and render(event[key]) == value for key, value in filters)

    return matches


def split(args):
    groups, current = [], []
    for arg in args:
        if arg == "--":
            groups.append(current)
            current = []
        else:
            current.append(arg)
    groups.append(current)
    return groups


def first(events, predicate, start=0):
    for index in range(start, len(events)):
        if predicate(events[index]):
            return index
    return None


def main(argv):
    path, command, args = argv[1], argv[2], argv[3:]
    events = load(path)
    if command == "has":
        ok = first(events, matcher(args)) is not None
        reason = f"no {' '.join(args)} among {len(events)} events"
    elif command == "count":
        op, expected = args[-2], int(args[-1])
        actual = sum(1 for event in events if matcher(args[:-2])(event))
        ok = actual == expected if op == "--eq" else actual >= expected
        reason = f"{' '.join(args[:-2])}: {actual} matches, wanted {op} {expected}"
    elif command == "before":
        a_spec, b_spec = split(args)
        a_index, b_index = first(events, matcher(a_spec)), first(events, matcher(b_spec))
        ok = a_index is not None and b_index is not None and a_index < b_index
        reason = f"{' '.join(a_spec)} at {a_index}, {' '.join(b_spec)} at {b_index}"
    elif command == "between":
        x_spec, a_spec, b_spec = split(args)
        a_index = first(events, matcher(a_spec))
        b_index = None if a_index is None else first(events, matcher(b_spec), a_index + 1)
        x_index = None if a_index is None else first(events, matcher(x_spec), a_index + 1)
        ok = b_index is not None and x_index is not None and x_index < b_index
        reason = f"{' '.join(x_spec)} at {x_index} between {a_index} and {b_index}"
    elif command == "positive":
        kind, field, filters = args[0], args[1], args[2:]
        predicate = matcher([kind, *filters])
        values = [event.get(field) for event in events if predicate(event)]
        ok = any(isinstance(value, (int, float)) and value > 0 for value in values)
        reason = f"{kind}.{field} values {values[:8]}"
    else:
        print(f"unknown command {command}", file=sys.stderr)
        return 2
    if not ok:
        print(f"    {command}: {reason}", file=sys.stderr)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
