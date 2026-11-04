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
   - Every pull request requires review and sign-off by `engineering-code-reviewer` across Correctness, Security, Maintainability, Performance, and Testing.
5. **Integrated Wire Validation (Gate G4)**:
   - Realistic end-to-end user journeys must be verified against live wire mock fixtures or `stripe-mock`.
