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

### 3. Multi-Tier Sequential Merge & Promotion Protocol (Strictly via `gh pr merge`)
All branch integrations and tier promotions MUST happen **over GitHub via GitHub CLI (`gh pr merge`)**. Local `git merge` commands across promotion tiers are strictly prohibited. The GitHub Pull Request audit trail is the single source of truth.

**Mandatory Pre-Merge Review Sign-Off (Gates G-Review, G2, G-Borrow)**:
Before calling `gh pr merge` on ANY pull request (unit, story, or module), an explicit code review, security check, and quality gate audit MUST be performed and approved via `gh pr review <PR> --approve -b '...'`.

1. **Tier 1 (Units to Story via `gh pr merge`)**:
   - Run tests across all 5 Level 3 micro-unit branches.
   - Execute code review and G-Borrow / G2 security checks on the unit diff.
   - Post approved review: `gh pr review <PR_NUMBER> --approve -b '...'`.
   - Merge the unit PRs over GitHub into the story branch using `gh pr merge <PR_NUMBER> --merge`.
2. **Tier 2 (Stories to Module via `gh pr merge`)**:
   - Pull the merged story branch into its worktree (`git pull origin <story-branch>`).
   - Run story-level integration and wire mock test suites on the User Story branch.
   - Execute story-level architectural and security review.
   - Post approved review: `gh pr review <PR_NUMBER> --approve -b '...'`.
   - Merge the User Story PR over GitHub into the module branch using `gh pr merge <PR_NUMBER> --merge`.
3. **Tier 3 (Module to Main via `gh pr merge`)**:
   - Pull the merged module branch into its worktree (`git pull origin <module-branch>`).
   - Run full workspace test suite (`cargo test --workspace`), clippy (`cargo clippy --workspace --all-targets -- -D warnings`), and fmt verification on the Module branch.
   - Execute module-wide Federated Gatekeeper Council sign-off.
   - Post approved review: `gh pr review <PR_NUMBER> --approve -b '...'`.
   - Merge the Level 1 Module PR over GitHub into `main` using `gh pr merge <PR_NUMBER> --merge`.

### 4. Quota Exhaustion Failover Across Profiles
- In case any profile encounters a quota depletion or rate limit error (429 / `RESOURCE_EXHAUSTED` / `exceeded your current quota`), the process/agent must automatically switch to the next account profile in round-robin sequence (`Account 1` -> `Account 2` -> `Account 3`).
- All 3 profiles are universal generators equipped to handle any task or role.

### 5. Mandatory `.worktrees/` Directory Isolation Invariant
- **Every agent MUST operate strictly inside the `.worktrees/` folder in this repository directory (`/workspaces/some-stripe/.worktrees/<branch-sanitized-name>`)**.
- Zero direct file editing or compilation at `/workspaces/some-stripe` directly on `main` is permitted.
- Worktree isolation guarantees zero workspace cross-contamination, eliminates cargo/file locking contention, and ensures atomic commit/PR boundaries.
- Once child PRs are reviewed and merged, worktrees are pruned cleanly using `git worktree remove .worktrees/<id>`.

### 6. Streaming Pipelined PR Creation & Ordered Cascading Merge Queue
To eliminate execution delays and avoid idle waiting:
1. **Immediate Pipelined PR Creation**:
   - Subagents MUST be granted full read, write, and command execution tools (`EnableWriteTools: true`).
   - As soon as any subagent finishes its code implementation and local unit testing in its worktree, it MUST immediately commit, push, and open its GitHub PR (`gh pr create`).
   - Never wait for subsequent or peer units to finish before opening an already completed unit's PR.
2. **Ordered Cascading Merge Queue ($1 \to 2 \to 3 \to 4 \to 5$)**:
   - **Unit 1**: When Unit 1 finishes and its PR is open, it has no preceding dependencies. It is immediately reviewed, approved, and merged into the story branch (`gh pr merge --merge`).
   - **Unit 2**: If Unit 2 finishes while Unit 1 is still in progress, Unit 2's PR remains open in queue. Once Unit 1 is merged into the story branch, Unit 2 pulls/merges the updated story branch, runs full validation (`cargo test`, `cargo clippy`, `cargo fmt`), passes Gatekeeper sign-off, and merges into the story branch (`gh pr merge --merge`).
   - **Unit 3**: Waits for Unit 2 to merge. Once Unit 2 merges, Unit 3 pulls the updated story branch (containing 1 and 2), runs validation, passes review sign-off, and merges into the story branch.
   - **Unit 4**: Waits for Unit 3 to merge, updates against story branch, validates, and merges.
   - **Unit 5**: Waits for Unit 4 to merge, updates against story branch, validates, and merges.
3. **Cascading Promotion for User Stories**:
   - The same streaming pipeline applies at Level 2:
   - When all units for User Story 1 (e.g., US-x.1.0) are merged, its Story PR is opened immediately and can be merged into the module branch upon review.
   - User Story 2 (US-x.2.0) opens its PR when ready, waits for Story 1 to merge into the module branch, pulls the updated module branch, runs integration validation, and merges into the module branch.
