//! Backend read surface of the account centre.
//!
//! Balance, ledger and allowance lots are account data, so the operator views are
//! served by the account domain under the `wallet` surface. Every query is scoped
//! by the authenticated tenant; the caller picks the wallet owner (personal user or
//! team organization) and this router never widens that scope.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Query, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use sdkwork_account_repository_sqlx::PostgresCommerceAccountStore;
use sdkwork_account_service::{
    PointsLotItem, PointsLotListQuery, StoreListPage, WalletAccountItem, WalletAccountListQuery,
    WalletTransactionItem, WalletTransactionListQuery,
};
use sdkwork_contract_service::{CommerceAccountAssetType, CommerceServiceError};
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_web_core::WebRequestContext;

use crate::api_response::{map_service_error, success_item, unauthorized, validation};
use crate::subject::backend_runtime_subject_from_extension;

pub type CommerceWalletReadFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

/// Read boundary for the account centre, so tests can substitute a fake.
pub trait CommerceWalletReadStore: Send + Sync {
    fn list_wallet_accounts<'a>(
        &'a self,
        query: WalletAccountListQuery,
    ) -> CommerceWalletReadFuture<'a, Vec<WalletAccountItem>>;

    fn list_wallet_transactions<'a>(
        &'a self,
        query: WalletTransactionListQuery,
    ) -> CommerceWalletReadFuture<'a, StoreListPage<WalletTransactionItem>>;

    fn list_points_lots<'a>(
        &'a self,
        query: PointsLotListQuery,
    ) -> CommerceWalletReadFuture<'a, StoreListPage<PointsLotItem>>;
}

impl CommerceWalletReadStore for PostgresCommerceAccountStore {
    fn list_wallet_accounts<'a>(
        &'a self,
        query: WalletAccountListQuery,
    ) -> CommerceWalletReadFuture<'a, Vec<WalletAccountItem>> {
        Box::pin(
            async move { PostgresCommerceAccountStore::list_wallet_accounts(self, query).await },
        )
    }

    fn list_wallet_transactions<'a>(
        &'a self,
        query: WalletTransactionListQuery,
    ) -> CommerceWalletReadFuture<'a, StoreListPage<WalletTransactionItem>> {
        Box::pin(async move {
            PostgresCommerceAccountStore::list_wallet_transactions(self, query).await
        })
    }

    fn list_points_lots<'a>(
        &'a self,
        query: PointsLotListQuery,
    ) -> CommerceWalletReadFuture<'a, StoreListPage<PointsLotItem>> {
        Box::pin(async move { PostgresCommerceAccountStore::list_points_lots(self, query).await })
    }
}

#[derive(Clone)]
struct BackendWalletReadState {
    store: Arc<dyn CommerceWalletReadStore>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletAccountsQueryRequest {
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default)]
    owner_user_id: Option<String>,
    #[serde(default)]
    owner_type: Option<String>,
    #[serde(default)]
    asset_type: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletTransactionsQueryRequest {
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default)]
    owner_user_id: Option<String>,
    #[serde(default)]
    owner_type: Option<String>,
    #[serde(default)]
    asset_type: Option<String>,
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    page: Option<i64>,
    /// The GET wire uses snake_case for paging, per the paging standard.
    #[serde(default, rename = "page_size")]
    page_size: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletPointsLotsQueryRequest {
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default)]
    owner_user_id: Option<String>,
    #[serde(default)]
    page: Option<i64>,
    #[serde(default, rename = "page_size")]
    page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WalletPageInfoResponse {
    mode: String,
    page: i64,
    page_size: i64,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    total_items: i64,
    has_more: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WalletAccountResponse {
    id: String,
    uuid: String,
    tenant_id: String,
    organization_id: Option<String>,
    owner_user_id: String,
    asset_type: String,
    currency_code: Option<String>,
    available_amount: String,
    frozen_amount: String,
    pending_amount: String,
    status: String,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WalletAccountsResponse {
    items: Vec<WalletAccountResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WalletTransactionResponse {
    id: String,
    uuid: String,
    account_id: String,
    tenant_id: String,
    organization_id: Option<String>,
    owner_user_id: String,
    asset_type: String,
    direction: String,
    amount: String,
    balance_before: String,
    balance_after: String,
    business_type: String,
    transaction_no: String,
    request_no: String,
    idempotency_key: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WalletTransactionsResponse {
    items: Vec<WalletTransactionResponse>,
    page_info: WalletPageInfoResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WalletPointsLotResponse {
    id: String,
    uuid: String,
    account_id: String,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    granted_amount: i64,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    remaining_amount: i64,
    source_type: String,
    source_id: String,
    expires_at: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WalletPointsLotsResponse {
    items: Vec<WalletPointsLotResponse>,
    page_info: WalletPageInfoResponse,
}

fn web_context(context: Option<Extension<WebRequestContext>>) -> Option<WebRequestContext> {
    context.map(|Extension(context)| context)
}

fn parse_asset_type(value: Option<&str>) -> Result<Option<CommerceAccountAssetType>, String> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some("cash") => Ok(Some(CommerceAccountAssetType::Cash)),
        Some("points") => Ok(Some(CommerceAccountAssetType::Points)),
        Some("token_bank") => Ok(Some(CommerceAccountAssetType::TokenBank)),
        Some(other) => Err(format!("unknown assetType {other:?}")),
    }
}

fn page_info(
    page: i64,
    page_size: i64,
    total_items: i64,
    has_more: bool,
) -> WalletPageInfoResponse {
    WalletPageInfoResponse {
        mode: "offset".to_owned(),
        page,
        page_size,
        total_items: total_items.max(0),
        has_more,
    }
}

fn paging(page: Option<i64>, page_size: Option<i64>) -> (i64, i64) {
    (
        page.unwrap_or(1).max(1),
        page_size.unwrap_or(50).clamp(1, 200),
    )
}

async fn list_wallet_accounts(
    State(state): State<BackendWalletReadState>,
    context: Option<Extension<IamAppContext>>,
    web: Option<Extension<WebRequestContext>>,
    Query(query): Query<WalletAccountsQueryRequest>,
) -> Response {
    let web = web_context(web);
    let Some(Extension(context)) = context else {
        return unauthorized(web.as_ref(), "authenticated runtime context is required");
    };
    let subject = match backend_runtime_subject_from_extension(Some(Extension(context.clone()))) {
        Ok(subject) => subject,
        Err(detail) => return unauthorized(web.as_ref(), detail),
    };
    let asset_type = match parse_asset_type(query.asset_type.as_deref()) {
        Ok(value) => value,
        Err(detail) => return validation(web.as_ref(), detail),
    };
    let owner_user_id = match query.owner_user_id.as_deref() {
        Some(value) if !value.trim().is_empty() => value.trim().to_owned(),
        _ => context.user_id.trim().to_owned(),
    };
    let listed = state
        .store
        .list_wallet_accounts(WalletAccountListQuery {
            asset_type,
            organization_id: query.organization_id,
            owner_user_id,
            tenant_id: subject.tenant_id.clone(),
            owner_type: query.owner_type,
        })
        .await;
    match listed {
        Ok(items) => success_item(
            web.as_ref(),
            WalletAccountsResponse {
                items: items.into_iter().map(map_account).collect(),
            },
        ),
        Err(error) => map_service_error(web.as_ref(), error),
    }
}

async fn list_wallet_transactions(
    State(state): State<BackendWalletReadState>,
    context: Option<Extension<IamAppContext>>,
    web: Option<Extension<WebRequestContext>>,
    Query(query): Query<WalletTransactionsQueryRequest>,
) -> Response {
    let web = web_context(web);
    let Some(Extension(context)) = context else {
        return unauthorized(web.as_ref(), "authenticated runtime context is required");
    };
    let subject = match backend_runtime_subject_from_extension(Some(Extension(context.clone()))) {
        Ok(subject) => subject,
        Err(detail) => return unauthorized(web.as_ref(), detail),
    };
    let asset_type = match parse_asset_type(query.asset_type.as_deref()) {
        Ok(value) => value,
        Err(detail) => return validation(web.as_ref(), detail),
    };
    let (page, page_size) = paging(query.page, query.page_size);
    let owner_user_id = match query.owner_user_id.as_deref() {
        Some(value) if !value.trim().is_empty() => value.trim().to_owned(),
        _ => context.user_id.trim().to_owned(),
    };
    let listed = state
        .store
        .list_wallet_transactions(WalletTransactionListQuery {
            account_id: query.account_id,
            asset_type,
            cursor: query.cursor,
            organization_id: query.organization_id,
            owner_user_id,
            page: Some(page),
            page_size: Some(page_size),
            tenant_id: subject.tenant_id.clone(),
            owner_type: query.owner_type,
        })
        .await;
    match listed {
        Ok(result) => success_item(
            web.as_ref(),
            WalletTransactionsResponse {
                items: result.items.into_iter().map(map_transaction).collect(),
                page_info: page_info(page, page_size, result.total_items, result.has_more),
            },
        ),
        Err(error) => map_service_error(web.as_ref(), error),
    }
}

async fn list_wallet_points_lots(
    State(state): State<BackendWalletReadState>,
    context: Option<Extension<IamAppContext>>,
    web: Option<Extension<WebRequestContext>>,
    Query(query): Query<WalletPointsLotsQueryRequest>,
) -> Response {
    let web = web_context(web);
    let Some(Extension(context)) = context else {
        return unauthorized(web.as_ref(), "authenticated runtime context is required");
    };
    let subject = match backend_runtime_subject_from_extension(Some(Extension(context.clone()))) {
        Ok(subject) => subject,
        Err(detail) => return unauthorized(web.as_ref(), detail),
    };
    let (page, page_size) = paging(query.page, query.page_size);
    let owner_user_id = match query.owner_user_id.as_deref() {
        Some(value) if !value.trim().is_empty() => value.trim().to_owned(),
        _ => context.user_id.trim().to_owned(),
    };
    let listed = state
        .store
        .list_points_lots(PointsLotListQuery {
            organization_id: query.organization_id,
            owner_user_id,
            page: Some(page),
            page_size: Some(page_size),
            tenant_id: subject.tenant_id.clone(),
        })
        .await;
    match listed {
        Ok(result) => success_item(
            web.as_ref(),
            WalletPointsLotsResponse {
                items: result.items.into_iter().map(map_points_lot).collect(),
                page_info: page_info(page, page_size, result.total_items, result.has_more),
            },
        ),
        Err(error) => map_service_error(web.as_ref(), error),
    }
}

fn map_account(item: WalletAccountItem) -> WalletAccountResponse {
    WalletAccountResponse {
        id: item.id,
        uuid: item.uuid,
        tenant_id: item.tenant_id,
        organization_id: item.organization_id,
        owner_user_id: item.owner_user_id,
        asset_type: item.asset_type.as_str().to_owned(),
        currency_code: item.currency_code,
        available_amount: item.available_amount.as_str().to_owned(),
        frozen_amount: item.frozen_amount.as_str().to_owned(),
        pending_amount: item.pending_amount.as_str().to_owned(),
        status: item.status,
        version: item.version,
    }
}

fn map_transaction(item: WalletTransactionItem) -> WalletTransactionResponse {
    WalletTransactionResponse {
        id: item.id,
        uuid: item.uuid,
        account_id: item.account_id,
        tenant_id: item.tenant_id,
        organization_id: item.organization_id,
        owner_user_id: item.owner_user_id,
        asset_type: item.asset_type.as_str().to_owned(),
        direction: item.direction.as_str().to_owned(),
        amount: item.amount.as_str().to_owned(),
        balance_before: item.balance_before.as_str().to_owned(),
        balance_after: item.balance_after.as_str().to_owned(),
        business_type: item.business_type,
        transaction_no: item.transaction_no,
        request_no: item.request_no,
        idempotency_key: item.idempotency_key,
        created_at: item.created_at,
    }
}

fn map_points_lot(item: PointsLotItem) -> WalletPointsLotResponse {
    WalletPointsLotResponse {
        id: item.id,
        uuid: item.uuid,
        account_id: item.account_id,
        granted_amount: item.granted_amount,
        remaining_amount: item.remaining_amount,
        source_type: item.source_type,
        source_id: item.source_id,
        expires_at: item.expires_at,
        status: item.status,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

pub fn backend_wallet_read_router_with_postgres_pool(pool: PgPool) -> Router {
    build_backend_wallet_read_router(Arc::new(PostgresCommerceAccountStore::new(pool)))
}

pub fn build_backend_wallet_read_router(store: Arc<dyn CommerceWalletReadStore>) -> Router {
    Router::new()
        .route("/backend/v3/api/wallet/accounts", get(list_wallet_accounts))
        .route(
            "/backend/v3/api/wallet/ledger_entries",
            get(list_wallet_transactions),
        )
        .route(
            "/backend/v3/api/wallet/points_lots",
            get(list_wallet_points_lots),
        )
        .with_state(BackendWalletReadState { store })
}
