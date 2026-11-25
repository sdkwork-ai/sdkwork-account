export interface WalletPointsLot {
  id: string;
  uuid: string;
  accountId: string;
  /** Points granted by this lot. */
  grantedAmount: string;
  /** Points still spendable in this lot. */
  remainingAmount: string;
  sourceType: string;
  sourceId: string;
  expiresAt?: string | null;
  status: string;
  createdAt: string;
  updatedAt: string;
}
