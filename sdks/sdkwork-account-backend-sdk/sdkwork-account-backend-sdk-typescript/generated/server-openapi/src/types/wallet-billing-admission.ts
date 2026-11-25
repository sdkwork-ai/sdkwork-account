export interface WalletBillingAdmission {
  decision: 'admit' | 'deferred' | 'free' | 'reject';
  /** Amount the account side would freeze, in points. */
  holdPoints: string;
  rejectCode?: 'insufficient_balance' | 'credit_limit_exceeded' | 'allowance_exhausted' | 'trial_exhausted' | null;
  policyId?: string | null;
}
