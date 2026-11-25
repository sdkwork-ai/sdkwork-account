export interface WalletAccountSummary {
  id: string;
  uuid: string;
  tenantId: string;
  organizationId?: string | null;
  ownerUserId: string;
  assetType: 'cash' | 'points' | 'token_bank';
  currencyCode?: string | null;
  availableAmount: string;
  frozenAmount: string;
  pendingAmount: string;
  status: string;
  /** Optimistic version of the account row. */
  version: string;
}
