import type { PageInfo } from './page-info';
import type { WalletBillingPolicy } from './wallet-billing-policy';

/** Billing policy list result schema exposed by the account service. */
export interface WalletBillingPolicyListResult {
  code: 0;
  data: { items: WalletBillingPolicy[]; pageInfo: PageInfo; };
  traceId: string;
}
