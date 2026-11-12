//! Integration Test Suite for Example 5: `actix_invoicing_and_billing_portal`
//!
//! Covers all 35 Actix-Web routes across `invoices`, `credit_notes`, `quotes`, and
//! `billing_portal`, including upcoming invoice preview (`id: None`), zero-copy PDF
//! streaming (`bytes::Bytes`), and credit note balance deduction validation.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/actix_web/actix_invoicing_and_billing_portal.rs"]
mod example_app;

use actix_web::{App, http::StatusCode, test, web};
use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_app::{RemoveLinesRequest, ValidateCreditDeductionRequest, configure_routes};
use some_stripe::{
    AddInvoiceLinesParams, CreateCreditNoteParams, CreateInvoiceParams,
    CreatePortalConfigurationParams, CreatePortalSessionParams, CreateQuoteParams,
    InvoiceUpcomingParams, PreviewCreditNoteParams, UpdateCreditNoteParams,
    UpdateInvoiceLineParams, UpdateInvoiceParams, UpdatePortalConfigurationParams,
    UpdateQuoteParams,
};

#[actix_web::test]
async fn scenario_1_invoices_lifecycle_lines_upcoming_and_pdf_stream() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","customer":"cus_001","status":"draft"}"#),
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","customer":"cus_001","status":"draft"}"#),
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","customer":"cus_001","status":"draft"}"#),
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","deleted":true}"#),
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","customer":"cus_001","status":"open"}"#),
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","customer":"cus_001","status":"paid"}"#),
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","customer":"cus_001","status":"void"}"#),
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","customer":"cus_001","status":"uncollectible"}"#),
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","customer":"cus_001","status":"open"}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"in_001","object":"invoice"}],"has_more":false,"url":"/v1/invoices"}"#),
        MockWireResponse::json(200, r#"{"id":null,"object":"invoice","customer":"cus_001","amount_due":4500}"#),
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","customer":"cus_001"}"#),
        MockWireResponse::json(200, r#"{"id":"il_001","object":"line_item","amount":1500,"currency":"usd"}"#),
        MockWireResponse::json(200, r#"{"id":"in_001","object":"invoice","customer":"cus_001"}"#),
        MockWireResponse::pdf(b"%PDF-1.7 fake invoice pdf bytes"),
    ])
    .await;

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices").set_json(&CreateInvoiceParams::new("cus_001")).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/invoices/in_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices/in_001").set_json(&UpdateInvoiceParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::delete().uri("/invoices/in_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices/in_001/finalize").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices/in_001/pay").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices/in_001/void").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices/in_001/mark-uncollectible").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices/in_001/send").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/invoices?limit=5").to_request()).await.status(), StatusCode::OK);

    let mut upcoming_params = InvoiceUpcomingParams::new();
    upcoming_params.customer = Some("cus_001".to_string());
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices/upcoming").set_json(&upcoming_params).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices/in_001/lines").set_json(&AddInvoiceLinesParams::new(vec![])).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices/in_001/lines/il_001").set_json(&UpdateInvoiceLineParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/invoices/in_001/remove-lines").set_json(&RemoveLinesRequest { line_ids: vec!["il_001".to_string()] }).to_request()).await.status(), StatusCode::OK);

    let pdf_resp = test::call_service(&app, test::TestRequest::get().uri("/invoices/in_001/pdf").to_request()).await;
    assert_eq!(pdf_resp.status(), StatusCode::OK);
    let pdf_bytes = test::read_body(pdf_resp).await;
    assert!(pdf_bytes.starts_with(b"%PDF-1.7"));

    assert_eq!(mock.finish().await.len(), 15);
}

#[actix_web::test]
async fn scenario_2_credit_notes_quotes_and_billing_portal() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"cn_001","object":"credit_note","invoice":"in_001","amount":1000,"currency":"usd","status":"issued"}"#),
        MockWireResponse::json(200, r#"{"id":"cn_001","object":"credit_note","invoice":"in_001","amount":1000,"currency":"usd","status":"issued"}"#),
        MockWireResponse::json(200, r#"{"id":"cn_001","object":"credit_note","invoice":"in_001","amount":1000,"currency":"usd","status":"issued"}"#),
        MockWireResponse::json(200, r#"{"id":"cn_001","object":"credit_note","invoice":"in_001","amount":1000,"currency":"usd","status":"void"}"#),
        MockWireResponse::json(200, r#"{"id":"cn_preview","object":"credit_note","invoice":"in_001","amount":1000,"currency":"usd","status":"issued"}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"cn_001","object":"credit_note","invoice":"in_001","amount":1000,"currency":"usd","status":"issued"}],"has_more":false,"url":"/v1/credit_notes"}"#),
        MockWireResponse::json(200, r#"{"id":"qt_001","object":"quote","status":"draft","amount_total":5000}"#),
        MockWireResponse::json(200, r#"{"id":"qt_001","object":"quote","status":"draft","amount_total":5000}"#),
        MockWireResponse::json(200, r#"{"id":"qt_001","object":"quote","status":"draft","amount_total":5000}"#),
        MockWireResponse::json(200, r#"{"id":"qt_001","object":"quote","status":"open","amount_total":5000}"#),
        MockWireResponse::json(200, r#"{"id":"qt_001","object":"quote","status":"accepted","amount_total":5000}"#),
        MockWireResponse::json(200, r#"{"id":"qt_001","object":"quote","status":"canceled","amount_total":5000}"#),
        MockWireResponse::pdf(b"%PDF-1.7 fake quote pdf bytes"),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"qt_001","object":"quote","status":"accepted"}],"has_more":false,"url":"/v1/quotes"}"#),
        MockWireResponse::json(200, r#"{"id":"bps_001","object":"billing_portal.session","customer":"cus_001","url":"https://billing.stripe.com/p/session/bps_001"}"#),
        MockWireResponse::json(200, r#"{"id":"bpc_001","object":"billing_portal.configuration","active":true}"#),
        MockWireResponse::json(200, r#"{"id":"bpc_001","object":"billing_portal.configuration","active":true}"#),
        MockWireResponse::json(200, r#"{"id":"bpc_001","object":"billing_portal.configuration","active":false}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"bpc_001","object":"billing_portal.configuration","active":true}],"has_more":false,"url":"/v1/billing_portal/configurations"}"#),
    ])
    .await;

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    let cn_create = CreateCreditNoteParams {
        invoice: "in_001".to_string(),
        amount: Some(1000),
        credit_amount: None,
        refund_amount: None,
        out_of_band_amount: None,
        reason: None,
        memo: None,
        lines: None,
        metadata: None,
        idempotency_key: None,
    };
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/credit-notes").set_json(&cn_create).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/credit-notes/cn_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/credit-notes/cn_001").set_json(&UpdateCreditNoteParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/credit-notes/cn_001/void").to_request()).await.status(), StatusCode::OK);

    let cn_preview = PreviewCreditNoteParams {
        invoice: "in_001".to_string(),
        amount: Some(1000),
        credit_amount: None,
        refund_amount: None,
        out_of_band_amount: None,
        reason: None,
        memo: None,
        lines: None,
        metadata: None,
    };
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/credit-notes/preview").set_json(&cn_preview).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/credit-notes?invoice=in_001").to_request()).await.status(), StatusCode::OK);

    // Validate credit deduction invariants (valid + invalid)
    assert_eq!(
        test::call_service(&app, test::TestRequest::post().uri("/credit-notes/validate-deduction").set_json(&ValidateCreditDeductionRequest { credit_amount: 1000, invoice_paid_amount: 5000 }).to_request()).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        test::call_service(&app, test::TestRequest::post().uri("/credit-notes/validate-deduction").set_json(&ValidateCreditDeductionRequest { credit_amount: 6000, invoice_paid_amount: 5000 }).to_request()).await.status(),
        StatusCode::BAD_GATEWAY
    );

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/quotes").set_json(&CreateQuoteParams::new()).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/quotes/qt_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/quotes/qt_001").set_json(&UpdateQuoteParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/quotes/qt_001/finalize").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/quotes/qt_001/accept").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/quotes/qt_001/cancel").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/quotes/qt_001/pdf").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/quotes?limit=5").to_request()).await.status(), StatusCode::OK);

    let mut portal_sess = CreatePortalSessionParams::new();
    portal_sess.customer = "cus_001".to_string();
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/billing-portal/sessions").set_json(&portal_sess).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/billing-portal/configurations").set_json(&CreatePortalConfigurationParams::new()).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/billing-portal/configurations/bpc_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/billing-portal/configurations/bpc_001").set_json(&UpdatePortalConfigurationParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/billing-portal/configurations").to_request()).await.status(), StatusCode::OK);

    assert_eq!(mock.finish().await.len(), 19);
}
