# US-4.2.0: Payment Links, Multi-Currency Pricing & Price Transparency

> **Spec Kit Specification**: [Specification Lifecycle Active (.specify/)](../../../.specify/memory/constitution.md)  
> **Behavior Blueprint**: `features/checkout/modules/links/behaviours/payment-links/`  
> **Slicing Standard**: [10-PR Vertical Slicing Framework](../../user-stories/README.md#the-10-pr-vertical-slicing-framework-rust-sdk-adaptation)  
> **Security Gateway**: `security-audit` Level 2 Enforced  
> **Quality Gates**: `(G0–G4 + G-Review + G-Borrow + G-Law)`  

---

## 1. Spec Kit Specification

### 1.1 User Story Narrative
**As an** e-commerce merchant and API integrator,  
**I want to** create, retrieve, update, and deactivate shareable Stripe Payment Links with multi-currency presentment and line item configurations,  
**So that** I can sell products and services directly across social platforms, invoices, and email links with EU Omnibus Directive 2019/2161 price transparency and automated multi-currency conversion.

#### 1.1.1 Preconditions
- The `checkout` feature flag is enabled in `Cargo.toml`.
- Valid line items with prices or custom price data are provided.
- Currency presentment follows ISO 4217 zero-float integer arithmetic.

#### 1.1.2 Postconditions
- A `PaymentLink` entity is returned with an active shareable URL (`link.url`).
- Inactive links reject checkout attempts (`link.active == false`).
- Price displays comply with EU Omnibus Directive 2019/2161 prior price reduction baseline disclosures.

### 1.2 Acceptance Criteria (Gherkin Scenarios)

```gherkin
Feature: Stripe Payment Links Management

  Scenario: [AC-US-4.2.0-01] Create a Shareable Payment Link
    Given a valid CreatePaymentLinkParams with line items and currency "usd"
    When client.checkout().payment_links().create(&params) is executed
    Then a PaymentLink is returned with active true
    And link.url is a valid Stripe Payment Link URL ("https://buy.stripe.com/...").

  Scenario: [AC-US-4.2.0-02] Deactivate an existing Payment Link
    Given an active Payment Link "plink_test_123"
    When client.checkout().payment_links().update("plink_test_123", &UpdatePaymentLinkParams { active: Some(false), .. }) is executed
    Then the link status updates to active false
    And visits to the link URL show a deactivated notice.

  Scenario: [AC-US-4.2.0-03] Multi-currency presentment pricing
    Given a payment link configured for multi-currency conversion (USD base with EUR, GBP presentment)
    When a customer checks out in EUR
    Then the exact minor currency integer amount is billed without IEEE 754 float precision loss.

  Scenario: [AC-US-4.2.0-LAW] EU Omnibus Directive 2019/2161 transparency
    Given any discounted price display on a Payment Link
    Then the previous lowest baseline price applied within the past 30 days is recorded and transparently presented to the consumer.
```

### 1.3 Test Cases & Traceability Matrix

| Test Case ID | Target Criterion | Test Type | Execution Level | Verification Assertion & Pass Criteria |
| :--- | :--- | :--- | :--- | :--- |
| `TC-4.2.0-01` | `AC-US-4.2.0-01` | Functional | Integration | Assert `create()` returns valid shareable URL and `active: true`. |
| `TC-4.2.0-02` | `AC-US-4.2.0-02` | Lifecycle | Integration | Call `update(active: false)` -> assert returned link has `active: false`. |
| `TC-4.2.0-03` | `AC-US-4.2.0-03` | Financial | Unit | Verify ISO 4217 integer arithmetic holds across multi-currency line items. |
| `TC-4.2.0-LAW` | `AC-US-4.2.0-LAW` | Statutory | Contract | Verify EU Omnibus Directive 2019/2161 baseline price integrity. |

---

## 2. Micro-Unit Architecture (Units 1..5)
1. **Unit 1 (`unit-1-payment-link-entities`)**: `PaymentLink`, `PaymentLinkStatus`, line item references.
2. **Unit 2 (`unit-2-payment-link-params`)**: `CreatePaymentLinkParams`, `UpdatePaymentLinkParams`, multi-currency schemas.
3. **Unit 3 (`unit-3-payment-links-resource-crud`)**: `PaymentLinksResource` (`create`, `retrieve`, `update`, `list_line_items`).
4. **Unit 4 (`unit-4-omnibus-price-wire-tests`)**: EU Omnibus Directive 2019/2161 test suite and `stripe-mock` wire tests.
5. **Unit 5 (`unit-5-borrow-audit-and-reconciliation`)**: Zero-clone borrowing audit, rustdoc doctests, and documentation.
