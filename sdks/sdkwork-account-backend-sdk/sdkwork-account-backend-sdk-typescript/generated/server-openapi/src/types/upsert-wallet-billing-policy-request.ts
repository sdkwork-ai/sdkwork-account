export interface UpsertWalletBillingPolicyRequest {
  organizationId?: string | null;
  scopeType: number;
  /** Owner id of the scope; 0 for the platform default. */
  scopeId?: string;
  priority?: number;
  policyCode: string;
  name: string;
  chargeMode: 'prepaid_balance' | 'credit_line' | 'postpaid_invoice' | 'plan_allowance' | 'free_trial' | 'byok';
  settlementMode: 'synchronous' | 'asynchronous';
  prechargeRatio: string;
  outputTokenCeiling: string;
  holdTtlSeconds: string;
  minHoldPoints?: string;
  maxHoldShare: string;
  admissionMode: 'strict' | 'strict_with_envelope' | 'bounded_overdraft';
  overdraftEnvelopePoints?: string;
  creditLimitPoints?: string;
  overagePolicy: 'allow_receivable' | 'fail' | 'degrade';
  effectiveFrom?: string | null;
  effectiveTo?: string | null;
}
