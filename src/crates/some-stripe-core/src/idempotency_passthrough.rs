//! Caller `Idempotency-Key` validation and retry-safe passthrough (US-9.1.0 Unit 5).
//!
//! Ensures caller-supplied idempotency keys are validated pre-flight before any
//! network I/O and preserved verbatim across transient HTTP and transport retries.
//!
//! # Examples
//!
//! ```
//! use some_stripe_core::idempotency_passthrough::{
//!     IDEMPOTENCY_KEY_HEADER, IdempotencyKey, MAX_IDEMPOTENCY_KEY_LEN,
//! };
//!
//! let key = IdempotencyKey::try_new("order-910-payment-attempt-1").unwrap();
//! assert_eq!(key.as_str(), "order-910-payment-attempt-1");
//! assert_eq!(IDEMPOTENCY_KEY_HEADER, "Idempotency-Key");
//! assert_eq!(MAX_IDEMPOTENCY_KEY_LEN, 255);
//!
//! // Oversized and control-character keys fail closed pre-flight.
//! assert!(IdempotencyKey::try_new("k".repeat(256)).is_err());
//! let crlf = format!("bad{}{}X-Header: inject", char::from(13), char::from(10));
//! assert!(IdempotencyKey::try_new(crlf).is_err());
//! ```

use std::fmt;
use std::time::Duration;

use crate::error::StripeError;

/// Maximum permitted byte length for a Stripe `Idempotency-Key` header value.
pub const MAX_IDEMPOTENCY_KEY_LEN: usize = 255;

/// Canonical HTTP header name used to transmit Stripe idempotency keys.
pub const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// HTTP status codes eligible for retry under [`RetryKeyPolicy`].
const RETRYABLE_HTTP_STATUSES: &[u16] = &[408, 429, 500, 502, 503, 504];

/// Validated Stripe `Idempotency-Key` header value.
///
/// Guarantees pre-flight that the key is non-empty, not whitespace-only, at most
/// [`MAX_IDEMPOTENCY_KEY_LEN`] (255) bytes, and free of ASCII control characters
/// (`0x00..=0x1F` and `0x7F`, including CR, LF, and NUL) to prevent HTTP header
/// injection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Validates and constructs an [`IdempotencyKey`] from a caller-supplied string.
    ///
    /// Preserves the exact caller bytes without mutation when valid.
    ///
    /// # Errors
    ///
    /// Returns [`StripeError::InvalidRequest`] if `raw`:
    /// - is empty or contains only whitespace,
    /// - exceeds [`MAX_IDEMPOTENCY_KEY_LEN`] (255) bytes, or
    /// - contains any ASCII control character (`0x00..=0x1F` or `0x7F`).
    ///
    /// # Examples
    ///
    /// ```
    /// use some_stripe_core::idempotency_passthrough::IdempotencyKey;
    ///
    /// let key = IdempotencyKey::try_new("checkout_sess_42").unwrap();
    /// assert_eq!(key.to_string(), "checkout_sess_42");
    /// assert!(IdempotencyKey::try_new("   ").is_err());
    /// assert!(IdempotencyKey::try_new(format!("line1{}line2", char::from(10))).is_err());
    /// ```
    pub fn try_new(raw: impl AsRef<str>) -> Result<Self, StripeError> {
        let value = raw.as_ref();
        if value.trim().is_empty() {
            return Err(StripeError::InvalidRequest(
                "Idempotency-Key must not be empty or whitespace-only".to_string(),
            ));
        }
        if value.len() > MAX_IDEMPOTENCY_KEY_LEN {
            return Err(StripeError::InvalidRequest(format!(
                "Idempotency-Key exceeds {MAX_IDEMPOTENCY_KEY_LEN} bytes (got {})",
                value.len()
            )));
        }
        if contains_ascii_control(value) {
            return Err(StripeError::InvalidRequest(
                "Idempotency-Key must not contain ASCII control characters".to_string(),
            ));
        }
        Ok(Self(value.to_string()))
    }

    /// Returns the validated idempotency key as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the newtype and returns the underlying owned [`String`].
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for IdempotencyKey {
    type Error = StripeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = StripeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

/// Returns `true` if `value` contains any ASCII control byte (`0x00..=0x1F` or `0x7F`).
fn contains_ascii_control(value: &str) -> bool {
    value.bytes().any(|b| b.is_ascii_control())
}

/// Returns `true` if the HTTP status code is retryable with a preserved [`IdempotencyKey`].
///
/// Retries are permitted for transient server errors (`500`, `502`, `503`, `504`),
/// rate limits (`429`), and request timeouts (`408`). Definitive `4xx` client
/// rejections (`400..=407`, `409..=428`, `430..=499`) are never retried.
///
/// # Examples
///
/// ```
/// use some_stripe_core::idempotency_passthrough::is_retryable_http_status;
///
/// assert!(is_retryable_http_status(503));
/// assert!(is_retryable_http_status(429));
/// assert!(!is_retryable_http_status(400));
/// assert!(!is_retryable_http_status(422));
/// ```
#[must_use]
pub fn is_retryable_http_status(status: u16) -> bool {
    RETRYABLE_HTTP_STATUSES.contains(&status)
}

/// Error returned by a single HTTP dispatch attempt under [`RetryKeyPolicy`].
#[derive(Debug)]
pub enum RetryableDispatchError {
    /// An HTTP error response returned by Stripe or an intermediary proxy.
    HttpStatus {
        /// HTTP status code (e.g. `503`, `429`, `400`).
        status: u16,
        /// Underlying domain or API error.
        error: StripeError,
    },
    /// A transient network or transport failure (e.g. socket reset or timeout).
    TransientTransport(StripeError),
    /// A terminal, non-retryable failure.
    NonRetryable(StripeError),
}

impl RetryableDispatchError {
    /// Constructs an HTTP status dispatch error.
    #[must_use]
    pub fn http_status(status: u16, error: StripeError) -> Self {
        Self::HttpStatus { status, error }
    }

    /// Constructs a retryable transient transport dispatch error.
    #[must_use]
    pub fn transient_transport(error: StripeError) -> Self {
        Self::TransientTransport(error)
    }

    /// Constructs a terminal non-retryable dispatch error.
    #[must_use]
    pub fn non_retryable(error: StripeError) -> Self {
        Self::NonRetryable(error)
    }

    /// Returns `true` if this error is eligible for retry with the same [`IdempotencyKey`].
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::HttpStatus { status, .. } => is_retryable_http_status(*status),
            Self::TransientTransport(_) => true,
            Self::NonRetryable(_) => false,
        }
    }

    /// Extracts the underlying [`StripeError`].
    #[must_use]
    pub fn into_stripe_error(self) -> StripeError {
        match self {
            Self::HttpStatus { error, .. }
            | Self::TransientTransport(error)
            | Self::NonRetryable(error) => error,
        }
    }
}

/// Retry policy that preserves a caller-supplied [`IdempotencyKey`] verbatim
/// across every retry attempt.
///
/// The policy never generates or mutates the idempotency key on retry; it passes
/// the exact same `&IdempotencyKey` reference to every invocation of the caller's
/// async dispatch closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryKeyPolicy {
    max_retries: usize,
    base_backoff: Duration,
}

impl RetryKeyPolicy {
    /// Creates a new retry policy with `max_retries` and zero initial backoff.
    #[must_use]
    pub const fn new(max_retries: usize) -> Self {
        Self {
            max_retries,
            base_backoff: Duration::ZERO,
        }
    }

    /// Configures the base exponential backoff delay between retries.
    #[must_use]
    pub const fn with_base_backoff(mut self, base_backoff: Duration) -> Self {
        self.base_backoff = base_backoff;
        self
    }

    /// Returns the maximum number of retry attempts after the initial dispatch.
    #[must_use]
    pub const fn max_retries(&self) -> usize {
        self.max_retries
    }

    /// Returns the configured base backoff delay.
    #[must_use]
    pub const fn base_backoff(&self) -> Duration {
        self.base_backoff
    }

    /// Executes `operation` while preserving `key` verbatim across all retries.
    ///
    /// The closure receives `(&IdempotencyKey, usize)` where the second argument
    /// is the zero-based attempt index (`0` for the initial call, `1..=max_retries`
    /// for retries).
    ///
    /// # Errors
    ///
    /// Returns the terminal [`StripeError`] if the operation fails with a
    /// non-retryable error (such as a definitive `4xx` rejection) or exhausts
    /// `max_retries`.
    pub async fn execute<T, Fut, F>(
        &self,
        key: &IdempotencyKey,
        mut operation: F,
    ) -> Result<T, StripeError>
    where
        F: FnMut(&IdempotencyKey, usize) -> Fut,
        Fut: std::future::Future<Output = Result<T, RetryableDispatchError>>,
    {
        let mut attempt = 0usize;
        loop {
            match operation(key, attempt).await {
                Ok(value) => return Ok(value),
                Err(err) if self.should_retry(&err, attempt) => {
                    self.sleep_before_retry(attempt).await;
                    attempt += 1;
                }
                Err(err) => return Err(err.into_stripe_error()),
            }
        }
    }

    /// Returns `true` when another retry attempt is permitted for `err`.
    fn should_retry(&self, err: &RetryableDispatchError, attempt: usize) -> bool {
        attempt < self.max_retries && err.is_retryable()
    }

    /// Sleeps for the exponential backoff delay if a non-zero base delay is set.
    async fn sleep_before_retry(&self, attempt: usize) {
        if self.base_backoff.is_zero() {
            return;
        }
        let shift = u32::try_from(attempt).unwrap_or(u32::MAX).min(10);
        let multiplier = 1u32 << shift;
        tokio::time::sleep(self.base_backoff.saturating_mul(multiplier)).await;
    }
}

impl Default for RetryKeyPolicy {
    fn default() -> Self {
        Self::new(2)
    }
}

/// Executes an async HTTP operation with up to `max_retries` retries, always
/// forwarding the exact same caller [`IdempotencyKey`] on every attempt.
///
/// Retries on HTTP `503`, `500`, `502`, `504`, `429`, `408`, and transient
/// transport failures; does not retry definitive `4xx` client rejections
/// (`400..=407`, `409..=428`, `430..=499`).
///
/// # Errors
///
/// Returns the last [`StripeError`] when a non-retryable error occurs or all
/// retry attempts are exhausted.
///
/// # Examples
///
/// ```
/// use some_stripe_core::StripeError;
/// use some_stripe_core::idempotency_passthrough::{
///     IdempotencyKey, RetryableDispatchError, execute_with_preserved_idempotency_key,
/// };
///
/// let rt = tokio::runtime::Builder::new_current_thread()
///     .enable_all()
///     .build()
///     .unwrap();
///
/// rt.block_on(async {
///     let key = IdempotencyKey::try_new("req-idem-123").unwrap();
///     let mut seen_keys = Vec::new();
///
///     let outcome = execute_with_preserved_idempotency_key(&key, 2, |k, attempt| {
///         seen_keys.push(k.as_str().to_string());
///         async move {
///             if attempt == 0 {
///                 Err(RetryableDispatchError::http_status(
///                     503,
///                     StripeError::ConnectionError("service unavailable".to_string()),
///                 ))
///             } else {
///                 Ok("charged")
///             }
///         }
///     })
///     .await
///     .unwrap();
///
///     assert_eq!(outcome, "charged");
///     assert_eq!(seen_keys, vec!["req-idem-123", "req-idem-123"]);
/// });
/// ```
pub async fn execute_with_preserved_idempotency_key<T, Fut, F>(
    key: &IdempotencyKey,
    max_retries: usize,
    operation: F,
) -> Result<T, StripeError>
where
    F: FnMut(&IdempotencyKey, usize) -> Fut,
    Fut: std::future::Future<Output = Result<T, RetryableDispatchError>>,
{
    RetryKeyPolicy::new(max_retries)
        .execute(key, operation)
        .await
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    use super::*;

    const HEADER_END: [u8; 4] = [13, 10, 13, 10];

    fn mock_api_error(status: u16) -> StripeError {
        StripeError::Api {
            status,
            code: format!("http_{status}"),
            message: format!("mock error {status}"),
            error_type: "api_error".to_string(),
            doc_url: None,
        }
    }

    async fn read_idempotency_header(stream: &mut TcpStream) -> Option<String> {
        let mut buf = Vec::with_capacity(1024);
        let mut chunk = [0u8; 512];
        while !buf.windows(4).any(|w| w == HEADER_END) && buf.len() < 8192 {
            let read = stream.read(&mut chunk).await.expect("read request bytes");
            if read == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..read]);
        }
        extract_header_value(&String::from_utf8_lossy(&buf), IDEMPOTENCY_KEY_HEADER)
    }

    fn extract_header_value(raw_request: &str, header_name: &str) -> Option<String> {
        raw_request.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(header_name)
                .then(|| value.trim().to_string())
        })
    }

    fn scripted_http_response(status: u16) -> Vec<u8> {
        let crlf = format!("{}{}", char::from(13), char::from(10));
        let (head, body) = match status {
            200 => ("200 OK", "ok"),
            _ => ("503 Service Unavailable", "unavailable"),
        };
        format!(
            "HTTP/1.1 {head}{crlf}Content-Length: {}{crlf}Connection: close{crlf}{crlf}{body}",
            body.len()
        )
        .into_bytes()
    }

    #[test]
    fn accepts_valid_keys_up_to_255_bytes_and_implements_traits() {
        let exact_max = "k".repeat(MAX_IDEMPOTENCY_KEY_LEN);
        let key = IdempotencyKey::try_new(&exact_max).expect("255-byte key must be accepted");
        assert_eq!(key.as_str(), exact_max);
        assert_eq!(key.as_ref(), exact_max.as_str());
        assert_eq!(key.to_string(), exact_max);

        let cloned = key.clone();
        assert_eq!(key, cloned);

        let mut set = HashSet::new();
        set.insert(key);
        assert!(set.contains(&cloned));

        let from_str = IdempotencyKey::try_from("idem-abc-123").expect("valid &str");
        let from_string = IdempotencyKey::try_from("idem-abc-123".to_string()).expect("valid");
        assert_eq!(from_str, from_string);
        assert_eq!(from_str.into_inner(), "idem-abc-123");
    }

    #[test]
    fn rejects_empty_and_whitespace_only_keys() {
        let crlf_ws = format!(" {}{} ", char::from(13), char::from(10));
        let tab = char::from(9).to_string();
        for invalid in ["", " ", "   ", tab.as_str(), crlf_ws.as_str()] {
            let err = IdempotencyKey::try_new(invalid).expect_err("must reject empty/whitespace");
            assert!(
                matches!(err, StripeError::InvalidRequest(_)),
                "expected InvalidRequest for {invalid:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn rejects_keys_exceeding_255_bytes() {
        let ascii_over = "a".repeat(MAX_IDEMPOTENCY_KEY_LEN + 1);
        let err = IdempotencyKey::try_new(&ascii_over).expect_err("256-byte key must fail");
        assert!(matches!(err, StripeError::InvalidRequest(_)));

        // 128 two-byte UTF-8 chars = 256 bytes (> MAX_IDEMPOTENCY_KEY_LEN).
        let utf8_over = char::from_u32(0x00E9)
            .expect("valid unicode scalar")
            .to_string()
            .repeat(128);
        assert_eq!(utf8_over.len(), 256);
        let utf8_err = IdempotencyKey::try_new(&utf8_over).expect_err("256-byte UTF-8 must fail");
        assert!(matches!(utf8_err, StripeError::InvalidRequest(_)));
    }

    #[test]
    fn rejects_ascii_control_characters_and_crlf_injection() {
        let payloads = [
            format!("key{}{}X-Injected: evil", char::from(13), char::from(10)),
            format!("key{}value", char::from(13)),
            format!("key{}value", char::from(10)),
            format!("key{}null", char::from(0)),
            format!("key{}escape", char::from(27)),
            format!("key{}del", char::from(127)),
            format!("key{}tab", char::from(9)),
        ];
        for payload in payloads {
            let err = IdempotencyKey::try_new(&payload).expect_err("control char must be rejected");
            assert!(
                matches!(err, StripeError::InvalidRequest(_)),
                "expected InvalidRequest for {payload:?}, got {err:?}"
            );
        }
    }

    #[tokio::test]
    async fn retries_503_then_200_over_tcp_preserving_exact_caller_key() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let addr = listener.local_addr().expect("listener local addr");

        let server = tokio::spawn(async move {
            let mut observed = Vec::new();
            for status in [503u16, 200u16] {
                let (mut stream, _) = listener.accept().await.expect("accept client");
                let header = read_idempotency_header(&mut stream)
                    .await
                    .expect("Idempotency-Key header must be present");
                observed.push(header);
                let response = scripted_http_response(status);
                stream
                    .write_all(&response)
                    .await
                    .expect("write scripted response");
                stream.shutdown().await.expect("shutdown stream");
            }
            observed
        });

        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("build reqwest client");
        let url = format!("http://{addr}/v1/payment_intents");
        let caller_key =
            IdempotencyKey::try_new("caller-us-9-1-0-unit-5-key").expect("valid caller key");

        let body = execute_with_preserved_idempotency_key(&caller_key, 2, |key, _attempt| {
            let client = client.clone();
            let url = url.clone();
            let key_value = key.as_str().to_string();
            async move {
                let resp = client
                    .post(&url)
                    .header(IDEMPOTENCY_KEY_HEADER, key_value)
                    .send()
                    .await
                    .map_err(|err| RetryableDispatchError::transient_transport(err.into()))?;
                let status = resp.status().as_u16();
                if status == 200 {
                    let text = resp
                        .text()
                        .await
                        .map_err(|err| RetryableDispatchError::transient_transport(err.into()))?;
                    return Ok(text);
                }
                Err(RetryableDispatchError::http_status(
                    status,
                    mock_api_error(status),
                ))
            }
        })
        .await
        .expect("503 followed by 200 must succeed");

        assert_eq!(body, "ok");
        let observed_keys = server.await.expect("join server task");
        assert_eq!(
            observed_keys,
            vec!["caller-us-9-1-0-unit-5-key", "caller-us-9-1-0-unit-5-key"]
        );
    }

    #[tokio::test]
    async fn retries_transient_statuses_and_transport_errors_with_identical_key() {
        let key = IdempotencyKey::try_new("idem-transient-matrix").expect("valid key");
        for status in [408u16, 429, 500, 502, 503, 504] {
            let mut seen = Vec::new();
            let result = execute_with_preserved_idempotency_key(&key, 1, |k, attempt| {
                seen.push(k.as_str().to_string());
                async move {
                    if attempt == 0 {
                        Err(RetryableDispatchError::http_status(
                            status,
                            mock_api_error(status),
                        ))
                    } else {
                        Ok(status)
                    }
                }
            })
            .await
            .expect("retryable status must succeed on retry");
            assert_eq!(result, status);
            assert_eq!(seen, vec!["idem-transient-matrix", "idem-transient-matrix"]);
        }

        let mut transport_seen = Vec::new();
        let policy = RetryKeyPolicy::new(2).with_base_backoff(Duration::from_millis(1));
        assert_eq!(policy.max_retries(), 2);
        assert_eq!(policy.base_backoff(), Duration::from_millis(1));

        let err = policy
            .execute(&key, |k, _| {
                transport_seen.push(k.as_str().to_string());
                async {
                    Err::<(), _>(RetryableDispatchError::transient_transport(
                        StripeError::ConnectionError("reset by peer".to_string()),
                    ))
                }
            })
            .await
            .expect_err("exhausted retries must return error");
        assert!(matches!(err, StripeError::ConnectionError(_)));
        assert_eq!(transport_seen.len(), 3);
        assert!(transport_seen.iter().all(|k| k == "idem-transient-matrix"));
    }

    #[tokio::test]
    async fn does_not_retry_definitive_4xx_client_rejections() {
        let key = IdempotencyKey::try_new("idem-no-retry-4xx").expect("valid key");
        let ranges = [400u16..=407, 409..=428, 430..=499];
        for range in ranges {
            for status in range {
                assert!(!is_retryable_http_status(status));
                let mut attempts = 0usize;
                let err = execute_with_preserved_idempotency_key(&key, 3, |_, _| {
                    attempts += 1;
                    async move {
                        Err::<(), _>(RetryableDispatchError::http_status(
                            status,
                            mock_api_error(status),
                        ))
                    }
                })
                .await
                .expect_err("definitive 4xx must fail immediately");
                assert_eq!(attempts, 1, "HTTP {status} must not be retried");
                assert!(matches!(err, StripeError::Api { status: s, .. } if s == status));
            }
        }
    }

    #[tokio::test]
    async fn does_not_retry_non_retryable_dispatch_error() {
        let key = IdempotencyKey::try_new("idem-terminal").expect("valid key");
        let mut attempts = 0usize;
        let err = RetryKeyPolicy::default()
            .execute(&key, |_, _| {
                attempts += 1;
                async {
                    Err::<(), _>(RetryableDispatchError::non_retryable(
                        StripeError::Config("invalid endpoint".to_string()),
                    ))
                }
            })
            .await
            .expect_err("non-retryable error must fail immediately");
        assert_eq!(attempts, 1);
        assert!(matches!(err, StripeError::Config(_)));
    }
}
