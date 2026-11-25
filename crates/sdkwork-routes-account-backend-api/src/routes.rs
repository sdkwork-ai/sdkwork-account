use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use sdkwork_account_repository_sqlx::PostgresCommerceAccountStore;
use sdkwork_account_service_host::AccountServiceHost;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_utils_rust::{SdkWorkApiResponse, SdkWorkResourceData};
use serde::Serialize;
use std::sync::Arc;

use crate::web_bootstrap::wrap_router_with_web_framework_from_env;
use crate::{
    backend_billing_policy_router_with_postgres_pool, backend_hold_router_with_postgres_pool,
    backend_outbox_router_with_postgres_pool, backend_wallet_read_router_with_postgres_pool,
    backend_wallet_router_with_postgres_pool,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WalletHealthItemResponse {
    status: String,
    database: String,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    outbox_pending_lag: i64,
}

async fn wallet_health(
    axum::extract::State(host): axum::extract::State<Arc<AccountServiceHost>>,
) -> (
    StatusCode,
    Json<SdkWorkApiResponse<SdkWorkResourceData<WalletHealthItemResponse>>>,
) {
    // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
    let DatabasePool::Postgres(pool, _) = host.database_pool() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(SdkWorkApiResponse::success(
                SdkWorkResourceData {
                    item: WalletHealthItemResponse {
                        status: "degraded".to_owned(),
                        database: "down".to_owned(),
                        outbox_pending_lag: -1,
                    },
                },
                sdkwork_utils_rust::uuid(),
            )),
        );
    };
    let db_ok = sqlx::query("SELECT 1").execute(pool).await.is_ok();
    let outbox_pending_lag = PostgresCommerceAccountStore::new(pool.clone())
        .pending_outbox_lag()
        .await
        .unwrap_or(-1);

    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let payload = SdkWorkResourceData {
        item: WalletHealthItemResponse {
            status: if db_ok { "ready" } else { "degraded" }.to_owned(),
            database: if db_ok { "up" } else { "down" }.to_owned(),
            outbox_pending_lag,
        },
    };
    (
        status,
        Json(SdkWorkApiResponse::success(
            payload,
            sdkwork_utils_rust::uuid(),
        )),
    )
}

pub fn build_account_backend_router(host: Arc<AccountServiceHost>) -> Router {
    let mut router = Router::new().route(
        "/backend/v3/api/wallet/health",
        get(wallet_health).with_state(host.clone()),
    );

    // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
    let DatabasePool::Postgres(pool, _) = host.database_pool() else {
        panic!("account backend router requires a PostgreSQL database pool");
    };
    let pool = pool.clone();
    router = router.merge(
        backend_wallet_router_with_postgres_pool(pool.clone())
            .merge(backend_hold_router_with_postgres_pool(pool.clone()))
            .merge(backend_billing_policy_router_with_postgres_pool(
                pool.clone(),
            ))
            .merge(backend_wallet_read_router_with_postgres_pool(pool.clone()))
            .merge(backend_outbox_router_with_postgres_pool(pool)),
    );

    router
}

pub async fn build_account_backend_router_with_framework(host: Arc<AccountServiceHost>) -> Router {
    wrap_router_with_web_framework_from_env(build_account_backend_router(host)).await
}
