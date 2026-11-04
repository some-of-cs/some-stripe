# Rule: Mandatory Agent Audit Logging & Full Execution Reporting

## Core Objective
Guarantee end-to-end accountability, auditability, and transparency across the multi-agent hierarchy by ensuring that every agent records a complete deliverable report detailing all actions taken, files altered, quality gates satisfied, and test outputs.

## Mandatory Invariants

### 1. Mandatory Report Structure
Every agent working at any tier (Level 3 Unit, Level 2 Story, Level 1 Module, or Level 4 Gatekeeper Council) MUST write a comprehensive markdown report upon task completion. Reports must follow this canonical template:

```markdown
# Agent Execution Report: <Task / Scope Name>

- **Agent Role**: <Role, e.g., Micro-Unit Implementer, Security Guardian>
- **Generated Via Profile**: <account1 | account2 | account3>
- **Worktree Path**: `.worktrees/<branch-sanitized-name>`
- **Branch**: `<branch-name>`
- **GitHub PR**: <PR Number & Link>
- **Commit SHA**: `<Commit Hash>`

---

## 1. Scope & Deliverables
- Detailed bullet points of exact functionality implemented or verified.

## 2. File Inventory
- **Created**: List of absolute or relative file paths created.
- **Modified**: List of modified files with brief description of changes.

## 3. Quality Gate Verification & Audit Results
- **Gate G0 (Rust 2024 Compilation)**: Status, Clippy (0 warnings), formatting check.
- **Gate G-Borrow (Zero-Clone Discipline)**: Verified borrowing patterns, pointer copies, move semantics.
- **Gate G2 (Security Gateway)**: Cryptographic invariants, secret redaction, constant-time checks.
- **Gate G-Law (Statutory & Financial Compliance)**: PCI-DSS 4.0 redaction, ISO 4217 minor currency units.
- **Gate G1 & G4 (Test & Wire Mock Validation)**: Automated test commands executed and full pass results.

## 4. Execution Logs & Verification Outputs
```bash
# Exact terminal commands executed and verbatim output summary
```

## 5. Reviewer Sign-Off
- Explicit approval comments by the Gatekeeper Council before GitHub PR merge.
```

### 2. File Location & Version Control
- All reports MUST be stored under `.agents/reports/<module-id>/`:
  - Unit reports: `.agents/reports/<module-id>/<user-story-id>/unit-<number>.md`
  - Story reports: `.agents/reports/<module-id>/<user-story-id>/story_summary.md`
  - Module reports: `.agents/reports/<module-id>/module_summary.md`
  - Gatekeeper audits: `.agents/reports/<module-id>/gatekeeper_audit.md`
- Reports MUST be committed to Git so they persist permanently in repository history.

### 3. Mandatory PR Body Attachment Invariant
When opening ANY pull request on GitHub (`gh pr create` / `gh pr edit`), the agent MUST directly embed the full report into the PR body or attach it as a primary audit comment.
The attached PR report MUST include:
- Scope and architectural deliverables.
- Complete inventory of created and modified files.
- Full verification and gate results: G0 (0 warnings), G-Borrow (zero-clone evidence), G2 (cryptography/security), G-Law (PCI-DSS & statutory compliance), G1/G4 (test coverage & wire mocks).
- Exact test execution commands and verbatim terminal output logs.
PRs submitted without a full report including validation and gate results are strictly prohibited and will be rejected by gatekeepers.
