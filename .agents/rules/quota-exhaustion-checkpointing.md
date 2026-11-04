# Rule: Continuous Progress Checkpointing & Quota Failover Recovery

## Core Objective
Ensure zero loss of execution state, zero duplicate LLM invocations, and deterministic resume capability when API quota is exhausted (`429 / RESOURCE_EXHAUSTED`), an account is switched, or an agent conversation is interrupted.

---

## Mandatory Invariants

### 1. Dedicated Checkpoint Store: `.agents/checkpoints/<module-id>/checkpoint.json`
Every active module MUST continuously maintain a machine-readable JSON checkpoint located at:
```
.agents/checkpoints/<module-id>/checkpoint.json
```
This file acts as the persistent, single source of truth for the multi-agent hierarchy across session boundaries and account rotations.

### 2. Checkpoint JSON Schema & State Representation
The checkpoint MUST include:
```json
{
  "module_id": "module-3",
  "module_branch": "feat/module-3-payments-and-settlement",
  "status": "IN_PROGRESS | COMPLETED",
  "active_profile": "agent-b",
  "next_failover_profile": "agent-c",
  "updated_at": "2026-11-04T12:00:00Z",
  "last_event": "PR #40 merged into feat/us-3.2.0-refunds-and-ledger",
  "next_action": "Wait for Unit 3.2.0-5 PR #47 to merge into story branch",
  "user_stories": {
    "us-3.1.0": {
      "status": "MERGED",
      "branch": "feat/us-3.1.0-payment-intents",
      "story_pr": { "id": 46, "status": "MERGED" },
      "units": {
        "unit-1": { "branch": "feat/us-3.1.0/unit-1-payment-intent-entities", "pr": 37, "status": "MERGED", "profile": "agent-a" },
        "unit-2": { "branch": "feat/us-3.1.0/unit-2-params-schemas", "pr": 43, "status": "MERGED", "profile": "agent-b" },
        "unit-3": { "branch": "feat/us-3.1.0/unit-3-crud-idempotency", "pr": 38, "status": "MERGED", "profile": "agent-c" },
        "unit-4": { "branch": "feat/us-3.1.0/unit-4-3ds-wire-tests", "pr": 44, "status": "MERGED", "profile": "agent-a" },
        "unit-5": { "branch": "feat/us-3.1.0/unit-5-borrow-audit-docs", "pr": 41, "status": "MERGED", "profile": "agent-b" }
      }
    },
    "us-3.2.0": {
      "status": "IN_PROGRESS",
      "branch": "feat/us-3.2.0-refunds-and-ledger",
      "story_pr": { "id": 48, "status": "OPEN" },
      "units": {
        "unit-1": { "branch": "feat/us-3.2.0/unit-1-refund-entity-and-statutory", "pr": 39, "status": "MERGED", "profile": "agent-c" },
        "unit-2": { "branch": "feat/us-3.2.0/unit-2-dispute-and-evidence-schemas", "pr": 42, "status": "MERGED", "profile": "agent-a" },
        "unit-3": { "branch": "feat/us-3.2.0/unit-3-balance-ledger-and-stream", "pr": 45, "status": "MERGED", "profile": "agent-b" },
        "unit-4": { "branch": "feat/us-3.2.0/unit-4-resource-crud-and-mock-tests", "pr": 40, "status": "MERGED", "profile": "agent-c" },
        "unit-5": { "branch": "feat/us-3.2.0/unit-5-minor-currency-audit-and-reports", "pr": 47, "status": "OPEN", "profile": "agent-a" }
      }
    }
  }
}
```

### 3. Continuous Atomic Checkpoint Triggers
The checkpoint MUST be atomically written to disk (`write_file`) upon each of the following milestones:
1. **Worktree Provisioning**: When any unit or story worktree is created.
2. **PR Creation**: Immediately after `gh pr create` finishes for any unit or story.
3. **PR Merge**: Immediately after any PR is merged via GitHub CLI/API.
4. **Pre-Exit / Quota Warning**: Before yielding execution turns or immediately when receiving a 429 quota exhaustion error.

### 4. Zero-Duplication Resume Protocol
Upon resuming execution (or after account rotation):
1. **Load Checkpoint**: Read `.agents/checkpoints/<module-id>/checkpoint.json`.
2. **Reconcile with Remote**: Compare checkpoint against live `gh pr list --state all` and `git branch -a`.
3. **Deterministic Promotion**: Resume strictly from the exact pending micro-unit or story identified in `next_action`.
4. **Prohibition on Re-Running**: Any unit marked `MERGED` in the checkpoint or GitHub PR list MUST NOT be re-spawned or re-executed.
