//! Live PostgreSQL checks for allowance-lot consumption.
//!
//! The primitive is the only place that decrements credited points, so its ordering,
//! expiry handling and audit trail are tested against a real database rather than a
//! fake. Skipped unless `SDKWORK_ACCOUNT_TEST_DATABASE_URL` is set.
//!
//! ```bash
//! SDKWORK_ACCOUNT_TEST_DATABASE_URL='postgres://user:pass@127.0.0.1:5432/db' \
//!   cargo test -p sdkwork-account-repository-sqlx --test postgres_allowance_lots_live
//! ```

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use sdkwork_account_repository_sqlx::take_allowance_lots;

const TENANT_ID: i64 = 100_002;
/// Each test owns a subject so parallel runs cannot clear each other's lots.
const OWNER_ORDER: i64 = 600_001;
const OWNER_BOUNDED: i64 = 600_002;
const OWNER_EMPTY: i64 = 600_003;

async fn connect() -> Option<PgPool> {
    let url = std::env::var("SDKWORK_ACCOUNT_TEST_DATABASE_URL").ok()?;
    match PgPoolOptions::new().max_connections(2).connect(&url).await {
        Ok(pool) => Some(pool),
        Err(error) => panic!("SDKWORK_ACCOUNT_TEST_DATABASE_URL is set but unusable: {error}"),
    }
}

fn next_id() -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    (nanos % 1_000_000_000) as i64 + 5_000
}

/// Seeds one wallet and returns its internal id.
async fn seed_wallet(pool: &PgPool, owner_id: i64) -> i64 {
    sqlx::query(
        "DELETE FROM acct_points_lot WHERE tenant_id = $1 AND account_id IN
             (SELECT id FROM acct_account WHERE tenant_id = $1 AND owner_id = $2)",
    )
    .bind(TENANT_ID)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("clear lots");
    sqlx::query("DELETE FROM acct_account WHERE tenant_id = $1 AND owner_id = $2")
        .bind(TENANT_ID)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("clear wallet");

    let account_id = next_id();
    sqlx::query(
        "INSERT INTO acct_account
             (id, uuid, tenant_id, organization_id, owner_type, owner_id, asset_code,
              currency_code, account_purpose, available_amount, frozen_amount, pending_amount,
              status, version, created_at, updated_at)
         VALUES ($1, $2, $3, 0, 'USER', $4, 'token_bank',
              'TOKEN_BANK', 'GENERAL', 0, 0, 0, 1, 1, now(), now())",
    )
    .bind(account_id)
    .bind(format!("allowance-live-wallet-{owner_id}"))
    .bind(TENANT_ID)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed the wallet");
    account_id
}

async fn seed_lot(pool: &PgPool, account_id: i64, remaining: i64, offset_days: Option<i64>) -> i64 {
    let lot_id = next_id();
    let expires_at = offset_days.map(|days| chrono::Utc::now() + chrono::Duration::days(days));
    sqlx::query(
        "INSERT INTO acct_points_lot
             (id, uuid, tenant_id, account_id, granted_amount, remaining_amount,
              source_type, source_id, expires_at, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $5, 'plan_grant', 0, $6, 1, now(), now())",
    )
    .bind(lot_id)
    .bind(format!("allowance-live-lot-{lot_id}"))
    .bind(TENANT_ID)
    .bind(account_id)
    .bind(remaining)
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("seed the lot");
    lot_id
}

async fn remaining(pool: &PgPool, lot_id: i64) -> i64 {
    sqlx::query_scalar("SELECT remaining_amount FROM acct_points_lot WHERE id = $1")
        .bind(lot_id)
        .fetch_one(pool)
        .await
        .expect("read remaining amount")
}

#[tokio::test]
async fn consumption_takes_the_soonest_expiring_lot_first() {
    let Some(pool) = connect().await else {
        return;
    };
    let account_id = seed_wallet(&pool, OWNER_ORDER).await;
    let soon = seed_lot(&pool, account_id, 1_000_000, Some(1)).await;
    let later = seed_lot(&pool, account_id, 2_000_000, Some(10)).await;
    let no_expiry = seed_lot(&pool, account_id, 500_000, None).await;
    let ledger_id = next_id();

    let mut tx = pool.begin().await.expect("begin");
    let consumed = take_allowance_lots(&mut tx, TENANT_ID, account_id, ledger_id, 1_500_000)
        .await
        .expect("consume allowance");
    tx.commit().await.expect("commit");

    assert_eq!(2, consumed.len(), "two lots cover the charge");
    assert_eq!(
        soon, consumed[0].lot_id,
        "the soonest expiry is spent first"
    );
    assert_eq!(1_000_000, consumed[0].amount);
    assert_eq!(later, consumed[1].lot_id);
    assert_eq!(500_000, consumed[1].amount);

    assert_eq!(0, remaining(&pool, soon).await);
    assert_eq!(1_500_000, remaining(&pool, later).await);
    assert_eq!(500_000, remaining(&pool, no_expiry).await);

    let allocations: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM acct_points_lot_allocation WHERE tenant_id = $1 AND ledger_id = $2",
    )
    .bind(TENANT_ID)
    .bind(ledger_id)
    .fetch_one(&pool)
    .await
    .expect("count allocations");
    assert_eq!(2, allocations, "every consumed lot keeps an audit row");
}

#[tokio::test]
async fn expired_and_empty_lots_cannot_pay_and_the_charge_is_bounded() {
    let Some(pool) = connect().await else {
        return;
    };
    let account_id = seed_wallet(&pool, OWNER_BOUNDED).await;
    let expired = seed_lot(&pool, account_id, 9_000_000, Some(-1)).await;
    let live = seed_lot(&pool, account_id, 2_000_000, Some(5)).await;
    let ledger_id = next_id();

    let mut tx = pool.begin().await.expect("begin");
    let consumed = take_allowance_lots(&mut tx, TENANT_ID, account_id, ledger_id, 50_000_000)
        .await
        .expect("consume allowance");
    tx.commit().await.expect("commit");

    let total: i64 = consumed.iter().map(|item| item.amount).sum();
    assert_eq!(
        2_000_000, total,
        "consumption is bounded by what the live lots actually hold"
    );
    assert_eq!(
        9_000_000,
        remaining(&pool, expired).await,
        "an expired lot must not pay for anything"
    );
    assert_eq!(0, remaining(&pool, live).await);
}

#[tokio::test]
async fn a_charge_without_allowance_lots_consumes_nothing() {
    let Some(pool) = connect().await else {
        return;
    };
    let account_id = seed_wallet(&pool, OWNER_EMPTY).await;
    let mut tx = pool.begin().await.expect("begin");
    let consumed = take_allowance_lots(&mut tx, TENANT_ID, account_id, next_id(), 1_000)
        .await
        .expect("consume allowance");
    tx.commit().await.expect("commit");
    assert!(
        consumed.is_empty(),
        "a wallet without lots is not funded by credit"
    );
}
