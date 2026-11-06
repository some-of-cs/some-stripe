//! Stripe Invoices resource implementations.
//!
//! Provides endpoints to create, retrieve, update, delete, finalize, pay, void,
//! mark uncollectible, send, and list invoices, adhering to EU VAT Directive 2006/112/EC
//! sequential numbering requirements and RFC 4122 UUID v4 idempotency.

use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateInvoiceParams, FinalizeInvoiceParams, Invoice, InvoiceList, ListInvoicesParams,
    PayInvoiceParams, UpdateInvoiceParams, VoidInvoiceParams,
};

/// Resource for managing Stripe Invoices (`stripe.invoices()`).
#[derive(Clone, Copy)]
pub struct InvoicesResource<'a> {
    client: &'a StripeClient,
}

impl<'a> InvoicesResource<'a> {
    /// Creates a new `InvoicesResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a new draft invoice (`POST /v1/invoices`).
    pub async fn create(&self, params: &CreateInvoiceParams) -> Result<Invoice, StripeError> {
        let url = format!("{}/v1/invoices", self.client.config.api_base);
        let mut form_data = Vec::new();

        form_data.push(("customer", params.customer.as_str()));

        if let Some(auto) = params.auto_advance {
            form_data.push(("auto_advance", if auto { "true" } else { "false" }));
        }
        if let Some(ref m) = params.collection_method {
            form_data.push(("collection_method", m.as_str()));
        }
        if let Some(ref c) = params.currency {
            form_data.push(("currency", c.as_str()));
        }
        if let Some(ref d) = params.description {
            form_data.push(("description", d.as_str()));
        }
        let due_date_str = params.due_date.map(|d| d.to_string());
        if let Some(ref d) = due_date_str {
            form_data.push(("due_date", d.as_str()));
        }
        let days_str = params.days_until_due.map(|d| d.to_string());
        if let Some(ref d) = days_str {
            form_data.push(("days_until_due", d.as_str()));
        }
        if let Some(ref s) = params.subscription {
            form_data.push(("subscription", s.as_str()));
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
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Retrieves an invoice by ID (`GET /v1/invoices/{id}`).
    pub async fn retrieve(&self, invoice_id: &str) -> Result<Invoice, StripeError> {
        let safe_id = validate_and_encode_id(invoice_id)?;
        let url = format!("{}/v1/invoices/{}", self.client.config.api_base, safe_id);

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Updates an invoice (`POST /v1/invoices/{id}`).
    pub async fn update(
        &self,
        invoice_id: &str,
        params: &UpdateInvoiceParams,
    ) -> Result<Invoice, StripeError> {
        let safe_id = validate_and_encode_id(invoice_id)?;
        let url = format!("{}/v1/invoices/{}", self.client.config.api_base, safe_id);
        let mut form_data = Vec::new();

        if let Some(auto) = params.auto_advance {
            form_data.push(("auto_advance", if auto { "true" } else { "false" }));
        }
        if let Some(ref m) = params.collection_method {
            form_data.push(("collection_method", m.as_str()));
        }
        if let Some(ref d) = params.description {
            form_data.push(("description", d.as_str()));
        }
        let due_date_str = params.due_date.map(|d| d.to_string());
        if let Some(ref d) = due_date_str {
            form_data.push(("due_date", d.as_str()));
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
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Deletes a draft invoice (`DELETE /v1/invoices/{id}`).
    pub async fn del(&self, invoice_id: &str) -> Result<serde_json::Value, StripeError> {
        let safe_id = validate_and_encode_id(invoice_id)?;
        let url = format!("{}/v1/invoices/{}", self.client.config.api_base, safe_id);

        let req = self.client.http.delete(&url);
        let val = self.client.execute(req).await?;
        Ok(val)
    }

    /// Finalizes an invoice (`POST /v1/invoices/{id}/finalize`).
    pub async fn finalize_invoice(
        &self,
        invoice_id: &str,
        params: Option<&FinalizeInvoiceParams>,
    ) -> Result<Invoice, StripeError> {
        let safe_id = validate_and_encode_id(invoice_id)?;
        let url = format!(
            "{}/v1/invoices/{}/finalize",
            self.client.config.api_base, safe_id
        );
        let mut form_data = Vec::new();

        if let Some(auto) = params.and_then(|p| p.auto_advance) {
            form_data.push(("auto_advance", if auto { "true" } else { "false" }));
        }

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
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Pays an invoice (`POST /v1/invoices/{id}/pay`).
    pub async fn pay(
        &self,
        invoice_id: &str,
        params: Option<&PayInvoiceParams>,
    ) -> Result<Invoice, StripeError> {
        let safe_id = validate_and_encode_id(invoice_id)?;
        let url = format!(
            "{}/v1/invoices/{}/pay",
            self.client.config.api_base, safe_id
        );
        let mut form_data = Vec::new();

        if let Some(p) = params {
            if let Some(forgive) = p.forgive {
                form_data.push(("forgive", if forgive { "true" } else { "false" }));
            }
            if let Some(ref m) = p.mandate {
                form_data.push(("mandate", m.as_str()));
            }
            if let Some(off) = p.off_session {
                form_data.push(("off_session", if off { "true" } else { "false" }));
            }
            if let Some(pob) = p.paid_out_of_band {
                form_data.push(("paid_out_of_band", if pob { "true" } else { "false" }));
            }
            if let Some(ref pm) = p.payment_method {
                form_data.push(("payment_method", pm.as_str()));
            }
            if let Some(ref s) = p.source {
                form_data.push(("source", s.as_str()));
            }
        }

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
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Voids an invoice (`POST /v1/invoices/{id}/void`).
    pub async fn void_invoice(
        &self,
        invoice_id: &str,
        params: Option<&VoidInvoiceParams>,
    ) -> Result<Invoice, StripeError> {
        let safe_id = validate_and_encode_id(invoice_id)?;
        let url = format!(
            "{}/v1/invoices/{}/void",
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
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Marks an invoice as uncollectible (`POST /v1/invoices/{id}/mark_uncollectible`).
    pub async fn mark_uncollectible(&self, invoice_id: &str) -> Result<Invoice, StripeError> {
        let safe_id = validate_and_encode_id(invoice_id)?;
        let url = format!(
            "{}/v1/invoices/{}/mark_uncollectible",
            self.client.config.api_base, safe_id
        );
        let form_data: Vec<(&str, &str)> = Vec::new();
        let key = crate::resources::generate_uuid_v4();

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &key);

        let val = self.client.execute(req).await?;
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Sends an invoice for manual collection (`POST /v1/invoices/{id}/send`).
    pub async fn send_invoice(&self, invoice_id: &str) -> Result<Invoice, StripeError> {
        let safe_id = validate_and_encode_id(invoice_id)?;
        let url = format!(
            "{}/v1/invoices/{}/send",
            self.client.config.api_base, safe_id
        );
        let form_data: Vec<(&str, &str)> = Vec::new();
        let key = crate::resources::generate_uuid_v4();

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &key);

        let val = self.client.execute(req).await?;
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Lists invoices (`GET /v1/invoices`).
    pub async fn list(
        &self,
        params: Option<&ListInvoicesParams>,
    ) -> Result<InvoiceList, StripeError> {
        let mut url = format!("{}/v1/invoices", self.client.config.api_base);
        let mut query_params = Vec::new();
        let limit_str = params.and_then(|p| p.limit).map(|l| l.to_string());

        if let Some(p) = params {
            if let Some(ref c) = p.customer {
                query_params.push(("customer", c.as_str()));
            }
            if let Some(s) = p.status {
                query_params.push(("status", s.as_str()));
            }
            if let Some(ref s) = p.subscription {
                query_params.push(("subscription", s.as_str()));
            }
            if let Some(ref m) = p.collection_method {
                query_params.push(("collection_method", m.as_str()));
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
        let list: InvoiceList = serde_json::from_value(val)?;
        Ok(list)
    }
}
