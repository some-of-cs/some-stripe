# Rule: Multi-Agent Divide-and-Conquer PR Lifecycle & Merge Protocol

## Core Objective
Orchestrate concurrent, non-blocking agent execution using a hierarchical PR roll-up model that guarantees quality gate enforcement at every tier of the project tree.

## Mandatory Invariants
1. **Hierarchical Promotion Hierarchy**:
   - Level 3 (Atomic Micro-Unit Agent) -> Branch: `feat/us-x.y.z/unit-w`
   - Level 2 (Story Lead Agent) -> Branch: `feat/us-x.y.z`
   - Level 1 (Module Lead Agent) -> Branch: `feat/module-n`
   - Level 0 (Root Orchestrator) -> Branch: `main`
2. **Quality Gate Sign-Off Before Merge**:
   - An atomic child PR cannot be merged into a story branch without passing local tests and G-Borrow zero-clone checks.
   - A story PR cannot be merged into a module branch without passing G1 (coverage >=90%), G2 (security audit), and G-Review sign-off.
   - A module PR cannot be merged into `main` without passing G4 (`stripe-mock` integration) and workspace-wide `cargo test --all-features`.
3. **No Direct Pushes to Parent Branches**:
   - Every agent operates in its own non-blocking feature branch. Code is integrated strictly via reviewed and validated pull requests.
4. **Profile Load Balancing**:
   - Tasks are distributed across the 3 configured accounts (Profile 1: `agent-a`, Profile 2: `agent-b`, Profile 3: `agent-c`) to ensure parallel throughput and respect API quota boundaries.
5. **Dedicated Git Worktree Isolation**:
   - Every agent MUST operate in a dedicated, isolated `git worktree` under `.worktrees/<branch-sanitized-name>`.
   - Worktree isolation guarantees zero workspace cross-contamination, eliminates cargo/file locking contention, and ensures atomic commit/PR boundaries.
   - Once child PRs are reviewed and merged, worktrees are pruned cleanly using `git worktree remove .worktrees/<id>`.
