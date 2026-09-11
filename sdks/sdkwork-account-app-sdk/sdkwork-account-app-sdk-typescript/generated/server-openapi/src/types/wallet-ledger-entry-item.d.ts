export interface WalletLedgerEntryItem {
    id: string;
    uuid: string;
    accountId: string;
    tenantId: string;
    organizationId?: string;
    ownerUserId: string;
    assetType: string;
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
//# sourceMappingURL=wallet-ledger-entry-item.d.ts.map