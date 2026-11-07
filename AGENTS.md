# some-stripe — Workspace Agent Rules & Guidelines

Welcome to the **`some-stripe`** workspace. All AI agents, contributors, and automated processes operating within this repository MUST strictly follow these architectural, cryptographic, and quality rules:

---

## 1. Active Workspace Rules

1. **[Shared Patterns, Zero Redundancy & Reusability](.agents/rules/shared-patterns-and-reusability.md)**:
   - Eliminate code duplication (DRY). Co-locate all shared architectural patterns, generic CRUD builders, and serializers in a dedicated `patterns/` module.
   - Mandatory rustdoc documentation with working doc-tests (`cargo test --doc`) and comprehensive unit tests on every public component.

2. **[Rust Zero-Clone Borrowing Discipline (Gate G-Borrow)](.agents/rules/rust-zero-clone.md)**:
   - Enforce strict Rust 2024 edition borrowing (`&T`, `&mut T`), explicit lifetimes (`'a`), and zero-copy byte slices (`bytes::Bytes`).
   - Zero deep `.clone()` calls on domain entities and models; `Arc::clone` and `Rc::clone` pointer copies are permitted only.

3. **[Security Gateway & Cryptographic Invariants (Gate G2)](.agents/rules/security-gateway.md)**:
   - Constant-time HMAC-SHA256 signature verification (`safe_constant_time_eq` / `subtle::ConstantTimeEq`).
   - Enforce 300-second timestamp drift tolerance (`DEFAULT_TOLERANCE_SECONDS = 300`).
   - Automatic secret zeroization on drop and `Debug` redaction (`[REDACTED]`).
   - Strict 1 MB maximum payload limits on incoming webhook requests.

4. **[Dual-Protocol Serialization & Nullability Mechanics](.agents/rules/dual-protocol-serialization.md)**:
   - API v1: Strict zero-indexed bracket notation for arrays of objects (`items[0][id]=...`).
   - 3-state `Nullable<T>` (`Value`, `Null` -> `""`, `Omit`).
   - API v2: Strict `application/json` payloads with operation-scoped request DTOs and URL-driven pagination.

5. **[Quality Gates & Code Review Standard (Federated Gatekeeper Council)](.agents/rules/quality-gates-and-code-review.md)**:
   - Zero-warning Rust 2024 compilation (`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`).
   - Mandatory test coverage: >=90% line and >=85% branch coverage in affected modules.
   - Microkernel architecture: keep the core runtime lean with zero monolithic codegen.
   - **Federated Gatekeeper Subagents & Skills (Zero Self-Approval)**: All quality gates MUST be evaluated by independent specialized subagents from `.agents/agents/` armed with dedicated skills from `.agents/skills/`:
     - **Gate G0, Gate G-Borrow & Gate G-Complexity**: `Rust Refactoring Specialist` (Skills: `upgrade-stripe`, `cyclomatic-complexity`)
     - **Gate G2 & Gate G-Law**: `Application Security Engineer` (Skills: `security-audit`, `stripe-best-practices`)
     - **Gate G1 & Gate G4**: `API Tester` (Skills: `stripe-docs`, `stripe-best-practices`)
     - **Gate G3**: `microkernel-architect` (Skills: `stripe-docs`, `stripe-best-practices`)
     - **Gate G-Docs**: `Technical Writer` (Skill: `stripe-docs`)
     - **Gate G-Review & Gate G-Complexity**: `Code Reviewer` (Skills: `security-audit`, `cyclomatic-complexity`, `stripe-best-practices`) — Council President, formal GitHub sign-off.
   - Merging via `gh pr merge` is strictly blocked until the independent `Code Reviewer` posts the structured sign-off comment. Self-approval is rejected.
   - **Mandatory 3-Tier Code Review & Self-Correction Loop (Never Terminate Until Clean)**:
     - Every Unit Implementer upon finishing implementation MUST invoke a distinct `Code Reviewer` subagent armed with `security-audit` and `cyclomatic-complexity`. If any issues or high complexity are flagged, the implementer must fix/decompose the issues and re-invoke review until 0 issues remain. **The Unit Implementer is NEVER terminated until the subagent Code Reviewer tells them everything is OK.**
     - At the User Story level, after all 5 units are merged into the story branch, the Story Leader MUST enter an iterative review and implementation loop by invoking a distinct `Code Reviewer` subagent with `security-audit` and `cyclomatic-complexity`. **The Story Leader is NEVER terminated until the Code Reviewer confirms everything is OK.**
     - **Story-Level Patch Implementer Invariant**: If ANY code review, security, or complexity issue is flagged at the story level, the Story Leader MUST NOT fix it directly in the story leader context. Instead, the Story Leader MUST invoke a dedicated `patch-implementer` subagent to implement the fixes in a dedicated patch branch and PR (`patch/us-x.y.0-<scope>`). The Story Leader reviews and merges the patch PR into the story branch via `gh pr merge`, and re-invokes the `Code Reviewer` subagent again, repeating this review-and-patch loop continuously until 0 problems remain.
     - **Hierarchical Supervision Schedules**: Active supervision schedules are strictly enforced across leadership tiers:
       - **Module Leader**: Maintains a supervision schedule every 3 minutes (`schedule` cron `*/3 * * * *`) to monitor Story Leaders, reconcile PR queues, and manage cascading promotions.
       - **Story Leaders**: Maintain a supervision schedule every 1 minute (`schedule` cron `* * * * *`) to monitor active Unit and Patch Implementers, track progress, and eliminate idling.
     - At the Module level, after all User Stories are merged into the module branch, the Module Leader MUST enter a full module review and implementation loop with an invoked distinct `Code Reviewer` subagent with `security-audit` and `cyclomatic-complexity`. **The Module Leader is NEVER terminated until the Code Reviewer gives a 100% clean sign-off.**
     - **Module-Level Patch Implementer Invariant**: If ANY code review, security, or complexity issue is flagged at the module level, the Module Leader MUST NOT fix it directly in the module leader context. Instead, the Module Leader MUST invoke a dedicated `patch-implementer` subagent to implement the fixes in a dedicated patch branch and PR (`patch/module-N-<scope>`). The Module Leader reviews and merges the patch PR into the module branch via `gh pr merge`, and re-invokes the `Code Reviewer` subagent again, repeating this review-and-patch loop continuously until 0 problems remain and 100% clean sign-off is achieved.
     - **Mandatory GitHub PR In-Conversation Audit Trail**: All implementation reports, security audit reports (`.agents/skills/security-audit`), and code review reports (including every iteration of review findings, cyclomatic complexity evaluations, and corresponding remediation summaries) MUST be documented directly as comments on the GitHub PR conversation (`gh pr comment <PR> --body "..."`). Merging without documented in-conversation evidence of clean sign-off is strictly prohibited.

6. **[Global Financial & Statutory Legal Compliance (Gate G-Law)](.agents/rules/legal-compliance.md)**:
   - PCI-DSS 4.0 data security and zero raw PAN handling in application memory.
   - PSD2 Strong Customer Authentication (SCA) 3D Secure 2.0 redirect support.
   - GDPR Article 15/17 consumer data rights and ISO 4217 minor currency unit arithmetic.

7. **[Multi-Agent Divide-and-Conquer PR Lifecycle & Merge Protocol](.agents/rules/divide-and-conquer-pr-workflow.md)**:
   - **Mandatory Git, Worktree & GitHub Lifecycle Subagent (`Git Workflow Master`)**:
     - All operations involving Git repositories, worktrees, branches, and GitHub CLI across all tiers MUST be managed and executed by the specialized subagent **`Git Workflow Master`** ([`.agents/agents/engineering-git-workflow-master.md`](.agents/agents/engineering-git-workflow-master.md)) paired with mandatory skills `antigravity-guide` and `stripe-best-practices`.
     - Implementers write and verify code locally, then hand off to `Git Workflow Master` (Skills: `antigravity-guide`, `stripe-best-practices`) to stage atomic conventional commits, set up upstream tracking (`git push -u origin <branch>`), author GitHub Pull Requests (`gh pr create`), and execute cascading promotion merges (`gh pr merge`).
   - **Mandatory `.worktrees/` Directory Invariant**: All agents MUST work strictly inside the `.worktrees/` folder in this directory (`/workspaces/some-stripe/.worktrees/<branch-sanitized-name>`). Direct code modification on `main` or at the repository root is strictly prohibited. Worktrees are provisioned and pruned exclusively by `Git Workflow Master` (Skills: `antigravity-guide`, `stripe-best-practices`).
   - **Strict Bottom-Up PR Creation Order**: Level 3 micro-units (Unit 1 -> 5) MUST be implemented, verified, committed, pushed, and have their GitHub PR opened (`gh pr create`) by `Git Workflow Master` (Skills: `antigravity-guide`, `stripe-best-practices`) BEFORE the Level 2 User Story PR is opened. Never open a parent story/module PR before child unit PRs exist.
   - **Mandatory Branch & PR Naming with Explicit Unit Number**:
     - **Micro-Unit Branches**: MUST strictly match `feat/us-x.y.0/unit-N-<scope>` (e.g. `feat/us-4.1.0/unit-1-session-line-item-entities`).
     - **Micro-Unit PR Titles**: MUST strictly match `feat(us-x.y.0/unit-N): <summary>` (e.g. `feat(us-4.1.0/unit-1): checkout session and line item domain entities`). Never omit the `/unit-N` from the PR title.
     - **User Story PR Titles**: MUST match `feat(us-x.y.0): <story-title> (Units 1..5 Promotion)`.
     - **Module PR Titles**: MUST match `feat(module-N): <module-title>`.
   - **Multi-Tier Merge & Validation Workflow (Strictly via `gh pr merge`)**:
     - **All tier merges MUST happen over GitHub using GitHub CLI (`gh pr merge`)**. Local `git merge` commands across promotion tiers are strictly prohibited.
     - **Mandatory Pre-Merge Review Sign-Off (Gates G-Review, G2, G-Borrow)**: Before calling `gh pr merge` on ANY pull request (unit, story, or module), an explicit code review and security validation must be executed and approved by an independent `Code Reviewer` subagent via `gh pr comment <PR> --body '### Gate G-Review Approval...'`. Merging without documented independent quality gate review is strictly prohibited. Self-approval by the authoring implementer is rejected.
     1. Run tests over all 5 Level 3 units in a user story -> review & approve -> `Git Workflow Master` (Skills: `antigravity-guide`, `stripe-best-practices`) merges units into the story branch via `gh pr merge <PR> --merge`.
     2. Pull updated story branch, run story integration and contract wire tests -> review & approve -> `Git Workflow Master` (Skills: `antigravity-guide`, `stripe-best-practices`) merges user story PRs into the module branch via `gh pr merge <PR> --merge`.
     3. Pull updated module branch, run workspace-wide test suite (`cargo test --workspace`) and quality gates -> review & approve -> `Git Workflow Master` (Skills: `antigravity-guide`, `stripe-best-practices`) merges the module PR into `main` via `gh pr merge <PR> --merge`.
   - **Streaming Pipelined PR Creation & Ordered Cascading Merge Queue**:
     - Subagents MUST be granted full read, write, and command execution tools (`EnableWriteTools: true`).
     - As soon as each micro-unit finishes implementation and local tests, `Git Workflow Master` (Skills: `antigravity-guide`, `stripe-best-practices`) MUST immediately commit, push, and open its GitHub PR (`gh pr create`) without waiting for other units.
     - Merges MUST follow the strict sequential dependency order ($1 \to 2 \to 3 \to 4 \to 5$):
       - Unit 1 merges into the story branch immediately upon completion and gate review via `Git Workflow Master`.
       - Unit 2 waits for Unit 1 to merge, then pulls/merges the updated story branch, runs full validation (`cargo test`, `cargo clippy`, `cargo fmt`), passes gate review, and `Git Workflow Master` merges it into the story branch.
       - Unit 3 waits for Unit 2 to merge, updates against the story branch, validates, and `Git Workflow Master` merges it.
       - Units 4 and 5 follow the exact same cascading merge sequence.
       - The same streaming pipeline and cascading merge applies for User Story PRs into the Module branch.
   - **Multi-Profile Round-Robin**: Tasks MUST be actively distributed across all 3 profiles (`agent-a`, `agent-b`, `agent-c`) to split the API quota and prevent single-account billing concentration.
   - **Automatic Quota Failover**: In case any profile exhausts quota (429 / `RESOURCE_EXHAUSTED`), agents and launchers must automatically switch and retry using the next available account profile.

8. **[Mandatory Agent Audit Logging & Full Execution Reporting](.agents/rules/agent-execution-reporting.md)**:
   - Every agent (Unit implementer, Story lead, Module lead, Gatekeeper) MUST record a comprehensive report detailing:
     - Exact scope, role, and profile used.
     - Complete inventory of created and modified files.
     - Verification logs for all quality gates (G0, G-Borrow, G2, G-Law, G1, G4).
     - Test commands and verbatim outputs.
     - GitHub PR link, commit SHA, and pre-merge review approval.
   - **Mandatory PR Attachment**: A full execution report including validation and gate results MUST be attached directly to every GitHub PR (in the PR body or primary audit comment) before review.
   - All reports MUST be committed under `.agents/reports/<module-id>/` to preserve a permanent audit trail.

9. **[Mandatory Microkernel Documentation, Doctests & README Standard (Gate G-Docs)](.agents/rules/microkernel-documentation-standard.md)**:
   - Every microkernel crate (under `src/crates/<crate-name>/`) MUST maintain a comprehensive `README.md` covering architectural invariants, security considerations, quickstarts, and quality gate compliance.
   - Every public module, struct, enum, and method in a microkernel MUST have comprehensive rustdoc documentation with working doc-tests (`cargo test --doc`) verified against the test harness.
   - Serde field attributes (such as `#[serde(default, skip_serializing_if = "Nullable::is_omit")]`) must be documented with runnable examples verifying payload expectations.

10. **[Mandatory Shared Cargo Build Cache (`CARGO_TARGET_DIR`) in Git Worktrees](.agents/rules/cargo-target-dir-worktree-caching.md)**:
   - Every agent running `cargo` commands in `.worktrees/` MUST export `export CARGO_TARGET_DIR=/workspaces/some-stripe/target`.
   - Never run un-cached cargo builds or baseline tests before creating worktrees; provision worktrees instantly via `git worktree add` and reuse precompiled artifacts to keep build times under 2 seconds.

11. **[Active Supervisor Watchdog & Immediate Subagent Spawning](.agents/rules/active-supervisor-watchdog.md)**:
   - **Zero Idle Talk Before Spawning**: Module Leaders must immediately spawn User Story Leaders in their first step; Story Leaders must immediately spawn their 5 Unit Implementers in their first step. Protracted analysis prior to spawning child subagents is strictly prohibited.
   - **3-Minute Watchdog Fallback**: The Main Agent actively monitors subagent progress every 3 minutes. If any leader runs for >3 minutes without spawning its required child agents, or idles in verbose monologues, the Main Agent MUST intervene immediately and directly spawn the missing subagents.

12. **[Idempotent State Reconciliation & Anti-Duplication on Quota Failover](.agents/rules/anti-duplication-quota-recovery.md)**:
   - **Mandatory Pre-Spawn Scan**: Before creating any story leader or unit implementer (especially after quota exhaustion, restart, or account switch), agents MUST scan `git worktree list`, `git branch -a`, `gh pr list --state all`, and active subagents.
   - **Zero Duplicate Agents**: Never spawn duplicate agents or stories for tasks already in progress or merged. Re-attach to existing branches and continue the cascading queue without burning extra quota.

13. **[Enforced Multi-Profile Round-Robin Allocation (`agent-a`, `agent-b`, `agent-c`)](.agents/rules/multi-profile-allocation.md)**:
   - **Strict Profile Rotation**: Agents MUST actively rotate across all 3 billing accounts (`account1`/`agent-a`, `account2`/`agent-b`, `account3`/`agent-c`) to split the API quota and avoid single-account concentration.
   - Every subagent MUST specify its assigned profile in its role title, system prompt, and audit report.

14. **[Continuous Progress Checkpointing & Quota Failover Recovery](.agents/rules/quota-exhaustion-checkpointing.md)**:
   - **Persistent Checkpoint Store**: Maintain `.agents/checkpoints/<module-id>/checkpoint.json` continuously updated on every worktree creation, PR creation, and PR merge.
   - **Zero-Duplication Resume**: In the event of quota exhaustion (429) or restart, agents must load `checkpoint.json`, reconcile with GitHub/Git, and resume strictly from the exact pending micro-unit or story without re-executing finished tasks or burning extra quota.

15. **[Mandatory Live Kanban Board Task Status Updating](.agents/rules/mandatory-kanban-status-tracking.md)**:
   - **Active Task State Transition**: Every subagent assigned to a task MUST update the module's `docs/user-stories/<module-dir>/KANBAN.md` file:
     - On task start: Move task to `🔄 In Progress`.
     - On PR creation: Move task to `🔍 In Review / PR Open` with PR link.
     - On PR merge: Move task to `✅ Merged` with commit SHA.

16. **[Strict Hierarchical Delegation Invariant (Zero Cross-Tier Spawning)](.agents/rules/strict-hierarchical-spawning.md)**:
   - **Module Leader Role Boundary**: The Module Leader is strictly responsible for spawning and supervising User Story Leaders (`US-X.1.0 Story Leader`, `US-X.2.0 Story Leader`). The Module Leader MUST NEVER spawn Unit Implementers directly.
   - **Story Leader Role Boundary**: Each User Story Leader is strictly responsible for spawning, supervising, and coordinating its own 5 Unit Implementers (`Unit 1`, `Unit 2`, `Unit 3`, `Unit 4`, `Unit 5`).
   - Any agent attempting cross-tier spawning will be rejected by the supervisor watchdog.

17. **[Dedicated Quota Sentinel Agent (`quota-sentinel`)](.agents/rules/quota-sentinel.md)**:
   - A dedicated, autonomous subagent **`quota-sentinel`** is maintained throughout workspace execution to monitor API quota consumption, rate limits (HTTP 429 / `RESOURCE_EXHAUSTED`), and token velocity.
   - **1% Quota Threshold Trigger**: As soon as quota drops to 1% (or upon 429 rate limit detection), the `quota-sentinel` immediately broadcasts a high-priority alert to all active agents and the main supervisor conversation, enforcing an immediate execution pause and scheduling a timer until the quota resets.

18. **[Main Conversation End-of-Module Whole-Codebase Validation & Review](.agents/rules/main-conversation-end-of-module-review.md)**:
   - Always maintain an active Module Leader during module execution.
   - At the main conversation level, upon completion and merge of every module, the Main Supervisor MUST execute whole-codebase verification (`cargo test --workspace`, `cargo test --doc`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`) and invoke an independent Gatekeeper Council review across the current module AND all preceding modules (Modules 1..N) to ensure zero regressions, zero debt, and 100% defense-in-depth across the entire repository.

19. **[Cyclomatic Complexity Minimization & Decomposition Standard (Gate G-Complexity)](.agents/rules/cyclomatic-complexity-minimization.md)**:
   - Drive branching complexity **as low as possible** ($CC = 1 + \sum \text{decision points}$) using [`.agents/skills/cyclomatic-complexity/SKILL.md`](.agents/skills/cyclomatic-complexity/SKILL.md).
   - **Ideal Target**: $CC \le 6$ for coordinator functions, endpoints, and primary execution paths.
   - **Acceptable Upper Ceiling**: $CC \le 10$ across all logic paths. Any function with $CC > 10$ requires mandatory decomposition before PR submission.
   - **Hard Merge Blocker**: Any function with $CC > 15$ strictly blocks PR merge (Gate G-Complexity failure).
   - Enforce maximum nesting depth $\le 2$ levels, early guard returns, table dispatch/pattern serializers, and named boolean predicate helpers.

---

## 2. Multi-Profile Quota Engine & Dynamic Agent Generation

Profiles are **universal agent generators** tied to independent billing accounts (Google AI Pro Accounts 1, 2, 3), NOT tied to specific domains or tasks. Any profile can generate and host ANY specialized agent role for any task.

- **Profile Capacity & Bill Splitting**:
  - `account1` (`agent-a`): Account slot 1 — generates agents on demand for any module or unit.
  - `account2` (`agent-b`): Account slot 2 — generates agents on demand for any module or unit.
  - `account3` (`agent-c`): Account slot 3 — generates agents on demand for any module or unit.
- **Dedicated Worktree Location Invariant**:
  - All agent execution, code changes, and test executions MUST take place inside `.worktrees/` in this repository directory (`/workspaces/some-stripe/.worktrees/`).
  - No subagent or process may write outside its designated `.worktrees/<worktree-name>` path.
- **Specialized Agents & Mandatory Skills Matrix**:
  - Agents are instantiated dynamically with specific tasks, roles, and mandatory skills:
    - **`Rust Refactoring Specialist`** (Skills: `upgrade-stripe`, `cyclomatic-complexity`): Audits Gate G0 (0-warning clippy/fmt), Gate G-Borrow (zero-clone borrowing discipline), and Gate G-Complexity ($CC \le 6$ target, hard cap $\le 10$, no functions $> 15$).
    - **`Application Security Engineer`** (Skills: `security-audit`, `stripe-best-practices`): Audits Gate G2 (constant-time crypto, zeroization) and Gate G-Law (PCI-DSS 4.0 zero PAN, ISO 4217, EU statutory consumer rights).
    - **`API Tester`** (Skills: `stripe-docs`, `stripe-best-practices`): Audits Gate G1 (acceptance tests & coverage) and Gate G4 (wire contract validation against `stripe-mock`).
    - **`microkernel-architect`** (Skills: `stripe-docs`, `stripe-best-practices`): Audits Gate G3 (lean microkernel architecture, progressive Cargo features, dual-protocol serializers).
    - **`Technical Writer`** (Skill: `stripe-docs`): Audits Gate G-Docs (`cargo test --doc`, crate README.md).
    - **`Code Reviewer`** (Skills: `security-audit`, `cyclomatic-complexity`, `stripe-best-practices`): Council President for Gate G-Review and Gate G-Complexity sign-off.
    - **`Quota Sentinel`** (Skills: `antigravity-guide`, `stripe-best-practices`): Dedicated quota monitor that detects 1% remaining capacity, notifies all active agents and the main conversation, and schedules wait timers until quota reset.
    - **`Git Workflow Master`** (Skills: `antigravity-guide`, `stripe-best-practices`): Mandatory manager for all Git and GitHub operations across all tiers—including worktree provisioning/pruning (`git worktree add`/`remove`), branch creation, atomic conventional commits, remote tracking (`git push -u`), GitHub PR creation (`gh pr create` with strict unit naming), and cascading promotion merges (`gh pr merge`).
    - **`DevOps Automator`** (Skills: `antigravity-guide`, `stripe-best-practices`): Cargo target cache (`CARGO_TARGET_DIR`), CI/CD pipeline management.
  - Tasks and gatekeeper reviews must be dispatched across the 3 profile generators in a round-robin / quota-balanced manner to evenly split the quota bill.
- **Hierarchical Validation & Merge Pipeline**:
  - `Units (1..5) Tested` -> `Merge to Story Branch` -> `Story Integration Validated` -> `Merge to Module Branch` -> `Full Workspace Validated` -> `Merge to Main`.
