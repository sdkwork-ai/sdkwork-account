export interface WalletLedgerEntry {
  id: string;
  uuid: string;
  accountId: string;
  tenantId: string;
  organizationId?: string | null;
  ownerUserId: string;
  assetType: 'cash' | 'points' | 'token_bank';
  direction: 'credit' | 'debit';
  amount: string;
  balanceBefore: string;
  balanceAfter: string;
  businessType: string;
  transactionNo: string;
  requestNo: string;
  idempotencyKey: string;
  createdAt: string;
}
