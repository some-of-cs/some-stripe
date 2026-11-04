# Microkernel Documentation, Doctests & README Standard (Gate G-Docs)

## 1. Overview & Architectural Invariants

In `some-stripe`, every microkernel crate (located under `src/crates/<crate-name>/`) is an independently publishable, highly modular Rust crate designed around single-responsibility boundaries.

To guarantee world-class developer experience, prevent regression, and enforce production-grade quality, all agents MUST adhere to these documentation and verification invariants:

---

## 2. Mandatory Crate-Level README (`src/crates/<crate-name>/README.md`)

Every microkernel crate MUST include a comprehensive `README.md` containing:
1. **Badges**: Crates.io version, Apache-2.0 license, Rust 2024 edition badge.
2. **Architectural Highlights**: Clear explanations of core design invariants (e.g. concurrency models, zero-clone borrowing, constant-time cryptography, drift guards).
3. **Quickstart & Runnable Code Examples**: Standalone, copy-pasteable usage examples showing initialization, request construction, and error handling.
4. **Quality Gates & Verification Table**: Explicit status table documenting compliance with Gates G0, G-Borrow, G2, G-Law, and G1.

---

## 3. Mandatory Rustdoc & Working Doctests (`cargo test --doc`)

1. **Module-Level Documentation**:
   - The root module (`src/lib.rs`) of each microkernel crate MUST contain comprehensive `//!` rustdoc documentation explaining the module's domain purpose, architectural guarantees, and security model.
2. **Working Doctests**:
   - Every public microkernel crate MUST provide runnable doctests embedded in its rustdoc comments.
   - Doctests MUST compile and pass cleanly via `cargo test --doc` with 0 warnings or failures.
   - Serde field attributes (such as `#[serde(default, skip_serializing_if = "Nullable::is_omit")]`) MUST be demonstrated with runnable assertions verifying wire payload serialization.

---

## 4. Verification & CI Enforcement

Before submitting any unit, story, or refactoring PR involving microkernels, the agent MUST verify:
- `cargo test --workspace` passes all unit tests, integration wire tests, and doc-tests.
- `cargo clippy --all-targets -- -D warnings` reports 0 warnings.
- `cargo fmt --check` reports 0 formatting differences.
- All microkernel `README.md` files are kept up to date with code changes.
