# Module 4: Hosted Checkout & Links — Live Multi-Agent Kanban Board

This Kanban board tracks the end-to-end execution of **Module 4 (Hosted Checkout & Links)** across all tiers and micro-units.

> [!IMPORTANT]
> **Mandatory Agent Rule**: Every subagent assigned to a task MUST update this file upon:
> 1. Starting execution $\to$ move its task from `todo` to `in_progress`.
> 2. Pushing branch and opening PR $\to$ move its task from `in_progress` to `review` and set the `ticket` metadata to the GitHub PR number (e.g. `ticket: '50'`).
> 3. Merging the PR $\to$ move its task to `done`.

---

## 1. Native Mermaid Kanban Board

```mermaid
---
config:
  kanban:
    ticketBaseUrl: 'https://github.com/some-of-cs/some-stripe/pull/#TICKET#'
---
kanban
  todo[📋 Backlog / Ready]
    m4[Module 4: Hosted Checkout & Links Final Promotion]@{ ticket: 'feat/module-4-checkout-and-links', assigned: 'agent-a', priority: 'Very High' }
    s41[US-4.1.0: Checkout Sessions Story Promotion]@{ ticket: 'feat/us-4.1.0-checkout-sessions', assigned: 'agent-b', priority: 'High' }
    u411[Unit 4.1.0-1: CheckoutSession & LineItem Entities]@{ ticket: 'unit-1', assigned: 'agent-a', priority: 'High' }
    u412[Unit 4.1.0-2: CreateCheckoutSessionParams Builder]@{ ticket: 'unit-2', assigned: 'agent-b', priority: 'High' }
    u413[Unit 4.1.0-3: SessionsResource CRUD & Expire]@{ ticket: 'unit-3', assigned: 'agent-c', priority: 'High' }
    u414[Unit 4.1.0-4: Async Fulfillment & Redirect Guard]@{ ticket: 'unit-4', assigned: 'agent-a', priority: 'High' }
    u415[Unit 4.1.0-5: Borrow Audit, Doctests & Mocks]@{ ticket: 'unit-5', assigned: 'agent-b', priority: 'High' }
    s42[US-4.2.0: Payment Links Story Promotion]@{ ticket: 'feat/us-4.2.0-payment-links', assigned: 'agent-c', priority: 'High' }
    u421[Unit 4.2.0-1: PaymentLink Domain Entities]@{ ticket: 'unit-1', assigned: 'agent-c', priority: 'High' }
    u422[Unit 4.2.0-2: PaymentLink Params & Multi-Currency]@{ ticket: 'unit-2', assigned: 'agent-a', priority: 'High' }
    u423[Unit 4.2.0-3: PaymentLinksResource CRUD]@{ ticket: 'unit-3', assigned: 'agent-b', priority: 'High' }
    u424[Unit 4.2.0-4: EU Omnibus 2019/2161 Wire Mocks]@{ ticket: 'unit-4', assigned: 'agent-c', priority: 'High' }
    u425[Unit 4.2.0-5: Borrow Audit & Reconciliation Docs]@{ ticket: 'unit-5', assigned: 'agent-a', priority: 'High' }
  in_progress[🔄 In Progress]
  review[🔍 In Review / PR Open]
  done[✅ Done / Merged]
```

---

## 2. Live Task Allocation & Status Matrix

| Task ID | Task Scope | Assigned Role | Profile | Worktree Directory | Branch | Status | PR |
| :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: |
| **M4-PROMO** | Module 4 Final Promotion to `main` | `Module 4 Lead Architect` | `agent-a` (`account1`) | `.worktrees/feat-module-4-checkout-and-links` | `feat/module-4-checkout-and-links` | 📋 Ready | Pending |
| **US-4.1.0** | Hosted Checkout Sessions Story | `US-4.1.0 Story Leader` | `agent-b` (`account2`) | `.worktrees/feat-us-410-checkout-sessions` | `feat/us-4.1.0-checkout-sessions` | 📋 Ready | Pending |
| **U-4.1.0-1** | `CheckoutSession` & `LineItem` Entities | `Entity Specialist` | `agent-a` (`account1`) | `.worktrees/feat-us-410-unit-1-session-line-item-entities` | `feat/us-4.1.0/unit-1-session-line-item-entities` | 📋 Ready | Pending |
| **U-4.1.0-2** | `CreateCheckoutSessionParams` & Bracket Serde | `Schema Specialist` | `agent-b` (`account2`) | `.worktrees/feat-us-410-unit-2-params-and-bracket-serialization` | `feat/us-4.1.0/unit-2-params-and-bracket-serialization` | 📋 Ready | Pending |
| **U-4.1.0-3** | `SessionsResource` CRUD & Expire | `CRUD Implementer` | `agent-c` (`account3`) | `.worktrees/feat-us-410-unit-3-sessions-resource-crud` | `feat/us-4.1.0/unit-3-sessions-resource-crud` | 📋 Ready | Pending |
| **U-4.1.0-4** | Async Payment Fulfillment & Open Redirect Guard | `AppSec Engineer` | `agent-a` (`account1`) | `.worktrees/feat-us-410-unit-4-async-fulfillment-wire-tests` | `feat/us-4.1.0/unit-4-async-fulfillment-wire-tests` | 📋 Ready | Pending |
| **U-4.1.0-5** | Gate G-Borrow Audit, Doctests & Mock Suite | `QA & Docs Specialist` | `agent-b` (`account2`) | `.worktrees/feat-us-410-unit-5-borrow-audit-and-doctests` | `feat/us-4.1.0/unit-5-borrow-audit-and-doctests` | 📋 Ready | Pending |
| **US-4.2.0** | Payment Links & Multi-Currency Story | `US-4.2.0 Story Leader` | `agent-c` (`account3`) | `.worktrees/feat-us-420-payment-links` | `feat/us-4.2.0-payment-links` | 📋 Ready | Pending |
| **U-4.2.0-1** | `PaymentLink` Domain Entity & Lifecycle | `Entity Specialist` | `agent-c` (`account3`) | `.worktrees/feat-us-420-unit-1-payment-link-entities` | `feat/us-4.2.0/unit-1-payment-link-entities` | 📋 Ready | Pending |
| **U-4.2.0-2** | `PaymentLinkParams` & Multi-Currency Schemas | `Schema Specialist` | `agent-a` (`account1`) | `.worktrees/feat-us-420-unit-2-payment-link-params` | `feat/us-4.2.0/unit-2-payment-link-params` | 📋 Ready | Pending |
| **U-4.2.0-3** | `PaymentLinksResource` CRUD & Line Items | `CRUD Implementer` | `agent-b` (`account2`) | `.worktrees/feat-us-420-unit-3-payment-links-resource-crud` | `feat/us-4.2.0/unit-3-payment-links-resource-crud` | 📋 Ready | Pending |
| **U-4.2.0-4** | EU Omnibus 2019/2161 Price Wire Mock Suite | `AppSec Engineer` | `agent-c` (`account3`) | `.worktrees/feat-us-420-unit-4-omnibus-price-wire-tests` | `feat/us-4.2.0/unit-4-omnibus-price-wire-tests` | 📋 Ready | Pending |
| **U-4.2.0-5** | Borrow Audit, Doctests & Reconciliation Guide | `QA & Docs Specialist` | `agent-a` (`account1`) | `.worktrees/feat-us-420-unit-5-borrow-audit-and-reconciliation` | `feat/us-4.2.0/unit-5-borrow-audit-and-reconciliation` | 📋 Ready | Pending |

---

## 3. How Agents Transition Tasks in Mermaid Kanban
1. Under the `kanban` block above, indent your task line under the appropriate column:
   - `todo[📋 Backlog / Ready]`
   - `in_progress[🔄 In Progress]`
   - `review[🔍 In Review / PR Open]` (update `@{ ticket: '<PR_NUMBER>' }`)
   - `done[✅ Done / Merged]`
2. Keep `assigned: '<profile>'` and `priority: 'High'`.
3. In the markdown table, update the `Status` and `PR` columns accordingly.
