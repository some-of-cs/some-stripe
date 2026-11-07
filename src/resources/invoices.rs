//! Stripe Invoices resource implementations.
//!
//! Provides endpoints to create, retrieve, update, delete, finalize, pay, void,
//! mark uncollectible, send, and list invoices, adhering to EU VAT Directive 2006/112/EC
//! sequential numbering requirements and RFC 4122 UUID v4 idempotency.

use some_stripe_core::patterns::{Paginator, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    AddInvoiceLinesParams, CreateInvoiceParams, FinalizeInvoiceParams, Invoice, InvoiceLineItem,
    InvoiceList, InvoiceUpcomingParams, ListInvoiceLineItemsParams, ListInvoicesParams,
    PayInvoiceParams, RemoveInvoiceLinesParams, UpdateInvoiceLineParams, UpdateInvoiceParams,
    VoidInvoiceParams, invoice_line_item_id,
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

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

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

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

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

        let idempotency_key =
            super::resolve_idempotency_key(params.and_then(|p| p.idempotency_key.as_deref()));

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

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

        let idempotency_key =
            super::resolve_idempotency_key(params.and_then(|p| p.idempotency_key.as_deref()));

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

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

        let idempotency_key =
            super::resolve_idempotency_key(params.and_then(|p| p.idempotency_key.as_deref()));

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

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
        let idempotency_key = super::resolve_idempotency_key(None);

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

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
        let idempotency_key = super::resolve_idempotency_key(None);

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

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

    /// Retrieves an upcoming invoice preview for a customer or subscription (`GET /v1/invoices/upcoming`).
    ///
    /// Handles the upcoming invoice null-ID wire safety invariant (`invoice.id` is `None`).
    pub async fn upcoming(&self, params: &InvoiceUpcomingParams) -> Result<Invoice, StripeError> {
        let mut url = format!("{}/v1/invoices/upcoming", self.client.config.api_base);
        let form_pairs = to_form_params(params)?;

        if !form_pairs.is_empty() {
            let encoded: Vec<String> = form_pairs.iter().map(|(k, v)| format!("{k}={v}")).collect();
            url.push('?');
            url.push_str(&encoded.join("&"));
        }

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Dynamically appends line items to an invoice (`POST /v1/invoices/{id}/add_lines`).
    pub async fn add_lines(
        &self,
        invoice_id: &str,
        params: &AddInvoiceLinesParams,
    ) -> Result<Invoice, StripeError> {
        let encoded_id = validate_and_encode_id(invoice_id)?;
        let url = format!(
            "{}/v1/invoices/{}/add_lines",
            self.client.config.api_base, encoded_id
        );

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());
        let form_pairs = to_form_params(params)?;

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_pairs)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Updates a specific line item on an invoice (`POST /v1/invoices/{id}/lines/{line_id}`).
    pub async fn update_lines(
        &self,
        invoice_id: &str,
        line_id: &str,
        params: &UpdateInvoiceLineParams,
    ) -> Result<InvoiceLineItem, StripeError> {
        let encoded_inv_id = validate_and_encode_id(invoice_id)?;
        let encoded_line_id = validate_and_encode_id(line_id)?;
        let url = format!(
            "{}/v1/invoices/{}/lines/{}",
            self.client.config.api_base, encoded_inv_id, encoded_line_id
        );

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());
        let form_pairs = to_form_params(params)?;

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_pairs)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let line_item: InvoiceLineItem = serde_json::from_value(val)?;
        Ok(line_item)
    }

    /// Removes line items from an invoice (`POST /v1/invoices/{id}/remove_lines`).
    pub async fn remove_lines(
        &self,
        invoice_id: &str,
        line_ids: &[&str],
    ) -> Result<Invoice, StripeError> {
        let encoded_id = validate_and_encode_id(invoice_id)?;
        let url = format!(
            "{}/v1/invoices/{}/remove_lines",
            self.client.config.api_base, encoded_id
        );

        let params = RemoveInvoiceLinesParams::from_ids(line_ids.iter().copied());
        let form_pairs = to_form_params(&params)?;
        let idempotency_key = super::resolve_idempotency_key(None);

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_pairs)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let invoice: Invoice = serde_json::from_value(val)?;
        Ok(invoice)
    }

    /// Returns a zero-clone paginator streaming line items for an Invoice (`GET /v1/invoices/{id}/lines`).
    pub fn list_line_items(
        &self,
        invoice_id: &str,
        params: &ListInvoiceLineItemsParams,
    ) -> Paginator<'a, InvoiceLineItem> {
        let (path, initial_err) = match validate_and_encode_id(invoice_id) {
            Ok(encoded) => (format!("v1/invoices/{encoded}/lines"), None),
            Err(e) => (String::new(), Some(e)),
        };

        let mut paginator = Paginator::new(
            self.client,
            path,
            params.limit.unwrap_or(10),
            invoice_line_item_id,
        );

        if let Some(err) = initial_err {
            paginator = paginator.with_initial_error(err);
        }
        if let Some(ref sa) = params.starting_after {
            paginator = paginator.starting_after(sa);
        }
        if let Some(ref eb) = params.ending_before {
            paginator = paginator.ending_before(eb);
        }

        paginator
    }

    /// Convenience helper returning an auto-paginator for line items with default page limit (10).
    pub fn list_line_items_default(&self, invoice_id: &str) -> Paginator<'a, InvoiceLineItem> {
        self.list_line_items(invoice_id, &ListInvoiceLineItemsParams::default())
    }

    /// Convenience helper returning an auto-paginator with an optional page limit.
    pub fn list_line_items_with_limit(
        &self,
        invoice_id: &str,
        limit: Option<u32>,
    ) -> Paginator<'a, InvoiceLineItem> {
        let params = ListInvoiceLineItemsParams {
            limit,
            ..Default::default()
        };
        self.list_line_items(invoice_id, &params)
    }

    /// Streams raw binary PDF bytes for an invoice (`GET /v1/invoices/{id}/pdf`).
    ///
    /// Adheres to Gate G-Borrow by returning zero-copy `bytes::Bytes`.
    pub async fn pdf(&self, invoice_id: &str) -> Result<bytes::Bytes, StripeError> {
        let safe_id = validate_and_encode_id(invoice_id)?;
        let url = format!(
            "{}/v1/invoices/{}/pdf",
            self.client.config.api_base, safe_id
        );

        let req = self.client.http.get(&url);
        self.client.execute_raw_bytes(req).await
    }
}
