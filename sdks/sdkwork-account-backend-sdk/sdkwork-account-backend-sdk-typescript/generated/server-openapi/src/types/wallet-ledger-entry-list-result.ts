import type { PageInfo } from './page-info';
import type { WalletLedgerEntry } from './wallet-ledger-entry';

/** Wallet ledger entry list result schema exposed by the account service. */
export interface WalletLedgerEntryListResult {
  code: 0;
  data: { items: WalletLedgerEntry[]; pageInfo: PageInfo; };
  traceId: string;
}
