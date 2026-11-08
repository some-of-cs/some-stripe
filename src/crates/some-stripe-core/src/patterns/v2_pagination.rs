//! Reusable URL-driven pagination stream engine for Stripe API v2 list endpoints.
//!
//! # Architecture & Protocol Contract
//! In Stripe API v2, list endpoints follow HATEOAS (Hypermedia as the Engine of Application State)
//! principles. Rather than client-side construction of pagination cursors (`starting_after`),
//! list responses return opaque next and previous page URLs (`next_page_url` and `previous_page_url`).
//!
//! The [`V2Paginator`] engine consumes `next_page_url` directly from the response envelope,
//! extracts the query path and parameters, and executes relative URL dispatch via [`StripeClient`].
//!
//! # Gate G-Borrow Invariants
//! - Zero deep clones on domain entities: items are popped directly from a [`VecDeque`] resident buffer.
//! - The entity type `T` does NOT require [`Clone`].
//! - Resident memory footprint is bounded strictly to $O(\text{page\_size})$.
//!
//! # Gate G-Complexity Invariants
//! - All branching complexity strictly capped at $CC \le 6$.
//! - Maximum nesting depth $\le 2$ levels.
//!
//! # Security Gateway (DoS Protection)
//! Intercepts cyclical `next_page_url` values using an internal loop detector to halt traversal
//! with [`StripeError::PaginationLoopDetected`].

use crate::client::{RequestOptions, StripeClient};
use crate::error::StripeError;
use some_stripe_types::V2List;
use std::collections::{HashSet, VecDeque};

/// Resolves a relative or absolute URL against the configured Stripe API base URL.
///
/// In Stripe API v2, pagination URLs returned in `next_page_url` may be root-relative
/// (e.g. `/v2/core/events?page=next_token`) or absolute (e.g. `https://api.stripe.com/v2/core/events?page=next_token`).
/// To maintain SSRF isolation and route correctly through mock servers or proxies,
/// the scheme, host, and port of `api_base` are always preserved, extracting only the path
/// and query components from absolute URL strings.
///
/// # Examples
/// ```rust
/// use some_stripe_core::patterns::v2_pagination::resolve_v2_url;
///
/// assert_eq!(
///     resolve_v2_url("https://api.stripe.com", "/v2/core/events?page=2"),
///     "https://api.stripe.com/v2/core/events?page=2"
/// );
/// assert_eq!(
///     resolve_v2_url("http://127.0.0.1:8080", "https://api.stripe.com/v2/core/events?page=2"),
///     "http://127.0.0.1:8080/v2/core/events?page=2"
/// );
/// assert_eq!(
///     resolve_v2_url("https://api.stripe.com/", "v2/core/events"),
///     "https://api.stripe.com/v2/core/events"
/// );
/// ```
#[must_use]
pub fn resolve_v2_url(api_base: &str, next_url: &str) -> String {
    let clean_base = api_base.trim_end_matches('/');
    let trimmed = next_url.trim();

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if let Some(pos) = trimmed.find("://") {
            let after_scheme = &trimmed[pos + 3..];
            if let Some(slash_idx) = after_scheme.find('/') {
                let path_and_query = &after_scheme[slash_idx..];
                return format!("{clean_base}{path_and_query}");
            }
        }
        return format!("{clean_base}/");
    }

    if trimmed.starts_with('/') {
        format!("{clean_base}{trimmed}")
    } else {
        format!("{clean_base}/{trimmed}")
    }
}

/// Zero-clone, URL-driven asynchronous stream paginator for Stripe API v2 collection endpoints.
///
/// Yields items by moving them directly out of the resident buffer without requiring `T: Clone`.
/// Consumes `next_page_url` directly from the HATEOAS response envelope to navigate through
/// collections without manual cursor construction.
///
/// # Gate G-Borrow Invariants
/// - Zero deep clones on domain entities: items are popped directly from a [`VecDeque<T>`] buffer.
/// - Resident memory footprint is bounded strictly to $O(\text{page\_size})$.
/// - Supports non-`Clone` domain entities.
///
/// # Loop Detection (DoS Protection)
/// Intercepts repetitive `next_page_url` values to prevent infinite traversal cycles,
/// returning [`StripeError::PaginationLoopDetected`] upon detecting a duplicate page URL.
///
/// # Examples
/// ```rust
/// use some_stripe_core::{StripeClient, SecretKey};
/// use some_stripe_core::patterns::v2_pagination::V2Paginator;
///
/// #[derive(serde::Deserialize)]
/// struct MockEvent {
///     id: String,
/// }
///
/// let client = StripeClient::new(SecretKey::new("sk_test_123"));
/// let paginator: V2Paginator<'_, MockEvent> = V2Paginator::new(&client, "/v2/core/events")
///     .with_account("acct_tenant_999");
///
/// assert_eq!(paginator.pages_fetched(), 0);
/// assert_eq!(paginator.stripe_account(), Some("acct_tenant_999"));
/// ```
pub struct V2Paginator<'a, T> {
    client: &'a StripeClient,
    next_url: Option<String>,
    buffer: VecDeque<T>,
    seen_urls: HashSet<String>,
    pages_fetched: usize,
    done: bool,
    options: Option<RequestOptions>,
    initial_error: Option<StripeError>,
}

impl<'a, T> V2Paginator<'a, T> {
    /// Creates a new [`V2Paginator`] pointing to an initial relative or absolute endpoint URL.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_core::StripeClient;
    /// use some_stripe_core::patterns::v2_pagination::V2Paginator;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct Item { id: String }
    ///
    /// let client = StripeClient::new("sk_test_123");
    /// let paginator: V2Paginator<'_, Item> = V2Paginator::new(&client, "/v2/core/events");
    /// assert_eq!(paginator.pages_fetched(), 0);
    /// assert_eq!(paginator.next_url(), Some("/v2/core/events"));
    /// ```
    #[must_use]
    pub fn new(client: &'a StripeClient, initial_url: impl Into<String>) -> Self {
        let raw_url = initial_url.into();
        let next_url = if raw_url.trim().is_empty() {
            None
        } else {
            Some(raw_url)
        };

        Self {
            client,
            next_url,
            buffer: VecDeque::new(),
            seen_urls: HashSet::new(),
            pages_fetched: 0,
            done: false,
            options: None,
            initial_error: None,
        }
    }

    /// Seeds a [`V2Paginator`] from an already-fetched [`V2List<T>`] response envelope.
    ///
    /// The items from `initial_page` are buffered immediately with zero network overhead.
    /// Subsequent calls to [`V2Paginator::next_item`] drain the buffered items first,
    /// then follow `initial_page.next_page_url` if present.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_core::StripeClient;
    /// use some_stripe_core::patterns::v2_pagination::V2Paginator;
    /// use some_stripe_types::V2List;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct Item { id: String }
    ///
    /// let client = StripeClient::new("sk_test_123");
    /// let initial_page = V2List::new(
    ///     vec![Item { id: "evt_1".into() }],
    ///     Some("/v2/core/events?page=2".into()),
    ///     None,
    ///     None,
    /// );
    ///
    /// let paginator = V2Paginator::from_page(&client, initial_page);
    /// assert_eq!(paginator.pages_fetched(), 1);
    /// assert_eq!(paginator.buffer_len(), 1);
    /// assert_eq!(paginator.next_url(), Some("/v2/core/events?page=2"));
    /// ```
    #[must_use]
    pub fn from_page(client: &'a StripeClient, initial_page: V2List<T>) -> Self {
        let next_url = initial_page.next_page_url.filter(|u| !u.trim().is_empty());
        Self {
            client,
            next_url,
            buffer: initial_page.data.into(),
            seen_urls: HashSet::new(),
            pages_fetched: 1,
            done: false,
            options: None,
            initial_error: None,
        }
    }

    /// Attaches the `Stripe-Account` tenant header for multi-tenant connected account requests.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_core::StripeClient;
    /// use some_stripe_core::patterns::v2_pagination::V2Paginator;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct Item { id: String }
    ///
    /// let client = StripeClient::new("sk_test_123");
    /// let paginator: V2Paginator<'_, Item> = V2Paginator::new(&client, "/v2/core/events")
    ///     .with_account("acct_tenant_123");
    /// assert_eq!(paginator.stripe_account(), Some("acct_tenant_123"));
    /// ```
    #[must_use]
    pub fn with_account(mut self, account: &'a str) -> Self {
        let mut opts = self.options.unwrap_or_default();
        opts.stripe_account = Some(account.to_string());
        self.options = Some(opts);
        self
    }

    /// Returns the configured connected account identifier, if any.
    #[must_use]
    pub fn stripe_account(&self) -> Option<&str> {
        self.options
            .as_ref()
            .and_then(|opts| opts.stripe_account.as_deref())
    }

    /// Attaches custom [`RequestOptions`] (e.g. pinned API version, idempotency key, timeout).
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_core::{StripeClient, RequestOptions};
    /// use some_stripe_core::patterns::v2_pagination::V2Paginator;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct Item { id: String }
    ///
    /// let client = StripeClient::new("sk_test_123");
    /// let opts = RequestOptions::new().stripe_version("2025-01-01");
    /// let paginator: V2Paginator<'_, Item> = V2Paginator::new(&client, "/v2/core/events")
    ///     .with_options(opts);
    /// assert_eq!(
    ///     paginator.options().and_then(|o| o.stripe_version.as_deref()),
    ///     Some("2025-01-01")
    /// );
    /// ```
    #[must_use]
    pub fn with_options(mut self, options: RequestOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Returns a borrowed reference to the active [`RequestOptions`], if configured.
    #[must_use]
    pub fn options(&self) -> Option<&RequestOptions> {
        self.options.as_ref()
    }

    /// Seeds the paginator with an initial error returned on the first invocation of [`next_item`](Self::next_item).
    #[must_use]
    pub fn with_initial_error(mut self, error: StripeError) -> Self {
        self.initial_error = Some(error);
        self
    }

    /// Total number of HTTP pages fetched over the wire.
    #[must_use]
    pub fn pages_fetched(&self) -> usize {
        self.pages_fetched
    }

    /// Returns `true` if the stream has exhausted all pages and items.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Number of items currently buffered and awaiting consumption.
    #[must_use]
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns `true` if the resident item buffer is empty.
    #[must_use]
    pub fn is_buffer_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Returns the pending `next_page_url` if another page is scheduled to be fetched.
    #[must_use]
    pub fn next_url(&self) -> Option<&str> {
        self.next_url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
    }

    /// Fluent self-return conforming to the `auto_paging_each()` stream pattern.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_core::StripeClient;
    /// use some_stripe_core::patterns::v2_pagination::V2Paginator;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct Item { id: String }
    ///
    /// let client = StripeClient::new("sk_test_123");
    /// let mut stream: V2Paginator<'_, Item> = V2Paginator::new(&client, "/v2/core/events")
    ///     .auto_paging_each();
    /// assert_eq!(stream.pages_fetched(), 0);
    /// ```
    #[must_use]
    pub fn auto_paging_each(self) -> Self {
        self
    }

    fn has_next_url(&self) -> bool {
        self.next_url
            .as_deref()
            .map(|url| !url.trim().is_empty())
            .unwrap_or(false)
    }

    fn record_and_check_loop(&mut self, url: &str) -> Result<(), StripeError> {
        if !self.seen_urls.insert(url.to_string()) {
            self.done = true;
            return Err(StripeError::PaginationLoopDetected {
                cursor: url.to_string(),
                page: self.pages_fetched + 1,
            });
        }
        Ok(())
    }
}

impl<'a, T: serde::de::DeserializeOwned> V2Paginator<'a, T> {
    async fn fetch_page(&mut self, url: &str) -> Result<V2List<T>, StripeError> {
        let resolved_url = resolve_v2_url(&self.client.config.api_base, url);
        let req = self
            .client
            .http
            .get(&resolved_url)
            .header(reqwest::header::ACCEPT, "application/json");

        let val = self
            .client
            .execute_with_options(req, self.options.as_ref())
            .await?;
        let page: V2List<T> = serde_json::from_value(val)?;
        self.pages_fetched += 1;
        Ok(page)
    }

    async fn fetch_and_buffer_next_page(&mut self) -> Result<(), StripeError> {
        let target_url = self.next_url.take().unwrap_or_default();
        self.record_and_check_loop(&target_url)?;

        let page = self.fetch_page(&target_url).await?;
        self.next_url = page.next_page_url.filter(|u| !u.trim().is_empty());

        if page.data.is_empty() && !self.has_next_url() {
            self.done = true;
        } else {
            self.buffer = page.data.into();
        }
        Ok(())
    }

    /// Fetches the next item by value with zero heap clones (Gate G-Borrow).
    ///
    /// Automatically traverses `next_page_url` links across page boundaries,
    /// buffering responses and terminating cleanly with `Ok(None)` when the collection ends.
    ///
    /// # Errors
    /// - Returns [`StripeError::PaginationLoopDetected`] if a cyclical `next_page_url` is encountered.
    /// - Returns [`StripeError::Http`] or [`StripeError::Json`] on network or deserialization failures.
    pub async fn next_item(&mut self) -> Result<Option<T>, StripeError> {
        if let Some(err) = self.initial_error.take() {
            self.done = true;
            return Err(err);
        }

        loop {
            // 1. Yield buffered item directly via move (Zero-Clone)
            if let Some(item) = self.buffer.pop_front() {
                return Ok(Some(item));
            }

            // 2. Check termination condition
            if self.done || !self.has_next_url() {
                self.done = true;
                return Ok(None);
            }

            // 3. Fetch subsequent page and populate buffer
            self.fetch_and_buffer_next_page().await?;
        }
    }

    /// Eagerly consumes and collects all remaining items across all pages into a `Vec<T>`.
    ///
    /// # Errors
    /// Returns any error encountered during page retrieval.
    pub async fn collect_all(mut self) -> Result<Vec<T>, StripeError> {
        let mut items = Vec::new();
        while let Some(item) = self.next_item().await? {
            items.push(item);
        }
        Ok(items)
    }

    /// Asynchronously invokes an action closure for every item yielded by the paginator.
    ///
    /// # Errors
    /// Returns immediately on the first error produced by item retrieval or the action closure.
    pub async fn for_each<F, Fut>(mut self, mut action: F) -> Result<(), StripeError>
    where
        F: FnMut(T) -> Fut,
        Fut: std::future::Future<Output = Result<(), StripeError>>,
    {
        while let Some(item) = self.next_item().await? {
            action(item).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    struct MockItem {
        id: String,
        name: String,
    }

    #[test]
    fn test_resolve_v2_url_variations() {
        assert_eq!(
            resolve_v2_url("https://api.stripe.com", "/v2/core/events?page=2"),
            "https://api.stripe.com/v2/core/events?page=2"
        );
        assert_eq!(
            resolve_v2_url("https://api.stripe.com/", "v2/core/events?page=2"),
            "https://api.stripe.com/v2/core/events?page=2"
        );
        assert_eq!(
            resolve_v2_url(
                "http://127.0.0.1:9090",
                "https://api.stripe.com/v2/core/events?limit=10&page=tok_xyz"
            ),
            "http://127.0.0.1:9090/v2/core/events?limit=10&page=tok_xyz"
        );
        assert_eq!(
            resolve_v2_url("https://api.stripe.com", "http://foreign-host/"),
            "https://api.stripe.com/"
        );
        assert_eq!(
            resolve_v2_url("https://api.stripe.com", "http://foreign-host"),
            "https://api.stripe.com/"
        );
    }

    #[test]
    fn test_paginator_builder_and_options() {
        let client = StripeClient::new("sk_test_123");
        let paginator: V2Paginator<'_, MockItem> = V2Paginator::new(&client, "/v2/core/events")
            .with_account("acct_tenant_456")
            .with_options(RequestOptions::new().stripe_version("2025-01-01"));

        assert_eq!(paginator.pages_fetched(), 0);
        assert!(!paginator.is_done());
        assert_eq!(paginator.buffer_len(), 0);
        assert!(paginator.is_buffer_empty());
        assert_eq!(
            paginator
                .options()
                .and_then(|o| o.stripe_version.as_deref()),
            Some("2025-01-01")
        );
    }

    #[test]
    fn test_paginator_from_page_preseeded() {
        let client = StripeClient::new("sk_test_123");
        let page = V2List::new(
            vec![
                MockItem {
                    id: "1".into(),
                    name: "Alpha".into(),
                },
                MockItem {
                    id: "2".into(),
                    name: "Beta".into(),
                },
            ],
            Some("/v2/core/events?page=next_tok".into()),
            None,
            None,
        );

        let paginator = V2Paginator::from_page(&client, page);
        assert_eq!(paginator.pages_fetched(), 1);
        assert_eq!(paginator.buffer_len(), 2);
        assert_eq!(paginator.next_url(), Some("/v2/core/events?page=next_tok"));
    }

    #[test]
    fn test_loop_detector_records_and_detects_cycle() {
        let client = StripeClient::new("sk_test_123");
        let mut paginator: V2Paginator<'_, MockItem> =
            V2Paginator::new(&client, "/v2/core/events?page=1");

        assert!(
            paginator
                .record_and_check_loop("/v2/core/events?page=1")
                .is_ok()
        );
        assert!(
            paginator
                .record_and_check_loop("/v2/core/events?page=2")
                .is_ok()
        );

        // Repeated URL triggers PaginationLoopDetected
        let err = paginator
            .record_and_check_loop("/v2/core/events?page=1")
            .unwrap_err();
        assert!(matches!(err, StripeError::PaginationLoopDetected { .. }));
        assert!(paginator.is_done());
    }

    #[tokio::test]
    async fn test_initial_error_returned_immediately() {
        let client = StripeClient::new("sk_test_123");
        let mut paginator: V2Paginator<'_, MockItem> = V2Paginator::new(&client, "/v2/core/events")
            .with_initial_error(StripeError::InvalidRequest("Intentional test error".into()));

        let result = paginator.next_item().await;
        assert!(matches!(result, Err(StripeError::InvalidRequest(_))));
        assert!(paginator.is_done());

        // Subsequent call returns None
        let next = paginator.next_item().await.unwrap();
        assert!(next.is_none());
    }
}
