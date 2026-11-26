//! Account billing policy rules.
//!
//! The vocabulary (modes, parameters, capacity, decisions, the authorization
//! query) lives in `sdkwork-contract-service`, so a consumer such as the routing
//! runtime depends on a contract instead of this implementation module. This
//! file owns the one rule that is genuinely the account domain's: whether a
//! subject can fund an estimated charge, and how much to freeze.

pub use sdkwork_contract_service::{
    format_bps, parse_bps, AccountCapacity, AdmissionDecision, AdmissionMode, AdmissionRejectCode,
    BillingAdmission, BillingAuthorizationQuery, BillingPolicyParams, ChargeMode, OveragePolicy,
    SettlementMode, BPS_SCALE, DEFAULT_HOLD_TTL_SECONDS, DEFAULT_MAX_HOLD_SHARE_BPS,
    DEFAULT_OUTPUT_TOKEN_CEILING, DEFAULT_PRECHARGE_RATIO_BPS, MAX_HOLD_TTL_SECONDS,
    MAX_OUTPUT_TOKEN_CEILING, MIN_HOLD_TTL_SECONDS,
};
/// Decides whether a subject can fund `estimated_points` and how much to freeze.
///
/// The estimate is the caller's charge for this request in token-bank points. It
/// is derived from prices the caller owns; this function never sees a price.
///
/// The reservation is bounded three ways, and each bound exists because a
/// worst-case estimate is not a cost: the multiplier discounts the estimate, the
/// ceiling stops a model-wide output cap from becoming a reservation, and the
/// share cap stops one request from freezing a wallet on its own.
pub fn authorize(
    params: &BillingPolicyParams,
    capacity: AccountCapacity,
    estimated_points: i64,
) -> AdmissionDecision {
    if estimated_points <= 0 || params.precharge_ratio_bps == 0 {
        return AdmissionDecision::Admit { hold_points: 0 };
    }

    match params.charge_mode {
        ChargeMode::Byok => return AdmissionDecision::AdmitFree,
        ChargeMode::FreeTrial => {
            if capacity.trial_remaining_points > 0 {
                // A trial credit is not wallet money: admit without a reservation
                // so settlement also waives the charge instead of debiting the
                // balance. The trial pool itself must be decremented by the
                // funding source once trial credits are wired; until then the
                // snapshot below reports zero and this branch never runs.
                return AdmissionDecision::AdmitFree;
            }
            return AdmissionDecision::Reject {
                code: AdmissionRejectCode::TrialExhausted,
            };
        }
        ChargeMode::PostpaidInvoice => return AdmissionDecision::AdmitDeferred,
        ChargeMode::PrepaidBalance | ChargeMode::CreditLine | ChargeMode::PlanAllowance => {}
    }

    // Funding the mode may reach, before the overdraft envelope.
    let mut capacity_points = match params.charge_mode {
        ChargeMode::PrepaidBalance => capacity.available_points,
        ChargeMode::CreditLine => capacity
            .available_points
            .saturating_add(capacity.credit_remaining_points),
        ChargeMode::PlanAllowance => {
            if capacity.allowance_remaining_points > 0 {
                capacity.allowance_remaining_points
            } else if matches!(
                params.overage_policy,
                OveragePolicy::AllowReceivable | OveragePolicy::Degrade
            ) {
                // The allowance is spent and the policy allows the balance to
                // take over, which is the documented allowance-to-balance
                // hand-off rather than a silent overdraft.
                capacity
                    .available_points
                    .saturating_add(capacity.credit_remaining_points)
            } else {
                return AdmissionDecision::Reject {
                    code: AdmissionRejectCode::AllowanceExhausted,
                };
            }
        }
        _ => capacity.available_points,
    };
    if params.charge_mode == ChargeMode::CreditLine
        && capacity.credit_remaining_points <= 0
        && capacity.available_points <= 0
    {
        return AdmissionDecision::Reject {
            code: AdmissionRejectCode::CreditLimitExceeded,
        };
    }

    // Affordability is judged on the (discounted) estimate; the frozen amount is
    // only a bounded part of it, so clamping never turns into free admission.
    let scaled = mul_bps(estimated_points.max(0), params.precharge_ratio_bps);
    let share_cap = mul_bps(capacity.total_points().max(0), params.max_hold_share_bps);
    let affordable_bar = scaled;
    let hold = scaled
        .max(params.min_hold_points)
        .min(share_cap)
        .min(capacity_points.max(0))
        .max(0);
    if params.settlement_mode == SettlementMode::Asynchronous {
        // No reservation, but an asynchronous mode still needs a solvent
        // subject: the usage fact becomes a debit that must land somewhere.
        return if capacity_points > 0 {
            AdmissionDecision::AdmitDeferred
        } else {
            AdmissionDecision::Reject {
                code: AdmissionRejectCode::InsufficientBalance,
            }
        };
    }
    if affordable_bar == 0 {
        return AdmissionDecision::Admit { hold_points: 0 };
    }

    capacity_points = capacity_points.saturating_add(match params.admission_mode {
        AdmissionMode::StrictWithEnvelope => params.overdraft_envelope_points,
        AdmissionMode::Strict | AdmissionMode::BoundedOverdraft => 0,
    });
    if capacity_points >= affordable_bar {
        return AdmissionDecision::Admit {
            hold_points: hold.min(capacity_points),
        };
    }
    match params.admission_mode {
        AdmissionMode::Strict | AdmissionMode::StrictWithEnvelope => AdmissionDecision::Reject {
            code: AdmissionRejectCode::InsufficientBalance,
        },
        AdmissionMode::BoundedOverdraft => {
            let fundable = capacity_points.min(hold).max(0);
            if fundable > 0 {
                AdmissionDecision::Admit {
                    hold_points: fundable,
                }
            } else {
                AdmissionDecision::Reject {
                    code: AdmissionRejectCode::InsufficientBalance,
                }
            }
        }
    }
}

/// `value * bps / 10_000`, rounded up so a reservation never under-freezes and
/// saturating so an extreme estimate cannot wrap.
fn mul_bps(value: i64, bps: u32) -> i64 {
    let scaled = (value as i128) * (bps as i128);
    let rounded = (scaled + (BPS_SCALE as i128) - 1) / (BPS_SCALE as i128);
    rounded.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}
#[cfg(test)]
mod tests {
    use super::*;

    fn capacity(available: i64, frozen: i64) -> AccountCapacity {
        AccountCapacity {
            available_points: available,
            frozen_points: frozen,
            ..AccountCapacity::default()
        }
    }

    #[test]
    fn round_trips_mode_vocabulary() {
        for mode in [
            ChargeMode::PrepaidBalance,
            ChargeMode::CreditLine,
            ChargeMode::PostpaidInvoice,
            ChargeMode::PlanAllowance,
            ChargeMode::FreeTrial,
            ChargeMode::Byok,
        ] {
            assert_eq!(Some(mode), ChargeMode::parse(mode.as_str()));
        }
        for mode in [SettlementMode::Synchronous, SettlementMode::Asynchronous] {
            assert_eq!(Some(mode), SettlementMode::parse(mode.as_str()));
        }
        for mode in [
            AdmissionMode::Strict,
            AdmissionMode::StrictWithEnvelope,
            AdmissionMode::BoundedOverdraft,
        ] {
            assert_eq!(Some(mode), AdmissionMode::parse(mode.as_str()));
        }
        for policy in [
            OveragePolicy::AllowReceivable,
            OveragePolicy::Fail,
            OveragePolicy::Degrade,
        ] {
            assert_eq!(Some(policy), OveragePolicy::parse(policy.as_str()));
        }
        assert_eq!(None, ChargeMode::parse("prepaid"));
    }

    #[test]
    fn defaults_are_the_documented_platform_behaviour() {
        let params = BillingPolicyParams::default();
        assert_eq!(ChargeMode::PrepaidBalance, params.charge_mode);
        assert_eq!(SettlementMode::Synchronous, params.settlement_mode);
        assert_eq!(AdmissionMode::Strict, params.admission_mode);
        assert_eq!(DEFAULT_PRECHARGE_RATIO_BPS, params.precharge_ratio_bps);
        assert_eq!(DEFAULT_MAX_HOLD_SHARE_BPS, params.max_hold_share_bps);
        assert_eq!(DEFAULT_OUTPUT_TOKEN_CEILING, params.output_token_ceiling);
        assert_eq!(0, params.overdraft_envelope_points);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn validate_rejects_values_outside_the_clamps() {
        let mut params = BillingPolicyParams::default();
        params.precharge_ratio_bps = 10 * BPS_SCALE + 1;
        assert!(params.validate().is_err());

        let mut params = BillingPolicyParams::default();
        params.max_hold_share_bps = 0;
        assert!(params.validate().is_err());

        let mut params = BillingPolicyParams::default();
        params.output_token_ceiling = 0;
        assert!(params.validate().is_err());

        let mut params = BillingPolicyParams::default();
        params.hold_ttl_seconds = MIN_HOLD_TTL_SECONDS - 1;
        assert!(params.validate().is_err());

        let mut params = BillingPolicyParams::default();
        params.credit_limit_points = -1;
        assert!(params.validate().is_err());
    }

    /// The regression this module exists for: a reservation must stay a bounded
    /// share of the wallet, so a large estimate cannot freeze it in one request.
    #[test]
    fn a_single_reservation_never_freezes_the_whole_wallet() {
        let params = BillingPolicyParams::default();
        let wallet = capacity(41_459_562, 33_256_530);
        let decision = authorize(&params, wallet, 16_443_980);
        let AdmissionDecision::Admit { hold_points } = decision else {
            panic!("a funded wallet must admit, got {decision:?}");
        };
        // The estimate is below the share cap, so the cap does not shrink it: the
        // estimation ceiling and this cap are complementary defences.
        assert_eq!(16_443_980, hold_points);
        assert!(hold_points <= mul_bps(wallet.total_points(), DEFAULT_MAX_HOLD_SHARE_BPS));

        // An extreme estimate is both unaffordable and clamped.
        assert_eq!(
            AdmissionDecision::Reject {
                code: AdmissionRejectCode::InsufficientBalance
            },
            authorize(&params, wallet, 10_000_000_000)
        );
        let bounded = BillingPolicyParams {
            admission_mode: AdmissionMode::BoundedOverdraft,
            ..params
        };
        assert_eq!(
            AdmissionDecision::Admit {
                hold_points: mul_bps(wallet.total_points(), DEFAULT_MAX_HOLD_SHARE_BPS)
            },
            authorize(&bounded, wallet, 10_000_000_000)
        );
    }

    #[test]
    fn a_prepaid_wallet_is_rejected_only_when_it_cannot_fund_the_hold() {
        let params = BillingPolicyParams::default();
        let funded = authorize(&params, capacity(1_000_000, 0), 100_000);
        assert!(matches!(
            funded,
            AdmissionDecision::Admit {
                hold_points: 100_000
            }
        ));

        let short = authorize(&params, capacity(50_000, 0), 100_000);
        assert_eq!(
            AdmissionDecision::Reject {
                code: AdmissionRejectCode::InsufficientBalance
            },
            short
        );
    }

    #[test]
    fn the_envelope_admits_a_bounded_shortfall() {
        let mut params = BillingPolicyParams {
            admission_mode: AdmissionMode::StrictWithEnvelope,
            ..BillingPolicyParams::default()
        };
        params.overdraft_envelope_points = 60_000;
        assert!(matches!(
            authorize(&params, capacity(50_000, 0), 100_000),
            AdmissionDecision::Admit { .. }
        ));
        params.overdraft_envelope_points = 0;
        assert!(matches!(
            authorize(&params, capacity(50_000, 0), 100_000),
            AdmissionDecision::Reject { .. }
        ));
    }

    #[test]
    fn bounded_overdraft_freezes_what_exists_and_defers_the_rest() {
        let params = BillingPolicyParams {
            admission_mode: AdmissionMode::BoundedOverdraft,
            ..BillingPolicyParams::default()
        };
        // 50_000 points of balance against a 100_000 point estimate: the share cap
        // bounds the freeze to 12_500, and the request is admitted on that.
        assert_eq!(
            AdmissionDecision::Admit {
                hold_points: 12_500
            },
            authorize(&params, capacity(50_000, 0), 100_000),
            "a bounded overdraft freezes what exists up to the share cap"
        );
        assert!(matches!(
            authorize(&params, capacity(0, 0), 100_000),
            AdmissionDecision::Reject { .. }
        ));
    }

    #[test]
    fn the_ratio_discounts_the_estimate() {
        let params = BillingPolicyParams {
            precharge_ratio_bps: 5_000,
            ..BillingPolicyParams::default()
        };
        assert_eq!(
            AdmissionDecision::Admit {
                hold_points: 50_000
            },
            authorize(&params, capacity(1_000_000, 0), 100_000)
        );
        let zero_ratio = BillingPolicyParams {
            precharge_ratio_bps: 0,
            ..BillingPolicyParams::default()
        };
        assert_eq!(
            AdmissionDecision::Admit { hold_points: 0 },
            authorize(&zero_ratio, capacity(1_000_000, 0), 100_000)
        );
    }

    #[test]
    fn the_minimum_hold_raises_a_positive_estimate_but_respects_the_share_cap() {
        let params = BillingPolicyParams {
            min_hold_points: 5_000,
            ..BillingPolicyParams::default()
        };
        assert_eq!(
            AdmissionDecision::Admit { hold_points: 5_000 },
            authorize(&params, capacity(1_000_000, 0), 10)
        );
        // A tiny wallet caps the minimum too: the share bound wins.
        let wallet = capacity(100, 0);
        let share_cap = mul_bps(wallet.total_points(), DEFAULT_MAX_HOLD_SHARE_BPS);
        assert_eq!(
            AdmissionDecision::Admit {
                hold_points: share_cap
            },
            authorize(&params, wallet, 10)
        );
    }

    #[test]
    fn allowance_falls_through_to_the_balance_only_when_policy_allows() {
        let allowance = BillingPolicyParams {
            charge_mode: ChargeMode::PlanAllowance,
            ..BillingPolicyParams::default()
        };
        let mut funded = capacity(1_000_000, 0);
        funded.allowance_remaining_points = 200_000;
        assert!(matches!(
            authorize(&allowance, funded, 100_000),
            AdmissionDecision::Admit { .. }
        ));

        let spent = capacity(1_000_000, 0);
        assert!(matches!(
            authorize(&allowance, spent, 100_000),
            AdmissionDecision::Admit { .. }
        ));

        let strict = BillingPolicyParams {
            overage_policy: OveragePolicy::Fail,
            ..allowance.clone()
        };
        assert_eq!(
            AdmissionDecision::Reject {
                code: AdmissionRejectCode::AllowanceExhausted
            },
            authorize(&strict, spent, 100_000)
        );
    }

    #[test]
    fn credit_line_adds_its_remaining_headroom() {
        let params = BillingPolicyParams {
            charge_mode: ChargeMode::CreditLine,
            ..BillingPolicyParams::default()
        };
        let mut wallet = capacity(10_000, 0);
        wallet.credit_remaining_points = 500_000;
        assert!(matches!(
            authorize(&params, wallet, 100_000),
            AdmissionDecision::Admit { .. }
        ));
        let exhausted = capacity(0, 0);
        assert_eq!(
            AdmissionDecision::Reject {
                code: AdmissionRejectCode::CreditLimitExceeded
            },
            authorize(&params, exhausted, 100_000)
        );
    }

    /// Plan allowance spends lot credits, not the wallet balance.
    #[test]
    fn plan_allowance_funds_requests_without_touching_the_balance() {
        let params = BillingPolicyParams {
            charge_mode: ChargeMode::PlanAllowance,
            ..BillingPolicyParams::default()
        };
        let mut funded = capacity(0, 0);
        funded.allowance_remaining_points = 5_000_000;
        // Allowance credit is consumed at settlement, not frozen up front: there is
        // no prepaid money to reserve, and the share cap is a share of the wallet.
        // Admitting with a zero hold is therefore the correct answer, and the lot
        // consumption happens when the usage is settled.
        assert_eq!(
            AdmissionDecision::Admit { hold_points: 0 },
            authorize(&params, funded, 1_000_000),
            "a funded allowance must admit without a wallet balance"
        );

        // Exhausted allowance with a strict overage policy is a distinct rejection,
        // not a generic insufficient balance: the subject has money in the wallet,
        // so "recharge" would be the wrong advice.
        let strict = BillingPolicyParams {
            charge_mode: ChargeMode::PlanAllowance,
            overage_policy: OveragePolicy::Fail,
            ..BillingPolicyParams::default()
        };
        assert_eq!(
            AdmissionDecision::Reject {
                code: AdmissionRejectCode::AllowanceExhausted,
            },
            authorize(&strict, capacity(1_000_000, 0), 1_000)
        );

        // The documented hand-off: an exhausted allowance lets the balance take over
        // when the policy allows a receivable.
        let handoff = BillingPolicyParams {
            charge_mode: ChargeMode::PlanAllowance,
            overage_policy: OveragePolicy::AllowReceivable,
            ..BillingPolicyParams::default()
        };
        assert!(matches!(
            authorize(&handoff, capacity(1_000_000, 0), 1_000),
            AdmissionDecision::Admit { .. }
        ));

        // A funded allowance is not a licence to freeze the whole wallet: the share
        // cap still bounds the reservation, and the balance is not the funding source.
        let mut huge = capacity(0, 0);
        huge.allowance_remaining_points = 100_000_000;
        assert!(matches!(
            authorize(&params, huge, 50_000_000),
            AdmissionDecision::Admit { hold_points } if hold_points <= 25_000_000
        ));
    }
    #[test]
    fn trial_and_byok_modes_need_no_reservation() {
        let trial = BillingPolicyParams {
            charge_mode: ChargeMode::FreeTrial,
            ..BillingPolicyParams::default()
        };
        let mut wallet = capacity(0, 0);
        wallet.trial_remaining_points = 10_000;
        // A trial credit is not wallet money. `AdmitFree` (not `AdmitDeferred`) is
        // what makes the routing side waive the settlement charge instead of
        // debiting the balance; a deferred admission would bill a trial user.
        assert_eq!(
            AdmissionDecision::AdmitFree,
            authorize(&trial, wallet, 1_000)
        );
        assert_eq!(
            AdmissionDecision::Reject {
                code: AdmissionRejectCode::TrialExhausted
            },
            authorize(&trial, capacity(1_000_000, 0), 1_000)
        );
        let byok = BillingPolicyParams {
            charge_mode: ChargeMode::Byok,
            ..BillingPolicyParams::default()
        };
        assert_eq!(
            AdmissionDecision::AdmitFree,
            authorize(&byok, capacity(0, 0), 1_000)
        );
    }

    #[test]
    fn asynchronous_settlement_checks_solvency_without_freezing() {
        let params = BillingPolicyParams {
            settlement_mode: SettlementMode::Asynchronous,
            ..BillingPolicyParams::default()
        };
        assert_eq!(
            AdmissionDecision::AdmitDeferred,
            authorize(&params, capacity(1, 0), 1_000_000)
        );
        assert!(matches!(
            authorize(&params, capacity(0, 0), 1_000_000),
            AdmissionDecision::Reject { .. }
        ));
    }

    #[test]
    fn postpaid_invoice_never_freezes_prepaid_funds() {
        let params = BillingPolicyParams {
            charge_mode: ChargeMode::PostpaidInvoice,
            ..BillingPolicyParams::default()
        };
        assert_eq!(
            AdmissionDecision::AdmitDeferred,
            authorize(&params, capacity(0, 0), 1_000_000)
        );
    }

    #[test]
    fn a_zero_estimate_admits_without_a_reservation() {
        let params = BillingPolicyParams::default();
        assert_eq!(
            AdmissionDecision::Admit { hold_points: 0 },
            authorize(&params, capacity(0, 0), 0)
        );
    }

    /// Acceptance: a real wallet funds eight concurrent agent calls.
    ///
    /// Wallet figures come from the production incident (74,716,092 points total
    /// with 33,256,530 already frozen, i.e. 41,459,562 spendable). The estimate is
    /// the corrected one: a relay declaring 393,216 output tokens at 4 CNY/M with
    /// the 32,768 ceiling and 1.0 CNY/M input over a 76,129-token body.
    /// Deterministic sweep over the funding matrix.
    ///
    /// The properties below are the ones a money path may never violate, and a
    /// single counter-example is a production incident:
    ///
    /// 1. a reservation never exceeds what it is allowed to freeze (share cap,
    ///    available balance);
    /// 2. an admission never freezes more than the (discounted) estimate when no
    ///    minimum hold is configured;
    /// 3. a rejection carries no reservation;
    /// 4. more available balance never turns an admission into a rejection;
    /// 5. free and postpaid modes never freeze prepaid funds.
    #[test]
    fn admission_invariants_hold_across_the_funding_matrix() {
        // Small LCG so the sweep is deterministic and reproducible in CI.
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as i64
        };

        let mut admitted = 0;
        let mut rejected = 0;
        let mut deferred = 0;

        for _ in 0..4_000 {
            let available = next().rem_euclid(80_000_000);
            let frozen = next().rem_euclid(40_000_000);
            let credit = if next() % 3 == 0 {
                next().rem_euclid(20_000_000)
            } else {
                0
            };
            let estimate = next().rem_euclid(30_000_000);
            let ratio_bps = (next().rem_euclid(30_000)) as u32;
            let share_bps = ((next().rem_euclid(10_000)) + 1) as u32;
            let mode = match next().rem_euclid(3) {
                0 => AdmissionMode::Strict,
                1 => AdmissionMode::StrictWithEnvelope,
                _ => AdmissionMode::BoundedOverdraft,
            };
            let envelope = if mode == AdmissionMode::StrictWithEnvelope {
                next().rem_euclid(5_000_000)
            } else {
                0
            };
            let settlement = if next() % 4 == 0 {
                SettlementMode::Asynchronous
            } else {
                SettlementMode::Synchronous
            };
            let params = BillingPolicyParams {
                admission_mode: mode,
                settlement_mode: settlement,
                precharge_ratio_bps: ratio_bps,
                max_hold_share_bps: share_bps,
                overdraft_envelope_points: envelope,
                min_hold_points: 0,
                ..BillingPolicyParams::default()
            };
            let capacity = AccountCapacity {
                available_points: available,
                frozen_points: frozen,
                credit_remaining_points: credit,
                ..AccountCapacity::default()
            };
            let decision = authorize(&params, capacity, estimate);

            // 1 + 3: a reservation is bounded by the share cap and the balance,
            // and a rejection never carries one.
            let share_cap =
                ((capacity.total_points().max(0) as i128 * share_bps as i128) + 9_999) / 10_000;
            let share_cap = share_cap as i64;
            let scaled = ((estimate.max(0) as i128 * ratio_bps as i128) + 9_999) / 10_000;
            let scaled = scaled as i64;
            match decision {
                AdmissionDecision::Admit { hold_points } => {
                    admitted += 1;
                    assert!(hold_points >= 0, "a hold is never negative");
                    assert!(
                        hold_points <= share_cap,
                        "hold {hold_points} exceeds the share cap {share_cap} for estimate {estimate}"
                    );
                    assert!(
                        hold_points <= available,
                        "hold {hold_points} exceeds the spendable balance {available}"
                    );
                    // 2: without a minimum hold the freeze never exceeds the
                    // discounted estimate it is derived from.
                    assert!(
                        hold_points <= scaled.max(0) || scaled == 0,
                        "hold {hold_points} exceeds the discounted estimate {scaled}"
                    );
                }
                AdmissionDecision::Reject { .. } => {
                    rejected += 1;
                }
                AdmissionDecision::AdmitDeferred | AdmissionDecision::AdmitFree => {
                    deferred += 1;
                }
            }

            // 4: more balance never makes an admission worse.
            let richer = AccountCapacity {
                available_points: available.saturating_add(10_000_000),
                ..capacity
            };
            let richer_decision = authorize(&params, richer, estimate);
            if matches!(decision, AdmissionDecision::Admit { .. }) {
                assert!(!matches!(richer_decision, AdmissionDecision::Reject { .. }));
            }
        }

        assert!(
            admitted > 100,
            "the sweep must exercise admissions: {admitted}"
        );
        assert!(
            rejected > 100,
            "the sweep must exercise rejections: {rejected}"
        );
        assert!(
            deferred > 10,
            "the sweep must exercise deferred admissions: {deferred}"
        );
    }

    /// Free and postpaid modes must never touch prepaid funds.
    #[test]
    fn modes_that_do_not_spend_the_balance_never_freeze_it() {
        let capacity = AccountCapacity {
            available_points: 50_000_000,
            frozen_points: 0,
            credit_remaining_points: 0,
            ..AccountCapacity::default()
        };
        for params in [
            BillingPolicyParams {
                charge_mode: ChargeMode::Byok,
                ..BillingPolicyParams::default()
            },
            BillingPolicyParams {
                charge_mode: ChargeMode::PostpaidInvoice,
                ..BillingPolicyParams::default()
            },
        ] {
            assert!(
                !matches!(
                    authorize(&params, capacity, 5_000_000),
                    AdmissionDecision::Admit { .. }
                ),
                "{:?} must not freeze prepaid funds",
                params.charge_mode
            );
        }
    }
    #[test]
    fn eight_parallel_calls_fit_a_small_wallet_with_the_capped_estimate() {
        let params = BillingPolicyParams::default();
        let mut available = 41_459_562_i64;
        let frozen = 33_256_530_i64;
        let corrected_estimate = 1_310_720_i64;
        let mut admitted = 0;
        for _ in 0..8 {
            let capacity = AccountCapacity {
                available_points: available,
                frozen_points: frozen,
                ..AccountCapacity::default()
            };
            match authorize(&params, capacity, corrected_estimate) {
                AdmissionDecision::Admit { hold_points } => {
                    assert_eq!(
                        corrected_estimate, hold_points,
                        "uncapped below the share cap"
                    );
                    available -= hold_points;
                    admitted += 1;
                }
                other => panic!("call {admitted} must be admitted, got {other:?}"),
            }
        }
        assert_eq!(8, admitted);
        assert!(
            available > 0,
            "the eighth call still leaves {} points spendable",
            available
        );
    }

    /// The regression this whole change exists for: with the legacy uncapped
    /// estimate the same wallet is wedged after two concurrent calls.
    #[test]
    fn the_legacy_uncapped_estimate_wedges_the_same_wallet() {
        let params = BillingPolicyParams::default();
        let mut available = 41_459_562_i64;
        let frozen = 33_256_530_i64;
        let legacy_estimate = 16_443_980_i64;
        let mut admitted = 0;
        let mut rejected = 0;
        for _ in 0..8 {
            let capacity = AccountCapacity {
                available_points: available,
                frozen_points: frozen,
                ..AccountCapacity::default()
            };
            match authorize(&params, capacity, legacy_estimate) {
                AdmissionDecision::Admit { hold_points } => {
                    available -= hold_points;
                    admitted += 1;
                }
                AdmissionDecision::Reject { .. } => rejected += 1,
                other => panic!("unexpected decision {other:?}"),
            }
        }
        assert_eq!(2, admitted, "only two legacy reservations fit");
        assert_eq!(6, rejected, "the other six are the reported 402s");
    }
}
