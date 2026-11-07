//! Reusable pagination stream engine and collection envelope abstractions.
//!
//! # Gate G-Borrow Invariants
//! - Zero deep clones on domain entities.
//! - Uses `VecDeque<T>` pop_front moves so that `T` does NOT require `Clone`.
//! - Bounded resident memory $O(\text{limit})$.
//! - `CursorLoopDetector` prevents cyclic cursor infinite loops (DoS protection).

use crate::client::StripeClient;
use crate::error::StripeError;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

/// Standard Stripe list response envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListResponse<T> {
    /// Object type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,
    /// The page of items returned by the API.
    pub data: Vec<T>,
    /// Whether there are more items available beyond this page.
    pub has_more: bool,
    /// The URL endpoint that produced this list.
    #[serde(default)]
    pub url: Option<String>,
}

fn default_list_object() -> String {
    "list".into()
}

/// Standard query parameters for cursor-based pagination.
///
/// # Examples
/// ```rust
/// use some_stripe_core::patterns::pagination::ListParams;
///
/// let params = ListParams::new()
///     .limit(25)
///     .starting_after("cus_last_seen");
/// assert_eq!(params.limit, Some(25));
/// assert_eq!(params.starting_after.as_deref(), Some("cus_last_seen"));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListParams {
    /// A cursor for use in pagination: an object ID defining your place in the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A cursor for use in backward pagination: an object ID defining your place in the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,

    /// A limit on the number of objects to be returned (1 to 100). Default: 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl ListParams {
    /// Creates empty, default [`ListParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of items to return (clamped to 100).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit.min(100));
        self
    }

    /// Fetches the page of objects immediately following this object ID.
    pub fn starting_after(mut self, id: impl Into<String>) -> Self {
        self.starting_after = Some(id.into());
        self
    }

    /// Fetches the page of objects immediately preceding this object ID.
    pub fn ending_before(mut self, id: impl Into<String>) -> Self {
        self.ending_before = Some(id.into());
        self
    }
}

/// Circular cursor sentinel intercepting repetitive cursors to prevent infinite loops (DoS).
///
/// Tracks encountered pagination cursor tokens and halts traversal with
/// [`StripeError::PaginationLoopDetected`] if a cycle is detected.
///
/// # Examples
/// ```rust
/// use some_stripe_core::patterns::pagination::CursorLoopDetector;
///
/// let mut detector = CursorLoopDetector::new();
/// assert!(detector.record_cursor("cus_page1").is_ok());
/// assert!(detector.record_cursor("cus_page2").is_ok());
/// // Encountering the same cursor again flags a loop:
/// assert!(detector.record_cursor("cus_page1").is_err());
/// ```
#[derive(Debug, Default, Clone)]
pub struct CursorLoopDetector {
    seen_cursors: HashSet<String>,
    page_count: usize,
}

impl CursorLoopDetector {
    /// Creates a new cursor loop detector.
    pub fn new() -> Self {
        Self {
            seen_cursors: HashSet::new(),
            page_count: 0,
        }
    }

    /// Records an inspected cursor, returning error if a repeat is encountered.
    pub fn record_cursor(&mut self, cursor: &str) -> Result<(), StripeError> {
        self.page_count += 1;
        if !self.seen_cursors.insert(cursor.to_string()) {
            return Err(StripeError::PaginationLoopDetected {
                cursor: cursor.to_string(),
                page: self.page_count,
            });
        }
        Ok(())
    }
}

/// Zero-clone, asynchronous stream paginator for Stripe list endpoints.
///
/// Yields items by moving them directly out of the resident buffer without requiring `T: Clone`.
pub struct Paginator<'a, T> {
    client: &'a StripeClient,
    path: String,
    limit: u32,
    buffer: VecDeque<T>,
    loop_detector: CursorLoopDetector,
    has_more: bool,
    pages_fetched: usize,
    done: bool,
    last_id_extractor: fn(&T) -> &str,
    last_seen_cursor: Option<String>,
    ending_before: Option<String>,
    initial_error: Option<StripeError>,
    stripe_account: Option<&'a str>,
}

impl<'a, T> Paginator<'a, T> {
    /// Creates a new zero-clone paginator.
    pub fn new(
        client: &'a StripeClient,
        path: impl Into<String>,
        limit: u32,
        id_extractor: fn(&T) -> &str,
    ) -> Self {
        let path_str = path.into();
        let path = path_str.trim_start_matches('/').to_string();
        Self {
            client,
            path,
            limit: limit.clamp(1, 100),
            buffer: VecDeque::new(),
            loop_detector: CursorLoopDetector::new(),
            has_more: true,
            pages_fetched: 0,
            done: false,
            last_id_extractor: id_extractor,
            last_seen_cursor: None,
            ending_before: None,
            initial_error: None,
            stripe_account: None,
        }
    }

    /// Attaches the `Stripe-Account` tenant header for multi-tenant connected account requests.
    ///
    /// When specified, all subsequent paginated requests dispatched over the wire
    /// will include `Stripe-Account: <account_id>`.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_core::{StripeClient, SecretKey};
    /// use some_stripe_core::patterns::pagination::Paginator;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct MockItem {
    ///     id: String,
    /// }
    ///
    /// let client = StripeClient::new(SecretKey::new("sk_test_123"));
    /// let paginator: Paginator<'_, MockItem> = Paginator::new(
    ///     &client,
    ///     "/v1/accounts",
    ///     10,
    ///     |item: &MockItem| &item.id,
    /// ).with_account("acct_123");
    ///
    /// assert_eq!(paginator.pages_fetched(), 0);
    /// assert_eq!(paginator.stripe_account(), Some("acct_123"));
    /// ```
    #[must_use]
    pub fn with_account(mut self, account: &'a str) -> Self {
        self.stripe_account = Some(account);
        self
    }

    /// Returns the configured connected account identifier, if any.
    pub fn stripe_account(&self) -> Option<&'a str> {
        self.stripe_account
    }

    /// Seeds the paginator with an initial `starting_after` cursor for forward pagination.
    pub fn starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.last_seen_cursor = Some(cursor.into());
        self
    }

    /// Seeds the paginator with an initial starting cursor for pagination resumption.
    ///
    /// This is useful when resuming pagination from a cursor stored in an external state,
    /// checkpoint, or previous API response.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_core::{StripeClient, SecretKey};
    /// use some_stripe_core::patterns::pagination::Paginator;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct MockItem {
    ///     id: String,
    /// }
    ///
    /// let client = StripeClient::new(SecretKey::new("sk_test_123"));
    /// let paginator: Paginator<'_, MockItem> = Paginator::new(
    ///     &client,
    ///     "/v1/customers",
    ///     10,
    ///     |item: &MockItem| &item.id,
    /// ).with_starting_after("cus_seed_999");
    ///
    /// assert_eq!(paginator.pages_fetched(), 0);
    /// ```
    #[must_use]
    pub fn with_starting_after(self, cursor: impl Into<String>) -> Self {
        self.starting_after(cursor)
    }

    /// Seeds the paginator with an initial `ending_before` cursor for backward pagination.
    pub fn ending_before(mut self, cursor: impl Into<String>) -> Self {
        self.ending_before = Some(cursor.into());
        self
    }

    /// Fluent alias for [`ending_before`].
    pub fn with_ending_before(self, cursor: impl Into<String>) -> Self {
        self.ending_before(cursor)
    }

    /// Seeds the paginator with an initial error that will be returned on the first call to `next_item`.
    pub fn with_initial_error(mut self, error: StripeError) -> Self {
        self.initial_error = Some(error);
        self
    }

    /// Total number of HTTP pages fetched over the wire.
    pub fn pages_fetched(&self) -> usize {
        self.pages_fetched
    }
}

impl<'a, T: serde::de::DeserializeOwned> Paginator<'a, T> {
    async fn fetch_page(&mut self) -> Result<ListResponse<T>, StripeError> {
        let sep = if self.path.contains('?') { '&' } else { '?' };
        let mut url = format!(
            "{}/{}{sep}limit={}",
            self.client.config.api_base.trim_end_matches('/'),
            self.path,
            self.limit
        );
        if let Some(eb) = self.ending_before.take() {
            let encoded_eb = crate::patterns::serializer::percent_encode(&eb);
            url.push_str(&format!("&ending_before={encoded_eb}"));
        } else if let Some(ref c) = self.last_seen_cursor {
            let encoded_cursor = crate::patterns::serializer::percent_encode(c);
            url.push_str(&format!("&starting_after={encoded_cursor}"));
        }

        let mut req = self.client.http.get(&url);
        if let Some(account) = self.stripe_account {
            req = req.header("Stripe-Account", account);
        }
        let val = self.client.execute(req).await?;
        let page: ListResponse<T> = serde_json::from_value(val)?;
        self.pages_fetched += 1;
        Ok(page)
    }

    /// Fetches the next item by value with zero heap clones.
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

            // 2. Check termination
            if self.done || !self.has_more {
                self.done = true;
                return Ok(None);
            }

            // 3. Fetch next page
            if let Some(ref cursor) = self.last_seen_cursor {
                self.loop_detector.record_cursor(cursor)?;
            }

            let response = self.fetch_page().await?;
            self.has_more = response.has_more;

            if let Some(last) = response.data.last() {
                self.last_seen_cursor = Some((self.last_id_extractor)(last).to_string());
            }

            if response.data.is_empty() {
                self.done = true;
                return Ok(None);
            }

            // Move elements into deque buffer without cloning
            self.buffer = response.data.into();

            // Guard against empty intermediate pages
            if self.buffer.is_empty() && self.has_more {
                continue;
            } else if self.buffer.is_empty() {
                self.done = true;
                return Ok(None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct DummyItem {
        id: String,
    }

    #[test]
    fn test_paginator_with_starting_after_builder() {
        let client = StripeClient::new("sk_test_123");
        let paginator: Paginator<DummyItem> =
            Paginator::new(&client, "v1/items", 10, |i: &DummyItem| &i.id)
                .with_starting_after("item_seeded_456");

        assert_eq!(
            paginator.last_seen_cursor.as_deref(),
            Some("item_seeded_456")
        );
    }

    #[test]
    fn test_list_params_builder() {
        let params = ListParams::new()
            .limit(20)
            .starting_after("item_1")
            .ending_before("item_0");

        assert_eq!(params.limit, Some(20));
        assert_eq!(params.starting_after.as_deref(), Some("item_1"));
        assert_eq!(params.ending_before.as_deref(), Some("item_0"));
    }

    #[test]
    fn test_paginator_with_account_builder() {
        let client = StripeClient::new("sk_test_123");
        let paginator: Paginator<DummyItem> =
            Paginator::new(&client, "v1/items", 10, |i: &DummyItem| &i.id)
                .with_account("acct_test_connected_123");

        assert_eq!(paginator.stripe_account(), Some("acct_test_connected_123"));
    }
}
