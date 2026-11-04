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
    u411[Unit 1: CheckoutSession & LineItem Entities]@{ ticket: 'unit-1', assigned: 'agent-a', priority: 'High' }
    u412[Unit 2: CreateCheckoutSessionParams Builder & Bracket Serde]@{ ticket: 'unit-2', assigned: 'agent-b', priority: 'High' }
    u413[Unit 3: SessionsResource CRUD & Expire Endpoint]@{ ticket: 'unit-3', assigned: 'agent-c', priority: 'High' }
    u414[Unit 4: Async Payment Fulfillment Guard & Open Redirect Protection]@{ ticket: 'unit-4', assigned: 'agent-a', priority: 'High' }
    u415[Unit 5: Gate G-Borrow Audit, Doctests & Mock Suite]@{ ticket: 'unit-5', assigned: 'agent-b', priority: 'High' }
    s41_pr[US-4.1.0 Story Integration Review & PR to Module]@{ ticket: 'feat/us-4.1.0-checkout-sessions', assigned: 'agent-b', priority: 'Very High' }
  spawning[🤖 Subagent Creating]
    s41_spawn_units[Spawn Units 1..5 Implementers across Worktrees]@{ ticket: 'feat/us-4.1.0-checkout-sessions', assigned: 'agent-b', priority: 'Very High' }
  in_progress[🔄 Units In Progress]
  review[🔍 PR Open / Gate Review]
  done[✅ Merged into Story]
```

---

## 2. Micro-Unit Implementer, Gatekeeper Subagents & Skills Matrix

| Unit ID | Unit Scope | Implementer & Profile | Mandatory Implementer Skill | Assigned Gatekeeper Subagents | Mandatory Gatekeeper Skills | Status | PR |
| :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: |
| **S41-SPAWN** | Spawn 5 Unit Implementers | `US-4.1.0 Story Leader` (`agent-b`) | `stripe-best-practices` | `Main Supervisor Watchdog` | `antigravity-guide` | 🤖 Spawning | Ready |
| **U-4.1.0-1** | `CheckoutSession` & `LineItem` Entities | `Entity Specialist` (`agent-a`) | `stripe-docs` | `Rust Refactoring Specialist`<br>`Code Reviewer` | `upgrade-stripe`<br>`security-audit` | 📋 Ready | Pending |
| **U-4.1.0-2** | `CreateCheckoutSessionParams` & Bracket Serde | `Schema Specialist` (`agent-b`) | `stripe-docs`<br>`stripe-best-practices` | `microkernel-architect`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices` | 📋 Ready | Pending |
| **U-4.1.0-3** | `SessionsResource` CRUD & Expire | `CRUD Implementer` (`agent-c`) | `stripe-docs`<br>`stripe-best-practices` | `API Tester`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices` | 📋 Ready | Pending |
| **U-4.1.0-4** | Async Fulfillment & Redirect Guard | `AppSec Engineer` (`agent-a`) | `security-audit`<br>`stripe-best-practices` | `Application Security Engineer`<br>`Code Reviewer` | `security-audit`<br>`stripe-best-practices` | 📋 Ready | Pending |
| **U-4.1.0-5** | Borrow Audit, Doctests & Wire Mocks | `QA & Docs Specialist` (`agent-b`) | `stripe-docs` | `Rust Refactoring Specialist`<br>`Technical Writer`<br>`Code Reviewer` | `upgrade-stripe`<br>`stripe-docs`<br>`security-audit` | 📋 Ready | Pending |
| **S41-PROMO** | Story Integration & Promotion PR | `US-4.1.0 Story Leader` (`agent-b`) | `stripe-best-practices` | `API Tester`<br>`microkernel-architect`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices`<br>`security-audit` | 📋 Ready | Pending |
