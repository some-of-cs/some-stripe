//! Stripe Quotes resource implementations.
//!
//! Provides endpoints to create, retrieve, update, finalize, accept, cancel, list,
//! and stream PDF documents for Quotes, adhering to RFC 4122 UUID v4 idempotency
//! and zero-copy binary streaming (`bytes::Bytes`).

use some_stripe_core::patterns::{Paginator, encode_form_pairs, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    AcceptQuoteParams, CancelQuoteParams, CreateQuoteParams, FinalizeQuoteParams, ListQuotesParams,
    Quote, QuoteList, UpdateQuoteParams,
};

/// Key extractor function for [`Quote`] in paginated streams.
pub fn quote_id(quote: &Quote) -> &str {
    quote.id.as_str()
}

/// Resource for managing Stripe Quotes (`stripe.quotes()`).
#[derive(Clone, Copy)]
pub struct QuotesResource<'a> {
    client: &'a StripeClient,
}

impl<'a> QuotesResource<'a> {
    /// Creates a new `QuotesResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a new quote in draft mode (`POST /v1/quotes`).
    pub async fn create(&self, params: &CreateQuoteParams) -> Result<Quote, StripeError> {
        let url = format!("{}/v1/quotes", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let quote: Quote = serde_json::from_value(val)?;
        Ok(quote)
    }

    /// Retrieves an existing quote by ID (`GET /v1/quotes/{id}`).
    pub async fn retrieve(&self, quote_id: &str) -> Result<Quote, StripeError> {
        let safe_id = validate_and_encode_id(quote_id)?;
        let url = format!("{}/v1/quotes/{}", self.client.config.api_base, safe_id);

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let quote: Quote = serde_json::from_value(val)?;
        Ok(quote)
    }

    /// Updates draft quote details (`POST /v1/quotes/{id}`).
    pub async fn update(
        &self,
        quote_id: &str,
        params: &UpdateQuoteParams,
    ) -> Result<Quote, StripeError> {
        let safe_id = validate_and_encode_id(quote_id)?;
        let url = format!("{}/v1/quotes/{}", self.client.config.api_base, safe_id);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let quote: Quote = serde_json::from_value(val)?;
        Ok(quote)
    }

    /// Finalizes a draft quote, transitioning its status to `open` (`POST /v1/quotes/{id}/finalize`).
    pub async fn finalize_quote(
        &self,
        quote_id: &str,
        params: Option<&FinalizeQuoteParams>,
    ) -> Result<Quote, StripeError> {
        let safe_id = validate_and_encode_id(quote_id)?;
        let url = format!(
            "{}/v1/quotes/{}/finalize",
            self.client.config.api_base, safe_id
        );
        let mut form_data = Vec::new();

        let exp_str = params.and_then(|p| p.expires_at).map(|e| e.to_string());
        if let Some(ref e) = exp_str {
            form_data.push(("expires_at", e.as_str()));
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
        let quote: Quote = serde_json::from_value(val)?;
        Ok(quote)
    }

    /// Accepts an open quote on behalf of customer (`POST /v1/quotes/{id}/accept`).
    pub async fn accept(
        &self,
        quote_id: &str,
        params: Option<&AcceptQuoteParams>,
    ) -> Result<Quote, StripeError> {
        let safe_id = validate_and_encode_id(quote_id)?;
        let url = format!(
            "{}/v1/quotes/{}/accept",
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
        let quote: Quote = serde_json::from_value(val)?;
        Ok(quote)
    }

    /// Cancels a quote (`POST /v1/quotes/{id}/cancel`).
    pub async fn cancel(
        &self,
        quote_id: &str,
        params: Option<&CancelQuoteParams>,
    ) -> Result<Quote, StripeError> {
        let safe_id = validate_and_encode_id(quote_id)?;
        let url = format!(
            "{}/v1/quotes/{}/cancel",
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
        let quote: Quote = serde_json::from_value(val)?;
        Ok(quote)
    }

    /// Streams raw binary PDF bytes for a quote (`GET /v1/quotes/{id}/pdf`).
    ///
    /// Adheres to Gate G-Borrow by returning zero-copy `bytes::Bytes`.
    pub async fn pdf(&self, quote_id: &str) -> Result<bytes::Bytes, StripeError> {
        let safe_id = validate_and_encode_id(quote_id)?;
        let url = format!("{}/v1/quotes/{}/pdf", self.client.config.api_base, safe_id);

        let req = self.client.http.get(&url);
        self.client.execute_raw_bytes(req).await
    }

    /// Lists quotes (`GET /v1/quotes`).
    pub async fn list(&self, params: Option<&ListQuotesParams>) -> Result<QuoteList, StripeError> {
        let url = match params {
            Some(p) => {
                let query_data = to_form_params(p)?;
                if query_data.is_empty() {
                    format!("{}/v1/quotes", self.client.config.api_base)
                } else {
                    let query_str = encode_form_pairs(&query_data);
                    format!("{}/v1/quotes?{query_str}", self.client.config.api_base)
                }
            }
            None => format!("{}/v1/quotes", self.client.config.api_base),
        };

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: QuoteList = serde_json::from_value(val)?;
        Ok(list)
    }

    /// Auto-paginator stream for quotes.
    pub fn list_paginated(&self, params: &ListQuotesParams) -> Paginator<'a, Quote> {
        let mut paginator = Paginator::new(
            self.client,
            "v1/quotes".to_string(),
            params.limit.unwrap_or(10),
            quote_id,
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
    pub fn list_default(&self) -> Paginator<'a, Quote> {
        self.list_paginated(&ListQuotesParams::default())
    }
}
