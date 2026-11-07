//! Stripe Connect KYC Persons resource implementations (`/v1/accounts/{account}/persons`).
//!
//! Provides bindings for managing Persons associated with a connected account
//! for FinCEN CDD, EU DAC7, and AMLD5 compliance.

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::person::{
    CreatePersonParams, DeletedPerson, ListPersonsParams, Person, PersonList, UpdatePersonParams,
    person_id,
};

/// Resource for managing Persons associated with a Stripe Connect Account (`stripe.accounts.persons`).
///
/// Wraps endpoints for creating, retrieving, updating, deleting, and listing individuals
/// associated with a connected account (such as owners, directors, executives, and representatives).
#[derive(Clone, Copy)]
pub struct PersonsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> PersonsResource<'a> {
    /// Creates a new `PersonsResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account.
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Creates a new Person on an account (`POST /v1/accounts/{account_id}/persons`).
    ///
    /// Validates `account_id` via [`validate_and_encode_id`] to prevent path traversal.
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `account_id` contains invalid characters or validation fails.
    /// - Returns [`StripeError::Api`] if the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`Person`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::person::CreatePersonParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreatePersonParams::new()
    ///     .first_name("Jane")
    ///     .last_name("Doe")
    ///     .email("jane.doe@example.com");
    ///
    /// let person = client.persons().create("acct_1032D82eZvKYlo2C", &params).await?;
    /// println!("Created person: {}", person.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        account_id: &str,
        params: &CreatePersonParams,
    ) -> Result<Person, StripeError> {
        let encoded_account = validate_and_encode_id(account_id)?;
        let url = format!(
            "{}/v1/accounts/{encoded_account}/persons",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let person: Person = serde_json::from_value(val)?;
        Ok(person)
    }

    /// Retrieves an existing Person (`GET /v1/accounts/{account_id}/persons/{person_id}`).
    ///
    /// Validates both `account_id` and `person_id` via [`validate_and_encode_id`].
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if any identifier contains forbidden characters.
    /// - Returns [`StripeError::Api`] if the Person does not exist or API fails.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let person = client.persons().retrieve("acct_1032D82eZvKYlo2C", "person_1032D82eZvKYlo2C").await?;
    /// println!("Retrieved person: {}", person.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, account_id: &str, person_id: &str) -> Result<Person, StripeError> {
        let encoded_account = validate_and_encode_id(account_id)?;
        let encoded_person = validate_and_encode_id(person_id)?;
        let url = format!(
            "{}/v1/accounts/{encoded_account}/persons/{encoded_person}",
            self.client.config.api_base
        );
        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let person: Person = serde_json::from_value(val)?;
        Ok(person)
    }

    /// Updates an existing Person (`POST /v1/accounts/{account_id}/persons/{person_id}`).
    ///
    /// Validates identifiers and injects an `Idempotency-Key` header if omitted.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if any identifier is invalid.
    /// - Returns [`StripeError::Api`] if the update fails.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::person::UpdatePersonParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = UpdatePersonParams::new()
    ///     .first_name("Janet");
    ///
    /// let person = client.persons().update("acct_1032D82eZvKYlo2C", "person_1032D82eZvKYlo2C", &params).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        account_id: &str,
        person_id: &str,
        params: &UpdatePersonParams,
    ) -> Result<Person, StripeError> {
        let encoded_account = validate_and_encode_id(account_id)?;
        let encoded_person = validate_and_encode_id(person_id)?;
        let url = format!(
            "{}/v1/accounts/{encoded_account}/persons/{encoded_person}",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let person: Person = serde_json::from_value(val)?;
        Ok(person)
    }

    /// Deletes a Person from an account (`DELETE /v1/accounts/{account_id}/persons/{person_id}`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if any identifier is invalid.
    /// - Returns [`StripeError::Api`] if deletion fails.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let deleted = client.persons().del("acct_1032D82eZvKYlo2C", "person_1032D82eZvKYlo2C").await?;
    /// assert!(deleted.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn del(
        &self,
        account_id: &str,
        person_id: &str,
    ) -> Result<DeletedPerson, StripeError> {
        let encoded_account = validate_and_encode_id(account_id)?;
        let encoded_person = validate_and_encode_id(person_id)?;
        let url = format!(
            "{}/v1/accounts/{encoded_account}/persons/{encoded_person}",
            self.client.config.api_base
        );
        let mut req = self.client.http.delete(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let deleted: DeletedPerson = serde_json::from_value(val)?;
        Ok(deleted)
    }

    /// Alias for [`del`](Self::del) to delete a Person.
    pub async fn delete(
        &self,
        account_id: &str,
        person_id: &str,
    ) -> Result<DeletedPerson, StripeError> {
        self.del(account_id, person_id).await
    }

    /// Lists Persons associated with an account (`GET /v1/accounts/{account_id}/persons`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `account_id` is invalid.
    /// - Returns [`StripeError::Api`] if the list request fails.
    /// - Returns [`StripeError::Json`] if deserialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::person::ListPersonsParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListPersonsParams::new().limit(10);
    /// let list = client.persons().list("acct_1032D82eZvKYlo2C", &params).await?;
    /// println!("Persons count: {}", list.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        account_id: &str,
        params: &ListPersonsParams,
    ) -> Result<PersonList, StripeError> {
        let encoded_account = validate_and_encode_id(account_id)?;
        let query_data = to_form_params(params)?;
        let url = if query_data.is_empty() {
            format!(
                "{}/v1/accounts/{encoded_account}/persons",
                self.client.config.api_base
            )
        } else {
            let query_str = some_stripe_core::patterns::serializer::encode_form_pairs(&query_data);
            format!(
                "{}/v1/accounts/{encoded_account}/persons?{query_str}",
                self.client.config.api_base
            )
        };
        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let list: PersonList = serde_json::from_value(val)?;
        Ok(list)
    }

    /// Returns a zero-clone paginator streaming persons under an account (`GET /v1/accounts/{account_id}/persons`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut paginator = client.persons().paginate("acct_1032D82eZvKYlo2C", Some(10))?;
    /// while let Some(person) = paginator.next_item().await? {
    ///     println!("Person: {}", person.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn paginate(
        &self,
        account_id: &str,
        limit: Option<u32>,
    ) -> Result<Paginator<'a, Person>, StripeError> {
        let encoded_account = validate_and_encode_id(account_id)?;
        let path = format!("v1/accounts/{encoded_account}/persons");
        let mut paginator = Paginator::new(self.client, path, limit.unwrap_or(10), person_id);
        if let Some(account) = self.account {
            paginator = paginator.with_account(account);
        }
        Ok(paginator)
    }
}
