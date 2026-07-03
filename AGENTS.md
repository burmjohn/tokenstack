# Agent operating rules

This file defines repository-local instructions for autonomous coding agents
working in TokenStack.

## Delivery requirements

- After any major code or documentation change, document the change and the
  verification that was run.
- Do not leave major changes uncommitted. Stage only task-related files, create
  a Lore-format commit, push the branch, and merge it into the intended base
  branch when the repository has a clear branch or pull-request workflow and the
  merge can be completed safely.
- If commit, push, or merge is blocked by missing authentication, failing
  checks, conflicts, branch protection, or an unclear merge target, report the
  blocker with the exact command output and the next recovery step.
- Do not include unrelated dirty files in a task commit.
