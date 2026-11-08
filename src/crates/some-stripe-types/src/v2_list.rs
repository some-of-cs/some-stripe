//! HATEOAS container envelope and pagination types for Stripe API v2.
//!
//! # Architecture & Protocol Contract
//! In Stripe API v2, list endpoints follow HATEOAS (Hypermedia as the Engine of Application State)
//! principles. Rather than client-side construction of pagination cursors (`starting_after`),
//! list responses return opaque next and previous page URLs (`next_page_url` and `previous_page_url`).
//!
//! # Gate G-Borrow Invariants
//! - Zero deep clones on domain entities during collection navigation.
//! - Read-only slice borrowing via [`V2List::items`].
//! - By-value ownership transfer via [`V2List::into_items`] and [`IntoIterator`].
//!
//! # Examples
//! ```rust
//! use some_stripe_types::{V2List, V2PageInfo};
//!
//! let page_info = V2PageInfo::new(Some(42), Some(true));
//! let list = V2List::new(
//!     vec!["evt_1".to_string(), "evt_2".to_string()],
//!     Some("/v2/core/events?page=next_token".into()),
//!     None,
//!     Some(page_info),
//! );
//!
//! assert!(list.has_next_page());
//! assert_eq!(list.next_url(), Some("/v2/core/events?page=next_token"));
//! assert_eq!(list.len(), 2);
//! assert_eq!(list.total_count(), Some(42));
//! assert_eq!(list.items()[0], "evt_1");
//! ```

use serde::{Deserialize, Serialize};

/// Metadata regarding total count and pagination state for Stripe API v2 list responses.
///
/// # Examples
/// ```rust
/// use some_stripe_types::V2PageInfo;
///
/// let info = V2PageInfo::new(Some(100), Some(true));
/// assert_eq!(info.total_count, Some(100));
/// assert_eq!(info.has_more, Some(true));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct V2PageInfo {
    /// Total count of items matching the query across all pages, if provided by the API.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u64>,

    /// Whether there are more items available beyond the current page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

impl V2PageInfo {
    /// Constructs a new [`V2PageInfo`] with total count and has_more indicators.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2PageInfo;
    ///
    /// let info = V2PageInfo::new(Some(15), Some(false));
    /// assert_eq!(info.total_count, Some(15));
    /// assert_eq!(info.has_more, Some(false));
    /// ```
    #[must_use]
    pub fn new(total_count: Option<u64>, has_more: Option<bool>) -> Self {
        Self {
            total_count,
            has_more,
        }
    }
}

/// Standard HATEOAS container envelope for Stripe API v2 collections.
///
/// # Serialization & Protocol Contract
/// In API v2, collection responses adhere to strict JSON encoding containing:
/// - `data`: Ordered vector of domain entities.
/// - `next_page_url`: Relative or absolute URL link to fetch the subsequent page.
/// - `previous_page_url`: Relative or absolute URL link to fetch the preceding page.
/// - `page_info`: Optional summary metadata including `total_count` and `has_more`.
///
/// # Examples
/// ```rust
/// use some_stripe_types::V2List;
///
/// let empty: V2List<String> = V2List::empty();
/// assert!(empty.is_empty());
/// assert!(!empty.has_next_page());
/// assert_eq!(empty.next_url(), None);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound(deserialize = "T: serde::de::Deserialize<'de>"))]
pub struct V2List<T> {
    /// The page of items returned by the API.
    #[serde(default = "Vec::new")]
    pub data: Vec<T>,

    /// URL link to the subsequent page of results, or `None` if at the end of the collection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_url: Option<String>,

    /// URL link to the preceding page of results, or `None` if at the beginning of the collection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_page_url: Option<String>,

    /// Supplementary pagination metadata such as total item count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_info: Option<V2PageInfo>,
}

impl<T> Default for V2List<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            next_page_url: None,
            previous_page_url: None,
            page_info: None,
        }
    }
}

impl<T> V2List<T> {
    /// Creates a new [`V2List`] envelope.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::{V2List, V2PageInfo};
    ///
    /// let list = V2List::new(
    ///     vec![1, 2, 3],
    ///     Some("/v2/items?page=2".into()),
    ///     None,
    ///     Some(V2PageInfo::new(Some(10), Some(true))),
    /// );
    /// assert_eq!(list.len(), 3);
    /// assert!(list.has_next_page());
    /// ```
    #[must_use]
    pub fn new(
        data: Vec<T>,
        next_page_url: Option<String>,
        previous_page_url: Option<String>,
        page_info: Option<V2PageInfo>,
    ) -> Self {
        Self {
            data,
            next_page_url,
            previous_page_url,
            page_info,
        }
    }

    /// Creates an empty [`V2List`] with no items and no pagination links.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2List;
    ///
    /// let list: V2List<i32> = V2List::empty();
    /// assert!(list.is_empty());
    /// assert_eq!(list.len(), 0);
    /// ```
    #[must_use]
    pub fn empty() -> Self {
        Self {
            data: Vec::new(),
            next_page_url: None,
            previous_page_url: None,
            page_info: None,
        }
    }

    /// Creates a [`V2List`] wrapping items with no pagination links.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2List;
    ///
    /// let list = V2List::with_items(vec!["alpha", "beta"]);
    /// assert_eq!(list.len(), 2);
    /// assert!(!list.has_next_page());
    /// ```
    #[must_use]
    pub fn with_items(data: Vec<T>) -> Self {
        Self {
            data,
            next_page_url: None,
            previous_page_url: None,
            page_info: None,
        }
    }

    /// Returns `true` if a non-empty `next_page_url` is present.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2List;
    ///
    /// let mut list: V2List<i32> = V2List::empty();
    /// assert!(!list.has_next_page());
    ///
    /// list.next_page_url = Some("/v2/core/events?page=2".into());
    /// assert!(list.has_next_page());
    ///
    /// list.next_page_url = Some("   ".into());
    /// assert!(!list.has_next_page());
    /// ```
    #[must_use]
    pub fn has_next_page(&self) -> bool {
        self.next_page_url
            .as_deref()
            .map(|url| !url.trim().is_empty())
            .unwrap_or(false)
    }

    /// Returns a borrowed slice reference to the `next_page_url`, if non-empty.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2List;
    ///
    /// let mut list: V2List<i32> = V2List::empty();
    /// list.next_page_url = Some("/v2/core/events?page=2".into());
    /// assert_eq!(list.next_url(), Some("/v2/core/events?page=2"));
    /// ```
    #[must_use]
    pub fn next_url(&self) -> Option<&str> {
        self.next_page_url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
    }

    /// Returns `true` if a non-empty `previous_page_url` is present.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2List;
    ///
    /// let mut list: V2List<i32> = V2List::empty();
    /// assert!(!list.has_previous_page());
    ///
    /// list.previous_page_url = Some("/v2/core/events?page=1".into());
    /// assert!(list.has_previous_page());
    /// ```
    #[must_use]
    pub fn has_previous_page(&self) -> bool {
        self.previous_page_url
            .as_deref()
            .map(|url| !url.trim().is_empty())
            .unwrap_or(false)
    }

    /// Returns a borrowed slice reference to the `previous_page_url`, if non-empty.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2List;
    ///
    /// let mut list: V2List<i32> = V2List::empty();
    /// list.previous_page_url = Some("/v2/core/events?page=1".into());
    /// assert_eq!(list.previous_url(), Some("/v2/core/events?page=1"));
    /// ```
    #[must_use]
    pub fn previous_url(&self) -> Option<&str> {
        self.previous_page_url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
    }

    /// Returns a borrowed slice of the items in the current page (Gate G-Borrow).
    ///
    /// Zero heap allocations or clones are performed.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2List;
    ///
    /// let list = V2List::with_items(vec![10, 20, 30]);
    /// assert_eq!(list.items(), &[10, 20, 30]);
    /// ```
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.data
    }

    /// Consumes the envelope and returns the underlying items vector by value.
    ///
    /// Zero deep clones are performed (Gate G-Borrow).
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2List;
    ///
    /// let list = V2List::with_items(vec!["a", "b"]);
    /// let items = list.into_items();
    /// assert_eq!(items, vec!["a", "b"]);
    /// ```
    #[must_use]
    pub fn into_items(self) -> Vec<T> {
        self.data
    }

    /// Number of items on the current page.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2List;
    ///
    /// let list = V2List::with_items(vec![1, 2, 3]);
    /// assert_eq!(list.len(), 3);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the current page contains no items.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::V2List;
    ///
    /// let list: V2List<i32> = V2List::empty();
    /// assert!(list.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the total count of items across all pages if reported by the API.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_types::{V2List, V2PageInfo};
    ///
    /// let mut list: V2List<i32> = V2List::empty();
    /// assert_eq!(list.total_count(), None);
    ///
    /// list.page_info = Some(V2PageInfo::new(Some(100), Some(true)));
    /// assert_eq!(list.total_count(), Some(100));
    /// ```
    #[must_use]
    pub fn total_count(&self) -> Option<u64> {
        self.page_info.as_ref().and_then(|info| info.total_count)
    }
}

impl<T> IntoIterator for V2List<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a V2List<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    struct MockEntity {
        id: String,
        name: String,
    }

    #[test]
    fn test_v2_list_serde_roundtrip() {
        let payload = r#"{
            "data": [
                {"id": "evt_1", "name": "Event 1"},
                {"id": "evt_2", "name": "Event 2"}
            ],
            "next_page_url": "/v2/core/events?page=token_abc",
            "previous_page_url": null,
            "page_info": {
                "total_count": 100,
                "has_more": true
            }
        }"#;

        let list: V2List<MockEntity> = serde_json::from_str(payload).unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.has_next_page());
        assert_eq!(list.next_url(), Some("/v2/core/events?page=token_abc"));
        assert!(!list.has_previous_page());
        assert_eq!(list.previous_url(), None);
        assert_eq!(list.total_count(), Some(100));
        assert_eq!(list.items()[0].id, "evt_1");

        let serialized = serde_json::to_string(&list).unwrap();
        assert!(serialized.contains("evt_1"));
        assert!(serialized.contains("/v2/core/events?page=token_abc"));
    }

    #[test]
    fn test_v2_list_empty_and_null_urls() {
        let list: V2List<MockEntity> = V2List::empty();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert!(!list.has_next_page());
        assert_eq!(list.next_url(), None);
        assert!(!list.has_previous_page());
        assert_eq!(list.previous_url(), None);
        assert_eq!(list.total_count(), None);

        let list_with_spaces = V2List {
            data: vec![MockEntity {
                id: "e1".into(),
                name: "test".into(),
            }],
            next_page_url: Some("   ".into()),
            previous_page_url: Some("".into()),
            page_info: None,
        };
        assert!(!list_with_spaces.has_next_page());
        assert_eq!(list_with_spaces.next_url(), None);
        assert!(!list_with_spaces.has_previous_page());
        assert_eq!(list_with_spaces.previous_url(), None);
    }

    #[test]
    fn test_v2_list_iteration_and_into_items() {
        let items = vec![
            MockEntity {
                id: "1".into(),
                name: "A".into(),
            },
            MockEntity {
                id: "2".into(),
                name: "B".into(),
            },
        ];
        let list = V2List::with_items(items);

        // Slice iteration
        let mut count = 0;
        for item in &list {
            count += 1;
            assert!(!item.id.is_empty());
        }
        assert_eq!(count, 2);

        // By-value iteration
        let drained: Vec<MockEntity> = list.into_iter().collect();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].name, "A");
    }

    #[test]
    fn test_v2_page_info_defaults() {
        let info = V2PageInfo::default();
        assert_eq!(info.total_count, None);
        assert_eq!(info.has_more, None);

        let json = serde_json::to_string(&info).unwrap();
        assert_eq!(json, "{}");
    }
}
