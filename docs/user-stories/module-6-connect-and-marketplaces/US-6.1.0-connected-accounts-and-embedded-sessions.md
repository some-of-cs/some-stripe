# US-6.1.0: Connected Accounts Onboarding, AccountSessions & KYC Verification

> **Spec Kit Specification**: [Specification Lifecycle Active (.specify/)](../../../.specify/memory/constitution.md)  
> **Behavior Blueprint**: `features/connect/modules/accounts/behaviours/account-onboarding/`  
> **Slicing Standard**: [10-PR Vertical Slicing Framework](../../user-stories/README.md#the-10-pr-vertical-slicing-framework-rust-sdk-adaptation)  
> **Security Gateway**: `security-audit` Level 2 Enforced  
> **Quality Gates**: `(G0–G4 + G-Review + G-Borrow + G-Law)`  

---

## 1. Spec Kit Specification

### 1.1 User Story Narrative
**As a** multi-party platform and marketplace architect,  
**I want to** provision Connected Accounts (Standard, Express, Custom), generate hosted Account Links for merchant onboarding, initialize embedded Account Sessions for client-side components, and manage KYC Person records,  
**So that** sellers and service providers can onboard seamlessly, verify their business identities according to regional AML/KYC mandates, and process marketplace payments with embedded payout management.

#### 1.1.1 Preconditions
- The `connect` feature flag is enabled.
- The platform Stripe account is activated with Connect platform capabilities.
- Country specifications, business type (`individual` or `company`), and capabilities (`card_payments`, `transfers`) are selected.

#### 1.1.2 Postconditions
- A new `Account` (`acct_...`) is created with configured controller properties and capabilities.
- Hosted onboarding URLs generated via `AccountLink` redirect merchants through Stripe-hosted KYC flows.
- `AccountSession` client secrets enable embedded Connect web components (e.g. payout management, document upload) in platform web applications.

### 1.2 Acceptance Criteria (Gherkin Scenarios)

```gherkin
Feature: Stripe Connect Account Provisioning and Onboarding

  Scenario: [AC-US-6.1.0-01] Create a Custom connected account with card_payments and transfers
    Given a platform onboarding a US company merchant
    When client.accounts().create(&CreateAccountParams { type: Some("custom".into()), country: Some("US".into()), capabilities: Some(AccountCapabilitiesParams { card_payments: Some(CapabilityStatus::Requested), transfers: Some(CapabilityStatus::Requested) }), ..Default::default() }) is executed
    Then an Account is created with status id "acct_custom_123"
    And capabilities for card_payments and transfers are in status "pending".

  Scenario: [AC-US-6.1.0-02] Generate an Account Link for hosted onboarding
    Given an unverified connected account "acct_123"
    When client.account_links().create(&CreateAccountLinkParams { account: "acct_123".into(), refresh_url: "https://platform.com/reauth".into(), return_url: "https://platform.com/return".into(), type: "account_onboarding".into(), ..Default::default() }) is executed
    Then an AccountLink is returned containing a temporary single-use onboarding URL ("https://connect.stripe.com/setup/c/...").

  Scenario: [AC-US-6.1.0-03] Create an AccountSession for embedded payout components
    Given an active connected account "acct_123"
    When client.account_sessions().create(&CreateAccountSessionParams { account: "acct_123".into(), components: AccountSessionComponents { payouts: Some(PayoutsComponent { enabled: true, ..Default::default() }), ..Default::default() } }) is executed
    Then an AccountSession is returned containing an ephemeral client_secret.

  Scenario: [AC-US-6.1.0-LAW] FinCEN Customer Due Diligence (CDD) and EU DAC7 / AMLD5 compliance
    Given any connected account onboarding or KYC Person collection
    When merchant identity documents or beneficial ownership details are collected
    Then beneficial ownership data (persons with >=25% equity) is collected and verified in strict accordance with FinCEN CDD Rule and EU AMLD5/DAC7 platform reporting directives.
```

### 1.3 Test Cases & Traceability Matrix

| Test Case ID | Target Criterion | Test Type | Execution Level | Verification Assertion & Pass Criteria |
| :--- | :--- | :--- | :--- | :--- |
| `TC-6.1.0-01` | `AC-US-6.1.0-01` | Functional | Integration | Assert `create()` returns Custom account with requested capabilities in `pending` state. |
| `TC-6.1.0-02` | `AC-US-6.1.0-02` | Functional | Integration | Assert `AccountLink` URL contains `"connect.stripe.com"`. |
| `TC-6.1.0-03` | `AC-US-6.1.0-03` | Functional | Contract | Assert `AccountSession` returns `client_secret` matching `"account_session_secret_..."`. |
| `TC-6.1.0-LAW` | `AC-US-6.1.0-LAW` | Compliance | Contract | Assert Person entities enforce beneficial owner percentage and identification verification fields. |

---

## 2. Behavior-Centric Architecture & Folder Blueprint

### 2.1 Colocated Behaviour Directory Tree

```
features/connect/modules/accounts/behaviours/account-onboarding/
├── services/        # AccountsResource, AccountLinksResource, AccountSessionsResource, PersonsResource
├── logic/           # Capability validator, KYC requirements evaluator, session token manager
├── types/           # Account, AccountLink, AccountSession, Person, CapabilityStatus
└── constants/       # ACCOUNT_TYPES ("standard", "express", "custom"), CAPABILITIES
```

### 2.2 Scoping & Promotion Blueprint
- **Tier 1 (Behaviour-Local)**: Capability formatting logic and onboarding URL builders.
- **Tier 2 (Module-Shared)**: `Account` entity shared across transfer and payout services.
- **Tier 3 (Feature-Shared)**: Re-exported for `stripe.accounts()`, `stripe.account_links()`, `stripe.account_sessions()`.
- **Tier 4 (Global-Shared)**: Re-exported via `some_stripe::resources::accounts::*`.

---

## 3. Technical Implementation Plan

### 3.1 Domain Invariants & Storage Schema
- **Multi-Tenancy Invariant**: Operations on connected accounts must pass `Stripe-Account: <acct_id>` via `RequestOptions` or target account ID parameters.
- **URL Ephemerality**: `AccountLink` URLs expire after a short TTL and can only be accessed once.
- **Rust Memory Law**: Strict borrowing of parameters (`&CreateAccountParams`); zero deep clones.

### 3.2 Activity & Interaction Diagram

```mermaid
sequenceDiagram
    autonumber
    actor Seller as Merchant Seller
    actor Platform as Platform Backend
    participant Accounts as AccountsResource
    participant Links as AccountLinksResource
    participant StripeAPI as Stripe REST API

    Platform->>Accounts: create(&CreateAccountParams { type: "express", country: "US" })
    Accounts->>StripeAPI: POST /v1/accounts
    StripeAPI-->>Accounts: 200 OK (id: "acct_seller_123")
    Accounts-->>Platform: Return Account
    Platform->>Links: create(&CreateAccountLinkParams { account: "acct_seller_123" })
    Links->>StripeAPI: POST /v1/account_links
    StripeAPI-->>Links: 200 OK (url: "https://connect.stripe.com/...")
    Links-->>Platform: Return AccountLink
    Platform-->>Seller: Redirect to Stripe Hosted Onboarding
```

---

## 4. Granular Implementation Tasks

### 4.1 The 10-PR Vertical Slicing Implementation Plan

| PR # | Pillar | Architectural Layer | Primary Scope & Files | Deliverables & Acceptance | Accountable Role / Agent | Assigned Skills |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **PR 1** | **Pillar 1** | **Connect Account Domain Models** | `crates/some-stripe-connect/src/types/account.rs` | Strongly typed `Account`, `Capabilities`, `Requirements`. | `domain-codegen-specialist` | `connect-recommend` |
| **PR 2** | **Pillar 2** | **Account Links & Sessions Models** | `crates/some-stripe-connect/src/types/links.rs` | Strongly typed `AccountLink`, `AccountSession`, components. | `domain-codegen-specialist` | `stripe-docs` |
| **PR 3** | **Pillar 3** | **KYC Person Entities** | `crates/some-stripe-connect/src/types/person.rs` | Strongly typed `Person`, `Relationship`, verification status. | `domain-codegen-specialist` | `connect-required-verification-information` |
| **PR 4** | **Pillar 4** | **Accounts Resource Implementation** | `crates/some-stripe-connect/src/resources/accounts.rs` | `create`, `retrieve`, `update`, `del`, `reject`. | `domain-codegen-specialist` | `stripe-docs` |
| **PR 5** | **Pillar 5** | **Account Links Resource** | `crates/some-stripe-connect/src/resources/links.rs` | `create` onboarding and update links. | `domain-codegen-specialist` | `stripe-docs` |
| **PR 6** | **Pillar 6** | **Account Sessions Resource** | `crates/some-stripe-connect/src/resources/sessions.rs` | `create` embedded UI session tokens. | `domain-codegen-specialist` | `stripe-docs` |
| **PR 7** | **Pillar 7** | **Persons Sub-Resource** | `crates/some-stripe-connect/src/resources/persons.rs` | `create`, `retrieve`, `update`, `list` persons on account. | `domain-codegen-specialist` | `connect-required-verification-information` |
| **PR 8** | **Pillar 8** | **Facade Mounting** | `crates/some-stripe-connect/src/facade.rs` | Mount `stripe.accounts()`, `stripe.account_links()` on `StripeClient`. | `microkernel-architect` | `stripe-best-practices` |
| **PR 9** | **Pillar 9** | **Stripe-Mock Wire Verification** | `tests/connect_accounts_mock_test.rs` | End-to-end account provisioning and link creation against `stripe-mock`. | `testing-api-tester` | `stripe-docs` |
| **PR 10** | **Pillar 10** | **Connect Onboarding Docs** | `README.md`, `docs/connect.md` | Guide on Connect account types and embedded component integration. | `engineering-technical-writer` | `connect-recommend` |

### 4.2 Implementation Phase Breakdown

#### Phase 1: Models & KYC Schemas (PR 1–3)
- [ ] **Task 1.1 (Types)**: Define `Account`, `AccountType`, `AccountCapabilities`, and `Person` in `types/`.  
  *Agent*: `domain-codegen-specialist` | *Skill*: `connect-recommend`
- [ ] **Task 1.2 (Constants)**: Pin `/v1/accounts`, `/v1/account_links`, `/v1/account_sessions` in `constants/`.  
  *Agent*: `domain-codegen-specialist` | *Skill*: `stripe-docs`
- [ ] **Task 1.3 (Logic)**: Implement `CreateAccountParams` and `CreateAccountLinkParams` builders.  
  *Agent*: `domain-codegen-specialist` | *Skill*: `connect-required-verification-information`
- [ ] **Task 1.4 (Unit Tests)**: Verify JSON deserialization of Connect account fixture payloads.  
  *Agent*: `testing-api-tester` | *Skill*: `stripe-docs`

#### Phase 2: Resource Methods & Wire Execution (PR 4–8)
- [ ] **Task 2.1 (Accounts)**: Implement `AccountsResource` with create, retrieve, update, and reject.  
  *Agent*: `domain-codegen-specialist` | *Skill*: `stripe-docs`
- [ ] **Task 2.2 (Links & Sessions)**: Implement `AccountLinksResource` and `AccountSessionsResource`.  
  *Agent*: `domain-codegen-specialist` | *Skill*: `stripe-docs`
- [ ] **Task 2.3 (Persons)**: Implement `PersonsResource` for beneficial owner management.  
  *Agent*: `domain-codegen-specialist` | *Skill*: `connect-required-verification-information`
- [ ] **Task 2.4 (Facade)**: Expose Connect accessors on `StripeClient`.  
  *Agent*: `microkernel-architect` | *Skill*: `stripe-docs`

#### Phase 3: Zero-Clone Borrowing & Multi-Tenant Audit (PR 8–9)
- [ ] **Task 3.1 (Borrow Check)**: Verify zero `.clone()` calls during account response handling.  
  *Agent*: `engineering-rust-refactoring-specialist` | *Skill*: `stripe-best-practices`
- [ ] **Task 3.2 (Tenant Isolation)**: Audit `Stripe-Account` header injection to guarantee zero cross-tenant leaks.  
  *Agent*: `security-appsec-engineer` | *Skill*: `security-audit`

#### Phase 4: Mock Verification & Release Sign-Off (PR 9–10)
- [ ] **Task 4.1 (Mock Suite)**: Run full account creation, person KYC, and link generation on `stripe-mock`.  
  *Agent*: `testing-api-tester` | *Skill*: `stripe-docs`
- [ ] **Task 4.2 (G-Review)**: Peer review for Connect compliance and multi-party security.  
  *Agent*: `engineering-code-reviewer` | *Skill*: `connect-recommend`
- [ ] **Task 4.3 (Docs)**: Publish developer documentation for Connect marketplace onboarding.  
  *Agent*: `engineering-technical-writer` | *Skill*: `connect-recommend`

---

## 5. Security Gateway (Powered by security-audit)

### 5.1 Trust Boundaries & Security Actors
- **Trust Boundaries**:
  - `Platform Core`: Marketplace orchestrator.
  - `Connected Account Scope`: Seller data partitioned by `acct_...`.
  - `KYC Verification Boundary`: Government IDs and SSN/EIN submitted to Stripe.
- **Authorized Principals**:
  - Platform Admin: Authorized to onboard and reject connected accounts.

### 5.2 Threat Modeling & Abuse Matrix (STRIDE / ASVS)

| Threat Vector | Severity | Vulnerability Mechanism | Mitigating Control & Security Invariant |
| :--- | :--- | :--- | :--- |
| **Cross-Account Data Leakage** | Critical | Reusing `Stripe-Account` context across distinct sellers | `RequestOptions` headers are scoped strictly per request; zero global state. |
| **KYC PII Exposure** | Critical | Plaintext SSN or passport numbers stored in application logs | `Person` verification structs enforce `Debug` redaction on sensitive identification fields. |
| **Phishing via Forged Link** | High | Attacker generating fake onboarding URLs | All onboarding links generated authoritatively by Stripe API with verified TLS certificates. |

### 5.3 Cryptographic & Data Protection Controls
- **TLS 1.3 Transport**: Encrypted channel for all KYC and beneficial ownership data.
- **Field-Level Redaction**: SSN, passport, and DOB fields redacted from log streams.

### 5.4 Automated Security Verification Checklist
- [ ] **PII Redaction Test**: Verified that `format!("{person:?}")` redacts SSN and tax ID fields.
- [ ] **Cross-Account Isolation**: Verified that requests omitting `Stripe-Account` operate exclusively in platform scope.

### 5.5 Security Gateway (G2) Sign-Off Checklist
- [ ] FinCEN CDD beneficial ownership data collection verified.
- [ ] Zero KYC PII leaks in trace logs.
- [ ] Code review sign-off obtained; zero findings.

---

## 6. Validation Gates & Acceptance Sign-Off

**Status: Ready for Implementation.**

### G1 — Test coverage
- [ ] **US-6.1.0-G1:** >=90% line and branch coverage across `accounts.rs` and `persons.rs`.

### G2 — Security audit
- [ ] **US-6.1.0-G2:** Multi-tenant account isolation and PII protection verified under security-audit.

### G3 — Architecture and maintainability
- [ ] **US-6.1.0-G3:** Clean feature isolation under `feature = "connect"`.

### G-Review — Code review
- [ ] **US-6.1.0-G-Review:** Senior review approved by `engineering-code-reviewer`.

### G-Borrow — Rust zero-clone borrowing validation
- [ ] **US-6.1.0-G-Borrow:** Parameters borrowed strictly by reference; zero deep clones.

### G-Law — Law validation
- [ ] **US-6.1.0-G-Law:** FinCEN CDD Rule and EU AMLD5/DAC7 compliance certified.

### G4 — Integrated acceptance
- [ ] **US-6.1.0-G4:** End-to-end account provisioning and onboarding validated against `stripe-mock`.
