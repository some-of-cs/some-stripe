# US-5.1.0: Customer Vaulting & Subscriptions — User Story Kanban

Tracks micro-units 1 through 5 for `US-5.1.0` (Customer Vaulting, Recurring Subscriptions & Multi-Phase Schedules).

> [!IMPORTANT]
> **Mandatory Subagent Spawning State**: The Story Leader MUST transition through the **`spawning[🤖 Subagent Creating]`** column. If idling for >3 minutes, the Main Supervisor Watchdog intervenes.

---

## 1. User Story 5.1.0 Mermaid Kanban

```mermaid
---
config:
  kanban:
    ticketBaseUrl: 'https://github.com/some-of-cs/some-stripe/pull/#TICKET#'
---
kanban
  todo[📋 Backlog / Ready]
  spawning[🤖 Subagent Creating]
  in_progress[🔄 Units In Progress]
    u514[Unit 4: Prorations, 14-Day Trial & GDPR Article 17 Tests]@{ ticket: 'main', assigned: 'agent-b', priority: 'High' }
    u515[Unit 5: Gate G-Borrow Audit, Doctests & Acceptance Gates]@{ ticket: 'main', assigned: 'agent-c', priority: 'High' }
  review[🔍 PR Open / Gate Review]
    u513[Unit 3: Customers & Subscriptions Resource CRUD & Lifecycle]@{ ticket: '77', assigned: 'agent-c', priority: 'High' }
  done[✅ Merged into Story Branch]
    u511[Unit 1: Customer, Subscription, Item & Schedule Entities]@{ ticket: '71', assigned: 'agent-b', priority: 'High' }
    u512[Unit 2: CreateSubscriptionParams & Multi-Item Form Serde]@{ ticket: '73', assigned: 'agent-c', priority: 'High' }
    s51_pr[US-5.1.0 Story Promotion PR into Module Branch]@{ ticket: 'main', assigned: 'agent-a', priority: 'Very High' }
```

---

## 2. Micro-Unit Task & Gatekeeper Matrix

| Unit ID | Scope & Files | Implementer & Profile | Mandatory Skill | Gatekeeper Council Subagents | Gatekeeper Skills | Branch | Target PR Title |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Unit 1** | `Customer`, `Subscription`, `SubscriptionItem`, `SubscriptionSchedule` domain entities in `some-stripe-types` | `domain-codegen-specialist` (`agent-b`) | `stripe-docs` | `Code Reviewer`<br>`AppSec Engineer` | `stripe-best-practices`<br>`security-audit` | `feat/us-5.1.0/unit-1-customer-subscription-entities` | `feat(us-5.1.0/unit-1): customer, subscription, and schedule domain entities` (✅ Merged, PR #71) |
| **Unit 2** | Core Subscription entity lifecycle state machine, `PaymentSettings`, and `Create/Update/Cancel/Resume/ListSubscriptionParams` | `microkernel-architect` (`agent-b`) | `stripe-docs` | `Code Reviewer`<br>`Rust Refactoring` | `stripe-best-practices`<br>`upgrade-stripe` | `feat/us-5.1.0/unit-2-multi-item-subscription-params-serde` | `feat(us-5.1.0/unit-2): core recurring subscriptions lifecycle and state machine` (✅ Merged, PR #73) |
| **Unit 3** | `CustomersResource`, `SubscriptionsResource`, `SubscriptionItemsResource` CRUD and facade mounting | `domain-codegen-specialist` (`agent-c`) | `stripe-docs` | `Code Reviewer`<br>`AppSec Engineer` | `stripe-best-practices`<br>`security-audit` | `feat/us-5.1.0/unit-3-customers-subscriptions-resources` | `feat(us-5.1.0/unit-3): multi-item subscriptions and indexed bracket serde` (🔍 PR #77 Open) |
| **Unit 4** | `stripe-mock` wire tests: 14-day trials, proration calculation, pause collection, GDPR Article 17 sanitization | `API Tester` (`agent-b`) | `stripe-docs` | `Code Reviewer`<br>`AppSec Engineer` | `stripe-best-practices`<br>`security-audit` | `feat/us-5.1.0/unit-4-proration-trials-mock-test` | `feat(us-5.1.0/unit-4): subscription wire mock tests and gdpr compliance verification` |
| **Unit 5** | Gate G-Borrow audit, 100% doctests passing, zero-warning clippy/fmt hygiene | `Rust Refactoring Specialist` (`agent-c`) | `upgrade-stripe` | `Code Reviewer`<br>`Technical Writer` | `stripe-best-practices`<br>`stripe-docs` | `feat/us-5.1.0/unit-5-borrow-audit-doctests-gates` | `feat(us-5.1.0/unit-5): zero-clone borrowing audit and documentation gates` |
