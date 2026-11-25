//! Live PostgreSQL checks for the billing policy repository.
//!
//! These tests exercise the real SQL rather than a fake store: the upsert
//! transaction, the live-row resolution, list pagination and the admission
//! evaluation against a seeded wallet. They are skipped unless
//! `SDKWORK_ACCOUNT_TEST_DATABASE_URL` points at a database with the account
//! baseline plus migrations applied (and the schema on the connection search path).
//!
//! Every test owns its own subject id (`scope_id` / `owner_id`), so the suite can
//! run in parallel without one test retiring another test's live revision. That is
//! also why the cleanup deletes only the rows a test owns.
//!
//! Run them on the build host with:
//!
//! ```bash
//! SDKWORK_ACCOUNT_TEST_DATABASE_URL='postgres://user:pass@127.0.0.1:55432/db?options=-csearch_path%3Ddb' \
//!   cargo test -p sdkwork-account-repository-sqlx --test postgres_billing_policy_live
//! ```

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use sdkwork_account_repository_sqlx::PostgresBillingPolicyStore;
use sdkwork_account_service::{
    BillingPolicyListQuery, BillingPolicyResolveQuery, UpsertBillingPolicyCommand,
};
use sdkwork_contract_service::{
    AdmissionDecision, BillingAuthorizationQuery, BillingPolicyParams, ChargeMode, OveragePolicy,
    SettlementMode,
};

const TENANT_ID: i64 = 100_001;

/// A subject id owned by a single test.
const SUBJECT_OWNERSHIP: i64 = 500_001;
const SUBJECT_REVISIONS: i64 = 500_002;
const SUBJECT_PAGING: i64 = 500_003;
const SUBJECT_ADMISSION: i64 = 500_004;

async fn connect() -> Option<PgPool> {
    let url = std::env::var("SDKWORK_ACCOUNT_TEST_DATABASE_URL").ok()?;
    match PgPoolOptions::new().max_connections(2).connect(&url).await {
        Ok(pool) => Some(pool),
        Err(error) => panic!("SDKWORK_ACCOUNT_TEST_DATABASE_URL is set but unusable: {error}"),
    }
}

/// Clears only the rows a single test owns.
async fn reset_subject(pool: &PgPool, owner_id: i64) {
    sqlx::query(
        "DELETE FROM acct_billing_policy
          WHERE tenant_id = $1 AND scope_type = 3 AND scope_id = $2",
    )
    .bind(TENANT_ID)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("clear billing policies for the test subject");
    sqlx::query("DELETE FROM acct_account WHERE tenant_id = $1 AND owner_id = $2")
        .bind(TENANT_ID)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("clear the seeded account");
}

fn user_policy(ratio_bps: u32, ceiling: i64) -> BillingPolicyParams {
    BillingPolicyParams {
        charge_mode: ChargeMode::PrepaidBalance,
        settlement_mode: SettlementMode::Synchronous,
        overage_policy: OveragePolicy::AllowReceivable,
        precharge_ratio_bps: ratio_bps,
        output_token_ceiling: ceiling,
        max_hold_share_bps: 2_500,
        ..BillingPolicyParams::default()
    }
}

fn upsert_command(scope_id: i64, params: BillingPolicyParams) -> UpsertBillingPolicyCommand {
    UpsertBillingPolicyCommand {
        tenant_id: TENANT_ID.to_string(),
        organization_id: None,
        scope_type: 3,
        scope_id: scope_id.to_string(),
        priority: 100,
        policy_code: "live-test-policy".to_owned(),
        name: "Live test policy".to_owned(),
        params,
        effective_from: None,
        effective_to: None,
        actor_user_id: None,
    }
}

async fn resolve(store: &PostgresBillingPolicyStore, scope_id: i64) -> Option<BillingPolicyParams> {
    store
        .resolve_billing_policy(&BillingPolicyResolveQuery {
            tenant_id: TENANT_ID.to_string(),
            organization_id: None,
            owner_user_id: scope_id.to_string(),
            api_key_id: None,
        })
        .await
        .expect("resolve the policy")
        .map(|item| item.params().expect("stored parameters parse"))
}

#[tokio::test]
async fn a_stored_policy_is_resolved_with_its_validated_parameters() {
    let Some(pool) = connect().await else {
        return;
    };
    let store = PostgresBillingPolicyStore::new(pool.clone());
    reset_subject(&pool, SUBJECT_OWNERSHIP).await;

    let stored = store
        .upsert_billing_policy(&upsert_command(
            SUBJECT_OWNERSHIP,
            user_policy(2_500, 32_768),
        ))
        .await
        .expect("store the policy");
    assert_eq!(1, stored.status);

    let params = resolve(&store, SUBJECT_OWNERSHIP)
        .await
        .expect("a live policy exists");
    assert_eq!(2_500, params.precharge_ratio_bps);
    assert_eq!(32_768, params.output_token_ceiling);
}

#[tokio::test]
async fn a_second_upsert_keeps_exactly_one_live_revision() {
    let Some(pool) = connect().await else {
        return;
    };
    let store = PostgresBillingPolicyStore::new(pool.clone());
    reset_subject(&pool, SUBJECT_REVISIONS).await;

    store
        .upsert_billing_policy(&upsert_command(
            SUBJECT_REVISIONS,
            user_policy(2_500, 32_768),
        ))
        .await
        .expect("first revision");
    store
        .upsert_billing_policy(&upsert_command(
            SUBJECT_REVISIONS,
            user_policy(5_000, 16_384),
        ))
        .await
        .expect("second revision");

    let live: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM acct_billing_policy
          WHERE tenant_id = $1 AND scope_type = 3 AND scope_id = $2
            AND deleted_at IS NULL AND status = 1",
    )
    .bind(TENANT_ID)
    .bind(SUBJECT_REVISIONS)
    .fetch_one(&pool)
    .await
    .expect("count live rows");
    assert_eq!(1, live, "the retired revision must not stay live");

    let params = resolve(&store, SUBJECT_REVISIONS)
        .await
        .expect("one live policy");
    assert_eq!(
        5_000, params.precharge_ratio_bps,
        "resolution must return the newest live revision"
    );
}

#[tokio::test]
async fn list_pages_do_not_repeat_their_first_row() {
    let Some(pool) = connect().await else {
        return;
    };
    let store = PostgresBillingPolicyStore::new(pool.clone());
    reset_subject(&pool, SUBJECT_PAGING).await;

    for priority in 0..4 {
        let mut command = upsert_command(SUBJECT_PAGING, user_policy(2_000, 32_768));
        command.priority = priority;
        command.policy_code = format!("live-page-{priority}");
        store
            .upsert_billing_policy(&command)
            .await
            .expect("store page row");
    }

    let page = |page: i64| BillingPolicyListQuery {
        tenant_id: TENANT_ID.to_string(),
        organization_id: None,
        page: Some(page),
        page_size: Some(2),
        ..BillingPolicyListQuery::default()
    };

    let page_one = store
        .list_billing_policies(&page(1))
        .await
        .expect("page one");
    let page_two = store
        .list_billing_policies(&page(2))
        .await
        .expect("page two");

    assert_eq!(2, page_one.items.len());
    assert_eq!(2, page_two.items.len());
    let first: Vec<&str> = page_one
        .items
        .iter()
        .map(|item| item.policy_code.as_str())
        .collect();
    let second: Vec<&str> = page_two
        .items
        .iter()
        .map(|item| item.policy_code.as_str())
        .collect();
    assert_ne!(first, second, "page two must advance past the first page");
}

#[tokio::test]
async fn admission_freezes_by_estimate_and_caps_at_the_share_limit() {
    let Some(pool) = connect().await else {
        return;
    };
    let store = PostgresBillingPolicyStore::new(pool.clone());
    reset_subject(&pool, SUBJECT_ADMISSION).await;

    sqlx::query(
        "INSERT INTO acct_account
             (id, uuid, tenant_id, organization_id, owner_type, owner_id, asset_code,
              currency_code, account_purpose, available_amount, frozen_amount, pending_amount,
              status, version, created_at, updated_at)
         VALUES ($1, $2, $3, 0, 'USER', $4, 'token_bank',
              'TOKEN_BANK', 'GENERAL', 41459562, 33256530, 0, 1, 1, now(), now())",
    )
    .bind(next_account_id())
    .bind(format!("live-test-account-{SUBJECT_ADMISSION}"))
    .bind(TENANT_ID)
    .bind(SUBJECT_ADMISSION)
    .execute(&pool)
    .await
    .expect("seed the wallet");

    store
        .upsert_billing_policy(&upsert_command(
            SUBJECT_ADMISSION,
            user_policy(10_000, 32_768),
        ))
        .await
        .expect("store the policy");

    let query = |estimated_points: i64| BillingAuthorizationQuery {
        tenant_id: TENANT_ID.to_string(),
        organization_id: None,
        owner_user_id: SUBJECT_ADMISSION.to_string(),
        api_key_id: None,
        estimated_points,
        wallet_owner_type: Some("USER".to_owned()),
        wallet_owner_id: Some(SUBJECT_ADMISSION.to_string()),
    };

    let normal = store
        .evaluate_billing(&query(1_310_720))
        .await
        .expect("evaluate a normal charge");
    assert_eq!(
        AdmissionDecision::Admit {
            hold_points: 1_310_720
        },
        normal.decision
    );

    let oversized = store
        .evaluate_billing(&query(20_971_520))
        .await
        .expect("evaluate an oversized charge");
    assert_eq!(
        AdmissionDecision::Admit {
            hold_points: 18_679_023
        },
        oversized.decision,
        "the reservation is capped by the share limit, not the estimate"
    );

    let frozen: i64 = sqlx::query_scalar(
        "SELECT frozen_amount FROM acct_account WHERE tenant_id = $1 AND owner_id = $2",
    )
    .bind(TENANT_ID)
    .bind(SUBJECT_ADMISSION)
    .fetch_one(&pool)
    .await
    .expect("read frozen amount");
    assert_eq!(33_256_530, frozen, "simulation must not move funds");
}

fn next_account_id() -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    (nanos % 1_000_000_000) as i64 + 1_000
}
