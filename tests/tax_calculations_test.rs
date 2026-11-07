//! Integration and mock wire tests for Stripe Tax Calculations resource (Unit 1).
//!
//! Covers:
//! - US multi-jurisdiction apportionment (state, county, special district).
//! - EU VAT Directive 2006/112/EC Article 196 reverse charge zero-rate evaluation.
//! - Wire protocol validation against mock HTTP server for `create`, `retrieve`, `list_line_items`, and `list_line_items_page`.
//! - Gate G2 UUID v4 automatic idempotency resolution.
//! - Gate G-Borrow zero deep-clone borrowing discipline.

use some_stripe::resources::CalculationsResource;
use some_stripe::{
    CreateTaxCalculationParams, CustomerTaxAddress, CustomerTaxDetails, EuVatReverseChargeError,
    ListTaxCalculationLineItemsParams, StripeClient, StripeConfig, TaxBreakdown, TaxCalculation,
    TaxCalculationCustomerTaxId, TaxCalculationLineItem, TaxCalculationMode, TaxCalculationStatus,
    TaxJurisdiction, TaxLineItemParam, TaxRateDetails, is_valid_uuid_v4,
    validate_eu_vat_reverse_charge,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_tax_calculation_mode_and_status_enums() {
    let mode = TaxCalculationMode::ReverseCharge;
    assert_eq!(mode.as_str(), "reverse_charge");
    assert!(mode.is_reverse_charge());
    assert!(!mode.is_exempt());

    let exempt = TaxCalculationMode::Exempt;
    assert!(exempt.is_exempt());
    assert!(!exempt.is_reverse_charge());

    let status = TaxCalculationStatus::Active;
    assert_eq!(status.as_str(), "active");
    assert!(status.is_active());
    assert!(!status.is_expired());
    assert!(!status.is_committed());

    let expired = TaxCalculationStatus::Expired;
    assert!(expired.is_expired());
    assert!(!expired.is_active());

    let committed = TaxCalculationStatus::Committed;
    assert!(committed.is_committed());
}

#[test]
fn test_customer_tax_address_and_details_builders() {
    let addr = CustomerTaxAddress::new("US")
        .state("CA")
        .postal_code("94103")
        .city("San Francisco")
        .line1("123 Market St")
        .line2("Suite 400");

    assert_eq!(addr.country, "US");
    assert_eq!(addr.state.as_deref(), Some("CA"));
    assert_eq!(addr.postal_code.as_deref(), Some("94103"));
    assert_eq!(addr.city.as_deref(), Some("San Francisco"));
    assert_eq!(addr.line1.as_deref(), Some("123 Market St"));
    assert_eq!(addr.line2.as_deref(), Some("Suite 400"));

    let details = CustomerTaxDetails::new()
        .address(addr)
        .address_source("shipping")
        .ip_address("192.168.1.1")
        .tax_id("eu_vat", "DE123456789")
        .taxability_override("none");

    assert_eq!(details.address_source.as_deref(), Some("shipping"));
    assert_eq!(details.ip_address.as_deref(), Some("192.168.1.1"));
    assert_eq!(details.taxability_override.as_deref(), Some("none"));
    assert!(details.tax_ids.is_some());
    let ids = details.tax_ids.unwrap();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].tax_id_type, "eu_vat");
    assert_eq!(ids[0].value, "DE123456789");

    let direct_id = TaxCalculationCustomerTaxId::new("gb_vat", "GB123456789");
    assert_eq!(direct_id.tax_id_type, "gb_vat");
    assert_eq!(direct_id.value, "GB123456789");
}

#[test]
fn test_us_multi_jurisdiction_tax_breakdown_full() {
    let json = r#"{
        "id": "taxcalc_us_ca_full",
        "object": "tax.calculation",
        "amount_total": 10863,
        "currency": "usd",
        "customer_details": {
            "address": {
                "city": "San Francisco",
                "country": "US",
                "line1": "901 Flower St",
                "postal_code": "94103",
                "state": "CA"
            },
            "address_source": "shipping"
        },
        "tax_amount_exclusive": 863,
        "tax_amount_inclusive": 0,
        "tax_breakdown": [
            {
                "amount": 600,
                "inclusive": false,
                "jurisdiction": {
                    "country": "US",
                    "state": "CA",
                    "level": "state",
                    "display_name": "California State Tax"
                },
                "tax_rate_details": {
                    "country": "US",
                    "state": "CA",
                    "percentage_decimal": "6.00",
                    "tax_type": "sales_tax"
                },
                "taxable_amount": 10000,
                "sourcing": "destination"
            },
            {
                "amount": 125,
                "inclusive": false,
                "jurisdiction": {
                    "country": "US",
                    "state": "CA",
                    "county": "San Francisco",
                    "level": "county",
                    "display_name": "San Francisco County Tax"
                },
                "tax_rate_details": {
                    "country": "US",
                    "state": "CA",
                    "percentage_decimal": "1.25",
                    "tax_type": "sales_tax"
                },
                "taxable_amount": 10000,
                "sourcing": "destination"
            },
            {
                "amount": 138,
                "inclusive": false,
                "jurisdiction": {
                    "country": "US",
                    "state": "CA",
                    "district": "BART District",
                    "level": "district",
                    "display_name": "Bay Area Rapid Transit District"
                },
                "tax_rate_details": {
                    "country": "US",
                    "state": "CA",
                    "percentage_decimal": "1.38",
                    "tax_type": "sales_tax"
                },
                "taxable_amount": 10000,
                "sourcing": "destination"
            }
        ],
        "tax_date": 1728345600,
        "status": "active",
        "calculation_mode": "standard"
    }"#;

    let calc: TaxCalculation = serde_json::from_str(json).unwrap();
    assert_eq!(calc.id(), "taxcalc_us_ca_full");
    assert_eq!(calc.amount_total, 10863);
    assert_eq!(calc.tax_amount_exclusive, 863);
    assert_eq!(calc.tax_amount_inclusive, 0);
    assert_eq!(calc.total_tax(), 863);

    let breakdowns = calc.tax_breakdown();
    assert_eq!(breakdowns.len(), 3);

    // Verify State Jurisdiction
    assert_eq!(breakdowns[0].jurisdiction.level, "state");
    assert_eq!(
        breakdowns[0].jurisdiction.display_name,
        "California State Tax"
    );
    assert_eq!(breakdowns[0].amount, 600);

    // Verify County Jurisdiction
    assert_eq!(breakdowns[1].jurisdiction.level, "county");
    assert_eq!(
        breakdowns[1].jurisdiction.display_name,
        "San Francisco County Tax"
    );
    assert_eq!(breakdowns[1].amount, 125);

    // Verify Special District Jurisdiction
    assert_eq!(breakdowns[2].jurisdiction.level, "district");
    assert_eq!(
        breakdowns[2].jurisdiction.display_name,
        "Bay Area Rapid Transit District"
    );
    assert_eq!(breakdowns[2].amount, 138);

    assert_eq!(
        breakdowns[0].amount + breakdowns[1].amount + breakdowns[2].amount,
        863
    );
}

#[test]
fn test_eu_vat_reverse_charge_directive_2006_112_ec_art_196_full_matrix() {
    let valid_b2b_calc = TaxCalculation {
        id: "taxcalc_eu_b2b_01".into(),
        object: "tax.calculation".into(),
        amount_total: 20000,
        currency: "eur".into(),
        customer: None,
        customer_details: CustomerTaxDetails::new()
            .address(CustomerTaxAddress::new("FR").city("Paris"))
            .address_source("shipping")
            .tax_id("eu_vat", "FR12345678901")
            .taxability_override("reverse_charge"),
        expires_at: Some(1730000000),
        line_items: None,
        livemode: false,
        shipping_cost: None,
        tax_amount_exclusive: 0,
        tax_amount_inclusive: 0,
        tax_breakdown: vec![TaxBreakdown {
            amount: 0,
            inclusive: false,
            jurisdiction: TaxJurisdiction {
                country: "FR".into(),
                level: "country".into(),
                display_name: "France".into(),
                ..Default::default()
            },
            taxability_reason: Some("reverse_charge".into()),
            taxable_amount: Some(20000),
            tax_rate_details: Some(TaxRateDetails {
                country: Some("FR".into()),
                percentage_decimal: "0.0".into(),
                tax_type: Some("vat".into()),
                rate_type: Some("percentage".into()),
                taxability_reason: Some("reverse_charge".into()),
                ..Default::default()
            }),
            sourcing: Some("destination".into()),
        }],
        tax_date: Some(1728345600),
        status: TaxCalculationStatus::Active,
        calculation_mode: TaxCalculationMode::ReverseCharge,
    };

    // Valid: Supplier IE -> Customer FR (cross-border, VAT ID supplied, reverse charge designated)
    let res = validate_eu_vat_reverse_charge(&valid_b2b_calc, "IE");
    assert!(res.is_ok());

    // Invariant 1: Domestic supplies are not eligible for cross-border Art 196 reverse charge
    let domestic = validate_eu_vat_reverse_charge(&valid_b2b_calc, "FR");
    assert!(matches!(
        domestic,
        Err(EuVatReverseChargeError::DomesticTransactionNotEligible)
    ));

    // Invariant 2: Total tax must be exactly zero
    let mut invalid_tax = valid_b2b_calc.clone();
    invalid_tax.tax_amount_exclusive = 4000;
    let nonzero = validate_eu_vat_reverse_charge(&invalid_tax, "IE");
    assert!(matches!(
        nonzero,
        Err(EuVatReverseChargeError::NonZeroTaxAmount(4000))
    ));

    // Invariant 3: Customer VAT ID is mandatory under Article 196
    let mut no_vat_id = valid_b2b_calc.clone();
    no_vat_id.customer_details.tax_ids = None;
    let missing_vat = validate_eu_vat_reverse_charge(&no_vat_id, "IE");
    assert!(matches!(
        missing_vat,
        Err(EuVatReverseChargeError::MissingCustomerVatId)
    ));

    // Invariant 4: Reverse charge designation must be explicit
    let mut no_designation = valid_b2b_calc.clone();
    no_designation.calculation_mode = TaxCalculationMode::Standard;
    no_designation.customer_details.taxability_override = None;
    no_designation.tax_breakdown[0].taxability_reason = Some("standard_rated".into());
    no_designation.tax_breakdown[0].tax_rate_details = None;
    let missing_designation = validate_eu_vat_reverse_charge(&no_designation, "IE");
    assert!(matches!(
        missing_designation,
        Err(EuVatReverseChargeError::MissingReverseChargeDesignation)
    ));
}

#[tokio::test]
async fn test_create_tax_calculation_mock_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v1/tax/calculations HTTP/1.1"));
        assert!(req_str.contains("currency=usd"));
        assert!(req_str.contains("customer_details%5Baddress%5D%5Bcountry%5D=US"));
        assert!(req_str.contains("customer_details%5Baddress%5D%5Bstate%5D=CA"));
        assert!(req_str.contains("customer_details%5Baddress%5D%5Bpostal_code%5D=94103"));
        assert!(req_str.contains("customer_details%5Baddress_source%5D=shipping"));
        assert!(req_str.contains("line_items%5B0%5D%5Bamount%5D=10000"));
        assert!(req_str.contains("line_items%5B0%5D%5Breference%5D=item_1"));
        assert!(req_str.contains("line_items%5B0%5D%5Btax_code%5D=txcd_10000000"));

        // Verify Gate G2 UUID v4 automatic idempotency header injection
        let idemp_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .map(|l| l.split(':').nth(1).unwrap().trim())
            .expect("Idempotency-Key header missing");
        assert!(is_valid_uuid_v4(idemp_header));

        let body = r#"{
            "id": "taxcalc_wire_001",
            "object": "tax.calculation",
            "amount_total": 10725,
            "currency": "usd",
            "customer_details": {
                "address": {
                    "country": "US",
                    "state": "CA",
                    "postal_code": "94103"
                },
                "address_source": "shipping"
            },
            "tax_amount_exclusive": 725,
            "tax_amount_inclusive": 0,
            "tax_breakdown": [
                {
                    "amount": 725,
                    "inclusive": false,
                    "jurisdiction": {
                        "country": "US",
                        "state": "CA",
                        "level": "state",
                        "display_name": "California"
                    },
                    "tax_rate_details": {
                        "country": "US",
                        "state": "CA",
                        "percentage_decimal": "7.25",
                        "tax_type": "sales_tax"
                    },
                    "taxable_amount": 10000
                }
            ],
            "tax_date": 1728345600,
            "status": "active",
            "calculation_mode": "standard"
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_tax").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = CalculationsResource::new(&client);

    let params = CreateTaxCalculationParams::new("usd")
        .customer_details(
            CustomerTaxDetails::new()
                .address(
                    CustomerTaxAddress::new("US")
                        .state("CA")
                        .postal_code("94103"),
                )
                .address_source("shipping"),
        )
        .line_item(
            TaxLineItemParam::new(10000, "item_1")
                .tax_code("txcd_10000000")
                .quantity(1),
        );

    let calculation = resource.create(&params).await.unwrap();
    assert_eq!(calculation.id, "taxcalc_wire_001");
    assert_eq!(calculation.amount_total, 10725);
    assert_eq!(calculation.tax_amount_exclusive, 725);
    assert_eq!(calculation.tax_breakdown.len(), 1);
    assert_eq!(calculation.tax_breakdown[0].jurisdiction.country, "US");

    server.await.unwrap();
}

#[tokio::test]
async fn test_retrieve_tax_calculation_mock_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("GET /v1/tax/calculations/taxcalc_retrieve_999 HTTP/1.1"));

        let body = r#"{
            "id": "taxcalc_retrieve_999",
            "object": "tax.calculation",
            "amount_total": 5400,
            "currency": "usd",
            "customer_details": {
                "address": {
                    "country": "US",
                    "state": "NY",
                    "postal_code": "10001"
                },
                "address_source": "shipping"
            },
            "tax_amount_exclusive": 400,
            "tax_amount_inclusive": 0,
            "tax_breakdown": [],
            "status": "active",
            "calculation_mode": "standard"
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_tax").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = CalculationsResource::new(&client);

    let calculation = resource.retrieve("taxcalc_retrieve_999").await.unwrap();
    assert_eq!(calculation.id, "taxcalc_retrieve_999");
    assert_eq!(calculation.amount_total, 5400);
    assert_eq!(calculation.tax_amount_exclusive, 400);

    server.await.unwrap();
}

#[tokio::test]
async fn test_list_tax_calculation_line_items_page_mock_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with(
            "GET /v1/tax/calculations/taxcalc_li_test/line_items?limit=10&starting_after=tax_li_prev HTTP/1.1"
        ));

        let body = r#"{
            "object": "list",
            "data": [
                {
                    "id": "tax_li_001",
                    "object": "tax.calculation_line_item",
                    "amount": 5000,
                    "amount_tax": 400,
                    "reference": "ref_widget_1",
                    "tax_code": "txcd_10000000",
                    "quantity": 1
                },
                {
                    "id": "tax_li_002",
                    "object": "tax.calculation_line_item",
                    "amount": 2500,
                    "amount_tax": 200,
                    "reference": "ref_widget_2",
                    "tax_code": "txcd_10000000",
                    "quantity": 1
                }
            ],
            "has_more": false,
            "url": "/v1/tax/calculations/taxcalc_li_test/line_items"
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_tax").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = CalculationsResource::new(&client);

    let params = ListTaxCalculationLineItemsParams::new()
        .limit(10)
        .starting_after("tax_li_prev");

    let page = resource
        .list_line_items_page("taxcalc_li_test", Some(&params))
        .await
        .unwrap();

    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0].id, "tax_li_001");
    assert_eq!(page.data[0].amount, 5000);
    assert_eq!(page.data[1].id, "tax_li_002");
    assert!(!page.has_more);

    server.await.unwrap();
}

#[tokio::test]
async fn test_list_tax_calculation_line_items_paginator_stream() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // Page 1
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8_lossy(&buf1[..n1]);
        assert!(
            req1.contains("GET /v1/tax/calculations/taxcalc_pag_test/line_items?limit=1 HTTP/1.1")
        );

        let page1 = r#"{
            "object": "list",
            "data": [
                {
                    "id": "tax_li_stream_1",
                    "object": "tax.calculation_line_item",
                    "amount": 1000,
                    "amount_tax": 80,
                    "reference": "item_stream_1"
                }
            ],
            "has_more": true,
            "url": "/v1/tax/calculations/taxcalc_pag_test/line_items"
        }"#;
        let resp1 = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            page1.len(),
            page1
        );
        socket1.write_all(resp1.as_bytes()).await.unwrap();

        // Page 2
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8_lossy(&buf2[..n2]);
        assert!(req2.contains("starting_after=tax_li_stream_1"));

        let page2 = r#"{
            "object": "list",
            "data": [
                {
                    "id": "tax_li_stream_2",
                    "object": "tax.calculation_line_item",
                    "amount": 2000,
                    "amount_tax": 160,
                    "reference": "item_stream_2"
                }
            ],
            "has_more": false,
            "url": "/v1/tax/calculations/taxcalc_pag_test/line_items"
        }"#;
        let resp2 = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            page2.len(),
            page2
        );
        socket2.write_all(resp2.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_tax").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = CalculationsResource::new(&client);

    let params = ListTaxCalculationLineItemsParams::new().limit(1);
    let mut paginator = resource.list_line_items("taxcalc_pag_test", &params);

    let item1: TaxCalculationLineItem = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Item 1 missing");
    assert_eq!(item1.id, "tax_li_stream_1");
    assert_eq!(item1.amount, 1000);

    let item2 = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Item 2 missing");
    assert_eq!(item2.id, "tax_li_stream_2");
    assert_eq!(item2.amount, 2000);

    let item3 = paginator.next_item().await.unwrap();
    assert!(item3.is_none());

    server.await.unwrap();
}

#[test]
fn test_gate_g_borrow_zero_clone_discipline() {
    let calculation = TaxCalculation {
        id: "taxcalc_borrow_test".into(),
        object: "tax.calculation".into(),
        amount_total: 10000,
        currency: "usd".into(),
        customer: None,
        customer_details: CustomerTaxDetails::new(),
        expires_at: Some(1730000000),
        line_items: None,
        livemode: false,
        shipping_cost: None,
        tax_amount_exclusive: 800,
        tax_amount_inclusive: 0,
        tax_breakdown: vec![TaxBreakdown {
            amount: 800,
            inclusive: false,
            jurisdiction: TaxJurisdiction {
                country: "US".into(),
                state: Some("CA".into()),
                level: "state".into(),
                display_name: "California".into(),
                ..Default::default()
            },
            tax_rate_details: Some(TaxRateDetails {
                country: Some("US".into()),
                state: Some("CA".into()),
                percentage_decimal: "8.0".into(),
                tax_type: Some("sales_tax".into()),
                rate_type: Some("percentage".into()),
                display_name: Some("CA State Tax".into()),
                taxability_reason: Some("standard_rated".into()),
            }),
            taxability_reason: Some("standard_rated".into()),
            taxable_amount: Some(10000),
            sourcing: Some("destination".into()),
        }],
        tax_date: Some(1728345600),
        status: TaxCalculationStatus::Active,
        calculation_mode: TaxCalculationMode::Standard,
    };

    // Zero clone borrowing accessors
    let id_ref: &str = calculation.id();
    assert_eq!(id_ref, "taxcalc_borrow_test");

    let breakdown_slice: &[TaxBreakdown] = calculation.tax_breakdown();
    assert_eq!(breakdown_slice.len(), 1);
    assert_eq!(breakdown_slice[0].amount, 800);

    assert!(!calculation.is_expired(1728000000));
    assert!(calculation.is_expired(1731000000));
    assert_eq!(calculation.total_tax(), 800);
}
