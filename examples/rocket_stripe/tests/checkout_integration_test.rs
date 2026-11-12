//! Integration tests for Hosted Checkout Sessions & EU Omnibus Payment Links (`/api/checkout/*`).
//!
//! Scenarios covered:
//! 1. Service index discovery (`GET /`) and custom JSON error catchers (`404`, `400`, `422`).
//! 2. Creating, retrieving, and expiring a hosted Checkout Session (`/api/checkout/sessions`).
//! 3. Creating and retrieving an EU Omnibus Directive compliant Payment Link (`/api/checkout/payment-links`).
//! 4. Rejecting non-compliant Omnibus discount announcements (`regular_price` vs `prior_30d_lowest_price`).
//! 5. Optional live `.env` sandbox verification when `STRIPE_SECRET_KEY` is provided in `.env`.

mod common;

use common::{
    read_http_request, setup_live_env_rocket_client, setup_mock_rocket_client, write_json_response,
};
use rocket::http::{ContentType, Status};
use serde_json::{Value, json};

#[tokio::test]
async fn test_service_index_and_error_catchers() {
    let (_listener, client) = setup_mock_rocket_client("whsec_test").await;

    // 1. Service directory index
    let res = client.get("/").dispatch().await;
    assert_eq!(res.status(), Status::Ok);
    let body: Value = res.into_json().await.unwrap();
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["service"], "some-stripe Rocket.rs Example");
    assert!(body["data"]["namespaces"]["checkout"].is_array());
    assert!(body["data"]["namespaces"]["webhooks"].is_array());

    // 2. 404 Not Found catcher
    let not_found_res = client.get("/api/non-existent-route").dispatch().await;
    assert_eq!(not_found_res.status(), Status::NotFound);
    let nf_body: Value = not_found_res.into_json().await.unwrap();
    assert_eq!(nf_body["success"], false);

    // 3. 400 / 422 Malformed JSON catcher
    let bad_json_res = client
        .post("/api/checkout/sessions")
        .header(ContentType::JSON)
        .body("{ malformed_json ")
        .dispatch()
        .await;
    assert!(
        bad_json_res.status() == Status::BadRequest
            || bad_json_res.status() == Status::UnprocessableEntity
    );
}

#[tokio::test]
async fn test_checkout_session_create_retrieve_and_expire_lifecycle() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    let mock_server = tokio::spawn(async move {
        // Step 1: POST /v1/checkout/sessions
        let (mut socket1, _) = listener.accept().await.unwrap();
        let (headers1, body1) = read_http_request(&mut socket1).await;
        assert!(headers1.starts_with("POST /v1/checkout/sessions HTTP/1.1"));
        assert!(headers1.to_lowercase().contains("idempotency-key:"));
        assert!(body1.contains("mode=payment"));
        assert!(body1.contains("unit_amount%5D=4900") || body1.contains("unit_amount]=4900"));

        let session_created = json!({
            "id": "cs_test_rocket_101",
            "object": "checkout.session",
            "status": "open",
            "payment_status": "unpaid",
            "url": "https://checkout.stripe.com/c/pay/cs_test_rocket_101"
        });
        write_json_response(&mut socket1, "200 OK", &session_created.to_string()).await;

        // Step 2: GET /v1/checkout/sessions/cs_test_rocket_101
        let (mut socket2, _) = listener.accept().await.unwrap();
        let (headers2, _) = read_http_request(&mut socket2).await;
        assert!(headers2.starts_with("GET /v1/checkout/sessions/cs_test_rocket_101 HTTP/1.1"));
        write_json_response(&mut socket2, "200 OK", &session_created.to_string()).await;

        // Step 3: POST /v1/checkout/sessions/cs_test_rocket_101/expire
        let (mut socket3, _) = listener.accept().await.unwrap();
        let (headers3, _) = read_http_request(&mut socket3).await;
        assert!(
            headers3.starts_with("POST /v1/checkout/sessions/cs_test_rocket_101/expire HTTP/1.1")
        );
        let session_expired = json!({
            "id": "cs_test_rocket_101",
            "object": "checkout.session",
            "status": "expired",
            "payment_status": "unpaid",
            "url": null
        });
        write_json_response(&mut socket3, "200 OK", &session_expired.to_string()).await;
    });

    // 1. Create Checkout Session
    let create_res = client
        .post("/api/checkout/sessions")
        .header(ContentType::JSON)
        .body(
            json!({
                "product_name": "Rocket Pro SDK License",
                "unit_amount": 4900,
                "currency": "usd",
                "quantity": 1
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(create_res.status(), Status::Ok);
    let create_json: Value = create_res.into_json().await.unwrap();
    assert_eq!(create_json["success"], true);
    assert_eq!(create_json["data"]["id"], "cs_test_rocket_101");

    // 2. Retrieve Checkout Session
    let get_res = client
        .get("/api/checkout/sessions/cs_test_rocket_101")
        .dispatch()
        .await;
    assert_eq!(get_res.status(), Status::Ok);
    let get_json: Value = get_res.into_json().await.unwrap();
    assert_eq!(get_json["data"]["status"], "open");

    // 3. Expire Checkout Session
    let expire_res = client
        .post("/api/checkout/sessions/cs_test_rocket_101/expire")
        .dispatch()
        .await;
    assert_eq!(expire_res.status(), Status::Ok);
    let expire_json: Value = expire_res.into_json().await.unwrap();
    assert_eq!(expire_json["data"]["status"], "expired");

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_payment_links_omnibus_price_transparency_and_crud() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    // 1. Reject Omnibus violation when prior_30d_lowest_price is missing on a price reduction
    let violation_res = client
        .post("/api/checkout/payment-links")
        .header(ContentType::JSON)
        .body(
            json!({
                "line_items": [{ "price": "price_123", "quantity": 1 }],
                "regular_price": 10000,
                "announced_discounted_price": 7500,
                "prior_30d_lowest_price": null
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(violation_res.status(), Status::BadRequest);
    let err_json: Value = violation_res.into_json().await.unwrap();
    assert_eq!(err_json["success"], false);
    assert!(
        err_json["error"]
            .as_str()
            .unwrap()
            .contains("Omnibus compliance violation")
    );

    // 2. Valid Omnibus-compliant Payment Link creation & retrieval
    let mock_server = tokio::spawn(async move {
        let (mut socket1, _) = listener.accept().await.unwrap();
        let (headers1, body1) = read_http_request(&mut socket1).await;
        assert!(headers1.starts_with("POST /v1/payment_links HTTP/1.1"));
        assert!(body1.contains("line_items%5B0%5D%5Bprice%5D=price_omnibus_ok"));

        let link_resp = json!({
            "id": "plink_test_omnibus_1",
            "object": "payment_link",
            "active": true,
            "url": "https://buy.stripe.com/test_omnibus_1"
        });
        write_json_response(&mut socket1, "200 OK", &link_resp.to_string()).await;

        let (mut socket2, _) = listener.accept().await.unwrap();
        let (headers2, _) = read_http_request(&mut socket2).await;
        assert!(headers2.starts_with("GET /v1/payment_links/plink_test_omnibus_1 HTTP/1.1"));
        write_json_response(&mut socket2, "200 OK", &link_resp.to_string()).await;
    });

    let ok_res = client
        .post("/api/checkout/payment-links")
        .header(ContentType::JSON)
        .body(
            json!({
                "line_items": [{ "price": "price_omnibus_ok", "quantity": 2 }],
                "regular_price": 10000,
                "announced_discounted_price": 7500,
                "prior_30d_lowest_price": 8000
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(ok_res.status(), Status::Ok);
    let ok_json: Value = ok_res.into_json().await.unwrap();
    assert_eq!(ok_json["data"]["id"], "plink_test_omnibus_1");

    let get_res = client
        .get("/api/checkout/payment-links/plink_test_omnibus_1")
        .dispatch()
        .await;
    assert_eq!(get_res.status(), Status::Ok);

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_live_env_checkout_session_if_configured() {
    let Some(client) = setup_live_env_rocket_client().await else {
        return;
    };

    let res = client
        .post("/api/checkout/sessions")
        .header(ContentType::JSON)
        .body(
            json!({
                "product_name": "Live .env Integration Test Item",
                "unit_amount": 1500,
                "currency": "usd",
                "quantity": 1
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let body: Value = res.into_json().await.unwrap();
    let session_id = body["data"]["id"].as_str().unwrap();
    assert!(session_id.starts_with("cs_"));

    let expire_res = client
        .post(format!("/api/checkout/sessions/{session_id}/expire"))
        .dispatch()
        .await;
    assert_eq!(expire_res.status(), Status::Ok);
}
