//! Integration and verification tests for Blocker 2:
//! Multi-tenant paginator isolation and filter parameter forwarding.

use some_stripe::patterns::Paginator;
use some_stripe::{
    CreditNotesResource, ListCreditNotesParams, ListQuotesParams, ListSubscriptionSchedulesParams,
    PayoutsResource, QuotesResource, StripeClient, StripeConfig, SubscriptionSchedulesResource,
};
use some_stripe_types::QuoteStatus;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn write_json_response(socket: &mut tokio::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

#[tokio::test]
async fn test_paginator_with_account_header() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("GET /v1/test_items?limit=10 HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_connected_tenant_123"),
            "Wire request must include Stripe-Account header: {wire}"
        );

        let body = r#"{
            "object": "list",
            "data": [],
            "has_more": false
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config = StripeConfig::new("sk_test_paginator").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    #[derive(serde::Deserialize)]
    struct Dummy {
        id: String,
    }

    let mut paginator: Paginator<'_, Dummy> =
        Paginator::new(&client, "v1/test_items", 10, |d: &Dummy| &d.id)
            .with_account("acct_connected_tenant_123");

    assert_eq!(
        paginator.stripe_account(),
        Some("acct_connected_tenant_123")
    );
    let item = paginator.next_item().await.unwrap();
    assert!(item.is_none());

    server.await.unwrap();
}

#[tokio::test]
async fn test_accounts_and_persons_paginate_forwards_account() {
    let config = StripeConfig::new("sk_test_accounts").with_api_base("http://localhost:12345");
    let client = StripeClient::with_config(config);

    // AccountsResource::paginate
    let accounts_resource = client.accounts().with_account("acct_tenant_acc");
    let paginator = accounts_resource.paginate(Some(15));
    assert_eq!(paginator.stripe_account(), Some("acct_tenant_acc"));

    // PersonsResource::paginate
    let persons_resource = client.persons().with_account("acct_tenant_per");
    let paginator = persons_resource
        .paginate("acct_target_123", Some(25))
        .unwrap();
    assert_eq!(paginator.stripe_account(), Some("acct_tenant_per"));
}

#[tokio::test]
async fn test_payouts_transfers_application_fees_with_account() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Payouts list
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/payouts HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_payout_custom")
        );
        let resp = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/payouts"}"#;
        write_json_response(&mut socket, resp).await;

        // 2. Transfers list
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/transfers HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_transfer_custom")
        );
        let resp = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/transfers"}"#;
        write_json_response(&mut socket, resp).await;

        // 3. Application Fees list
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/application_fees HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_fee_custom")
        );
        let resp = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/application_fees"}"#;
        write_json_response(&mut socket, resp).await;
    });

    let config = StripeConfig::new("sk_test_resources").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let payouts: PayoutsResource<'_> = client.payouts().with_account("acct_payout_custom");
    let _ = payouts.list(None).await.unwrap();

    let transfers = client.transfers().with_account("acct_transfer_custom");
    let _ = transfers.list(None).await.unwrap();

    let fees = client.application_fees().with_account("acct_fee_custom");
    let _ = fees.list(None).await.unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_schedules_list_encodes_filters() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("GET /v1/subscription_schedules?"));
        assert!(wire.contains("customer=cus_sched_123"));
        assert!(wire.contains("scheduled=true"));
        assert!(wire.contains("limit=15"));

        let resp =
            r#"{"object":"list","data":[],"has_more":false,"url":"/v1/subscription_schedules"}"#;
        write_json_response(&mut socket, resp).await;
    });

    let config = StripeConfig::new("sk_test_schedules").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = ListSubscriptionSchedulesParams {
        customer: Some("cus_sched_123".into()),
        scheduled: Some(true),
        limit: Some(15),
        ..Default::default()
    };

    let schedules: SubscriptionSchedulesResource<'_> = client.subscription_schedules();
    let mut paginator = schedules.list(&params);
    let item = paginator.next_item().await.unwrap();
    assert!(item.is_none());

    server.await.unwrap();
}

#[tokio::test]
async fn test_credit_notes_list_paginated_encodes_filters() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("GET /v1/credit_notes?"));
        assert!(wire.contains("customer=cus_cn_456"));
        assert!(wire.contains("invoice=in_cn_456"));
        assert!(wire.contains("limit=20"));

        let resp = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/credit_notes"}"#;
        write_json_response(&mut socket, resp).await;
    });

    let config = StripeConfig::new("sk_test_cn").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = ListCreditNotesParams {
        customer: Some("cus_cn_456".into()),
        invoice: Some("in_cn_456".into()),
        limit: Some(20),
        ..Default::default()
    };

    let credit_notes: CreditNotesResource<'_> = client.credit_notes();
    let mut paginator = credit_notes.list_paginated(&params);
    let item = paginator.next_item().await.unwrap();
    assert!(item.is_none());

    server.await.unwrap();
}

#[tokio::test]
async fn test_quotes_list_paginated_encodes_filters() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("GET /v1/quotes?"));
        assert!(wire.contains("customer=cus_quote_789"));
        assert!(wire.contains("status=open"));
        assert!(wire.contains("limit=5"));

        let resp = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/quotes"}"#;
        write_json_response(&mut socket, resp).await;
    });

    let config = StripeConfig::new("sk_test_quotes").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = ListQuotesParams {
        customer: Some("cus_quote_789".into()),
        status: Some(QuoteStatus::Open),
        limit: Some(5),
        ..Default::default()
    };

    let quotes: QuotesResource<'_> = client.quotes();
    let mut paginator = quotes.list_paginated(&params);
    let item = paginator.next_item().await.unwrap();
    assert!(item.is_none());

    server.await.unwrap();
}
