# Golden-path replay

What the scripted provider of `golden_path_fixture_plan_merges_green` writes:
each task's solution, as the files it changes, written after the plan in
`../plan/` was frozen (backlog 3116). A task's directory holds the whole of
each file it writes, on top of the work of the tasks it depends on.
`T3-wrong/` is the wrong solution T3 gets twice on its start rung, before it
passes one rung up.
