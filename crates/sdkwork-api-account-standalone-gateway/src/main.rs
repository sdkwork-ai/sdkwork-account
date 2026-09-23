//! Account API server entrypoint.
//!
//! Production bootstrap:
//! - CORS restricted to `SDKWORK_CORS_ALLOWED_ORIGINS` (fail-closed when unset).
//! - Readiness reflects database health via `SELECT 1`.
//! - Graceful shutdown drains in-flight requests on SIGINT / SIGTERM.

use sdkwork_api_account_assembly::assemble_api_router_from_env;
use sdkwork_iam_web_adapter::{
    build_web_framework_builder, iam_web_request_context_resolver_from_env,
};
use sdkwork_web_bootstrap::{infra_public_path_prefixes, ApiModuleRegistry};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let assembly = match assemble_api_router_from_env().await {
        Ok(assembly) => assembly,
        Err(error) => {
            tracing::error!(target = "account.bootstrap", error = %error, "account API assembly bootstrap failed");
            return Err(error.into());
        }
    };
    let framework = build_web_framework_builder(
        iam_web_request_context_resolver_from_env().await,
        assembly.route_manifest.clone(),
        infra_public_path_prefixes(),
    );
    let mut module_registry = ApiModuleRegistry::new();
    module_registry.add_modules(vec![assembly]);
    let app = module_registry
        .try_compose("SDKWork Account API")
        .map_err(std::io::Error::other)?
        .into_hosted(framework)
        .router
        .layer(TraceLayer::new_for_http())
        .layer(sdkwork_web_bootstrap::application_cors_layer_from_env(
            &["SDKWORK_ACCOUNT_ENVIRONMENT", "ACCOUNT_ENVIRONMENT"],
            &["SDKWORK_CORS_ALLOWED_ORIGINS"],
        ));

    let addr = std::env::var("ACCOUNT_API_BIND").unwrap_or_else(|_| "0.0.0.0:18095".to_owned());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(target = "account.bootstrap", %addr, "account api server listening");

    let serve = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

    if let Err(error) = serve.await {
        tracing::error!(target = "account.runtime", error = %error, "axum serve failed");
        return Err(error.into());
    }

    tracing::info!(target = "account.runtime", "account api server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(target = "account.runtime", error = %error, "ctrl_c signal handler failed");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                tracing::warn!(target = "account.runtime", error = %error, "SIGTERM signal handler failed");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!(
        target = "account.runtime",
        "account api server shutdown signal received, draining in-flight requests"
    );
}
