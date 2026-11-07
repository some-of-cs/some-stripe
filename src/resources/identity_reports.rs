//! Stripe Identity Verification Reports resource implementation.
//!
//! Provides endpoints to retrieve individual verification reports and list or stream
//! historical verification reports with zero-clone cursor pagination.
//!
//! # Architecture & Quality Invariants:
//! - **Gate G-Borrow**: Zero deep clones; stream pagination yields reports by value move.
//! - **Gate G-Docs**: Runnable doc-tests and complete rustdoc documentation.
//! - **Gate G-Complexity**: Cyclomatic complexity <= 6 on all functions.
//! - **Gate G2 & G-Law**: Path traversal protection and zero PII leakage.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//! use some_stripe_types::ListVerificationReportsParams;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//! let reports_resource = client.identity().verification_reports();
//!
//! // Retrieve a single verification report
//! let report = reports_resource.retrieve("vr_test_123").await?;
//! println!("Report ID: {}, Status: {:?}", report.id, report.status);
//!
//! // List reports with pagination
//! let params = ListVerificationReportsParams::new().limit(20);
//! let mut paginator = reports_resource.list(&params);
//! while let Some(item) = paginator.next_item().await? {
//!     println!("Streamed report: {}", item.id);
//! }
//! # Ok(())
//! # }
//! ```

use some_stripe_core::patterns::{Paginator, encode_form_pairs, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    ListVerificationReportsParams, VerificationReport, verification_report_id,
};

/// Resource for managing Stripe Identity Verification Reports (`stripe.identity().verification_reports()`).
#[derive(Clone, Copy)]
pub struct VerificationReportsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> VerificationReportsResource<'a> {
    /// Creates a new `VerificationReportsResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Retrieves an existing identity verification report (`GET /v1/identity/verification_reports/{id}`).
    ///
    /// Validates `report_id` to prevent path traversal and injection attacks before transmitting.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `report_id` is empty, contains whitespace,
    ///   or includes invalid path characters.
    /// - Returns [`StripeError::Api`] if the report does not exist or the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response body cannot be deserialized into a [`VerificationReport`].
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let report = client.identity().verification_reports().retrieve("vr_123").await?;
    /// assert_eq!(report.id, "vr_123");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, report_id: &str) -> Result<VerificationReport, StripeError> {
        let encoded = validate_and_encode_id(report_id)?;
        let url = format!(
            "{}/v1/identity/verification_reports/{}",
            self.client.config.api_base, encoded
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let report: VerificationReport = serde_json::from_value(val)?;
        Ok(report)
    }

    /// Lists identity verification reports with cursor-based pagination (`GET /v1/identity/verification_reports`).
    ///
    /// Returns a zero-clone [`Paginator`] stream yielding [`VerificationReport`] records.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::ListVerificationReportsParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListVerificationReportsParams::new().limit(10);
    /// let mut stream = client.identity().verification_reports().list(&params);
    /// while let Some(report) = stream.next_item().await? {
    ///     println!("Report ID: {}", report.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list(
        &self,
        params: &ListVerificationReportsParams,
    ) -> Paginator<'a, VerificationReport> {
        let (base_path, initial_err) = match to_form_params(params) {
            Ok(mut pairs) => {
                pairs
                    .retain(|(k, _)| k != "limit" && k != "starting_after" && k != "ending_before");
                let path = if pairs.is_empty() {
                    "v1/identity/verification_reports".to_string()
                } else {
                    format!(
                        "v1/identity/verification_reports?{}",
                        encode_form_pairs(&pairs)
                    )
                };
                (path, None)
            }
            Err(e) => ("v1/identity/verification_reports".to_string(), Some(e)),
        };

        let mut paginator = Paginator::new(
            self.client,
            base_path,
            params.limit.unwrap_or(10),
            verification_report_id,
        );

        if let Some(err) = initial_err {
            paginator = paginator.with_initial_error(err);
        }
        if let Some(ref sa) = params.starting_after {
            paginator = paginator.starting_after(sa);
        }
        if let Some(ref eb) = params.ending_before {
            paginator = paginator.ending_before(eb);
        }

        paginator
    }

    /// Lists identity verification reports with default parameters.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut stream = client.identity().verification_reports().list_default();
    /// if let Some(first) = stream.next_item().await? {
    ///     println!("First report: {}", first.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_default(&self) -> Paginator<'a, VerificationReport> {
        self.list(&ListVerificationReportsParams::default())
    }
}
