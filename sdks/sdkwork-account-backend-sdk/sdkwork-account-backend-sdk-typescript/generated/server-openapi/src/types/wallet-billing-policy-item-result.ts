import type { WalletBillingPolicy } from './wallet-billing-policy';

/** Billing policy result schema exposed by the account service. */
export interface WalletBillingPolicyItemResult {
  code: 0;
  data: { item: WalletBillingPolicy | null; };
  traceId: string;
}
