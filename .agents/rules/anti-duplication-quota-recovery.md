# Rule: Idempotent State Reconciliation & Anti-Duplication on Quota Failover

## Core Objective
Prevent duplicate agent generation, duplicate story branches, and wasted quota/tokens when recovering from quota exhaustion (`429 / RESOURCE_EXHAUSTED`), account switches, or agent restarts.

---

## Mandatory Invariants

### 1. Mandatory Pre-Spawn State Reconciliation Scan
Before spawning ANY story leader, unit implementer, or validator, every agent (Root, Module Lead, Story Lead) MUST perform an idempotent repository state scan:
1. **Worktrees Scan**: `git worktree list` — identify all existing worktrees for stories and units.
2. **Branch Scan**: `git branch -a` — identify existing local and remote branches (`feat/us-x.y.z/unit-w`, `feat/us-x.y.z`).
3. **Pull Request Scan**: `gh pr list --state all` — verify whether pull requests are already open, approved, or merged for the units and stories.
4. **Active Subagent Scan**: `manage_subagents` list — inspect active child conversations to identify running agents.

### 2. Strict Prohibition on Duplicate Agent Spawning
- If a worktree, branch, or PR for a User Story or Micro-Unit already exists:
  - **DO NOT spawn a duplicate agent with the same or similar prompt.**
  - **DO NOT create duplicate story branches (e.g. creating two stories for the same scope).**
- Instead, the agent MUST re-attach to the existing state:
  - If a unit PR is already open or merged, advance to the next step in the cascading queue ($1 \to 2 \to 3 \to 4 \to 5$).
  - If a story PR is already merged into the module branch, proceed immediately to the next story or module promotion.

### 3. Quota Failover Recovery Protocol
- When an account switches due to quota exhaustion:
  1. Inspect the last committed SHA and open PRs.
  2. Determine the exact pending micro-unit or user story in the cascading pipeline.
  3. Resume execution strictly from that pending tier without repeating prior work or re-spawning finished roles.
