Status: draft · budget 200 words · owner spec-ce1484

# Abstract

Language-model agents do real software work, but they are variable workers: the same task can pass on one run and
fail on the next, errors build up over long tasks, and an agent can satisfy a visible check without doing the work.
Roko is a cybernetic harness for multi-agent orchestration. It runs agent loops and API models as workers against a
plan of small tasks, each carrying an executable check written before the work starts. Its design is five feedback
loops nested by time scale. The tool-call loop permits, bounds and screens every action. The attempt loop judges each
attempt by its checks and, on failure, retries it or escalates to a stronger model. The plan loop runs independent
tasks in parallel, each in its own worktree, and integrates them under a whole-plan check. The learning loops update
model routing, retry budgets, playbooks and knowledge from verified outcomes only. The audit level checks the checks
and the learning: it re-verifies a random sample of passes and tests whether each learning loop changes decisions for
the better. The executable verdict is the one fact every loop reads, and each loop leaves durable records for slower
loops. We describe each loop, the mechanisms behind it and the principles that join them.
