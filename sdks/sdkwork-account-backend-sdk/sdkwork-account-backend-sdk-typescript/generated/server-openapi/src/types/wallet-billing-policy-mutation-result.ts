import type { WalletBillingPolicy } from './wallet-billing-policy';

/** Billing policy mutation result schema exposed by the account service. */
export interface WalletBillingPolicyMutationResult {
  code: 0;
  data: { item: WalletBillingPolicy | null; };
  traceId: string;
}
