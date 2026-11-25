import type { PageInfo } from './page-info';
import type { WalletPointsLot } from './wallet-points-lot';

/** Wallet points lot list result schema exposed by the account service. */
export interface WalletPointsLotListResult {
  code: 0;
  data: { items: WalletPointsLot[]; pageInfo: PageInfo; };
  traceId: string;
}
