//! Unit tests for Customer and Subscription domain entities (US-5.1.0 Unit 1).

use some_stripe_types::customer::Customer;
use some_stripe_types::subscription::{Subscription, SubscriptionStatus};

#[test]
fn test_customer_deserialization_and_fields() {
    let json = r#"{
        "id": "cus_test_123",
        "object": "customer",
        "email": "test@example.com",
        "name": "Jane Test",
        "balance": -5000,
        "currency": "usd",
        "created": 1728345600,
        "delinquent": false,
        "livemode": false,
        "address": {
            "city": "San Francisco",
            "country": "US",
            "line1": "123 Market St",
            "postal_code": "94105",
            "state": "CA"
        }
    }"#;

    let cus: Customer = serde_json::from_str(json).expect("Failed to deserialize Customer");
    assert_eq!(cus.id, "cus_test_123");
    assert_eq!(cus.email.as_deref(), Some("test@example.com"));
    assert_eq!(cus.balance, -5000);
    assert_eq!(
        cus.address.as_ref().unwrap().city.as_deref(),
        Some("San Francisco")
    );
}

#[test]
fn test_subscription_status_transitions_and_predicates() {
    let json = r#"{
        "id": "sub_test_456",
        "object": "subscription",
        "customer": "cus_test_123",
        "status": "trialing",
        "current_period_start": 1728345600,
        "current_period_end": 1730937600,
        "cancel_at_period_end": false,
        "created": 1728345600,
        "trial_start": 1728345600,
        "trial_end": 1729555200,
        "items": {
            "object": "list",
            "data": [
                {
                    "id": "si_item_001",
                    "object": "subscription_item",
                    "subscription": "sub_test_456",
                    "created": 1728345600,
                    "quantity": 2
                }
            ],
            "has_more": false,
            "url": "/v1/subscription_items"
        }
    }"#;

    let sub: Subscription = serde_json::from_str(json).expect("Failed to deserialize Subscription");
    assert_eq!(sub.id, "sub_test_456");
    assert_eq!(sub.status, SubscriptionStatus::Trialing);
    assert!(sub.status.is_operational());
    assert!(!sub.status.is_delinquent_or_canceled());
    assert_eq!(sub.items.data.len(), 1);
    assert_eq!(sub.items.data[0].quantity, Some(2));
}
