//! Reusable pagination structures, parameters, and stream adapters for Stripe list endpoints.
//!
//! # Purpose
//! Standardizes cursor-based collection responses (`object: "list"`) across all Stripe resource domains,
//! eliminating redundant response envelope modeling.

use serde::{Deserialize, Serialize};

/// Generic envelope returned by Stripe cursor-based list endpoints.
///
/// # Examples
///
/// ```
/// use some_stripe::patterns::pagination::ListResponse;
///
/// let json = r#"{"object":"list","data":["item1","item2"],"has_more":false,"url":"/v1/items"}"#;
/// let list: ListResponse<String> = serde_json::from_str(json).unwrap();
/// assert_eq!(list.data.len(), 2);
/// assert!(!list.has_more);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    /// Object type, always `"list"`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// Array containing the queried resource elements.
    pub data: Vec<T>,

    /// Whether there are more elements available through subsequent cursor pagination.
    pub has_more: bool,

    /// URL path where this list was queried.
    pub url: String,
}

fn default_list_object() -> String {
    "list".into()
}

/// Common query parameters accepted by Stripe cursor-based list endpoints.
///
/// # Examples
///
/// ```
/// use some_stripe::patterns::pagination::ListParams;
///
/// let params = ListParams::new().limit(50).starting_after("cus_123");
/// assert_eq!(params.limit, Some(50));
/// assert_eq!(params.starting_after.as_deref(), Some("cus_123"));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListParams {
    /// A limit on the number of objects to be returned (1 to 100). Default is 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// A cursor for pagination: an object ID defining your place in the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A cursor for backwards pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListParams {
    /// Creates a new empty parameter set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of items to return (clamped between 1 and 100).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit.clamp(1, 100));
        self
    }

    /// Sets the cursor to fetch items after the specified object ID.
    pub fn starting_after(mut self, id: impl Into<String>) -> Self {
        self.starting_after = Some(id.into());
        self
    }

    /// Sets the cursor to fetch items before the specified object ID.
    pub fn ending_before(mut self, id: impl Into<String>) -> Self {
        self.ending_before = Some(id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_response_deserialization() {
        let json = r#"{
            "object": "list",
            "data": [
                {"id": "pi_1", "amount": 1000},
                {"id": "pi_2", "amount": 2000}
            ],
            "has_more": true,
            "url": "/v1/payment_intents"
        }"#;

        #[derive(Deserialize, PartialEq, Debug)]
        struct DummyItem {
            id: String,
            amount: i64,
        }

        let resp: ListResponse<DummyItem> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 2);
        assert!(resp.has_more);
        assert_eq!(resp.data[0].id, "pi_1");
    }

    #[test]
    fn test_list_params_builder() {
        let params = ListParams::new()
            .limit(150) // Should clamp to 100
            .starting_after("obj_xyz");

        assert_eq!(params.limit, Some(100));
        assert_eq!(params.starting_after, Some("obj_xyz".into()));
        assert!(params.ending_before.is_none());
    }
}
