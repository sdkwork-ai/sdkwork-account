use chrono::Utc;
use sdkwork_account_service::{
    ExpirePointsLotsCommand, ExpirePointsLotsOutcome, PointsLotAllocationItem,
    PointsLotAllocationListQuery, PointsLotMismatchItem, PointsReconciliationQuery,
    PointsReconciliationSnapshot, PointsSummarySnapshot, WalletAccountListQuery,
};
use sdkwork_contract_service::{
    CommerceAccountAssetType, CommerceLedgerBusinessType, CommerceLedgerDirection,
    CommerceRequestHash, CommerceServiceError,
};
use sdkwork_utils_rust::MAX_LIST_PAGE_SIZE;
use sqlx::{Postgres, Transaction};

use crate::postgres_account::StoredAccount;
use crate::postgres_account::{format_amount_minor, integer_cell, parse_amount_minor, string_cell};
use crate::store::{
    account_guard::require_positive_amount,
    asset_code_from_type,
    balance::sum_amount_strings,
    billing_projection, format_i64, next_entity_id, next_entity_uuid, org_id_from_option,
    outbox::{emit_domain_outbox_postgres, OUTBOX_EVENT_TYPE_POINTS_LOTS_EXPIRED},
    parse_subject_i64, store_error, ACCOUNT_STATUS_ACTIVE, OWNER_TYPE_USER,
    POINTS_LOT_EXPIRE_SCOPE, POINTS_LOT_STATUS_EXPIRED, POINTS_RECONCILE_BATCH_SIZE,
};

impl crate::postgres_account::PostgresCommerceAccountStore {
    pub async fn expire_points_lots(
        &self,
        command: ExpirePointsLotsCommand,
        request_hash: CommerceRequestHash,
    ) -> Result<ExpirePointsLotsOutcome, CommerceServiceError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| store_error("failed to begin points expire transaction", error))?;
        let now = Utc::now().to_rfc3339();
        let tenant_id = parse_subject_i64("tenant_id", &command.tenant_id)?;
        let organization_id = org_id_from_option(command.organization_id.as_deref())?;

        if let Some(replayed) = try_replay_points_expire(
            &mut tx,
            tenant_id,
            &command.idempotency_key,
            request_hash.as_str(),
        )
        .await?
        {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit points expire replay", error))?;
            return Ok(replayed);
        }

        crate::postgres_hold::insert_idempotency_scoped_public(
            &mut tx,
            tenant_id,
            POINTS_LOT_EXPIRE_SCOPE,
            &command.idempotency_key,
            request_hash.as_str(),
            "points_expire",
            &now,
        )
        .await?;

        let owner_filter = command
            .owner_user_id
            .as_deref()
            .map(|value| parse_subject_i64("owner_user_id", value))
            .transpose()?;
        let account_filter = command
            .account_id
            .as_deref()
            .map(|value| parse_subject_i64("account_id", value))
            .transpose()?;

        let mut expired_lot_count = 0_i64;
        let mut expired_points_total = 0_i64;

        loop {
            let rows = sqlx::query(
                r#"
                SELECT lot.id, lot.account_id, lot.remaining_amount
                FROM acct_points_lot lot
                INNER JOIN acct_account account
                    ON account.id = lot.account_id
                   AND account.tenant_id = lot.tenant_id
                WHERE lot.tenant_id = $1
                  AND account.organization_id = $2
                  AND account.asset_code = 'points'
                  AND lot.status = $3
                  AND lot.remaining_amount > 0
                  AND lot.expires_at IS NOT NULL
                  AND lot.expires_at <= $4::timestamptz
                  AND ($5::bigint IS NULL OR account.owner_id = $5)
                  AND ($6::bigint IS NULL OR lot.account_id = $6)
                ORDER BY lot.expires_at ASC, lot.id ASC
                LIMIT $7
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(ACCOUNT_STATUS_ACTIVE)
            .bind(&now)
            .bind(owner_filter)
            .bind(account_filter)
            .bind(crate::store::EXPIRE_SWEEP_BATCH_SIZE)
            .fetch_all(&mut *tx)
            .await
            .map_err(|error| store_error("failed to load expired points lots", error))?;

            if rows.is_empty() {
                break;
            }

            let batch_len = rows.len();
            for row in rows {
                let lot_id = integer_cell(&row, "id");
                let account_id = integer_cell(&row, "account_id");
                let amount = integer_cell(&row, "remaining_amount");
                if amount <= 0 {
                    continue;
                }
                expire_one_points_lot(
                    &mut tx, tenant_id, lot_id, account_id, amount, &command, &now,
                )
                .await?;
                expired_lot_count += 1;
                expired_points_total += amount;
            }

            if batch_len < crate::store::EXPIRE_SWEEP_BATCH_SIZE as usize {
                break;
            }
        }

        let outcome = ExpirePointsLotsOutcome {
            accepted: true,
            replayed: false,
            expired_lot_count,
            expired_points_total,
        };

        complete_points_expire_idempotency(
            &mut tx,
            tenant_id,
            &command.idempotency_key,
            &outcome,
            &now,
        )
        .await?;

        let aggregate_id = account_filter.unwrap_or(if organization_id != 0 {
            organization_id
        } else {
            tenant_id
        });
        emit_domain_outbox_postgres(
            &mut tx,
            tenant_id,
            aggregate_id,
            OUTBOX_EVENT_TYPE_POINTS_LOTS_EXPIRED,
            &command.idempotency_key,
            &serde_json::json!({
                "expiredLotCount": expired_lot_count,
                "expiredPointsTotal": expired_points_total,
                "organizationId": organization_id,
                "ownerUserId": command.owner_user_id,
                "accountId": command.account_id,
            }),
            &now,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit points expire transaction", error))?;

        Ok(outcome)
    }

    pub async fn list_points_lot_allocations(
        &self,
        query: PointsLotAllocationListQuery,
    ) -> Result<Vec<PointsLotAllocationItem>, CommerceServiceError> {
        let tenant_id = parse_subject_i64("tenant_id", &query.tenant_id)?;
        let organization_id = org_id_from_option(query.organization_id.as_deref())?;
        let owner_id = parse_subject_i64("owner_user_id", &query.owner_user_id)?;
        let ledger_id = parse_subject_i64("ledger_entry_id", &query.ledger_entry_id)?;

        let ledger = sqlx::query(
            r#"
            SELECT id
            FROM acct_ledger_entry
            WHERE tenant_id = $1
              AND organization_id = $2
              AND owner_id = $3
              AND id = $4
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(owner_id)
        .bind(ledger_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| store_error("failed to verify ledger entry ownership", error))?;

        if ledger.is_none() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query(
            r#"
            SELECT id, uuid, ledger_id, lot_id, amount, created_at
            FROM acct_points_lot_allocation
            WHERE tenant_id = $1 AND ledger_id = $2
            ORDER BY id ASC
            LIMIT $3
            "#,
        )
        .bind(tenant_id)
        .bind(ledger_id)
        .bind(i64::from(MAX_LIST_PAGE_SIZE))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| store_error("failed to list points lot allocations", error))?;

        rows.iter().map(map_points_lot_allocation).collect()
    }

    pub async fn retrieve_points_summary(
        &self,
        query: WalletAccountListQuery,
    ) -> Result<PointsSummarySnapshot, CommerceServiceError> {
        let snapshot = self.retrieve_points_account_snapshot(query.clone()).await?;
        let tenant_id = parse_subject_i64("tenant_id", &query.tenant_id)?;
        let account_id = parse_subject_i64("account_id", &snapshot.account.id)?;

        let now = Utc::now().to_rfc3339();
        let stats = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(CASE
                    WHEN expires_at IS NOT NULL AND expires_at <= $1::timestamptz
                    THEN remaining_amount ELSE 0
                END), 0) AS unswept_expired_points
            FROM acct_points_lot
            WHERE tenant_id = $2 AND account_id = $3 AND status = $4
            "#,
        )
        .bind(&now)
        .bind(tenant_id)
        .bind(account_id)
        .bind(ACCOUNT_STATUS_ACTIVE)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| store_error("failed to load unswept expired points", error))?;

        let month_stats = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN direction = 'credit' THEN CAST(amount AS BIGINT) ELSE 0 END), 0) AS month_credit,
                COALESCE(SUM(CASE WHEN direction = 'debit' THEN CAST(amount AS BIGINT) ELSE 0 END), 0) AS month_debit
            FROM acct_ledger_entry
            WHERE tenant_id = $1
              AND account_id = $2
              AND asset_code = 'points'
              AND created_at >= date_trunc('month', NOW() AT TIME ZONE 'UTC')
            "#,
        )
        .bind(tenant_id)
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| store_error("failed to load monthly points ledger stats", error))?;

        let available = snapshot.account.available_amount.as_str().to_owned();
        let frozen = snapshot.account.frozen_amount.as_str().to_owned();
        let pending = snapshot.account.pending_amount.as_str().to_owned();

        Ok(PointsSummarySnapshot {
            account: snapshot.account,
            available_points: available.clone(),
            frozen_points: frozen.clone(),
            pending_points: pending.clone(),
            total_points: sum_amount_strings(&available, &frozen, &pending),
            active_lot_count: snapshot.active_lot_count,
            expiring_points: snapshot.expiring_points,
            unswept_expired_points: integer_cell(&stats, "unswept_expired_points"),
            month_credit_points: integer_cell(&month_stats, "month_credit"),
            month_debit_points: integer_cell(&month_stats, "month_debit"),
        })
    }

    pub async fn reconcile_points_lots(
        &self,
        query: PointsReconciliationQuery,
    ) -> Result<PointsReconciliationSnapshot, CommerceServiceError> {
        let tenant_id = parse_subject_i64("tenant_id", &query.tenant_id)?;
        let organization_id = org_id_from_option(query.organization_id.as_deref())?;
        let owner_filter = query
            .owner_user_id
            .as_deref()
            .map(|value| parse_subject_i64("owner_user_id", value))
            .transpose()?;

        let mut mismatches = Vec::new();
        let mut offset = 0_i64;

        loop {
            let account_rows = sqlx::query(
                r#"
                SELECT id, available_amount
                FROM acct_account
                WHERE tenant_id = $1
                  AND organization_id = $2
                  AND asset_code = 'points'
                  AND ($3::bigint IS NULL OR owner_id = $3)
                ORDER BY id ASC
                LIMIT $4 OFFSET $5
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(owner_filter)
            .bind(POINTS_RECONCILE_BATCH_SIZE)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                store_error("failed to load points accounts for reconciliation", error)
            })?;

            if account_rows.is_empty() {
                break;
            }

            for row in &account_rows {
                let account_id = integer_cell(row, "id");
                let available = format_i64(integer_cell(row, "available_amount"));
                let lot_sum: i64 = sqlx::query_scalar(
                    r#"
                    SELECT COALESCE(SUM(remaining_amount), 0)
                    FROM acct_points_lot
                    WHERE tenant_id = $1 AND account_id = $2
                    "#,
                )
                .bind(tenant_id)
                .bind(account_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|error| {
                    store_error("failed to sum lot remaining for reconciliation", error)
                })?;
                let available_i64 = available.trim().parse::<i64>().map_err(|_| {
                    CommerceServiceError::storage("available_amount is not a valid integer")
                })?;
                if available_i64 != lot_sum {
                    mismatches.push(PointsLotMismatchItem {
                        account_id: format_i64(account_id),
                        available_points: available,
                        lot_remaining_total: lot_sum,
                        delta: available_i64 - lot_sum,
                    });
                }
            }

            offset += account_rows.len() as i64;
        }

        let checked_account_count = offset;

        Ok(PointsReconciliationSnapshot {
            checked_account_count,
            mismatch_count: mismatches.len() as i64,
            mismatches,
        })
    }
}

async fn expire_one_points_lot(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    lot_id: i64,
    account_id: i64,
    amount: i64,
    command: &ExpirePointsLotsCommand,
    now: &str,
) -> Result<(), CommerceServiceError> {
    require_positive_amount(i128::from(amount), "lot remaining_amount")?;

    let account = load_account_for_expire(tx, tenant_id, account_id).await?;
    let current_balance = parse_amount_minor(&account.available_amount)?;
    let amount_i128 = i128::from(amount);
    if current_balance < amount_i128 {
        return Err(CommerceServiceError::invalid_state(
            "insufficient account balance for points lot expiry",
        ));
    }

    let balance_before = format_amount_minor(current_balance);
    let balance_after = format_amount_minor(current_balance - amount_i128);
    let next_version = account.version + 1;

    let update = sqlx::query(
        r#"
        UPDATE acct_account
        SET available_amount = $1::bigint, version = $2, updated_at = $3::timestamptz
        WHERE id = $4 AND version = $5
          AND tenant_id = $6 AND organization_id = $7
        "#,
    )
    .bind(&balance_after)
    .bind(next_version)
    .bind(now)
    .bind(account.id)
    .bind(account.version)
    .bind(account.tenant_id)
    .bind(account.organization_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to update account for lot expiry", error))?;
    if update.rows_affected() != 1 {
        return Err(CommerceServiceError::conflict(
            "account balance update was not applied atomically during lot expiry",
        ));
    }

    let journal_id = next_entity_id()?;
    let journal_uuid = next_entity_uuid();
    let ledger_id = next_entity_id()?;
    let ledger_uuid = next_entity_uuid();
    let trace_id = next_entity_uuid();
    let transaction_no = format!("points-expire:lot:{lot_id}");
    let idempotency_key = format!("{}:lot:{lot_id}", command.idempotency_key.trim());

    sqlx::query(
        r#"
        INSERT INTO acct_journal
            (id, uuid, tenant_id, business_type, business_no, request_no, idempotency_key,
             status, trace_id, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::timestamptz)
        "#,
    )
    .bind(journal_id)
    .bind(&journal_uuid)
    .bind(tenant_id)
    .bind(CommerceLedgerBusinessType::POINTS_EXPIRE)
    .bind(&transaction_no)
    .bind(&command.request_no)
    .bind(&idempotency_key)
    .bind(ACCOUNT_STATUS_ACTIVE)
    .bind(&trace_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert expire journal", error))?;

    sqlx::query(
        r#"
        INSERT INTO acct_ledger_entry
            (id, uuid, tenant_id, organization_id, account_id, journal_id, owner_type, owner_id,
             asset_code, currency_code, ledger_type, entry_type, direction, amount,
             balance_before, balance_after, business_type, business_no, request_no,
             idempotency_key, reversed_ledger_id, trace_id, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'AVAILABLE', $11, $12, $13::bigint, $14::bigint, $15::bigint, $16, $17, $18, $19, $20, $21, $22::timestamptz)
        "#,
    )
    .bind(ledger_id)
    .bind(&ledger_uuid)
    .bind(tenant_id)
    .bind(account.organization_id)
    .bind(account.id)
    .bind(journal_id)
    .bind(OWNER_TYPE_USER)
    .bind(account.owner_id)
    .bind(asset_code_from_type(&CommerceAccountAssetType::Points))
    .bind("POINT")
    .bind("DEBIT")
    .bind("DEBIT")
    .bind(format_amount_minor(amount_i128))
    .bind(&balance_before)
    .bind(&balance_after)
    .bind(CommerceLedgerBusinessType::POINTS_EXPIRE)
    .bind(&transaction_no)
    .bind(&command.request_no)
    .bind(&idempotency_key)
    .bind(None::<i64>)
    .bind(&trace_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert expire ledger", error))?;

    sqlx::query(
        r#"
        UPDATE acct_points_lot
        SET remaining_amount = 0, status = $1, updated_at = $2::timestamptz
        WHERE id = $3 AND tenant_id = $4 AND status = $5
        "#,
    )
    .bind(POINTS_LOT_STATUS_EXPIRED)
    .bind(now)
    .bind(lot_id)
    .bind(tenant_id)
    .bind(ACCOUNT_STATUS_ACTIVE)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to mark points lot expired", error))?;

    sqlx::query(
        r#"
        INSERT INTO acct_points_lot_allocation
            (id, uuid, tenant_id, account_id, ledger_id, lot_id, amount, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8::timestamptz)
        "#,
    )
    .bind(next_entity_id()?)
    .bind(next_entity_uuid())
    .bind(tenant_id)
    .bind(account.id)
    .bind(ledger_id)
    .bind(lot_id)
    .bind(amount)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert expire lot allocation", error))?;

    use sdkwork_account_service::AppendLedgerEntryCommand;
    use sdkwork_contract_service::CommerceMoney;

    let billing_command = AppendLedgerEntryCommand::with_options(
        &command.tenant_id,
        command.organization_id.as_deref(),
        &format_i64(account.id),
        &format_i64(account.owner_id),
        CommerceAccountAssetType::Points,
        Some("POINT"),
        CommerceLedgerDirection::Debit,
        CommerceMoney::new(&amount.to_string()).map_err(CommerceServiceError::validation)?,
        CommerceLedgerBusinessType::POINTS_EXPIRE,
        &transaction_no,
        &command.request_no,
        &idempotency_key,
        None,
        None,
    )?;

    billing_projection::insert_billing_history_for_ledger_append_postgres(
        &mut **tx,
        tenant_id,
        account.organization_id,
        account.owner_id,
        ledger_id,
        &billing_command,
        now,
    )
    .await?;

    Ok(())
}

async fn load_account_for_expire(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    account_id: i64,
) -> Result<StoredAccount, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, uuid, tenant_id, organization_id, owner_id, asset_code, currency_code,
               available_amount, frozen_amount, pending_amount, status, version
        FROM acct_account
        WHERE tenant_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(account_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load account for lot expiry", error))?
    .ok_or_else(|| CommerceServiceError::not_found("account was not found"))?;

    crate::postgres_account::map_stored_account(&row)
}

async fn try_replay_points_expire(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    idempotency_key: &str,
    request_hash: &str,
) -> Result<Option<ExpirePointsLotsOutcome>, CommerceServiceError> {
    let Some(row) = crate::postgres_hold::load_idempotency_scoped_public(
        tx,
        tenant_id,
        POINTS_LOT_EXPIRE_SCOPE,
        idempotency_key,
    )
    .await?
    else {
        return Ok(None);
    };

    if string_cell(&row, "request_hash") != request_hash {
        return Err(CommerceServiceError::conflict(
            "idempotency key was used with a different request hash",
        ));
    }

    match crate::store::resolve_idempotency_from_row_fields(
        request_hash,
        &string_cell(&row, "request_hash"),
        &string_cell(&row, "status"),
        &string_cell(&row, "locked_until"),
        Utc::now(),
    )? {
        crate::store::IdempotencyRecordAction::ReclaimLock => {
            crate::postgres_hold::reclaim_idempotency_scoped_public(
                tx,
                tenant_id,
                POINTS_LOT_EXPIRE_SCOPE,
                idempotency_key,
                request_hash,
                &Utc::now().to_rfc3339(),
            )
            .await?;
            return Ok(None);
        }
        crate::store::IdempotencyRecordAction::Replay => {}
    }

    let snapshot = string_cell(&row, "response_snapshot");
    let value: serde_json::Value = serde_json::from_str(&snapshot).map_err(|error| {
        CommerceServiceError::storage(format!("failed to decode expire replay snapshot: {error}"))
    })?;
    Ok(Some(ExpirePointsLotsOutcome {
        accepted: value
            .get("accepted")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        replayed: true,
        expired_lot_count: value
            .get("expiredLotCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        expired_points_total: value
            .get("expiredPointsTotal")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
    }))
}

async fn complete_points_expire_idempotency(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    idempotency_key: &str,
    outcome: &ExpirePointsLotsOutcome,
    now: &str,
) -> Result<(), CommerceServiceError> {
    let snapshot = serde_json::json!({
        "accepted": outcome.accepted,
        "replayed": outcome.replayed,
        "expiredLotCount": outcome.expired_lot_count,
        "expiredPointsTotal": outcome.expired_points_total,
    })
    .to_string();

    sqlx::query(
        r#"
        UPDATE acct_idempotency_record
        SET status = 'COMPLETED', response_snapshot = $1, locked_until = NULL, updated_at = $2::timestamptz
        WHERE tenant_id = $3 AND scope = $4 AND idempotency_key = $5
        "#,
    )
    .bind(snapshot)
    .bind(now)
    .bind(tenant_id)
    .bind(POINTS_LOT_EXPIRE_SCOPE)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to complete points expire idempotency", error))?;
    Ok(())
}

fn map_points_lot_allocation(
    row: &sqlx::postgres::PgRow,
) -> Result<PointsLotAllocationItem, CommerceServiceError> {
    Ok(PointsLotAllocationItem {
        id: format_i64(integer_cell(row, "id")),
        uuid: string_cell(row, "uuid"),
        ledger_id: format_i64(integer_cell(row, "ledger_id")),
        lot_id: format_i64(integer_cell(row, "lot_id")),
        amount: integer_cell(row, "amount"),
        created_at: string_cell(row, "created_at"),
    })
}
