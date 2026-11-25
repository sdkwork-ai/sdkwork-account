import type { PageInfo } from './page-info';
import type { WalletLedgerEntry } from './wallet-ledger-entry';

export interface WalletLedgerEntryListData {
  items: WalletLedgerEntry[];
  /** Page info field on the ledger list response. */
  pageInfo: PageInfo;
}
