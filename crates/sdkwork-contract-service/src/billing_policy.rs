//! Shared billing-policy and admission contracts.
//!
//! These types are the wire between the account domain (which owns funding
//! policy and wallets) and any consumer that turns measured usage into a charge
//! (the routing runtime). They live in the contract crate so a consumer depends
//! on a stable contract instead of another service's implementation module.
//!
//! Two rules keep the contract honest:
//!
//! * A policy states **funding** rules only. Unit prices, price books and
//!   catalogue defaults stay in the pricing owner; the consumer sends an
//!   estimated charge in token-bank points and never a price.
//! * Admission never changes the settled charge. [`AdmissionDecision`] says how
//!   much to freeze (or whether to defer); settlement always charges measured
//!   usage afterwards.
//!
//! Ratios and shares are integer basis points (`10_000` = 1.0) because they are
//! persisted as exact decimals and money must never round through a float.

use crate::CommerceServiceError;

/// Default share of a wallet a single reservation may freeze (25%).
pub const DEFAULT_MAX_HOLD_SHARE_BPS: u32 = 2_500;
/// Default prepaid reservation multiplier applied to the estimated charge.
pub const DEFAULT_PRECHARGE_RATIO_BPS: u32 = 10_000;
/// Default ceiling applied to a client-declared output-token cap.
pub const DEFAULT_OUTPUT_TOKEN_CEILING: i64 = 32_768;
/// Default lifetime of a synchronous prepaid reservation.
pub const DEFAULT_HOLD_TTL_SECONDS: i64 = 1_800;
/// Lowest accepted reservation lifetime (one minute).
pub const MIN_HOLD_TTL_SECONDS: i64 = 60;
/// Highest accepted reservation lifetime (one day).
pub const MAX_HOLD_TTL_SECONDS: i64 = 86_400;
/// Highest accepted output-token ceiling (one million tokens).
pub const MAX_OUTPUT_TOKEN_CEILING: i64 = 1_000_000;
/// Basis-point scale used by every ratio and share in this contract.
pub const BPS_SCALE: u32 = 10_000;

/// Where a charge is funded from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChargeMode {
    /// Prepaid balance only: a reservation must be fully funded.
    PrepaidBalance,
    /// Prepaid balance plus an explicit credit line.
    CreditLine,
    /// Contract billing: charges are invoiced on the settlement period.
    PostpaidInvoice,
    /// A period allowance funds the charge before any balance is touched.
    PlanAllowance,
    /// Trial credits fund the charge; nothing is billed to the subject.
    FreeTrial,
    /// The caller brings its own upstream key; inference is not charged here.
    Byok,
}

impl ChargeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PrepaidBalance => "prepaid_balance",
            Self::CreditLine => "credit_line",
            Self::PostpaidInvoice => "postpaid_invoice",
            Self::PlanAllowance => "plan_allowance",
            Self::FreeTrial => "free_trial",
            Self::Byok => "byok",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value.trim() {
            "prepaid_balance" => Self::PrepaidBalance,
            "credit_line" => Self::CreditLine,
            "postpaid_invoice" => Self::PostpaidInvoice,
            "plan_allowance" => Self::PlanAllowance,
            "free_trial" => Self::FreeTrial,
            "byok" => Self::Byok,
            _ => return None,
        })
    }

    /// Whether the mode may draw on the account's token-bank balance or credit.
    pub fn draws_on_balance(self) -> bool {
        matches!(self, Self::PrepaidBalance | Self::CreditLine)
    }
}

/// When a charge is deducted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettlementMode {
    /// Freeze before dispatch and settle the reservation against measured usage.
    Synchronous,
    /// Admit on a solvency check and deduct once the usage fact is recorded.
    Asynchronous,
}

impl SettlementMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Synchronous => "synchronous",
            Self::Asynchronous => "asynchronous",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value.trim() {
            "synchronous" => Self::Synchronous,
            "asynchronous" => Self::Asynchronous,
            _ => return None,
        })
    }
}

/// How a reservation that cannot be funded in full is handled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionMode {
    /// Reserve the full amount or reject the request.
    Strict,
    /// Reserve the full amount when the balance plus the overdraft envelope
    /// covers it; the envelope is a bounded, operator-set tolerance.
    StrictWithEnvelope,
    /// Reserve whatever the balance can fund and settle the remainder
    /// afterwards; reject only when nothing at all is available.
    BoundedOverdraft,
}

impl AdmissionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::StrictWithEnvelope => "strict_with_envelope",
            Self::BoundedOverdraft => "bounded_overdraft",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value.trim() {
            "strict" => Self::Strict,
            "strict_with_envelope" => Self::StrictWithEnvelope,
            "bounded_overdraft" => Self::BoundedOverdraft,
            _ => return None,
        })
    }
}

/// What happens when measured usage exceeds the frozen reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OveragePolicy {
    /// Charge the excess as a receivable owed by the subject.
    AllowReceivable,
    /// Reject the request instead of admitting a possible overage.
    Fail,
    /// Fund the excess from the balance when it can, otherwise defer.
    Degrade,
}

impl OveragePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllowReceivable => "allow_receivable",
            Self::Fail => "fail",
            Self::Degrade => "degrade",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value.trim() {
            "allow_receivable" => Self::AllowReceivable,
            "fail" => Self::Fail,
            "degrade" => Self::Degrade,
            _ => return None,
        })
    }
}

/// Why a request was rejected at admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionRejectCode {
    /// Neither the balance, the credit line nor the envelope can fund it.
    InsufficientBalance,
    /// The mode draws on a credit line and it is spent.
    CreditLimitExceeded,
    /// A plan allowance is exhausted and the overage policy forbids more.
    AllowanceExhausted,
    /// Trial credits are exhausted.
    TrialExhausted,
}

impl AdmissionRejectCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InsufficientBalance => "insufficient_balance",
            Self::CreditLimitExceeded => "credit_limit_exceeded",
            Self::AllowanceExhausted => "allowance_exhausted",
            Self::TrialExhausted => "trial_exhausted",
        }
    }
}

/// Admission parameters of one policy row.
///
/// Every field is part of the policy aggregate, so a caller resolves one row and
/// reads both the funding mode and the reservation limits from it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BillingPolicyParams {
    pub charge_mode: ChargeMode,
    pub settlement_mode: SettlementMode,
    pub admission_mode: AdmissionMode,
    pub overage_policy: OveragePolicy,
    /// Multiplier applied to the estimated charge before freezing (`10_000` = 1x).
    pub precharge_ratio_bps: u32,
    /// Ceiling applied to a client-declared output-token cap while estimating.
    pub output_token_ceiling: i64,
    /// Lifetime of a synchronous reservation.
    pub hold_ttl_seconds: i64,
    /// Lowest reservation raised for a positive estimate, in token-bank points.
    pub min_hold_points: i64,
    /// Largest share of the wallet one reservation may freeze (`10_000` = 100%).
    pub max_hold_share_bps: u32,
    /// Funding tolerated beyond the balance, in token-bank points.
    pub overdraft_envelope_points: i64,
    /// Credit line available to `CreditLine` and `PostpaidInvoice`.
    pub credit_limit_points: i64,
}

impl Default for BillingPolicyParams {
    fn default() -> Self {
        Self {
            charge_mode: ChargeMode::PrepaidBalance,
            settlement_mode: SettlementMode::Synchronous,
            admission_mode: AdmissionMode::Strict,
            overage_policy: OveragePolicy::AllowReceivable,
            precharge_ratio_bps: DEFAULT_PRECHARGE_RATIO_BPS,
            output_token_ceiling: DEFAULT_OUTPUT_TOKEN_CEILING,
            hold_ttl_seconds: DEFAULT_HOLD_TTL_SECONDS,
            min_hold_points: 0,
            max_hold_share_bps: DEFAULT_MAX_HOLD_SHARE_BPS,
            overdraft_envelope_points: 0,
            credit_limit_points: 0,
        }
    }
}

impl BillingPolicyParams {
    /// Rejects parameters outside the documented clamps.
    ///
    /// The database carries the same clamps as CHECK constraints; this keeps an
    /// in-memory or RPC caller from smuggling a value that the storage layer
    /// would refuse anyway, and it is the only place that reports them.
    pub fn validate(&self) -> Result<(), CommerceServiceError> {
        if self.precharge_ratio_bps > 10 * BPS_SCALE {
            return Err(CommerceServiceError::validation(
                "precharge ratio must be between 0x and 10x",
            ));
        }
        if self.max_hold_share_bps == 0 || self.max_hold_share_bps > BPS_SCALE {
            return Err(CommerceServiceError::validation(
                "max hold share must be greater than 0 and at most 100%",
            ));
        }
        if self.output_token_ceiling < 1 || self.output_token_ceiling > MAX_OUTPUT_TOKEN_CEILING {
            return Err(CommerceServiceError::validation(
                "output token ceiling must be between 1 and 1000000",
            ));
        }
        if self.hold_ttl_seconds < MIN_HOLD_TTL_SECONDS
            || self.hold_ttl_seconds > MAX_HOLD_TTL_SECONDS
        {
            return Err(CommerceServiceError::validation(
                "hold ttl must be between 60 and 86400 seconds",
            ));
        }
        if self.min_hold_points < 0
            || self.overdraft_envelope_points < 0
            || self.credit_limit_points < 0
        {
            return Err(CommerceServiceError::validation(
                "policy point amounts must not be negative",
            ));
        }
        Ok(())
    }
}

/// Funding the subject can reach right now, in token-bank points.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AccountCapacity {
    /// Spendable balance.
    pub available_points: i64,
    /// Amount already frozen by in-flight reservations.
    pub frozen_points: i64,
    /// Unused credit line.
    pub credit_remaining_points: i64,
    /// Unused period allowance.
    pub allowance_remaining_points: i64,
    /// Unused trial credits.
    pub trial_remaining_points: i64,
}

impl AccountCapacity {
    /// Wallet size used by the single-request freeze share.
    pub fn total_points(&self) -> i64 {
        self.available_points.saturating_add(self.frozen_points)
    }
}

/// Outcome of an admission decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionDecision {
    /// Dispatch and freeze `hold_points` (zero means no reservation).
    Admit { hold_points: i64 },
    /// Dispatch with no reservation and settle from the recorded usage.
    AdmitDeferred,
    /// Dispatch without charging the subject (trial credits or BYOK).
    AdmitFree,
    /// Refuse to dispatch.
    Reject { code: AdmissionRejectCode },
}

/// Asks the account side whether one estimated charge may proceed.
///
/// The caller owns pricing and sends only the estimated points; the account side
/// owns the funding policy, the wallet snapshot and the decision.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BillingAuthorizationQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub api_key_id: Option<String>,
    /// Estimated charge for this request, in token-bank points.
    pub estimated_points: i64,
    /// Wallet owner kind (`USER` or `ORGANIZATION`); `None` means `USER`.
    pub wallet_owner_type: Option<String>,
    /// Wallet owner id for team billing (the organization id); `None` means the
    /// calling user.
    pub wallet_owner_id: Option<String>,
}

/// The decision plus the policy revision it was made under.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BillingAdmission {
    /// What the caller should do with the request.
    pub decision: AdmissionDecision,
    /// Policy row that governed the decision; `None` means platform defaults.
    pub policy_id: Option<String>,
    /// Amount that may be reserved, in points (`0` when the decision is not a hold).
    pub hold_points: i64,
}

/// Parses a stored decimal ratio/share (`"1.0000"`) into basis points.
///
/// Storage keeps an exact `NUMERIC(10,4)`; this converts it without a float and
/// rejects anything the policy clamps would refuse anyway.
pub fn parse_bps(value: &str) -> Option<u32> {
    let text = value.trim();
    if text.is_empty() || text.starts_with('-') {
        return None;
    }
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (text, ""),
    };
    if !whole.chars().all(|c| c.is_ascii_digit()) || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let whole: u64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    // Basis points keep four decimals: pad or reject beyond that precision.
    let mut fraction = fraction.to_owned();
    if fraction.len() > 4 {
        return None;
    }
    while fraction.len() < 4 {
        fraction.push('0');
    }
    let fraction: u64 = fraction.parse().ok()?;
    let bps = whole
        .checked_mul(u64::from(BPS_SCALE))?
        .checked_add(fraction)?;
    u32::try_from(bps).ok()
}

/// Renders basis points as the stored `NUMERIC(10,4)` text form.
pub fn format_bps(bps: u32) -> String {
    format!("{}.{:04}", bps / BPS_SCALE, bps % BPS_SCALE)
}
