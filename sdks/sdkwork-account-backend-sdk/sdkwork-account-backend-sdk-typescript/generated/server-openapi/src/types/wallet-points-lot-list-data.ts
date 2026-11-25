import type { PageInfo } from './page-info';
import type { WalletPointsLot } from './wallet-points-lot';

export interface WalletPointsLotListData {
  items: WalletPointsLot[];
  /** Page info field on the points lot list response. */
  pageInfo: PageInfo;
}
