# Rule: Mandatory Live Kanban Board Task Status Updating

## Core Objective
Ensure complete visibility and traceability by requiring every agent to maintain and update the live Kanban board file inside the target module folder (`docs/user-stories/<module-dir>/KANBAN.md`) as tasks transition through their lifecycle.

---

## Mandatory Invariants

### 1. Dedicated Module Kanban File
Every module maintains a live Kanban board markdown file located at:
```
docs/user-stories/<module-dir>/KANBAN.md
```
(e.g. `docs/user-stories/module-4-hosted-checkout-and-links/KANBAN.md`)

### 2. Mandatory Agent Transition Protocol
Every subagent (Unit Implementer, Story Leader, Module Leader) assigned to any task MUST update this file upon entering each phase:

1. **Phase 1: Task Pickup (`📋 Ready` $\to$ `🔄 In Progress`)**:
   - As its first action upon invocation, the agent MUST edit `KANBAN.md`:
     - Move its task node in the Mermaid chart from `BACKLOG` to `IN_PROGRESS`.
     - In the Task Allocation table, change its status from `📋 Ready` to `🔄 In Progress`.
2. **Phase 2: PR Creation (`🔄 In Progress` $\to$ `🔍 In Review / PR Open`)**:
   - Immediately after running `gh pr create`, the agent MUST edit `KANBAN.md`:
     - Move its task node in the Mermaid chart to `REVIEW`.
     - Update the Task Allocation table status to `🔍 PR #XX Open` with the clickable PR link.
3. **Phase 3: PR Merged (`🔍 In Review` $\to$ `✅ Merged`)**:
   - Once the pull request passes gatekeeper review and is merged over GitHub CLI/API (`gh pr merge` / GitHub API), the agent MUST edit `KANBAN.md`:
     - Move its task node in the Mermaid chart to `DONE`.
     - Update the Task Allocation table status to `✅ Merged (commit: <sha>)`.

### 3. Checkpoint Mirroring
When updating `KANBAN.md`, the agent must also ensure that `.agents/checkpoints/<module-id>/checkpoint.json` reflects the updated status, ensuring both human-readable Markdown and machine-readable JSON tracking remain in sync.
