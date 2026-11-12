//! Integration tests for Customers, Subscriptions, Invoices, and Billing Portal (`/api/billing/*`).
//!
//! Scenarios covered:
//! 1. Customer vaulting lifecycle: create (`POST /api/billing/customers`), retrieve (`GET`),
//!    and delete (`DELETE /api/billing/customers/<id>`).
//! 2. Subscriptions lifecycle: create (`POST /api/billing/subscriptions`), retrieve (`GET`),
//!    and cancel (`POST /api/billing/subscriptions/<id>/cancel`).
//! 3. Invoices lifecycle: create draft (`POST /api/billing/invoices`), retrieve (`GET`),
//!    finalize (`POST /finalize`), and pay (`POST /pay`).
//! 4. Customer Billing Portal session creation (`POST /api/billing/portal/sessions/<customer_id>`).
//! 5. Optional live `.env` sandbox verification when `STRIPE_SECRET_KEY` is provided in `.env`.

#[path = "../common/mod.rs"]
mod common;

use common::{
    read_http_request, setup_live_env_rocket_client, setup_mock_rocket_client, write_json_response,
};
use rocket::http::{ContentType, Status};
use serde_json::{Value, json};

#[tokio::test]
async fn test_customers_and_subscriptions_lifecycle() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    let mock_server = tokio::spawn(async move {
        // 1. POST /v1/customers
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, b1) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("POST /v1/customers HTTP/1.1"));
        assert!(b1.contains("email=billing%40example.com"));
        let customer_json = json!({
            "id": "cus_rocket_101",
            "object": "customer",
            "email": "billing@example.com",
            "name": "Rocket User",
            "balance": 0,
            "created": 1728345600,
            "delinquent": false,
            "livemode": false
        });
        write_json_response(&mut s1, "200 OK", &customer_json.to_string()).await;

        // 2. GET /v1/customers/cus_rocket_101
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, _) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("GET /v1/customers/cus_rocket_101 HTTP/1.1"));
        write_json_response(&mut s2, "200 OK", &customer_json.to_string()).await;

        // 3. POST /v1/subscriptions
        let (mut s3, _) = listener.accept().await.unwrap();
        let (h3, b3) = read_http_request(&mut s3).await;
        assert!(h3.starts_with("POST /v1/subscriptions HTTP/1.1"));
        assert!(b3.contains("customer=cus_rocket_101"));
        assert!(
            b3.contains("items%5B0%5D%5Bprice%5D=price_pro_monthly")
                || b3.contains("items[0][price]=price_pro_monthly")
        );
        let sub_json = json!({
            "id": "sub_rocket_202",
            "object": "subscription",
            "customer": "cus_rocket_101",
            "status": "active",
            "current_period_start": 1728345600,
            "current_period_end": 1730937600,
            "cancel_at_period_end": false,
            "created": 1728345600,
            "livemode": false
        });
        write_json_response(&mut s3, "200 OK", &sub_json.to_string()).await;

        // 4. GET /v1/subscriptions/sub_rocket_202
        let (mut s4, _) = listener.accept().await.unwrap();
        let (h4, _) = read_http_request(&mut s4).await;
        assert!(h4.starts_with("GET /v1/subscriptions/sub_rocket_202 HTTP/1.1"));
        write_json_response(&mut s4, "200 OK", &sub_json.to_string()).await;

        // 5. DELETE /v1/subscriptions/sub_rocket_202
        let (mut s5, _) = listener.accept().await.unwrap();
        let (h5, _) = read_http_request(&mut s5).await;
        assert!(h5.starts_with("DELETE /v1/subscriptions/sub_rocket_202 HTTP/1.1"));
        let sub_canceled = json!({
            "id": "sub_rocket_202",
            "object": "subscription",
            "customer": "cus_rocket_101",
            "status": "canceled",
            "current_period_start": 1728345600,
            "current_period_end": 1730937600,
            "cancel_at_period_end": false,
            "created": 1728345600,
            "livemode": false
        });
        write_json_response(&mut s5, "200 OK", &sub_canceled.to_string()).await;

        // 6. DELETE /v1/customers/cus_rocket_101
        let (mut s6, _) = listener.accept().await.unwrap();
        let (h6, _) = read_http_request(&mut s6).await;
        assert!(h6.starts_with("DELETE /v1/customers/cus_rocket_101 HTTP/1.1"));
        write_json_response(
            &mut s6,
            "200 OK",
            &json!({
                "id": "cus_rocket_101",
                "object": "customer",
                "deleted": true
            })
            .to_string(),
        )
        .await;
    });

    // 1. Create Customer
    let c_res = client
        .post("/api/billing/customers")
        .header(ContentType::JSON)
        .body(
            json!({
                "email": "billing@example.com",
                "name": "Rocket User",
                "description": "Integration test customer"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(c_res.status(), Status::Ok);

    // 2. Get Customer
    let g_res = client
        .get("/api/billing/customers/cus_rocket_101")
        .dispatch()
        .await;
    assert_eq!(g_res.status(), Status::Ok);

    // 3. Create Subscription
    let s_res = client
        .post("/api/billing/subscriptions")
        .header(ContentType::JSON)
        .body(
            json!({
                "customer": "cus_rocket_101",
                "price_id": "price_pro_monthly",
                "quantity": 2
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(s_res.status(), Status::Ok);

    // 4. Get Subscription
    let sg_res = client
        .get("/api/billing/subscriptions/sub_rocket_202")
        .dispatch()
        .await;
    assert_eq!(sg_res.status(), Status::Ok);

    // 5. Cancel Subscription
    let sc_res = client
        .post("/api/billing/subscriptions/sub_rocket_202/cancel")
        .dispatch()
        .await;
    assert_eq!(sc_res.status(), Status::Ok);
    let sc_json: Value = sc_res.into_json().await.unwrap();
    assert_eq!(sc_json["data"]["status"], "canceled");

    // 6. Delete Customer
    let del_res = client
        .delete("/api/billing/customers/cus_rocket_101")
        .dispatch()
        .await;
    assert_eq!(del_res.status(), Status::Ok);
    let del_json: Value = del_res.into_json().await.unwrap();
    assert_eq!(del_json["data"]["deleted"], true);

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_invoices_and_billing_portal_lifecycle() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    let mock_server = tokio::spawn(async move {
        // 1. POST /v1/invoices
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, b1) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("POST /v1/invoices HTTP/1.1"));
        assert!(b1.contains("customer=cus_rocket_101"));
        let inv_draft = json!({
            "id": "in_rocket_303",
            "object": "invoice",
            "customer": "cus_rocket_101",
            "status": "draft",
            "amount_due": 4900,
            "currency": "usd"
        });
        write_json_response(&mut s1, "200 OK", &inv_draft.to_string()).await;

        // 2. GET /v1/invoices/in_rocket_303
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, _) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("GET /v1/invoices/in_rocket_303 HTTP/1.1"));
        write_json_response(&mut s2, "200 OK", &inv_draft.to_string()).await;

        // 3. POST /v1/invoices/in_rocket_303/finalize
        let (mut s3, _) = listener.accept().await.unwrap();
        let (h3, _) = read_http_request(&mut s3).await;
        assert!(h3.starts_with("POST /v1/invoices/in_rocket_303/finalize HTTP/1.1"));
        let inv_open = json!({
            "id": "in_rocket_303",
            "object": "invoice",
            "customer": "cus_rocket_101",
            "status": "open",
            "amount_due": 4900,
            "currency": "usd"
        });
        write_json_response(&mut s3, "200 OK", &inv_open.to_string()).await;

        // 4. POST /v1/invoices/in_rocket_303/pay
        let (mut s4, _) = listener.accept().await.unwrap();
        let (h4, _) = read_http_request(&mut s4).await;
        assert!(h4.starts_with("POST /v1/invoices/in_rocket_303/pay HTTP/1.1"));
        let inv_paid = json!({
            "id": "in_rocket_303",
            "object": "invoice",
            "customer": "cus_rocket_101",
            "status": "paid",
            "amount_due": 4900,
            "amount_paid": 4900,
            "currency": "usd"
        });
        write_json_response(&mut s4, "200 OK", &inv_paid.to_string()).await;

        // 5. POST /v1/billing_portal/sessions
        let (mut s5, _) = listener.accept().await.unwrap();
        let (h5, b5) = read_http_request(&mut s5).await;
        assert!(h5.starts_with("POST /v1/billing_portal/sessions HTTP/1.1"));
        assert!(b5.contains("customer=cus_rocket_101"));
        write_json_response(
            &mut s5,
            "200 OK",
            &json!({
                "id": "bps_rocket_404",
                "object": "billing_portal.session",
                "created": 1728345600,
                "customer": "cus_rocket_101",
                "url": "https://billing.stripe.com/p/session/bps_rocket_404",
                "return_url": "https://example.com/checkout/success"
            })
            .to_string(),
        )
        .await;
    });

    // Create Invoice
    let i_res = client
        .post("/api/billing/invoices")
        .header(ContentType::JSON)
        .body(
            json!({
                "customer": "cus_rocket_101",
                "collection_method": "charge_automatically",
                "description": "Monthly SaaS Invoice"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(i_res.status(), Status::Ok);

    // Get Invoice
    let ig_res = client
        .get("/api/billing/invoices/in_rocket_303")
        .dispatch()
        .await;
    assert_eq!(ig_res.status(), Status::Ok);

    // Finalize Invoice
    let if_res = client
        .post("/api/billing/invoices/in_rocket_303/finalize")
        .dispatch()
        .await;
    assert_eq!(if_res.status(), Status::Ok);
    let if_json: Value = if_res.into_json().await.unwrap();
    assert_eq!(if_json["data"]["status"], "open");

    // Pay Invoice
    let ip_res = client
        .post("/api/billing/invoices/in_rocket_303/pay")
        .dispatch()
        .await;
    assert_eq!(ip_res.status(), Status::Ok);
    let ip_json: Value = ip_res.into_json().await.unwrap();
    assert_eq!(ip_json["data"]["status"], "paid");

    // Create Billing Portal Session
    let bp_res = client
        .post("/api/billing/portal/sessions/cus_rocket_101")
        .dispatch()
        .await;
    assert_eq!(bp_res.status(), Status::Ok);
    let bp_json: Value = bp_res.into_json().await.unwrap();
    assert_eq!(bp_json["data"]["id"], "bps_rocket_404");

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_live_env_customer_vault_if_configured() {
    let Some(client) = setup_live_env_rocket_client().await else {
        return;
    };

    let c_res = client
        .post("/api/billing/customers")
        .header(ContentType::JSON)
        .body(
            json!({
                "email": "live_env_test@example.com",
                "name": "Live Env Tester"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(c_res.status(), Status::Ok);
    let c_json: Value = c_res.into_json().await.unwrap();
    let cus_id = c_json["data"]["id"].as_str().unwrap();

    let del_res = client
        .delete(format!("/api/billing/customers/{cus_id}"))
        .dispatch()
        .await;
    assert_eq!(del_res.status(), Status::Ok);
}
