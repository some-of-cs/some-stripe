# Dedicated Quota Sentinel Agent (`quota-sentinel`)

## 1. Purpose & Mandate

To prevent sudden workspace stalls due to API quota exhaustion (`RESOURCE_EXHAUSTED` / HTTP 429), a dedicated **Quota Sentinel** agent operates continuously in the background.

## 2. Responsibilities

1. **Continuous Quota Monitoring**:
   - Monitors model usage, rate limits, and remaining quota percentages across active profiles (`agent-a`, `agent-b`, `agent-c`).
   - Tracks response error streams for 429 warnings or high token consumption.
2. **1% Quota Threshold Warning**:
   - When remaining quota drops to **1%** (or upon any rate limit spike / 429), the Quota Sentinel immediately broadcasts a high-priority alert to:
     - All active child agents (Module Leader, Story Leaders, Unit Implementers).
     - The Main Supervisor conversation.
3. **Automated Standby Timer**:
   - Instructs all agents to pause execution and save checkpoints in `.agents/checkpoints/`.
   - Schedules a wait timer (`schedule` tool) until the quota reset window arrives.
   - Upon reset, notifies all agents to resume execution safely.
