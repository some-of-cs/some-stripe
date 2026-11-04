# Rule: Active Supervisor Watchdog & Immediate Subagent Spawning

## Core Objective
Eliminate delays and passive idling in the hierarchical multi-agent divide-and-conquer workflow by enforcing instantaneous child subagent dispatch and active 3-minute supervisor watchdog oversight.

---

## Mandatory Invariants

### 1. Instantaneous Child Subagent Dispatch (No Idle Chatter)
- **Module Leader Requirement**: Upon invocation, the Module Leader's **FIRST STEP** must be to define and instantiate the child User Story Leaders (`invoke_subagent`). Protracted planning, reading files, or verbal monologues before spawning story leaders is strictly prohibited.
- **Story Leader Requirement**: Upon invocation, the Story Leader's **FIRST STEP** must be to provision the 5 micro-unit worktrees and spawn all 5 child Unit Implementers (`invoke_subagent`). Prolonged architectural analysis prior to spawning implementers is strictly prohibited.
- **Immediate Task Delivery**: Architectural specifications, scope, and quality gate requirements must be passed directly in the child subagent's initial prompt so implementers begin code modifications immediately.

### 2. Main Agent Active Watchdog (3-Minute Supervision Cycle)
- The Root/Main Agent is NOT a passive bystander awaiting user messages.
- The Main Agent operates as the **Active Executive Supervisor & Watchdog**:
  1. Every 3 minutes, the supervisor evaluates the status of the subagent tree (`manage_subagents` list and subagent logs).
  2. The supervisor checks whether the expected child subagents (Story Leaders and Unit Implementers) have been successfully spawned.
  3. **Intervention Threshold**: If any leader has been active for more than 3 minutes without spawning its required child agents, or is spinning in idle logs/monologues:
     - The supervisor MUST immediately intervene.
     - The supervisor directly spawns the missing child subagents (e.g. spawns Story Leaders or Unit Implementers) to unblock parallel execution.
     - The supervisor sends an escalation directive to the stalled agent to resume its gatekeeping role.

### 3. Failure Mode & Fallback Prevention
- The user should NEVER have to ask "where are the agents?" or "why hasn't the subagent been created yet?".
- If an agent is blocked by a file lock or external command, the watchdog detects the contention, cancels or waits appropriately, and ensures forward progress.
