//! Stripe Treasury Outbound Transfers, Outbound Payments, and Received Credits resources.
//!
//! Provides API client implementations for commercial banking money movement under Stripe Treasury:
//! - [`TreasuryTransfersResource`]: Unified resource namespace.
//! - [`OutboundTransfersResource`]: Outbound transfers between FinancialAccounts and external bank accounts.
//! - [`OutboundPaymentsResource`]: Outbound payments to third-party counterparties.
//! - [`ReceivedCreditsResource`]: Inbound credit receipts.

use some_stripe_core::patterns::{encode_form_pairs, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CancelOutboundPaymentParams, CancelOutboundTransferParams, CreateOutboundPaymentParams,
    CreateOutboundTransferParams, ListOutboundPaymentsParams, ListOutboundTransfersParams,
    ListReceivedCreditsParams, OutboundPayment, OutboundPaymentList, OutboundTransfer,
    OutboundTransferList, ReceivedCredit, ReceivedCreditList,
};

/// Resource for managing Stripe Treasury money movement operations.
#[derive(Clone, Copy)]
pub struct TreasuryTransfersResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> TreasuryTransfersResource<'a> {
    /// Creates a new `TreasuryTransfersResource`.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Returns a reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account
    /// by attaching the `Stripe-Account` tenant header to all outbound requests.
    #[must_use]
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Access the dedicated Outbound Transfers sub-resource.
    #[must_use]
    pub fn outbound_transfers(&self) -> OutboundTransfersResource<'a> {
        OutboundTransfersResource {
            client: self.client,
            account: self.account,
        }
    }

    /// Access the dedicated Outbound Payments sub-resource.
    #[must_use]
    pub fn outbound_payments(&self) -> OutboundPaymentsResource<'a> {
        OutboundPaymentsResource {
            client: self.client,
            account: self.account,
        }
    }

    /// Access the dedicated Received Credits sub-resource.
    #[must_use]
    pub fn received_credits(&self) -> ReceivedCreditsResource<'a> {
        ReceivedCreditsResource {
            client: self.client,
            account: self.account,
        }
    }

    // --- Outbound Transfer Direct Accessors ---

    /// Creates an OutboundTransfer from a Treasury FinancialAccount.
    pub async fn create_outbound_transfer(
        &self,
        params: &CreateOutboundTransferParams,
    ) -> Result<OutboundTransfer, StripeError> {
        self.outbound_transfers().create(params).await
    }

    /// Retrieves an OutboundTransfer by ID.
    pub async fn retrieve_outbound_transfer(
        &self,
        id: &str,
    ) -> Result<OutboundTransfer, StripeError> {
        self.outbound_transfers().retrieve(id).await
    }

    /// Cancels an OutboundTransfer by ID.
    pub async fn cancel_outbound_transfer(
        &self,
        id: &str,
        params: Option<&CancelOutboundTransferParams>,
    ) -> Result<OutboundTransfer, StripeError> {
        self.outbound_transfers().cancel(id, params).await
    }

    /// Lists OutboundTransfers for a FinancialAccount.
    pub async fn list_outbound_transfers(
        &self,
        params: &ListOutboundTransfersParams,
    ) -> Result<OutboundTransferList, StripeError> {
        self.outbound_transfers().list(params).await
    }

    // --- Outbound Payment Direct Accessors ---

    /// Creates an OutboundPayment from a Treasury FinancialAccount.
    pub async fn create_outbound_payment(
        &self,
        params: &CreateOutboundPaymentParams,
    ) -> Result<OutboundPayment, StripeError> {
        self.outbound_payments().create(params).await
    }

    /// Retrieves an OutboundPayment by ID.
    pub async fn retrieve_outbound_payment(
        &self,
        id: &str,
    ) -> Result<OutboundPayment, StripeError> {
        self.outbound_payments().retrieve(id).await
    }

    /// Cancels an OutboundPayment by ID.
    pub async fn cancel_outbound_payment(
        &self,
        id: &str,
        params: Option<&CancelOutboundPaymentParams>,
    ) -> Result<OutboundPayment, StripeError> {
        self.outbound_payments().cancel(id, params).await
    }

    /// Lists OutboundPayments for a FinancialAccount.
    pub async fn list_outbound_payments(
        &self,
        params: &ListOutboundPaymentsParams,
    ) -> Result<OutboundPaymentList, StripeError> {
        self.outbound_payments().list(params).await
    }

    // --- Received Credit Direct Accessors ---

    /// Retrieves a ReceivedCredit by ID.
    pub async fn retrieve_received_credit(&self, id: &str) -> Result<ReceivedCredit, StripeError> {
        self.received_credits().retrieve(id).await
    }

    /// Lists ReceivedCredits for a FinancialAccount.
    pub async fn list_received_credits(
        &self,
        params: &ListReceivedCreditsParams,
    ) -> Result<ReceivedCreditList, StripeError> {
        self.received_credits().list(params).await
    }
}

/// Resource for managing Treasury Outbound Transfers (`/v1/treasury/outbound_transfers`).
#[derive(Clone, Copy)]
pub struct OutboundTransfersResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> OutboundTransfersResource<'a> {
    /// Scopes this resource to operate on behalf of a specific connected account.
    #[must_use]
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Creates a new OutboundTransfer.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if parameters fail validation or zero PAN invariants.
    pub async fn create(
        &self,
        params: &CreateOutboundTransferParams,
    ) -> Result<OutboundTransfer, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/treasury/outbound_transfers",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let transfer: OutboundTransfer = serde_json::from_value(val)?;
        Ok(transfer)
    }

    /// Retrieves an existing OutboundTransfer.
    pub async fn retrieve(&self, id: &str) -> Result<OutboundTransfer, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/treasury/outbound_transfers/{encoded}",
            self.client.config.api_base
        );

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let transfer: OutboundTransfer = serde_json::from_value(val)?;
        Ok(transfer)
    }

    /// Cancels an existing OutboundTransfer.
    pub async fn cancel(
        &self,
        id: &str,
        params: Option<&CancelOutboundTransferParams>,
    ) -> Result<OutboundTransfer, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/treasury/outbound_transfers/{encoded}/cancel",
            self.client.config.api_base
        );

        let form_data: Vec<(&str, &str)> = Vec::new();
        let idempotency_key =
            super::resolve_idempotency_key(params.and_then(|p| p.idempotency_key.as_deref()));

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let transfer: OutboundTransfer = serde_json::from_value(val)?;
        Ok(transfer)
    }

    /// Lists OutboundTransfers for a specified FinancialAccount.
    pub async fn list(
        &self,
        params: &ListOutboundTransfersParams,
    ) -> Result<OutboundTransferList, StripeError> {
        let query_data = to_form_params(params)?;
        let url = if query_data.is_empty() {
            format!(
                "{}/v1/treasury/outbound_transfers",
                self.client.config.api_base
            )
        } else {
            let query_str = encode_form_pairs(&query_data);
            format!(
                "{}/v1/treasury/outbound_transfers?{query_str}",
                self.client.config.api_base
            )
        };

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let list: OutboundTransferList = serde_json::from_value(val)?;
        Ok(list)
    }
}

/// Resource for managing Treasury Outbound Payments (`/v1/treasury/outbound_payments`).
#[derive(Clone, Copy)]
pub struct OutboundPaymentsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> OutboundPaymentsResource<'a> {
    /// Scopes this resource to operate on behalf of a specific connected account.
    #[must_use]
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Creates a new OutboundPayment.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if parameters fail validation or zero PAN invariants.
    pub async fn create(
        &self,
        params: &CreateOutboundPaymentParams,
    ) -> Result<OutboundPayment, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/treasury/outbound_payments",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let payment: OutboundPayment = serde_json::from_value(val)?;
        Ok(payment)
    }

    /// Retrieves an existing OutboundPayment.
    pub async fn retrieve(&self, id: &str) -> Result<OutboundPayment, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/treasury/outbound_payments/{encoded}",
            self.client.config.api_base
        );

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let payment: OutboundPayment = serde_json::from_value(val)?;
        Ok(payment)
    }

    /// Cancels an existing OutboundPayment.
    pub async fn cancel(
        &self,
        id: &str,
        params: Option<&CancelOutboundPaymentParams>,
    ) -> Result<OutboundPayment, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/treasury/outbound_payments/{encoded}/cancel",
            self.client.config.api_base
        );

        let form_data: Vec<(&str, &str)> = Vec::new();
        let idempotency_key =
            super::resolve_idempotency_key(params.and_then(|p| p.idempotency_key.as_deref()));

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let payment: OutboundPayment = serde_json::from_value(val)?;
        Ok(payment)
    }

    /// Lists OutboundPayments for a specified FinancialAccount.
    pub async fn list(
        &self,
        params: &ListOutboundPaymentsParams,
    ) -> Result<OutboundPaymentList, StripeError> {
        let query_data = to_form_params(params)?;
        let url = if query_data.is_empty() {
            format!(
                "{}/v1/treasury/outbound_payments",
                self.client.config.api_base
            )
        } else {
            let query_str = encode_form_pairs(&query_data);
            format!(
                "{}/v1/treasury/outbound_payments?{query_str}",
                self.client.config.api_base
            )
        };

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let list: OutboundPaymentList = serde_json::from_value(val)?;
        Ok(list)
    }
}

/// Resource for managing Treasury Received Credits (`/v1/treasury/received_credits`).
#[derive(Clone, Copy)]
pub struct ReceivedCreditsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> ReceivedCreditsResource<'a> {
    /// Scopes this resource to operate on behalf of a specific connected account.
    #[must_use]
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Retrieves an existing ReceivedCredit.
    pub async fn retrieve(&self, id: &str) -> Result<ReceivedCredit, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/treasury/received_credits/{encoded}",
            self.client.config.api_base
        );

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let credit: ReceivedCredit = serde_json::from_value(val)?;
        Ok(credit)
    }

    /// Lists ReceivedCredits for a specified FinancialAccount.
    pub async fn list(
        &self,
        params: &ListReceivedCreditsParams,
    ) -> Result<ReceivedCreditList, StripeError> {
        let query_data = to_form_params(params)?;
        let url = if query_data.is_empty() {
            format!(
                "{}/v1/treasury/received_credits",
                self.client.config.api_base
            )
        } else {
            let query_str = encode_form_pairs(&query_data);
            format!(
                "{}/v1/treasury/received_credits?{query_str}",
                self.client.config.api_base
            )
        };

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let list: ReceivedCreditList = serde_json::from_value(val)?;
        Ok(list)
    }
}
