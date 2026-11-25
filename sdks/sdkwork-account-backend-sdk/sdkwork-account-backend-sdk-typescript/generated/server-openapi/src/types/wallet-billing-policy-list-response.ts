import type { PageInfo } from './page-info';
import type { WalletBillingPolicy } from './wallet-billing-policy';

export interface WalletBillingPolicyListResponse {
  items: WalletBillingPolicy[];
  /** Page info field on the billing policy list response. */
  pageInfo: PageInfo;
}
