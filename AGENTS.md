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
   - Hierarchical PR promotion: Level 3 atomic units -> Level 2 story branches -> Level 1 module branches -> Level 0 `main`.
   - Quality gates must be validated before merging at every level.

---

## 2. Multi-Profile Agent Environment

When launching agents via [`scripts/launch-agent.sh`](scripts/launch-agent.sh):
- **Profile 1 (`agent-a`)**: Microkernel Core, Concurrency, Serialization, V2 Engine (`microkernel-architect`)
- **Profile 2 (`agent-b`)**: Cryptographic Webhook Engine, Edge Security, Replay Guards (`security-webhook-engineer`)
- **Profile 3 (`agent-c`)**: OpenAPI Domain Plugins, Payments, Billing, Connect, Integration Tests (`domain-codegen-specialist`)
