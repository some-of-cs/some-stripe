//! Stripe Transfers resource implementations.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateTransferParams, CreateTransferReversalParams, ListTransferReversalsParams,
    ListTransfersParams, Transfer, TransferList, TransferReversal, TransferReversalList,
    UpdateTransferParams,
};

/// Resource for managing Stripe Transfers (`stripe.transfers`).
#[derive(Clone, Copy)]
pub struct TransfersResource<'a> {
    client: &'a StripeClient,
}

impl<'a> TransfersResource<'a> {
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    pub async fn create(&self, params: &CreateTransferParams) -> Result<Transfer, StripeError> {
        let url = format!("{}/v1/transfers", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let transfer: Transfer = serde_json::from_value(val)?;
        Ok(transfer)
    }

    pub async fn retrieve(&self, id: &str) -> Result<Transfer, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!("{}/v1/transfers/{encoded}", self.client.config.api_base);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let transfer: Transfer = serde_json::from_value(val)?;
        Ok(transfer)
    }

    pub async fn update(
        &self,
        id: &str,
        params: &UpdateTransferParams,
    ) -> Result<Transfer, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!("{}/v1/transfers/{encoded}", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let transfer: Transfer = serde_json::from_value(val)?;
        Ok(transfer)
    }

    pub async fn reverse(
        &self,
        id: &str,
        params: Option<&CreateTransferReversalParams>,
    ) -> Result<TransferReversal, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/transfers/{encoded}/reversals",
            self.client.config.api_base
        );
        let form_data = match params {
            Some(p) => to_form_params(p)?,
            None => to_form_params(&CreateTransferReversalParams::new())?,
        };

        let idempotency_key =
            super::resolve_idempotency_key(params.and_then(|p| p.idempotency_key.as_deref()));

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let reversal: TransferReversal = serde_json::from_value(val)?;
        Ok(reversal)
    }

    pub async fn list(
        &self,
        params: Option<&ListTransfersParams>,
    ) -> Result<TransferList, StripeError> {
        let mut url = format!("{}/v1/transfers", self.client.config.api_base);
        let mut query_params = Vec::new();
        let limit_str = params.and_then(|p| p.limit).map(|l| l.to_string());

        if let Some(p) = params {
            if let Some(ref tg) = p.transfer_group {
                query_params.push(("transfer_group", tg.as_str()));
            }
            if let Some(ref dest) = p.destination {
                query_params.push(("destination", dest.as_str()));
            }
            if let Some(ref l) = limit_str {
                query_params.push(("limit", l.as_str()));
            }
            if let Some(ref a) = p.starting_after {
                query_params.push(("starting_after", a.as_str()));
            }
            if let Some(ref b) = p.ending_before {
                query_params.push(("ending_before", b.as_str()));
            }
        }

        if !query_params.is_empty() {
            let encoded: Vec<String> = query_params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            url.push('?');
            url.push_str(&encoded.join("&"));
        }

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: TransferList = serde_json::from_value(val)?;
        Ok(list)
    }

    pub async fn list_reversals(
        &self,
        id: &str,
        params: Option<&ListTransferReversalsParams>,
    ) -> Result<TransferReversalList, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let mut url = format!(
            "{}/v1/transfers/{encoded}/reversals",
            self.client.config.api_base
        );
        let mut query_params = Vec::new();
        let limit_str = params.and_then(|p| p.limit).map(|l| l.to_string());

        if let Some(p) = params {
            if let Some(ref l) = limit_str {
                query_params.push(("limit", l.as_str()));
            }
            if let Some(ref a) = p.starting_after {
                query_params.push(("starting_after", a.as_str()));
            }
            if let Some(ref b) = p.ending_before {
                query_params.push(("ending_before", b.as_str()));
            }
        }

        if !query_params.is_empty() {
            let encoded_params: Vec<String> = query_params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            url.push('?');
            url.push_str(&encoded_params.join("&"));
        }

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: TransferReversalList = serde_json::from_value(val)?;
        Ok(list)
    }
}
