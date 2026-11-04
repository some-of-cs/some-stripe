# Rule: Enforced Multi-Profile Round-Robin Allocation (`agent-a`, `agent-b`, `agent-c`)

## Core Objective
Strictly enforce active distribution of subagent workloads across all 3 billing accounts (`account1`/`agent-a`, `account2`/`agent-b`, `account3`/`agent-c`) to split the API quota evenly and prevent single-account exhaustion.

---

## Mandatory Invariants

### 1. Universal Agent Generator Rotation
- Profiles are **universal agent generators** tied to separate Google AI Pro billing accounts:
  - `account1` (`agent-a`): Account slot 1
  - `account2` (`agent-b`): Account slot 2
  - `account3` (`agent-c`): Account slot 3
- Workloads and child subagent invocations MUST actively rotate across the 3 profiles in round-robin fashion:
  - Subagent 1: Assigned to `agent-a` (`account1`)
  - Subagent 2: Assigned to `agent-b` (`account2`)
  - Subagent 3: Assigned to `agent-c` (`account3`)
  - Subagent 4: Assigned to `agent-a` (`account1`)
  - Subagent 5: Assigned to `agent-b` (`account2`)
  - And continuing across tiers.

### 2. Mandatory Profile Designation in Role & Audit Report
- Every spawned subagent MUST specify its assigned profile in its role title and prompt:
  - Example: `Role: "Unit 4 Implementer (Profile: agent-b / account2)"`
- Every agent's audit report under `.agents/reports/` MUST explicitly document:
  ```markdown
  - **Billing Profile**: `account2` (`agent-b`)
  ```
- Concentrating multiple sequential subagents on a single account without round-robin rotation is strictly prohibited.

### 3. Automatic Quota Failover
- In the event of an HTTP 429 (`RESOURCE_EXHAUSTED` / `exceeded your current quota`), the process must immediately rotate to the next profile (`agent-a` -> `agent-b` -> `agent-c`) without halting the pipeline.
