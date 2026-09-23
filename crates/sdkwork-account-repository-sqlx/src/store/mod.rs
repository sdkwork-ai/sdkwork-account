pub mod account_guard;
pub mod account_summary;
pub mod balance;
pub mod billing_projection;
pub mod idempotency;
pub mod outbox;
pub mod outbox_relay;
pub mod pagination;

pub use idempotency::{
    idempotency_lock_expires_at, idempotency_lock_expires_at_rfc3339, map_idempotency_insert_error,
    resolve_idempotency_from_row_fields, resolve_idempotency_record_action,
    IdempotencyRecordAction, IDEMPOTENCY_LOCK_TTL_SECS,
};
pub use pagination::{
    fetch_limit_for_page, finalize_list_page, resolve_list_sql_paging, ListSqlPaging,
};

/// Maximum rows processed per expire-sweep batch to avoid unbounded memory use.
pub const EXPIRE_SWEEP_BATCH_SIZE: i64 = 500;

/// Maximum points lots loaded per FIFO debit batch inside a ledger transaction.
pub const POINTS_LOT_DEBIT_BATCH_SIZE: i64 = 100;

/// Maximum points accounts scanned per reconciliation batch.
pub const POINTS_RECONCILE_BATCH_SIZE: i64 = 100;

/// Default rows returned per outbox dispatch batch for backend relay jobs.
pub const OUTBOX_DISPATCH_BATCH_DEFAULT: i64 = 100;

use chrono::{DateTime, Utc};
use sdkwork_account_service::AppendLedgerEntryCommand;
use sdkwork_contract_service::{CommerceAccountAssetType, CommerceServiceError};
use sdkwork_database_id::SnowflakeIdGenerator;
use sdkwork_utils_rust::{parse_datetime, uuid as new_uuid};
use std::sync::Mutex;

pub const LEDGER_APPEND_SCOPE: &str = "wallet.adjustments.create";
pub const HOLD_CREATE_SCOPE: &str = "wallet.holds.create";
pub const HOLD_SETTLE_SCOPE: &str = "wallet.holds.settle";
pub const HOLD_RELEASE_SCOPE: &str = "wallet.holds.release";
pub const HOLD_EXPIRE_SCOPE: &str = "wallet.holds.expire";
pub const TRANSFER_CREATE_SCOPE: &str = "wallet.transfers.create";
pub const POINTS_LOT_EXPIRE_SCOPE: &str = "wallet.points.lots.expire";
pub const POINTS_LOT_STATUS_DEPLETED: i32 = 2;
pub const POINTS_LOT_STATUS_EXPIRED: i32 = 3;
pub const OWNER_TYPE_USER: &str = "USER";
pub const ACCOUNT_STATUS_ACTIVE: i32 = 1;
pub const ACCOUNT_PURPOSE_GENERAL: &str = "GENERAL";
pub const HOLD_STATUS_HELD: i32 = 1;
pub const HOLD_STATUS_SETTLED: i32 = 2;
pub const HOLD_STATUS_RELEASED: i32 = 3;
pub const HOLD_STATUS_EXPIRED: i32 = 4;
pub const TRANSFER_STATUS_COMPLETED: i32 = 2;

pub fn parse_subject_i64(field_name: &str, value: &str) -> Result<i64, CommerceServiceError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CommerceServiceError::validation(format!(
            "{field_name} is required"
        )));
    }
    trimmed.parse::<i64>().map_err(|_| {
        CommerceServiceError::validation(format!("{field_name} must be a valid int64"))
    })
}

pub fn org_id_from_option(value: Option<&str>) -> Result<i64, CommerceServiceError> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => parse_subject_i64("organization_id", value),
        None => Ok(0),
    }
}

pub fn asset_code_from_type(asset_type: &CommerceAccountAssetType) -> &'static str {
    asset_type.as_str()
}

pub fn asset_type_from_code(value: &str) -> Result<CommerceAccountAssetType, CommerceServiceError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "cash" => Ok(CommerceAccountAssetType::Cash),
        "points" => Ok(CommerceAccountAssetType::Points),
        "token_bank" => Ok(CommerceAccountAssetType::TokenBank),
        _ => Err(CommerceServiceError::validation("asset_code is invalid")),
    }
}

pub fn points_lot_status_label(status: i32) -> &'static str {
    match status {
        1 => "active",
        2 => "depleted",
        3 => "expired",
        _ => "unknown",
    }
}

pub fn default_currency_code(asset_type: &CommerceAccountAssetType) -> &'static str {
    match asset_type {
        CommerceAccountAssetType::Cash => "",
        CommerceAccountAssetType::Points => "POINT",
        CommerceAccountAssetType::TokenBank => "TOKEN_BANK",
    }
}

/// Default fiat currency for cash accounts.
///
/// Platform payment channels (WeChat Pay, Alipay) and the console UI settle
/// in CNY, and the `chk_acct_account_currency` CHECK constraint forbids an
/// empty cash currency code — so provisioning must pick a real code.
pub const DEFAULT_CASH_CURRENCY: &str = "CNY";

/// Currency code used when provisioning an owner's account on first read.
///
/// Unlike [`default_currency_code`], the cash branch is non-empty so the
/// inserted row satisfies the `acct_account` CHECK constraint.
pub fn provision_currency_code(asset_type: &CommerceAccountAssetType) -> &'static str {
    match asset_type {
        CommerceAccountAssetType::Cash => DEFAULT_CASH_CURRENCY,
        CommerceAccountAssetType::Points => "POINT",
        CommerceAccountAssetType::TokenBank => "TOKEN_BANK",
    }
}

pub fn currency_code_for_command(command: &AppendLedgerEntryCommand) -> String {
    command
        .currency_code
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| provision_currency_code(&command.asset_type))
        .to_string()
}

pub fn hold_status_label(status: i32) -> &'static str {
    match status {
        HOLD_STATUS_HELD => "held",
        HOLD_STATUS_SETTLED => "settled",
        HOLD_STATUS_RELEASED => "released",
        HOLD_STATUS_EXPIRED => "expired",
        _ => "unknown",
    }
}

pub fn account_status_label(status: i32) -> &'static str {
    match status {
        1 => "active",
        2 => "frozen",
        3 => "closed",
        _ => "unknown",
    }
}

pub fn store_error(context: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    CommerceServiceError::storage(format!("{context}: {error}"))
}

pub fn parse_wallet_transaction_cursor(
    cursor: Option<&str>,
) -> Result<Option<DateTime<Utc>>, CommerceServiceError> {
    let Some(raw) = cursor.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if let Some(parsed) = parse_datetime(raw, None) {
        return Ok(Some(parsed));
    }
    Err(CommerceServiceError::validation(
        "cursor must be an RFC3339 timestamp",
    ))
}

pub struct AccountIdGenerator {
    snowflake: SnowflakeIdGenerator,
}

impl AccountIdGenerator {
    pub fn new() -> Result<Self, CommerceServiceError> {
        let worker_id = resolve_snowflake_worker_id_from_env();
        SnowflakeIdGenerator::new(worker_id)
            .map(|snowflake| Self { snowflake })
            .map_err(|error| CommerceServiceError::storage(error.to_string()))
    }

    pub fn next_id(&self) -> Result<i64, CommerceServiceError> {
        self.snowflake
            .generate()
            .map_err(|error| CommerceServiceError::storage(error.to_string()))
    }

    pub fn next_uuid(&self) -> String {
        new_uuid()
    }
}

impl Default for AccountIdGenerator {
    fn default() -> Self {
        Self::new().expect("account id generator must initialize")
    }
}

thread_local! {
    static ID_GENERATOR: Mutex<AccountIdGenerator> =
        Mutex::new(AccountIdGenerator::new().expect("account id generator must initialize"));
}

pub fn next_entity_id() -> Result<i64, CommerceServiceError> {
    ID_GENERATOR.with(|generator| generator.lock().expect("id generator lock").next_id())
}

pub fn next_entity_uuid() -> String {
    ID_GENERATOR.with(|generator| generator.lock().expect("id generator lock").next_uuid())
}

fn resolve_snowflake_worker_id_from_env() -> u16 {
    const WORKER_ENV: &str = "ACCOUNT_SNOWFLAKE_WORKER_ID";
    match std::env::var(WORKER_ENV) {
        Ok(raw) => match raw.trim().parse::<u16>() {
            Ok(worker_id) => worker_id,
            Err(error) => {
                tracing::warn!(
                    target = "account.id",
                    env = WORKER_ENV,
                    error = %error,
                    "invalid snowflake worker id; falling back to 0"
                );
                0
            }
        },
        Err(_) => {
            tracing::warn!(
                target = "account.id",
                env = WORKER_ENV,
                "snowflake worker id not configured; defaulting to 0 — set unique ids per instance in production"
            );
            0
        }
    }
}

pub fn format_i64(value: i64) -> String {
    value.to_string()
}

pub fn optional_org_string(value: i64) -> Option<String> {
    if value == 0 {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        asset_code_from_type, asset_type_from_code, currency_code_for_command,
        default_currency_code, provision_currency_code,
    };
    use sdkwork_account_service::AppendLedgerEntryCommand;
    use sdkwork_contract_service::{
        CommerceAccountAssetType, CommerceLedgerDirection, CommerceMoney,
    };

    #[test]
    fn maps_token_bank_asset_without_forbidden_token_aliases() {
        assert_eq!(
            asset_code_from_type(&CommerceAccountAssetType::TokenBank),
            "token_bank"
        );
        assert_eq!(
            asset_type_from_code("token_bank").expect("token bank asset"),
            CommerceAccountAssetType::TokenBank
        );
        assert_eq!(
            default_currency_code(&CommerceAccountAssetType::TokenBank),
            "TOKEN_BANK"
        );
        assert!(asset_type_from_code("token").is_err());
        assert!(asset_type_from_code("tokens").is_err());
    }

    #[test]
    fn provision_currency_satisfies_account_check_constraint() {
        assert_eq!(
            provision_currency_code(&CommerceAccountAssetType::Cash),
            "CNY"
        );
        assert_eq!(
            provision_currency_code(&CommerceAccountAssetType::Points),
            "POINT"
        );
        assert_eq!(
            provision_currency_code(&CommerceAccountAssetType::TokenBank),
            "TOKEN_BANK"
        );
        for asset_type in [
            CommerceAccountAssetType::Cash,
            CommerceAccountAssetType::Points,
            CommerceAccountAssetType::TokenBank,
        ] {
            assert!(!provision_currency_code(&asset_type).is_empty());
        }
    }

    #[test]
    fn command_currency_defaults_to_provision_code_per_asset() {
        let amount = CommerceMoney::new("100").expect("valid amount");
        let command = |asset_type, currency_code| {
            AppendLedgerEntryCommand::with_options(
                "tenant-1",
                None,
                "",
                "owner-1",
                asset_type,
                currency_code,
                CommerceLedgerDirection::Credit,
                amount.clone(),
                "grant",
                "txn-1",
                "req-1",
                "key-1",
                None,
                None,
            )
            .expect("valid append command")
        };
        // Cash without an explicit currency must fall back to the platform
        // default (CNY) instead of an empty string that violates the
        // `chk_acct_account_currency` CHECK constraint on insert.
        assert_eq!(
            currency_code_for_command(&command(CommerceAccountAssetType::Cash, None)),
            "CNY"
        );
        assert_eq!(
            currency_code_for_command(&command(CommerceAccountAssetType::Points, None)),
            "POINT"
        );
        assert_eq!(
            currency_code_for_command(&command(CommerceAccountAssetType::TokenBank, None)),
            "TOKEN_BANK"
        );
        // An explicit currency always wins.
        assert_eq!(
            currency_code_for_command(&command(CommerceAccountAssetType::Cash, Some("USD"))),
            "USD"
        );
    }
}
