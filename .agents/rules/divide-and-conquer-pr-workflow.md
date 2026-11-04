# Rule: Multi-Agent Divide-and-Conquer PR Lifecycle & Merge Protocol

## Core Objective
Orchestrate concurrent, non-blocking agent execution using a hierarchical PR roll-up model that guarantees quality gate enforcement at every tier of the project tree and splits LLM quota bills evenly across the 3 configured accounts.

## Mandatory Invariants

### 1. Strict Chronological Bottom-Up Promotion & PR Creation Order
All agents and processes MUST adhere to a strict bottom-up promotion sequence. **Never open a parent PR before all of its child unit PRs are created and verified:**
1. **Level 3 (Atomic Micro-Unit Agent)**:
   - Operates on branch `feat/us-x.y.z/unit-w` in dedicated worktree `.worktrees/feat-us-x.y.z-unit-w`.
   - Physically modifies files on disk in its isolated worktree.
   - Validates all local unit tests, clippy (0 warnings), and Gate G-Borrow zero-clone rules.
   - Commits with conventional commit format: `feat(scope): ...` or `test(scope): ...`.
   - Pushes branch to origin.
   - **Immediately creates the Level 3 Unit Pull Request on GitHub** (`gh pr create`) with detailed architectural and quality gate description.
   - Micro-units must be created in ascending order: Unit 1 -> Unit 2 -> Unit 3 -> Unit 4 -> Unit 5.
2. **Level 2 (Story Lead Agent)**:
   - Operates on branch `feat/us-x.y.z` in dedicated worktree `.worktrees/feat-us-x.y.z`.
   - **PRECONDITION**: All 5 child Level 3 unit PRs for this user story MUST already be created on GitHub and pass verification.
   - Validates integration across all 5 units, runs story-level test suites and contract wire tests.
   - Commits and pushes branch `feat/us-x.y.z` to origin.
   - **Creates the Level 2 User Story Pull Request on GitHub** (`gh pr create`) detailing story deliverables and gate compliance.
3. **Level 1 (Module Lead Agent)**:
   - Operates on branch `feat/module-n` in dedicated worktree `.worktrees/feat-module-n`.
   - **PRECONDITION**: All constituent Level 2 User Story PRs (e.g., US-x.1.0, US-x.2.0, US-x.3.0) MUST already be created on GitHub.
   - Integrates user stories into the module branch, executes workspace-wide compilation and test suites (`cargo test --workspace`, `cargo clippy --workspace -- -D warnings`).
   - Commits and pushes `feat/module-n` to origin.
   - **Creates the Level 1 Module Pull Request on GitHub** (`gh pr create`).
4. **Level 0 (Root Orchestrator)**:
   - Operates on `main`.
   - Validates against Gate G4 (`stripe-mock` integration) and reviews full module PR before merging into `main`.

### 2. Multi-Profile Quota Balancing & Universal Agent Generation
Profiles are **universal agent generators** tied to separate Google AI Pro billing accounts (Account 1: `agent-a`, Account 2: `agent-b`, Account 3: `agent-c`).
- Profiles are **NOT** bound to fixed tasks or domains.
- Any profile can generate and execute **ANY** specialized agent for any role or task (e.g. `microkernel-architect`, `security-webhook-engineer`, `domain-codegen-specialist`, `rust-refactoring-specialist`, `code-reviewer`, `api-tester`, or micro-unit worker agents).
- Workloads and agent generation MUST rotate across the 3 profile generators in round-robin fashion to distribute token consumption and split the quota bill evenly.

### 3. Quality Gate Sign-Off Before Merge
- An atomic child PR cannot be merged into a story branch without passing local tests and G-Borrow zero-clone checks.
- A story PR cannot be merged into a module branch without passing G1 (coverage >=90%), G2 (security audit), and G-Review sign-off.
- A module PR cannot be merged into `main` without passing G4 (`stripe-mock` integration) and workspace-wide `cargo test --all-features`.

### 4. No Direct Pushes to Parent Branches
- Every agent operates in its own non-blocking feature branch. Code is integrated strictly via reviewed and validated pull requests and branch merges.

### 5. Dedicated Git Worktree Isolation
- Every agent MUST operate in a dedicated, isolated `git worktree` under `.worktrees/<branch-sanitized-name>`.
- Worktree isolation guarantees zero workspace cross-contamination, eliminates cargo/file locking contention, and ensures atomic commit/PR boundaries.
- Once child PRs are reviewed and merged, worktrees are pruned cleanly using `git worktree remove .worktrees/<id>`.
