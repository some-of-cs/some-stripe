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
}

impl<'a, T: serde::de::DeserializeOwned> Paginator<'a, T> {
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
        }
    }

    /// Total number of HTTP pages fetched over the wire.
    pub fn pages_fetched(&self) -> usize {
        self.pages_fetched
    }

    async fn fetch_page(&mut self) -> Result<ListResponse<T>, StripeError> {
        let mut url = format!(
            "{}/{}?limit={}",
            self.client.config.api_base, self.path, self.limit
        );
        if let Some(ref c) = self.last_seen_cursor {
            url.push_str(&format!("&starting_after={c}"));
        }

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let page: ListResponse<T> = serde_json::from_value(val)?;
        self.pages_fetched += 1;
        Ok(page)
    }

    /// Fetches the next item by value with zero heap clones.
    pub async fn next_item(&mut self) -> Result<Option<T>, StripeError> {
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
