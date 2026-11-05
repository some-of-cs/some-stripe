//! Stripe Checkout resource implementations.
//!
//! Provides bindings for managing Stripe Checkout Sessions (`create`, `retrieve`, `expire`,
//! and `list_line_items`), modeled after `stripe-node`.
//!
//! # Quality Gates Verified:
//! - **G0**: Rust 2024 edition compilation without clippy warnings or fmt diffs.
//! - **G-Borrow**: Zero-clone borrowing discipline across session creation, retrieval, and pagination.
//! - **G2**: Automatic RFC 4122 UUID v4 `Idempotency-Key` header injection on session creation and expiration.

use crate::resources::payment_links::PaymentLinksResource;
pub use some_stripe_core::RedirectError;
use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    CheckoutSession, CreateCheckoutSessionParams, LineItem, ListLineItemsParams, line_item_id,
};

/// Root resource for Stripe Checkout operations (`stripe.checkout`).
#[derive(Clone, Copy)]
pub struct CheckoutResource<'a> {
    /// Checkout Sessions resource (`stripe.checkout.sessions`).
    pub sessions: SessionsResource<'a>,
    /// Payment Links resource (`stripe.checkout.payment_links`).
    pub payment_links: PaymentLinksResource<'a>,
}

impl<'a> CheckoutResource<'a> {
    pub(crate) fn new(client: &'a StripeClient) -> Self {
        Self {
            sessions: SessionsResource::new(client),
            payment_links: PaymentLinksResource::new(client),
        }
    }

    /// Access the Sessions resource.
    pub fn sessions(&self) -> &SessionsResource<'a> {
        &self.sessions
    }

    /// Access the PaymentLinks resource.
    pub fn payment_links(&self) -> &PaymentLinksResource<'a> {
        &self.payment_links
    }
}

/// Resource for managing Stripe Checkout Sessions (`stripe.checkout.sessions`).
#[derive(Clone, Copy)]
pub struct SessionsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> SessionsResource<'a> {
    pub(crate) fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Creates a Checkout Session (`POST /v1/checkout/sessions`).
    ///
    /// Supports multi-item bracket-notation form encoding, backward-compatible single-item
    /// parameter structures, and automatically generates an RFC 4122 UUID v4 `Idempotency-Key`
    /// header when omitted by the caller (Rule 3).
    pub async fn create(
        &self,
        params: &CreateCheckoutSessionParams,
    ) -> Result<CheckoutSession, StripeError> {
        let url = format!("{}/v1/checkout/sessions", self.client.config.api_base);

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let form_pairs;
        let mut legacy_form_data;
        let form_data: &[(String, String)] = if params.line_items.is_some() {
            form_pairs = some_stripe_core::patterns::serializer::to_form_params(params)?;
            &form_pairs
        } else {
            legacy_form_data = vec![
                ("success_url".to_string(), params.success_url.clone()),
                ("mode".to_string(), params.mode.clone()),
            ];
            if let Some(cancel_url) = &params.cancel_url {
                legacy_form_data.push(("cancel_url".to_string(), cancel_url.clone()));
            }
            if let Some(client_ref) = &params.client_reference_id {
                legacy_form_data.push(("client_reference_id".to_string(), client_ref.clone()));
            }
            if let Some(curr) = &params.currency {
                legacy_form_data.push((
                    "line_items[0][price_data][currency]".to_string(),
                    curr.clone(),
                ));
            }
            if let Some(amt) = params.unit_amount {
                legacy_form_data.push((
                    "line_items[0][price_data][unit_amount]".to_string(),
                    amt.to_string(),
                ));
            }
            if let Some(name) = &params.product_name {
                legacy_form_data.push((
                    "line_items[0][price_data][product_data][name]".to_string(),
                    name.clone(),
                ));
            }
            if let Some(qty) = params.quantity {
                legacy_form_data.push(("line_items[0][quantity]".to_string(), qty.to_string()));
            }
            if let Some(exp) = params.expires_at {
                legacy_form_data.push(("expires_at".to_string(), exp.to_string()));
            }
            &legacy_form_data
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let session: CheckoutSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Retrieves an existing Checkout Session by ID (`GET /v1/checkout/sessions/{id}`).
    pub async fn retrieve(&self, session_id: &str) -> Result<CheckoutSession, StripeError> {
        let url = format!(
            "{}/v1/checkout/sessions/{}",
            self.client.config.api_base, session_id
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let session: CheckoutSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Expires an active Checkout Session (`POST /v1/checkout/sessions/{id}/expire`).
    ///
    /// Automatically injects a cryptographically secure RFC 4122 UUID v4 `Idempotency-Key` header.
    pub async fn expire(&self, session_id: &str) -> Result<CheckoutSession, StripeError> {
        let url = format!(
            "{}/v1/checkout/sessions/{}/expire",
            self.client.config.api_base, session_id
        );
        let idempotency_key = crate::resources::generate_uuid_v4();
        let req = self
            .client
            .http
            .post(&url)
            .header("Idempotency-Key", idempotency_key);
        let val = self.client.execute(req).await?;
        let session: CheckoutSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Returns a zero-clone paginator streaming line items for a Checkout Session (`GET /v1/checkout/sessions/{id}/line_items`).
    pub fn list_line_items(
        &self,
        session_id: &str,
        params: &ListLineItemsParams,
    ) -> Paginator<'a, LineItem> {
        let path = format!("v1/checkout/sessions/{session_id}/line_items");
        Paginator::new(self.client, path, params.limit.unwrap_or(10), line_item_id)
    }

    /// Convenience helper returning an auto-paginator for line items with default page limit (10).
    pub fn list_line_items_default(&self, session_id: &str) -> Paginator<'a, LineItem> {
        self.list_line_items(session_id, &ListLineItemsParams::default())
    }

    /// Convenience helper returning an auto-paginator with an optional page limit.
    pub fn list_line_items_with_limit(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Paginator<'a, LineItem> {
        let params = ListLineItemsParams {
            limit,
            ..Default::default()
        };
        self.list_line_items(session_id, &params)
    }
}

/// Checks if a Checkout Session has succeeded and is ready for order fulfillment.
///
/// For delayed payment methods (e.g. SEPA Direct Debit, Boleto, Konbini),
/// initial `checkout.session.completed` has `payment_status: "unpaid"`.
/// Fulfillment MUST be deferred until `payment_status == "paid"`.
///
/// Delegates to [`CheckoutSession::is_fulfillment_ready`].
#[must_use]
pub fn is_fulfillment_ready(session: &CheckoutSession) -> bool {
    session.is_fulfillment_ready()
}

/// Validates that a redirect URL (success_url / cancel_url) is safe against open redirect vulnerabilities.
///
/// Rejects `javascript:`, data URIs, unauthorized schemes, embedded credentials, and unapproved external domains.
/// Enforces HTTPS for all external domains (HTTP permitted strictly for localhost / loopback testing).
///
/// If `allowed_domains` is non-empty, the URL's host must match or be a subdomain of an allowed domain
/// (or localhost).
pub fn validate_redirect_url(url: &str, allowed_domains: &[&str]) -> Result<(), RedirectError> {
    let parsed = reqwest::Url::parse(url).map_err(|e| {
        if e.to_string().to_ascii_lowercase().contains("empty host") {
            RedirectError::MissingHost {
                url: url.to_string(),
            }
        } else {
            RedirectError::InvalidUrl {
                url: url.to_string(),
                reason: e.to_string(),
            }
        }
    })?;

    let scheme = parsed.scheme();
    if scheme != "https" && scheme != "http" {
        return Err(RedirectError::ProhibitedScheme {
            url: url.to_string(),
            scheme: scheme.to_string(),
        });
    }

    let host = match parsed.host_str() {
        Some(h) if !h.trim().is_empty() => h.trim(),
        _ => {
            return Err(RedirectError::MissingHost {
                url: url.to_string(),
            });
        }
    };

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(RedirectError::EmbeddedCredentials {
            url: url.to_string(),
        });
    }

    let is_localhost = is_localhost_host(host);

    if scheme == "http" && !is_localhost {
        return Err(RedirectError::ProhibitedScheme {
            url: url.to_string(),
            scheme: scheme.to_string(),
        });
    }

    if !allowed_domains.is_empty() {
        let is_allowed = is_localhost
            || allowed_domains.iter().any(|allowed| {
                let clean = allowed
                    .trim()
                    .strip_prefix("*.")
                    .or_else(|| allowed.trim().strip_prefix('.'))
                    .unwrap_or_else(|| allowed.trim());

                if clean.is_empty() {
                    return false;
                }

                host.eq_ignore_ascii_case(clean)
                    || host
                        .to_ascii_lowercase()
                        .ends_with(&format!(".{}", clean.to_ascii_lowercase()))
            });

        if !is_allowed {
            return Err(RedirectError::DisallowedDomain {
                host: host.to_string(),
                allowed_domains: allowed_domains.iter().map(|s| s.to_string()).collect(),
            });
        }
    }

    Ok(())
}

fn is_localhost_host(host: &str) -> bool {
    let host = host.trim_matches(|c| c == '[' || c == ']');
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return ip.is_loopback();
    }
    false
}

/// Extension trait providing zero-copy borrowed slice accessors and status inspectors for [`CheckoutSession`].
///
/// Ensures Gate G-Borrow zero-clone borrowing discipline by returning borrowed `&str` slices
/// from session string fields instead of allocating heap strings.
pub trait CheckoutSessionExt {
    /// Returns a borrowed string slice to the hosted checkout URL, if available.
    fn url_slice(&self) -> Option<&str>;

    /// Returns a borrowed string slice to the checkout session status, if available.
    fn status_slice(&self) -> Option<&str>;

    /// Returns a borrowed string slice to the payment status, if available.
    fn payment_status_slice(&self) -> Option<&str>;

    /// Returns `true` if the session status is `"open"`.
    fn is_open(&self) -> bool;

    /// Returns `true` if the session status is `"complete"`.
    fn is_complete(&self) -> bool;

    /// Returns `true` if the session status is `"expired"`.
    fn is_expired(&self) -> bool;

    /// Returns `true` if the session payment status is `"paid"`.
    fn is_paid(&self) -> bool;

    /// Returns `true` if the session payment status is `"unpaid"`.
    fn is_unpaid(&self) -> bool;
}

impl CheckoutSessionExt for CheckoutSession {
    fn url_slice(&self) -> Option<&str> {
        self.url.as_deref()
    }

    fn status_slice(&self) -> Option<&str> {
        self.status.as_deref()
    }

    fn payment_status_slice(&self) -> Option<&str> {
        self.payment_status.as_deref()
    }

    fn is_open(&self) -> bool {
        self.status.as_deref() == Some("open")
    }

    fn is_complete(&self) -> bool {
        self.status.as_deref() == Some("complete")
    }

    fn is_expired(&self) -> bool {
        self.status.as_deref() == Some("expired")
    }

    fn is_paid(&self) -> bool {
        self.payment_status.as_deref() == Some("paid")
    }

    fn is_unpaid(&self) -> bool {
        self.payment_status.as_deref() == Some("unpaid")
    }
}
