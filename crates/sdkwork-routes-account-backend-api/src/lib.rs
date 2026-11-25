use axum::Router;
use sdkwork_account_service_host::AccountServiceHost;
use std::sync::Arc;

pub mod api_response;
pub mod billing_policy_router;
pub mod hold_router;
pub mod http_route_manifest;
pub mod outbox_router;
pub mod routes;
pub mod subject;
pub mod wallet_read_router;
pub mod wallet_router;
pub mod web_bootstrap;

pub use billing_policy_router::{
    backend_billing_policy_router_with_postgres_pool, build_backend_billing_policy_router,
    CommerceBillingPolicyFuture, CommerceBillingPolicyStore,
};
pub use hold_router::{
    backend_hold_router_with_postgres_pool, build_backend_hold_router,
    CommerceAccountHoldWriteStore, CommerceHoldWriteFuture,
};
pub use http_route_manifest::gateway_route_manifest;
pub use outbox_router::{
    backend_outbox_router_with_postgres_pool, build_backend_outbox_router,
    CommerceOutboxRelayFuture, CommerceOutboxRelayStore,
};
pub use routes::build_account_backend_router;
pub use routes::build_account_backend_router_with_framework;
pub use wallet_read_router::{
    backend_wallet_read_router_with_postgres_pool, build_backend_wallet_read_router,
    CommerceWalletReadFuture, CommerceWalletReadStore,
};
pub use wallet_router::{
    backend_wallet_router_with_postgres_pool, build_backend_wallet_router,
    CommerceAccountLedgerWriteStore, CommerceLedgerWriteFuture,
};

pub async fn gateway_mount(host: Arc<AccountServiceHost>) -> Router {
    build_account_backend_router_with_framework(host).await
}
