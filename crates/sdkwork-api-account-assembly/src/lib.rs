//! API assembly for sdkwork-account.
//! Application bootstrap lives in `bootstrap.rs`; route inventory is in `assembly-manifest.json`.
//! SDKWORK-ASSEMBLY-LIB-CUSTOM: exports beyond the canonical materializer template.

mod bootstrap;
mod generated;

pub use bootstrap::{
    assemble_api_router, assemble_api_router_from_env, assemble_api_router_with_pool,
    assemble_app_api_contribution, assemble_app_api_contribution_from_env,
    assemble_app_api_contribution_with_pool, web_module, web_module_with_pool, ApiAssembly,
    ApiAssemblyContribution,
};

/// Account app-surface route inventory for host applications that compose the
/// account contribution into their own app surface (API_ASSEMBLY_SPEC §3/§6.1:
/// dependency manifests enter through the dependency assembly entrypoint, not
/// through direct `sdkwork-routes-*` imports).
pub fn app_api_route_manifest() -> sdkwork_web_core::HttpRouteManifest {
    sdkwork_routes_account_app_api::gateway_route_manifest()
}

pub fn assembly_route_count() -> usize {
    generated::ROUTE_CRATE_COUNT
}
