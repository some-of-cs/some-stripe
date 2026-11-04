# Rule: Quality Gates & Code Review Standard (G0–G4 + G-Review)

## Core Objective
Ensure that every contribution to `some-stripe` adheres to the highest standards of Rust engineering, idiomatic ergonomics, test coverage, and compiler warnings hygiene.

## Mandatory Invariants
1. **Zero-Warning Rust 2024 Compilation**:
   - Code must target the Rust 2024 edition (Rust 1.85+).
   - `cargo clippy --all-targets -- -D warnings` must pass with 0 warnings.
   - `cargo fmt --check` must pass with 0 formatting deviations.
2. **Comprehensive Test Coverage (Gate G1)**:
   - All acceptance criteria must be covered with automated tests.
   - Affected crates must maintain >=90% line coverage and >=85% branch coverage.
3. **Microkernel Architecture & Feature Isolation (Gate G3)**:
   - The root core runtime crate must remain ultra-lightweight with zero monolithic code-generated models.
   - Resource domains must be isolated behind modular Cargo feature flags.
4. **Independent Senior Code Review (Gate G-Review)**:
   - Every pull request requires an independent review and sign-off by a dedicated `Code Reviewer` subagent before merging.
   - **No Self-Approvals**: The authoring implementer or leader MUST NOT sign off on their own PR.
   - **Invocation Protocol**: Upon PR creation (`gh pr create`), the Story Leader or Supervisor invokes a `Code Reviewer` subagent (assigned an alternating billing profile).
   - **Review Audit Scope**:
     - Inspect `gh pr diff <PR>` for correctness, edge cases, and panic hardening.
     - Verify Gate G-Borrow (zero unauthorized `.clone()` calls, zero-copy references).
     - Verify Gate G2 (PCI-DSS credential redaction `[REDACTED]`, constant-time crypto).
     - Verify Gate G-Law (ISO 4217 zero-float integer arithmetic, statutory protections).
     - Verify Gate G4 (clean clippy, clean rustfmt).
   - **Formal GitHub Sign-Off**: The `Code Reviewer` posts the structured review sign-off directly on the PR (`gh pr comment <PR> --body "..."`).
   - Merging via `gh pr merge` is strictly blocked until this review comment is posted.
5. **Integrated Wire Validation (Gate G4)**:
   - Realistic end-to-end user journeys must be verified against live wire mock fixtures or `stripe-mock`.
