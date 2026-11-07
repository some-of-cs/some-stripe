//! Stripe Issuing Authorizations resource implementations.
//!
//! Provides endpoints to retrieve, update, approve, decline, and list card authorizations
//! (`/v1/issuing/authorizations`), enforcing strict input sanitization, zero-clone borrowing,
//! and Bank Secrecy Act / AML compliance.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    ApproveAuthorizationParams, DeclineAuthorizationParams, IssuingAuthorization,
    IssuingAuthorizationList, ListIssuingAuthorizationsParams, UpdateIssuingAuthorizationParams,
};

/// Resource for managing Stripe Issuing Authorizations (`stripe.issuing().authorizations()` or direct).
///
/// Wraps endpoints for retrieving authorizations (`GET /v1/issuing/authorizations/{id}`),
/// updating metadata (`POST /v1/issuing/authorizations/{id}`), approving pending authorizations
/// (`POST /v1/issuing/authorizations/{id}/approve`), declining pending authorizations
/// (`POST /v1/issuing/authorizations/{id}/decline`), and listing authorizations
/// (`GET /v1/issuing/authorizations`).
#[derive(Clone, Copy)]
pub struct IssuingAuthorizationsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> IssuingAuthorizationsResource<'a> {
    /// Creates a new `IssuingAuthorizationsResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Sets the `Stripe-Account` header for multi-tenant Connect operations.
    #[must_use]
    pub fn with_account(mut self, account: &'a str) -> Self {
        self.account = Some(account);
        self
    }

    /// Returns a reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Retrieves an existing issuing authorization (`GET /v1/issuing/authorizations/{id}`).
    ///
    /// Validates `id` to prevent path traversal and injection attacks before transmitting.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` is empty, contains whitespace,
    ///   or includes invalid characters such as `'..'`, `'/'`, `'\\'`, `'?'`, or `'#'`.
    /// - Returns [`StripeError::Api`] if the authorization does not exist or the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response body cannot be deserialized into an [`IssuingAuthorization`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let auth = client.issuing_authorizations().retrieve("iauth_12345").await?;
    ///
    /// println!("Authorization ID: {}", auth.id);
    /// println!("Amount: {} {}", auth.amount, auth.currency);
    /// println!("Approved: {}", auth.approved);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<IssuingAuthorization, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/issuing/authorizations/{encoded}",
            self.client.config.api_base
        );
        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let auth: IssuingAuthorization = serde_json::from_value(val)?;
        Ok(auth)
    }

    /// Updates specified properties of an existing issuing authorization (`POST /v1/issuing/authorizations/{id}`).
    ///
    /// Primarily used to attach or update custom key-value `metadata`.
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` fails path traversal checks.
    /// - Returns [`StripeError::Api`] if the API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into an [`IssuingAuthorization`].
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateIssuingAuthorizationParams,
    ) -> Result<IssuingAuthorization, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/issuing/authorizations/{encoded}",
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
        let auth: IssuingAuthorization = serde_json::from_value(val)?;
        Ok(auth)
    }

    /// Approves a pending issuing authorization (`POST /v1/issuing/authorizations/{id}/approve`).
    ///
    /// Allows setting a custom authorized hold amount if `pending_request.is_amount_controllable` is true.
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` fails path traversal checks.
    /// - Returns [`StripeError::Api`] if the authorization is not in a pending state or cannot be approved.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into an [`IssuingAuthorization`].
    pub async fn approve(
        &self,
        id: &str,
        params: Option<&ApproveAuthorizationParams>,
    ) -> Result<IssuingAuthorization, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/issuing/authorizations/{encoded}/approve",
            self.client.config.api_base
        );

        let form_data = match params {
            Some(p) => to_form_params(p)?,
            None => Vec::new(),
        };

        let explicit_idempotency = params.and_then(|p| p.idempotency_key.as_deref());
        let idempotency_key = super::resolve_idempotency_key(explicit_idempotency);

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
        let auth: IssuingAuthorization = serde_json::from_value(val)?;
        Ok(auth)
    }

    /// Declines a pending issuing authorization (`POST /v1/issuing/authorizations/{id}/decline`).
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` fails path traversal checks.
    /// - Returns [`StripeError::Api`] if the authorization is not in a pending state or cannot be declined.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into an [`IssuingAuthorization`].
    pub async fn decline(
        &self,
        id: &str,
        params: Option<&DeclineAuthorizationParams>,
    ) -> Result<IssuingAuthorization, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/issuing/authorizations/{encoded}/decline",
            self.client.config.api_base
        );

        let form_data = match params {
            Some(p) => to_form_params(p)?,
            None => Vec::new(),
        };

        let explicit_idempotency = params.and_then(|p| p.idempotency_key.as_deref());
        let idempotency_key = super::resolve_idempotency_key(explicit_idempotency);

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
        let auth: IssuingAuthorization = serde_json::from_value(val)?;
        Ok(auth)
    }

    /// Lists issuing authorizations matching the specified query criteria (`GET /v1/issuing/authorizations`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::Api`] if the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into an [`IssuingAuthorizationList`].
    pub async fn list(
        &self,
        params: Option<&ListIssuingAuthorizationsParams>,
    ) -> Result<IssuingAuthorizationList, StripeError> {
        let mut url = format!("{}/v1/issuing/authorizations", self.client.config.api_base);
        let query_entries = build_authorizations_query(params);
        if !query_entries.is_empty() {
            let serialized = query_entries
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            url.push('?');
            url.push_str(&serialized);
        }

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let list: IssuingAuthorizationList = serde_json::from_value(val)?;
        Ok(list)
    }
}

/// Builds flat key-value pairs for listing issuing authorizations with low branching complexity.
fn build_authorizations_query(
    params: Option<&ListIssuingAuthorizationsParams>,
) -> Vec<(&'static str, String)> {
    let mut query = Vec::new();
    let Some(p) = params else {
        return query;
    };
    if let Some(ref card) = p.card {
        query.push(("card", card.clone()));
    }
    if let Some(ref cardholder) = p.cardholder {
        query.push(("cardholder", cardholder.clone()));
    }
    if let Some(status) = p.status {
        query.push(("status", status.to_string()));
    }
    if let Some(created) = p.created {
        query.push(("created", created.to_string()));
    }
    if let Some(limit) = p.limit {
        query.push(("limit", limit.to_string()));
    }
    if let Some(ref after) = p.starting_after {
        query.push(("starting_after", after.clone()));
    }
    if let Some(ref before) = p.ending_before {
        query.push(("ending_before", before.clone()));
    }
    query
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_authorizations_query_empty() {
        let q = build_authorizations_query(None);
        assert!(q.is_empty());
    }

    #[test]
    fn test_build_authorizations_query_populated() {
        use some_stripe_types::IssuingAuthorizationStatus;

        let params = ListIssuingAuthorizationsParams::new()
            .with_card("ic_card_abc")
            .with_cardholder("ich_holder_xyz")
            .with_status(IssuingAuthorizationStatus::Pending)
            .with_limit(10)
            .with_starting_after("iauth_prev");

        let q = build_authorizations_query(Some(&params));
        assert_eq!(q.len(), 5);
        assert!(q.contains(&("card", "ic_card_abc".to_string())));
        assert!(q.contains(&("cardholder", "ich_holder_xyz".to_string())));
        assert!(q.contains(&("status", "pending".to_string())));
        assert!(q.contains(&("limit", "10".to_string())));
        assert!(q.contains(&("starting_after", "iauth_prev".to_string())));
    }

    #[test]
    fn test_resource_instantiation() {
        let client = StripeClient::new("sk_test_mock");
        let resource = IssuingAuthorizationsResource::new(&client).with_account("acct_connect_1");
        assert_eq!(resource.account, Some("acct_connect_1"));
    }
}
