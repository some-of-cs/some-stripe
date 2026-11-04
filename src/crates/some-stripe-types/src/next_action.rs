//! Strongly typed 3D Secure (3DS) and PSD2 Strong Customer Authentication (SCA) `NextAction` parser.
//!
//! Complies with Gate G-Law and PSD2 Regulatory Technical Standards (RTS) under Article 98
//! by providing strongly typed models for customer authentication challenge redirects and
//! Stripe SDK native invocation payloads.

use serde::{Deserialize, Serialize};

/// Discriminated action type for a PaymentIntent requiring customer action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionType {
    /// Customer must be redirected to an external URL (e.g. 3DS challenge page or hosted page).
    RedirectToUrl,

    /// Customer authentication must be executed using the Stripe mobile/web client SDK.
    UseStripeSdk,

    /// Bank transfer instructions or payment voucher should be displayed to the customer.
    DisplayBankTransferInstructions,

    /// WeChat Pay QR code should be displayed to the customer.
    WechatPayDisplayQrCode,

    /// Customer must verify microdeposits into their bank account.
    VerifyWithMicrodeposits,

    /// Fallback for future or unrecognized next action types.
    #[serde(other)]
    Unknown,
}

impl NextActionType {
    /// Returns `true` if this action is [`NextActionType::RedirectToUrl`].
    pub fn is_redirect(&self) -> bool {
        matches!(self, Self::RedirectToUrl)
    }

    /// Returns `true` if this action is [`NextActionType::UseStripeSdk`].
    pub fn is_stripe_sdk(&self) -> bool {
        matches!(self, Self::UseStripeSdk)
    }

    /// Returns the canonical snake_case string representation of the action type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RedirectToUrl => "redirect_to_url",
            Self::UseStripeSdk => "use_stripe_sdk",
            Self::DisplayBankTransferInstructions => "display_bank_transfer_instructions",
            Self::WechatPayDisplayQrCode => "wechat_pay_display_qr_code",
            Self::VerifyWithMicrodeposits => "verify_with_microdeposits",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for NextActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Challenge redirect URL configuration for 3D Secure PSD2 compliance (Gate G-Law / PSD2 Article 98).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedirectToUrl {
    /// The URL you must redirect your customer to in order to authenticate the payment.
    pub url: String,

    /// The URL that the customer will be directed to after the authentication is complete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,
}

impl RedirectToUrl {
    /// Creates a new [`RedirectToUrl`] with the specified URL and optional return URL.
    pub fn new(url: impl Into<String>, return_url: Option<impl Into<String>>) -> Self {
        Self {
            url: url.into(),
            return_url: return_url.map(Into::into),
        }
    }

    /// Validates that the challenge redirect URL is well-formed (Gate G-Law / PSD2 Article 98).
    pub fn is_valid_url(&self) -> bool {
        self.url.starts_with("https://") || self.url.starts_with("http://")
    }
}

/// Strongly typed 3D Secure PSD2 NextAction payload.
///
/// Implements PSD2 Regulatory Technical Standards (RTS) under Article 98 for Strong Customer
/// Authentication (SCA) challenges, specifying redirect flows or SDK execution hooks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NextAction {
    /// Type of the next action to perform.
    pub r#type: NextActionType,

    /// Details for redirecting the customer to an external 3DS authentication page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect_to_url: Option<RedirectToUrl>,

    /// Raw payload containing parameters required by the Stripe SDK.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_stripe_sdk: Option<serde_json::Value>,
}

impl NextAction {
    /// Returns `true` if this action requires a browser redirect to complete 3DS challenge.
    pub fn is_redirect(&self) -> bool {
        self.r#type.is_redirect() && self.redirect_to_url.is_some()
    }

    /// Returns `true` if this action should be handed over to the Stripe client SDK.
    pub fn is_stripe_sdk(&self) -> bool {
        self.r#type.is_stripe_sdk() && self.use_stripe_sdk.is_some()
    }

    /// Convenience accessor for the 3DS redirect challenge URL.
    pub fn redirect_url(&self) -> Option<&str> {
        self.redirect_to_url.as_ref().map(|r| r.url.as_str())
    }

    /// Convenience accessor for the post-authentication return URL.
    pub fn return_url(&self) -> Option<&str> {
        self.redirect_to_url
            .as_ref()
            .and_then(|r| r.return_url.as_deref())
    }
}

impl TryFrom<&serde_json::Value> for NextAction {
    type Error = serde_json::Error;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        NextAction::deserialize(value)
    }
}

impl TryFrom<serde_json::Value> for NextAction {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_3ds_redirect_next_action() {
        let json = serde_json::json!({
            "type": "redirect_to_url",
            "redirect_to_url": {
                "url": "https://hooks.stripe.com/3d_secure_2/hosted",
                "return_url": "https://merchant.example.com/checkout/return"
            }
        });

        let action: NextAction = serde_json::from_value(json).expect("Failed to parse next_action");
        assert_eq!(action.r#type, NextActionType::RedirectToUrl);
        assert!(action.is_redirect());
        assert!(!action.is_stripe_sdk());
        assert_eq!(
            action.redirect_url(),
            Some("https://hooks.stripe.com/3d_secure_2/hosted")
        );
        assert_eq!(
            action.return_url(),
            Some("https://merchant.example.com/checkout/return")
        );
        let redirect = action.redirect_to_url.as_ref().unwrap();
        assert!(redirect.is_valid_url());
    }

    #[test]
    fn test_parse_use_stripe_sdk_next_action() {
        let json = serde_json::json!({
            "type": "use_stripe_sdk",
            "use_stripe_sdk": {
                "type": "stripe_3ds2_fingerprint",
                "directory_server_name": "visa"
            }
        });

        let action: NextAction = serde_json::from_value(json).expect("Failed to parse next_action");
        assert_eq!(action.r#type, NextActionType::UseStripeSdk);
        assert!(!action.is_redirect());
        assert!(action.is_stripe_sdk());
        assert!(action.use_stripe_sdk.is_some());
    }

    #[test]
    fn test_unknown_next_action_fallback() {
        let json = serde_json::json!({
            "type": "future_flow_unknown_action"
        });

        let action: NextAction = serde_json::from_value(json).expect("Should fallback cleanly");
        assert_eq!(action.r#type, NextActionType::Unknown);
        assert!(!action.is_redirect());
    }
}
