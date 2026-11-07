//! 3-state `Nullable<T>` container for Stripe parameter mutation semantics.
//!
//! # States
//! - `Nullable::Value(T)`: Emits the serialized value.
//! - `Nullable::Null`: Emits an empty string (`key=""`) in API v1 form-encoded requests
//!   or `null` in API v2 JSON requests, clearing the stored value on Stripe servers.
//! - `Nullable::Omit`: Skips the field entirely during serialization.
//!
//! # Mandatory Serde Invariant
//!
//! When defining request parameter structs containing [`Nullable<T>`], annotate
//! the struct field with `#[serde(default, skip_serializing_if = "Nullable::is_omit")]`.
//!
//! ```rust
//! use serde::Serialize;
//! use some_stripe_types::Nullable;
//!
//! #[derive(Serialize, Default)]
//! struct UpdateCustomerParams<'a> {
//!     #[serde(default, skip_serializing_if = "Nullable::is_omit")]
//!     description: Nullable<&'a str>,
//! }
//!
//! // 1. Omit state: field is omitted entirely from JSON / form payload
//! let omit_params = UpdateCustomerParams::default();
//! let json = serde_json::to_string(&omit_params).unwrap();
//! assert_eq!(json, "{}");
//!
//! // 2. Null state: field is emitted as null / empty string (clears remote field)
//! let null_params = UpdateCustomerParams { description: Nullable::Null };
//! let json = serde_json::to_string(&null_params).unwrap();
//! assert_eq!(json, r#"{"description":null}"#);
//!
//! // 3. Value state: field emits updated concrete value
//! let val_params = UpdateCustomerParams { description: Nullable::Value("VIP Client") };
//! let json = serde_json::to_string(&val_params).unwrap();
//! assert_eq!(json, r#"{"description":"VIP Client"}"#);
//! ```

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A three-state container modeling updatable Stripe fields.
///
/// Distinguishes between omitting a field (`Omit`), explicitly resetting/clearing it (`Null`),
/// and updating it to a new concrete value (`Value(T)`).
///
/// # Examples
///
/// ```
/// use some_stripe_types::Nullable;
///
/// let val = Nullable::Value("cus_123");
/// assert!(val.is_value());
/// assert_eq!(val.into_value(), Some("cus_123"));
///
/// let null_val: Nullable<String> = Nullable::Null;
/// assert!(null_val.is_null());
/// assert_eq!(null_val.into_value(), None);
///
/// let omit_val: Nullable<String> = Nullable::Omit;
/// assert!(omit_val.is_omit());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Nullable<T> {
    /// Omitted from serialization.
    #[default]
    Omit,
    /// Explicitly set to null (clears the remote Stripe field).
    Null,
    /// Explicitly set to a concrete value.
    Value(T),
}

impl<T> Nullable<T> {
    /// Returns `true` if the field is omitted.
    #[inline]
    pub const fn is_omit(&self) -> bool {
        matches!(self, Self::Omit)
    }

    /// Returns `true` if the field is explicitly cleared.
    #[inline]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns `true` if the field contains a concrete value.
    #[inline]
    pub const fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    /// Borrows the contained value if present.
    pub fn as_ref(&self) -> Nullable<&T> {
        match self {
            Self::Omit => Nullable::Omit,
            Self::Null => Nullable::Null,
            Self::Value(v) => Nullable::Value(v),
        }
    }

    /// Converts into an `Option<T>`, keeping only `Value(T)`.
    pub fn into_value(self) -> Option<T> {
        match self {
            Self::Value(v) => Some(v),
            _ => None,
        }
    }

    /// Returns an `Option<&T>` referencing the inner value if `Value(T)`.
    pub fn as_option(&self) -> Option<&T> {
        match self {
            Self::Value(v) => Some(v),
            _ => None,
        }
    }

    /// Maps `Nullable<T>` to `Nullable<U>`.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Nullable<U> {
        match self {
            Self::Omit => Nullable::Omit,
            Self::Null => Nullable::Null,
            Self::Value(v) => Nullable::Value(f(v)),
        }
    }
}

impl<T> From<T> for Nullable<T> {
    fn from(val: T) -> Self {
        Self::Value(val)
    }
}

impl<T> From<Option<T>> for Nullable<T> {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => Self::Value(v),
            None => Self::Omit,
        }
    }
}

impl<T: Serialize> Serialize for Nullable<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Omit => serializer.serialize_none(),
            Self::Null => serializer.serialize_none(),
            Self::Value(v) => v.serialize(serializer),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Nullable<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|opt| match opt {
            Some(v) => Nullable::Value(v),
            None => Nullable::Null,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nullable_states() {
        let v: Nullable<i32> = Nullable::Value(10);
        assert!(v.is_value());
        assert!(!v.is_null());
        assert!(!v.is_omit());
        assert_eq!(v.into_value(), Some(10));
        assert_eq!(v.as_option(), Some(&10));

        let n: Nullable<i32> = Nullable::Null;
        assert!(!n.is_value());
        assert!(n.is_null());
        assert!(!n.is_omit());
        assert_eq!(n.into_value(), None);
        assert_eq!(n.as_option(), None);

        let o: Nullable<i32> = Nullable::Omit;
        assert!(!o.is_value());
        assert!(!o.is_null());
        assert!(o.is_omit());
        assert_eq!(o.into_value(), None);
        assert_eq!(o.as_option(), None);
    }

    #[test]
    fn test_nullable_conversions() {
        let from_val: Nullable<String> = "hello".to_string().into();
        assert_eq!(from_val, Nullable::Value("hello".to_string()));

        let from_some: Nullable<i32> = Some(42).into();
        assert_eq!(from_some, Nullable::Value(42));

        let from_none: Nullable<i32> = None.into();
        assert_eq!(from_none, Nullable::Omit);
    }
}
