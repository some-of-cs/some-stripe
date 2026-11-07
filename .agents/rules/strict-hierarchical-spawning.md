# Strict Hierarchical Delegation Invariant (Zero Cross-Tier Spawning)

## 1. Architectural Role Hierarchy

The `some-stripe` workspace mandates a strict multi-tier delegation model:

```mermaid
flowchart TD
    Main[Main Conversation / Supervisor] --> ModLead[Module Leader (US-X.0.0)]
    ModLead --> US1Lead[US-X.1.0 Story Leader]
    ModLead --> US2Lead[US-X.2.0 Story Leader]
    US1Lead --> U11[Unit 1 Implementer]
    US1Lead --> U12[Unit 2 Implementer]
    US1Lead --> U13[Unit 3 Implementer]
    US1Lead --> U14[Unit 4 Implementer]
    US1Lead --> U15[Unit 5 Implementer]
    US2Lead --> U21[Unit 1 Implementer]
    US2Lead --> U22[Unit 2 Implementer]
    US2Lead --> U23[Unit 3 Implementer]
    US2Lead --> U24[Unit 4 Implementer]
    US2Lead --> U25[Unit 5 Implementer]
```

## 2. Inviolable Delegation Rules

1. **Module Leader Boundary**:
   - The Module Leader MUST ONLY spawn and supervise User Story Leaders (`US-X.1.0 Story Leader`, `US-X.2.0 Story Leader`).
   - The Module Leader is **STRICTLY PROHIBITED** from generating Unit Implementers directly.
2. **Story Leader Boundary**:
   - Each User Story Leader is strictly responsible for spawning its own 5 Unit Implementers (`Unit 1`, `Unit 2`, `Unit 3`, `Unit 4`, `Unit 5`).
   - Each Story Leader coordinates cascading PR merges for its 5 units into the story branch.
3. **Unit Implementer Boundary**:
   - Each Unit Implementer works in its own isolated worktree (`.worktrees/feat-us-x.y.0-unit-N-...`).
   - Each Unit Implementer writes tests, ensures zero warnings, invokes `Code Reviewer`, fixes any flagged issues, and opens its PR via `Git Workflow Master`.
