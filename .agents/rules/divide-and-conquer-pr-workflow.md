# Rule: Multi-Agent Divide-and-Conquer PR Lifecycle & Merge Protocol

## Core Objective
Orchestrate concurrent, non-blocking agent execution using a hierarchical PR roll-up model that guarantees quality gate enforcement at every tier of the project tree and splits LLM quota bills evenly across the 3 configured accounts.

## Mandatory Invariants

### 1. Strict Chronological Bottom-Up Promotion & PR Creation Order
All agents and processes MUST adhere to a strict bottom-up promotion sequence. **Never open a parent PR before all of its child unit PRs are created and verified:**
1. **Level 3 (Atomic Micro-Unit Agent)**:
   - Operates on branch `feat/us-x.y.z/unit-w-<scope>` in dedicated worktree `.worktrees/feat-us-x.y.z-unit-w-<scope>`.
   - **Mandatory Branch Naming**: Must explicitly include `/unit-w` (e.g. `feat/us-4.1.0/unit-1-session-line-item-entities`).
   - Physically modifies files on disk in its isolated worktree.
   - Validates all local unit tests, clippy (0 warnings), and Gate G-Borrow zero-clone rules.
   - Commits with conventional commit format: `feat(scope): ...` or `test(scope): ...`.
   - Pushes branch to origin.
   - **Immediately creates the Level 3 Unit Pull Request on GitHub** (`gh pr create`).
   - **Mandatory Unit PR Title**: MUST explicitly start with `feat(us-x.y.z/unit-w): <summary>` (e.g. `feat(us-4.1.0/unit-1): checkout session and line item domain entities`). Never omit the `/unit-w` in PR titles!
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

**Mandatory Pre-Merge Gatekeeper Council Sign-Off (Gates G0–G4, G-Borrow, G2, G-Law, G-Docs, G-Review)**:
Before calling `gh pr merge` on ANY pull request (unit, story, or module), an explicit code review and security validation MUST be performed by independent specialized subagents from `.agents/agents/` armed with dedicated skills from `.agents/skills/`. **Self-approval is strictly rejected.**

1. **Tier 1 (Units to Story via `gh pr merge`)**:
   - Authoring implementer opens Unit PR: `feat(us-x.y.0/unit-N): <summary>`.
   - `Rust Refactoring Specialist` (Skill: `upgrade-stripe`) audits **Gate G0** (clippy/fmt) and **Gate G-Borrow** (zero-clone discipline).
   - `Application Security Engineer` (Skills: `security-audit`, `stripe-best-practices`) audits **Gate G2** (cryptography/redaction) and **Gate G-Law** (PCI-DSS 4.0, ISO 4217 minor currency, statutory EU compliance).
   - `API Tester` (Skills: `stripe-docs`, `stripe-best-practices`) audits **Gate G1** (acceptance test coverage).
   - Independent `Code Reviewer` (Skills: `security-audit`, `stripe-best-practices`) audits PR diff, synthesizes gate results, and posts formal Gate G-Review approval: `gh pr comment <PR_NUMBER> --body '### 🛡️ Gate G-Review Approval...'`.
   - Merge the unit PRs over GitHub into the story branch using `gh pr merge <PR_NUMBER> --merge`.
2. **Tier 2 (Stories to Module via `gh pr merge`)**:
   - Pull the merged story branch into its worktree (`git pull origin <story-branch>`).
   - Run story-level integration and wire mock test suites on the User Story branch.
   - `API Tester` (Skills: `stripe-docs`, `stripe-best-practices`) verifies **Gate G1** & **Gate G4** (wire mock contracts and E2E suites).
   - `microkernel-architect` (Skills: `stripe-docs`, `stripe-best-practices`) verifies **Gate G3** (microkernel architecture & progressive Cargo features).
   - `Technical Writer` (Skill: `stripe-docs`) verifies **Gate G-Docs** (`cargo test --doc` and crate README).
   - `Code Reviewer` (Skills: `security-audit`, `stripe-best-practices`) posts formal Gate G-Review approval comment.
   - Merge the User Story PR over GitHub into the module branch using `gh pr merge <PR_NUMBER> --merge`.
3. **Tier 3 (Module to Main via `gh pr merge`)**:
   - Pull the merged module branch into its worktree (`git pull origin <module-branch>`).
   - Run full workspace test suite (`cargo test --workspace`), clippy (`cargo clippy --workspace --all-targets -- -D warnings`), and fmt verification on the Module branch.
   - Full Federated Gatekeeper Council sign-off (`Rust Refactoring Specialist`, `Application Security Engineer`, `API Tester`, `Technical Writer`, `microkernel-architect`).
   - Independent `Code Reviewer` posts formal Module Release Gate G-Review approval comment.
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

### 7. Mandatory Shared Cargo Build Cache (`CARGO_TARGET_DIR`) in Git Worktrees
- All `cargo` invocations within any worktree MUST be preceded by:
  ```bash
  export CARGO_TARGET_DIR=/workspaces/some-stripe/target
  ```
- **Prohibition on Pre-Flight Compilation**: Agents must never run baseline or pre-flight `cargo test` runs before creating child worktrees. Worktrees must be provisioned immediately via `git worktree add`, and unit implementers dispatched without delay.

### 8. Active Supervisor Watchdog & Immediate Subagent Spawning (Fallbacks)
- **Zero Idle Talk Invariant**:
  - The Module Leader's **first action** MUST be spawning User Story Leaders (`invoke_subagent`).
  - The User Story Leader's **first action** MUST be provisioning worktrees and spawning all 5 Unit Implementers (`invoke_subagent`).
  - No prolonged planning or monologues before dispatching children.
- **3-Minute Watchdog Fallback**:
  - The Main Agent supervises the hierarchy, checking subagent health every 3 minutes.
  - If a leader runs for >3 minutes without creating its required child agents, or idles without active execution, the Main Agent directly intervenes and spawns the missing subagents.

### 9. Idempotent State Reconciliation & Anti-Duplication on Quota Failover
- **Pre-Spawn State Scan**:
  - Before spawning ANY story or unit agent (especially after quota exhaustion, restart, or account switch), agents MUST scan `git worktree list`, `git branch -a`, `gh pr list --state all`, and active subagents.
  - Zero duplicate agents or redundant user stories. Re-attach to existing branches and continue the cascading queue without burning extra quota.

### 10. Enforced Multi-Profile Round-Robin Allocation (`agent-a`, `agent-b`, `agent-c`)
- Subagents MUST be distributed evenly across `account1` (`agent-a`), `account2` (`agent-b`), `account3` (`agent-c`) to split API quota and prevent single-account exhaustion.
- Every subagent MUST specify its assigned profile in its role title and audit report.

### 11. Continuous Progress Checkpointing & Quota Recovery Protocol
- **Persistent Machine-Readable Checkpoint**:
  - Maintain `.agents/checkpoints/<module-id>/checkpoint.json` containing live status of all stories, units, PR numbers, merge commits, active profiles, and `next_action`.
  - Atomically update `checkpoint.json` on every worktree creation, PR creation, and PR merge.
- **Failover / Resume Guarantee**:
  - When quota is depleted or an account switches, execution state is already persisted on disk.
  - On restart, read `checkpoint.json` and resume execution directly from the pending micro-unit or promotion step with zero duplicate agent calls.

### 12. Mandatory Live Kanban Board Task Status Updating
- **Dedicated Module Kanban**: Every module has a live `docs/user-stories/<module-dir>/KANBAN.md`.
- **Mandatory Transitions**:
  - On task start: Update Kanban board task status to `🔄 In Progress`.
  - On PR creation: Update Kanban board task status to `🔍 In Review / PR Open (#XX)`.
  - On PR merge: Update Kanban board task status to `✅ Merged` with commit SHA.
- **Pre-Provisioning Invariant**: All worktrees for the module, stories, and units MUST be pre-provisioned via `git worktree add` before agent generation begins.
