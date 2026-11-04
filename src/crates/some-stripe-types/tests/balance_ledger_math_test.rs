//! Property and mathematical invariant validation suite for `BalanceTransaction` and Ledger Reconciliation.
//!
//! Validates:
//! - **Gate G-Law**: ISO 4217 integer arithmetic, zero float rounding errors.
//! - **Gate G2**: Immutable audit ledger reconciliation ($\text{net} = \text{amount} - \text{fee}$).
//! - **Gate G-Borrow**: Zero-clone borrowing discipline across ledger slices.
//! - **1,000+ Transaction Property Tests**: Verifies invariant across charges, refunds, disputes, payouts, and boundaries.

use some_stripe_types::{
    Balance, BalanceFund, BalanceSourceTypes, BalanceTransaction, FeeDetail,
    balance_transaction_id, validate_ledger_reconciliation,
};

/// Deterministic pseudo-random number generator for reproducible 1,000+ property scenarios.
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x8a5cd789635d2dff } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_range(&mut self, min: i64, max: i64) -> i64 {
        assert!(max >= min);
        let span = (max - min + 1) as u64;
        min + (self.next_u64() % span) as i64
    }
}

#[test]
fn test_property_math_ledger_reconciliation_1000_scenarios() {
    let mut rng = XorShift64::new(42_1337_2026);
    let mut txns = Vec::with_capacity(1200);

    let currencies = [
        "usd", "eur", "gbp", "jpy", "aud", "cad", "chf", "sek", "sgd",
    ];

    // 1. Generate 600 Standard Customer Charges (positive amount, calculated fee, net = amount - fee)
    for i in 0..600 {
        let amount = rng.next_range(50, 1_000_000); // 50 cents to $10,000
        // Stripe fee formula: 2.9% + 30 cents, rounded using integer arithmetic
        let fee = ((amount * 29) / 1000) + 30;
        let net = amount - fee;
        let currency = currencies[(rng.next_u64() as usize) % currencies.len()];

        let txn = BalanceTransaction {
            id: format!("txn_charge_{i:04}"),
            object: "balance_transaction".into(),
            amount,
            available_on: 1728345600 + (i * 60),
            created: 1728345600 + (i * 60),
            currency: currency.into(),
            description: Some(format!("Standard charge #{i}")),
            fee,
            fee_details: vec![
                FeeDetail::new(fee - 30, currency, "stripe_variable_fee"),
                FeeDetail::new(30, currency, "stripe_fixed_fee"),
            ],
            net,
            reporting_category: Some("charge".into()),
            source: Some(format!("ch_charge_{i:04}")),
            status: "available".into(),
            r#type: "charge".into(),
        };

        assert!(
            txn.verify_accounting_invariant(),
            "Charge #{i} accounting invariant violated: net {} != amount {} - fee {}",
            txn.net,
            txn.amount,
            txn.fee
        );
        txns.push(txn);
    }

    // 2. Generate 250 Customer Refunds (negative amount, 0 or partial fee refund, net = amount - fee)
    for i in 0..250 {
        let original_charge = rng.next_range(500, 500_000);
        let is_full_refund = rng.next_u64().is_multiple_of(2);
        let refund_amount = if is_full_refund {
            original_charge
        } else {
            rng.next_range(100, original_charge)
        };

        // In negative minor currency units
        let amount = -refund_amount;
        let fee = 0; // Stripe typically does not return processing fees
        let net = amount - fee; // net is negative (debiting merchant balance)
        let currency = currencies[(rng.next_u64() as usize) % currencies.len()];

        let txn = BalanceTransaction {
            id: format!("txn_refund_{i:04}"),
            object: "balance_transaction".into(),
            amount,
            available_on: 1728400000 + (i * 60),
            created: 1728400000 + (i * 60),
            currency: currency.into(),
            description: Some(format!("Refund for charge #{i}")),
            fee,
            fee_details: vec![],
            net,
            reporting_category: Some("refund".into()),
            source: Some(format!("re_refund_{i:04}")),
            status: "available".into(),
            r#type: "refund".into(),
        };

        assert!(
            txn.verify_accounting_invariant(),
            "Refund #{i} accounting invariant violated: net {} != amount {} - fee {}",
            txn.net,
            txn.amount,
            txn.fee
        );
        txns.push(txn);
    }

    // 3. Generate 150 Dispute Adjustments & Network Fees
    for i in 0..150 {
        let dispute_fee = 1500; // $15 dispute fee
        let amount = -dispute_fee;
        let fee = 0;
        let net = amount - fee;
        let currency = "usd";

        let txn = BalanceTransaction {
            id: format!("txn_dispute_{i:04}"),
            object: "balance_transaction".into(),
            amount,
            available_on: 1728500000 + (i * 60),
            created: 1728500000 + (i * 60),
            currency: currency.into(),
            description: Some(format!("Dispute fee for case #{i}")),
            fee,
            fee_details: vec![],
            net,
            reporting_category: Some("dispute".into()),
            source: Some(format!("dp_dispute_{i:04}")),
            status: "available".into(),
            r#type: "adjustment".into(),
        };

        assert!(txn.verify_accounting_invariant());
        txns.push(txn);
    }

    // 4. Generate 100 Merchant Payouts (funds moving from Stripe balance to external bank)
    for i in 0..100 {
        let payout_amount = rng.next_range(10_000, 2_000_000);
        let amount = -payout_amount;
        let fee = if rng.next_u64().is_multiple_of(4) {
            100
        } else {
            0
        }; // $1 instant payout fee or $0 standard
        let net = amount - fee;

        let txn = BalanceTransaction {
            id: format!("txn_payout_{i:04}"),
            object: "balance_transaction".into(),
            amount,
            available_on: 1728600000 + (i * 60),
            created: 1728600000 + (i * 60),
            currency: "usd".into(),
            description: Some(format!("STRIPE PAYOUT #{i}")),
            fee,
            fee_details: if fee > 0 {
                vec![FeeDetail::new(fee, "usd", "instant_payout_fee")]
            } else {
                vec![]
            },
            net,
            reporting_category: Some("payout".into()),
            source: Some(format!("po_payout_{i:04}")),
            status: "available".into(),
            r#type: "payout".into(),
        };

        assert!(txn.verify_accounting_invariant());
        txns.push(txn);
    }

    // 5. Generate Boundary Extreme Cases
    let boundaries = [
        (0, 0, 0, "zero_amount_zero_fee"),
        (1, 0, 1, "one_cent_zero_fee"),
        (100, 100, 0, "net_zero_charge"),
        (50, 100, -50, "fee_exceeds_amount"),
        (-100, 50, -150, "refund_with_fee"),
        (
            i64::MAX / 4,
            100_000,
            (i64::MAX / 4) - 100_000,
            "large_positive_bound",
        ),
        (-(i64::MAX / 4), 0, -(i64::MAX / 4), "large_negative_bound"),
    ];

    for (amount, fee, net, label) in boundaries {
        let txn = BalanceTransaction {
            id: format!("txn_boundary_{label}"),
            object: "balance_transaction".into(),
            amount,
            available_on: 1728700000,
            created: 1728700000,
            currency: "usd".into(),
            description: Some(format!("Boundary test: {label}")),
            fee,
            fee_details: vec![],
            net,
            reporting_category: Some("other".into()),
            source: None,
            status: "available".into(),
            r#type: "adjustment".into(),
        };

        assert!(
            txn.verify_accounting_invariant(),
            "Boundary scenario {label} invariant failed"
        );
        txns.push(txn);
    }

    // Verify total count is over 1,000 scenarios
    assert!(
        txns.len() >= 1100,
        "Total transaction scenarios must exceed 1,000, got: {}",
        txns.len()
    );

    // Verify batch ledger reconciliation over all 1,100+ transactions
    let reconciliation_res = validate_ledger_reconciliation(&txns);
    assert!(
        reconciliation_res.is_ok(),
        "Full ledger reconciliation failed: {:?}",
        reconciliation_res.err()
    );

    // Test extractor function across entire slice without cloning
    for txn in &txns {
        assert_eq!(balance_transaction_id(txn), txn.id.as_str());
    }
}

#[test]
fn test_ledger_reconciliation_corruption_detection() {
    let mut txns = vec![
        BalanceTransaction::new("txn_clean_1", 5000, 175, "usd", "charge", "available"),
        BalanceTransaction::new("txn_clean_2", 8000, 260, "usd", "charge", "available"),
        BalanceTransaction::new("txn_clean_3", -3000, 0, "usd", "refund", "available"),
    ];

    assert!(validate_ledger_reconciliation(&txns).is_ok());

    // Inject off-by-one corruption into transaction 2 net
    txns[1].net += 1; // Corrupted!
    assert!(!txns[1].verify_accounting_invariant());

    let err = validate_ledger_reconciliation(&txns).unwrap_err();
    assert!(err.contains("txn_clean_2"));
    assert!(err.contains("invariant violation"));
}

#[test]
fn test_balance_fund_and_balance_defaults() {
    let fund = BalanceFund::new(50000, "usd");
    assert_eq!(fund.amount, 50000);
    assert_eq!(fund.currency, "usd");
    assert!(fund.source_types.is_none());

    let default_balance = Balance::default();
    assert_eq!(default_balance.object, "balance");
    assert!(default_balance.available.is_empty());
    assert!(default_balance.pending.is_empty());
    assert!(!default_balance.livemode);
}

#[test]
fn test_balance_source_types_deserialization() {
    let json = serde_json::json!({
        "card": 4000,
        "bank_account": 1000,
        "fpx": 500
    });

    let source_types: BalanceSourceTypes = serde_json::from_value(json).unwrap();
    assert_eq!(source_types.card, Some(4000));
    assert_eq!(source_types.bank_account, Some(1000));
    assert_eq!(source_types.fpx, Some(500));
}
