//! Stripe Tax Registrations resource implementation.
//!
//! Provides management of regional tax nexus registrations (`/v1/tax/registrations`).
//! Adheres to:
//! - **Gate G-Borrow**: Zero-clone borrowing discipline with streaming auto-pagination via [`Paginator`].
//! - **Gate G-Law**: EU VAT Directive 2006/112/EC and international tax nexus compliance.
//! - **Gate G-Docs**: Runnable doctests and comprehensive rustdoc on public APIs.
//! - **Gate G-Complexity**: Strict cyclomatic complexity <= 6.

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::patterns::{encode_form_pairs, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateTaxRegistrationParams, ListTaxRegistrationsParams, TaxRegistration, TaxRegistrationList,
    UpdateTaxRegistrationParams,
};

/// Key extractor function extracting object ID from [`TaxRegistration`] without cloning.
pub fn tax_registration_id(reg: &TaxRegistration) -> &str {
    reg.id()
}

/// Resource for managing Stripe Tax Registrations (`/v1/tax/registrations`).
///
/// Tax Registrations inform Stripe where the account has legal tax obligations,
/// determining jurisdictions where taxes must be actively collected.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
/// use some_stripe::resources::TaxRegistrationsResource;
///
/// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
/// let client = StripeClient::new("sk_test_123");
/// let registrations_resource = TaxRegistrationsResource::new(&client);
/// let registration = registrations_resource.retrieve("taxreg_123").await?;
/// println!("Registration country: {}", registration.country());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy)]
pub struct TaxRegistrationsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> TaxRegistrationsResource<'a> {
    /// Creates a new `TaxRegistrationsResource` bound to the provided client.
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Borrows the inner `StripeClient`.
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account
    /// by attaching the `Stripe-Account` tenant header to all outbound requests.
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Creates a new tax registration (`POST /v1/tax/registrations`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if head office address is missing or country options are malformed.
    pub async fn create(
        &self,
        params: &CreateTaxRegistrationParams,
    ) -> Result<TaxRegistration, StripeError> {
        let url = format!("{}/v1/tax/registrations", self.client.config.api_base);
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(None);

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
        let reg: TaxRegistration = serde_json::from_value(val)?;
        Ok(reg)
    }

    /// Retrieves an existing tax registration by ID (`GET /v1/tax/registrations/{id}`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if the ID is invalid or registration is not found.
    pub async fn retrieve(&self, id: &str) -> Result<TaxRegistration, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/tax/registrations/{encoded}",
            self.client.config.api_base
        );
        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let reg: TaxRegistration = serde_json::from_value(val)?;
        Ok(reg)
    }

    /// Updates an existing tax registration (`POST /v1/tax/registrations/{id}`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if the ID is invalid or registration update fails.
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateTaxRegistrationParams,
    ) -> Result<TaxRegistration, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/tax/registrations/{encoded}",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(None);

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
        let reg: TaxRegistration = serde_json::from_value(val)?;
        Ok(reg)
    }

    /// Returns a streaming, zero-clone auto-paginator for tax registrations (`GET /v1/tax/registrations`).
    pub fn list(&self, params: &ListTaxRegistrationsParams) -> Paginator<'a, TaxRegistration> {
        let (base_path, initial_err) = match to_form_params(params) {
            Ok(mut pairs) => {
                pairs
                    .retain(|(k, _)| k != "limit" && k != "starting_after" && k != "ending_before");
                let path = if pairs.is_empty() {
                    "v1/tax/registrations".to_string()
                } else {
                    format!("v1/tax/registrations?{}", encode_form_pairs(&pairs))
                };
                (path, None)
            }
            Err(e) => ("v1/tax/registrations".to_string(), Some(e)),
        };

        let mut paginator = Paginator::new(
            self.client,
            base_path,
            params.limit.unwrap_or(10),
            tax_registration_id,
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

    /// Retrieves a single page of tax registrations (`GET /v1/tax/registrations`).
    pub async fn list_page(
        &self,
        params: Option<&ListTaxRegistrationsParams>,
    ) -> Result<TaxRegistrationList, StripeError> {
        let url = match params {
            Some(p) => {
                let query_data = to_form_params(p)?;
                if query_data.is_empty() {
                    format!("{}/v1/tax/registrations", self.client.config.api_base)
                } else {
                    let query_str = encode_form_pairs(&query_data);
                    format!(
                        "{}/v1/tax/registrations?{query_str}",
                        self.client.config.api_base
                    )
                }
            }
            None => format!("{}/v1/tax/registrations", self.client.config.api_base),
        };

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let list: TaxRegistrationList = serde_json::from_value(val)?;
        Ok(list)
    }
}
