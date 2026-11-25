//! Backend routes for the account billing policy aggregate.
//!
//! These endpoints are the operator surface of the account centre: they read the
//! policy that governs a subject, list stored revisions, and store or retire a
//! policy. They live under the account domain's `wallet` surface because the
//! policy is account data; pricing stays in the routing owner.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use sdkwork_account_repository_sqlx::PostgresBillingPolicyStore;
use sdkwork_account_service::{
    parse_bps, AdmissionDecision, AdmissionMode, BillingAdmission, BillingAuthorizationQuery,
    BillingPolicyItem, BillingPolicyListQuery, BillingPolicyParams, BillingPolicyResolveQuery,
    ChargeMode, OveragePolicy, SetBillingPolicyStatusCommand, SettlementMode, StoreListPage,
    UpsertBillingPolicyCommand,
};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::{IamAppContext, LoginScope};
use sdkwork_web_core::WebRequestContext;

use crate::api_response::{
    map_service_error, success_created_item, success_item, success_items, unauthorized, validation,
};
use crate::subject::backend_runtime_subject_from_extension;

pub type CommerceBillingPolicyFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

/// Store boundary for the router, so tests can substitute a fake.
pub trait CommerceBillingPolicyStore: Send + Sync {
    fn resolve_billing_policy<'a>(
        &'a self,
        query: BillingPolicyResolveQuery,
    ) -> CommerceBillingPolicyFuture<'a, Option<BillingPolicyItem>>;

    fn list_billing_policies<'a>(
        &'a self,
        query: BillingPolicyListQuery,
    ) -> CommerceBillingPolicyFuture<'a, StoreListPage<BillingPolicyItem>>;

    fn upsert_billing_policy<'a>(
        &'a self,
        command: UpsertBillingPolicyCommand,
    ) -> CommerceBillingPolicyFuture<'a, BillingPolicyItem>;

    fn set_billing_policy_status<'a>(
        &'a self,
        command: SetBillingPolicyStatusCommand,
    ) -> CommerceBillingPolicyFuture<'a, BillingPolicyItem>;

    /// Answers what the account side would decide for an estimated charge.
    fn authorize_billing<'a>(
        &'a self,
        query: BillingAuthorizationQuery,
    ) -> CommerceBillingPolicyFuture<'a, BillingAdmission>;
}

impl CommerceBillingPolicyStore for PostgresBillingPolicyStore {
    fn resolve_billing_policy<'a>(
        &'a self,
        query: BillingPolicyResolveQuery,
    ) -> CommerceBillingPolicyFuture<'a, Option<BillingPolicyItem>> {
        Box::pin(
            async move { PostgresBillingPolicyStore::resolve_billing_policy(self, &query).await },
        )
    }

    fn list_billing_policies<'a>(
        &'a self,
        query: BillingPolicyListQuery,
    ) -> CommerceBillingPolicyFuture<'a, StoreListPage<BillingPolicyItem>> {
        Box::pin(
            async move { PostgresBillingPolicyStore::list_billing_policies(self, &query).await },
        )
    }

    fn upsert_billing_policy<'a>(
        &'a self,
        command: UpsertBillingPolicyCommand,
    ) -> CommerceBillingPolicyFuture<'a, BillingPolicyItem> {
        Box::pin(
            async move { PostgresBillingPolicyStore::upsert_billing_policy(self, &command).await },
        )
    }

    fn set_billing_policy_status<'a>(
        &'a self,
        command: SetBillingPolicyStatusCommand,
    ) -> CommerceBillingPolicyFuture<'a, BillingPolicyItem> {
        Box::pin(async move {
            PostgresBillingPolicyStore::set_billing_policy_status(self, &command).await
        })
    }

    fn authorize_billing<'a>(
        &'a self,
        query: BillingAuthorizationQuery,
    ) -> CommerceBillingPolicyFuture<'a, BillingAdmission> {
        Box::pin(async move { PostgresBillingPolicyStore::evaluate_billing(self, &query).await })
    }
}

#[derive(Clone)]
struct BackendBillingPolicyState {
    store: Arc<dyn CommerceBillingPolicyStore>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveBillingPolicyQueryRequest {
    #[serde(default)]
    organization_id: Option<String>,
    owner_user_id: String,
    #[serde(default)]
    api_key_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListBillingPoliciesQueryRequest {
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default)]
    scope_type: Option<i16>,
    #[serde(default)]
    status: Option<i16>,
    #[serde(default)]
    page: Option<i64>,
    /// The GET wire uses snake_case for paging (page_size), per the paging standard.
    #[serde(default, rename = "page_size")]
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertBillingPolicyRequest {
    #[serde(default)]
    organization_id: Option<String>,
    scope_type: i16,
    #[serde(default)]
    scope_id: String,
    #[serde(default = "default_policy_priority")]
    priority: i32,
    policy_code: String,
    name: String,
    charge_mode: String,
    settlement_mode: String,
    /// Stored decimal form (`"0.5000"`); converted to basis points exactly.
    precharge_ratio: String,
    output_token_ceiling: i64,
    hold_ttl_seconds: i64,
    #[serde(default)]
    min_hold_points: i64,
    max_hold_share: String,
    admission_mode: String,
    #[serde(default)]
    overdraft_envelope_points: i64,
    #[serde(default)]
    credit_limit_points: i64,
    overage_policy: String,
    #[serde(default)]
    effective_from: Option<String>,
    #[serde(default)]
    effective_to: Option<String>,
}

fn default_policy_priority() -> i32 {
    100
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BillingPolicyResponse {
    id: String,
    uuid: String,
    tenant_id: String,
    organization_id: String,
    policy_code: String,
    name: String,
    scope_type: i16,
    scope_id: String,
    priority: i32,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    revision: i64,
    status: i16,
    charge_mode: String,
    settlement_mode: String,
    precharge_ratio: String,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    output_token_ceiling: i64,
    input_token_source: String,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    hold_ttl_seconds: i64,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    min_hold_points: i64,
    max_hold_share: String,
    admission_mode: String,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    overdraft_envelope_points: i64,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    credit_limit_points: i64,
    allowance_period: Option<String>,
    allowance_quota: Option<String>,
    allowance_reset_mode: Option<String>,
    overage_policy: String,
    effective_from: String,
    effective_to: Option<String>,
    updated_at: String,
}

impl From<BillingPolicyItem> for BillingPolicyResponse {
    fn from(value: BillingPolicyItem) -> Self {
        Self {
            id: value.id,
            uuid: value.uuid,
            tenant_id: value.tenant_id,
            organization_id: value.organization_id,
            policy_code: value.policy_code,
            name: value.name,
            scope_type: value.scope_type,
            scope_id: value.scope_id,
            priority: value.priority,
            revision: value.revision,
            status: value.status,
            charge_mode: value.charge_mode,
            settlement_mode: value.settlement_mode,
            precharge_ratio: value.precharge_ratio,
            output_token_ceiling: value.output_token_ceiling,
            input_token_source: value.input_token_source,
            hold_ttl_seconds: value.hold_ttl_seconds,
            min_hold_points: value.min_hold_points,
            max_hold_share: value.max_hold_share,
            admission_mode: value.admission_mode,
            overdraft_envelope_points: value.overdraft_envelope_points,
            credit_limit_points: value.credit_limit_points,
            allowance_period: value.allowance_period,
            allowance_quota: value.allowance_quota.map(|value| value.to_string()),
            allowance_reset_mode: value.allowance_reset_mode,
            overage_policy: value.overage_policy,
            effective_from: value.effective_from,
            effective_to: value.effective_to,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizeBillingRequest {
    #[serde(default)]
    organization_id: Option<String>,
    owner_user_id: String,
    #[serde(default)]
    api_key_id: Option<String>,
    /// Estimated charge for the hypothetical request, in token-bank points.
    estimated_points: i64,
    /// Wallet owner kind (`USER` or `ORGANIZATION`); `None` means `USER`.
    #[serde(default)]
    wallet_owner_type: Option<String>,
    #[serde(default)]
    wallet_owner_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BillingAdmissionResponse {
    /// `admit`, `deferred`, `free` or `reject`.
    decision: String,
    #[serde(with = "sdkwork_utils_rust::serde_int64")]
    hold_points: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    reject_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy_id: Option<String>,
}

impl From<BillingAdmission> for BillingAdmissionResponse {
    fn from(value: BillingAdmission) -> Self {
        let (decision, reject_code) = match value.decision {
            AdmissionDecision::Admit { .. } => ("admit", None),
            AdmissionDecision::AdmitDeferred => ("deferred", None),
            AdmissionDecision::AdmitFree => ("free", None),
            AdmissionDecision::Reject { code } => ("reject", Some(code.as_str().to_owned())),
        };
        Self {
            decision: decision.to_owned(),
            hold_points: value.hold_points,
            reject_code,
            policy_id: value.policy_id,
        }
    }
}
fn web_context(context: Option<Extension<WebRequestContext>>) -> Option<WebRequestContext> {
    context.map(|Extension(context)| context)
}

fn tenant_of(subject: &crate::subject::BackendRuntimeSubject) -> String {
    subject.tenant_id.clone()
}

async fn resolve_billing_policy(
    State(state): State<BackendBillingPolicyState>,
    context: Option<Extension<IamAppContext>>,
    web: Option<Extension<WebRequestContext>>,
    Query(query): Query<ResolveBillingPolicyQueryRequest>,
) -> Response {
    let web = web_context(web);
    let subject = match backend_runtime_subject_from_extension(context) {
        Ok(subject) => subject,
        Err(detail) => return unauthorized(web.as_ref(), detail),
    };
    let resolved = state
        .store
        .resolve_billing_policy(BillingPolicyResolveQuery {
            tenant_id: tenant_of(&subject),
            organization_id: query.organization_id,
            owner_user_id: query.owner_user_id,
            api_key_id: query.api_key_id,
        })
        .await;
    match resolved {
        Ok(Some(item)) => success_item(web.as_ref(), BillingPolicyResponse::from(item)),
        // No stored row means the documented platform defaults govern the
        // subject; answering with an empty item is the honest representation.
        Ok(None) => success_item(web.as_ref(), Option::<BillingPolicyResponse>::None),
        Err(error) => map_service_error(web.as_ref(), error),
    }
}

async fn list_billing_policies(
    State(state): State<BackendBillingPolicyState>,
    context: Option<Extension<IamAppContext>>,
    web: Option<Extension<WebRequestContext>>,
    Query(query): Query<ListBillingPoliciesQueryRequest>,
) -> Response {
    let web = web_context(web);
    let subject = match backend_runtime_subject_from_extension(context) {
        Ok(subject) => subject,
        Err(detail) => return unauthorized(web.as_ref(), detail),
    };
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(1, 200);
    let listed = state
        .store
        .list_billing_policies(BillingPolicyListQuery {
            tenant_id: tenant_of(&subject),
            organization_id: query.organization_id,
            scope_type: query.scope_type,
            status: query.status,
            page: Some(page),
            page_size: Some(page_size),
        })
        .await;
    match listed {
        Ok(result) => success_items(
            web.as_ref(),
            result
                .items
                .into_iter()
                .map(BillingPolicyResponse::from)
                .collect::<Vec<_>>(),
            page,
            page_size,
        ),
        Err(error) => map_service_error(web.as_ref(), error),
    }
}

async fn upsert_billing_policy(
    State(state): State<BackendBillingPolicyState>,
    context: Option<Extension<IamAppContext>>,
    web: Option<Extension<WebRequestContext>>,
    axum::Json(request): axum::Json<UpsertBillingPolicyRequest>,
) -> Response {
    let web = web_context(web);
    let Some(Extension(context)) = context else {
        return unauthorized(web.as_ref(), "authenticated runtime context is required");
    };
    let subject = match backend_runtime_subject_from_extension(Some(Extension(context.clone()))) {
        Ok(subject) => subject,
        Err(detail) => return unauthorized(web.as_ref(), detail),
    };
    // A tenant-scoped operator may only write policies for itself; organization
    // scope and above is what the routing owner authorizes separately.
    if request.scope_type == sdkwork_account_service::BILLING_POLICY_SCOPE_USER
        && context.login_scope == LoginScope::Tenant
        && request.scope_id.trim() != context.user_id.trim()
    {
        return unauthorized(
            web.as_ref(),
            "a tenant-scoped operator may only write its own user policy",
        );
    }
    let params = match policy_params_from_request(&request) {
        Ok(params) => params,
        Err(detail) => return validation(web.as_ref(), detail),
    };
    let command = UpsertBillingPolicyCommand {
        tenant_id: tenant_of(&subject),
        organization_id: request.organization_id,
        scope_type: request.scope_type,
        scope_id: request.scope_id,
        priority: request.priority,
        policy_code: request.policy_code,
        name: request.name,
        params,
        effective_from: request.effective_from,
        effective_to: request.effective_to,
        actor_user_id: Some(context.user_id.clone()),
    };
    match state.store.upsert_billing_policy(command).await {
        Ok(item) => success_created_item(web.as_ref(), BillingPolicyResponse::from(item)),
        Err(error) => map_service_error(web.as_ref(), error),
    }
}

async fn activate_billing_policy(
    State(state): State<BackendBillingPolicyState>,
    context: Option<Extension<IamAppContext>>,
    web: Option<Extension<WebRequestContext>>,
    Path(policy_id): Path<String>,
) -> Response {
    set_policy_status(state, context, web, policy_id, 1).await
}

async fn deactivate_billing_policy(
    State(state): State<BackendBillingPolicyState>,
    context: Option<Extension<IamAppContext>>,
    web: Option<Extension<WebRequestContext>>,
    Path(policy_id): Path<String>,
) -> Response {
    set_policy_status(state, context, web, policy_id, 0).await
}

async fn set_policy_status(
    state: BackendBillingPolicyState,
    context: Option<Extension<IamAppContext>>,
    web: Option<Extension<WebRequestContext>>,
    policy_id: String,
    status: i16,
) -> Response {
    let web = web_context(web);
    let Some(Extension(context)) = context else {
        return unauthorized(web.as_ref(), "authenticated runtime context is required");
    };
    let subject = match backend_runtime_subject_from_extension(Some(Extension(context.clone()))) {
        Ok(subject) => subject,
        Err(detail) => return unauthorized(web.as_ref(), detail),
    };
    let command = SetBillingPolicyStatusCommand {
        tenant_id: tenant_of(&subject),
        policy_id: policy_id.clone(),
        status,
        actor_user_id: Some(context.user_id.clone()),
    };
    match state.store.set_billing_policy_status(command).await {
        Ok(item) => success_item(web.as_ref(), BillingPolicyResponse::from(item)),
        Err(error) => map_service_error(web.as_ref(), error),
    }
}

fn policy_params_from_request(
    request: &UpsertBillingPolicyRequest,
) -> Result<BillingPolicyParams, String> {
    let charge_mode = ChargeMode::parse(&request.charge_mode)
        .ok_or_else(|| format!("unknown chargeMode {:?}", request.charge_mode))?;
    let settlement_mode = SettlementMode::parse(&request.settlement_mode)
        .ok_or_else(|| format!("unknown settlementMode {:?}", request.settlement_mode))?;
    let admission_mode = AdmissionMode::parse(&request.admission_mode)
        .ok_or_else(|| format!("unknown admissionMode {:?}", request.admission_mode))?;
    let overage_policy = OveragePolicy::parse(&request.overage_policy)
        .ok_or_else(|| format!("unknown overagePolicy {:?}", request.overage_policy))?;
    let precharge_ratio_bps = parse_bps(&request.precharge_ratio)
        .ok_or_else(|| format!("invalid prechargeRatio {:?}", request.precharge_ratio))?;
    let max_hold_share_bps = parse_bps(&request.max_hold_share)
        .ok_or_else(|| format!("invalid maxHoldShare {:?}", request.max_hold_share))?;
    let params = BillingPolicyParams {
        charge_mode,
        settlement_mode,
        admission_mode,
        overage_policy,
        precharge_ratio_bps,
        output_token_ceiling: request.output_token_ceiling,
        hold_ttl_seconds: request.hold_ttl_seconds,
        min_hold_points: request.min_hold_points,
        max_hold_share_bps,
        overdraft_envelope_points: request.overdraft_envelope_points,
        credit_limit_points: request.credit_limit_points,
    };
    params
        .validate()
        .map_err(|error| error.message().to_owned())?;
    Ok(params)
}

/// Answers what the account side would decide for a hypothetical charge.
///
/// This is the simulate primitive: it resolves the policy, reads the wallet and
/// applies the admission rule without creating a hold, so an operator can see
/// exactly why a request would be rejected before it happens.
async fn authorize_billing(
    State(state): State<BackendBillingPolicyState>,
    context: Option<Extension<IamAppContext>>,
    web: Option<Extension<WebRequestContext>>,
    axum::Json(request): axum::Json<AuthorizeBillingRequest>,
) -> Response {
    let web = web_context(web);
    let subject = match backend_runtime_subject_from_extension(context) {
        Ok(subject) => subject,
        Err(detail) => return unauthorized(web.as_ref(), detail),
    };
    let admission = state
        .store
        .authorize_billing(BillingAuthorizationQuery {
            tenant_id: tenant_of(&subject),
            organization_id: request.organization_id,
            owner_user_id: request.owner_user_id,
            api_key_id: request.api_key_id,
            estimated_points: request.estimated_points,
            wallet_owner_type: request.wallet_owner_type,
            wallet_owner_id: request.wallet_owner_id,
        })
        .await;
    match admission {
        Ok(admission) => success_item(web.as_ref(), BillingAdmissionResponse::from(admission)),
        Err(error) => map_service_error(web.as_ref(), error),
    }
}
pub fn backend_billing_policy_router_with_postgres_pool(pool: PgPool) -> Router {
    build_backend_billing_policy_router(Arc::new(PostgresBillingPolicyStore::new(pool)))
}

pub fn build_backend_billing_policy_router(store: Arc<dyn CommerceBillingPolicyStore>) -> Router {
    Router::new()
        .route(
            "/backend/v3/api/wallet/billing_policies",
            get(list_billing_policies).post(upsert_billing_policy),
        )
        .route(
            "/backend/v3/api/wallet/billing_policies/{policyId}/activate",
            post(activate_billing_policy),
        )
        .route(
            "/backend/v3/api/wallet/billing_policies/{policyId}/deactivate",
            post(deactivate_billing_policy),
        )
        .route(
            "/backend/v3/api/wallet/billing_policy",
            get(resolve_billing_policy),
        )
        .route(
            "/backend/v3/api/wallet/billing_policy/authorize",
            post(authorize_billing),
        )
        .with_state(BackendBillingPolicyState { store })
}
