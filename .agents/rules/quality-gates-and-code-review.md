# Rule: Quality Gates & Code Review Standard (Federated Gatekeeper Council)

## Core Objective
Ensure that every contribution to `some-stripe` adheres to the highest standards of Rust engineering, idiomatic ergonomics, test coverage, compiler warnings hygiene, and statutory compliance.

All quality gates MUST be executed and verified by **independent specialized subagents** selected from `.agents/agents/` paired with their mandatory **specialized skills** from `.agents/skills/`.

> [!CAUTION]
> **Zero Self-Approval Invariant**: The authoring implementer or parent leader CANNOT sign off on their own PR. Every PR requires an independent review and sign-off posted by an independent subagent before `gh pr merge` is permitted.

---

## 1. Federated Gatekeeper Subagent & Skill Matrix

Every quality gate is explicitly owned and executed by a specialized subagent persona activated with dedicated skill capabilities:

| Gate ID | Gate Name | Assigned Subagent Persona | Persona Spec (`.agents/agents/`) | Mandatory Skill(s) (`.agents/skills/`) | Core Verification Scope |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Gate G0** | Syntax, Fmt & Clippy Hygiene | **Rust Refactoring Specialist** | `engineering-rust-refactoring-specialist.md` | `upgrade-stripe` | `cargo check`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (0 warnings). Safe compiler remediation without shortcuts. |
| **Gate G-Borrow** | Zero-Clone Discipline | **Rust Refactoring Specialist** | `engineering-rust-refactoring-specialist.md` | `upgrade-stripe` | Enforce Rust 2024 zero-clone borrowing (`&T`, `'a`, `bytes::Bytes`). Zero deep `.clone()` calls on domain entities/models; `Arc::clone` permitted only. |
| **Gate G2** | Security & Cryptography | **Application Security Engineer** | `security-appsec-engineer.md` | `security-audit`<br>`stripe-best-practices` | Constant-time HMAC-SHA256 (`safe_constant_time_eq`), 300s drift guard, `ZeroizeOnDrop`, `Debug` redaction (`[REDACTED]`), 1MB payload limits, CSPRNG idempotency keys. |
| **Gate G-Law** | Global Statutory Compliance | **Application Security Engineer** | `security-appsec-engineer.md` | `security-audit`<br>`stripe-best-practices` | PCI-DSS 4.0 (zero raw PAN in application memory), PSD2 SCA 3DS2 redirect URLs, GDPR Art 15/17 rights, ISO 4217 minor currency units (zero float rounding), EU Directive 2011/83/EU 14-day restitution, EU Omnibus Directive 2019/2161 price transparency. |
| **Gate G1** | Acceptance Test Coverage | **API Tester** | `testing-api-tester.md` | `stripe-docs`<br>`stripe-best-practices` | Comprehensive acceptance test suite. Verify >=90% line and >=85% branch coverage in affected modules. Boundary tests, error paths, HTTP status mapping, idempotency deduplication. |
| **Gate G3** | Microkernel Architecture | **microkernel-architect** | `microkernel-architect.md` | `stripe-docs`<br>`stripe-best-practices` | Microkernel runtime lean with zero monolithic codegen. Modular crate boundaries, progressive Cargo feature flags, and dual-protocol serialization (`v1` RFC 1738 indexed bracket vs `v2` JSON). |
| **Gate G4** | Wire Contract & Integrated E2E | **API Tester** | `testing-api-tester.md` | `stripe-docs`<br>`stripe-best-practices` | End-to-end integration journeys against live wire mock fixtures and `stripe-mock`. Verify serialization/deserialization fidelity, pagination streams, retries, and network failure resilience. |
| **Gate G-Docs** | Microkernel Documentation | **Technical Writer** | `engineering-technical-writer.md` | `stripe-docs` | Crate `README.md` passing the 5-second test, comprehensive rustdoc on public APIs with runnable doctests (`cargo test --doc`), and runnable serde examples verifying wire format. |
| **Gate G-Review** | Senior Code Review & Sign-Off | **Code Reviewer** | `engineering-code-reviewer.md` | `security-audit`<br>`stripe-best-practices` | Council President. Independent code review of `gh pr diff <PR>`. Reviews edge cases, panic safety, maintainability. Synthesizes gatekeeper findings and posts formal PR approval sign-off comment. |
| **Git & PR Lifecycle** | Version Control & PR Management | **Git Workflow Master** | `engineering-git-workflow-master.md` | `antigravity-guide`<br>`stripe-best-practices` | Worktree orchestration (`git worktree add/remove`), atomic conventional commits, remote tracking branches, Pull Request creation (`gh pr create`), and cascading promotion merges (`gh pr merge`). |

---

## 2. Gate Verification Execution Protocol

### Step 1: Pre-Submission Verification (Authoring Implementer & Git Workflow Master)
- Implementer develops code inside designated worktree (`/workspaces/some-stripe/.worktrees/<worktree-id>`).
- Implementer exports `export CARGO_TARGET_DIR=/workspaces/some-stripe/target`.
- Implementer runs local verification: `cargo test` and `cargo check`.
- Implementer hands off to `Git Workflow Master` (Skills: `antigravity-guide`, `stripe-best-practices`) to stage atomic commits, push branch with `-u origin <branch>`, and create GitHub Pull Request via `gh pr create`.
- `Git Workflow Master` attaches initial execution report to the PR.
- Implementer transitions Kanban task status to `🔍 In Review / Gate Sign-Off`.
- **Implementer stops and yields to the Gatekeeper Council. Implementer CANNOT self-approve or merge.**

### Step 2: Specialized Subagent Invocations with Dedicated Skills
The Supervising Leader or Main Agent Supervisor invokes the designated specialized subagents, each activating their specialized skills:

1. **For Level 3 Micro-Unit PRs**:
   - `Rust Refactoring Specialist` (Skill: `upgrade-stripe`): Evaluates **Gate G0** & **Gate G-Borrow**.
   - `Application Security Engineer` (Skills: `security-audit`, `stripe-best-practices`): Evaluates **Gate G2** & **Gate G-Law**.
   - `API Tester` (Skills: `stripe-docs`, `stripe-best-practices`): Evaluates **Gate G1**.
   - `Code Reviewer` (Skills: `security-audit`, `stripe-best-practices`): Performs comprehensive diff review, audits panic safety, edge cases, aggregates all gate findings, and posts the formal Gate G-Review approval comment.

2. **For Level 2 User Story PRs**:
   - `API Tester` (Skills: `stripe-docs`, `stripe-best-practices`): Evaluates **Gate G1** & **Gate G4** (story integration and wire mock suite).
   - `microkernel-architect` (Skills: `stripe-docs`, `stripe-best-practices`): Evaluates **Gate G3** (microkernel modularity, crate isolation, Cargo feature flags).
   - `Technical Writer` (Skill: `stripe-docs`): Evaluates **Gate G-Docs** (`cargo test --doc`, crate documentation).
   - `Code Reviewer` (Skills: `security-audit`, `stripe-best-practices`): Performs story-wide diff review, verifies unit merge integrity, and posts formal Gate G-Review approval comment.

3. **For Level 1 Module PRs**:
   - Full Federated Gatekeeper Council executes workspace-wide validation:
     - `cargo test --workspace`
     - `cargo clippy --workspace --all-targets -- -D warnings`
     - `cargo test --doc`
     - End-to-end integration and wire contract tests.
   - `Code Reviewer` posts the final Module Release Gate G-Review sign-off comment.

---

## 3. Mandatory GitHub PR Review Comment Template

The independent `Code Reviewer` subagent MUST post the following structured sign-off comment on the PR before merging:

```markdown
### 🛡️ Gate G-Review Approval — PR #<PR_NUMBER>

- **Independent Reviewer**: Code Reviewer (Billing Profile: <agent-a|agent-b|agent-c>)
- **Reviewer Skill Activated**: `security-audit`, `stripe-best-practices`
- **PR Scope**: `<feat(us-x.y.0/unit-N): title>`
- **Author**: `<implementer-role>` (Billing Profile: `<implementer-profile>`)

#### 📋 Federated Quality Gates Audit Summary

| Quality Gate | Assigned Subagent | Activated Skill | Verdict | Verification Details |
| :--- | :--- | :--- | :---: | :--- |
| **Gate G0** (Syntax & Clippy) | Rust Refactoring Specialist | `upgrade-stripe` | ✅ PASS | 0 clippy warnings, clean `cargo fmt --check` |
| **Gate G-Borrow** (Zero-Clone) | Rust Refactoring Specialist | `upgrade-stripe` | ✅ PASS | 0 unauthorized `.clone()`, explicit lifetimes `'a`, zero-copy `Bytes` |
| **Gate G2** (Security & Crypto) | Application Security Engineer | `security-audit`<br>`stripe-best-practices` | ✅ PASS | Constant-time HMAC, 300s drift guard, secret zeroization, `[REDACTED]` debug |
| **Gate G-Law** (Statutory & PCI) | Application Security Engineer | `security-audit`<br>`stripe-best-practices` | ✅ PASS | Zero PAN memory, ISO 4217 integer currency, EU 14-day restitution / price rules |
| **Gate G1** (Acceptance Tests) | API Tester | `stripe-docs`<br>`stripe-best-practices` | ✅ PASS | All acceptance criteria verified, line cov >=90%, branch cov >=85% |
| **Gate G3** (Microkernel Arch) | microkernel-architect | `stripe-docs`<br>`stripe-best-practices` | ✅ PASS | Clean crate boundaries, feature flags isolated, dual-protocol serializers |
| **Gate G4** (Wire Contract E2E) | API Tester | `stripe-docs`<br>`stripe-best-practices` | ✅ PASS | Wire mock contract verified against Stripe mock fixtures |
| **Gate G-Docs** (Doctests & Docs) | Technical Writer | `stripe-docs` | ✅ PASS | `cargo test --doc` passed, public API doc-tests passing, README compliant |

#### 🔍 Senior Code Review Findings
- **Correctness & Contract Safety**: Verified all edge cases, optionals, and error variants.
- **Panic Hardening**: Zero `unwrap()` in production paths; errors cleanly returned via `Result<T, StripeError>`.
- **Maintainability**: Clean idiomatic Rust 2024 patterns, reusable abstractions in patterns crate.

#### 🚀 Merge Verdict
**APPROVED FOR MERGE** into `<target-branch>` via `gh pr merge <PR_NUMBER> --merge`.
*(Executed by `Git Workflow Master` with skills `antigravity-guide` and `stripe-best-practices`)*
```

---

## 4. Multi-Profile Rotation for Gatekeeper Subagents
To prevent single-account quota concentration:
- Gatekeepers MUST be assigned alternating profiles from the authoring implementer:
  - If Implementer is on `agent-a` (`account1`), Gatekeepers (`Code Reviewer`, `AppSec`, `Rust Specialist`, `API Tester`) run on `agent-b` (`account2`) or `agent-c` (`account3`).
  - If Implementer is on `agent-b` (`account2`), Gatekeepers run on `agent-c` (`account3`) or `agent-a` (`account1`).
- Quota depletion triggers automatic failover to the next available profile in the round-robin sequence.
