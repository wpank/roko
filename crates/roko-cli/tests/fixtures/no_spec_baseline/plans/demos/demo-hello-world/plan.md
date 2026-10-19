---
plan: demo-hello-world
estimated_tasks: 1
estimated_parallel_width: 1
estimated_minutes: 1
parallel_safe: true
tags: [demo, hello-world, start-here]
---

# Start here: hello world demo

The demo to run first. One task writes `demo/hello-world/hello.sh`, a shell script that prints
`hello world`, and its check runs the script. It needs nothing installed beyond `sh`, takes about
a minute and costs well under a cent.

The script is demo output and says so in its first line. The run leaves it on its own branch
(`roko/batch/<run>`): look at the change, and leave it there; never merge it. If it lands on your
branch, a later run of this demo finds its check already passing and has nothing new to show.

`hermes roko init` added this plan. Delete its folder whenever you like.
