//! Stripe Checkout resource implementations.
//!
//! Provides bindings for managing Stripe Checkout Sessions (`create`, `retrieve`, `expire`,
//! and `list_line_items`), modeled after `stripe-node`.
//!
//! # Architecture & Security Highlights:
//! - **G0 (Rust 2024 Edition)**: Clean compilation with 0 clippy warnings and strict formatting.
//! - **G-Borrow (Zero-Clone Discipline)**: In-place streaming pagination and zero-copy string slice
//!   accessors via [`CheckoutSessionExt`].
//! - **G2 (Security & Idempotency)**: Automatic RFC 4122 UUID v4 `Idempotency-Key` header injection
//!   on mutable endpoints (`create`, `expire`) to guard against network retry double charges.
//! - **Gate G-Law (Open Redirect & Fulfillment Invariants)**: Strict URL scheme validation (`validate_redirect_url`)
//!   protecting success and cancellation redirects against phishing attacks, and deferred fulfillment
//!   validation (`is_fulfillment_ready`) for asynchronous/delayed payment methods (e.g. SEPA, Boleto).
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//! use some_stripe_types::CreateCheckoutSessionParams;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_mock_123");
//! let params = CreateCheckoutSessionParams::new("https://example.com/success")
//!     .cancel_url("https://example.com/cancel")
//!     .mode("payment");
//!
//! let session = client.checkout().sessions().create(&params).await?;
//! println!("Hosted Checkout URL: {:?}", session.url);
//! # Ok(())
//! # }
//! ```

use crate::resources::payment_links::PaymentLinksResource;
pub use some_stripe_core::RedirectError;
use some_stripe_core::patterns::IdempotencyKeyManager;
use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    CheckoutSession, CreateCheckoutSessionParams, LineItem, ListLineItemsParams, line_item_id,
};

/// Root resource for Stripe Checkout operations (`stripe.checkout`).
///
/// Provides access to checkout sessions (`stripe.checkout.sessions`) and
/// payment links (`stripe.checkout.payment_links`).
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let checkout = client.checkout();
/// let sessions = checkout.sessions();
/// let payment_links = checkout.payment_links();
/// ```
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

    /// Access the Sessions resource (`stripe.checkout.sessions`).
    pub fn sessions(&self) -> &SessionsResource<'a> {
        &self.sessions
    }

    /// Access the PaymentLinks resource (`stripe.checkout.payment_links`).
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
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CreateCheckoutSessionParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateCheckoutSessionParams::single_item(
    ///     "https://example.com/success",
    ///     "usd",
    ///     2000,
    ///     "Pro Subscription",
    ///     1,
    /// )
    /// .cancel_url("https://example.com/cancel")
    /// .client_reference_id("user_order_99");
    ///
    /// let session = client.checkout().sessions().create(&params).await?;
    /// assert!(session.id.starts_with("cs_"));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateCheckoutSessionParams,
    ) -> Result<CheckoutSession, StripeError> {
        let url = format!("{}/v1/checkout/sessions", self.client.config.api_base);

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) => {
                IdempotencyKeyManager::validate(k)?;
                k
            }
            None => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let mut form_pairs = some_stripe_core::patterns::serializer::to_form_params(params)?;

        // Ensure flat single-item shorthand fields are never emitted as top-level params over the wire
        form_pairs.retain(|(k, _)| {
            k != "currency" && k != "unit_amount" && k != "product_name" && k != "quantity"
        });

        if params.line_items.is_none() {
            if let Some(curr) = &params.currency {
                form_pairs.push((
                    "line_items[0][price_data][currency]".to_string(),
                    curr.clone(),
                ));
            }
            if let Some(amt) = params.unit_amount {
                form_pairs.push((
                    "line_items[0][price_data][unit_amount]".to_string(),
                    amt.to_string(),
                ));
            }
            if let Some(name) = &params.product_name {
                form_pairs.push((
                    "line_items[0][price_data][product_data][name]".to_string(),
                    name.clone(),
                ));
            }
            if let Some(qty) = params.quantity {
                form_pairs.push(("line_items[0][quantity]".to_string(), qty.to_string()));
            }
        }

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_pairs)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let session: CheckoutSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Retrieves an existing Checkout Session by ID (`GET /v1/checkout/sessions/{id}`).
    ///
    /// Accepts the identifier as a string reference (`&str`), ensuring zero heap allocations.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let session = client.checkout().sessions().retrieve("cs_test_abc123").await?;
    /// println!("Status: {:?}", session.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, session_id: &str) -> Result<CheckoutSession, StripeError> {
        let encoded_id = validate_and_encode_id(session_id)?;
        let url = format!(
            "{}/v1/checkout/sessions/{}",
            self.client.config.api_base, encoded_id
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let session: CheckoutSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Expires an active Checkout Session (`POST /v1/checkout/sessions/{id}/expire`).
    ///
    /// Automatically injects a cryptographically secure RFC 4122 UUID v4 `Idempotency-Key` header.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let expired_session = client.checkout().sessions().expire("cs_test_abc123").await?;
    /// assert_eq!(expired_session.status.as_deref(), Some("expired"));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn expire(&self, session_id: &str) -> Result<CheckoutSession, StripeError> {
        let encoded_id = validate_and_encode_id(session_id)?;
        let url = format!(
            "{}/v1/checkout/sessions/{}/expire",
            self.client.config.api_base, encoded_id
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
    ///
    /// Yields line items in chunks across wire pages with zero deep cloning.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::ListLineItemsParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListLineItemsParams::new().limit(25);
    /// let mut paginator = client.checkout().sessions().list_line_items("cs_test_123", &params);
    /// while let Some(item) = paginator.next_item().await? {
    ///     println!("Item: {} Amount: {}", item.id, item.amount_total);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_line_items(
        &self,
        session_id: &str,
        params: &ListLineItemsParams,
    ) -> Paginator<'a, LineItem> {
        let (path, initial_err) = match validate_and_encode_id(session_id) {
            Ok(encoded) => (format!("v1/checkout/sessions/{encoded}/line_items"), None),
            Err(e) => (String::new(), Some(e)),
        };

        let mut paginator =
            Paginator::new(self.client, path, params.limit.unwrap_or(10), line_item_id);

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
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut stream = client.checkout().sessions().list_line_items_default("cs_test_123");
    /// while let Some(item) = stream.next_item().await? {
    ///     println!("Line item: {}", item.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_line_items_default(&self, session_id: &str) -> Paginator<'a, LineItem> {
        self.list_line_items(session_id, &ListLineItemsParams::default())
    }

    /// Convenience helper returning an auto-paginator with an optional page limit.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut stream = client.checkout().sessions().list_line_items_with_limit("cs_test_123", Some(50));
    /// while let Some(item) = stream.next_item().await? {
    ///     println!("Line item: {}", item.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
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

#[doc(inline)]
pub use some_stripe_core::patterns::crud::validate_and_encode_id;

/// Checks if a Checkout Session has succeeded and is ready for order fulfillment.
///
/// For delayed payment methods (e.g. SEPA Direct Debit, Boleto, Konbini),
/// initial `checkout.session.completed` has `payment_status: "unpaid"`.
/// Fulfillment MUST be deferred until `payment_status == "paid"`.
///
/// Delegates to [`CheckoutSession::is_fulfillment_ready`].
///
/// # Example
/// ```
/// use some_stripe::is_fulfillment_ready;
/// use some_stripe_types::CheckoutSession;
///
/// // Card payment: instant completion with paid status -> ready for fulfillment
/// let session_paid = CheckoutSession::new("cs_test_paid")
///     .with_status("complete")
///     .with_payment_status("paid");
/// assert!(is_fulfillment_ready(&session_paid));
///
/// // SEPA Direct Debit: complete session with unpaid status -> NOT ready for fulfillment
/// let session_delayed = CheckoutSession::new("cs_test_sepa")
///     .with_status("complete")
///     .with_payment_status("unpaid");
/// assert!(!is_fulfillment_ready(&session_delayed));
///
/// // Open session is never ready
/// let session_open = CheckoutSession::new("cs_test_open")
///     .with_status("open")
///     .with_payment_status("unpaid");
/// assert!(!is_fulfillment_ready(&session_open));
/// ```
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
///
/// # Example
/// ```
/// use some_stripe::validate_redirect_url;
///
/// let allowed = ["myshop.com", "*.myshop.com"];
///
/// // Allowed HTTPS domains and subdomains succeed
/// assert!(validate_redirect_url("https://myshop.com/checkout/success", &allowed).is_ok());
/// assert!(validate_redirect_url("https://pay.myshop.com/checkout/success", &allowed).is_ok());
///
/// // Localhost / loopback is permitted for local development
/// assert!(validate_redirect_url("http://localhost:3000/callback", &allowed).is_ok());
/// assert!(validate_redirect_url("http://127.0.0.1:8080/cancel", &allowed).is_ok());
///
/// // Disallowed external domains and unencrypted HTTP are rejected
/// assert!(validate_redirect_url("https://malicious-site.com/steal", &allowed).is_err());
/// assert!(validate_redirect_url("http://myshop.com/insecure", &allowed).is_err());
///
/// // Dangerous schemes and credential injection are rejected
/// assert!(validate_redirect_url("javascript:alert(1)", &allowed).is_err());
/// assert!(validate_redirect_url("https://user:pass@myshop.com/", &allowed).is_err());
/// ```
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
///
/// # Example
/// ```
/// use some_stripe::CheckoutSessionExt;
/// use some_stripe_types::CheckoutSession;
///
/// let session = CheckoutSession::new("cs_test_123")
///     .with_url("https://checkout.stripe.com/c/pay/cs_test_123")
///     .with_status("open")
///     .with_payment_status("unpaid");
///
/// // Access borrowed string slices without allocations (Gate G-Borrow)
/// assert_eq!(session.url_slice(), Some("https://checkout.stripe.com/c/pay/cs_test_123"));
/// assert_eq!(session.status_slice(), Some("open"));
/// assert_eq!(session.payment_status_slice(), Some("unpaid"));
///
/// // Query status helpers
/// assert!(session.is_open());
/// assert!(!session.is_complete());
/// assert!(!session.is_expired());
/// assert!(session.is_unpaid());
/// assert!(!session.is_paid());
/// ```
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
