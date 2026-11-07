//! Stripe Treasury Financial Accounts resource implementation (`POST /v1/treasury/financial_accounts`).
//!
//! Provides CRUD operations and lifecycle actions for commercial bank-replacement financial accounts,
//! balance monitoring, feature toggling, and account closure.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CloseFinancialAccountParams, CreateFinancialAccountParams, FinancialAccount,
    FinancialAccountList, ListFinancialAccountsParams, UpdateFinancialAccountParams,
};

/// Resource for managing Stripe Treasury Financial Accounts (`stripe.treasury().financial_accounts()`
/// or `stripe.treasury_financial_accounts()`).
#[derive(Clone, Copy)]
pub struct TreasuryFinancialAccountsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> TreasuryFinancialAccountsResource<'a> {
    /// Creates a new resource handle wrapping the microkernel [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Returns the underlying [`StripeClient`] reference.
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account
    /// by attaching the `Stripe-Account` tenant header to all outbound requests.
    #[must_use]
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Creates a new Treasury Financial Account.
    ///
    /// # Errors
    /// Returns [`StripeError`] if parameter validation fails, network transport errors occur,
    /// or the Stripe API returns a non-2xx status code.
    pub async fn create(
        &self,
        params: &CreateFinancialAccountParams,
    ) -> Result<FinancialAccount, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/treasury/financial_accounts",
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
        let fa: FinancialAccount = serde_json::from_value(val)?;
        Ok(fa)
    }

    /// Retrieves an existing Treasury Financial Account by ID.
    ///
    /// # Errors
    /// Returns [`StripeError`] if the ID is invalid, or the resource is not found.
    pub async fn retrieve(&self, id: &str) -> Result<FinancialAccount, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/treasury/financial_accounts/{encoded}",
            self.client.config.api_base
        );

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let fa: FinancialAccount = serde_json::from_value(val)?;
        Ok(fa)
    }

    /// Updates metadata, features, or platform restrictions on an existing Financial Account.
    ///
    /// # Errors
    /// Returns [`StripeError`] on validation failure or API error.
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateFinancialAccountParams,
    ) -> Result<FinancialAccount, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/treasury/financial_accounts/{encoded}",
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
        let fa: FinancialAccount = serde_json::from_value(val)?;
        Ok(fa)
    }

    /// Lists Treasury Financial Accounts with optional pagination filters.
    ///
    /// # Errors
    /// Returns [`StripeError`] on invalid pagination parameters or API error.
    pub async fn list(
        &self,
        params: Option<&ListFinancialAccountsParams>,
    ) -> Result<FinancialAccountList, StripeError> {
        if let Some(p) = params {
            p.validate()
                .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
        }

        let mut url = format!(
            "{}/v1/treasury/financial_accounts",
            self.client.config.api_base
        );
        let query_params = build_list_query(params);
        append_query_string(&mut url, &query_params);

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let list: FinancialAccountList = serde_json::from_value(val)?;
        Ok(list)
    }

    /// Closes a Treasury Financial Account, optionally routing residual funds.
    ///
    /// # Errors
    /// Returns [`StripeError`] if closing fails or forwarding settings are invalid.
    pub async fn close(
        &self,
        id: &str,
        params: Option<&CloseFinancialAccountParams>,
    ) -> Result<FinancialAccount, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        if let Some(p) = params {
            p.validate()
                .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
        }

        let url = format!(
            "{}/v1/treasury/financial_accounts/{encoded}/close",
            self.client.config.api_base
        );

        let form_data = match params {
            Some(p) => to_form_params(p)?,
            None => to_form_params(&CloseFinancialAccountParams::new())?,
        };

        let idempotency_key =
            super::resolve_idempotency_key(params.and_then(|p| p.idempotency_key.as_deref()));

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
        let fa: FinancialAccount = serde_json::from_value(val)?;
        Ok(fa)
    }
}

fn push_query_param(
    query: &mut Vec<(&'static str, String)>,
    key: &'static str,
    value: Option<String>,
) {
    if let Some(v) = value {
        query.push((key, v));
    }
}

fn build_list_query(params: Option<&ListFinancialAccountsParams>) -> Vec<(&'static str, String)> {
    let mut query = Vec::new();
    if let Some(p) = params {
        push_query_param(&mut query, "limit", p.limit.map(|l| l.to_string()));
        push_query_param(&mut query, "status", p.status.map(|s| s.to_string()));
        push_query_param(&mut query, "created", p.created.map(|c| c.to_string()));
        push_query_param(&mut query, "starting_after", p.starting_after.clone());
        push_query_param(&mut query, "ending_before", p.ending_before.clone());
    }
    query
}

fn append_query_string(url: &mut String, query_params: &[(&'static str, String)]) {
    if query_params.is_empty() {
        return;
    }
    let encoded_pairs: Vec<String> = query_params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    url.push('?');
    url.push_str(&encoded_pairs.join("&"));
}
