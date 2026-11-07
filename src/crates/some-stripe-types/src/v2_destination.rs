//! Stripe API v2 Event Destination domain entities and request DTOs.
//!
//! Models Stripe API v2 Event Destinations used to route state-change notifications
//! to external transports including Amazon EventBridge, Azure Event Grid, and standard
//! Webhook endpoints.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero-copy borrowing across parameter models and string slices.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 6$ across all validator and builder methods.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and runnable doc-tests (`cargo test --doc`).
//! - **Gate G2**: Secure input sanitization and AWS/Azure ARN format compliance.
//!
//! # Example
//! ```
//! use some_stripe_types::v2_destination::{
//!     CreateEventDestinationParams, EventDestination, EventDestinationStatus,
//!     EventDestinationType,
//! };
//!
//! let params = CreateEventDestinationParams::amazon_eventbridge(
//!     "123456789012",
//!     "us-east-1",
//!     vec!["v2.billing.meter.error_report_triggered".to_string()],
//! );
//! assert!(params.validate().is_ok());
//! ```

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// Helper to deserialize timestamps represented as either integer unix epoch seconds
/// or RFC 3339 formatted date-time strings.
pub(crate) fn deserialize_flexible_timestamp<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TimestampHelper {
        Int(i64),
        Str(String),
    }

    match TimestampHelper::deserialize(deserializer)? {
        TimestampHelper::Int(i) => Ok(i),
        TimestampHelper::Str(s) => chrono::DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.timestamp())
            .map_err(serde::de::Error::custom),
    }
}

/// Helper to deserialize optional timestamps represented as either integer unix epoch seconds,
/// RFC 3339 formatted date-time strings, or null.
pub(crate) fn deserialize_flexible_timestamp_opt<'de, D>(
    deserializer: D,
) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TimestampOptHelper {
        Int(i64),
        Str(String),
        None,
    }

    match Option::<TimestampOptHelper>::deserialize(deserializer)? {
        Some(TimestampOptHelper::Int(i)) => Ok(Some(i)),
        Some(TimestampOptHelper::Str(s)) => chrono::DateTime::parse_from_rfc3339(&s)
            .map(|dt| Some(dt.timestamp()))
            .map_err(serde::de::Error::custom),
        Some(TimestampOptHelper::None) | None => Ok(None),
    }
}

/// Validation errors when constructing or modifying event destinations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EventDestinationValidationError {
    /// The event types list cannot be empty.
    #[error("Events list cannot be empty; at least one event type pattern is required")]
    EmptyEvents,

    /// Missing required configuration details for destination type.
    #[error("Missing configuration block for destination type: {0}")]
    MissingConfiguration(&'static str),

    /// The AWS Account ID is invalid (must be exactly 12 numeric digits).
    #[error("Invalid AWS Account ID: '{0}'. Expected exactly 12 numeric digits")]
    InvalidAwsAccountId(String),

    /// The AWS Region is invalid.
    #[error("Invalid AWS Region: '{0}'. Expected valid region string (e.g., 'us-east-1')")]
    InvalidAwsRegion(String),

    /// The Webhook URL is invalid.
    #[error("Invalid Webhook URL: '{0}'. Expected valid absolute HTTP or HTTPS URL")]
    InvalidWebhookUrl(String),

    /// Azure Event Grid details are incomplete or invalid.
    #[error("Invalid Azure Event Grid details: {0}")]
    InvalidAzureDetails(String),
}

/// Checks if an AWS account ID conforms to the 12-digit standard format.
#[inline]
pub fn is_valid_aws_account_id(id: &str) -> bool {
    id.len() == 12 && id.chars().all(|c| c.is_ascii_digit())
}

/// Checks if an AWS region is non-empty and contains valid region characters.
#[inline]
pub fn is_valid_aws_region(region: &str) -> bool {
    !region.is_empty()
        && region.len() <= 32
        && region
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Checks if a webhook URL is well-formed.
#[inline]
pub fn is_valid_webhook_url(url: &str) -> bool {
    if url.is_empty() || url.len() > 2048 {
        return false;
    }
    (url.starts_with("https://") || url.starts_with("http://"))
        && !url.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// Checks if an Azure configuration field is valid.
#[inline]
pub fn is_valid_azure_detail(field: &str) -> bool {
    let trimmed = field.trim();
    !trimmed.is_empty() && trimmed.len() <= 256
}

/// Operational status of a Stripe API v2 Event Destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventDestinationStatus {
    /// Destination is actively receiving and delivering events.
    Enabled,
    /// Destination is temporarily paused or disabled.
    Disabled,
}

impl EventDestinationStatus {
    /// Returns the wire string representation of the status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

impl std::fmt::Display for EventDestinationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Delivery transport types supported by Stripe API v2 Event Destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventDestinationType {
    /// Delivers events to Amazon EventBridge partner event sources.
    #[serde(rename = "amazon_eventbridge")]
    AmazonEventbridge,
    /// Delivers events to Azure Event Grid partner topics.
    #[serde(rename = "azure_event_grid")]
    AzureEventGrid,
    /// Delivers events to HTTPS webhook endpoints.
    #[serde(rename = "webhook_endpoint")]
    WebhookEndpoint,
    /// Delivers events to Amazon CloudWatch logs and metrics.
    #[serde(rename = "cloudwatch", alias = "aws_cloudwatch")]
    Cloudwatch,
}

impl EventDestinationType {
    /// Returns the wire string representation of the destination type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AmazonEventbridge => "amazon_eventbridge",
            Self::AzureEventGrid => "azure_event_grid",
            Self::WebhookEndpoint => "webhook_endpoint",
            Self::Cloudwatch => "cloudwatch",
        }
    }
}

impl std::fmt::Display for EventDestinationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Amazon EventBridge configuration details for Stripe API v2 Event Destinations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmazonEventBridgeDetails {
    /// The 12-digit AWS account ID that receives events.
    pub aws_account_id: String,
    /// The AWS region (e.g., `"us-east-1"`, `"eu-west-1"`).
    pub aws_region: String,
    /// Partner event source ARN generated by Stripe (populated after creation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partner_event_source_arn: Option<String>,
}

impl AmazonEventBridgeDetails {
    /// Creates a new Amazon EventBridge configuration.
    pub fn new(aws_account_id: impl Into<String>, aws_region: impl Into<String>) -> Self {
        Self {
            aws_account_id: aws_account_id.into(),
            aws_region: aws_region.into(),
            partner_event_source_arn: None,
        }
    }

    /// Sets the partner event source ARN.
    pub fn with_partner_event_source_arn(mut self, arn: impl Into<String>) -> Self {
        self.partner_event_source_arn = Some(arn.into());
        self
    }
}

/// Azure Event Grid configuration details for Stripe API v2 Event Destinations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AzureEventGridDetails {
    /// The Microsoft Azure Tenant ID.
    pub azure_tenant_id: String,
    /// The Azure Subscription ID.
    pub azure_subscription_id: String,
    /// The Azure Resource Group containing the target topic.
    pub azure_resource_group: String,
    /// The name of the Azure Event Grid partner topic.
    pub azure_event_grid_topic_name: String,
}

impl AzureEventGridDetails {
    /// Creates a new Azure Event Grid configuration.
    pub fn new(
        azure_tenant_id: impl Into<String>,
        azure_subscription_id: impl Into<String>,
        azure_resource_group: impl Into<String>,
        azure_event_grid_topic_name: impl Into<String>,
    ) -> Self {
        Self {
            azure_tenant_id: azure_tenant_id.into(),
            azure_subscription_id: azure_subscription_id.into(),
            azure_resource_group: azure_resource_group.into(),
            azure_event_grid_topic_name: azure_event_grid_topic_name.into(),
        }
    }
}

/// Webhook endpoint configuration details for Stripe API v2 Event Destinations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookEndpointDetails {
    /// The HTTPS URL that receives webhook event payloads.
    pub url: String,
}

impl WebhookEndpointDetails {
    /// Creates a new webhook endpoint configuration.
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

/// Amazon CloudWatch configuration details for Stripe API v2 Event Destinations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudWatchDetails {
    /// The 12-digit AWS account ID that receives CloudWatch metrics and logs.
    pub aws_account_id: String,
    /// The AWS region (e.g., `"us-east-1"`, `"eu-west-1"`).
    pub aws_region: String,
    /// Optional CloudWatch log group or stream name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_group_name: Option<String>,
}

impl CloudWatchDetails {
    /// Creates a new Amazon CloudWatch configuration.
    pub fn new(aws_account_id: impl Into<String>, aws_region: impl Into<String>) -> Self {
        Self {
            aws_account_id: aws_account_id.into(),
            aws_region: aws_region.into(),
            log_group_name: None,
        }
    }

    /// Sets the CloudWatch log group name.
    pub fn with_log_group_name(mut self, log_group_name: impl Into<String>) -> Self {
        self.log_group_name = Some(log_group_name.into());
        self
    }
}

/// Stripe API v2 Event Destination entity (`v2.core.event_destination`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDestination {
    /// Unique identifier for the event destination (e.g. `"edest_123"`).
    pub id: String,
    /// Object descriptor, always `"v2.core.event_destination"`.
    pub object: String,
    /// Current operational status (`enabled` or `disabled`).
    pub status: EventDestinationStatus,
    /// Delivery transport type.
    #[serde(rename = "type")]
    pub destination_type: EventDestinationType,
    /// List of event type patterns subscribed to by this destination.
    #[serde(alias = "enabled_events")]
    pub events: Vec<String>,
    /// Amazon EventBridge configuration if `destination_type` is `amazon_eventbridge`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amazon_eventbridge: Option<AmazonEventBridgeDetails>,
    /// Azure Event Grid configuration if `destination_type` is `azure_event_grid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azure_event_grid: Option<AzureEventGridDetails>,
    /// Webhook endpoint configuration if `destination_type` is `webhook_endpoint`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_endpoint: Option<WebhookEndpointDetails>,
    /// Amazon CloudWatch configuration if `destination_type` is `cloudwatch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloudwatch: Option<CloudWatchDetails>,
    /// Creation timestamp (Unix epoch seconds).
    #[serde(deserialize_with = "deserialize_flexible_timestamp")]
    pub created: i64,
    /// Timestamp when this destination was last updated (Unix epoch seconds).
    #[serde(
        default,
        deserialize_with = "deserialize_flexible_timestamp_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub updated: Option<i64>,
    /// Optional human-readable name for the destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional description for the destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional payload type (`"thin"` or `"snapshot"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_payload: Option<String>,
    /// Whether this destination operates in livemode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livemode: Option<bool>,
}

impl EventDestination {
    /// Creates a new `EventDestination` instance.
    pub fn new(
        id: impl Into<String>,
        status: EventDestinationStatus,
        destination_type: EventDestinationType,
        events: Vec<String>,
        created: i64,
    ) -> Self {
        Self {
            id: id.into(),
            object: "v2.core.event_destination".to_string(),
            status,
            destination_type,
            events,
            amazon_eventbridge: None,
            azure_event_grid: None,
            webhook_endpoint: None,
            cloudwatch: None,
            created,
            updated: None,
            name: None,
            description: None,
            event_payload: None,
            livemode: None,
        }
    }

    /// Sets the Amazon EventBridge details.
    pub fn with_amazon_eventbridge(mut self, details: AmazonEventBridgeDetails) -> Self {
        self.amazon_eventbridge = Some(details);
        self
    }

    /// Sets the Azure Event Grid details.
    pub fn with_azure_event_grid(mut self, details: AzureEventGridDetails) -> Self {
        self.azure_event_grid = Some(details);
        self
    }

    /// Sets the Webhook endpoint details.
    pub fn with_webhook_endpoint(mut self, details: WebhookEndpointDetails) -> Self {
        self.webhook_endpoint = Some(details);
        self
    }

    /// Sets the Amazon CloudWatch details.
    pub fn with_cloudwatch(mut self, details: CloudWatchDetails) -> Self {
        self.cloudwatch = Some(details);
        self
    }

    /// Sets the updated timestamp.
    pub fn with_updated(mut self, updated: i64) -> Self {
        self.updated = Some(updated);
        self
    }

    /// Sets the name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the event payload type.
    pub fn with_event_payload(mut self, payload: impl Into<String>) -> Self {
        self.event_payload = Some(payload.into());
        self
    }

    /// Sets livemode.
    pub fn with_livemode(mut self, livemode: bool) -> Self {
        self.livemode = Some(livemode);
        self
    }

    /// Checks if this destination is currently enabled.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.status == EventDestinationStatus::Enabled
    }

    /// Checks if this destination is currently disabled.
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.status == EventDestinationStatus::Disabled
    }

    /// Checks if this destination delivers to Amazon EventBridge.
    #[inline]
    pub fn is_amazon_eventbridge(&self) -> bool {
        self.destination_type == EventDestinationType::AmazonEventbridge
    }

    /// Checks if this destination delivers to Azure Event Grid.
    #[inline]
    pub fn is_azure_event_grid(&self) -> bool {
        self.destination_type == EventDestinationType::AzureEventGrid
    }

    /// Checks if this destination delivers to a Webhook endpoint.
    #[inline]
    pub fn is_webhook_endpoint(&self) -> bool {
        self.destination_type == EventDestinationType::WebhookEndpoint
    }

    /// Checks if this destination delivers to Amazon CloudWatch.
    #[inline]
    pub fn is_cloudwatch(&self) -> bool {
        self.destination_type == EventDestinationType::Cloudwatch
    }
}

/// Parameters for creating a new Stripe API v2 Event Destination (`POST /v2/core/event_destinations`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateEventDestinationParams {
    /// Delivery transport type.
    #[serde(rename = "type")]
    pub destination_type: EventDestinationType,
    /// List of event type patterns to subscribe to.
    pub events: Vec<String>,
    /// Amazon EventBridge configuration if `destination_type` is `amazon_eventbridge`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amazon_eventbridge: Option<AmazonEventBridgeDetails>,
    /// Azure Event Grid configuration if `destination_type` is `azure_event_grid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azure_event_grid: Option<AzureEventGridDetails>,
    /// Webhook endpoint configuration if `destination_type` is `webhook_endpoint`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_endpoint: Option<WebhookEndpointDetails>,
    /// Amazon CloudWatch configuration if `destination_type` is `cloudwatch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloudwatch: Option<CloudWatchDetails>,
    /// Optional human-readable name for the destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional description for the destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional event payload type (`"thin"` or `"snapshot"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_payload: Option<String>,
    /// Optional metadata key-value pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl CreateEventDestinationParams {
    /// Creates a new `CreateEventDestinationParams` builder with the specified type and event patterns.
    pub fn new(destination_type: EventDestinationType, events: Vec<String>) -> Self {
        Self {
            destination_type,
            events,
            amazon_eventbridge: None,
            azure_event_grid: None,
            webhook_endpoint: None,
            cloudwatch: None,
            name: None,
            description: None,
            event_payload: None,
            metadata: None,
        }
    }

    /// Convenience constructor for Amazon EventBridge destinations.
    pub fn amazon_eventbridge(
        aws_account_id: impl Into<String>,
        aws_region: impl Into<String>,
        events: Vec<String>,
    ) -> Self {
        Self {
            destination_type: EventDestinationType::AmazonEventbridge,
            events,
            amazon_eventbridge: Some(AmazonEventBridgeDetails::new(aws_account_id, aws_region)),
            azure_event_grid: None,
            webhook_endpoint: None,
            name: None,
            description: None,
            event_payload: None,
            metadata: None,
        }
    }

    /// Convenience constructor for Azure Event Grid destinations.
    pub fn azure_event_grid(
        azure_tenant_id: impl Into<String>,
        azure_subscription_id: impl Into<String>,
        azure_resource_group: impl Into<String>,
        azure_event_grid_topic_name: impl Into<String>,
        events: Vec<String>,
    ) -> Self {
        Self {
            destination_type: EventDestinationType::AzureEventGrid,
            events,
            amazon_eventbridge: None,
            azure_event_grid: Some(AzureEventGridDetails::new(
                azure_tenant_id,
                azure_subscription_id,
                azure_resource_group,
                azure_event_grid_topic_name,
            )),
            webhook_endpoint: None,
            name: None,
            description: None,
            event_payload: None,
            metadata: None,
        }
    }

    /// Convenience constructor for standard Webhook destinations.
    pub fn webhook_endpoint(url: impl Into<String>, events: Vec<String>) -> Self {
        Self {
            destination_type: EventDestinationType::WebhookEndpoint,
            events,
            amazon_eventbridge: None,
            azure_event_grid: None,
            webhook_endpoint: Some(WebhookEndpointDetails::new(url)),
            cloudwatch: None,
            name: None,
            description: None,
            event_payload: None,
            metadata: None,
        }
    }

    /// Convenience constructor for Amazon CloudWatch destinations.
    pub fn cloudwatch(
        aws_account_id: impl Into<String>,
        aws_region: impl Into<String>,
        events: Vec<String>,
    ) -> Self {
        Self {
            destination_type: EventDestinationType::Cloudwatch,
            events,
            amazon_eventbridge: None,
            azure_event_grid: None,
            webhook_endpoint: None,
            cloudwatch: Some(CloudWatchDetails::new(aws_account_id, aws_region)),
            name: None,
            description: None,
            event_payload: None,
            metadata: None,
        }
    }

    /// Sets the destination name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the destination description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the event payload delivery type (`"thin"` or `"snapshot"`).
    pub fn with_event_payload(mut self, payload: impl Into<String>) -> Self {
        self.event_payload = Some(payload.into());
        self
    }

    /// Sets metadata key-value pairs.
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets the Amazon EventBridge details.
    pub fn with_amazon_eventbridge(mut self, details: AmazonEventBridgeDetails) -> Self {
        self.amazon_eventbridge = Some(details);
        self
    }

    /// Sets the Azure Event Grid details.
    pub fn with_azure_event_grid(mut self, details: AzureEventGridDetails) -> Self {
        self.azure_event_grid = Some(details);
        self
    }

    /// Sets the Webhook endpoint details.
    pub fn with_webhook_endpoint(mut self, details: WebhookEndpointDetails) -> Self {
        self.webhook_endpoint = Some(details);
        self
    }

    /// Sets the Amazon CloudWatch details.
    pub fn with_cloudwatch(mut self, details: CloudWatchDetails) -> Self {
        self.cloudwatch = Some(details);
        self
    }

    /// Validates the parameter payload against destination invariants.
    pub fn validate(&self) -> Result<(), EventDestinationValidationError> {
        if self.events.is_empty() {
            return Err(EventDestinationValidationError::EmptyEvents);
        }

        match self.destination_type {
            EventDestinationType::AmazonEventbridge => self.validate_eventbridge(),
            EventDestinationType::AzureEventGrid => self.validate_azure(),
            EventDestinationType::WebhookEndpoint => self.validate_webhook(),
            EventDestinationType::Cloudwatch => self.validate_cloudwatch(),
        }
    }

    fn validate_cloudwatch(&self) -> Result<(), EventDestinationValidationError> {
        let details = self.cloudwatch.as_ref().ok_or(
            EventDestinationValidationError::MissingConfiguration("cloudwatch"),
        )?;
        if !is_valid_aws_account_id(&details.aws_account_id) {
            return Err(EventDestinationValidationError::InvalidAwsAccountId(
                details.aws_account_id.clone(),
            ));
        }
        if !is_valid_aws_region(&details.aws_region) {
            return Err(EventDestinationValidationError::InvalidAwsRegion(
                details.aws_region.clone(),
            ));
        }
        Ok(())
    }

    fn validate_eventbridge(&self) -> Result<(), EventDestinationValidationError> {
        let details = self.amazon_eventbridge.as_ref().ok_or(
            EventDestinationValidationError::MissingConfiguration("amazon_eventbridge"),
        )?;
        if !is_valid_aws_account_id(&details.aws_account_id) {
            return Err(EventDestinationValidationError::InvalidAwsAccountId(
                details.aws_account_id.clone(),
            ));
        }
        if !is_valid_aws_region(&details.aws_region) {
            return Err(EventDestinationValidationError::InvalidAwsRegion(
                details.aws_region.clone(),
            ));
        }
        Ok(())
    }

    fn validate_azure(&self) -> Result<(), EventDestinationValidationError> {
        let details = self.azure_event_grid.as_ref().ok_or(
            EventDestinationValidationError::MissingConfiguration("azure_event_grid"),
        )?;
        if !is_valid_azure_detail(&details.azure_tenant_id)
            || !is_valid_azure_detail(&details.azure_subscription_id)
            || !is_valid_azure_detail(&details.azure_resource_group)
            || !is_valid_azure_detail(&details.azure_event_grid_topic_name)
        {
            return Err(EventDestinationValidationError::InvalidAzureDetails(
                "All Azure Event Grid fields must be non-empty strings".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_webhook(&self) -> Result<(), EventDestinationValidationError> {
        let details = self.webhook_endpoint.as_ref().ok_or(
            EventDestinationValidationError::MissingConfiguration("webhook_endpoint"),
        )?;
        if !is_valid_webhook_url(&details.url) {
            return Err(EventDestinationValidationError::InvalidWebhookUrl(
                details.url.clone(),
            ));
        }
        Ok(())
    }
}

/// Parameters for updating a Stripe API v2 Event Destination (`POST /v2/core/event_destinations/{id}`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateEventDestinationParams {
    /// Updated human-readable name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Updated description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Updated operational status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<EventDestinationStatus>,
    /// Updated list of subscribed event types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<String>>,
    /// Updated Amazon EventBridge configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amazon_eventbridge: Option<AmazonEventBridgeDetails>,
    /// Updated Azure Event Grid configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azure_event_grid: Option<AzureEventGridDetails>,
    /// Updated Webhook endpoint configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_endpoint: Option<WebhookEndpointDetails>,
    /// Updated Amazon CloudWatch configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloudwatch: Option<CloudWatchDetails>,
    /// Updated metadata key-value pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl UpdateEventDestinationParams {
    /// Creates an empty `UpdateEventDestinationParams` builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the updated name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the updated description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the updated operational status.
    pub fn with_status(mut self, status: EventDestinationStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Sets the updated event types list.
    pub fn with_events(mut self, events: Vec<String>) -> Self {
        self.events = Some(events);
        self
    }

    /// Sets the updated Amazon EventBridge details.
    pub fn with_amazon_eventbridge(mut self, details: AmazonEventBridgeDetails) -> Self {
        self.amazon_eventbridge = Some(details);
        self
    }

    /// Sets the updated Azure Event Grid details.
    pub fn with_azure_event_grid(mut self, details: AzureEventGridDetails) -> Self {
        self.azure_event_grid = Some(details);
        self
    }

    /// Sets the updated Webhook endpoint details.
    pub fn with_webhook_endpoint(mut self, details: WebhookEndpointDetails) -> Self {
        self.webhook_endpoint = Some(details);
        self
    }

    /// Sets the updated Amazon CloudWatch details.
    pub fn with_cloudwatch(mut self, details: CloudWatchDetails) -> Self {
        self.cloudwatch = Some(details);
        self
    }

    /// Sets the updated metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Validates the updated parameter payload.
    pub fn validate(&self) -> Result<(), EventDestinationValidationError> {
        if let Some(events) = &self.events {
            if events.is_empty() {
                return Err(EventDestinationValidationError::EmptyEvents);
            }
        }
        if let Some(eb) = &self.amazon_eventbridge {
            if !is_valid_aws_account_id(&eb.aws_account_id) {
                return Err(EventDestinationValidationError::InvalidAwsAccountId(
                    eb.aws_account_id.clone(),
                ));
            }
            if !is_valid_aws_region(&eb.aws_region) {
                return Err(EventDestinationValidationError::InvalidAwsRegion(
                    eb.aws_region.clone(),
                ));
            }
        }
        if let Some(wh) = &self.webhook_endpoint {
            if !is_valid_webhook_url(&wh.url) {
                return Err(EventDestinationValidationError::InvalidWebhookUrl(
                    wh.url.clone(),
                ));
            }
        }
        if let Some(cw) = &self.cloudwatch {
            if !is_valid_aws_account_id(&cw.aws_account_id) {
                return Err(EventDestinationValidationError::InvalidAwsAccountId(
                    cw.aws_account_id.clone(),
                ));
            }
            if !is_valid_aws_region(&cw.aws_region) {
                return Err(EventDestinationValidationError::InvalidAwsRegion(
                    cw.aws_region.clone(),
                ));
            }
        }
        if let Some(az) = &self.azure_event_grid {
            if !is_valid_azure_detail(&az.azure_tenant_id)
                || !is_valid_azure_detail(&az.azure_subscription_id)
                || !is_valid_azure_detail(&az.azure_resource_group)
                || !is_valid_azure_detail(&az.azure_event_grid_topic_name)
            {
                return Err(EventDestinationValidationError::InvalidAzureDetails(
                    "Azure Event Grid fields must be non-empty strings".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Paginated list envelope for Stripe API v2 Event Destinations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDestinationList {
    /// Object type descriptor (e.g. `"list"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// Array of event destinations in this page.
    pub data: Vec<EventDestination>,
    /// URL to fetch the next page of results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_url: Option<String>,
    /// URL to fetch the previous page of results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_page_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eventbridge_destination_serialization_roundtrip() {
        let dest = EventDestination::new(
            "edest_test_123",
            EventDestinationStatus::Enabled,
            EventDestinationType::AmazonEventbridge,
            vec!["v2.billing.meter.error_report_triggered".to_string()],
            1728345600,
        )
        .with_amazon_eventbridge(
            AmazonEventBridgeDetails::new("123456789012", "us-east-1")
                .with_partner_event_source_arn(
                    "arn:aws:events:us-east-1:123456789012:event-bus/stripe",
                ),
        )
        .with_name("Production EventBridge")
        .with_updated(1728345700);

        let json = serde_json::to_string(&dest).unwrap();
        let parsed: EventDestination = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "edest_test_123");
        assert_eq!(parsed.object, "v2.core.event_destination");
        assert_eq!(parsed.status, EventDestinationStatus::Enabled);
        assert_eq!(
            parsed.destination_type,
            EventDestinationType::AmazonEventbridge
        );
        assert_eq!(parsed.created, 1728345600);
        assert_eq!(parsed.updated, Some(1728345700));
        assert!(parsed.is_enabled());
        assert!(parsed.is_amazon_eventbridge());

        let eb = parsed.amazon_eventbridge.unwrap();
        assert_eq!(eb.aws_account_id, "123456789012");
        assert_eq!(eb.aws_region, "us-east-1");
        assert_eq!(
            eb.partner_event_source_arn.as_deref(),
            Some("arn:aws:events:us-east-1:123456789012:event-bus/stripe")
        );
    }

    #[test]
    fn test_webhook_destination_deserialization_from_rfc3339() {
        let json = r#"{
            "id": "edest_wh_001",
            "object": "v2.core.event_destination",
            "status": "disabled",
            "type": "webhook_endpoint",
            "events": ["v1.billing.meter.*"],
            "created": "2024-10-22T16:20:09Z",
            "updated": "2024-10-22T16:25:00Z",
            "webhook_endpoint": {
                "url": "https://api.example.com/stripe-webhooks"
            }
        }"#;

        let dest: EventDestination = serde_json::from_str(json).unwrap();
        assert_eq!(dest.id, "edest_wh_001");
        assert_eq!(dest.status, EventDestinationStatus::Disabled);
        assert!(dest.is_disabled());
        assert!(dest.is_webhook_endpoint());
        assert_eq!(dest.created, 1729614009);
        assert_eq!(dest.updated, Some(1729614300));
        let wh = dest.webhook_endpoint.unwrap();
        assert_eq!(wh.url, "https://api.example.com/stripe-webhooks");
    }

    #[test]
    fn test_azure_destination_validation() {
        let valid_params = CreateEventDestinationParams::azure_event_grid(
            "tenant-123",
            "sub-456",
            "rg-stripe",
            "topic-events",
            vec!["v2.*".to_string()],
        );
        assert!(valid_params.validate().is_ok());

        let empty_events = CreateEventDestinationParams::azure_event_grid(
            "tenant-123",
            "sub-456",
            "rg-stripe",
            "topic-events",
            vec![],
        );
        assert_eq!(
            empty_events.validate(),
            Err(EventDestinationValidationError::EmptyEvents)
        );

        let invalid_azure = CreateEventDestinationParams::azure_event_grid(
            "",
            "sub-456",
            "rg-stripe",
            "topic-events",
            vec!["v2.*".to_string()],
        );
        assert!(matches!(
            invalid_azure.validate(),
            Err(EventDestinationValidationError::InvalidAzureDetails(_))
        ));
    }

    #[test]
    fn test_aws_account_id_validation() {
        assert!(is_valid_aws_account_id("123456789012"));
        assert!(!is_valid_aws_account_id("12345"));
        assert!(!is_valid_aws_account_id("12345678901a"));
        assert!(!is_valid_aws_account_id(" 123456789012"));
    }

    #[test]
    fn test_webhook_url_validation() {
        assert!(is_valid_webhook_url("https://example.com/hook"));
        assert!(is_valid_webhook_url("http://localhost:8080/hook"));
        assert!(!is_valid_webhook_url("ftp://example.com"));
        assert!(!is_valid_webhook_url("https://example.com/with space"));
        assert!(!is_valid_webhook_url(""));
    }

    #[test]
    fn test_cloudwatch_destination_roundtrip_and_validation() {
        let params = CreateEventDestinationParams::cloudwatch(
            "123456789012",
            "eu-west-1",
            vec!["v2.billing.meter.*".to_string()],
        )
        .with_cloudwatch(
            CloudWatchDetails::new("123456789012", "eu-west-1")
                .with_log_group_name("/stripe/v2/events"),
        );
        assert!(params.validate().is_ok());

        let dest = EventDestination::new(
            "edest_cw_999",
            EventDestinationStatus::Enabled,
            EventDestinationType::Cloudwatch,
            vec!["v2.billing.meter.*".to_string()],
            1728345600,
        )
        .with_cloudwatch(
            CloudWatchDetails::new("123456789012", "eu-west-1")
                .with_log_group_name("/stripe/v2/events"),
        );

        let json = serde_json::to_string(&dest).unwrap();
        let parsed: EventDestination = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "edest_cw_999");
        assert!(parsed.is_cloudwatch());
        assert_eq!(parsed.destination_type, EventDestinationType::Cloudwatch);
        let cw = parsed.cloudwatch.unwrap();
        assert_eq!(cw.aws_account_id, "123456789012");
        assert_eq!(cw.aws_region, "eu-west-1");
        assert_eq!(cw.log_group_name.as_deref(), Some("/stripe/v2/events"));

        let invalid_cw = CreateEventDestinationParams::cloudwatch(
            "invalid_id",
            "eu-west-1",
            vec!["v2.*".to_string()],
        );
        assert!(matches!(
            invalid_cw.validate(),
            Err(EventDestinationValidationError::InvalidAwsAccountId(_))
        ));
    }
}
