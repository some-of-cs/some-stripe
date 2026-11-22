# Module `9`: `some-payment` Orchestrator Alignment & Optional Adapter (`feat/module-9-some-payment-alignment`)

- **Module Branch**: `feat/module-9-some-payment-alignment`
- **Base Branch**: `main`
- **Field Profile**: `software`
- **User Stories**: `2` Stories (`US-9.1.0`, `US-9.2.0`) | **Total Parallel Units**: `9` Units
- **Merge Order Position to `main`**: 9th (after `some-payment` Module 1 `some-payment-core` is available; prerequisite for `some-payment` Module 4)

## 1. User Story & Implementation Unit Index

| Story ID | Title & Spec Link | $S_{\text{story}}$ Score | Parallel Units ($K$) | Owned Directory / Paths |
| :--- | :--- | :---: | :---: | :--- |
| `US-9.1.0` | [`us-9.1.0-stripe-core-hardening-and-currency-guardrails.md`](./us-9.1.0-stripe-core-hardening-and-currency-guardrails.md) | `2.60` | `5` | `src/crates/some-stripe-core/src/{version_transport,idempotency_passthrough}.rs, src/crates/some-stripe-types/src/{currency_guardrails,payment_error_detail,refund_lifecycle}.rs` |
| `US-9.2.0` | [`us-9.2.0-stripe-webhook-hardening-and-adapter.md`](./us-9.2.0-stripe-webhook-hardening-and-adapter.md) | `2.45` | `4` | `src/crates/some-stripe-webhooks/src/multi_v1_verifier.rs, src/adapter/{mod,mappers,provider}.rs` |

---

## 2. Copy-Paste Module Execution Prompt

Copy and paste the prompt block below directly into your AI agent session to execute **Module `9`** end-to-end using the `some-divide-and-conquer` multi-agent workflow:

```text
Execute Module 9 (`some-payment` Orchestrator Alignment & Optional Adapter) on https://github.com/some-of-cs/some-stripe (base: main) using the some-divide-and-conquer plugin in full parallel mode.

### 1. Architecture, Scope & Feature/Deliverable Contract
- Module branch: `feat/module-9-some-payment-alignment` (base: `main`).
- Feature flags / Deliverable boundaries: Hardens `some-stripe` (`v0.8.0` -> `v0.9.0`) against all P0/P1 items in `some-payment/docs/sdk-implementation.md` and adds optional `some-payment-adapter = ["dep:some-payment-core"]` feature without breaking standalone `some-stripe` usage.
- **Upstream `some-payment-core` Contract (Merged on `some-of-cs/some-payment` `main` at `b8e2ca4`)**:
  - Add optional dependency in `Cargo.toml`:
    `some-payment-core = { git = "https://github.com/some-of-cs/some-payment", package = "some-payment-core", optional = true }`
  - Implement `some_payment_core::PaymentProvider` (`id`, `capabilities`, `create_payment`, `get_payment`, `create_checkout`, `capture`, `void_payment`, `refund`, `parse_and_verify_webhook`) behind `#[cfg(feature = "some-payment-adapter")]`.
  - Use exact `some-payment-core` types: `Money`, `CurrencyCode`, `ProviderId`, `ProviderReference`, `PaymentError` (redacted `Debug` & `Display`, `is_definitive_client_rejection` excludes `ProviderUnavailable` and HTTP `408`/`429`), `PaymentCapabilities` (14 flags), `CreatePaymentRequest`, `CheckoutSessionRequest`, `CaptureRequest`, `VoidRequest`, `RefundRequest`, `PaymentResult`, `RefundResult`, `PaymentStatus`, `NextAction` (`ZeroizeOnDrop`), `RawWebhookRequest` (redacted `Debug`, zeroized on drop), `NormalizedWebhookEvent`, and `VerificationChallenge`.
- Specifications to implement 100% (every Acceptance Criterion `AC-*` and Test Case `TC-*`):
  1. `docs/user-stories/module-9-some-payment-alignment/us-9.1.0-stripe-core-hardening-and-currency-guardrails.md`
  2. `docs/user-stories/module-9-some-payment-alignment/us-9.2.0-stripe-webhook-hardening-and-adapter.md`

### 2. Agent Roster, Roles & Skills (29 Total Agents)
- **Shared Infrastructure Agents (3)**:
  - `git-workflow-master` (1): Exclusive owner of `.worktrees/`, branches, PR creation, and ordered cascading merges.
  - `github-projects-kanban` (1): Syncs `.agents/checkpoints/module-9-some-payment-alignment/checkpoint.json` and PR conversation reports.
  - `quota-sentinel` (1): Monitors rate limits and flushes state on HTTP 429 / 1% remaining quota.
- **Tier 1 — Module Level (2 Agents)**:
  - `module-leader` (1) on `feat/module-9-some-payment-alignment` (3-minute watchdog `*/3 * * * *`). NEVER spawns Unit Implementers directly and NEVER edits code directly.
  - Independent `code-review-council` + `code-reviewer-council-president` (1) for the Module PR.
- **Tier 2 — Story Level (6 Agents = 2 Story Leaders + 2 Integration Verifiers + 2 Story Review Councils)**:
  - 2 `story-leader` agents (`US-9.1.0`, `US-9.2.0`, 1-minute watchdog `* * * * *`). NEVER edits code directly.
  - 2 integration/API tester agents (`testing-api-tester`) for story-level verification.
  - 2 independent `code-review-council` subagents (1 per Story PR).
- **Tier 3 — Unit Level (18 Agents = 9 Unit Implementers + 9 Independent Unit Review Councils)**:
  - **US-9.1.0 (5 Units + 5 Reviewers)**:
    - Unit 1 (`feat/us-9.1.0/unit-1-version-transport` -> `src/crates/some-stripe-core/src/version_transport.rs`): Explicit `Stripe-Version: 2026-11-18.endive` config, injectable `reqwest::Client`, timeout & redacted `Debug` | Agent: `backend-architect`
    - Unit 2 (`feat/us-9.1.0/unit-2-currency-guardrails` -> `src/crates/some-stripe-types/src/currency_guardrails.rs`): Lowercase ISO-4217 normalization, 0-dec, ISK/UGX `00`, and 3-dec multiple-of-10 (`BHD`/`JOD`/`KWD`/`OMR`/`TND`) guardrails | Agent: `backend-architect`
    - Unit 3 (`feat/us-9.1.0/unit-3-payment-error-detail` -> `src/crates/some-stripe-types/src/payment_error_detail.rs`): `LastPaymentError` deserialization (`decline_code`, `advice_code`, `network_decline_code`) & CheckoutSession cancel/expire rule | Agent: `api-platform-engineer`
    - Unit 4 (`feat/us-9.1.0/unit-4-refund-lifecycle` -> `src/crates/some-stripe-types/src/refund_lifecycle.rs`): Full 5-status Refund resource (`pending_reason`, `next_action`, `failure_reason`) + `POST /v1/refunds/:id/cancel` | Agent: `api-platform-engineer`
    - Unit 5 (`feat/us-9.1.0/unit-5-idempotency-passthrough` -> `src/crates/some-stripe-core/src/idempotency_passthrough.rs`): Caller `Idempotency-Key` (`<= 255` chars) validation & retry-safe key preservation | Agent: `backend-architect`
  - **US-9.2.0 (4 Units + 4 Reviewers)**:
    - Unit 1 (`feat/us-9.2.0/unit-1-multi-v1-verifier` -> `src/crates/some-stripe-webhooks/src/multi_v1_verifier.rs`): Order-agnostic `Stripe-Signature` parser, constant-time multi-`v1` verification, `v0`/debug ignore, 300s replay window, snapshot vs thin detection | Agent: `security-engineer`
    - Unit 2 (`feat/us-9.2.0/unit-2-stripe-mappers` -> `src/adapter/mappers.rs`): Pure mappers between `some-stripe` types and `some-payment-core` (`Money`, `PaymentStatus`, `NextAction`, `PaymentError`) | Agent: `backend-architect`
    - Unit 3 (`feat/us-9.2.0/unit-3-stripe-provider` -> `src/adapter/provider.rs`): `#[cfg(feature = "some-payment-adapter")] pub struct StripeProvider` implementing `PaymentProvider` | Agent: `backend-architect`
    - Unit 4 (`feat/us-9.2.0/unit-4-stripe-conformance` -> `src/adapter/conformance_tests.rs`): Offline `wiremock` test suite verifying capabilities, currency rules, webhooks, and secret redaction | Agent: `test-automation-engineer`
- **On-Demand Remediation Agent**: `patch-implementer` (spawned by any Story Leader or Module Leader whenever a Review Council, CodeScene, or CodeRabbit check flags any issue).
- **Skills Loaded**: `divide-and-conquer-orchestrator`, `preflight-git-gh`, `git-workflow-master`, `github-projects-kanban`, `quota-sentinel`, `security-audit`, `code-review-council`, `code-reviewer-council-president`, `cyclomatic-complexity`, `patch-implementer`.

### 3. Mandatory Anti-Regression Rules (Hard Invariants)
1. **Zero Shared-Index (`lib.rs` / `mod.rs` / `Cargo.toml`) Clobbering Across Stories & Units**: In Wave 1, `module-leader` MUST pre-scaffold all module index files (`lib.rs`, `mod.rs`, `Cargo.toml`) and empty stub files for all `9` units on `feat/module-9-some-payment-alignment` BEFORE branching the story branches. Story and Unit branches MUST NOT delete sibling declarations.
2. **Never Self-Review or Merge Before Independent Subagent Approval**: Every Unit PR, Story PR, Patch PR, and Module PR MUST receive a structured Implementation Report comment AND an independent `code-review-council` subagent review (`event="COMMENT"` when using a single GitHub account to avoid self-approval API errors). Loop until `Gate G-Review: Approved` with 0 issues.
3. **Plain UTF-8 Commits Only**: When committing files via GitHub API / MCP tools (`GITHUB_COMMIT_MULTIPLE_FILES`), ALWAYS pass plain-text UTF-8 strings with `encoding: "utf-8"` (never pre-encode Base64).
4. **Static Lookup Tables for Multi-Branch Mapping (`CC <= 6`, Nesting `<= 2`)**: Any enum or dispatcher with >5 variants mapping to/from strings MUST use a static `const KNOWN_*: &[(&str, Self)]` lookup table so cyclomatic complexity stays `CC <= 3`.
5. **Security, Bounds & Secret Hygiene (`G2` & `G-Law`)**: Enforce domain bounds during deserialization, require non-empty suffixes on prefixed IDs, redact all secrets/tokens/PII (`[REDACTED]`) in `Debug`/logs, zeroize intermediate credential buffers, and use checked integer math for monetary values (zero `f32`/`f64` floats).
6. **Complete Documentation, Runnable Doc-Tests & Compile-Fail Guards (`G-Docs` & `G1`)**: Every public symbol/interface MUST include doc comments, runnable doc-tests, and negative/compile-time guards where applicable.
7. **Memory-Safe Verification Execution**: Run `CARGO_BUILD_JOBS=1 cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo test --doc` sequentially so linking never OOMs.

### 4. Step-by-Step Execution & Strict Merge-Order Constraint into `main`
- **Step 1 (Gate 0 & Wave 1 Scaffolding)**: Run `preflight-git-gh`, start `quota-sentinel`, create `feat/module-9-some-payment-alignment` from `main`, pre-scaffold index files + `9` empty unit stub files, and spawn all `2` `story-leader` subagents in parallel.
- **Step 2 (Wave 2 Parallel Unit Loop)**: Each `story-leader` spawns its `K` Unit Implementers in parallel. Each Unit Implementer implements its owned file + unit tests + doc-tests, verifies build/lint/format, opens its Unit PR, posts its Implementation Report comment, and loops with its dedicated Unit Review Council subagent until `APPROVED`.
- **Step 3 (Wave 3 Ordered Cascading Unit Merges & Story Review Loop)**: Merge Unit PRs `1 -> 2 -> ... -> K` one by one per story (rebasing/updating each next Unit PR onto the updated story branch). Open each Story PR into `feat/module-9-some-payment-alignment`, post the Story Implementation Report, run the independent Story Review Council subagent, and spawn `patch-implementer` if any issue is flagged until all `2` Story PRs are `APPROVED` and merged sequentially.
- **Step 4 (Wave 4 Module PR, Whole-Module Audit & Strict `main` Merge Order)**:
  - Open the Module `9` PR (`feat/module-9-some-payment-alignment` -> `main`), post the Module Implementation Report, and run the independent Module `9` Review Council subagent (+ `patch-implementer` loop if needed) until `GATE G-REVIEW APPROVED` with 0 issues.
  - **STRICT MERGE ORDER CONSTRAINT TO `main` (`Modules 1-8 (merged) -> Module 9 (after some-payment Module 1, before some-payment Module 4)`)**: Module `9` MUST be merged into `main` in position **9th (after `some-payment` Module 1 `some-payment-core` is available; prerequisite for `some-payment` Module 4)**. Before merging into `main`, sync/rebase `feat/module-9-some-payment-alignment` onto the latest `main`, re-run the full verification suite (`fmt`, `clippy`, `test`, `doc`), and then merge into `main`.
```
