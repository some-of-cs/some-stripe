//! Stripe Credit Notes resource implementations.
//!
//! Provides endpoints to issue, retrieve, update, void, preview, and list credit notes,
//! enforcing balance deduction safety against paid invoices without mutating historical
//! ledger records, ISO 4217 integer currency arithmetic, and RFC 4122 UUID v4 idempotency.

use some_stripe_core::patterns::{Paginator, encode_form_pairs, to_form_params};
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
        let form_data = to_form_params(params)?;

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

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

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

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

        let generated_key;
        let idempotency_key = match params.and_then(|p| p.idempotency_key.as_deref()) {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let note: CreditNote = serde_json::from_value(val)?;
        Ok(note)
    }

    /// Previews a credit note before creation (`GET /v1/credit_notes/preview`).
    pub async fn preview(
        &self,
        params: &PreviewCreditNoteParams,
    ) -> Result<CreditNote, StripeError> {
        let query_data = to_form_params(params)?;
        let url = if query_data.is_empty() {
            format!("{}/v1/credit_notes/preview", self.client.config.api_base)
        } else {
            let query_str = encode_form_pairs(&query_data);
            format!(
                "{}/v1/credit_notes/preview?{query_str}",
                self.client.config.api_base
            )
        };

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
        let url = match params {
            Some(p) => {
                let query_data = to_form_params(p)?;
                if query_data.is_empty() {
                    format!("{}/v1/credit_notes", self.client.config.api_base)
                } else {
                    let query_str = encode_form_pairs(&query_data);
                    format!(
                        "{}/v1/credit_notes?{query_str}",
                        self.client.config.api_base
                    )
                }
            }
            None => format!("{}/v1/credit_notes", self.client.config.api_base),
        };

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
