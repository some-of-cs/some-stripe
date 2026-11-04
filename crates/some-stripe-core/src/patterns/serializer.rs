//! Dual-protocol serialization engine and 3-state [`Nullable`] type system for the Stripe API.

use crate::error::StripeError;
use crate::patterns::nullable::Nullable;
use serde::Serialize;

/// Encodes a string slice using RFC 3986 percent-encoding.
#[must_use]
pub fn percent_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";
    for &byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push('%');
                encoded.push(HEX_CHARS[(byte >> 4) as usize] as char);
                encoded.push(HEX_CHARS[(byte & 0x0F) as usize] as char);
            }
        }
    }
    encoded
}

/// Serializes a data structure into a list of key-value tuples representing
/// Stripe API v1 form parameters.
pub fn to_form_params<T: Serialize>(value: &T) -> Result<Vec<(String, String)>, StripeError> {
    let json_val = serde_json::to_value(value)?;
    match json_val {
        serde_json::Value::Object(map) => {
            let mut params = Vec::new();
            for (k, v) in map {
                flatten_value(&k, &v, &mut params, 0)?;
            }
            Ok(params)
        }
        _ => Err(StripeError::InvalidRequest(
            "Top-level form parameters must serialize to a key-value object".into(),
        )),
    }
}

const MAX_SERIALIZATION_DEPTH: usize = 16;

fn flatten_value(
    prefix: &str,
    value: &serde_json::Value,
    out: &mut Vec<(String, String)>,
    depth: usize,
) -> Result<(), StripeError> {
    if depth > MAX_SERIALIZATION_DEPTH {
        return Err(StripeError::InvalidRequest(
            "Serialization recursion depth exceeded maximum limit (16 levels)".into(),
        ));
    }

    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let sub_key = format!("{prefix}[{k}]");
                flatten_value(&sub_key, v, out, depth + 1)?;
            }
        }
        serde_json::Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                let sub_key = format!("{prefix}[{idx}]");
                flatten_value(&sub_key, item, out, depth + 1)?;
            }
        }
        serde_json::Value::Null => {
            out.push((prefix.to_string(), String::new()));
        }
        serde_json::Value::Bool(b) => {
            out.push((prefix.to_string(), b.to_string()));
        }
        serde_json::Value::Number(n) => {
            out.push((prefix.to_string(), n.to_string()));
        }
        serde_json::Value::String(s) => {
            out.push((prefix.to_string(), s.clone()));
        }
    }
    Ok(())
}

/// Serializes a data structure into an RFC 3986 compliant `application/x-www-form-urlencoded` string.
pub fn to_form_urlencoded<T: Serialize>(value: &T) -> Result<String, StripeError> {
    let params = to_form_params(value)?;
    let mut encoded = String::new();
    for (i, (k, v)) in params.into_iter().enumerate() {
        if i > 0 {
            encoded.push('&');
        }
        encoded.push_str(&percent_encode(&k));
        encoded.push('=');
        encoded.push_str(&percent_encode(&v));
    }
    Ok(encoded)
}

/// Alias for [`to_form_urlencoded`] matching the Stripe API v1 protocol contract.
#[inline]
pub fn serialize_v1_form<T: Serialize>(value: &T) -> Result<String, StripeError> {
    to_form_urlencoded(value)
}

/// Serializes a request DTO into an `application/json` payload for Stripe API v2 operations.
#[inline]
pub fn serialize_v2_json<T: Serialize>(value: &T) -> Result<String, StripeError> {
    serde_json::to_string(value).map_err(StripeError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_zero_indexed_bracket_notation() {
        #[derive(Serialize)]
        struct Item {
            id: String,
            amount: i64,
        }

        #[derive(Serialize)]
        struct Batch {
            items: Vec<Item>,
        }

        let batch = Batch {
            items: vec![
                Item { id: "si_1".into(), amount: 100 },
                Item { id: "si_2".into(), amount: 200 },
            ],
        };

        let pairs = to_form_params(&batch).unwrap();
        assert_eq!(
            pairs,
            vec![
                ("items[0][id]".to_string(), "si_1".to_string()),
                ("items[0][amount]".to_string(), "100".to_string()),
                ("items[1][id]".to_string(), "si_2".to_string()),
                ("items[1][amount]".to_string(), "200".to_string()),
            ]
        );
    }
}
