# Module 6: Connect & Marketplaces — Executive Module Kanban

Master high-level tracking for **Module 6 (Connect & Marketplaces)** execution across User Stories `US-6.1.0` and `US-6.2.0`.

> [!IMPORTANT]
> **Strict Hierarchical Delegation Invariant**:
> - **The Module Leader generates ONLY User Story Leaders** (`US-6.1.0 Story Leader`, `US-6.2.0 Story Leader`).
> - The Module Leader is STRICTLY FORBIDDEN from creating Unit Implementers directly.
> - **3-Minute Supervision Schedule**: The Module Leader maintains a 3-minute supervision schedule to track story progress.

---

## 1. Module Level Executive Kanban

```mermaid
---
config:
  kanban:
    ticketBaseUrl: 'https://github.com/some-of-cs/some-stripe/pull/#TICKET#'
---
kanban
  todo[📋 Backlog / Ready]
  spawning[🤖 Subagent Creating]
  in_progress[🔄 Stories In Progress]
    m6_s61_promotion[US-6.1.0 Promotion PR into Module Branch]@{ ticket: 'main', assigned: 'agent-b', priority: 'Very High' }
  review[🔍 Module Review & Gate Sign-off]
    m6_gates[Module-Wide Quality Gates Audit: G0-G4, G-Borrow, G2, G-Law]@{ ticket: 'main', assigned: 'agent-c', priority: 'Very High' }
  done[✅ Done / Merged to Module Branch]
    m6_setup[Module 6 Architecture & Implementation Tasks Initialized]@{ ticket: 'main', assigned: 'agent-a', priority: 'High' }
    m6_spawn_stories[Spawn US-6.1.0 & US-6.2.0 Story Leaders]@{ ticket: 'main', assigned: 'agent-c', priority: 'Very High' }
    m6_s62_merged[US-6.2.0 Multi-Party Transfers & Fees Units 1..5 Merged]@{ ticket: '98', assigned: 'agent-b', priority: 'Very High' }
```

---

## 2. Story Promotion & Release Milestone Matrix

| Story ID | Story Title | Story Leader & Profile | Story Branch | Promotion PR Title | Status |
| :--- | :--- | :--- | :--- | :--- | :---: |
| **US-6.1.0** | Connected Accounts Onboarding, AccountSessions & KYC Verification | `US-6.1.0 Story Lead` (`agent-b`) | `feat/us-6.1.0-connected-accounts-and-embedded-sessions` | `feat(us-6.1.0): Connected Accounts, Links, Sessions & KYC (Units 1..5 Promotion)` | 🔍 In Review / Promotion Open |
| **US-6.2.0** | Multi-Party Payment Routing, Transfers & Application Fees | `US-6.2.0 Story Lead` (`agent-b`) | `feat/us-6.2.0-multi-party-transfers-and-application-fees` | `feat(us-6.2.0): Multi-Party Transfers and Application Fees (Units 1..5 Promotion)` | ✅ Merged into Module Branch ([#98](https://github.com/some-of-cs/some-stripe/pull/98)) |
| **MODULE-6** | Connect & Marketplaces Full Release | `Module 6 Lead Architect` (`agent-c`) | `feat/module-6-connect-and-marketplaces` | `feat(module-6): connect and marketplaces` | 🔄 In Progress |
