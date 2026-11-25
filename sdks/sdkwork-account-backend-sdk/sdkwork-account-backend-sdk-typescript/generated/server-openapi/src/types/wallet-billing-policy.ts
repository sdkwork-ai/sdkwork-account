export interface WalletBillingPolicy {
  id: string;
  uuid: string;
  tenantId: string;
  organizationId: string;
  policyCode: string;
  name: string;
  /** 0 platform, 1 tenant, 2 organization, 3 user, 4 api key. */
  scopeType: number;
  scopeId: string;
  priority: number;
  /** Revision number of this scope policy. */
  revision: string;
  /** 1 live, 0 retired. */
  status: number;
  chargeMode: 'prepaid_balance' | 'credit_line' | 'postpaid_invoice' | 'plan_allowance' | 'free_trial' | 'byok';
  settlementMode: 'synchronous' | 'asynchronous';
  /** Decimal multiplier applied to the estimated charge. */
  prechargeRatio: string;
  /** Ceiling applied to a client-declared output-token cap. */
  outputTokenCeiling: string;
  inputTokenSource: 'bytes_div4' | 'tokenizer';
  /** Lifetime of a synchronous reservation. */
  holdTtlSeconds: string;
  /** Lowest reservation raised for a positive estimate. */
  minHoldPoints: string;
  /** Largest share of the wallet one reservation may freeze. */
  maxHoldShare: string;
  admissionMode: 'strict' | 'strict_with_envelope' | 'bounded_overdraft';
  /** Funding tolerated beyond the balance. */
  overdraftEnvelopePoints: string;
  /** Credit line available to credit modes. */
  creditLimitPoints: string;
  allowancePeriod?: 'day' | 'week' | 'month' | 'none' | null;
  /** Period allowance quota, when the mode uses one. */
  allowanceQuota?: string;
  allowanceResetMode?: 'calendar' | 'rolling' | null;
  overagePolicy: 'allow_receivable' | 'fail' | 'degrade';
  effectiveFrom: string;
  effectiveTo?: string | null;
  updatedAt: string;
}
