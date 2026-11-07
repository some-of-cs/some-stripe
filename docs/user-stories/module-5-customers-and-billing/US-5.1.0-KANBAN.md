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
    s51_pr[US-5.1.0 Story Promotion PR into Module Branch]@{ ticket: 'main', assigned: 'agent-a', priority: 'Very High' }
  review[🔍 PR Open / Gate Review]
  done[✅ Merged into Story Branch]
    u511[Unit 1: Customer, Subscription, Item & Schedule Entities (PR #71)]@{ ticket: '71', assigned: 'agent-a', priority: 'High' }
    u512[Unit 2: Core Recurring Subscriptions Lifecycle & State Machine (PR #73)]@{ ticket: '73', assigned: 'agent-b', priority: 'High' }
    u513[Unit 3: Multi-Item Subscriptions & Indexed Bracket Serde (PR #77)]@{ ticket: '77', assigned: 'agent-c', priority: 'High' }
    u514[Unit 4: Subscription Schedules & Multi-Phase Transitions (PR #78)]@{ ticket: '78', assigned: 'agent-a', priority: 'High' }
    u515[Unit 5: In-Flight Plan Prorations, Trials & Pause Collection (PR #74)]@{ ticket: '74', assigned: 'agent-b', priority: 'High' }
```

---

## 2. Micro-Unit Task & Gatekeeper Matrix

| Unit ID | Scope & Files | Implementer & Profile | Mandatory Skill | Gatekeeper Council Subagents | Gatekeeper Skills | Branch | Target PR Title | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Unit 1** | `Customer`, `Subscription`, `SubscriptionItem`, `SubscriptionSchedule` domain entities in `some-stripe-types` | `domain-codegen-specialist` (`agent-a`) | `stripe-docs` | `Code Reviewer`<br>`AppSec Engineer` | `stripe-best-practices`<br>`security-audit` | `feat/us-5.1.0/unit-1-customer-subscription-entities` | `feat(us-5.1.0/unit-1): customer, subscription, and schedule domain entities` | ✅ Merged (PR #71, PR #76) |
| **Unit 2** | `CreateSubscriptionParams`, `UpdateSubscriptionParams`, `PauseCollectionParams`, indexed form serde | `microkernel-architect` (`agent-b`) | `stripe-docs` | `Code Reviewer`<br>`Rust Refactoring` | `stripe-best-practices`<br>`upgrade-stripe` | `feat/us-5.1.0/unit-2-multi-item-subscription-params-serde` | `feat(us-5.1.0/unit-2): core recurring subscriptions lifecycle and state machine` | ✅ Merged (PR #73) |
| **Unit 3** | Multi-item subscriptions, bracket serde, `SubscriptionItemsResource`, usage records | `microkernel-architect` (`agent-c`) | `stripe-docs` | `Code Reviewer`<br>`AppSec Engineer` | `stripe-best-practices`<br>`security-audit` | `feat/us-5.1.0/unit-3-customers-subscriptions-resources` | `feat(us-5.1.0/unit-3): multi-item subscriptions and indexed bracket serde` | ✅ Merged (PR #77) |
| **Unit 4** | Subscription schedules, multi-phase transitions, `SubscriptionSchedulesResource` | `domain-codegen-specialist` (`agent-a`) | `stripe-docs` | `Code Reviewer`<br>`AppSec Engineer` | `stripe-best-practices`<br>`security-audit` | `feat/us-5.1.0/unit-4-proration-trials-mock-test` | `feat(us-5.1.0/unit-4): subscription schedules and multi-phase transitions` | ✅ Merged (PR #78) |
| **Unit 5** | In-flight plan prorations, 14-day trials, pause collection, borrow audit | `Rust Refactoring Specialist` (`agent-b`) | `upgrade-stripe` | `Code Reviewer`<br>`Technical Writer` | `stripe-best-practices`<br>`security-audit` | `feat/us-5.1.0/unit-5-borrow-audit-doctests-gates` | `feat(us-5.1.0/unit-5): in-flight plan prorations, trials, and pause collection` | ✅ Merged (PR #74) |
