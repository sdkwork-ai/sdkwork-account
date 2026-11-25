mod billing_policy;
pub use billing_policy::*;

use sdkwork_contract_service::{
    CommerceAccountAssetType, CommerceLedgerDirection, CommerceMoney, CommercePoints,
    CommerceServiceError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountBalance<T> {
    pub available: T,
    pub frozen: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountSummary {
    pub cash: AccountBalance<CommerceMoney>,
    pub owner_user_id: String,
    pub points: AccountBalance<CommercePoints>,
    pub tenant_id: String,
    pub token_bank: AccountBalance<CommerceMoney>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StoreListPage<T> {
    pub items: Vec<T>,
    pub total_items: i64,
    pub has_more: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccountSummarySnapshot {
    pub id: String,
    pub name: String,
    pub email: String,
    pub is_verified: bool,
    pub tier: String,
    pub organization: String,
    pub available_points: String,
    pub est_days_remaining: i64,
    pub monthly_points_consumed: String,
    pub consumption_by_service: Vec<AccountConsumptionItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccountConsumptionItem {
    pub name: String,
    pub points_consumed: String,
    pub color: String,
    pub percentage: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerPolicy {
    pub require_append_only: bool,
    pub require_idempotency_key: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerEntryDraft {
    pub account_id: String,
    pub amount: CommerceMoney,
    pub asset_type: CommerceAccountAssetType,
    pub direction: CommerceLedgerDirection,
    pub idempotency_key: String,
    pub owner_user_id: String,
    pub request_no: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalletAccountItem {
    pub id: String,
    pub uuid: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub asset_type: CommerceAccountAssetType,
    pub currency_code: Option<String>,
    pub available_amount: CommerceMoney,
    pub frozen_amount: CommerceMoney,
    pub pending_amount: CommerceMoney,
    pub status: String,
    pub version: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalletOverview {
    pub accounts: Vec<WalletAccountItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointsAccountSnapshot {
    pub account: WalletAccountItem,
    pub active_lot_count: i64,
    pub expiring_points: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointsSummarySnapshot {
    pub account: WalletAccountItem,
    pub available_points: String,
    pub frozen_points: String,
    pub pending_points: String,
    pub total_points: String,
    pub active_lot_count: i64,
    pub expiring_points: i64,
    pub unswept_expired_points: i64,
    pub month_credit_points: i64,
    pub month_debit_points: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointsLotAllocationItem {
    pub id: String,
    pub uuid: String,
    pub ledger_id: String,
    pub lot_id: String,
    pub amount: i64,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpirePointsLotsOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub expired_lot_count: i64,
    pub expired_points_total: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpireExpiredHoldsOutcome {
    pub accepted: bool,
    pub replayed: bool,
    pub expired_hold_count: i64,
    pub released_amount_total: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointsLotMismatchItem {
    pub account_id: String,
    pub available_points: String,
    pub lot_remaining_total: i64,
    pub delta: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointsReconciliationSnapshot {
    pub checked_account_count: i64,
    pub mismatch_count: i64,
    pub mismatches: Vec<PointsLotMismatchItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboxDispatchItem {
    pub id: String,
    pub uuid: String,
    pub tenant_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub event_version: i32,
    pub event_key: String,
    pub payload: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboxDispatchOutcome {
    pub dispatched_count: i64,
    pub pending_lag: i64,
    pub items: Vec<OutboxDispatchItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointsLotItem {
    pub id: String,
    pub uuid: String,
    pub account_id: String,
    pub granted_amount: i64,
    pub remaining_amount: i64,
    pub source_type: String,
    pub source_id: String,
    pub expires_at: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalletTransactionItem {
    pub id: String,
    pub uuid: String,
    pub account_id: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub asset_type: CommerceAccountAssetType,
    pub direction: CommerceLedgerDirection,
    pub amount: CommerceMoney,
    pub balance_before: CommerceMoney,
    pub balance_after: CommerceMoney,
    pub business_type: String,
    pub transaction_no: String,
    pub request_no: String,
    pub idempotency_key: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BillingHistoryItem {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub history_no: String,
    pub history_type: String,
    pub direction: String,
    pub asset_type: String,
    pub amount: CommerceMoney,
    pub currency_code: Option<String>,
    pub points_delta: i64,
    pub status: String,
    pub title: String,
    pub reference_no: Option<String>,
    pub source_type: String,
    pub source_id: String,
    pub related_order_id: Option<String>,
    pub related_order_no: Option<String>,
    pub payment_method: Option<String>,
    pub occurred_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalletOperation {
    pub request_no: String,
    pub transactions: Vec<WalletTransactionItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendLedgerEntryOutcome {
    pub account: WalletAccountItem,
    pub ledger_entry: WalletTransactionItem,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountHoldItem {
    pub id: String,
    pub uuid: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub account_id: String,
    pub owner_user_id: String,
    pub asset_type: String,
    pub amount: String,
    pub settled_amount: String,
    pub released_amount: String,
    pub status: String,
    pub business_type: String,
    pub business_no: String,
    pub source_type: String,
    pub source_id: String,
    pub request_no: String,
    pub idempotency_key: String,
    pub expires_at: Option<String>,
    pub settled_at: Option<String>,
    pub released_at: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountTransferItem {
    pub id: String,
    pub uuid: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub from_account_id: String,
    pub to_account_id: String,
    pub owner_user_id: String,
    pub asset_type: String,
    pub amount: String,
    pub status: String,
    pub business_type: String,
    pub business_no: String,
    pub request_no: String,
    pub idempotency_key: String,
    pub journal_id: String,
    pub trace_id: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoldMutationOutcome {
    pub hold: AccountHoldItem,
    pub account: WalletAccountItem,
    pub ledger_entry: Option<WalletTransactionItem>,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferMutationOutcome {
    pub transfer: AccountTransferItem,
    pub from_account: WalletAccountItem,
    pub to_account: WalletAccountItem,
    pub debit_entry: WalletTransactionItem,
    pub credit_entry: WalletTransactionItem,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreholdStatus {
    Held,
    Settled,
    Released,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreholdTransition {
    from: PreholdStatus,
    to: PreholdStatus,
}

impl<T> AccountBalance<T> {
    pub fn new(available: T, frozen: T) -> Result<Self, CommerceServiceError> {
        Ok(Self { available, frozen })
    }
}

impl AccountSummary {
    pub fn empty(tenant_id: &str, owner_user_id: &str) -> Self {
        Self {
            cash: AccountBalance::new(
                CommerceMoney::new("0").expect("zero money is valid"),
                CommerceMoney::new("0").expect("zero money is valid"),
            )
            .expect("zero balance is valid"),
            owner_user_id: owner_user_id.to_string(),
            points: AccountBalance::new(
                CommercePoints::new("0").expect("zero points are valid"),
                CommercePoints::new("0").expect("zero points are valid"),
            )
            .expect("zero balance is valid"),
            tenant_id: tenant_id.to_string(),
            token_bank: AccountBalance::new(
                CommerceMoney::new("0").expect("zero token bank amount is valid"),
                CommerceMoney::new("0").expect("zero token bank amount is valid"),
            )
            .expect("zero balance is valid"),
        }
    }
}

impl Default for AccountSummarySnapshot {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            email: String::new(),
            is_verified: false,
            tier: "Standard".to_owned(),
            organization: String::new(),
            available_points: "0".to_owned(),
            est_days_remaining: 0,
            monthly_points_consumed: "0".to_owned(),
            consumption_by_service: Vec::new(),
        }
    }
}

impl LedgerPolicy {
    pub fn standard() -> Self {
        Self {
            require_append_only: true,
            require_idempotency_key: true,
        }
    }
}

impl LedgerEntryDraft {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        account_id: &str,
        owner_user_id: &str,
        asset_type: CommerceAccountAssetType,
        direction: CommerceLedgerDirection,
        amount: CommerceMoney,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::require_non_empty("tenant_id", tenant_id)?;
        crate::validation::require_non_empty("account_id", account_id)?;
        crate::validation::require_non_empty("owner_user_id", owner_user_id)?;
        crate::validation::require_non_empty("request_no", request_no)?;
        crate::validation::require_non_empty("idempotency_key", idempotency_key)?;

        Ok(Self {
            account_id: account_id.to_string(),
            amount,
            asset_type,
            direction,
            idempotency_key: idempotency_key.to_string(),
            owner_user_id: owner_user_id.to_string(),
            request_no: request_no.to_string(),
            tenant_id: tenant_id.to_string(),
        })
    }
}

impl WalletAccountItem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        uuid: &str,
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        asset_type: CommerceAccountAssetType,
        currency_code: Option<&str>,
        available_amount: &str,
        frozen_amount: &str,
        pending_amount: &str,
        status: &str,
        version: i64,
    ) -> Result<Self, CommerceServiceError> {
        require_non_empty_service("id", id)?;
        require_non_empty_service("uuid", uuid)?;
        require_non_empty_service("tenant_id", tenant_id)?;
        require_non_empty_service("owner_user_id", owner_user_id)?;
        require_non_empty_service("status", status)?;
        if version < 0 {
            return Err(CommerceServiceError::validation(
                "account version must not be negative",
            ));
        }

        Ok(Self {
            id: id.to_string(),
            uuid: uuid.to_string(),
            tenant_id: tenant_id.to_string(),
            organization_id: normalize_optional_text(organization_id),
            owner_user_id: owner_user_id.to_string(),
            asset_type,
            currency_code: normalize_optional_text(currency_code),
            available_amount: CommerceMoney::new(available_amount)
                .map_err(CommerceServiceError::validation)?,
            frozen_amount: CommerceMoney::new(frozen_amount)
                .map_err(CommerceServiceError::validation)?,
            pending_amount: CommerceMoney::new(pending_amount)
                .map_err(CommerceServiceError::validation)?,
            status: status.to_string(),
            version,
        })
    }
}

impl WalletOverview {
    pub fn new(accounts: Vec<WalletAccountItem>) -> Self {
        Self { accounts }
    }
}

impl WalletTransactionItem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        uuid: &str,
        account_id: &str,
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        asset_type: CommerceAccountAssetType,
        direction: CommerceLedgerDirection,
        amount: &str,
        balance_before: &str,
        balance_after: &str,
        business_type: &str,
        transaction_no: &str,
        request_no: &str,
        idempotency_key: &str,
        created_at: &str,
    ) -> Result<Self, CommerceServiceError> {
        require_non_empty_service("id", id)?;
        require_non_empty_service("uuid", uuid)?;
        require_non_empty_service("account_id", account_id)?;
        require_non_empty_service("tenant_id", tenant_id)?;
        require_non_empty_service("owner_user_id", owner_user_id)?;
        require_non_empty_service("business_type", business_type)?;
        require_non_empty_service("transaction_no", transaction_no)?;
        require_non_empty_service("request_no", request_no)?;
        require_non_empty_service("idempotency_key", idempotency_key)?;
        require_non_empty_service("created_at", created_at)?;

        Ok(Self {
            id: id.to_string(),
            uuid: uuid.to_string(),
            account_id: account_id.to_string(),
            tenant_id: tenant_id.to_string(),
            organization_id: normalize_optional_text(organization_id),
            owner_user_id: owner_user_id.to_string(),
            asset_type,
            direction,
            amount: CommerceMoney::new(amount).map_err(CommerceServiceError::validation)?,
            balance_before: CommerceMoney::new(balance_before)
                .map_err(CommerceServiceError::validation)?,
            balance_after: CommerceMoney::new(balance_after)
                .map_err(CommerceServiceError::validation)?,
            business_type: business_type.to_string(),
            transaction_no: transaction_no.to_string(),
            request_no: request_no.to_string(),
            idempotency_key: idempotency_key.to_string(),
            created_at: created_at.to_string(),
        })
    }
}

impl BillingHistoryItem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        history_no: &str,
        history_type: &str,
        direction: &str,
        asset_type: &str,
        amount: &str,
        currency_code: Option<&str>,
        points_delta: i64,
        status: &str,
        title: &str,
        reference_no: Option<&str>,
        source_type: &str,
        source_id: &str,
        related_order_id: Option<&str>,
        related_order_no: Option<&str>,
        payment_method: Option<&str>,
        occurred_at: &str,
    ) -> Result<Self, CommerceServiceError> {
        require_non_empty_service("id", id)?;
        require_non_empty_service("tenant_id", tenant_id)?;
        require_non_empty_service("owner_user_id", owner_user_id)?;
        require_non_empty_service("history_no", history_no)?;
        require_non_empty_service("history_type", history_type)?;
        require_non_empty_service("direction", direction)?;
        require_non_empty_service("asset_type", asset_type)?;
        require_non_empty_service("status", status)?;
        require_non_empty_service("title", title)?;
        require_non_empty_service("source_type", source_type)?;
        require_non_empty_service("source_id", source_id)?;
        require_non_empty_service("occurred_at", occurred_at)?;

        Ok(Self {
            id: id.to_string(),
            tenant_id: tenant_id.to_string(),
            organization_id: normalize_optional_text(organization_id),
            owner_user_id: owner_user_id.to_string(),
            history_no: history_no.to_string(),
            history_type: history_type.to_string(),
            direction: direction.to_string(),
            asset_type: asset_type.to_string(),
            amount: CommerceMoney::new(amount).map_err(CommerceServiceError::validation)?,
            currency_code: normalize_optional_text(currency_code),
            points_delta,
            status: status.to_string(),
            title: title.to_string(),
            reference_no: normalize_optional_text(reference_no),
            source_type: source_type.to_string(),
            source_id: source_id.to_string(),
            related_order_id: normalize_optional_text(related_order_id),
            related_order_no: normalize_optional_text(related_order_no),
            payment_method: normalize_optional_text(payment_method),
            occurred_at: occurred_at.to_string(),
        })
    }
}

impl WalletOperation {
    pub fn new(
        request_no: &str,
        transactions: Vec<WalletTransactionItem>,
    ) -> Result<Self, CommerceServiceError> {
        require_non_empty_service("request_no", request_no)?;
        Ok(Self {
            request_no: request_no.to_string(),
            transactions,
        })
    }
}

impl AppendLedgerEntryOutcome {
    pub fn executed(account: WalletAccountItem, ledger_entry: WalletTransactionItem) -> Self {
        Self {
            account,
            ledger_entry,
            replayed: false,
        }
    }

    pub fn replayed(account: WalletAccountItem, ledger_entry: WalletTransactionItem) -> Self {
        Self {
            account,
            ledger_entry,
            replayed: true,
        }
    }
}

impl PreholdTransition {
    pub fn new(from: PreholdStatus, to: PreholdStatus) -> Self {
        Self { from, to }
    }

    pub fn validate(&self) -> Result<(), CommerceServiceError> {
        match (&self.from, &self.to) {
            (PreholdStatus::Held, PreholdStatus::Settled)
            | (PreholdStatus::Held, PreholdStatus::Released)
            | (PreholdStatus::Held, PreholdStatus::Expired) => Ok(()),
            _ => Err(CommerceServiceError::invalid_state(
                "invalid prehold transition",
            )),
        }
    }
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn require_non_empty_service(field_name: &str, value: &str) -> Result<(), CommerceServiceError> {
    crate::validation::require_non_empty(field_name, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_account_item_rejects_empty_account_id() {
        let error = WalletAccountItem::new(
            "",
            "uuid-1",
            "100001",
            None,
            "1",
            CommerceAccountAssetType::Points,
            Some("POINT"),
            "0",
            "0",
            "0",
            "active",
            0,
        )
        .expect_err("empty account id must fail");

        assert_eq!(error.code(), "validation");
    }

    #[test]
    fn wallet_transaction_item_requires_request_no_and_idempotency_key() {
        let error = WalletTransactionItem::new(
            "ledger-1",
            "ledger-uuid-1",
            "account-1",
            "100001",
            None,
            "1",
            CommerceAccountAssetType::Points,
            CommerceLedgerDirection::Credit,
            "10",
            "0",
            "10",
            "points_recharge",
            "txn-1",
            "",
            "",
            "2026-05-20T00:00:00Z",
        )
        .expect_err("request number and idempotency key must be required");

        assert_eq!(error.code(), "validation");
    }
}
