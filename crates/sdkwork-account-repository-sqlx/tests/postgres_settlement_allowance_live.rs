//! Live PostgreSQL check for allowance credit at settlement (ledger model B).
//!
//! The ledger records only the wallet part of a capture (`amount == before - after`),
//! while the credited part is recorded in `acct_points_lot_allocation` against the
//! same ledger row. Skipped unless `SDKWORK_ACCOUNT_TEST_DATABASE_URL` is set.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use sdkwork_account_repository_sqlx::PostgresCommerceAccountStore;
use sdkwork_account_service::SettleAccountHoldCommand;
use sdkwork_contract_service::CommerceRequestHash;

const TENANT_ID: i64 = 100_003;
const OWNER_ID: i64 = 700_001;

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
    (nanos % 900_000_000) as i64 + 10_000
}

#[tokio::test]
async fn settlement_charges_allowance_credit_before_the_wallet() {
    let Some(pool) = connect().await else {
        return;
    };
    // The store under test reads and writes through its own pool.
    let store = PostgresCommerceAccountStore::new(pool.clone());

    // Each table is cleared by its own static statement; a formatted statement would
    // need an injection audit and buys nothing here. The idempotency record matters
    // most: a run that failed mid-transaction leaves a pending record whose empty
    // response snapshot would fail the next replay check.
    for statement in [
        "DELETE FROM acct_points_lot_allocation WHERE tenant_id = $1",
        "DELETE FROM acct_points_lot WHERE tenant_id = $1",
        "DELETE FROM acct_outbox_event WHERE tenant_id = $1",
        "DELETE FROM acct_journal WHERE tenant_id = $1",
        "DELETE FROM acct_idempotency_record WHERE tenant_id = $1",
        "DELETE FROM acct_ledger_entry WHERE tenant_id = $1",
        "DELETE FROM acct_hold WHERE tenant_id = $1",
        "DELETE FROM acct_account WHERE tenant_id = $1",
    ] {
        sqlx::query(statement)
            .bind(TENANT_ID)
            .execute(&pool)
            .await
            .expect("clear the previous run's rows");
    }

    // A wallet that already reserved 5,000,000 for an in-flight request.
    let account_id = next_id();
    sqlx::query(
        "INSERT INTO acct_account
             (id, uuid, tenant_id, organization_id, owner_type, owner_id, asset_code,
              currency_code, account_purpose, available_amount, frozen_amount, pending_amount,
              status, version, created_at, updated_at)
         VALUES ($1, $2, $3, 0, 'USER', $4, 'token_bank',
              'TOKEN_BANK', 'GENERAL', 5000000, 5000000, 0, 1, 1, now(), now())",
    )
    .bind(account_id)
    .bind(format!("settle-live-wallet-{OWNER_ID}"))
    .bind(TENANT_ID)
    .bind(OWNER_ID)
    .execute(&pool)
    .await
    .expect("seed the wallet");

    // 2,000,000 of credited points that must pay before the wallet does.
    let lot_id = next_id();
    sqlx::query(
        "INSERT INTO acct_points_lot
             (id, uuid, tenant_id, account_id, granted_amount, remaining_amount,
              source_type, source_id, expires_at, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 2000000, 2000000, 'plan_grant', 0,
              now() + interval '30 days', 1, now(), now())",
    )
    .bind(lot_id)
    .bind(format!("settle-live-lot-{lot_id}"))
    .bind(TENANT_ID)
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("seed the allowance lot");

    let hold_uuid = format!("settle-live-hold-{OWNER_ID}");
    sqlx::query(
        "INSERT INTO acct_hold
             (id, uuid, tenant_id, organization_id, account_id, owner_type, owner_id,
              asset_code, currency_code, amount, settled_amount, released_amount, status,
              business_type, business_no, source_type, source_id, request_no, idempotency_key,
              expires_at, version, trace_id, created_at, updated_at)
         VALUES ($1, $2, $3, 0, $4, 'USER', $5, 'token_bank', 'TOKEN_BANK',
              5000000, 0, 0, 1, 'gateway_invocation_billing', $2, 'gateway', $5, $2, $2,
              now() + interval '30 minutes', 1, 'settle-live-trace', now(), now())",
    )
    .bind(next_id())
    .bind(&hold_uuid)
    .bind(TENANT_ID)
    .bind(account_id)
    .bind(OWNER_ID)
    .bind(format!("{hold_uuid}-settle"))
    .execute(&pool)
    .await
    .expect("seed the hold");

    store
        .settle_account_hold(
            SettleAccountHoldCommand {
                tenant_id: TENANT_ID.to_string(),
                hold_id: hold_uuid.clone(),
                business_type: "gateway_invocation_billing".to_owned(),
                transaction_no: format!("{hold_uuid}-settle"),
                request_no: format!("{hold_uuid}-settle"),
                idempotency_key: format!("{hold_uuid}-settle"),
                settled_amount: Some("5000000".to_owned()),
            },
            CommerceRequestHash::new("settle-live-hash").expect("request hash"),
        )
        .await
        .expect("settle the hold");

    // The lot paid 2,000,000 of the 5,000,000 capture.
    let lot_remaining: i64 =
        sqlx::query_scalar("SELECT remaining_amount FROM acct_points_lot WHERE id = $1")
            .bind(lot_id)
            .fetch_one(&pool)
            .await
            .expect("read the lot");
    assert_eq!(0, lot_remaining, "the credited points paid first");

    // The wallet therefore lost 3,000,000, not 5,000,000: 5,000,000 + (5,000,000 - 3,000,000).
    let (available, frozen): (i64, i64) =
        sqlx::query_as("SELECT available_amount, frozen_amount FROM acct_account WHERE id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("read the wallet");
    assert_eq!(7_000_000, available);
    assert_eq!(0, frozen);

    // The ledger records only the wallet part, so amount == before - after still holds.
    let (amount, before, after): (i64, i64, i64) = sqlx::query_as(
        "SELECT amount, balance_before, balance_after FROM acct_ledger_entry
          WHERE tenant_id = $1 AND hold_id = (SELECT id FROM acct_hold WHERE uuid = $2)
          ORDER BY id DESC LIMIT 1",
    )
    .bind(TENANT_ID)
    .bind(&hold_uuid)
    .fetch_one(&pool)
    .await
    .expect("read the settlement ledger entry");
    assert_eq!(3_000_000, amount, "the ledger carries only the wallet part");
    assert_eq!(before - after, amount, "amount == before - after must hold");
    assert_eq!(10_000_000, before);

    // And the credit consumption is auditable against that same ledger row.
    let allocated: i64 = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(amount), 0) AS BIGINT) FROM acct_points_lot_allocation
          WHERE tenant_id = $1 AND ledger_id = (
              SELECT id FROM acct_ledger_entry
               WHERE tenant_id = $1 AND hold_id = (SELECT id FROM acct_hold WHERE uuid = $2)
               ORDER BY id DESC LIMIT 1)",
    )
    .bind(TENANT_ID)
    .bind(&hold_uuid)
    .fetch_one(&pool)
    .await
    .expect("sum the allocations");
    assert_eq!(2_000_000, allocated);
}
