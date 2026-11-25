//! Account billing policy storage.
//!
//! The table is the single source of truth for funding rules: which mode a
//! subject pays with, whether a reservation is taken, and how large that
//! reservation may become. Resolution picks the most specific live row for a
//! subject; writes supersede a revision instead of mutating one, so audit and
//! rollback come from the rows themselves.

use sdkwork_account_service::{
    authorize, AccountCapacity, AdmissionDecision, BillingAdmission, BillingAdmissionPort,
    BillingAuthorizationQuery, BillingPolicyItem, BillingPolicyListQuery, BillingPolicyParams,
    BillingPolicyResolvePort, BillingPolicyResolveQuery, SetBillingPolicyStatusCommand,
    StoreListPage, UpsertBillingPolicyCommand,
};
use sdkwork_contract_service::CommerceServiceError;
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::store::{
    finalize_list_page, format_i64, next_entity_id, next_entity_uuid, org_id_from_option,
    parse_subject_i64, resolve_list_sql_paging, store_error, OWNER_TYPE_USER,
};

/// Columns projected by every read, in the order [`billing_policy_row`] reads them.
const POLICY_COLUMNS: &str = r#"
    id, uuid, tenant_id, organization_id, policy_code, name,
    scope_type, scope_id, priority, revision, status,
    charge_mode, settlement_mode,
    CAST(precharge_ratio AS TEXT) AS precharge_ratio,
    output_token_ceiling, input_token_source, hold_ttl_seconds, min_hold_points,
    CAST(max_hold_share AS TEXT) AS max_hold_share,
    admission_mode, overdraft_envelope_points, credit_limit_points,
    allowance_period, allowance_quota, allowance_reset_mode, overage_policy,
    CAST(effective_from AS TEXT) AS effective_from,
    CAST(effective_to AS TEXT) AS effective_to,
    CAST(updated_at AS TEXT) AS updated_at
"#;

#[derive(Debug, Clone)]
pub struct PostgresBillingPolicyStore {
    pool: PgPool,
}

impl PostgresBillingPolicyStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Resolves the live policy for one subject.
    ///
    /// Specificity wins over priority: an API-key policy beats a user policy,
    /// which beats the organization, the tenant and finally the platform default
    /// row. Within one scope the highest `priority`, then the newest `revision`,
    /// then the latest `effective_from` wins, and only rows inside their
    /// effective window are considered.
    ///
    /// `Ok(None)` means no row is configured and the caller applies the
    /// documented platform defaults, which is what an empty table means.
    pub async fn resolve_billing_policy(
        &self,
        query: &BillingPolicyResolveQuery,
    ) -> Result<Option<BillingPolicyItem>, CommerceServiceError> {
        let tenant_id = parse_subject_i64("tenant_id", &query.tenant_id)?;
        let organization_id = org_id_from_option(query.organization_id.as_deref())?;
        let owner_user_id = parse_subject_i64("owner_user_id", &query.owner_user_id)?;
        let api_key_id = match query
            .api_key_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(value) => Some(parse_subject_i64("api_key_id", value)?),
            None => None,
        };

        let sql = format!(
            r#"
            SELECT {POLICY_COLUMNS}
            FROM acct_billing_policy
            WHERE tenant_id = $1
              AND organization_id = $2
              AND deleted_at IS NULL
              AND status = 1
              AND effective_from <= now()
              AND (effective_to IS NULL OR effective_to > now())
              AND (
                    ($3::bigint IS NOT NULL AND scope_type = 4 AND scope_id = $3)
                 OR (scope_type = 3 AND scope_id = $4)
                 OR (scope_type = 2 AND scope_id = $2)
                 OR (scope_type = 1 AND scope_id = $1)
                 OR scope_type = 0
              )
            ORDER BY scope_type DESC, priority DESC, revision DESC, effective_from DESC
            LIMIT 1
            "#
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(api_key_id)
            .bind(owner_user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| store_error("failed to resolve billing policy", error))?;
        row.as_ref().map(billing_policy_row).transpose()
    }

    /// Lists stored policies for the operator surface.
    pub async fn list_billing_policies(
        &self,
        query: &BillingPolicyListQuery,
    ) -> Result<StoreListPage<BillingPolicyItem>, CommerceServiceError> {
        let tenant_id = parse_subject_i64("tenant_id", &query.tenant_id)?;
        let organization_id = org_id_from_option(query.organization_id.as_deref())?;
        let paging = resolve_list_sql_paging(query.page, query.page_size, None)?;

        let sql = format!(
            r#"
            SELECT {POLICY_COLUMNS},
                   COUNT(*) OVER() AS total_rows
            FROM acct_billing_policy
            WHERE tenant_id = $1
              AND organization_id = $2
              AND deleted_at IS NULL
              AND ($3::smallint IS NULL OR scope_type = $3)
              AND ($4::smallint IS NULL OR status = $4)
            ORDER BY scope_type DESC, priority DESC, revision DESC, id DESC
            LIMIT $5 OFFSET $6
            "#
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(query.scope_type)
            .bind(query.status)
            .bind(paging.fetch_limit)
            .bind(paging.sql_offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| store_error("failed to list billing policies", error))?;
        let total_items = rows
            .first()
            .map(|row| row.try_get::<i64, _>("total_rows").unwrap_or(0))
            .unwrap_or(0);
        let items = rows
            .iter()
            .map(billing_policy_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(finalize_list_page(
            items,
            paging.params.page_size,
            total_items.max(0),
        ))
    }

    /// Stores the next revision of one (scope, priority) policy.
    ///
    /// The previous live revision is closed in the same transaction, so the
    /// unique live-row index keeps holding and a reader never sees two live
    /// policies for the same scope.
    pub async fn upsert_billing_policy(
        &self,
        command: &UpsertBillingPolicyCommand,
    ) -> Result<BillingPolicyItem, CommerceServiceError> {
        command.validate()?;
        let tenant_id = parse_subject_i64("tenant_id", &command.tenant_id)?;
        let organization_id = org_id_from_option(command.organization_id.as_deref())?;
        let scope_id = if command.scope_id.trim().is_empty() {
            0
        } else {
            parse_subject_i64("scope_id", &command.scope_id)?
        };
        let actor = match command.actor_user_id.as_deref() {
            Some(value) if !value.trim().is_empty() => {
                Some(parse_subject_i64("actor_user_id", value)?)
            }
            _ => None,
        };
        let params = &command.params;
        let id = next_entity_id()?;
        let uuid = next_entity_uuid();
        let mut tx =
            self.pool.begin().await.map_err(|error| {
                store_error("failed to begin billing policy transaction", error)
            })?;

        sqlx::query(
            r#"
            UPDATE acct_billing_policy
               SET deleted_at = now(), status = 0, deleted_by = $6,
                   updated_at = now(), version = version + 1
             WHERE tenant_id = $1 AND organization_id = $2
               AND scope_type = $3 AND scope_id = $4 AND priority = $5
               AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(i16::from(command.scope_type))
        .bind(scope_id)
        .bind(command.priority)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to close the previous billing policy", error))?;

        let revision: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(revision), 0) + 1
              FROM acct_billing_policy
             WHERE tenant_id = $1 AND organization_id = $2
               AND scope_type = $3 AND scope_id = $4 AND priority = $5
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(i16::from(command.scope_type))
        .bind(scope_id)
        .bind(command.priority)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| store_error("failed to read the next billing policy revision", error))?;

        sqlx::query(
            r#"
            INSERT INTO acct_billing_policy (
                id, uuid, tenant_id, organization_id, data_scope, status, version,
                created_at, updated_at, deleted_at, deleted_by, metadata,
                policy_code, name, scope_type, scope_id, priority, revision,
                charge_mode, settlement_mode, precharge_ratio, output_token_ceiling,
                input_token_source, hold_ttl_seconds, min_hold_points, max_hold_share,
                admission_mode, overdraft_envelope_points, credit_limit_points,
                allowance_period, allowance_quota, allowance_reset_mode, overage_policy,
                effective_from, effective_to
            ) VALUES (
                $1, $2, $3, $4, 0, 1, 0,
                now(), now(), NULL, NULL, '{}'::jsonb,
                $5, $6, $7, $8, $9, $10,
                $11, $12, CAST($13 AS numeric), $14,
                $15, $16, $17, CAST($18 AS numeric),
                $19, $20, $21,
                $22, $23, $24, $25,
                COALESCE(CAST($26 AS timestamptz), now()), CAST($27 AS timestamptz)
            )
            "#,
        )
        .bind(id)
        .bind(&uuid)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(&command.policy_code)
        .bind(&command.name)
        .bind(i16::from(command.scope_type))
        .bind(scope_id)
        .bind(command.priority)
        .bind(revision)
        .bind(params.charge_mode.as_str())
        .bind(params.settlement_mode.as_str())
        .bind(sdkwork_account_service::format_bps(
            params.precharge_ratio_bps,
        ))
        .bind(params.output_token_ceiling)
        .bind("bytes_div4")
        .bind(params.hold_ttl_seconds)
        .bind(params.min_hold_points)
        .bind(sdkwork_account_service::format_bps(
            params.max_hold_share_bps,
        ))
        .bind(params.admission_mode.as_str())
        .bind(params.overdraft_envelope_points)
        .bind(params.credit_limit_points)
        .bind(Option::<String>::None)
        .bind(Option::<i64>::None)
        .bind(Option::<String>::None)
        .bind(params.overage_policy.as_str())
        .bind(command.effective_from.as_deref())
        .bind(command.effective_to.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert the billing policy revision", error))?;

        let row = fetch_policy_by_id(&mut tx, id).await?;
        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit billing policy transaction", error))?;
        row.ok_or_else(|| CommerceServiceError::storage("billing policy vanished after insert"))
    }

    /// Enables or retires one stored revision.
    pub async fn set_billing_policy_status(
        &self,
        command: &SetBillingPolicyStatusCommand,
    ) -> Result<BillingPolicyItem, CommerceServiceError> {
        let tenant_id = parse_subject_i64("tenant_id", &command.tenant_id)?;
        // The policy id is a globally unique snowflake, so tenant + id is enough
        // to address one row while still isolating tenants. The console does not
        // have to know which organization scope the row belongs to in order to
        // enable or retire it.
        let policy_id = parse_subject_i64("policy_id", &command.policy_id)?;
        let mut tx = self.pool.begin().await.map_err(|error| {
            store_error("failed to begin billing policy status transaction", error)
        })?;
        let affected = sqlx::query(
            r#"
            UPDATE acct_billing_policy
               SET status = $3, updated_at = now(), version = version + 1
             WHERE tenant_id = $1 AND id = $2
               AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(policy_id)
        .bind(command.status)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to update the billing policy status", error))?;
        if affected.rows_affected() == 0 {
            return Err(CommerceServiceError::not_found("billing policy not found"));
        }
        let row = fetch_policy_by_id(&mut tx, policy_id).await?;
        tx.commit().await.map_err(|error| {
            store_error("failed to commit billing policy status transaction", error)
        })?;
        row.ok_or_else(|| CommerceServiceError::storage("billing policy vanished after update"))
    }
}

impl BillingPolicyResolvePort for PostgresBillingPolicyStore {
    fn resolve_billing_policy<'a>(
        &'a self,
        query: BillingPolicyResolveQuery,
    ) -> sdkwork_account_service::BillingPolicyResolveFuture<'a> {
        Box::pin(
            async move { PostgresBillingPolicyStore::resolve_billing_policy(self, &query).await },
        )
    }
}

async fn fetch_policy_by_id(
    tx: &mut Transaction<'_, Postgres>,
    id: i64,
) -> Result<Option<BillingPolicyItem>, CommerceServiceError> {
    let sql = format!(
        r#"
        SELECT {POLICY_COLUMNS}
        FROM acct_billing_policy
        WHERE id = $1
        "#
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| store_error("failed to load the billing policy", error))?;
    row.as_ref().map(billing_policy_row).transpose()
}

fn billing_policy_row(
    row: &sqlx::postgres::PgRow,
) -> Result<BillingPolicyItem, CommerceServiceError> {
    Ok(BillingPolicyItem {
        id: format_i64(
            row.try_get::<i64, _>("id")
                .map_err(|error| store_error("billing policy id", error))?,
        ),
        uuid: text_cell(row, "uuid")?,
        tenant_id: format_i64(int_cell(row, "tenant_id")?),
        organization_id: format_i64(int_cell(row, "organization_id")?),
        policy_code: text_cell(row, "policy_code")?,
        name: text_cell(row, "name")?,
        scope_type: smallint_cell(row, "scope_type")?,
        scope_id: format_i64(int_cell(row, "scope_id")?),
        priority: row
            .try_get::<i32, _>("priority")
            .map_err(|error| store_error("billing policy priority", error))?,
        revision: int_cell(row, "revision")?,
        status: smallint_cell(row, "status")?,
        charge_mode: text_cell(row, "charge_mode")?,
        settlement_mode: text_cell(row, "settlement_mode")?,
        precharge_ratio: text_cell(row, "precharge_ratio")?,
        output_token_ceiling: int_cell(row, "output_token_ceiling")?,
        input_token_source: text_cell(row, "input_token_source")?,
        hold_ttl_seconds: int_cell(row, "hold_ttl_seconds")?,
        min_hold_points: int_cell(row, "min_hold_points")?,
        max_hold_share: text_cell(row, "max_hold_share")?,
        admission_mode: text_cell(row, "admission_mode")?,
        overdraft_envelope_points: int_cell(row, "overdraft_envelope_points")?,
        credit_limit_points: int_cell(row, "credit_limit_points")?,
        allowance_period: optional_text_cell(row, "allowance_period")?,
        allowance_quota: optional_int_cell(row, "allowance_quota")?,
        allowance_reset_mode: optional_text_cell(row, "allowance_reset_mode")?,
        overage_policy: text_cell(row, "overage_policy")?,
        effective_from: text_cell(row, "effective_from")?,
        effective_to: optional_text_cell(row, "effective_to")?,
        updated_at: text_cell(row, "updated_at")?,
    })
}

fn int_cell(row: &sqlx::postgres::PgRow, column: &str) -> Result<i64, CommerceServiceError> {
    row.try_get::<i64, _>(column)
        .map_err(|error| store_error("billing policy column", error))
}

fn smallint_cell(row: &sqlx::postgres::PgRow, column: &str) -> Result<i16, CommerceServiceError> {
    row.try_get::<i16, _>(column)
        .map_err(|error| store_error("billing policy column", error))
}

fn text_cell(row: &sqlx::postgres::PgRow, column: &str) -> Result<String, CommerceServiceError> {
    row.try_get::<String, _>(column)
        .map_err(|error| store_error("billing policy column", error))
}

fn optional_text_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<String>, CommerceServiceError> {
    row.try_get::<Option<String>, _>(column)
        .map_err(|error| store_error("billing policy column", error))
}

fn optional_int_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<i64>, CommerceServiceError> {
    row.try_get::<Option<i64>, _>(column)
        .map_err(|error| store_error("billing policy column", error))
}

impl PostgresBillingPolicyStore {
    /// Decides whether one estimated charge may proceed.
    ///
    /// This is the seam the routing runtime calls in-process: the account side
    /// resolves the funding policy, reads the wallet snapshot and applies the
    /// domain admission rule, so the funding rules live in exactly one place.
    ///
    /// Period allowances and trial credits have no funded source yet, so both
    /// snapshots are zero and those two modes fail closed instead of admitting
    /// against funds that do not exist.
    pub async fn evaluate_billing(
        &self,
        query: &BillingAuthorizationQuery,
    ) -> Result<BillingAdmission, CommerceServiceError> {
        let tenant_id = parse_subject_i64("tenant_id", &query.tenant_id)?;
        let organization_id = org_id_from_option(query.organization_id.as_deref())?;
        let owner_user_id = parse_subject_i64("owner_user_id", &query.owner_user_id)?;
        let policy = self
            .resolve_billing_policy(&BillingPolicyResolveQuery {
                tenant_id: query.tenant_id.clone(),
                organization_id: query.organization_id.clone(),
                owner_user_id: query.owner_user_id.clone(),
                api_key_id: query.api_key_id.clone(),
            })
            .await?;
        let params = match policy.as_ref() {
            Some(item) => item.params()?,
            None => BillingPolicyParams::default(),
        };
        let owner_type = match query
            .wallet_owner_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(value) => value.to_owned(),
            None => OWNER_TYPE_USER.to_owned(),
        };
        let owner_id = match query
            .wallet_owner_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(value) => parse_subject_i64("wallet_owner_id", value)?,
            None => owner_user_id,
        };
        let wallet = sqlx::query(
            r#"
            SELECT available_amount, frozen_amount
            FROM acct_account
            WHERE tenant_id = $1 AND organization_id = $2 AND owner_type = $3
              AND owner_id = $4 AND asset_code = 'token_bank'
              AND account_purpose = 'GENERAL'
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(&owner_type)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| store_error("failed to read the wallet snapshot", error))?;
        let (available_points, frozen_points) = match wallet {
            Some(row) => (
                row.try_get::<i64, _>("available_amount")
                    .map_err(|error| store_error("wallet available amount", error))?,
                row.try_get::<i64, _>("frozen_amount")
                    .map_err(|error| store_error("wallet frozen amount", error))?,
            ),
            None => (0, 0),
        };
        let capacity = AccountCapacity {
            available_points: available_points.max(0),
            frozen_points: frozen_points.max(0),
            // LIMITATION: the configured credit line is treated as fully
            // available on every request because there is no outstanding-credit
            // ledger to subtract from yet. Credit modes therefore stay disabled
            // (the column defaults to 0, which fails closed) until an
            // outstanding-credit source exists; wiring a limit without it would
            // let N concurrent requests each borrow the whole line.
            credit_remaining_points: params.credit_limit_points,
            // No allowance lot or trial pool is a funding source yet: both stay
            // zero so those modes fail closed instead of admitting unbacked money.
            allowance_remaining_points: 0,
            trial_remaining_points: 0,
        };
        let decision = authorize(&params, capacity, query.estimated_points);
        let hold_points = match decision {
            AdmissionDecision::Admit { hold_points } => hold_points,
            AdmissionDecision::AdmitDeferred | AdmissionDecision::AdmitFree => 0,
            AdmissionDecision::Reject { .. } => 0,
        };
        Ok(BillingAdmission {
            decision,
            policy_id: policy.map(|item| item.id),
            hold_points,
        })
    }
}

impl BillingAdmissionPort for PostgresBillingPolicyStore {
    fn authorize_billing<'a>(
        &'a self,
        query: BillingAuthorizationQuery,
    ) -> sdkwork_account_service::BillingAdmissionFuture<'a> {
        Box::pin(async move { PostgresBillingPolicyStore::evaluate_billing(self, &query).await })
    }
}
