//! 3-state `Nullable<T>` container for Stripe parameter mutation semantics.
//!
//! # States
//! - `Nullable::Value(T)`: Emits the serialized value.
//! - `Nullable::Null`: Emits an empty string (`key=""`) in API v1 form-encoded requests
//!   or `null` in API v2 JSON requests, clearing the stored value on Stripe servers.
//! - `Nullable::Omit`: Skips the field entirely during serialization.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A three-state container modeling updatable Stripe fields.
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

        let n: Nullable<i32> = Nullable::Null;
        assert!(!n.is_value());
        assert!(n.is_null());
        assert!(!n.is_omit());
        assert_eq!(n.into_value(), None);

        let o: Nullable<i32> = Nullable::Omit;
        assert!(!o.is_value());
        assert!(!o.is_null());
        assert!(o.is_omit());
        assert_eq!(o.into_value(), None);
    }
}
