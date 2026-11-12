//! Master router aggregating all service domain sub-routers.

use axum::Router;
use crate::state::AppState;

pub mod billing;
pub mod checkout;
pub mod connect;
pub mod payments;
pub mod specialized;
pub mod v2_services;
pub mod webhooks;

/// Builds the comprehensive application router covering all 8 Stripe SDK modules.
pub fn build_app_router() -> Router<AppState> {
    Router::new()
        .merge(webhooks::router())
        .merge(payments::router())
        .merge(checkout::router())
        .merge(billing::router())
        .merge(connect::router())
        .merge(specialized::router())
        .merge(v2_services::router())
}
