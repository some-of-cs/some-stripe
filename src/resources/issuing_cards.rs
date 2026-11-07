//! Stripe Issuing Cards & Cardholders resource implementation.
//!
//! Provides management of virtual and physical payment cards, cardholders,
//! real-time spending controls, and PCI-DSS SAQ A PAN isolation.

use some_stripe_core::patterns::{encode_form_pairs, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateCardParams, CreateCardholderParams, IssuingCard, IssuingCardList, IssuingCardholder,
    IssuingCardholderList, ListIssuingCardholdersParams, ListIssuingCardsParams, UpdateCardParams,
    UpdateCardholderParams,
};

/// Helper to build a request with optional `Stripe-Account` tenant header and `Idempotency-Key`.
fn build_post_request<'a>(
    client: &'a StripeClient,
    url: &str,
    form_data: &[(String, String)],
    idempotency_key: &str,
    account: Option<&'a str>,
) -> reqwest::RequestBuilder {
    let mut req = client
        .http
        .post(url)
        .form(form_data)
        .header("Idempotency-Key", idempotency_key);
    if let Some(acct) = account {
        req = req.header("Stripe-Account", acct);
    }
    req
}

/// Helper to build a GET request with optional `Stripe-Account` tenant header.
fn build_get_request<'a>(
    client: &'a StripeClient,
    url: &str,
    account: Option<&'a str>,
) -> reqwest::RequestBuilder {
    let mut req = client.http.get(url);
    if let Some(acct) = account {
        req = req.header("Stripe-Account", acct);
    }
    req
}

/// Helper to build a URL with optional query parameters.
fn build_list_url<T: serde::Serialize>(
    base_url: &str,
    params: Option<&T>,
) -> Result<String, StripeError> {
    match params {
        Some(p) => {
            let query_data = to_form_params(p)?;
            if query_data.is_empty() {
                Ok(base_url.to_string())
            } else {
                let query_str = encode_form_pairs(&query_data);
                Ok(format!("{base_url}?{query_str}"))
            }
        }
        None => Ok(base_url.to_string()),
    }
}

/// Resource for managing Stripe Issuing Cards (`stripe.issuing().cards()`).
#[derive(Clone, Copy)]
pub struct IssuingCardsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> IssuingCardsResource<'a> {
    /// Creates a new `IssuingCardsResource`.
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Returns the underlying StripeClient reference.
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a connected account.
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Creates a new Issuing card.
    ///
    /// # Errors
    /// Returns [`StripeError`] if serialization fails or the API rejects the request.
    pub async fn create(&self, params: &CreateCardParams) -> Result<IssuingCard, StripeError> {
        let url = format!("{}/v1/issuing/cards", self.client.config.api_base);
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = build_post_request(
            self.client,
            &url,
            &form_data,
            idempotency_key.as_ref(),
            self.account,
        );
        let val = self.client.execute(req).await?;
        let card: IssuingCard = serde_json::from_value(val)?;
        Ok(card)
    }

    /// Retrieves an Issuing card by ID.
    ///
    /// # Errors
    /// Returns [`StripeError`] if the ID is invalid or the API request fails.
    pub async fn retrieve(&self, id: &str) -> Result<IssuingCard, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!("{}/v1/issuing/cards/{encoded}", self.client.config.api_base);
        let req = build_get_request(self.client, &url, self.account);
        let val = self.client.execute(req).await?;
        let card: IssuingCard = serde_json::from_value(val)?;
        Ok(card)
    }

    /// Updates an existing Issuing card.
    ///
    /// # Errors
    /// Returns [`StripeError`] if the ID is invalid or the API request fails.
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateCardParams,
    ) -> Result<IssuingCard, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!("{}/v1/issuing/cards/{encoded}", self.client.config.api_base);
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = build_post_request(
            self.client,
            &url,
            &form_data,
            idempotency_key.as_ref(),
            self.account,
        );
        let val = self.client.execute(req).await?;
        let card: IssuingCard = serde_json::from_value(val)?;
        Ok(card)
    }

    /// Lists Issuing cards with optional filters.
    ///
    /// # Errors
    /// Returns [`StripeError`] if parameter serialization or network request fails.
    pub async fn list(
        &self,
        params: Option<&ListIssuingCardsParams>,
    ) -> Result<IssuingCardList, StripeError> {
        let base_url = format!("{}/v1/issuing/cards", self.client.config.api_base);
        let url = build_list_url(&base_url, params)?;
        let req = build_get_request(self.client, &url, self.account);
        let val = self.client.execute(req).await?;
        let list: IssuingCardList = serde_json::from_value(val)?;
        Ok(list)
    }
}

/// Resource for managing Stripe Issuing Cardholders (`stripe.issuing().cardholders()`).
#[derive(Clone, Copy)]
pub struct IssuingCardholdersResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> IssuingCardholdersResource<'a> {
    /// Creates a new `IssuingCardholdersResource`.
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Returns the underlying StripeClient reference.
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a connected account.
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Creates a new Issuing cardholder.
    ///
    /// # Errors
    /// Returns [`StripeError`] if serialization fails or the API rejects the request.
    pub async fn create(
        &self,
        params: &CreateCardholderParams,
    ) -> Result<IssuingCardholder, StripeError> {
        let url = format!("{}/v1/issuing/cardholders", self.client.config.api_base);
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = build_post_request(
            self.client,
            &url,
            &form_data,
            idempotency_key.as_ref(),
            self.account,
        );
        let val = self.client.execute(req).await?;
        let cardholder: IssuingCardholder = serde_json::from_value(val)?;
        Ok(cardholder)
    }

    /// Retrieves an Issuing cardholder by ID.
    ///
    /// # Errors
    /// Returns [`StripeError`] if the ID is invalid or the API request fails.
    pub async fn retrieve(&self, id: &str) -> Result<IssuingCardholder, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/issuing/cardholders/{encoded}",
            self.client.config.api_base
        );
        let req = build_get_request(self.client, &url, self.account);
        let val = self.client.execute(req).await?;
        let cardholder: IssuingCardholder = serde_json::from_value(val)?;
        Ok(cardholder)
    }

    /// Updates an existing Issuing cardholder.
    ///
    /// # Errors
    /// Returns [`StripeError`] if the ID is invalid or the API request fails.
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateCardholderParams,
    ) -> Result<IssuingCardholder, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/issuing/cardholders/{encoded}",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = build_post_request(
            self.client,
            &url,
            &form_data,
            idempotency_key.as_ref(),
            self.account,
        );
        let val = self.client.execute(req).await?;
        let cardholder: IssuingCardholder = serde_json::from_value(val)?;
        Ok(cardholder)
    }

    /// Lists Issuing cardholders with optional filters.
    ///
    /// # Errors
    /// Returns [`StripeError`] if parameter serialization or network request fails.
    pub async fn list(
        &self,
        params: Option<&ListIssuingCardholdersParams>,
    ) -> Result<IssuingCardholderList, StripeError> {
        let base_url = format!("{}/v1/issuing/cardholders", self.client.config.api_base);
        let url = build_list_url(&base_url, params)?;
        let req = build_get_request(self.client, &url, self.account);
        let val = self.client.execute(req).await?;
        let list: IssuingCardholderList = serde_json::from_value(val)?;
        Ok(list)
    }
}

/// Namespace providing access to Issuing resources.
#[derive(Clone, Copy)]
pub struct IssuingResource<'a> {
    client: &'a StripeClient,
}

impl<'a> IssuingResource<'a> {
    /// Creates a new `IssuingResource`.
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access Issuing Cards operations.
    pub fn cards(&self) -> IssuingCardsResource<'a> {
        IssuingCardsResource::new(self.client)
    }

    /// Access Issuing Cardholders operations.
    pub fn cardholders(&self) -> IssuingCardholdersResource<'a> {
        IssuingCardholdersResource::new(self.client)
    }

    /// Access Issuing Authorizations operations.
    pub fn authorizations(
        &self,
    ) -> super::issuing_authorizations::IssuingAuthorizationsResource<'a> {
        super::issuing_authorizations::IssuingAuthorizationsResource::new(self.client)
    }

    /// Access Issuing Transactions operations.
    pub fn transactions(&self) -> super::issuing_transactions::IssuingTransactionsResource<'a> {
        super::issuing_transactions::IssuingTransactionsResource::new(self.client)
    }
}
