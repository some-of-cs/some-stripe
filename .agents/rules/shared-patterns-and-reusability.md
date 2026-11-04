# Rule: Shared Patterns, Zero Redundancy & Mandatory Documentation/Tests

## Core Objective
Strictly eliminate code duplication (DRY) across all resource implementations, co-locate reusable architectural patterns in a unified patterns module, and enforce 100% rustdoc documentation with working doctests and unit tests on every public component.

## Mandatory Invariants

### 1. Zero Redundant Boilerplate (DRY)
- **Shared Request Execution**:
  - Individual resource services MUST NOT duplicate low-level HTTP client calls, manual idempotency header construction, basic auth injection, or raw response unwrapping.
  - All endpoint invocations MUST route through shared patterns in `src/patterns/` (e.g. `StripeRequestBuilder`, shared CRUD abstractions).
- **Reusable Resource Modeling**:
  - Common patterns—such as expandable foreign references (`Expandable<T>`), 3-state nullability (`Nullable<T>`), metadata maps (`Metadata`), and cursor lists (`List<T>`)—MUST reside in shared modules and never be re-implemented per-resource.
- **Pattern Co-location**:
  - All architectural abstractions, generic builders, and declarative endpoint adapters MUST be centralized in `src/patterns/` (or `crates/some-stripe-core/src/patterns/`) to maximize reuse across all 38+ Stripe services.

### 2. Mandatory Documentation & Doc-Tests
- **100% Public API Documentation**:
  - Every public struct, enum, trait, function, method, and macro MUST have complete rustdoc comments (`///`).
  - Documentation must explain:
    1. Purpose and behavior.
    2. Parameters and invariants.
    3. Error conditions (`# Errors`).
    4. Practical, executable code examples (`# Examples`).
- **Working Doc-Tests**:
  - Every example in rustdoc MUST compile and pass under `cargo test --doc`.
  - Use `no_run` only when requiring live network credentials; purely local logical examples must execute and assert correctness directly.

### 3. Comprehensive Unit Testing
- **Co-located Unit Tests**:
  - Every pattern and logic component MUST have co-located `#[cfg(test)] mod tests` verifying:
    1. Happy path execution.
    2. Boundary conditions and zero/empty states.
    3. Error handling and invalid input rejection.
    4. Memory safety and zero unauthorized `.clone()` calls.
