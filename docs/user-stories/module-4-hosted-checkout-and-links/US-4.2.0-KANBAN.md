# US-4.2.0: Payment Links & Price Transparency — Story Kanban Board

This Kanban board tracks the micro-units of **User Story 4.2.0 (Payment Links & Price Transparency)** with an explicit **`🤖 Subagent Creating`** state to track unit implementer generation.

> [!IMPORTANT]
> **Story Leader Invariant**: Upon invocation, the Story Leader moves into `spawning[🤖 Subagent Creating]` and immediately spawns all 5 Unit Implementers. If this stage takes >3 minutes, the Supervisor Watchdog intervenes.
>
> **Federated Gatekeeper Council & Skills Invariant**: Every PR must undergo independent verification by specialized gatekeeper subagents from `.agents/agents/` armed with designated skills from `.agents/skills/`. **Zero self-approval**.

---

## 1. User Story 4.2.0 Mermaid Kanban

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
  review[🔍 PR Open / Gate Review]
  done[✅ Merged into Story / Module]
    s42_pr[US-4.2.0 Story Promotion PR #65 Merged into Module]@{ ticket: '65', assigned: 'agent-a', priority: 'Very High' }
    s42_spawn_units[Spawn Units 1..5 Implementers across Worktrees]@{ ticket: 'feat/us-4.2.0-payment-links', assigned: 'agent-a', priority: 'Very High' }
    u421[Unit 1: PaymentLink Domain Entity & State Machine]@{ ticket: '54', assigned: 'agent-a', priority: 'High' }
    u422[Unit 2: CreatePaymentLinkParams & Multi-Currency]@{ ticket: '56', assigned: 'agent-b', priority: 'High' }
    u423[Unit 3: PaymentLinksResource CRUD & Line Items]@{ ticket: '58', assigned: 'agent-c', priority: 'High' }
    u424[Unit 4: EU Omnibus 2019/2161 Price Wire Mock Suite]@{ ticket: '61', assigned: 'agent-a', priority: 'High' }
    u425[Unit 5: Gate G-Borrow Audit, Doctests & Reconciliation Docs]@{ ticket: '62', assigned: 'agent-b', priority: 'High' }
```

---

## 2. Micro-Unit Implementer, Gatekeeper Subagents & Skills Matrix

| Unit ID | Unit Scope | Implementer & Profile | Mandatory Implementer Skill | Assigned Gatekeeper Subagents | Mandatory Gatekeeper Skills | Status | PR |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: |
| **S42-SPAWN** | Spawn 5 Unit Implementers | `US-4.2.0 Story Leader` (`agent-a`) | `stripe-best-practices` | `Main Supervisor Watchdog` | `antigravity-guide` | ✅ Done | Spawned Implementers |
| **U-4.2.0-1** | `PaymentLink` Domain Entity & State Machine | `Entity Specialist` (`agent-a`) | `stripe-docs` | `Rust Refactoring Specialist`<br>`Code Reviewer` | `upgrade-stripe`<br>`security-audit` | ✅ Merged | [#54](https://github.com/some-of-cs/some-stripe/pull/54) (`4f3bcda`) |
| **U-4.2.0-2** | `PaymentLinkParams` & Multi-Currency Schemas | `Schema Specialist` (`agent-a`) | `stripe-docs`<br>`stripe-best-practices` | `microkernel-architect`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices` | ✅ Merged | [#56](https://github.com/some-of-cs/some-stripe/pull/56) (`e865b0e`) |
| **U-4.2.0-3** | `PaymentLinksResource` CRUD & Line Items | `CRUD Implementer` (`agent-a`) | `stripe-docs`<br>`stripe-best-practices` | `API Tester`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices` | ✅ Merged | [#58](https://github.com/some-of-cs/some-stripe/pull/58) (`b50debd`) |
| **U-4.2.0-4** | EU Omnibus 2019/2161 Price Wire Mock Suite | `AppSec Engineer` (`agent-a`) | `security-audit`<br>`stripe-best-practices` | `Application Security Engineer`<br>`Code Reviewer` | `security-audit`<br>`stripe-best-practices` | ✅ Merged | [#61](https://github.com/some-of-cs/some-stripe/pull/61) (`59a6ffa`) |
| **U-4.2.0-5** | Borrow Audit, Doctests & Reconciliation Guide | `QA & Docs Specialist` (`agent-a`) | `stripe-docs` | `Rust Refactoring Specialist`<br>`Technical Writer`<br>`Code Reviewer` | `upgrade-stripe`<br>`stripe-docs`<br>`security-audit` | ✅ Merged | [#62](https://github.com/some-of-cs/some-stripe/pull/62) (`e94ab53`) |
| **S42-PROMO** | Story Integration & Promotion PR | `US-4.2.0 Story Leader` (`agent-a`) | `stripe-best-practices` | `API Tester`<br>`microkernel-architect`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices`<br>`security-audit` | ✅ Merged | [#65](https://github.com/some-of-cs/some-stripe/pull/65) (`31676af`) |
