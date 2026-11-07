//! Stripe Terminal POS resources implementation (`stripe.terminal`).
//!
//! Provides bindings for managing in-person Terminal point-of-sale hardware:
//! - Readers (`stripe.terminal.readers`): Device registration, status monitoring, and action cancellation.
//! - Locations (`stripe.terminal.locations`): Physical store locations and address management.
//! - Connection Tokens (`stripe.terminal.connection_tokens`): Ephemeral credentials for Terminal SDK reader connection.
//!
//! # Architecture & Quality Invariants
//! - **AC-US-7.2.0-03**: Secure ephemeral connection token issuance with location binding.
//! - **Gate G-Borrow**: Zero-copy borrowing across string slices and parameter models.
//! - **Gate G2 (Security Gateway)**: Automatic RFC 4122 UUID v4 `Idempotency-Key` header injection on
//!   mutable requests (`create`, `cancel_action`), guarding against double registration and duplicate token generation.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 5$ across all resource methods with early guard returns.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//! use some_stripe::types::terminal::{
//!     CreateConnectionTokenParams, CreateLocationParams, CreateReaderParams,
//! };
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//!
//! // 1. Create a physical retail location
//! let location = client.terminal().locations().create(&CreateLocationParams::new("Downtown Store")).await?;
//!
//! // 2. Generate an ephemeral connection token scoped to that location (AC-US-7.2.0-03)
//! let token = client.terminal().connection_tokens().create(&CreateConnectionTokenParams::for_location(&location.id)).await?;
//! assert!(!token.secret.is_empty());
//!
//! // 3. Register an in-person POS smart reader
//! let reader = client.terminal().readers().create(
//!     &CreateReaderParams::new("simulated-wpos-1")
//!         .with_label("Register 1")
//!         .with_location(&location.id)
//! ).await?;
//! assert!(reader.is_online());
//! # Ok(())
//! # }
//! ```

use some_stripe_core::patterns::serializer::{encode_form_pairs, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::terminal::{
    ConnectionToken, CreateConnectionTokenParams, CreateLocationParams, CreateReaderParams,
    ListTerminalLocationsParams, ListTerminalReadersParams, TerminalLocation, TerminalLocationList,
    TerminalReader, TerminalReaderList,
};

/// Root resource for Stripe Terminal operations (`stripe.terminal`).
///
/// Provides access to Readers, Locations, and Connection Tokens.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let terminal = client.terminal();
/// let _readers = terminal.readers();
/// let _locations = terminal.locations();
/// let _tokens = terminal.connection_tokens();
/// ```
#[derive(Clone, Copy)]
pub struct TerminalResource<'a> {
    /// Terminal Readers resource (`stripe.terminal.readers`).
    pub readers: TerminalReadersResource<'a>,
    /// Terminal Locations resource (`stripe.terminal.locations`).
    pub locations: TerminalLocationsResource<'a>,
    /// Terminal Connection Tokens resource (`stripe.terminal.connection_tokens`).
    pub connection_tokens: TerminalConnectionTokensResource<'a>,
    client: &'a StripeClient,
}

impl<'a> TerminalResource<'a> {
    /// Creates a new `TerminalResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            readers: TerminalReadersResource::new(client),
            locations: TerminalLocationsResource::new(client),
            connection_tokens: TerminalConnectionTokensResource::new(client),
            client,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the Terminal Readers resource (`stripe.terminal.readers`).
    pub fn readers(&self) -> &TerminalReadersResource<'a> {
        &self.readers
    }

    /// Access the Terminal Locations resource (`stripe.terminal.locations`).
    pub fn locations(&self) -> &TerminalLocationsResource<'a> {
        &self.locations
    }

    /// Access the Terminal Connection Tokens resource (`stripe.terminal.connection_tokens`).
    pub fn connection_tokens(&self) -> &TerminalConnectionTokensResource<'a> {
        &self.connection_tokens
    }
}

/// Resource for managing Stripe Terminal Readers (`stripe.terminal.readers`).
#[derive(Clone, Copy)]
pub struct TerminalReadersResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> TerminalReadersResource<'a> {
    /// Creates a new `TerminalReadersResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account.
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Registers a new Terminal Reader device (`POST /v1/terminal/readers`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if registration code or location ID validation fails.
    /// - Returns [`StripeError::Api`] if the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized.
    pub async fn create(&self, params: &CreateReaderParams) -> Result<TerminalReader, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v1/terminal/readers", self.client.config.api_base);
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let reader: TerminalReader = serde_json::from_value(val)?;
        Ok(reader)
    }

    /// Retrieves an existing Terminal Reader (`GET /v1/terminal/readers/{id}`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if `reader_id` is invalid.
    /// - Returns [`StripeError::Api`] if the reader does not exist or API fails.
    pub async fn retrieve(&self, reader_id: &str) -> Result<TerminalReader, StripeError> {
        validate_and_encode_id(reader_id)?;

        let url = format!(
            "{}/v1/terminal/readers/{}",
            self.client.config.api_base, reader_id
        );
        let mut req = self.client.http.get(&url);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let reader: TerminalReader = serde_json::from_value(val)?;
        Ok(reader)
    }

    /// Lists Terminal Readers with optional filtering (`GET /v1/terminal/readers`).
    ///
    /// # Errors
    /// - Returns [`StripeError::Api`] if the request fails.
    pub async fn list(
        &self,
        params: &ListTerminalReadersParams,
    ) -> Result<TerminalReaderList, StripeError> {
        let query_data = to_form_params(params)?;
        let url = if query_data.is_empty() {
            format!("{}/v1/terminal/readers", self.client.config.api_base)
        } else {
            let query_str = encode_form_pairs(&query_data);
            format!(
                "{}/v1/terminal/readers?{query_str}",
                self.client.config.api_base
            )
        };

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let list: TerminalReaderList = serde_json::from_value(val)?;
        Ok(list)
    }

    /// Cancels the current action on a Terminal Reader (`POST /v1/terminal/readers/{id}/cancel_action`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if `reader_id` is invalid.
    /// - Returns [`StripeError::Api`] if cancellation fails.
    pub async fn cancel_action(&self, reader_id: &str) -> Result<TerminalReader, StripeError> {
        validate_and_encode_id(reader_id)?;

        let url = format!(
            "{}/v1/terminal/readers/{}/cancel_action",
            self.client.config.api_base, reader_id
        );
        let idempotency_key = super::resolve_idempotency_key(None);

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let reader: TerminalReader = serde_json::from_value(val)?;
        Ok(reader)
    }
}

/// Resource for managing Stripe Terminal Locations (`stripe.terminal.locations`).
#[derive(Clone, Copy)]
pub struct TerminalLocationsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> TerminalLocationsResource<'a> {
    /// Creates a new `TerminalLocationsResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account.
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Creates a new physical Terminal Location (`POST /v1/terminal/locations`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if display name validation fails.
    /// - Returns [`StripeError::Api`] if creation fails.
    pub async fn create(
        &self,
        params: &CreateLocationParams,
    ) -> Result<TerminalLocation, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v1/terminal/locations", self.client.config.api_base);
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let location: TerminalLocation = serde_json::from_value(val)?;
        Ok(location)
    }

    /// Retrieves a Terminal Location (`GET /v1/terminal/locations/{id}`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if `location_id` is invalid.
    /// - Returns [`StripeError::Api`] if the location does not exist.
    pub async fn retrieve(&self, location_id: &str) -> Result<TerminalLocation, StripeError> {
        validate_and_encode_id(location_id)?;

        let url = format!(
            "{}/v1/terminal/locations/{}",
            self.client.config.api_base, location_id
        );
        let mut req = self.client.http.get(&url);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let location: TerminalLocation = serde_json::from_value(val)?;
        Ok(location)
    }

    /// Lists Terminal Locations (`GET /v1/terminal/locations`).
    ///
    /// # Errors
    /// - Returns [`StripeError::Api`] if the list request fails.
    pub async fn list(
        &self,
        params: &ListTerminalLocationsParams,
    ) -> Result<TerminalLocationList, StripeError> {
        let query_data = to_form_params(params)?;
        let url = if query_data.is_empty() {
            format!("{}/v1/terminal/locations", self.client.config.api_base)
        } else {
            let query_str = encode_form_pairs(&query_data);
            format!(
                "{}/v1/terminal/locations?{query_str}",
                self.client.config.api_base
            )
        };

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let list: TerminalLocationList = serde_json::from_value(val)?;
        Ok(list)
    }
}

/// Resource for generating Stripe Terminal Connection Tokens (`stripe.terminal.connection_tokens`).
#[derive(Clone, Copy)]
pub struct TerminalConnectionTokensResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> TerminalConnectionTokensResource<'a> {
    /// Creates a new `TerminalConnectionTokensResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account.
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Generates an ephemeral connection token for Terminal SDK reader connection (`POST /v1/terminal/connection_tokens`).
    ///
    /// Satisfies **AC-US-7.2.0-03**: Generating ephemeral connection token with optional location binding.
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if location validation fails.
    /// - Returns [`StripeError::Api`] if the Stripe API returns an error.
    pub async fn create(
        &self,
        params: &CreateConnectionTokenParams,
    ) -> Result<ConnectionToken, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/terminal/connection_tokens",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let token: ConnectionToken = serde_json::from_value(val)?;
        Ok(token)
    }
}
