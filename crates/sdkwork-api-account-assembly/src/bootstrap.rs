//! Gateway bootstrap for sdkwork-account.
//! Multi-surface merges mount shared infrastructure routes once at the assembly layer
//! so `/healthz`, `/livez`, `/readyz`, and `/metrics` are not duplicated per surface.
//!
//! The assembly exports the indivisible `ApiAssemblyContribution` contract
//! (API_ASSEMBLY_SPEC.md section 4); the platform cloud gateway composes the
//! contribution with its process-shared PostgreSQL pool.

use axum::Router;
use sdkwork_account_service_host::AccountServiceHost;
use sdkwork_database_sqlx::DatabasePool;
pub use sdkwork_web_bootstrap::ApiAssemblyContribution;
use sdkwork_web_bootstrap::{ReadinessCheck, ReadinessFuture, WebModule};
use sdkwork_web_core::HttpRouteManifest;
use std::sync::Arc;

/// Indivisible host-neutral API assembly contribution (web-bootstrap contract).
pub type ApiAssembly = ApiAssemblyContribution;

fn combined_route_manifest() -> HttpRouteManifest {
    let manifests = [
        sdkwork_routes_account_app_api::gateway_route_manifest(),
        sdkwork_routes_account_backend_api::gateway_route_manifest(),
    ];
    HttpRouteManifest::from_owned_routes(
        manifests
            .into_iter()
            .flat_map(|manifest| manifest.routes().to_vec())
            .collect(),
    )
}

fn contribution_from(
    router: Router,
    readiness_check: Arc<dyn ReadinessCheck>,
) -> Result<ApiAssembly, String> {
    ApiAssemblyContribution::from_manifest(
        "sdkwork-account",
        "SDKWork Account API",
        router,
        combined_route_manifest(),
        Vec::new(),
        readiness_check,
    )
}

pub async fn assemble_api_router(host: Arc<AccountServiceHost>) -> ApiAssembly {
    let mut router = Router::new();
    router = router.merge(sdkwork_routes_account_app_api::build_account_app_router(
        host.clone(),
    ));
    router = router
        .merge(sdkwork_routes_account_backend_api::build_account_backend_router(host.clone()));
    contribution_from(
        router,
        Arc::new(AccountReadiness {
            pool: host.database_pool().clone(),
        }),
    )
    .expect("account contribution contract is valid")
}

pub async fn assemble_api_router_from_env() -> Result<ApiAssembly, String> {
    let host = Arc::new(AccountServiceHost::from_env().await?);
    Ok(assemble_api_router(host).await)
}

#[derive(Clone)]
struct AccountReadiness {
    pool: DatabasePool,
}

impl ReadinessCheck for AccountReadiness {
    fn check(&self) -> ReadinessFuture<'_> {
        let pool = self.pool.clone();
        Box::pin(async move {
            match pool.test_connection().await {
                Ok(true) => Ok(()),
                Ok(false) => Err("account database readiness query returned no row".to_owned()),
                Err(error) => Err(format!("account database readiness check failed: {error}")),
            }
        })
    }
}

/// Host-neutral App API contribution for embedding the account surface into a
/// composed gateway (same pattern as order/membership/notary).
pub async fn assemble_app_api_contribution_from_env() -> Result<ApiAssemblyContribution, String> {
    let host = Arc::new(AccountServiceHost::from_env().await?);
    Ok(assemble_app_api_contribution(host).await)
}

/// Same-origin dependency composition: build the account App API contribution
/// on a shared pool owned by the consuming host.
pub async fn assemble_app_api_contribution_with_pool(
    pool: &DatabasePool,
) -> Result<ApiAssemblyContribution, String> {
    let host = Arc::new(AccountServiceHost::from_pool(pool).await?);
    Ok(assemble_app_api_contribution(host).await)
}

pub async fn assemble_app_api_contribution(
    host: Arc<AccountServiceHost>,
) -> ApiAssemblyContribution {
    let router = sdkwork_routes_account_app_api::build_account_app_router(host.clone());
    ApiAssemblyContribution::from_manifest(
        "sdkwork-account",
        "SDKWork Account API",
        router,
        sdkwork_routes_account_app_api::gateway_route_manifest(),
        Vec::new(),
        Arc::new(AccountReadiness {
            pool: host.database_pool().clone(),
        }),
    )
    .expect("account App API contribution should build")
}

/// Assemble the Account contribution against a caller-provided database pool so
/// the platform cloud gateway can share its process-wide PostgreSQL pool.
pub async fn assemble_api_router_with_pool(pool: DatabasePool) -> Result<ApiAssembly, String> {
    let host = Arc::new(AccountServiceHost::from_pool(&pool).await?);
    Ok(assemble_api_router(host).await)
}

/// Canonical Web Module definition for this application
/// (API_ASSEMBLY_SPEC §4.1.1): the complete HTTP surface — every route,
/// manifest, and OpenAPI document of this owner — as one installable module.
pub async fn web_module() -> Result<WebModule, String> {
    Ok(WebModule::from_contribution(
        assemble_api_router_from_env().await?,
    ))
}

/// Same as [`web_module`] but composed on a process-shared database pool
/// (platform gateways, API_ASSEMBLY_SPEC §4.1.1).
pub async fn web_module_with_pool(pool: DatabasePool) -> Result<WebModule, String> {
    Ok(WebModule::from_contribution(
        assemble_api_router_with_pool(pool).await?,
    ))
}
