#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Environment {
    Development,
    Test,
    Staging,
    Production,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeploymentMode {
    Saas,
    Local,
    Private,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommerceSurfaceProfile {
    App,
    Console,
    Admin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommerceRuntimeContext {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub user_id: String,
    pub session_id: String,
    pub app_id: String,
    pub deployment_mode: DeploymentMode,
    pub environment: Environment,
    pub surface_profile: CommerceSurfaceProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommerceRuntimeContextInput {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub user_id: String,
    pub session_id: String,
    pub app_id: String,
    pub deployment_mode: DeploymentMode,
    pub environment: Environment,
    pub surface_profile: CommerceSurfaceProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommerceAccountAssetType {
    Cash,
    Points,
    TokenBank,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommerceLedgerDirection {
    Credit,
    Debit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PromotionCouponStatus {
    Draft,
    Active,
    Redeemed,
    Expired,
    Disabled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommerceRechargeStatus {
    Pending,
    Paid,
    Fulfilled,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommercePaymentStatus {
    Pending,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommerceExchangeStatus {
    Pending,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommerceMoney(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercePoints(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommerceRequestHash(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdempotencyStatus {
    Locked,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdempotencyDecision {
    Execute,
    Replay,
    Conflict,
    InProgress,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdempotencyRepositoryCommand {
    Find,
    Lock,
    Complete,
    Fail,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommerceIdempotencyRecord {
    pub tenant_id: String,
    pub scope: String,
    pub idempotency_key: String,
    pub request_hash: CommerceRequestHash,
    pub response_json: Option<String>,
    pub status: IdempotencyStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityFlag {
    name: String,
    enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommerceServiceContract {
    pub domain: &'static str,
    pub service_name: &'static str,
    pub write_commands: Vec<&'static str>,
    pub read_queries: Vec<&'static str>,
    pub ports: Vec<&'static str>,
    pub requires_idempotency_for_writes: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationExecutionPolicy {
    ReadOnly,
    TransactionalWrite,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommerceOperationContract {
    pub operation_id: &'static str,
    pub service_name: &'static str,
    pub execution_policy: OperationExecutionPolicy,
    pub capability_name: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommerceOperationRequest {
    context: CommerceRuntimeContext,
    contract: CommerceOperationContract,
    idempotency_key: Option<String>,
    request_hash: Option<CommerceRequestHash>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionBoundaryKind {
    Required,
    NotRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommerceTransactionScope {
    operation_id: String,
    service_name: String,
    boundary_kind: TransactionBoundaryKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommerceServiceErrorKind {
    Unauthenticated,
    Unauthorized,
    NotFound,
    Conflict,
    Locked,
    InvalidState,
    /// The account's available balance cannot cover the requested hold,
    /// precharge, or transfer. This is a distinct kind (rather than generic
    /// `InvalidState`) because it is the only commerce rejection a *caller* can
    /// self-heal: the wallet owner must fund the account. Downstream surfaces
    /// (cloud router gateway, agents clients) map this kind to an actionable
    /// "recharge" problem instead of a generic upstream failure, so the
    /// classification must survive the error boundary by type, never by
    /// matching the human-readable message.
    InsufficientBalance,
    Validation,
    Transport,
    UnsupportedCapability,
    ProviderUnavailable,
    Storage,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommerceServiceError {
    kind: CommerceServiceErrorKind,
    message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommerceStatusMachine {
    Order,
    Payment,
    Invoice,
}

impl CommerceRuntimeContext {
    pub fn new(input: CommerceRuntimeContextInput) -> Self {
        Self {
            tenant_id: input.tenant_id,
            organization_id: input.organization_id,
            user_id: input.user_id,
            session_id: input.session_id,
            app_id: input.app_id,
            deployment_mode: input.deployment_mode,
            environment: input.environment,
            surface_profile: input.surface_profile,
        }
    }
}

impl CommerceSurfaceProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Console => "console",
            Self::Admin => "admin",
        }
    }
}

impl CommerceAccountAssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cash => "cash",
            Self::Points => "points",
            Self::TokenBank => "token_bank",
        }
    }
}

impl CommerceLedgerDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Credit => "credit",
            Self::Debit => "debit",
        }
    }
}

impl PromotionCouponStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Redeemed => "redeemed",
            Self::Expired => "expired",
            Self::Disabled => "disabled",
        }
    }
}

impl CommerceRechargeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Paid => "paid",
            Self::Fulfilled => "fulfilled",
            Self::Closed => "closed",
        }
    }
}

impl CommercePaymentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }
}

impl CommerceExchangeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

/// Shared amount precision used across commerce money/points values.
/// Token Bank points are carried with up to 6 decimal places so fractional
/// usage debits and balances stay precise; settlement rounds up at this scale
/// so the merchant never underpays (no merchant loss).
pub struct CommercePrecision;

impl CommercePrecision {
    pub const POINTS_SCALE: usize = 6;
}

/// Validates an amount string as a non-negative decimal with at most
/// `max_scale` fractional digits (e.g. `12` or `12.000001`).
pub fn is_non_negative_amount(value: &str, max_scale: usize) -> bool {
    let value = value.trim();
    if value.is_empty() || value.starts_with('-') || value.starts_with('+') {
        return false;
    }

    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some() {
        // More than one '.' present.
        return false;
    }

    let whole_ok = whole.is_empty()
        || (whole.chars().all(|ch| ch.is_ascii_digit())
            && (whole.len() == 1 || !whole.starts_with('0')));

    let fraction_ok = match fraction {
        Some(fraction) => {
            !value.ends_with('.')
                && !fraction.is_empty()
                && fraction.chars().all(|ch| ch.is_ascii_digit())
                && fraction.len() <= max_scale
        }
        None => true,
    };

    whole_ok && fraction_ok
}

impl CommerceMoney {
    pub fn new(value: &str) -> Result<Self, &'static str> {
        if is_non_negative_amount(value, CommercePrecision::POINTS_SCALE) {
            Ok(Self(value.to_string()))
        } else {
            Err("money amount must be a non-negative number with up to 6 decimal places")
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl CommercePoints {
    pub fn new(value: &str) -> Result<Self, &'static str> {
        if is_non_negative_amount(value, CommercePrecision::POINTS_SCALE) {
            Ok(Self(value.to_string()))
        } else {
            Err("points amount must be a non-negative number with up to 6 decimal places")
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl CommerceRequestHash {
    pub fn new(value: &str) -> Result<Self, CommerceServiceError> {
        let normalized = value.trim();
        if normalized.is_empty() {
            return Err(CommerceServiceError::validation("request_hash is required"));
        }

        Ok(Self(normalized.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl CommerceIdempotencyRecord {
    pub fn locked(
        tenant_id: impl Into<String>,
        scope: impl Into<String>,
        idempotency_key: impl Into<String>,
        request_hash: CommerceRequestHash,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            scope: scope.into(),
            idempotency_key: idempotency_key.into(),
            request_hash,
            response_json: None,
            status: IdempotencyStatus::Locked,
        }
    }

    pub fn completed(
        tenant_id: impl Into<String>,
        scope: impl Into<String>,
        idempotency_key: impl Into<String>,
        request_hash: CommerceRequestHash,
        response_json: impl Into<String>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            scope: scope.into(),
            idempotency_key: idempotency_key.into(),
            request_hash,
            response_json: Some(response_json.into()),
            status: IdempotencyStatus::Completed,
        }
    }

    pub fn mark_completed(mut self, response_json: impl Into<String>) -> Self {
        self.response_json = Some(response_json.into());
        self.status = IdempotencyStatus::Completed;
        self
    }

    pub fn mark_failed(mut self) -> Self {
        self.status = IdempotencyStatus::Failed;
        self
    }

    pub fn decide(&self, request_hash: &CommerceRequestHash) -> IdempotencyDecision {
        if self.request_hash != *request_hash {
            return IdempotencyDecision::Conflict;
        }

        match self.status {
            IdempotencyStatus::Completed => IdempotencyDecision::Replay,
            IdempotencyStatus::Locked => IdempotencyDecision::InProgress,
            IdempotencyStatus::Failed => IdempotencyDecision::Execute,
        }
    }
}

impl IdempotencyRepositoryCommand {
    pub fn standard_commands() -> Vec<Self> {
        vec![Self::Find, Self::Lock, Self::Complete, Self::Fail]
    }
}

impl CapabilityFlag {
    pub fn new(name: &str, enabled: bool) -> Result<Self, CommerceServiceError> {
        let normalized = name.trim();
        if normalized.is_empty() {
            return Err(CommerceServiceError::validation(
                "capability name is required",
            ));
        }

        Ok(Self {
            name: normalized.to_string(),
            enabled,
        })
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl CommerceServiceContract {
    pub fn new(
        domain: &'static str,
        service_name: &'static str,
        write_commands: Vec<&'static str>,
        read_queries: Vec<&'static str>,
        ports: Vec<&'static str>,
        requires_idempotency_for_writes: bool,
    ) -> Self {
        Self {
            domain,
            service_name,
            write_commands,
            read_queries,
            ports,
            requires_idempotency_for_writes,
        }
    }

    pub fn validate(&self) -> Result<(), CommerceServiceError> {
        if self.domain.trim().is_empty() {
            return Err(CommerceServiceError::validation(
                "service domain is required",
            ));
        }
        if !self.service_name.starts_with("commerce.") {
            return Err(CommerceServiceError::validation(
                "service_name must start with commerce.",
            ));
        }
        if self.write_commands.is_empty() {
            return Err(CommerceServiceError::validation(
                "service contract requires write commands",
            ));
        }
        if self.read_queries.is_empty() {
            return Err(CommerceServiceError::validation(
                "service contract requires read queries",
            ));
        }
        if self.ports.is_empty() {
            return Err(CommerceServiceError::validation(
                "service contract requires ports",
            ));
        }
        if !self.requires_idempotency_for_writes {
            return Err(CommerceServiceError::validation(
                "commerce write commands require idempotency",
            ));
        }

        Ok(())
    }
}

impl OperationExecutionPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::TransactionalWrite => "transactional-write",
        }
    }

    pub fn requires_transaction(&self) -> bool {
        matches!(self, Self::TransactionalWrite)
    }

    pub fn requires_idempotency(&self) -> bool {
        matches!(self, Self::TransactionalWrite)
    }
}

impl CommerceOperationContract {
    pub const fn new(
        operation_id: &'static str,
        service_name: &'static str,
        execution_policy: OperationExecutionPolicy,
        capability_name: &'static str,
    ) -> Self {
        Self {
            operation_id,
            service_name,
            execution_policy,
            capability_name,
        }
    }

    pub fn validate(&self) -> Result<(), CommerceServiceError> {
        if self.operation_id.trim().is_empty() {
            return Err(CommerceServiceError::validation("operation_id is required"));
        }
        if !self.service_name.starts_with("commerce.") {
            return Err(CommerceServiceError::validation(
                "operation service_name must start with commerce.",
            ));
        }
        if self.capability_name.trim().is_empty() {
            return Err(CommerceServiceError::validation(
                "capability_name is required",
            ));
        }
        if !self.capability_name.starts_with("commerce.") {
            return Err(CommerceServiceError::validation(
                "capability_name must start with commerce.",
            ));
        }

        Ok(())
    }

    pub fn requires_transaction(&self) -> bool {
        self.execution_policy.requires_transaction()
    }

    pub fn requires_idempotency(&self) -> bool {
        self.execution_policy.requires_idempotency()
    }

    pub fn ensure_capability_enabled(
        &self,
        capabilities: &[CapabilityFlag],
    ) -> Result<(), CommerceServiceError> {
        if capabilities
            .iter()
            .any(|capability| capability.name() == self.capability_name && capability.enabled())
        {
            Ok(())
        } else {
            Err(CommerceServiceError::unsupported_capability(
                "required commerce capability is disabled",
            ))
        }
    }
}

impl CommerceOperationRequest {
    pub fn new(
        context: CommerceRuntimeContext,
        contract: CommerceOperationContract,
        idempotency_key: Option<&str>,
        request_hash: Option<CommerceRequestHash>,
    ) -> Self {
        Self {
            context,
            contract,
            idempotency_key: idempotency_key.map(str::to_string),
            request_hash,
        }
    }

    pub fn context(&self) -> &CommerceRuntimeContext {
        &self.context
    }

    pub fn contract(&self) -> &CommerceOperationContract {
        &self.contract
    }

    pub fn idempotency_key(&self) -> Option<&str> {
        self.idempotency_key.as_deref()
    }

    pub fn request_hash(&self) -> Option<&CommerceRequestHash> {
        self.request_hash.as_ref()
    }

    pub fn idempotency_scope(&self) -> &str {
        self.contract.operation_id
    }

    pub fn validate_for_execution(&self) -> Result<(), CommerceServiceError> {
        self.contract.validate()?;
        validate_commerce_context(&self.context).map_err(CommerceServiceError::validation)?;

        if self.contract.requires_idempotency() {
            match self.idempotency_key.as_deref().map(str::trim) {
                Some(value) if !value.is_empty() => {}
                _ => {
                    return Err(CommerceServiceError::validation(
                        "idempotency_key is required for write operation",
                    ));
                }
            }

            if self.request_hash.is_none() {
                return Err(CommerceServiceError::validation(
                    "request_hash is required for write operation",
                ));
            }
        }

        Ok(())
    }
}

impl TransactionBoundaryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::NotRequired => "not-required",
        }
    }
}

impl CommerceTransactionScope {
    pub fn new(
        operation_id: impl Into<String>,
        service_name: impl Into<String>,
        boundary_kind: TransactionBoundaryKind,
    ) -> Result<Self, CommerceServiceError> {
        let operation_id = operation_id.into();
        let service_name = service_name.into();

        if operation_id.trim().is_empty() {
            return Err(CommerceServiceError::validation("operation_id is required"));
        }
        if !service_name.starts_with("commerce.") {
            return Err(CommerceServiceError::validation(
                "service_name must start with commerce.",
            ));
        }

        Ok(Self {
            operation_id,
            service_name,
            boundary_kind,
        })
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    pub fn boundary_kind(&self) -> &TransactionBoundaryKind {
        &self.boundary_kind
    }

    pub fn requires_transaction(&self) -> bool {
        matches!(self.boundary_kind, TransactionBoundaryKind::Required)
    }
}

impl CommerceServiceError {
    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::Unauthenticated, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::Unauthorized, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::NotFound, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::Conflict, message)
    }

    pub fn locked(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::Locked, message)
    }

    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::InvalidState, message)
    }

    /// Builds an insufficient-balance rejection. Callers that need to render a
    /// funding action (recharge/membership entry point) must match on
    /// [`CommerceServiceErrorKind::InsufficientBalance`] via [`Self::kind`]
    /// rather than on the message text.
    pub fn insufficient_balance(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::InsufficientBalance, message)
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::Validation, message)
    }

    pub fn transport(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::Transport, message)
    }

    pub fn unsupported_capability(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::UnsupportedCapability, message)
    }

    pub fn provider_unavailable(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::ProviderUnavailable, message)
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::Storage, message)
    }

    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new(CommerceServiceErrorKind::Unknown, message)
    }

    pub fn code(&self) -> &'static str {
        match self.kind {
            CommerceServiceErrorKind::Unauthenticated => "unauthenticated",
            CommerceServiceErrorKind::Unauthorized => "unauthorized",
            CommerceServiceErrorKind::NotFound => "not-found",
            CommerceServiceErrorKind::Conflict => "conflict",
            CommerceServiceErrorKind::Locked => "locked",
            CommerceServiceErrorKind::InvalidState => "invalid-state",
            CommerceServiceErrorKind::InsufficientBalance => "insufficient-balance",
            CommerceServiceErrorKind::Validation => "validation",
            CommerceServiceErrorKind::Transport => "transport",
            CommerceServiceErrorKind::UnsupportedCapability => "unsupported-capability",
            CommerceServiceErrorKind::ProviderUnavailable => "provider-unavailable",
            CommerceServiceErrorKind::Storage => "storage",
            CommerceServiceErrorKind::Unknown => "unknown",
        }
    }

    /// Exposes the error kind so downstream surfaces can classify a commerce
    /// rejection by type instead of by message text. The insufficient-balance
    /// path relies on this: a wallet rejection that can be fixed by funding the
    /// account must stay distinguishable from every other state error.
    pub fn kind(&self) -> &CommerceServiceErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    fn new(kind: CommerceServiceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

pub fn assert_status_transition(
    machine: CommerceStatusMachine,
    from: &str,
    to: &str,
) -> Result<(), CommerceServiceError> {
    let allowed = match machine {
        CommerceStatusMachine::Order => matches!(
            (from, to),
            ("draft", "pending_payment")
                | ("pending_payment", "paid")
                | ("paid", "fulfilled")
                | ("fulfilled", "completed")
                | ("pending_payment", "cancelled")
                | ("pending_payment", "expired")
                | ("paid", "refunding")
                | ("refunding", "refunded")
        ),
        CommerceStatusMachine::Payment => matches!(
            (from, to),
            ("created", "pending")
                | ("pending", "succeeded")
                | ("pending", "failed")
                | ("pending", "closed")
                | ("succeeded", "refunding")
                | ("refunding", "refunded")
        ),
        CommerceStatusMachine::Invoice => matches!(
            (from, to),
            ("draft", "submitted")
                | ("submitted", "reviewing")
                | ("reviewing", "issued")
                | ("submitted", "cancelled")
                | ("reviewing", "rejected")
                | ("issued", "voided")
        ),
    };

    if allowed {
        Ok(())
    } else {
        Err(CommerceServiceError::invalid_state(match machine {
            CommerceStatusMachine::Order => "invalid order status transition",
            CommerceStatusMachine::Payment => "invalid payment status transition",
            CommerceStatusMachine::Invoice => "invalid invoice status transition",
        }))
    }
}

pub fn validate_commerce_context(context: &CommerceRuntimeContext) -> Result<(), &'static str> {
    if context.tenant_id.trim().is_empty() {
        return Err("tenant_id is required");
    }

    if context.user_id.trim().is_empty() {
        return Err("user_id is required");
    }

    if context.session_id.trim().is_empty() {
        return Err("session_id is required");
    }

    if context.app_id.trim().is_empty() {
        return Err("app_id is required");
    }

    Ok(())
}

/// Canonical commerce ledger `business_type` values for account wallet mutations.
pub struct CommerceLedgerBusinessType;

impl CommerceLedgerBusinessType {
    pub const POINTS_RECHARGE: &str = "points_recharge";
    pub const POINTS_EARN: &str = "points_earn";
    pub const POINTS_BURN: &str = "points_burn";
    pub const POINTS_EXPIRE: &str = "points_expire";
    pub const POINTS_REFUND: &str = "points_refund";
    pub const POINTS_CLAWBACK: &str = "points_cloudback";
    pub const POINTS_TRANSFER: &str = "points_transfer";
    pub const HOLD_SETTLE: &str = "hold_settle";
    pub const CASH_ADJUSTMENT: &str = "cash_adjustment";
    pub const TOKEN_BANK_ISSUE: &str = "token_bank_issue";
    pub const TOKEN_BANK_PURCHASE_CREDIT: &str = "token_bank_purchase_credit";
    pub const TOKEN_BANK_GRANT: &str = "token_bank_grant";
    pub const TOKEN_BANK_TRANSFER: &str = "token_bank_transfer";
    pub const TOKEN_BANK_HOLD: &str = "token_bank_hold";
    pub const TOKEN_BANK_HOLD_SETTLE: &str = "token_bank_hold_settle";
    pub const TOKEN_BANK_HOLD_RELEASE: &str = "token_bank_hold_release";
    pub const TOKEN_BANK_DEBIT: &str = "token_bank_debit";
    pub const TOKEN_BANK_SERVICE_INCOME: &str = "token_bank_service_income";
    pub const TOKEN_BANK_BURN: &str = "token_bank_burn";
    pub const TOKEN_BANK_REVERSAL: &str = "token_bank_reversal";
    pub const JOIN_FEE_COMMISSION: &str = "join_fee_commission";
    pub const COMMISSION_EARN: &str = "commission_earn";
    pub const COMMISSION_WITHDRAW_HOLD: &str = "commission_withdraw_hold";
    pub const COMMISSION_WITHDRAW_RELEASE: &str = "commission_withdraw_release";
    pub const COMMISSION_WITHDRAW_PAID: &str = "commission_withdraw_paid";
    pub const COMMISSION_ADJUSTMENT: &str = "commission_adjustment";
    pub const USAGE_SETTLEMENT: &str = "usage_settlement";
    pub const MANUAL_ADJUSTMENT: &str = "manual_adjustment";

    pub fn validate(value: &str) -> Result<(), CommerceServiceError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(CommerceServiceError::validation(
                "business_type is required",
            ));
        }
        if value.len() > 64 {
            return Err(CommerceServiceError::validation(
                "business_type must be at most 64 characters",
            ));
        }
        if !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        {
            return Err(CommerceServiceError::validation(
                "business_type must use lowercase letters, digits, or underscores",
            ));
        }
        Ok(())
    }
}

/// Canonical account owner types (subject kinds) for `acct_account.owner_type`.
pub struct CommerceAccountOwnerType;

impl CommerceAccountOwnerType {
    pub const USER: &str = "USER";
    pub const ORG: &str = "ORG";
    pub const PROJECT: &str = "PROJECT";
    pub const SERVICE: &str = "SERVICE";
    pub const SYSTEM: &str = "SYSTEM";
    /// Partner (agent/distributor) subject kind.
    pub const PARTNER: &str = "PARTNER";

    pub fn validate(value: &str) -> Result<(), CommerceServiceError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(CommerceServiceError::validation(
                "owner_type is required",
            ));
        }
        if value.len() > 32 {
            return Err(CommerceServiceError::validation(
                "owner_type must be at most 32 characters",
            ));
        }
        if !value
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
        {
            return Err(CommerceServiceError::validation(
                "owner_type must use uppercase letters, digits, or underscores",
            ));
        }
        Ok(())
    }
}

/// Canonical account purposes for `acct_account.account_purpose`.
pub struct CommerceAccountPurpose;

impl CommerceAccountPurpose {
    pub const GENERAL: &str = "GENERAL";
    pub const BUDGET: &str = "BUDGET";
    pub const RESERVE: &str = "RESERVE";
    /// Settlement account used for commission/partner revenue.
    pub const SETTLEMENT: &str = "SETTLEMENT";
    pub const BURN: &str = "BURN";
    pub const SUSPENSE: &str = "SUSPENSE";
}


