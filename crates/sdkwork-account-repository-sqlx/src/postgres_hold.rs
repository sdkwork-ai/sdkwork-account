use chrono::Utc;
use sdkwork_account_service::{
    AccountHoldDetailQuery, AccountHoldItem, AccountHoldListQuery, AccountTransferItem,
    CreateAccountHoldCommand, CreateAccountTransferCommand, ExpireExpiredHoldsCommand,
    ExpireExpiredHoldsOutcome, HoldMutationOutcome, ReleaseAccountHoldCommand,
    SettleAccountHoldCommand, StoreListPage, TransferMutationOutcome, WalletTransactionItem,
};
use sdkwork_contract_service::{
    CommerceAccountAssetType, CommerceLedgerDirection, CommerceRequestHash, CommerceServiceError,
};
use sdkwork_utils_rust::LIST_TOTAL_SQL_COLUMN;
use sqlx::{Postgres, Row, Transaction};

use crate::postgres_account::PostgresCommerceAccountStore;
use crate::postgres_account::StoredAccount;
use crate::postgres_account::{format_amount_minor, parse_amount_minor};
use crate::store::{
    account_guard::{
        ensure_hold_not_expired, require_positive_amount, validate_account_owner_scope,
        validate_transfer_account_pair,
    },
    asset_code_from_type, asset_type_from_code, finalize_list_page, format_i64, hold_status_label,
    next_entity_id, next_entity_uuid, optional_org_string, org_id_from_option,
    outbox::{
        emit_domain_outbox_postgres, OUTBOX_EVENT_TYPE_HOLD_CREATED,
        OUTBOX_EVENT_TYPE_HOLD_EXPIRED, OUTBOX_EVENT_TYPE_HOLD_RELEASED,
        OUTBOX_EVENT_TYPE_HOLD_SETTLED, OUTBOX_EVENT_TYPE_TRANSFER_COMPLETED,
    },
    parse_subject_i64, resolve_list_sql_paging, store_error, ACCOUNT_STATUS_ACTIVE,
    HOLD_CREATE_SCOPE, HOLD_EXPIRE_SCOPE, HOLD_RELEASE_SCOPE, HOLD_SETTLE_SCOPE, HOLD_STATUS_HELD,
    HOLD_STATUS_RELEASED, HOLD_STATUS_SETTLED, OWNER_TYPE_USER, TRANSFER_CREATE_SCOPE,
    TRANSFER_STATUS_COMPLETED,
};

impl PostgresCommerceAccountStore {
    pub async fn create_account_hold(
        &self,
        command: CreateAccountHoldCommand,
        request_hash: CommerceRequestHash,
    ) -> Result<HoldMutationOutcome, CommerceServiceError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| store_error("failed to begin hold transaction", error))?;
        let now = Utc::now().to_rfc3339();
        let tenant_id = parse_subject_i64("tenant_id", &command.tenant_id)?;
        let organization_id = org_id_from_option(command.organization_id.as_deref())?;
        let owner_id = parse_subject_i64("owner_user_id", &command.owner_user_id)?;

        if let Some(replayed) = try_replay_hold_mutation(
            &mut tx,
            tenant_id,
            HOLD_CREATE_SCOPE,
            &command.idempotency_key,
            request_hash.as_str(),
        )
        .await?
        {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit hold replay", error))?;
            return Ok(replayed);
        }

        insert_idempotency_scoped(
            &mut tx,
            tenant_id,
            HOLD_CREATE_SCOPE,
            &command.idempotency_key,
            request_hash.as_str(),
            "hold",
            &now,
        )
        .await?;

        let account = load_account_for_hold(
            &mut tx,
            tenant_id,
            organization_id,
            owner_id,
            &command.account_id,
            &command.asset_type,
            command.owner_type.as_deref(),
            command.account_purpose.as_deref(),
            &now,
        )
        .await?;
        let hold_amount = parse_amount_minor(command.amount.as_str())?;
        require_positive_amount(hold_amount, "amount")?;
        validate_account_owner_scope(
            account.organization_id,
            account.owner_id,
            organization_id,
            owner_id,
        )?;
        let available = parse_amount_minor(&account.available_amount)?;
        if available < hold_amount {
            return Err(CommerceServiceError::insufficient_balance(
                "insufficient available balance for hold",
            ));
        }
        if command.asset_type == CommerceAccountAssetType::Points {
            let spendable = crate::store::account_summary::sum_spendable_points_lots_postgres(
                &mut tx, tenant_id, account.id, &now,
            )
            .await?;
            if i128::from(spendable) < hold_amount {
                return Err(CommerceServiceError::insufficient_balance(
                    "insufficient spendable points lots for hold",
                ));
            }
        }
        let frozen = parse_amount_minor(&account.frozen_amount)?;
        let next_available = format_amount_minor(available - hold_amount);
        let next_frozen = format_amount_minor(frozen + hold_amount);
        let account =
            update_account_balances(&mut tx, &account, &next_available, &next_frozen, &now).await?;

        let hold_id = next_entity_id()?;
        let hold_uuid = next_entity_uuid();
        let hold_trace_id = next_entity_uuid();
        let source_id = parse_subject_i64("source_id", &command.source_id)?;
        sqlx::query(
            r#"
            INSERT INTO acct_hold
                (id, uuid, tenant_id, organization_id, account_id, owner_type, owner_id, asset_code,
                 currency_code, amount, settled_amount, released_amount, status, business_type,
                 business_no, source_type, source_id, idempotency_key, request_no, expires_at,
                 version, trace_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::bigint, '0', '0', $11, $12, $13, $14, $15, $16, $17, $18::timestamptz, 0, $19, $20::timestamptz, $21::timestamptz)
            "#,
        )
        .bind(hold_id)
        .bind(&hold_uuid)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(account.id)
        .bind(command.owner_type.as_deref().unwrap_or(OWNER_TYPE_USER))
        .bind(account.owner_id)
        .bind(asset_code_from_type(&command.asset_type))
        .bind(&account.currency_code)
        .bind(command.amount.as_str())
        .bind(HOLD_STATUS_HELD)
        .bind(&command.business_type)
        .bind(&command.business_no)
        .bind(&command.source_type)
        .bind(source_id)
        .bind(&command.idempotency_key)
        .bind(&command.request_no)
        .bind(command.expires_at.as_deref())
        .bind(&hold_trace_id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert account hold", error))?;

        let hold = load_hold_by_uuid(&mut tx, tenant_id, &hold_uuid).await?;
        complete_idempotency_scoped(
            &mut tx,
            tenant_id,
            HOLD_CREATE_SCOPE,
            &command.idempotency_key,
            hold_id,
            &serde_json::json!({ "holdUuid": hold_uuid }).to_string(),
            &now,
        )
        .await?;

        emit_domain_outbox_postgres(
            &mut tx,
            tenant_id,
            account.id,
            OUTBOX_EVENT_TYPE_HOLD_CREATED,
            &command.idempotency_key,
            &serde_json::json!({
                "holdUuid": hold_uuid,
                "accountId": format_i64(account.id),
                "amount": command.amount.as_str(),
                "assetType": command.asset_type.as_str(),
            }),
            &now,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit hold transaction", error))?;

        Ok(HoldMutationOutcome {
            hold,
            account: account.to_wallet_item()?,
            ledger_entry: None,
            replayed: false,
        })
    }

    pub async fn release_account_hold(
        &self,
        command: ReleaseAccountHoldCommand,
        request_hash: CommerceRequestHash,
    ) -> Result<HoldMutationOutcome, CommerceServiceError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| store_error("failed to begin hold release transaction", error))?;
        let now = Utc::now().to_rfc3339();
        let tenant_id = parse_subject_i64("tenant_id", &command.tenant_id)?;

        if let Some(replayed) = try_replay_hold_mutation(
            &mut tx,
            tenant_id,
            HOLD_RELEASE_SCOPE,
            &command.idempotency_key,
            request_hash.as_str(),
        )
        .await?
        {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit hold release replay", error))?;
            return Ok(replayed);
        }

        insert_idempotency_scoped(
            &mut tx,
            tenant_id,
            HOLD_RELEASE_SCOPE,
            &command.idempotency_key,
            request_hash.as_str(),
            "hold",
            &now,
        )
        .await?;

        let hold = load_hold_by_uuid(&mut tx, tenant_id, &command.hold_id).await?;
        if hold.status != "held" {
            return Err(CommerceServiceError::invalid_state(
                "hold is not in held status",
            ));
        }
        ensure_hold_not_expired(hold.expires_at.as_deref(), &now)?;
        let hold_amount = parse_amount_minor(&hold.amount)?;
        let account = load_account_by_internal_id(
            &mut tx,
            tenant_id,
            parse_subject_i64("account_id", &hold.account_id)?,
        )
        .await?;
        let available = parse_amount_minor(&account.available_amount)?;
        let frozen = parse_amount_minor(&account.frozen_amount)?;
        if frozen < hold_amount {
            return Err(CommerceServiceError::invalid_state(
                "hold release exceeds frozen balance",
            ));
        }
        let account = update_account_balances(
            &mut tx,
            &account,
            &format_amount_minor(available + hold_amount),
            &format_amount_minor(frozen - hold_amount),
            &now,
        )
        .await?;

        sqlx::query(
            r#"
            UPDATE acct_hold
            SET status = $1, released_amount = amount, released_at = $2::timestamptz, updated_at = $3::timestamptz, version = version + 1
            WHERE tenant_id = $4 AND uuid = $5 AND status = $6
            "#,
        )
        .bind(HOLD_STATUS_RELEASED)
        .bind(&now)
        .bind(&now)
        .bind(tenant_id)
        .bind(&command.hold_id)
        .bind(HOLD_STATUS_HELD)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to release account hold", error))?;

        let hold = load_hold_by_uuid(&mut tx, tenant_id, &command.hold_id).await?;
        complete_idempotency_scoped(
            &mut tx,
            tenant_id,
            HOLD_RELEASE_SCOPE,
            &command.idempotency_key,
            parse_subject_i64("hold_id", &hold.id)?,
            &serde_json::json!({ "holdUuid": hold.uuid }).to_string(),
            &now,
        )
        .await?;

        emit_domain_outbox_postgres(
            &mut tx,
            tenant_id,
            account.id,
            OUTBOX_EVENT_TYPE_HOLD_RELEASED,
            &command.idempotency_key,
            &serde_json::json!({
                "holdUuid": hold.uuid,
                "accountId": format_i64(account.id),
                "amount": hold.amount,
                "assetType": hold.asset_type,
            }),
            &now,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit hold release transaction", error))?;

        Ok(HoldMutationOutcome {
            hold,
            account: account.to_wallet_item()?,
            ledger_entry: None,
            replayed: false,
        })
    }

    pub async fn settle_account_hold(
        &self,
        command: SettleAccountHoldCommand,
        request_hash: CommerceRequestHash,
    ) -> Result<HoldMutationOutcome, CommerceServiceError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| store_error("failed to begin hold settle transaction", error))?;
        let now = Utc::now().to_rfc3339();
        let tenant_id = parse_subject_i64("tenant_id", &command.tenant_id)?;

        if let Some(replayed) = try_replay_hold_mutation(
            &mut tx,
            tenant_id,
            HOLD_SETTLE_SCOPE,
            &command.idempotency_key,
            request_hash.as_str(),
        )
        .await?
        {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit hold settle replay", error))?;
            return Ok(replayed);
        }

        insert_idempotency_scoped(
            &mut tx,
            tenant_id,
            HOLD_SETTLE_SCOPE,
            &command.idempotency_key,
            request_hash.as_str(),
            "hold",
            &now,
        )
        .await?;

        let hold = load_hold_by_uuid(&mut tx, tenant_id, &command.hold_id).await?;
        if hold.status != "held" {
            return Err(CommerceServiceError::invalid_state(
                "hold is not in held status",
            ));
        }
        ensure_hold_not_expired(hold.expires_at.as_deref(), &now)?;
        let hold_amount = parse_amount_minor(&hold.amount)?;
        let account = load_account_by_internal_id(
            &mut tx,
            tenant_id,
            parse_subject_i64("account_id", &hold.account_id)?,
        )
        .await?;
        let frozen = parse_amount_minor(&account.frozen_amount)?;
        if frozen < hold_amount {
            return Err(CommerceServiceError::invalid_state(
                "hold settle exceeds frozen balance",
            ));
        }
        let account = update_account_balances(
            &mut tx,
            &account,
            &account.available_amount,
            &format_amount_minor(frozen - hold_amount),
            &now,
        )
        .await?;

        let asset_type = asset_type_from_code(&hold.asset_type)?;
        let ledger_entry = append_settlement_ledger(
            &mut tx,
            SettlementLedgerInput {
                account: &account,
                amount: hold_amount,
                asset_type,
                command: &command,
                hold_id: parse_subject_i64("hold_id", &hold.id)?,
                now: &now,
                tenant_id,
            },
        )
        .await?;

        sqlx::query(
            r#"
            UPDATE acct_hold
            SET status = $1, settled_amount = amount, settled_at = $2::timestamptz, updated_at = $3::timestamptz, version = version + 1
            WHERE tenant_id = $4 AND uuid = $5 AND status = $6
            "#,
        )
        .bind(HOLD_STATUS_SETTLED)
        .bind(&now)
        .bind(&now)
        .bind(tenant_id)
        .bind(&command.hold_id)
        .bind(HOLD_STATUS_HELD)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to settle account hold", error))?;

        let hold = load_hold_by_uuid(&mut tx, tenant_id, &command.hold_id).await?;
        complete_idempotency_scoped(
            &mut tx,
            tenant_id,
            HOLD_SETTLE_SCOPE,
            &command.idempotency_key,
            parse_subject_i64("hold_id", &hold.id)?,
            &serde_json::json!({ "holdUuid": hold.uuid, "ledgerEntryUuid": ledger_entry.uuid })
                .to_string(),
            &now,
        )
        .await?;

        emit_domain_outbox_postgres(
            &mut tx,
            tenant_id,
            account.id,
            OUTBOX_EVENT_TYPE_HOLD_SETTLED,
            &command.idempotency_key,
            &serde_json::json!({
                "holdUuid": hold.uuid,
                "accountId": format_i64(account.id),
                "ledgerEntryUuid": ledger_entry.uuid,
                "amount": hold.amount,
                "assetType": hold.asset_type,
            }),
            &now,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit hold settle transaction", error))?;

        Ok(HoldMutationOutcome {
            hold,
            account: account.to_wallet_item()?,
            ledger_entry: Some(ledger_entry),
            replayed: false,
        })
    }

    pub async fn list_account_holds(
        &self,
        query: AccountHoldListQuery,
    ) -> Result<StoreListPage<AccountHoldItem>, CommerceServiceError> {
        let tenant_id = parse_subject_i64("tenant_id", &query.tenant_id)?;
        let organization_id = org_id_from_option(query.organization_id.as_deref())?;
        let owner_id = parse_subject_i64("owner_user_id", &query.owner_user_id)?;
        let account_id = match query.account_id.as_deref() {
            Some(value) if !value.trim().is_empty() => {
                Some(parse_subject_i64("account_id", value)?)
            }
            _ => None,
        };
        let asset_code = query
            .asset_type
            .as_ref()
            .map(asset_code_from_type)
            .map(str::to_owned);
        let status = query.status.as_deref().map(hold_status_to_code);
        let paging = resolve_list_sql_paging(query.page, query.page_size, None)?;

        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT hold.id, hold.uuid, hold.tenant_id, hold.organization_id, hold.account_id,
                   hold.owner_id, hold.asset_code, hold.amount, hold.settled_amount,
                   hold.released_amount, hold.status, hold.business_type, hold.business_no,
                   hold.source_type, hold.source_id, hold.request_no, hold.idempotency_key,
                   hold.expires_at, hold.settled_at, hold.released_at, hold.version,
                   hold.created_at, hold.updated_at,
                   COUNT(*) OVER() AS {LIST_TOTAL_SQL_COLUMN}
            FROM acct_hold hold
            WHERE hold.tenant_id = $1
              AND hold.organization_id = $2
              AND hold.owner_type = $3
              AND hold.owner_id = $4
              AND ($5 IS NULL OR hold.account_id = $6)
              AND ($7 IS NULL OR hold.asset_code = $8)
              AND ($9 IS NULL OR hold.status = $10)
            ORDER BY hold.created_at DESC
            LIMIT $11 OFFSET $12
            "#
        )))
        .bind(tenant_id)
        .bind(organization_id)
        .bind(OWNER_TYPE_USER)
        .bind(owner_id)
        .bind(account_id)
        .bind(account_id)
        .bind(asset_code.as_deref())
        .bind(asset_code.as_deref())
        .bind(status)
        .bind(status)
        .bind(paging.fetch_limit)
        .bind(paging.sql_offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| store_error("failed to list account holds", error))?;

        let total_items = rows
            .first()
            .map(|row| integer_cell(row, LIST_TOTAL_SQL_COLUMN))
            .unwrap_or(0);
        let items: Result<Vec<_>, _> = rows.iter().map(map_hold_row).collect();
        Ok(finalize_list_page(
            items?,
            paging.params.page_size,
            total_items,
        ))
    }

    pub async fn retrieve_account_hold(
        &self,
        query: AccountHoldDetailQuery,
    ) -> Result<Option<AccountHoldItem>, CommerceServiceError> {
        let tenant_id = parse_subject_i64("tenant_id", &query.tenant_id)?;
        let organization_id = org_id_from_option(query.organization_id.as_deref())?;
        let owner_id = parse_subject_i64("owner_user_id", &query.owner_user_id)?;

        let row = sqlx::query(
            r#"
            SELECT hold.id, hold.uuid, hold.tenant_id, hold.organization_id, hold.account_id,
                   hold.owner_id, hold.asset_code, hold.amount, hold.settled_amount,
                   hold.released_amount, hold.status, hold.business_type, hold.business_no,
                   hold.source_type, hold.source_id, hold.request_no, hold.idempotency_key,
                   hold.expires_at, hold.settled_at, hold.released_at, hold.version,
                   hold.created_at, hold.updated_at
            FROM acct_hold hold
            WHERE hold.tenant_id = $1
              AND hold.organization_id = $2
              AND hold.owner_type = $3
              AND hold.owner_id = $4
              AND hold.uuid = $5
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(OWNER_TYPE_USER)
        .bind(owner_id)
        .bind(query.hold_id.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| store_error("failed to retrieve account hold", error))?;

        row.as_ref().map(map_hold_row).transpose()
    }

    pub async fn create_account_transfer(
        &self,
        command: CreateAccountTransferCommand,
        request_hash: CommerceRequestHash,
    ) -> Result<TransferMutationOutcome, CommerceServiceError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| store_error("failed to begin transfer transaction", error))?;
        let now = Utc::now().to_rfc3339();
        let tenant_id = parse_subject_i64("tenant_id", &command.tenant_id)?;
        let organization_id = org_id_from_option(command.organization_id.as_deref())?;
        let from_id = parse_subject_i64("from_account_id", &command.from_account_id)?;
        let to_id = parse_subject_i64("to_account_id", &command.to_account_id)?;
        if from_id == to_id {
            return Err(CommerceServiceError::validation(
                "from_account_id and to_account_id must differ",
            ));
        }

        if let Some(row) = load_idempotency_scoped(
            &mut tx,
            tenant_id,
            TRANSFER_CREATE_SCOPE,
            &command.idempotency_key,
        )
        .await?
        {
            match crate::store::resolve_idempotency_from_row_fields(
                request_hash.as_str(),
                &string_cell(&row, "request_hash"),
                &string_cell(&row, "status"),
                &string_cell(&row, "locked_until"),
                Utc::now(),
            )? {
                crate::store::IdempotencyRecordAction::Replay => {
                    let replayed =
                        load_transfer_replay(&mut tx, tenant_id, &command.idempotency_key).await?;
                    tx.commit()
                        .await
                        .map_err(|error| store_error("failed to commit transfer replay", error))?;
                    return Ok(replayed);
                }
                crate::store::IdempotencyRecordAction::ReclaimLock => {
                    reclaim_idempotency_scoped_public(
                        &mut tx,
                        tenant_id,
                        TRANSFER_CREATE_SCOPE,
                        &command.idempotency_key,
                        request_hash.as_str(),
                        &now,
                    )
                    .await?;
                }
            }
        } else {
            insert_idempotency_scoped(
                &mut tx,
                tenant_id,
                TRANSFER_CREATE_SCOPE,
                &command.idempotency_key,
                request_hash.as_str(),
                "transfer",
                &now,
            )
            .await?;
        }

        let mut from_account = load_account_by_internal_id(&mut tx, tenant_id, from_id).await?;
        let to_account = load_account_by_internal_id(&mut tx, tenant_id, to_id).await?;
        let owner_id = parse_subject_i64("owner_user_id", &command.owner_user_id)?;
        validate_transfer_account_pair(
            from_account.organization_id,
            from_account.owner_id,
            to_account.organization_id,
            owner_id,
        )?;
        if from_account.asset_type != command.asset_type
            || to_account.asset_type != command.asset_type
        {
            return Err(CommerceServiceError::validation(
                "transfer accounts must match command asset_type",
            ));
        }

        let amount = parse_amount_minor(command.amount.as_str())?;
        require_positive_amount(amount, "amount")?;
        let from_available = parse_amount_minor(&from_account.available_amount)?;
        if from_available < amount {
            return Err(CommerceServiceError::insufficient_balance(
                "insufficient available balance for transfer",
            ));
        }

        from_account = update_account_balances(
            &mut tx,
            &from_account,
            &format_amount_minor(from_available - amount),
            &from_account.frozen_amount,
            &now,
        )
        .await?;
        let to_available = parse_amount_minor(&to_account.available_amount)?;
        let to_account = update_account_balances(
            &mut tx,
            &to_account,
            &format_amount_minor(to_available + amount),
            &to_account.frozen_amount,
            &now,
        )
        .await?;

        let journal_id = next_entity_id()?;
        let journal_uuid = next_entity_uuid();
        let trace_id = next_entity_uuid();
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
        .bind(&command.business_type)
        .bind(&command.business_no)
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(TRANSFER_STATUS_COMPLETED)
        .bind(&trace_id)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert transfer journal", error))?;

        let debit_entry = insert_transfer_ledger(
            &mut tx,
            TransferLedgerInput {
                account: &from_account,
                amount,
                command: &command,
                direction: CommerceLedgerDirection::Debit,
                journal_id,
                now: &now,
                organization_id,
                tenant_id,
                trace_id: &trace_id,
            },
        )
        .await?;
        let credit_entry = insert_transfer_ledger(
            &mut tx,
            TransferLedgerInput {
                account: &to_account,
                amount,
                command: &command,
                direction: CommerceLedgerDirection::Credit,
                journal_id,
                now: &now,
                organization_id,
                tenant_id,
                trace_id: &trace_id,
            },
        )
        .await?;

        if command.asset_type == CommerceAccountAssetType::Points {
            let lot_amount = i64::try_from(amount).map_err(|_| {
                CommerceServiceError::validation("points amount exceeds supported lot range")
            })?;
            let debit_ledger_id = parse_subject_i64("ledger_id", &debit_entry.id)?;
            let credit_ledger_id = parse_subject_i64("ledger_id", &credit_entry.id)?;
            crate::postgres_account::apply_points_lot_movement(
                &mut tx,
                crate::postgres_account::PointsLotMovementInput {
                    account_id: from_account.id,
                    amount: lot_amount,
                    direction: CommerceLedgerDirection::Debit,
                    expires_at: None,
                    ledger_id: debit_ledger_id,
                    now: &now,
                    source_type: &command.business_type,
                    tenant_id,
                },
            )
            .await?;
            crate::postgres_account::apply_points_lot_movement(
                &mut tx,
                crate::postgres_account::PointsLotMovementInput {
                    account_id: to_account.id,
                    amount: lot_amount,
                    direction: CommerceLedgerDirection::Credit,
                    expires_at: None,
                    ledger_id: credit_ledger_id,
                    now: &now,
                    source_type: &command.business_type,
                    tenant_id,
                },
            )
            .await?;
        }

        let transfer_id = next_entity_id()?;
        let transfer_uuid = next_entity_uuid();
        sqlx::query(
            r#"
            INSERT INTO acct_transfer
                (id, uuid, tenant_id, organization_id, from_account_id, to_account_id, asset_code,
                 amount, status, business_type, business_no, idempotency_key, request_no,
                 journal_id, trace_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16::timestamptz)
            "#,
        )
        .bind(transfer_id)
        .bind(&transfer_uuid)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(from_account.id)
        .bind(to_account.id)
        .bind(asset_code_from_type(&command.asset_type))
        .bind(command.amount.as_str())
        .bind(TRANSFER_STATUS_COMPLETED)
        .bind(&command.business_type)
        .bind(&command.business_no)
        .bind(&command.idempotency_key)
        .bind(&command.request_no)
        .bind(journal_id)
        .bind(&trace_id)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert account transfer", error))?;

        let transfer = map_transfer_row(TransferRowInput {
            command: &command,
            created_at: &now,
            from_account_id: from_account.id,
            journal_id,
            organization_id,
            tenant_id,
            to_account_id: to_account.id,
            trace_id: &trace_id,
            transfer_id,
            transfer_uuid: &transfer_uuid,
        })?;

        complete_idempotency_scoped(
            &mut tx,
            tenant_id,
            TRANSFER_CREATE_SCOPE,
            &command.idempotency_key,
            transfer_id,
            &serde_json::json!({ "transferUuid": transfer_uuid }).to_string(),
            &now,
        )
        .await?;

        emit_domain_outbox_postgres(
            &mut tx,
            tenant_id,
            from_account.id,
            OUTBOX_EVENT_TYPE_TRANSFER_COMPLETED,
            &command.idempotency_key,
            &serde_json::json!({
                "transferUuid": transfer_uuid,
                "fromAccountId": format_i64(from_account.id),
                "toAccountId": format_i64(to_account.id),
                "amount": command.amount.as_str(),
                "assetType": command.asset_type.as_str(),
            }),
            &now,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit transfer transaction", error))?;

        Ok(TransferMutationOutcome {
            transfer,
            from_account: from_account.to_wallet_item()?,
            to_account: to_account.to_wallet_item()?,
            debit_entry,
            credit_entry,
            replayed: false,
        })
    }

    pub async fn expire_expired_holds(
        &self,
        command: ExpireExpiredHoldsCommand,
        request_hash: CommerceRequestHash,
    ) -> Result<ExpireExpiredHoldsOutcome, CommerceServiceError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| store_error("failed to begin hold expire transaction", error))?;
        let now = Utc::now().to_rfc3339();
        let tenant_id = parse_subject_i64("tenant_id", &command.tenant_id)?;
        let organization_id = org_id_from_option(command.organization_id.as_deref())?;

        if let Some(replayed) = try_replay_hold_expire_postgres(
            &mut tx,
            tenant_id,
            &command.idempotency_key,
            request_hash.as_str(),
        )
        .await?
        {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit hold expire replay", error))?;
            return Ok(replayed);
        }

        insert_idempotency_scoped(
            &mut tx,
            tenant_id,
            HOLD_EXPIRE_SCOPE,
            &command.idempotency_key,
            request_hash.as_str(),
            "hold_expire",
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

        let mut expired_hold_count = 0_i64;
        let mut released_amount_total = 0_i128;
        let mut last_account_id = organization_id;

        loop {
            let rows = sqlx::query(
                r#"
                SELECT hold.uuid, hold.account_id, hold.amount, hold.asset_code
                FROM acct_hold hold
                INNER JOIN acct_account account
                    ON account.id = hold.account_id
                   AND account.tenant_id = hold.tenant_id
                WHERE hold.tenant_id = $1
                  AND account.organization_id = $2
                  AND hold.status = $3
                  AND hold.expires_at IS NOT NULL
                  AND hold.expires_at <= $4::timestamptz
                  AND ($5::bigint IS NULL OR account.owner_id = $5)
                  AND ($6::bigint IS NULL OR hold.account_id = $6)
                ORDER BY hold.expires_at ASC, hold.id ASC
                LIMIT $7
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(HOLD_STATUS_HELD)
            .bind(&now)
            .bind(owner_filter)
            .bind(account_filter)
            .bind(crate::store::EXPIRE_SWEEP_BATCH_SIZE)
            .fetch_all(&mut *tx)
            .await
            .map_err(|error| store_error("failed to load expired holds", error))?;

            if rows.is_empty() {
                break;
            }

            let batch_len = rows.len();
            for row in rows {
                let hold_uuid = string_cell(&row, "uuid");
                let account_id = integer_cell(&row, "account_id");
                let amount = string_cell(&row, "amount");
                let asset_type = asset_type_from_code(&string_cell(&row, "asset_code"))?;
                expire_one_expired_hold_postgres(
                    &mut tx, tenant_id, account_id, &hold_uuid, &amount, asset_type, &now,
                )
                .await?;
                expired_hold_count += 1;
                released_amount_total += parse_amount_minor(&amount)?;
                last_account_id = account_id;
            }

            if batch_len < crate::store::EXPIRE_SWEEP_BATCH_SIZE as usize {
                break;
            }
        }

        let outcome = ExpireExpiredHoldsOutcome {
            accepted: true,
            replayed: false,
            expired_hold_count,
            released_amount_total: released_amount_total.to_string(),
        };

        complete_hold_expire_idempotency_postgres(
            &mut tx,
            tenant_id,
            &command.idempotency_key,
            &outcome,
            &now,
        )
        .await?;

        if expired_hold_count > 0 {
            emit_domain_outbox_postgres(
                &mut tx,
                tenant_id,
                last_account_id,
                OUTBOX_EVENT_TYPE_HOLD_EXPIRED,
                &command.idempotency_key,
                &serde_json::json!({
                    "expiredHoldCount": expired_hold_count,
                    "releasedAmountTotal": outcome.released_amount_total,
                    "organizationId": organization_id,
                    "ownerUserId": command.owner_user_id,
                    "accountId": command.account_id,
                }),
                &now,
            )
            .await?;
        }

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit hold expire transaction", error))?;

        Ok(outcome)
    }
}

async fn expire_one_expired_hold_postgres(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    account_id: i64,
    hold_uuid: &str,
    hold_amount_text: &str,
    _asset_type: CommerceAccountAssetType,
    now: &str,
) -> Result<(), CommerceServiceError> {
    let hold_amount = parse_amount_minor(hold_amount_text)?;
    let account = load_account_by_internal_id(tx, tenant_id, account_id).await?;
    let available = parse_amount_minor(&account.available_amount)?;
    let frozen = parse_amount_minor(&account.frozen_amount)?;
    if frozen < hold_amount {
        return Err(CommerceServiceError::invalid_state(
            "hold expire exceeds frozen balance",
        ));
    }
    update_account_balances(
        tx,
        &account,
        &format_amount_minor(available + hold_amount),
        &format_amount_minor(frozen - hold_amount),
        now,
    )
    .await?;

    sqlx::query(
        r#"
        UPDATE acct_hold
        SET status = $1, released_amount = amount, released_at = $2::timestamptz, updated_at = $3::timestamptz, version = version + 1
        WHERE tenant_id = $4 AND uuid = $5 AND status = $6
        "#,
    )
    .bind(crate::store::HOLD_STATUS_EXPIRED)
    .bind(now)
    .bind(now)
    .bind(tenant_id)
    .bind(hold_uuid)
    .bind(HOLD_STATUS_HELD)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to expire account hold", error))?;
    Ok(())
}

async fn try_replay_hold_expire_postgres(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    idempotency_key: &str,
    request_hash: &str,
) -> Result<Option<ExpireExpiredHoldsOutcome>, CommerceServiceError> {
    let Some(row) =
        load_idempotency_scoped_public(tx, tenant_id, HOLD_EXPIRE_SCOPE, idempotency_key).await?
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
            reclaim_idempotency_scoped_public(
                tx,
                tenant_id,
                HOLD_EXPIRE_SCOPE,
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
        CommerceServiceError::storage(format!(
            "failed to decode hold expire replay snapshot: {error}"
        ))
    })?;
    Ok(Some(ExpireExpiredHoldsOutcome {
        accepted: value
            .get("accepted")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        replayed: true,
        expired_hold_count: value
            .get("expiredHoldCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        released_amount_total: value
            .get("releasedAmountTotal")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_owned(),
    }))
}

async fn complete_hold_expire_idempotency_postgres(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    idempotency_key: &str,
    outcome: &ExpireExpiredHoldsOutcome,
    now: &str,
) -> Result<(), CommerceServiceError> {
    let snapshot = serde_json::json!({
        "accepted": outcome.accepted,
        "replayed": outcome.replayed,
        "expiredHoldCount": outcome.expired_hold_count,
        "releasedAmountTotal": outcome.released_amount_total,
    })
    .to_string();

    sqlx::query(
        r#"
        UPDATE acct_idempotency_record
        SET status = 'COMPLETED', target_id = 0, response_snapshot = $1::jsonb, locked_until = NULL, updated_at = $2::timestamptz
        WHERE tenant_id = $3 AND scope = $4 AND idempotency_key = $5
        "#,
    )
    .bind(&snapshot)
    .bind(now)
    .bind(tenant_id)
    .bind(HOLD_EXPIRE_SCOPE)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to complete hold expire idempotency", error))?;
    Ok(())
}

struct SettlementLedgerInput<'a> {
    account: &'a StoredAccount,
    amount: i128,
    asset_type: CommerceAccountAssetType,
    command: &'a SettleAccountHoldCommand,
    hold_id: i64,
    now: &'a str,
    tenant_id: i64,
}

async fn append_settlement_ledger(
    tx: &mut Transaction<'_, Postgres>,
    input: SettlementLedgerInput<'_>,
) -> Result<WalletTransactionItem, CommerceServiceError> {
    let SettlementLedgerInput {
        account,
        amount,
        asset_type,
        command,
        hold_id,
        now,
        tenant_id,
    } = input;
    let balance_before = account.available_amount.clone();
    let balance_after = account.available_amount.clone();
    let journal_id = next_entity_id()?;
    let journal_uuid = next_entity_uuid();
    let ledger_id = next_entity_id()?;
    let ledger_uuid = next_entity_uuid();
    let trace_id = next_entity_uuid();

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
    .bind(&command.business_type)
    .bind(&command.transaction_no)
    .bind(&command.request_no)
    .bind(&command.idempotency_key)
    .bind(ACCOUNT_STATUS_ACTIVE)
    .bind(&trace_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert settlement journal", error))?;

    sqlx::query(
        r#"
        INSERT INTO acct_ledger_entry
            (id, uuid, tenant_id, organization_id, account_id, journal_id, owner_type, owner_id,
             asset_code, currency_code, ledger_type, entry_type, direction, amount,
             balance_before, balance_after, business_type, business_no, request_no,
             idempotency_key, hold_id, trace_id, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'AVAILABLE', 'DEBIT', 'DEBIT', $11::bigint, $12::bigint, $13::bigint, $14, $15, $16, $17, $18, $19, $20::timestamptz)
        "#,
    )
    .bind(ledger_id)
    .bind(&ledger_uuid)
    .bind(tenant_id)
    .bind(account.organization_id)
    .bind(account.id)
    .bind(journal_id)
    .bind(account.owner_type.as_str())
    .bind(account.owner_id)
    .bind(asset_code_from_type(&asset_type))
    .bind(&account.currency_code)
    .bind(format_amount_minor(amount))
    .bind(&balance_before)
    .bind(&balance_after)
    .bind(&command.business_type)
    .bind(&command.transaction_no)
    .bind(&command.request_no)
    .bind(&command.idempotency_key)
    .bind(hold_id)
    .bind(&trace_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert settlement ledger", error))?;

    if asset_type == CommerceAccountAssetType::Points {
        crate::postgres_account::apply_points_lot_movement(
            tx,
            crate::postgres_account::PointsLotMovementInput {
                account_id: account.id,
                amount: i64::try_from(amount).map_err(|_| {
                    CommerceServiceError::validation("points amount exceeds supported lot range")
                })?,
                direction: CommerceLedgerDirection::Debit,
                expires_at: None,
                ledger_id,
                now,
                source_type: &command.business_type,
                tenant_id,
            },
        )
        .await?;
    }

    WalletTransactionItem::new(
        &format_i64(ledger_id),
        &ledger_uuid,
        &format_i64(account.id),
        &format_i64(tenant_id),
        optional_org_string(account.organization_id).as_deref(),
        &format_i64(account.owner_id),
        asset_type,
        CommerceLedgerDirection::Debit,
        &format_amount_minor(amount),
        &balance_before,
        &balance_after,
        &command.business_type,
        &command.transaction_no,
        &command.request_no,
        &command.idempotency_key,
        now,
    )
}

struct TransferLedgerInput<'a> {
    account: &'a StoredAccount,
    amount: i128,
    command: &'a CreateAccountTransferCommand,
    direction: CommerceLedgerDirection,
    journal_id: i64,
    now: &'a str,
    organization_id: i64,
    tenant_id: i64,
    trace_id: &'a str,
}

async fn insert_transfer_ledger(
    tx: &mut Transaction<'_, Postgres>,
    input: TransferLedgerInput<'_>,
) -> Result<WalletTransactionItem, CommerceServiceError> {
    let TransferLedgerInput {
        account,
        amount,
        command,
        direction,
        journal_id,
        now,
        organization_id,
        tenant_id,
        trace_id,
    } = input;
    let available = parse_amount_minor(&account.available_amount)?;
    let (balance_before, balance_after) = match &direction {
        CommerceLedgerDirection::Debit => {
            let before = available + amount;
            (
                format_amount_minor(before),
                account.available_amount.clone(),
            )
        }
        CommerceLedgerDirection::Credit => {
            let before = available - amount;
            (
                format_amount_minor(before),
                account.available_amount.clone(),
            )
        }
    };
    let ledger_id = next_entity_id()?;
    let ledger_uuid = next_entity_uuid();
    let entry_type = match &direction {
        CommerceLedgerDirection::Credit => "CREDIT",
        CommerceLedgerDirection::Debit => "DEBIT",
    };
    let ledger_business_no = format!("{}:{}", command.business_no, direction.as_str());

    sqlx::query(
        r#"
        INSERT INTO acct_ledger_entry
            (id, uuid, tenant_id, organization_id, account_id, journal_id, owner_type, owner_id,
             asset_code, currency_code, ledger_type, entry_type, direction, amount,
             balance_before, balance_after, business_type, business_no, request_no,
             idempotency_key, trace_id, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'AVAILABLE', $11::bigint, $12::bigint, $13::bigint, $14::bigint, $15, $16, $17, $18, $19, $20, $21::timestamptz)
        "#,
    )
    .bind(ledger_id)
    .bind(&ledger_uuid)
    .bind(tenant_id)
    .bind(organization_id)
    .bind(account.id)
    .bind(journal_id)
    .bind(OWNER_TYPE_USER)
    .bind(account.owner_id)
    .bind(asset_code_from_type(&account.asset_type))
    .bind(&account.currency_code)
    .bind(entry_type)
    .bind(direction.as_str())
    .bind(format_amount_minor(amount))
    .bind(&balance_before)
    .bind(&balance_after)
    .bind(&command.business_type)
    .bind(&ledger_business_no)
    .bind(&command.request_no)
    .bind(format!("{}:{}", command.idempotency_key, direction.as_str()))
    .bind(trace_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert transfer ledger", error))?;

    sqlx::query(
        r#"
        INSERT INTO acct_journal_line
            (id, journal_id, account_id, direction, amount, ledger_id, created_at)
        VALUES ($1, $2, $3, $4, $5::bigint, $6, $7::timestamptz)
        "#,
    )
    .bind(next_entity_id()?)
    .bind(journal_id)
    .bind(account.id)
    .bind(direction.as_str().to_ascii_uppercase())
    .bind(format_amount_minor(amount))
    .bind(ledger_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert transfer journal line", error))?;

    let entry_idempotency = format!("{}:{}", command.idempotency_key, direction.as_str());
    WalletTransactionItem::new(
        &format_i64(ledger_id),
        &ledger_uuid,
        &format_i64(account.id),
        &format_i64(tenant_id),
        optional_org_string(organization_id).as_deref(),
        &format_i64(account.owner_id),
        account.asset_type.clone(),
        direction,
        &format_amount_minor(amount),
        &balance_before,
        &balance_after,
        &command.business_type,
        &ledger_business_no,
        &command.request_no,
        &entry_idempotency,
        now,
    )
}

async fn load_account_for_hold(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    organization_id: i64,
    owner_id: i64,
    account_id: &str,
    asset_type: &CommerceAccountAssetType,
    owner_type: Option<&str>,
    account_purpose: Option<&str>,
    now: &str,
) -> Result<StoredAccount, CommerceServiceError> {
    let trimmed = account_id.trim();
    if !trimmed.is_empty() {
        let account_id = parse_subject_i64("account_id", trimmed)?;
        let account = load_account_by_internal_id(tx, tenant_id, account_id).await?;
        validate_account_owner_scope(
            account.organization_id,
            account.owner_id,
            organization_id,
            owner_id,
        )?;
        return Ok(account);
    }
    load_account_by_owner_asset(
        tx,
        tenant_id,
        organization_id,
        owner_id,
        asset_type,
        owner_type,
        account_purpose,
        now,
    )
    .await
}

async fn load_account_by_owner_asset(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    organization_id: i64,
    owner_id: i64,
    asset_type: &CommerceAccountAssetType,
    owner_type: Option<&str>,
    account_purpose: Option<&str>,
    now: &str,
) -> Result<StoredAccount, CommerceServiceError> {
    if let Some(account) = sqlx::query(
        r#"
        SELECT id, uuid, tenant_id, organization_id, owner_type, owner_id, asset_code, currency_code,
               available_amount, frozen_amount, pending_amount, status, version
        FROM acct_account
        WHERE tenant_id = $1 AND organization_id = $2 AND owner_type = $3 AND owner_id = $4 AND asset_code = $5 AND account_purpose = $6
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(owner_type.unwrap_or(OWNER_TYPE_USER))
    .bind(owner_id)
    .bind(asset_code_from_type(asset_type))
    .bind(account_purpose.unwrap_or(crate::store::ACCOUNT_PURPOSE_GENERAL))
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load account for hold", error))?
    {
        return map_stored_account(&account);
    }

    let account_id = next_entity_id()?;
    let account_uuid = next_entity_uuid();
    let currency_code = crate::store::provision_currency_code(&asset_type);
    sqlx::query(
        r#"
        INSERT INTO acct_account
            (id, uuid, tenant_id, organization_id, owner_type, owner_id, asset_code, currency_code,
             available_amount, frozen_amount, pending_amount, status, version, account_purpose, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, '0', '0', '0', $9, 0, $10, $11::timestamptz, $12::timestamptz)
        "#,
    )
    .bind(account_id)
    .bind(&account_uuid)
    .bind(tenant_id)
    .bind(organization_id)
    .bind(owner_type.unwrap_or(OWNER_TYPE_USER))
    .bind(owner_id)
    .bind(asset_code_from_type(asset_type))
    .bind(currency_code)
    .bind(ACCOUNT_STATUS_ACTIVE)
    .bind(account_purpose.unwrap_or(crate::store::ACCOUNT_PURPOSE_GENERAL))
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to create account for hold", error))?;

    Ok(StoredAccount {
        id: account_id,
        uuid: account_uuid,
        tenant_id,
        organization_id,
        owner_type: owner_type.unwrap_or(OWNER_TYPE_USER).to_string(),
        owner_id,
        asset_type: asset_type.clone(),
        currency_code: currency_code.to_owned(),
        available_amount: "0".to_owned(),
        frozen_amount: "0".to_owned(),
        pending_amount: "0".to_owned(),
        status: ACCOUNT_STATUS_ACTIVE,
        version: 0,
    })
}

async fn load_account_by_internal_id(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    account_id: i64,
) -> Result<StoredAccount, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, uuid, tenant_id, organization_id, owner_type, owner_id, asset_code, currency_code,
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
    .map_err(|error| store_error("failed to load account", error))?
    .ok_or_else(|| CommerceServiceError::not_found("account was not found"))?;
    map_stored_account(&row)
}

async fn update_account_balances(
    tx: &mut Transaction<'_, Postgres>,
    account: &StoredAccount,
    available_amount: &str,
    frozen_amount: &str,
    now: &str,
) -> Result<StoredAccount, CommerceServiceError> {
    let next_version = account.version.checked_add(1).ok_or_else(|| {
        CommerceServiceError::storage("commerce account version increment overflow")
    })?;
    let update = sqlx::query(
        r#"
        UPDATE acct_account
        SET available_amount = $1::bigint, frozen_amount = $2::bigint, version = $3, updated_at = $4::timestamptz
        WHERE id = $5 AND version = $6
        "#,
    )
    .bind(available_amount)
    .bind(frozen_amount)
    .bind(next_version)
    .bind(now)
    .bind(account.id)
    .bind(account.version)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to update account balances", error))?;
    if update.rows_affected() != 1 {
        return Err(CommerceServiceError::conflict(
            "commerce account balance update was not applied atomically",
        ));
    }
    Ok(StoredAccount {
        available_amount: available_amount.to_owned(),
        frozen_amount: frozen_amount.to_owned(),
        version: next_version,
        ..account.clone()
    })
}

async fn load_hold_by_uuid(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    hold_uuid: &str,
) -> Result<AccountHoldItem, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, uuid, tenant_id, organization_id, account_id, owner_id, asset_code, amount,
               settled_amount, released_amount, status, business_type, business_no, source_type,
               source_id, request_no, idempotency_key, expires_at, settled_at, released_at,
               version, created_at, updated_at
        FROM acct_hold
        WHERE tenant_id = $1 AND uuid = $2
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(hold_uuid.trim())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load account hold", error))?
    .ok_or_else(|| CommerceServiceError::not_found("account hold was not found"))?;
    map_hold_row(&row)
}

fn map_hold_row(row: &sqlx::postgres::PgRow) -> Result<AccountHoldItem, CommerceServiceError> {
    Ok(AccountHoldItem {
        id: format_i64(integer_cell(row, "id")),
        uuid: string_cell(row, "uuid"),
        tenant_id: format_i64(integer_cell(row, "tenant_id")),
        organization_id: optional_org_string(integer_cell(row, "organization_id")),
        account_id: format_i64(integer_cell(row, "account_id")),
        owner_user_id: format_i64(integer_cell(row, "owner_id")),
        asset_type: asset_type_from_code(&string_cell(row, "asset_code"))?
            .as_str()
            .to_owned(),
        amount: format_i64(integer_cell(row, "amount")),
        settled_amount: format_i64(integer_cell(row, "settled_amount")),
        released_amount: format_i64(integer_cell(row, "released_amount")),
        status: hold_status_label(integer_cell(row, "status") as i32).to_owned(),
        business_type: string_cell(row, "business_type"),
        business_no: string_cell(row, "business_no"),
        source_type: string_cell(row, "source_type"),
        source_id: format_i64(integer_cell(row, "source_id")),
        request_no: string_cell(row, "request_no"),
        idempotency_key: string_cell(row, "idempotency_key"),
        expires_at: optional_string_cell(row, "expires_at"),
        settled_at: optional_string_cell(row, "settled_at"),
        released_at: optional_string_cell(row, "released_at"),
        version: integer_cell(row, "version"),
        created_at: string_cell(row, "created_at"),
        updated_at: string_cell(row, "updated_at"),
    })
}

struct TransferRowInput<'a> {
    command: &'a CreateAccountTransferCommand,
    created_at: &'a str,
    from_account_id: i64,
    journal_id: i64,
    organization_id: i64,
    tenant_id: i64,
    to_account_id: i64,
    trace_id: &'a str,
    transfer_id: i64,
    transfer_uuid: &'a str,
}

fn map_transfer_row(
    input: TransferRowInput<'_>,
) -> Result<AccountTransferItem, CommerceServiceError> {
    let TransferRowInput {
        command,
        created_at,
        from_account_id,
        journal_id,
        organization_id,
        tenant_id,
        to_account_id,
        trace_id,
        transfer_id,
        transfer_uuid,
    } = input;
    Ok(AccountTransferItem {
        id: format_i64(transfer_id),
        uuid: transfer_uuid.to_owned(),
        tenant_id: format_i64(tenant_id),
        organization_id: optional_org_string(organization_id),
        from_account_id: format_i64(from_account_id),
        to_account_id: format_i64(to_account_id),
        owner_user_id: command.owner_user_id.clone(),
        asset_type: command.asset_type.as_str().to_owned(),
        amount: command.amount.as_str().to_owned(),
        status: "completed".to_owned(),
        business_type: command.business_type.clone(),
        business_no: command.business_no.clone(),
        request_no: command.request_no.clone(),
        idempotency_key: command.idempotency_key.clone(),
        journal_id: format_i64(journal_id),
        trace_id: trace_id.to_owned(),
        created_at: created_at.to_owned(),
    })
}

async fn try_replay_hold_mutation(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    scope: &str,
    idempotency_key: &str,
    request_hash: &str,
) -> Result<Option<HoldMutationOutcome>, CommerceServiceError> {
    let Some(row) = load_idempotency_scoped(tx, tenant_id, scope, idempotency_key).await? else {
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
            reclaim_idempotency_scoped_public(
                tx,
                tenant_id,
                scope,
                idempotency_key,
                request_hash,
                &Utc::now().to_rfc3339(),
            )
            .await?;
            return Ok(None);
        }
        crate::store::IdempotencyRecordAction::Replay => {}
    }

    let snapshot: serde_json::Value = serde_json::from_str(&string_cell(&row, "response_snapshot"))
        .map_err(|error| store_error("failed to parse hold idempotency snapshot", error))?;
    let hold_uuid = snapshot
        .get("holdUuid")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            CommerceServiceError::storage("hold idempotency snapshot missing holdUuid")
        })?;
    let hold = load_hold_by_uuid(tx, tenant_id, hold_uuid).await?;
    let account = load_account_by_internal_id(
        tx,
        tenant_id,
        parse_subject_i64("account_id", &hold.account_id)?,
    )
    .await?;
    Ok(Some(HoldMutationOutcome {
        hold,
        account: account.to_wallet_item()?,
        ledger_entry: None,
        replayed: true,
    }))
}

async fn load_transfer_replay(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    idempotency_key: &str,
) -> Result<TransferMutationOutcome, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, uuid, tenant_id, organization_id, from_account_id, to_account_id, asset_code,
               amount, status, business_type, business_no, request_no, idempotency_key,
               journal_id, trace_id, created_at
        FROM acct_transfer
        WHERE tenant_id = $1 AND idempotency_key = $2
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load transfer replay", error))?
    .ok_or_else(|| CommerceServiceError::not_found("transfer replay target was not found"))?;

    let transfer = AccountTransferItem {
        id: format_i64(integer_cell(&row, "id")),
        uuid: string_cell(&row, "uuid"),
        tenant_id: format_i64(integer_cell(&row, "tenant_id")),
        organization_id: optional_org_string(integer_cell(&row, "organization_id")),
        from_account_id: format_i64(integer_cell(&row, "from_account_id")),
        to_account_id: format_i64(integer_cell(&row, "to_account_id")),
        owner_user_id: String::new(),
        asset_type: asset_type_from_code(&string_cell(&row, "asset_code"))?
            .as_str()
            .to_owned(),
        amount: string_cell(&row, "amount"),
        status: "completed".to_owned(),
        business_type: string_cell(&row, "business_type"),
        business_no: string_cell(&row, "business_no"),
        request_no: string_cell(&row, "request_no"),
        idempotency_key: string_cell(&row, "idempotency_key"),
        journal_id: format_i64(integer_cell(&row, "journal_id")),
        trace_id: string_cell(&row, "trace_id"),
        created_at: string_cell(&row, "created_at"),
    };
    let from_account =
        load_account_by_internal_id(tx, tenant_id, integer_cell(&row, "from_account_id")).await?;
    let to_account =
        load_account_by_internal_id(tx, tenant_id, integer_cell(&row, "to_account_id")).await?;
    Ok(TransferMutationOutcome {
        transfer,
        from_account: from_account.to_wallet_item()?,
        to_account: to_account.to_wallet_item()?,
        debit_entry: load_transfer_ledger_by_journal(
            tx,
            tenant_id,
            integer_cell(&row, "journal_id"),
            from_account.id,
            CommerceLedgerDirection::Debit,
        )
        .await?,
        credit_entry: load_transfer_ledger_by_journal(
            tx,
            tenant_id,
            integer_cell(&row, "journal_id"),
            to_account.id,
            CommerceLedgerDirection::Credit,
        )
        .await?,
        replayed: true,
    })
}

async fn load_transfer_ledger_by_journal(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    journal_id: i64,
    account_id: i64,
    direction: CommerceLedgerDirection,
) -> Result<WalletTransactionItem, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, uuid, account_id, tenant_id, organization_id, owner_id, asset_code,
               direction, amount, balance_before, balance_after, business_type, business_no,
               request_no, idempotency_key, created_at
        FROM acct_ledger_entry
        WHERE tenant_id = $1 AND journal_id = $2 AND account_id = $3 AND direction = $4
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(journal_id)
    .bind(account_id)
    .bind(direction.as_str())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load transfer ledger replay", error))?
    .ok_or_else(|| CommerceServiceError::invalid_state("transfer ledger replay entry missing"))?;

    WalletTransactionItem::new(
        &format_i64(integer_cell(&row, "id")),
        &string_cell(&row, "uuid"),
        &format_i64(integer_cell(&row, "account_id")),
        &format_i64(integer_cell(&row, "tenant_id")),
        optional_org_string(integer_cell(&row, "organization_id")).as_deref(),
        &format_i64(integer_cell(&row, "owner_id")),
        asset_type_from_code(&string_cell(&row, "asset_code"))?,
        parse_direction(&string_cell(&row, "direction"))?,
        &string_cell(&row, "amount"),
        &string_cell(&row, "balance_before"),
        &string_cell(&row, "balance_after"),
        &string_cell(&row, "business_type"),
        &string_cell(&row, "business_no"),
        &string_cell(&row, "request_no"),
        &string_cell(&row, "idempotency_key"),
        &string_cell(&row, "created_at"),
    )
}

fn parse_direction(value: &str) -> Result<CommerceLedgerDirection, CommerceServiceError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "credit" => Ok(CommerceLedgerDirection::Credit),
        "debit" => Ok(CommerceLedgerDirection::Debit),
        _ => Err(CommerceServiceError::validation("direction is invalid")),
    }
}

pub(crate) async fn insert_idempotency_scoped_public(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    scope: &str,
    idempotency_key: &str,
    request_hash: &str,
    target_type: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    insert_idempotency_scoped(
        tx,
        tenant_id,
        scope,
        idempotency_key,
        request_hash,
        target_type,
        now,
    )
    .await
}

pub(crate) async fn reclaim_idempotency_scoped_public(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    scope: &str,
    idempotency_key: &str,
    request_hash: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    let lock_expires = crate::store::idempotency_lock_expires_at_rfc3339(Utc::now());
    let updated = sqlx::query(
        r#"
        UPDATE acct_idempotency_record
        SET status = 'LOCKED', request_hash = $1, locked_until = $2::timestamptz, updated_at = $3::timestamptz
        WHERE tenant_id = $4 AND scope = $5 AND idempotency_key = $6
          AND (status = 'FAILED' OR (status = 'LOCKED' AND locked_until <= $3::timestamptz))
        "#,
    )
    .bind(request_hash)
    .bind(&lock_expires)
    .bind(now)
    .bind(tenant_id)
    .bind(scope)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to reclaim idempotency lock", error))?;

    if updated.rows_affected() == 0 {
        return Err(CommerceServiceError::locked(
            "idempotency request in progress; retry with the same idempotency key",
        ));
    }
    Ok(())
}

pub(crate) async fn load_idempotency_scoped_public(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    scope: &str,
    idempotency_key: &str,
) -> Result<Option<sqlx::postgres::PgRow>, CommerceServiceError> {
    load_idempotency_scoped(tx, tenant_id, scope, idempotency_key).await
}

async fn load_idempotency_scoped(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    scope: &str,
    idempotency_key: &str,
) -> Result<Option<sqlx::postgres::PgRow>, CommerceServiceError> {
    sqlx::query(
        r#"
        SELECT request_hash, status, response_snapshot
        FROM acct_idempotency_record
        WHERE tenant_id = $1 AND scope = $2 AND idempotency_key = $3
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(scope)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load idempotency record", error))
}

async fn insert_idempotency_scoped(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    scope: &str,
    idempotency_key: &str,
    request_hash: &str,
    target_type: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    let lock_expires = crate::store::idempotency_lock_expires_at_rfc3339(Utc::now());
    sqlx::query(
        r#"
        INSERT INTO acct_idempotency_record
            (id, uuid, tenant_id, scope, idempotency_key, request_hash, target_type, status,
             locked_until, expire_at, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'LOCKED', $8::timestamptz, $9::timestamptz, $10::timestamptz, $11::timestamptz)
        "#,
    )
    .bind(next_entity_id()?)
    .bind(next_entity_uuid())
    .bind(tenant_id)
    .bind(scope)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(target_type)
    .bind(&lock_expires)
    .bind(&lock_expires)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| {
        crate::store::map_idempotency_insert_error("failed to insert idempotency lock", error)
    })?;
    Ok(())
}

async fn complete_idempotency_scoped(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    scope: &str,
    idempotency_key: &str,
    target_id: i64,
    response_snapshot: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        UPDATE acct_idempotency_record
        SET status = 'COMPLETED', target_id = $1, response_snapshot = $2::jsonb, locked_until = NULL, updated_at = $3::timestamptz
        WHERE tenant_id = $4 AND scope = $5 AND idempotency_key = $6
        "#,
    )
    .bind(target_id)
    .bind(response_snapshot)
    .bind(now)
    .bind(tenant_id)
    .bind(scope)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to complete idempotency record", error))?;
    Ok(())
}

fn map_stored_account(row: &sqlx::postgres::PgRow) -> Result<StoredAccount, CommerceServiceError> {
    crate::postgres_account::map_stored_account(row)
}

fn hold_status_to_code(status: &str) -> i32 {
    match status.trim().to_ascii_lowercase().as_str() {
        "held" => HOLD_STATUS_HELD,
        "settled" => HOLD_STATUS_SETTLED,
        "released" => HOLD_STATUS_RELEASED,
        "expired" => crate::store::HOLD_STATUS_EXPIRED,
        _ => HOLD_STATUS_HELD,
    }
}

fn string_cell(row: &sqlx::postgres::PgRow, name: &str) -> String {
    row.try_get::<String, _>(name).unwrap_or_default()
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, name: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(name).ok().flatten()
}

fn integer_cell(row: &sqlx::postgres::PgRow, name: &str) -> i64 {
    // `status`-style INT4 columns fail `try_get::<i64>`; fall back to i32 so
    // both INT4 and INT8 cells decode.
    row.try_get::<i32, _>(name)
        .map(i64::from)
        .or_else(|_| row.try_get::<i64, _>(name))
        .unwrap_or_default()
}
