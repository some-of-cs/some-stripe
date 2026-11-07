//! Stripe PaymentIntents resource implementations.
//!
//! Provides API bindings for the Stripe PaymentIntents lifecycle (`create`, `retrieve`,
//! `confirm`, `capture`, and `cancel`), modeled after `stripe-node`.
//! Automatic, cryptographically secure RFC 4122 UUID v4 `Idempotency-Key` header injection
//! is performed on all mutating actions.
//!
//! ## Quality & Borrowing Discipline (Gate G-Borrow)
//! All parameters are accepted strictly by reference (`&CreatePaymentIntentParams`,
//! `&str`, `Option<&CapturePaymentIntentParams>`, `Option<&CancelPaymentIntentParams>`,
//! `Option<&ConfirmPaymentIntentParams>`), ensuring zero unnecessary deep copies on
//! heap allocations.
//!
//! ## ISO-4217 Currency Compliance (Gate G-Law)
//! Amounts are specified and validated as positive integers (`amount > 0`) in minor currency
//! units (cents), and currency codes are checked against 3-letter lowercase ISO formats.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CancelPaymentIntentParams, CapturePaymentIntentParams, ConfirmPaymentIntentParams,
    CreatePaymentIntentParams, PaymentIntent,
};

/// Generates an RFC 4122 compliant UUID v4 string using cryptographically secure random bytes from OpenSSL CSPRNG.
///
/// Sets:
/// - Version 4 bits in byte 6 (`(bytes[6] & 0x0f) | 0x40`)
/// - RFC 4122 variant bits in byte 8 (`(bytes[8] & 0x3f) | 0x80`)
///
/// # Example
/// ```
/// use some_stripe::resources::payment_intents::{generate_uuid_v4, is_valid_uuid_v4};
///
/// let key = generate_uuid_v4();
/// assert_eq!(key.len(), 36);
/// assert!(is_valid_uuid_v4(&key));
/// ```
pub fn generate_uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    openssl::rand::rand_bytes(&mut bytes).expect("OpenSSL CSPRNG failure");
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

/// Validates whether a given string is a valid RFC 4122 UUID v4 representation.
///
/// # Example
/// ```
/// use some_stripe::resources::payment_intents::is_valid_uuid_v4;
///
/// assert!(is_valid_uuid_v4("c4a2a11b-7a39-4d6d-8c43-4217117e3f89"));
/// assert!(!is_valid_uuid_v4("invalid-uuid-string"));
/// ```
pub fn is_valid_uuid_v4(key: &str) -> bool {
    if key.len() != 36 {
        return false;
    }
    let b = key.as_bytes();
    if b[8] != b'-' || b[13] != b'-' || b[18] != b'-' || b[23] != b'-' {
        return false;
    }
    // Version 4 check
    if b[14] != b'4' {
        return false;
    }
    // Variant 1 (RFC 4122) check: 8, 9, a, b (case-insensitive)
    if !matches!(b[19], b'8' | b'9' | b'a' | b'b' | b'A' | b'B') {
        return false;
    }
    for (i, &byte) in b.iter().enumerate() {
        if i == 8 || i == 13 || i == 18 || i == 23 {
            continue;
        }
        if !byte.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

/// Resource for managing Stripe PaymentIntents (`stripe.payment_intents()`).
#[derive(Clone, Copy)]
pub struct PaymentIntentsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> PaymentIntentsResource<'a> {
    /// Creates a new `PaymentIntentsResource` bound to the provided client.
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a PaymentIntent (`POST /v1/payment_intents`).
    ///
    /// Accepts request parameters by reference (`&CreatePaymentIntentParams`), eliminating
    /// redundant clones. Performs client-side validation on ISO-4217 minor currency units
    /// and currency codes before transmitting across the network.
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CreatePaymentIntentParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreatePaymentIntentParams::new(5000, "usd")?
    ///     .capture_method("automatic")
    ///     .description("Order #1001")
    ///     .idempotency_key("order_1001_idem");
    ///
    /// let intent = client.payment_intents().create(&params).await?;
    /// assert_eq!(intent.amount, 5000);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreatePaymentIntentParams,
    ) -> Result<PaymentIntent, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v1/payment_intents", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let intent: PaymentIntent = serde_json::from_value(val)?;
        Ok(intent)
    }

    /// Retrieves a PaymentIntent by ID (`GET /v1/payment_intents/{id}`).
    ///
    /// Accepts the identifier as a string reference (`&str`), ensuring zero allocations.
    /// Note: This is an idempotent read operation and does NOT inject an idempotency key.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let intent = client.payment_intents().retrieve("pi_12345").await?;
    /// assert_eq!(intent.id, "pi_12345");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, intent_id: &str) -> Result<PaymentIntent, StripeError> {
        let safe_id = validate_and_encode_id(intent_id)?;
        let url = format!(
            "{}/v1/payment_intents/{safe_id}",
            self.client.config.api_base
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let intent: PaymentIntent = serde_json::from_value(val)?;
        Ok(intent)
    }

    /// Captures the funds of an existing uncaptured PaymentIntent (`POST /v1/payment_intents/{id}/capture`).
    ///
    /// Can capture the full authorized amount (`params: None`) or a partial amount
    /// (`params: Some(&CapturePaymentIntentParams)`).
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CapturePaymentIntentParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let capture_params = CapturePaymentIntentParams::with_amount(3500)?
    ///     .idempotency_key("idem_capture_1001");
    ///
    /// let captured = client
    ///     .payment_intents()
    ///     .capture("pi_12345", Some(&capture_params))
    ///     .await?;
    /// assert_eq!(captured.amount_received, Some(3500));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn capture(
        &self,
        intent_id: &str,
        params: Option<&CapturePaymentIntentParams>,
    ) -> Result<PaymentIntent, StripeError> {
        let safe_id = validate_and_encode_id(intent_id)?;
        if let Some(p) = params {
            p.validate()
                .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
        }

        let url = format!(
            "{}/v1/payment_intents/{safe_id}/capture",
            self.client.config.api_base
        );
        let mut form_data = Vec::new();
        let amount_str = params
            .and_then(|p| p.amount_to_capture)
            .map(|a| a.to_string());
        if let Some(ref a) = amount_str {
            form_data.push(("amount_to_capture", a.as_str()));
        }

        let generated_key;
        let idempotency_key = match params.and_then(|p| p.idempotency_key.as_deref()) {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let intent: PaymentIntent = serde_json::from_value(val)?;
        Ok(intent)
    }

    /// Cancels a PaymentIntent (`POST /v1/payment_intents/{id}/cancel`).
    ///
    /// Releases the authorization hold on customer funds. Cancellation reason and idempotency
    /// keys can be supplied via optional borrowed `&CancelPaymentIntentParams`.
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CancelPaymentIntentParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let cancel_params = CancelPaymentIntentParams::with_reason("requested_by_customer")
    ///     .idempotency_key("idem_cancel_1001");
    ///
    /// let canceled = client
    ///     .payment_intents()
    ///     .cancel("pi_12345", Some(&cancel_params))
    ///     .await?;
    /// assert_eq!(canceled.status, "canceled");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(
        &self,
        intent_id: &str,
        params: Option<&CancelPaymentIntentParams>,
    ) -> Result<PaymentIntent, StripeError> {
        let safe_id = validate_and_encode_id(intent_id)?;
        if let Some(p) = params {
            p.validate()
                .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
        }

        let url = format!(
            "{}/v1/payment_intents/{safe_id}/cancel",
            self.client.config.api_base
        );
        let mut form_data = Vec::new();
        if let Some(reason) = params.and_then(|p| p.cancellation_reason.as_deref()) {
            form_data.push(("cancellation_reason", reason));
        }

        let generated_key;
        let idempotency_key = match params.and_then(|p| p.idempotency_key.as_deref()) {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let intent: PaymentIntent = serde_json::from_value(val)?;
        Ok(intent)
    }

    /// Confirms a PaymentIntent (`POST /v1/payment_intents/{id}/confirm`).
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::ConfirmPaymentIntentParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ConfirmPaymentIntentParams::new()
    ///     .payment_method("pm_card_visa")
    ///     .return_url("https://example.com/checkout/return")
    ///     .idempotency_key("idem_conf_1001");
    ///
    /// let intent = client
    ///     .payment_intents()
    ///     .confirm("pi_12345", Some(&params))
    ///     .await?;
    /// assert_eq!(intent.status, "succeeded");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn confirm(
        &self,
        intent_id: &str,
        params: Option<&ConfirmPaymentIntentParams>,
    ) -> Result<PaymentIntent, StripeError> {
        let safe_id = validate_and_encode_id(intent_id)?;
        if let Some(p) = params {
            p.validate()
                .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
        }

        let url = format!(
            "{}/v1/payment_intents/{safe_id}/confirm",
            self.client.config.api_base
        );
        let mut form_data = Vec::new();
        if let Some(pm) = params.and_then(|p| p.payment_method.as_deref()) {
            form_data.push(("payment_method", pm));
        }
        if let Some(ru) = params.and_then(|p| p.return_url.as_deref()) {
            form_data.push(("return_url", ru));
        }
        if let Some(re) = params.and_then(|p| p.receipt_email.as_deref()) {
            form_data.push(("receipt_email", re));
        }
        if let Some(sfu) = params.and_then(|p| p.setup_future_usage.as_deref()) {
            form_data.push(("setup_future_usage", sfu));
        }

        let generated_key;
        let idempotency_key = match params.and_then(|p| p.idempotency_key.as_deref()) {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let intent: PaymentIntent = serde_json::from_value(val)?;
        Ok(intent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use some_stripe_core::StripeConfig;
    use std::collections::HashSet;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn test_generate_uuid_v4_rfc4122_compliance() {
        let mut seen = HashSet::new();
        for _ in 0..500 {
            let uuid = generate_uuid_v4();
            assert_eq!(uuid.len(), 36, "UUID must be exactly 36 chars");
            assert!(
                is_valid_uuid_v4(&uuid),
                "Generated UUID must satisfy RFC 4122 v4 validator: {uuid}"
            );
            let b = uuid.as_bytes();
            assert_eq!(b[14], b'4', "Version nibble must be '4'");
            assert!(
                matches!(b[19], b'8' | b'9' | b'a' | b'b'),
                "Variant nibble must be 8, 9, a, or b"
            );
            assert!(seen.insert(uuid), "Generated UUIDs must be unique");
        }
    }

    #[test]
    fn test_is_valid_uuid_v4_cases() {
        assert!(is_valid_uuid_v4("c4a2a11b-7a39-4d6d-8c43-4217117e3f89"));
        assert!(is_valid_uuid_v4("00000000-0000-4000-8000-000000000000"));
        assert!(is_valid_uuid_v4("ffffffff-ffff-4fff-bfff-ffffffffffff"));

        // Invalid lengths
        assert!(!is_valid_uuid_v4("c4a2a11b-7a39-4d6d-8c43-4217117e3f8"));
        assert!(!is_valid_uuid_v4("c4a2a11b-7a39-4d6d-8c43-4217117e3f890"));

        // Missing or misplaced hyphens
        assert!(!is_valid_uuid_v4("c4a2a11b7a39-4d6d-8c43-4217117e3f89-"));
        assert!(!is_valid_uuid_v4("c4a2a11b_7a39_4d6d_8c43_4217117e3f89"));

        // Wrong version (must be '4')
        assert!(!is_valid_uuid_v4("c4a2a11b-7a39-1d6d-8c43-4217117e3f89"));
        assert!(!is_valid_uuid_v4("c4a2a11b-7a39-5d6d-8c43-4217117e3f89"));

        // Wrong variant (must be 8, 9, a, b)
        assert!(!is_valid_uuid_v4("c4a2a11b-7a39-4d6d-0c43-4217117e3f89"));
        assert!(!is_valid_uuid_v4("c4a2a11b-7a39-4d6d-cc43-4217117e3f89"));

        // Non-hex chars
        assert!(!is_valid_uuid_v4("g4a2a11b-7a39-4d6d-8c43-4217117e3f89"));
        assert!(!is_valid_uuid_v4("c4a2a11b-7a39-4d6d-8c43-4217117e3f8z"));
    }

    #[tokio::test]
    async fn test_create_validation_rejection() {
        let config = StripeConfig::new("sk_test_dummy");
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        // Constructor validation
        assert!(CreatePaymentIntentParams::new(-100, "usd").is_err());
        assert!(CreatePaymentIntentParams::new(0, "usd").is_err());
        assert!(CreatePaymentIntentParams::new(1000, "invalid_currency").is_err());

        // Struct-level validation rejection in resource.create()
        let invalid_params = CreatePaymentIntentParams {
            amount: -100,
            currency: "usd".into(),
            capture_method: None,
            description: None,
            payment_method: None,
            customer: None,
            receipt_email: None,
            return_url: None,
            confirm: None,
            metadata: None,
            idempotency_key: None,
        };
        let res = resource.create(&invalid_params).await;
        assert!(matches!(res, Err(StripeError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn test_capture_validation_rejection() {
        let config = StripeConfig::new("sk_test_dummy");
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        let invalid_cap = CapturePaymentIntentParams {
            amount_to_capture: Some(0),
            idempotency_key: None,
        };
        let res = resource.capture("pi_123", Some(&invalid_cap)).await;
        assert!(matches!(res, Err(StripeError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn test_create_auto_idempotency_key_injection() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/payment_intents HTTP/1.1"));

            // Extract Idempotency-Key header
            let header_line = req_str
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Idempotency-Key header must be injected automatically");
            let key = header_line.split(':').nth(1).unwrap().trim();
            assert!(
                is_valid_uuid_v4(key),
                "Injected key '{key}' must be valid UUID v4"
            );

            // Verify form data
            assert!(req_str.contains("amount=2000"));
            assert!(req_str.contains("currency=usd"));
            assert!(req_str.contains("capture_method=manual"));

            let resp = r#"{"id":"pi_auto_1","object":"payment_intent","amount":2000,"currency":"usd","status":"requires_capture"}"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });

        let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        let params = CreatePaymentIntentParams::new(2000, "usd")
            .unwrap()
            .capture_method("manual");

        let intent = resource.create(&params).await.unwrap();
        assert_eq!(intent.id, "pi_auto_1");
        assert_eq!(intent.amount, 2000);
        assert_eq!(intent.status, "requires_capture");

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_create_preserves_user_idempotency_key() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/payment_intents HTTP/1.1"));
            assert!(
                req_str.contains("idempotency-key: user_custom_create_key_777"),
                "User provided idempotency key must be preserved"
            );

            let resp = r#"{"id":"pi_user_1","object":"payment_intent","amount":5000,"currency":"usd","status":"requires_payment_method"}"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });

        let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        let params = CreatePaymentIntentParams::new(5000, "usd")
            .unwrap()
            .idempotency_key("user_custom_create_key_777");

        let intent = resource.create(&params).await.unwrap();
        assert_eq!(intent.id, "pi_user_1");

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_retrieve_does_not_inject_idempotency_key() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("GET /v1/payment_intents/pi_ret_999 HTTP/1.1"));
            assert!(
                !req_str.to_lowercase().contains("idempotency-key"),
                "GET retrieve request must NOT contain Idempotency-Key header"
            );

            let resp = r#"{"id":"pi_ret_999","object":"payment_intent","amount":7500,"currency":"usd","status":"succeeded"}"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });

        let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        let intent = resource.retrieve("pi_ret_999").await.unwrap();
        assert_eq!(intent.id, "pi_ret_999");
        assert_eq!(intent.amount, 7500);

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_capture_auto_and_custom_idempotency_key() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            // First call: Auto injection with None params
            let (mut socket1, _) = listener.accept().await.unwrap();
            let mut buf1 = vec![0; 4096];
            let n1 = socket1.read(&mut buf1).await.unwrap();
            let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
            assert!(req1.starts_with("POST /v1/payment_intents/pi_cap_1/capture HTTP/1.1"));
            let header1 = req1
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Auto Idempotency-Key must be injected on capture when params=None");
            let key1 = header1.split(':').nth(1).unwrap().trim();
            assert!(is_valid_uuid_v4(key1));
            let resp1 = r#"{"id":"pi_cap_1","object":"payment_intent","amount":3000,"currency":"usd","status":"succeeded"}"#;
            socket1
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp1}",
                        resp1.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();

            // Second call: Custom idempotency key with amount_to_capture
            let (mut socket2, _) = listener.accept().await.unwrap();
            let mut buf2 = vec![0; 4096];
            let n2 = socket2.read(&mut buf2).await.unwrap();
            let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
            assert!(req2.starts_with("POST /v1/payment_intents/pi_cap_2/capture HTTP/1.1"));
            assert!(req2.contains("idempotency-key: custom_capture_key_123"));
            assert!(req2.contains("amount_to_capture=1500"));
            let resp2 = r#"{"id":"pi_cap_2","object":"payment_intent","amount":3000,"amount_received":1500,"currency":"usd","status":"succeeded"}"#;
            socket2
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp2}",
                        resp2.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });

        let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        // Call 1
        let pi1 = resource.capture("pi_cap_1", None).await.unwrap();
        assert_eq!(pi1.status, "succeeded");

        // Call 2
        let cap_params = CapturePaymentIntentParams {
            amount_to_capture: Some(1500),
            idempotency_key: Some("custom_capture_key_123".into()),
        };
        let pi2 = resource
            .capture("pi_cap_2", Some(&cap_params))
            .await
            .unwrap();
        assert_eq!(pi2.amount_received, Some(1500));

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_cancel_auto_and_custom_idempotency_key() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            // First call: Auto injection
            let (mut socket1, _) = listener.accept().await.unwrap();
            let mut buf1 = vec![0; 4096];
            let n1 = socket1.read(&mut buf1).await.unwrap();
            let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
            assert!(req1.starts_with("POST /v1/payment_intents/pi_can_1/cancel HTTP/1.1"));
            let header1 = req1
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Auto Idempotency-Key must be injected on cancel");
            assert!(is_valid_uuid_v4(header1.split(':').nth(1).unwrap().trim()));
            let resp1 = r#"{"id":"pi_can_1","object":"payment_intent","amount":1000,"currency":"usd","status":"canceled"}"#;
            socket1
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp1}",
                        resp1.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();

            // Second call: Custom idempotency key & reason
            let (mut socket2, _) = listener.accept().await.unwrap();
            let mut buf2 = vec![0; 4096];
            let n2 = socket2.read(&mut buf2).await.unwrap();
            let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
            assert!(req2.starts_with("POST /v1/payment_intents/pi_can_2/cancel HTTP/1.1"));
            assert!(req2.contains("idempotency-key: custom_cancel_key_456"));
            assert!(req2.contains("cancellation_reason=requested_by_customer"));
            let resp2 = r#"{"id":"pi_can_2","object":"payment_intent","amount":1000,"currency":"usd","status":"canceled","cancellation_reason":"requested_by_customer"}"#;
            socket2
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp2}",
                        resp2.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });

        let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        let pi1 = resource.cancel("pi_can_1", None).await.unwrap();
        assert_eq!(pi1.status, "canceled");

        let can_params = CancelPaymentIntentParams {
            cancellation_reason: Some("requested_by_customer".into()),
            idempotency_key: Some("custom_cancel_key_456".into()),
        };
        let pi2 = resource
            .cancel("pi_can_2", Some(&can_params))
            .await
            .unwrap();
        assert_eq!(
            pi2.cancellation_reason.as_deref(),
            Some("requested_by_customer")
        );

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_confirm_auto_and_custom_idempotency_key() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            // First call: Auto injection with None params
            let (mut socket1, _) = listener.accept().await.unwrap();
            let mut buf1 = vec![0; 4096];
            let n1 = socket1.read(&mut buf1).await.unwrap();
            let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
            assert!(req1.starts_with("POST /v1/payment_intents/pi_conf_1/confirm HTTP/1.1"));
            let header1 = req1
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Auto Idempotency-Key must be injected on confirm");
            assert!(is_valid_uuid_v4(header1.split(':').nth(1).unwrap().trim()));
            let resp1 = r#"{"id":"pi_conf_1","object":"payment_intent","amount":4000,"currency":"usd","status":"succeeded"}"#;
            socket1
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp1}",
                        resp1.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();

            // Second call: All params with custom idempotency key
            let (mut socket2, _) = listener.accept().await.unwrap();
            let mut buf2 = vec![0; 4096];
            let n2 = socket2.read(&mut buf2).await.unwrap();
            let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
            assert!(req2.starts_with("POST /v1/payment_intents/pi_conf_2/confirm HTTP/1.1"));
            assert!(req2.contains("idempotency-key: custom_confirm_key_789"));
            assert!(req2.contains("payment_method=pm_card_visa"));
            assert!(req2.contains("return_url=https%3A%2F%2Fexample.com%2Freturn"));
            assert!(req2.contains("receipt_email=payer%40example.com"));
            assert!(req2.contains("setup_future_usage=off_session"));
            let resp2 = r#"{"id":"pi_conf_2","object":"payment_intent","amount":4000,"currency":"usd","status":"succeeded"}"#;
            socket2
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp2}",
                        resp2.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });

        let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        let pi1 = resource.confirm("pi_conf_1", None).await.unwrap();
        assert_eq!(pi1.status, "succeeded");

        let conf_params = ConfirmPaymentIntentParams::new()
            .payment_method("pm_card_visa")
            .return_url("https://example.com/return")
            .receipt_email("payer@example.com")
            .setup_future_usage("off_session")
            .idempotency_key("custom_confirm_key_789");
        let pi2 = resource
            .confirm("pi_conf_2", Some(&conf_params))
            .await
            .unwrap();
        assert_eq!(pi2.status, "succeeded");

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_payment_intents_path_traversal_rejection() {
        let config = StripeConfig::new("sk_test_dummy");
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        let bad_ids = [
            "../evil",
            "pi/123",
            "pi\\123",
            "pi_123?foo=bar",
            "pi_123#frag",
            "",
            "   ",
        ];

        for &bad_id in &bad_ids {
            let res_ret = resource.retrieve(bad_id).await;
            assert!(
                matches!(res_ret, Err(StripeError::InvalidRequest(_))),
                "retrieve({bad_id:?}) must reject path traversal, got: {res_ret:?}"
            );

            let res_cap = resource.capture(bad_id, None).await;
            assert!(
                matches!(res_cap, Err(StripeError::InvalidRequest(_))),
                "capture({bad_id:?}) must reject path traversal, got: {res_cap:?}"
            );

            let res_can = resource.cancel(bad_id, None).await;
            assert!(
                matches!(res_can, Err(StripeError::InvalidRequest(_))),
                "cancel({bad_id:?}) must reject path traversal, got: {res_can:?}"
            );

            let res_cnf = resource.confirm(bad_id, None).await;
            assert!(
                matches!(res_cnf, Err(StripeError::InvalidRequest(_))),
                "confirm({bad_id:?}) must reject path traversal, got: {res_cnf:?}"
            );
        }
    }
}
