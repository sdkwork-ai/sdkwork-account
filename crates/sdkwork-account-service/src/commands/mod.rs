use sdkwork_contract_service::{
    CommerceAccountAssetType, CommerceLedgerDirection, CommerceMoney, CommerceServiceError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendLedgerEntryCommand {
    pub account_id: String,
    pub amount: CommerceMoney,
    pub asset_type: CommerceAccountAssetType,
    pub business_type: String,
    pub currency_code: Option<String>,
    pub direction: CommerceLedgerDirection,
    pub expires_at: Option<String>,
    pub idempotency_key: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub request_no: String,
    pub reversed_ledger_id: Option<String>,
    pub tenant_id: String,
    pub transaction_no: String,
    /// Owner subject kind (defaults to USER when None).
    pub owner_type: Option<String>,
    /// Account purpose (defaults to GENERAL when None).
    pub account_purpose: Option<String>,
}

impl AppendLedgerEntryCommand {
    /// Set the account subject (owner type + account purpose) for non-user
    /// accounts (e.g. PARTNER settlement accounts). None keeps USER/GENERAL.
    pub fn with_account_subject(mut self, owner_type: &str, account_purpose: &str) -> Self {
        self.owner_type = Some(owner_type.to_string());
        self.account_purpose = Some(account_purpose.to_string());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateAccountHoldCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub account_id: String,
    pub asset_type: CommerceAccountAssetType,
    pub amount: CommerceMoney,
    pub business_type: String,
    pub business_no: String,
    pub source_type: String,
    pub source_id: String,
    pub request_no: String,
    pub idempotency_key: String,
    pub expires_at: Option<String>,
    /// Owner subject kind (defaults to USER when None).
    pub owner_type: Option<String>,
    /// Account purpose (defaults to GENERAL when None).
    pub account_purpose: Option<String>,
}

impl CreateAccountHoldCommand {
    /// Set the account subject (owner type + account purpose) for non-user
    /// accounts (e.g. PARTNER settlement accounts). None keeps USER/GENERAL.
    pub fn with_account_subject(mut self, owner_type: &str, account_purpose: &str) -> Self {
        self.owner_type = Some(owner_type.to_string());
        self.account_purpose = Some(account_purpose.to_string());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettleAccountHoldCommand {
    pub tenant_id: String,
    pub hold_id: String,
    pub business_type: String,
    pub transaction_no: String,
    pub request_no: String,
    pub idempotency_key: String,
    /// Actual amount to capture from the hold, in the account's smallest unit.
    /// `None` keeps the historical full-hold capture; `Some` captures the amount
    /// and returns the remainder to available balance atomically.
    pub settled_amount: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseAccountHoldCommand {
    pub tenant_id: String,
    pub hold_id: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateAccountTransferCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub from_account_id: String,
    pub to_account_id: String,
    pub owner_user_id: String,
    pub asset_type: CommerceAccountAssetType,
    pub amount: CommerceMoney,
    pub business_type: String,
    pub business_no: String,
    pub request_no: String,
    pub idempotency_key: String,
}

impl CreateAccountHoldCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        account_id: &str,
        owner_user_id: &str,
        asset_type: CommerceAccountAssetType,
        amount: CommerceMoney,
        business_type: &str,
        business_no: &str,
        source_type: &str,
        source_id: &str,
        request_no: &str,
        idempotency_key: &str,
        expires_at: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        Ok(Self {
            tenant_id: required_text("tenant_id", tenant_id)?,
            organization_id: optional_text(organization_id),
            account_id: optional_account_id(account_id),
            owner_user_id: required_text("owner_user_id", owner_user_id)?,
            asset_type,
            amount,
            business_type: required_text("business_type", business_type)?,
            business_no: required_text("business_no", business_no)?,
            source_type: required_text("source_type", source_type)?,
            source_id: required_text("source_id", source_id)?,
            request_no: required_text("request_no", request_no)?,
            idempotency_key: required_text("idempotency_key", idempotency_key)?,
            expires_at: optional_text(expires_at),
            owner_type: None,
            account_purpose: None,
        })
    }
}

impl SettleAccountHoldCommand {
    pub fn new(
        tenant_id: &str,
        hold_id: &str,
        business_type: &str,
        transaction_no: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Ok(Self {
            tenant_id: required_text("tenant_id", tenant_id)?,
            hold_id: required_text("hold_id", hold_id)?,
            business_type: required_text("business_type", business_type)?,
            transaction_no: required_text("transaction_no", transaction_no)?,
            request_no: required_text("request_no", request_no)?,
            idempotency_key: required_text("idempotency_key", idempotency_key)?,
            settled_amount: None,
        })
    }

    /// Captures only `settled_amount` from the hold and returns the remainder to
    /// the available balance. The repository validates
    /// `0 < settled_amount <= hold.amount` and applies the capture, remainder
    /// release, and ledger write in one transaction.
    pub fn with_settled_amount(
        mut self,
        settled_amount: &str,
    ) -> Result<Self, CommerceServiceError> {
        self.settled_amount = Some(required_text("settled_amount", settled_amount)?);
        Ok(self)
    }
}

impl ReleaseAccountHoldCommand {
    pub fn new(
        tenant_id: &str,
        hold_id: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Ok(Self {
            tenant_id: required_text("tenant_id", tenant_id)?,
            hold_id: required_text("hold_id", hold_id)?,
            request_no: required_text("request_no", request_no)?,
            idempotency_key: required_text("idempotency_key", idempotency_key)?,
        })
    }
}

impl CreateAccountTransferCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        from_account_id: &str,
        to_account_id: &str,
        owner_user_id: &str,
        asset_type: CommerceAccountAssetType,
        amount: CommerceMoney,
        business_type: &str,
        business_no: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Ok(Self {
            tenant_id: required_text("tenant_id", tenant_id)?,
            organization_id: optional_text(organization_id),
            from_account_id: required_text("from_account_id", from_account_id)?,
            to_account_id: required_text("to_account_id", to_account_id)?,
            owner_user_id: required_text("owner_user_id", owner_user_id)?,
            asset_type,
            amount,
            business_type: required_text("business_type", business_type)?,
            business_no: required_text("business_no", business_no)?,
            request_no: required_text("request_no", request_no)?,
            idempotency_key: required_text("idempotency_key", idempotency_key)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpirePointsLotsCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: Option<String>,
    pub account_id: Option<String>,
    pub request_no: String,
    pub idempotency_key: String,
}

impl ExpirePointsLotsCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: Option<&str>,
        account_id: Option<&str>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Ok(Self {
            tenant_id: required_text("tenant_id", tenant_id)?,
            organization_id: optional_text(organization_id),
            owner_user_id: optional_text(owner_user_id),
            account_id: optional_text(account_id),
            request_no: required_text("request_no", request_no)?,
            idempotency_key: required_text("idempotency_key", idempotency_key)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpireExpiredHoldsCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: Option<String>,
    pub account_id: Option<String>,
    pub request_no: String,
    pub idempotency_key: String,
}

impl ExpireExpiredHoldsCommand {
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: Option<&str>,
        account_id: Option<&str>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Ok(Self {
            tenant_id: required_text("tenant_id", tenant_id)?,
            organization_id: optional_text(organization_id),
            owner_user_id: optional_text(owner_user_id),
            account_id: optional_text(account_id),
            request_no: required_text("request_no", request_no)?,
            idempotency_key: required_text("idempotency_key", idempotency_key)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatePreholdCommand {
    pub account_id: String,
    pub amount: CommerceMoney,
    pub idempotency_key: String,
    pub owner_user_id: String,
    pub request_no: String,
    pub tenant_id: String,
}

impl AppendLedgerEntryCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &str,
        organization_id: Option<&str>,
        account_id: &str,
        owner_user_id: &str,
        asset_type: CommerceAccountAssetType,
        currency_code: Option<&str>,
        direction: CommerceLedgerDirection,
        amount: CommerceMoney,
        business_type: &str,
        transaction_no: &str,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<Self, CommerceServiceError> {
        Self::with_options(
            tenant_id,
            organization_id,
            account_id,
            owner_user_id,
            asset_type,
            currency_code,
            direction,
            amount,
            business_type,
            transaction_no,
            request_no,
            idempotency_key,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_options(
        tenant_id: &str,
        organization_id: Option<&str>,
        account_id: &str,
        owner_user_id: &str,
        asset_type: CommerceAccountAssetType,
        currency_code: Option<&str>,
        direction: CommerceLedgerDirection,
        amount: CommerceMoney,
        business_type: &str,
        transaction_no: &str,
        request_no: &str,
        idempotency_key: &str,
        expires_at: Option<&str>,
        reversed_ledger_id: Option<&str>,
    ) -> Result<Self, CommerceServiceError> {
        crate::validation::validate_ledger_business_type(business_type)?;
        Ok(Self {
            account_id: optional_account_id(account_id),
            amount,
            asset_type,
            business_type: required_text("business_type", business_type)?,
            currency_code: optional_text(currency_code),
            direction,
            expires_at: optional_text(expires_at),
            idempotency_key: required_text("idempotency_key", idempotency_key)?,
            organization_id: optional_text(organization_id),
            owner_user_id: required_text("owner_user_id", owner_user_id)?,
            request_no: required_text("request_no", request_no)?,
            reversed_ledger_id: optional_text(reversed_ledger_id),
            tenant_id: required_text("tenant_id", tenant_id)?,
            transaction_no: required_text("transaction_no", transaction_no)?,
            owner_type: None,
            account_purpose: None,
        })
    }
}

fn optional_account_id(value: &str) -> String {
    value.trim().to_string()
}

fn required_text(field_name: &str, value: &str) -> Result<String, CommerceServiceError> {
    crate::validation::require_non_empty(field_name, value)?;
    Ok(value.trim().to_string())
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
