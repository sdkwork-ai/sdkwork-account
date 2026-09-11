export interface AccountHoldItem {
    id: string;
    uuid: string;
    tenantId: string;
    organizationId?: string;
    accountId: string;
    ownerUserId: string;
    assetType: string;
    amount: string;
    settledAmount: string;
    releasedAmount: string;
    status: string;
    businessType: string;
    businessNo: string;
    sourceType: string;
    sourceId: string;
    requestNo: string;
    idempotencyKey: string;
    expiresAt?: string;
    settledAt?: string;
    releasedAt?: string;
    version: string;
    createdAt: string;
    updatedAt: string;
}
//# sourceMappingURL=account-hold-item.d.ts.map