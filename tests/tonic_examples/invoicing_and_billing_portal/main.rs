//! Integration Test Suite for Example 5: `tonic_invoicing_and_billing_portal`
//!
//! Covers all 35 Tonic gRPC methods across `invoices`, `credit_notes`, `quotes`, and
//! `billing_portal`, including upcoming invoice preview (`id: None`), binary PDF
//! downloads (`BinaryPdfResponse`), and credit note balance deduction validation.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/tonic_grpc/tonic_invoicing_and_billing_portal.rs"]
mod example_grpc;

use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_grpc::{
    InvoicingPortalGrpcServer, InvoicingPortalGrpcService, InvoicingRpcRequest, RemoveLinesInput,
    ValidateCreditDeductionInput,
};
use some_stripe::{
    AddInvoiceLinesParams, CreateCreditNoteParams, CreateInvoiceParams,
    CreatePortalConfigurationParams, CreatePortalSessionParams, CreateQuoteParams,
    InvoiceUpcomingParams, PreviewCreditNoteParams, UpdateCreditNoteParams,
    UpdateInvoiceLineParams, UpdateInvoiceParams, UpdatePortalConfigurationParams,
    UpdateQuoteParams,
};
use tonic::{Code, Request};

/// Verifies `Invoice` lifecycle, line mutations, upcoming preview (`id: None`), and binary PDF RPCs.
#[tokio::test]
async fn scenario_1_invoices_lifecycle_lines_upcoming_and_pdf_download() {
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

    let server = InvoicingPortalGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    assert!(server.create_invoice(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&CreateInvoiceParams::new("cus_001")).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_invoice(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_invoice(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&UpdateInvoiceParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.delete_invoice(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.finalize_invoice(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.pay_invoice(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.void_invoice(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.mark_invoice_uncollectible(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.send_invoice(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_invoices(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());

    let mut upcoming_params = InvoiceUpcomingParams::new();
    upcoming_params.customer = Some("cus_001".to_string());
    assert!(server.retrieve_upcoming_invoice(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&upcoming_params).expect("json"),
    })).await.is_ok());
    assert!(server.add_invoice_lines(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&AddInvoiceLinesParams::new(vec![])).expect("json"),
    })).await.is_ok());
    assert!(server.update_invoice_line(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: "il_001".to_string(),
        json_params: serde_json::to_string(&UpdateInvoiceLineParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.remove_invoice_lines(Request::new(InvoicingRpcRequest {
        id: "in_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&RemoveLinesInput { line_ids: vec!["il_001".to_string()] }).expect("json"),
    })).await.is_ok());

    let pdf_resp = server
        .download_invoice_pdf(Request::new(InvoicingRpcRequest {
            id: "in_001".to_string(),
            sub_id: String::new(),
            json_params: String::new(),
        }))
        .await
        .expect("download_invoice_pdf")
        .into_inner();
    assert_eq!(pdf_resp.content_type, "application/pdf");
    assert!(pdf_resp.pdf_bytes.starts_with(b"%PDF-1.7"));

    assert_eq!(mock.finish().await.len(), 15);
}

/// Verifies `CreditNote`, `Quote`, and `BillingPortal` gRPC methods and deduction invariant checks.
#[tokio::test]
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

    let server = InvoicingPortalGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

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
    assert!(server.create_credit_note(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&cn_create).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_credit_note(Request::new(InvoicingRpcRequest {
        id: "cn_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_credit_note(Request::new(InvoicingRpcRequest {
        id: "cn_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&UpdateCreditNoteParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.void_credit_note(Request::new(InvoicingRpcRequest {
        id: "cn_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());

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
    assert!(server.preview_credit_note(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&cn_preview).expect("json"),
    })).await.is_ok());
    assert!(server.list_credit_notes(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());

    assert!(server.validate_credit_deduction(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&ValidateCreditDeductionInput {
            credit_amount: 1000,
            invoice_paid_amount: 5000,
        }).expect("json"),
    })).await.is_ok());

    let bad_deduct = server
        .validate_credit_deduction(Request::new(InvoicingRpcRequest {
            id: String::new(),
            sub_id: String::new(),
            json_params: serde_json::to_string(&ValidateCreditDeductionInput {
                credit_amount: 6000,
                invoice_paid_amount: 5000,
            })
            .expect("json"),
        }))
        .await
        .expect_err("excessive credit deduction must fail");
    assert!(matches!(
        bad_deduct.code(),
        Code::InvalidArgument | Code::Unavailable
    ));

    assert!(server.create_quote(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&CreateQuoteParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_quote(Request::new(InvoicingRpcRequest {
        id: "qt_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_quote(Request::new(InvoicingRpcRequest {
        id: "qt_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&UpdateQuoteParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.finalize_quote(Request::new(InvoicingRpcRequest {
        id: "qt_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.accept_quote(Request::new(InvoicingRpcRequest {
        id: "qt_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.cancel_quote(Request::new(InvoicingRpcRequest {
        id: "qt_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    let qt_pdf = server
        .download_quote_pdf(Request::new(InvoicingRpcRequest {
            id: "qt_001".to_string(),
            sub_id: String::new(),
            json_params: String::new(),
        }))
        .await
        .expect("download_quote_pdf")
        .into_inner();
    assert!(qt_pdf.pdf_bytes.starts_with(b"%PDF-1.7"));
    assert!(server.list_quotes(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());

    let mut portal_sess = CreatePortalSessionParams::new();
    portal_sess.customer = "cus_001".to_string();
    assert!(server.create_portal_session(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&portal_sess).expect("json"),
    })).await.is_ok());
    assert!(server.create_portal_configuration(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&CreatePortalConfigurationParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_portal_configuration(Request::new(InvoicingRpcRequest {
        id: "bpc_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_portal_configuration(Request::new(InvoicingRpcRequest {
        id: "bpc_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&UpdatePortalConfigurationParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.list_portal_configurations(Request::new(InvoicingRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());

    assert_eq!(mock.finish().await.len(), 19);
}
