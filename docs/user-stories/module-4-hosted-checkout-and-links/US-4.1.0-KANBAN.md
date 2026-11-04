# US-4.1.0: Checkout Sessions & Line Items — Story Kanban Board

This Kanban board tracks the micro-units of **User Story 4.1.0 (Checkout Sessions & Line Items)** with an explicit **`🤖 Subagent Creating`** state to track unit implementer generation.

> [!IMPORTANT]
> **Story Leader Invariant**: Upon invocation, the Story Leader moves into `spawning[🤖 Subagent Creating]` and immediately spawns all 5 Unit Implementers. If this stage takes >3 minutes, the Supervisor Watchdog intervenes.
>
> **Federated Gatekeeper Council & Skills Invariant**: Every PR must undergo independent verification by specialized gatekeeper subagents from `.agents/agents/` armed with designated skills from `.agents/skills/`. **Zero self-approval**.

---

## 1. User Story 4.1.0 Mermaid Kanban

```mermaid
---
config:
  kanban:
    ticketBaseUrl: 'https://github.com/some-of-cs/some-stripe/pull/#TICKET#'
---
kanban
  todo[📋 Backlog / Ready]
    s41_pr[US-4.1.0 Story Integration Review & PR to Module]@{ ticket: 'feat/us-4.1.0-checkout-sessions', assigned: 'agent-a', priority: 'Very High' }
  spawning[🤖 Subagent Creating]
  in_progress[🔄 Units In Progress]
    u414[Unit 4: Async Payment Fulfillment Guard & Open Redirect Protection]@{ ticket: 'unit-4', assigned: 'agent-c', priority: 'High' }
    u415[Unit 5: Gate G-Borrow Audit, Doctests & Mock Suite]@{ ticket: 'unit-5', assigned: 'agent-a', priority: 'High' }
  review[🔍 PR Open / Gate Review]
  done[✅ Merged into Story]
    s41_spawn_units[Spawn Units 1..5 Implementers across Worktrees]@{ ticket: 'feat/us-4.1.0-checkout-sessions', assigned: 'agent-a', priority: 'Very High' }
    u411[Unit 1: CheckoutSession & LineItem Entities]@{ ticket: '53', assigned: 'agent-c', priority: 'High' }
    u412[Unit 2: CreateCheckoutSessionParams Builder & Bracket Serde]@{ ticket: '55', assigned: 'agent-a', priority: 'High' }
    u413[Unit 3: SessionsResource CRUD & Expire Endpoint]@{ ticket: '57', assigned: 'agent-b', priority: 'High' }
```

---

## 2. Micro-Unit Implementer, Gatekeeper Subagents & Skills Matrix

| Unit ID | Unit Scope | Implementer & Profile | Mandatory Implementer Skill | Assigned Gatekeeper Subagents | Mandatory Gatekeeper Skills | Status | PR |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: |
| **S41-SPAWN** | Spawn 5 Unit Implementers | `US-4.1.0 Story Leader` (`agent-b`) | `stripe-best-practices` | `Main Supervisor Watchdog` | `antigravity-guide` | ✅ Complete | Ready |
| **U-4.1.0-1** | `CheckoutSession` & `LineItem` Entities | `Entity Specialist` (`agent-c`) | `stripe-docs` | `Rust Refactoring Specialist`<br>`Code Reviewer` | `upgrade-stripe`<br>`security-audit` | ✅ Merged | [#53](https://github.com/some-of-cs/some-stripe/pull/53) |
| **U-4.1.0-2** | `CreateCheckoutSessionParams` & Bracket Serde | `Schema Specialist` (`agent-a`) | `stripe-docs`<br>`stripe-best-practices` | `microkernel-architect`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices` | ✅ Merged | [#55](https://github.com/some-of-cs/some-stripe/pull/55) |
| **U-4.1.0-3** | `SessionsResource` CRUD & Expire | `CRUD Implementer` (`agent-b`) | `stripe-docs`<br>`stripe-best-practices` | `API Tester`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices` | ✅ Merged | [#57](https://github.com/some-of-cs/some-stripe/pull/57) |
| **U-4.1.0-4** | Async Fulfillment & Redirect Guard | `AppSec Engineer` (`agent-c`) | `security-audit`<br>`stripe-best-practices` | `Application Security Engineer`<br>`Code Reviewer` | `security-audit`<br>`stripe-best-practices` | 🔄 In Progress | Pending |
| **U-4.1.0-5** | Borrow Audit, Doctests & Wire Mocks | `QA & Docs Specialist` (`agent-a`) | `stripe-docs` | `Rust Refactoring Specialist`<br>`Technical Writer`<br>`Code Reviewer` | `upgrade-stripe`<br>`stripe-docs`<br>`security-audit` | 🔄 In Progress | Pending |
| **S41-PROMO** | Story Integration & Promotion PR | `US-4.1.0 Story Leader` (`agent-b`) | `stripe-best-practices` | `API Tester`<br>`microkernel-architect`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices`<br>`security-audit` | 📋 Ready | Pending |
