import type { WalletAccountSummary } from './wallet-account-summary';

/** Wallet account list result schema exposed by the account service. */
export interface WalletAccountListResult {
  code: 0;
  data: { items: WalletAccountSummary[]; };
  traceId: string;
}
