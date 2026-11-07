//! Stripe Credit Notes resource implementations.
//!
//! Provides endpoints to issue, retrieve, update, void, preview, and list credit notes,
//! enforcing balance deduction safety against paid invoices without mutating historical
//! ledger records, ISO 4217 integer currency arithmetic, and RFC 4122 UUID v4 idempotency.

use some_stripe_core::patterns::Paginator;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateCreditNoteParams, CreditNote, CreditNoteList, ListCreditNotesParams,
    PreviewCreditNoteParams, UpdateCreditNoteParams, VoidCreditNoteParams,
};

/// Key extractor function for [`CreditNote`] in paginated streams.
pub fn credit_note_id(cn: &CreditNote) -> &str {
    cn.id.as_str()
}

/// Resource for managing Stripe Credit Notes (`stripe.credit_notes()`).
#[derive(Clone, Copy)]
pub struct CreditNotesResource<'a> {
    client: &'a StripeClient,
}

impl<'a> CreditNotesResource<'a> {
    /// Creates a new `CreditNotesResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Issues a new credit note against an invoice (`POST /v1/credit_notes`).
    ///
    /// Implements Gate G2 UUID v4 idempotency injection when no key is explicitly supplied.
    pub async fn create(&self, params: &CreateCreditNoteParams) -> Result<CreditNote, StripeError> {
        let url = format!("{}/v1/credit_notes", self.client.config.api_base);
        let mut form_data = Vec::new();

        form_data.push(("invoice", params.invoice.as_str()));

        let amount_str = params.amount.map(|a| a.to_string());
        if let Some(ref a) = amount_str {
            form_data.push(("amount", a.as_str()));
        }

        let credit_amount_str = params.credit_amount.map(|a| a.to_string());
        if let Some(ref ca) = credit_amount_str {
            form_data.push(("credit_amount", ca.as_str()));
        }

        let refund_amount_str = params.refund_amount.map(|a| a.to_string());
        if let Some(ref ra) = refund_amount_str {
            form_data.push(("refund_amount", ra.as_str()));
        }

        let out_of_band_str = params.out_of_band_amount.map(|a| a.to_string());
        if let Some(ref ob) = out_of_band_str {
            form_data.push(("out_of_band_amount", ob.as_str()));
        }

        if let Some(r) = params.reason {
            form_data.push(("reason", r.as_str()));
        }

        if let Some(ref m) = params.memo {
            form_data.push(("memo", m.as_str()));
        }

        let mut metadata_entries = Vec::new();
        if let Some(ref meta) = params.metadata {
            for (k, v) in meta {
                metadata_entries.push((format!("metadata[{k}]"), v.as_str()));
            }
        }
        for (k, v) in &metadata_entries {
            form_data.push((k.as_str(), *v));
        }

        // Bracket serialization for line items (Gate G3 RFC 1738 indexed bracket notation)
        let mut line_entries = Vec::new();
        if let Some(ref lines) = params.lines {
            for (i, line) in lines.iter().enumerate() {
                line_entries.push((format!("lines[{i}][type]"), line.r#type.clone()));
                if let Some(ref il) = line.invoice_line_item {
                    line_entries.push((format!("lines[{i}][invoice_line_item]"), il.clone()));
                }
                if let Some(amount) = line.amount {
                    line_entries.push((format!("lines[{i}][amount]"), amount.to_string()));
                }
                if let Some(ref desc) = line.description {
                    line_entries.push((format!("lines[{i}][description]"), desc.clone()));
                }
                if let Some(qty) = line.quantity {
                    line_entries.push((format!("lines[{i}][quantity]"), qty.to_string()));
                }
                if let Some(unit) = line.unit_amount {
                    line_entries.push((format!("lines[{i}][unit_amount]"), unit.to_string()));
                }
            }
        }
        for (k, v) in &line_entries {
            form_data.push((k.as_str(), v.as_str()));
        }

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let note: CreditNote = serde_json::from_value(val)?;
        Ok(note)
    }

    /// Retrieves an existing credit note by ID (`GET /v1/credit_notes/{id}`).
    pub async fn retrieve(&self, credit_note_id: &str) -> Result<CreditNote, StripeError> {
        let safe_id = validate_and_encode_id(credit_note_id)?;
        let url = format!(
            "{}/v1/credit_notes/{}",
            self.client.config.api_base, safe_id
        );

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let note: CreditNote = serde_json::from_value(val)?;
        Ok(note)
    }

    /// Updates memo or metadata of an existing credit note (`POST /v1/credit_notes/{id}`).
    pub async fn update(
        &self,
        credit_note_id: &str,
        params: &UpdateCreditNoteParams,
    ) -> Result<CreditNote, StripeError> {
        let safe_id = validate_and_encode_id(credit_note_id)?;
        let url = format!(
            "{}/v1/credit_notes/{}",
            self.client.config.api_base, safe_id
        );
        let mut form_data = Vec::new();

        if let Some(ref m) = params.memo {
            form_data.push(("memo", m.as_str()));
        }

        let mut metadata_entries = Vec::new();
        if let Some(ref meta) = params.metadata {
            for (k, v) in meta {
                metadata_entries.push((format!("metadata[{k}]"), v.as_str()));
            }
        }
        for (k, v) in &metadata_entries {
            form_data.push((k.as_str(), *v));
        }

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let note: CreditNote = serde_json::from_value(val)?;
        Ok(note)
    }

    /// Voids an issued credit note (`POST /v1/credit_notes/{id}/void`).
    pub async fn void_credit_note(
        &self,
        credit_note_id: &str,
        params: Option<&VoidCreditNoteParams>,
    ) -> Result<CreditNote, StripeError> {
        let safe_id = validate_and_encode_id(credit_note_id)?;
        let url = format!(
            "{}/v1/credit_notes/{}/void",
            self.client.config.api_base, safe_id
        );
        let form_data: Vec<(&str, &str)> = Vec::new();

        let idempotency_key =
            super::resolve_idempotency_key(params.and_then(|p| p.idempotency_key.as_deref()));

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let note: CreditNote = serde_json::from_value(val)?;
        Ok(note)
    }

    /// Previews a credit note before creation (`GET /v1/credit_notes/preview`).
    pub async fn preview(
        &self,
        params: &PreviewCreditNoteParams,
    ) -> Result<CreditNote, StripeError> {
        let mut url = format!("{}/v1/credit_notes/preview", self.client.config.api_base);
        let mut query_params: Vec<(&str, String)> = Vec::new();

        query_params.push(("invoice", params.invoice.clone()));

        if let Some(a) = params.amount {
            query_params.push(("amount", a.to_string()));
        }
        if let Some(ca) = params.credit_amount {
            query_params.push(("credit_amount", ca.to_string()));
        }
        if let Some(ra) = params.refund_amount {
            query_params.push(("refund_amount", ra.to_string()));
        }
        if let Some(ob) = params.out_of_band_amount {
            query_params.push(("out_of_band_amount", ob.to_string()));
        }
        if let Some(r) = params.reason {
            query_params.push(("reason", r.as_str().to_string()));
        }
        if let Some(ref m) = params.memo {
            query_params.push(("memo", m.clone()));
        }

        if let Some(ref meta) = params.metadata {
            for (k, v) in meta {
                query_params.push(("metadata", format!("{k}={v}")));
            }
        }

        if let Some(ref lines) = params.lines {
            for (i, line) in lines.iter().enumerate() {
                query_params.push(("line_type", format!("lines[{i}][type]={}", line.r#type)));
                if let Some(ref il) = line.invoice_line_item {
                    query_params.push(("line_il", format!("lines[{i}][invoice_line_item]={il}")));
                }
                if let Some(amount) = line.amount {
                    query_params.push(("line_amount", format!("lines[{i}][amount]={amount}")));
                }
                if let Some(ref desc) = line.description {
                    query_params.push(("line_desc", format!("lines[{i}][description]={desc}")));
                }
                if let Some(qty) = line.quantity {
                    query_params.push(("line_qty", format!("lines[{i}][quantity]={qty}")));
                }
                if let Some(unit) = line.unit_amount {
                    query_params.push(("line_unit", format!("lines[{i}][unit_amount]={unit}")));
                }
            }
        }

        if !query_params.is_empty() {
            let encoded: Vec<String> = query_params
                .iter()
                .map(|(k, v)| {
                    if k.starts_with("line_") || *k == "metadata" {
                        v.clone()
                    } else {
                        format!("{k}={v}")
                    }
                })
                .collect();
            url.push('?');
            url.push_str(&encoded.join("&"));
        }

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let note: CreditNote = serde_json::from_value(val)?;
        Ok(note)
    }

    /// Lists credit notes (`GET /v1/credit_notes`).
    pub async fn list(
        &self,
        params: Option<&ListCreditNotesParams>,
    ) -> Result<CreditNoteList, StripeError> {
        let mut url = format!("{}/v1/credit_notes", self.client.config.api_base);
        let mut query_params = Vec::new();
        let limit_str = params.and_then(|p| p.limit).map(|l| l.to_string());

        if let Some(p) = params {
            if let Some(ref inv) = p.invoice {
                query_params.push(("invoice", inv.as_str()));
            }
            if let Some(ref c) = p.customer {
                query_params.push(("customer", c.as_str()));
            }
            if let Some(ref l) = limit_str {
                query_params.push(("limit", l.as_str()));
            }
            if let Some(ref a) = p.starting_after {
                query_params.push(("starting_after", a.as_str()));
            }
            if let Some(ref b) = p.ending_before {
                query_params.push(("ending_before", b.as_str()));
            }
        }

        if !query_params.is_empty() {
            let encoded: Vec<String> = query_params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            url.push('?');
            url.push_str(&encoded.join("&"));
        }

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: CreditNoteList = serde_json::from_value(val)?;
        Ok(list)
    }

    /// Auto-paginator stream for credit notes.
    pub fn list_paginated(&self, params: &ListCreditNotesParams) -> Paginator<'a, CreditNote> {
        let mut paginator = Paginator::new(
            self.client,
            "v1/credit_notes".to_string(),
            params.limit.unwrap_or(10),
            credit_note_id,
        );

        if let Some(ref sa) = params.starting_after {
            paginator = paginator.starting_after(sa);
        }
        if let Some(ref eb) = params.ending_before {
            paginator = paginator.ending_before(eb);
        }

        paginator
    }

    /// Convenience helper returning an auto-paginator with default page limit (10).
    pub fn list_default(&self) -> Paginator<'a, CreditNote> {
        self.list_paginated(&ListCreditNotesParams::default())
    }

    /// Validates that credit note deduction against an invoice preserves historical ledger immutability:
    /// credit amount must be strictly positive and cannot exceed total paid amount on the invoice.
    pub fn validate_balance_deduction(
        &self,
        credit_amount: i64,
        invoice_paid_amount: i64,
    ) -> Result<(), StripeError> {
        if credit_amount <= 0 {
            return Err(StripeError::InvalidRequest(
                "Credit note amount must be strictly positive".into(),
            ));
        }
        if credit_amount > invoice_paid_amount {
            return Err(StripeError::InvalidRequest(
                "Credit note amount cannot exceed total invoice paid amount".into(),
            ));
        }
        Ok(())
    }
}
