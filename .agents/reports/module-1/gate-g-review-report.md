# Gate G-Review & Gate G-Borrow Code Review Report: `module-1-core-reliability`

**Reviewer / Profile**: Code Reviewer Subagent (`agent-c`), Council President  
**Assigned Worktree**: `/workspaces/some-stripe/.worktrees/patch-module-1-core-reliability`  
**Target Branch**: `patch/module-1-core-reliability` (against `main`)  
**Scope**: Module 1 Core Reliability, Borrowing Discipline, Anti-Traversal Security, Idempotent Retries, Connection Pooling, Cursor Loop Protection  
**Date**: November 6, 2026  

---

## 1. Executive Summary

This code review evaluates the changes introduced in `patch/module-1-core-reliability` across the microkernel crate [`some-stripe-core`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core).

The modifications enhance production reliability, cryptographic hygiene, zero-clone borrowing discipline, and defense-in-depth API security:
1. **Gate G-Borrow**: Zero deep clone discipline strictly maintained across all domain entities, requests, and pagination models. Move semantics and borrowed references (`&T`, `&str`, slices) are used everywhere; only shallow `Arc` atomic pointer copies are permitted.
2. **Gate G0**: Zero-warning Rust 2024 compilation (`cargo clippy --workspace --all-targets -- -D warnings`), formatting compliance (`cargo fmt --check`), and 100% test pass rate across 31 tests + 18 doctests.
3. **Gate G2 & Gate G-Law**: In-memory secret scrubbing via `zeroize::ZeroizeOnDrop`, `[REDACTED]` debug masking, constant-time secret equality (`subtle::ConstantTimeEq`), and strict redirect policy (`Policy::none()`) preventing auth token leakage.
4. **CRUD Anti-Traversal Validation**: Robust validation against directory traversal tokens (`..`), path separators (`/`, `\`), and URI query/fragment injection characters (`?`, `#`) in [`validate_and_encode_id`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/patterns/crud.rs#L35-L55).
5. **Idempotency-Aware Retries & Connection Pooling**: Production [`ReqwestTransport`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/patterns/transport.rs#L38-L61) with 90-second idle pool, 60-second TCP keepalive, exponential backoff with deterministic jitter, and strict refusal to retry state-mutating POST requests on network drops unless protected by an `Idempotency-Key`.
6. **Cursor Sentinel & Resumption**: Bounded [`CursorLoopDetector`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/patterns/pagination.rs#L96-L115) to prevent infinite loops, alongside [`Paginator::with_starting_after`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/patterns/pagination.rs#L193-L197) cursor resumption.

---

## 2. Gate-by-Gate Audit Findings

### 2.1 Gate G-Borrow: Strict Zero-Clone Borrowing Discipline
- **Domain Entity & Stream Zero-Clone**:
  - In [`Paginator<'a, T>`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/patterns/pagination.rs#L125-L135), streamed items are stored in a `VecDeque<T>` and consumed via `pop_front()` move semantics. Stream consumption requires zero copies or clones (`T: DeserializeOwned` without requiring `T: Clone`).
  - Item cursor extraction closure `key_fn: impl Fn(&T) -> &str` operates strictly on borrowed references `&T`, avoiding any cloning of domain models or IDs.
  - Client state [`StripeClient`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/client.rs#L259-L263) wraps `Arc<ClientInner>`. Cloning the client produces only an `Arc` atomic pointer copy (`borrow_discipline_audit_test::test_client_clone_is_shallow_arc_pointer_copy` verified via `Arc::ptr_eq`).
- **Serializer Borrowing**:
  - [`encode_form_pairs`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/patterns/serializer.rs#L102-L114) takes a borrowed slice `&[(String, String)]` and references each item during percent-encoding, eliminating unnecessary clones.
  - [`StripeRequestBuilder::form_json`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/patterns/builder.rs#L94-L103) calls `to_form_params` once and encodes directly via `encode_form_pairs(&pairs)`, eliminating duplicate serialization traversals.
- **Verdict**: **PASS**

### 2.2 Gate G0: Zero-Warning Compilation & Test Suite
Commands executed:
```bash
touch src/crates/some-stripe-core/src/lib.rs && export CARGO_TARGET_DIR=/workspaces/some-stripe/target && cargo test -p some-stripe-core && cargo test --doc -p some-stripe-core && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```
- Unit & Integration Tests: **31 passed; 0 failed; 0 ignored**
  - `src/lib.rs`: 15 passed
  - `tests/borrow_discipline_audit_test.rs`: 3 passed
  - `tests/concurrency_test.rs`: 2 passed (including 10,000 Tokio tasks concurrency stress test)
  - `tests/facade_test.rs`: 2 passed
  - `tests/pagination_wire_test.rs`: 2 passed
  - `tests/sentinel_loop_detector_test.rs`: 2 passed
  - `tests/serializer_wire_test.rs`: 3 passed
  - `tests/serializer_zero_clone_audit_test.rs`: 2 passed
- Documentation Tests (`cargo test --doc`): **18 passed; 0 failed**
- Linter (`cargo clippy --workspace --all-targets -- -D warnings`): **0 warnings**
- Formatter (`cargo fmt --check`): **0 diffs**
- **Verdict**: **PASS**

### 2.3 Gate G2 & Gate G-Law: Security, Cryptography & Compliance
- **Secret Key Handling**: `SecretKey` redacts to `[REDACTED]` in `fmt::Debug`, implements `zeroize::ZeroizeOnDrop`, and checks equality via `subtle::ConstantTimeEq` (PCI-DSS 4.0 compliant, prevents timing side-channel attacks).
- **HTTP Redirect Policy**: Initialized with `redirect(reqwest::redirect::Policy::none())` in `ReqwestTransport`, preventing credential and basic auth header leakage over redirects per RFC 9110 / Gate G2.
- **Path Traversal & Injection Shield**:
  - [`validate_and_encode_id`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/patterns/crud.rs#L35-L55) rigorously validates resource IDs, rejecting `..`, `/`, `\`, `?`, and `#`.
  - [`try_resource_url_with_id`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/patterns/crud.rs#L89-L98) and [`try_resource_sub_url`](file:///workspaces/some-stripe/.worktrees/patch-module-1-core-reliability/src/crates/some-stripe-core/src/patterns/crud.rs#L126-L137) provide fallible, secure endpoint assembly.
- **Verdict**: **PASS**

### 2.4 Architectural Correctness & Production Resilience
- **Idempotency-Aware Network Retries**:
  - Safe HTTP methods (`GET`, `HEAD`, `DELETE`, `OPTIONS`, `PUT`) retry across network drops.
  - State-mutating `POST` requests are **never** retried on network drops without an `Idempotency-Key` header, preventing duplicate financial transactions on Stripe.
  - Unit tests `test_non_idempotent_post_without_idempotency_key_does_not_retry_network_error` and `test_non_idempotent_post_with_idempotency_key_retries_network_error` verify this invariant.
- **Backoff & Jitter**:
  - Exponential backoff with pseudorandom jitter implemented in `compute_backoff_delay(attempt, base_ms, max_ms)`.
  - Zero-indexed retry tracking (`retries_taken < max_retries`) correctly respects `config.max_retries`.
- **Connection Pooling**:
  - Configured with `pool_idle_timeout(Duration::from_secs(90))`, `pool_max_idle_per_host(32)`, and `tcp_keepalive(Duration::from_secs(60))`.
- **Sentinel Loop Detection**:
  - `CursorLoopDetector` prevents infinite pagination loops when upstream Stripe responses repeat cursor tokens.
- **Verdict**: **PASS**

---

## 3. Review Checklist Verification

| Quality Gate | Description | Status | Evidence |
| :--- | :--- | :---: | :--- |
| **Gate G0** | 0 warnings, formatting clean, 100% test pass rate | **PASS** | 49 passed (31 tests + 18 doctests), 0 clippy warnings, 0 fmt diffs |
| **Gate G-Borrow** | Zero deep clones on domain entities, explicit lifetimes | **PASS** | Move semantics in `Paginator`, shallow `Arc` client clones |
| **Gate G2** | Cryptographic side-channel & token leak protection | **PASS** | `ConstantTimeEq`, `ZeroizeOnDrop`, `Policy::none()` |
| **Gate G-Law** | PCI-DSS 4.0 secret hygiene & redaction | **PASS** | `[REDACTED]` Debug masking, zero raw credential persistence |
| **Gate G-Docs** | Rustdoc examples and doctests on all public APIs | **PASS** | 18 runnable doctests passing via `cargo test --doc` |
| **Architecture** | Microkernel patterns, CRUD anti-traversal, connection pooling | **PASS** | Traversal validation, idempotency guards, 90s idle pool |

---

## 4. Final Review Determination

All requirements, architectural standards, borrowing invariants, security boundaries, and test suites have been rigorously evaluated and verified. The patch is ready for promotion.

### Review Status: APPROVED (Clean)
