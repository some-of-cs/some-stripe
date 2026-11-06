# Module 5: Customers & Billing — Module Level Executive Kanban

This executive Kanban board tracks high-level milestone promotions for **Module 5 (Customers & Billing)** with an explicit **`🤖 Subagent Creating`** state to eliminate spawning latency.

> [!IMPORTANT]
> **Supervisor Invariant**: If the Module Leader remains in `spawning[🤖 Subagent Creating]` for >3 minutes without creating the 2 User Story Leaders, the Active Supervisor Watchdog intervenes and spawns them directly.
>
> **Gatekeeper Subagents & Skills Invariant**: Module promotions require full sign-off by the Federated Gatekeeper Council, each executing with their designated specialized skills from `.agents/skills/`.

---

## 1. Module Level Mermaid Kanban

```mermaid
---
config:
  kanban:
    ticketBaseUrl: 'https://github.com/some-of-cs/some-stripe/pull/#TICKET#'
---
kanban
  todo[📋 Backlog / Ready]
  spawning[🤖 Subagent Creating]
    m5_setup[Pre-Provision 13 Git Worktrees & Remote Origin Branches]@{ ticket: 'main', assigned: 'agent-c', priority: 'High' }
    m5_spawn_stories[Spawn US-5.1.0 & US-5.2.0 Story Leaders]@{ ticket: 'main', assigned: 'agent-c', priority: 'Very High' }
  in_progress[🔄 Stories In Progress]
    m5_s51_progress[US-5.1.0 Customer Vaulting & Subscriptions in Progress]@{ ticket: 'main', assigned: 'agent-a', priority: 'High' }
    m5_s52_progress[US-5.2.0 Invoices, Credit Notes & Portal in Progress]@{ ticket: 'main', assigned: 'agent-b', priority: 'High' }
  review[🔍 Module Review & Gate Sign-off]
    m5_gates[Module-Wide Quality Gates Audit: G0-G4, G-Borrow, G2, G-Law]@{ ticket: 'main', assigned: 'agent-c', priority: 'Very High' }
  done[✅ Done / Merged to main]
```

---

## 2. Milestone Promotion Matrix with Subagents & Skills

| Milestone ID | Milestone Scope | Lead Assignee & Profile | Mandatory Lead Skill | Assigned Gatekeeper Council Subagents | Mandatory Gatekeeper Skills | Status | PR / SHA |
| :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: |
| **M5-SETUP** | Provision 13 worktrees & origin branches | `Module Lead` (`agent-c`) | `antigravity-guide` | `Git Workflow Master` | `stripe-best-practices` | 📋 Ready | Pending |
| **M5-SPAWN** | Spawn US-5.1.0 & US-5.2.0 Story Leaders | `Module 5 Lead Architect` (`agent-c`) | `stripe-docs` | `Main Supervisor Watchdog` | `antigravity-guide` | 🤖 Spawning | Pending |
| **M5-S51** | US-5.1.0 Promotion (Customer & Subs) | `US-5.1.0 Story Leader` (`agent-a`) | `stripe-best-practices` | `API Tester`<br>`microkernel-architect`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices`<br>`security-audit` | 📋 Ready | Story PR |
| **M5-S52** | US-5.2.0 Promotion (Invoices & Portal) | `US-5.2.0 Story Leader` (`agent-b`) | `stripe-best-practices` | `API Tester`<br>`microkernel-architect`<br>`Code Reviewer` | `stripe-docs`<br>`stripe-best-practices`<br>`security-audit` | 📋 Ready | Story PR |
| **M5-GATES** | Full Workspace Quality Gate Sign-Off | `Module 5 Lead Architect` (`agent-c`) | `stripe-best-practices` | Full Council (`Code Reviewer`, `AppSec`, `Rust Refactoring`, `API Tester`, `Tech Writer`) | `security-audit`<br>`upgrade-stripe`<br>`stripe-docs`<br>`stripe-best-practices` | 📋 Ready | Council Review |
| **M5-MAIN** | Module 5 Final Merge into `main` | `Module 5 Lead Architect` (`agent-c`) | `stripe-best-practices` | `Code Reviewer`<br>`Git Workflow Master` | `security-audit`<br>`stripe-best-practices` | 📋 Ready | Module PR |
