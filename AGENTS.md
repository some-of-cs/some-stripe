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

5. **[Quality Gates & Code Review Standard (G0–G4 + G-Review)](.agents/rules/quality-gates-and-code-review.md)**:
   - Code must compile with 0 warnings on Rust 2024 edition (`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`).
   - Mandatory test coverage: >=90% line and >=85% branch coverage in affected modules.
   - Microkernel architecture: keep the core runtime lean with zero monolithic codegen.

6. **[Global Financial & Statutory Legal Compliance (Gate G-Law)](.agents/rules/legal-compliance.md)**:
   - PCI-DSS 4.0 data security and zero raw PAN handling in application memory.
   - PSD2 Strong Customer Authentication (SCA) 3D Secure 2.0 redirect support.
   - GDPR Article 15/17 consumer data rights and ISO 4217 minor currency unit arithmetic.

7. **[Multi-Agent Divide-and-Conquer PR Lifecycle & Merge Protocol](.agents/rules/divide-and-conquer-pr-workflow.md)**:
   - **Mandatory `.worktrees/` Directory Invariant**: All agents MUST work strictly inside the `.worktrees/` folder in this directory (`/workspaces/some-stripe/.worktrees/<branch-sanitized-name>`). Direct code modification on `main` or at the repository root is strictly prohibited.
   - **Strict Bottom-Up PR Creation Order**: Level 3 micro-units (Unit 1 -> 5) MUST be implemented, verified, committed, pushed, and have their GitHub PR opened (`gh pr create`) BEFORE the Level 2 User Story PR is opened. Never open a parent story/module PR before child unit PRs exist.
   - **Multi-Tier Merge & Validation Workflow (Strictly via `gh pr merge`)**:
     - **All tier merges MUST happen over GitHub using GitHub CLI (`gh pr merge`)**. Local `git merge` commands across promotion tiers are strictly prohibited.
     - **Mandatory Pre-Merge Review Sign-Off (Gates G-Review, G2, G-Borrow)**: Before calling `gh pr merge` on ANY pull request (unit, story, or module), an explicit code review and security validation must be executed and approved via `gh pr review <PR> --approve -b '...'`. Merging without documented quality gate review is strictly prohibited.
     1. Run tests over all 5 Level 3 units in a user story -> review & approve -> merge units into the story branch via `gh pr merge <PR> --merge`.
     2. Pull updated story branch, run story integration and contract wire tests -> review & approve -> merge user story PRs into the module branch via `gh pr merge <PR> --merge`.
     3. Pull updated module branch, run workspace-wide test suite (`cargo test --workspace`) and quality gates -> review & approve -> merge the module PR into `main` via `gh pr merge <PR> --merge`.
   - **Streaming Pipelined PR Creation & Ordered Cascading Merge Queue**:
     - Subagents MUST be granted full read, write, and command execution tools (`EnableWriteTools: true`).
     - As soon as each micro-unit finishes implementation and local tests, it MUST immediately push and open its GitHub PR (`gh pr create`) without waiting for other units.
     - Merges MUST follow the strict sequential dependency order ($1 \to 2 \to 3 \to 4 \to 5$):
       - Unit 1 merges into the story branch immediately upon completion and gate review.
       - Unit 2 waits for Unit 1 to merge, then pulls/merges the updated story branch, runs full validation (`cargo test`, `cargo clippy`, `cargo fmt`), passes gate review, and merges into the story branch.
       - Unit 3 waits for Unit 2 to merge, updates against the story branch, validates, and merges.
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
- **Specialized Agents (Role & Task Specific)**:
  - Agents are instantiated dynamically with specific tasks and roles (e.g. `microkernel-architect`, `security-webhook-engineer`, `domain-codegen-specialist`, `code-reviewer`, `api-tester`, `rust-refactoring-specialist`, or task-level unit implementers).
  - Tasks must be dispatched across the 3 profile generators in a round-robin / quota-balanced manner to evenly split the quota bill.
- **Hierarchical Validation & Merge Pipeline**:
  - `Units (1..5) Tested` -> `Merge to Story Branch` -> `Story Integration Validated` -> `Merge to Module Branch` -> `Full Workspace Validated` -> `Merge to Main`.
