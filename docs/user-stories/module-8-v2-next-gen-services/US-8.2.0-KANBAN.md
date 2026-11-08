# US-8.2.0: High-Throughput Usage Meter Ingestion & Event Stream Billing — Kanban Board

Hierarchical task tracking for **US-8.2.0 (High-Throughput Usage Meter Ingestion & Event Stream Billing)**.

---

## 1. Status Board

```mermaid
---
config:
  kanban:
    ticketBaseUrl: 'https://github.com/some-of-cs/some-stripe/pull/#TICKET#'
---
kanban
  todo[📋 Backlog / Ready]
  in_progress[🔄 In Progress]
    u1[Unit 1: Meter Event Sessions & Token Lifecycles]@{ ticket: 'feat/us-8.2.0/unit-1-meter-session-resource', assigned: 'agent-c', priority: 'Very High' }
    u2[Unit 2: Batch Meter Event Streams & Serializer]@{ ticket: 'feat/us-8.2.0/unit-2-meter-stream-resource', assigned: 'agent-a', priority: 'High' }
    u3[Unit 3: Meter Event Adjustments & Cancelations]@{ ticket: 'feat/us-8.2.0/unit-3-meter-adjustments-resource', assigned: 'agent-b', priority: 'High' }
    u4[Unit 4: V2 Billing Meters Management]@{ ticket: 'feat/us-8.2.0/unit-4-billing-meters-resource', assigned: 'agent-c', priority: 'Very High' }
    u5[Unit 5: V2 Billing Facade & Ingestion Engine]@{ ticket: 'feat/us-8.2.0/unit-5-v2-billing-facade-engine', assigned: 'agent-b', priority: 'High' }
  in_review[🔍 In Review / PR Open]
  done[✅ Merged into Story Branch]
```

---

## 2. Micro-Unit Specifications Matrix

| Unit ID | Unit Title & Functional Scope | Assigned Profile | Branch Name | Target PR Title | Status |
| :--- | :--- | :---: | :--- | :--- | :---: |
| **Unit 1** | **Meter Event Sessions & Token Lifecycles**: `MeterEventSession`, `MeterEventSessionStatus`, `SessionTokenDetails`, `CreateMeterEventSessionParams`, and `MeterEventSessionResource` (`create`, `retrieve`, `expire`) with memory zeroization on drop. | `agent-c` | `feat/us-8.2.0/unit-1-meter-session-resource` | `feat(us-8.2.0/unit-1): meter event sessions domain models and resource endpoints` | 🔄 In Progress |
| **Unit 2** | **Batch Meter Event Streams & Serializer**: `MeterEventPayload`, `MeterPayload`, `MeterEventBatch`, `BatchIngestionResponse`, `CreateMeterEventStreamParams`, `MeterBatchSerializer` (zero-copy JSON batch streaming), and `MeterEventStreamResource` (`create`, `create_batch`). | `agent-a` | `feat/us-8.2.0/unit-2-meter-stream-resource` | `feat(us-8.2.0/unit-2): batch meter event streams and zero-copy serializer` | 🔄 In Progress |
| **Unit 3** | **Meter Event Adjustments & Cancelations**: `MeterEventAdjustment`, `CancelAdjustmentDetails`, `AdjustmentReason`, `CreateMeterEventAdjustmentParams`, `ListMeterEventAdjustmentsParams`, and `MeterEventAdjustmentsResource` (`create`, `retrieve`, `list`) with client deduplication audit. | `agent-b` | `feat/us-8.2.0/unit-3-meter-adjustments-resource` | `feat(us-8.2.0/unit-3): meter event adjustments domain entities and resource endpoints` | 🔄 In Progress |
| **Unit 4** | **V2 Billing Meters Management**: `V2BillingMeter`, `MeterStatus`, `ValueSettings`, `CustomerMappingSettings`, `AggregationFormula`, `CreateBillingMeterParams`, `UpdateBillingMeterParams`, `ListBillingMetersParams`, and `BillingMetersResource` (`create`, `retrieve`, `update`, `deactivate`, `reactivate`, `list`). | `agent-c` | `feat/us-8.2.0/unit-4-billing-meters-resource` | `feat(us-8.2.0/unit-4): v2 billing meters management domain models and resource` | 🔄 In Progress |
| **Unit 5** | **V2 Billing Facade & Ingestion Engine**: `V2Billing` resource facade mounting `meter_event_session()`, `meter_event_stream()`, `meter_event_adjustments()`, `meters()`, `MeterIngestionEngine` (pipelined keep-alive connection pool & idempotency cache), and `StripeClient` integration `client.v2().billing()`. | `agent-b` | `feat/us-8.2.0/unit-5-v2-billing-facade-engine` | `feat(us-8.2.0/unit-5): v2 billing facade and high-throughput meter ingestion engine` | 🔄 In Progress |
