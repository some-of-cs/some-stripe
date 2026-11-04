# Rule: Global Financial & Statutory Legal Compliance (Gate G-Law)

## Core Objective
Ensure that `some-stripe` complies with international payment industry standards, financial consumer protections, and statutory tax and privacy directives.

## Mandatory Invariants
1. **PCI-DSS 4.0 Non-Transit**:
   - The SDK MUST NEVER store, process, or log raw Primary Account Numbers (PAN), magnetic stripes, or card verification codes (CVC/CVV).
   - Only tokenized payment method IDs (`pm_...`) or client secret tokens are permitted in memory.
2. **PSD2 Strong Customer Authentication (SCA) & 3DS 2.0**:
   - PaymentIntent and SetupIntent models must support `requires_action` transitions with redirect URLs and SDK authentication payloads.
3. **GDPR / CCPA Data Minimization & Retention**:
   - Personal customer data must support erasure workflows (GDPR Article 17) while retaining non-PII financial ledger entries for the statutory tax period.
4. **Minor Currency Unit Exactness**:
   - Monetary values must be represented strictly as integer minor currency units (`i64` cents, pence, satoshis). Zero floating-point arithmetic is permitted for amounts.
5. **Invoicing & Tax Transparency**:
   - Finalized invoices must mandate sequential numbering, tax IDs, and tax breakdowns per EU VAT Directive 2006/112/EC and ZATCA regulations.
