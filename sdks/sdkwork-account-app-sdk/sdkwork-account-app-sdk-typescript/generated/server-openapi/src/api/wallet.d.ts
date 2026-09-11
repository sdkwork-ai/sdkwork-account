import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { AccountHoldItem, AccountHoldListData, CashAccountItem, PointsAccountItem, PointsLotAllocationListData, PointsLotListData, PointsSummaryItem, WalletAccountListData, WalletLedgerEntryItem, WalletLedgerListData, WalletOverviewItem, WalletPortfolioItem } from '../types';
export declare class WalletPortfolioApi {
    private client;
    constructor(client: HttpClient);
    /** Retrieve the aggregated wallet portfolio (cash account, token bank and compute points) in one call */
    list(requestOptions?: ApiRequestOptions): Promise<WalletPortfolioItem>;
}
export interface WalletHoldsListParams {
    accountId?: string;
    assetType?: 'cash' | 'points' | 'token_bank';
    status?: string;
    page?: string;
    pageSize?: string;
}
export declare class WalletHoldsApi {
    private client;
    constructor(client: HttpClient);
    list(params?: WalletHoldsListParams, requestOptions?: ApiRequestOptions): Promise<AccountHoldListData>;
    retrieve(holdId: string, requestOptions?: ApiRequestOptions): Promise<AccountHoldItem>;
}
export interface WalletPointsLotsListParams {
    accountId?: string;
    status?: string;
    page?: string;
    pageSize?: string;
    cursor?: string;
}
export declare class WalletPointsLotsApi {
    private client;
    constructor(client: HttpClient);
    list(params?: WalletPointsLotsListParams, requestOptions?: ApiRequestOptions): Promise<PointsLotListData>;
}
export declare class WalletPointsSummaryApi {
    private client;
    constructor(client: HttpClient);
    retrieve(requestOptions?: ApiRequestOptions): Promise<PointsSummaryItem>;
}
export declare class WalletPointsApi {
    readonly summary: WalletPointsSummaryApi;
    readonly lots: WalletPointsLotsApi;
    constructor(client: HttpClient);
}
export declare class WalletLedgerEntriesAllocationsApi {
    private client;
    constructor(client: HttpClient);
    list(ledgerEntryId: string, requestOptions?: ApiRequestOptions): Promise<PointsLotAllocationListData>;
}
export interface WalletLedgerEntriesPointsListParams {
    accountId?: string;
    page?: string;
    pageSize?: string;
    cursor?: string;
}
export declare class WalletLedgerEntriesPointsApi {
    private client;
    constructor(client: HttpClient);
    list(params?: WalletLedgerEntriesPointsListParams, requestOptions?: ApiRequestOptions): Promise<WalletLedgerListData>;
}
export interface WalletLedgerEntriesCashListParams {
    accountId?: string;
    page?: string;
    pageSize?: string;
    cursor?: string;
}
export declare class WalletLedgerEntriesCashApi {
    private client;
    constructor(client: HttpClient);
    list(params?: WalletLedgerEntriesCashListParams, requestOptions?: ApiRequestOptions): Promise<WalletLedgerListData>;
}
export interface WalletLedgerEntriesListParams {
    accountId?: string;
    assetType?: 'cash' | 'points' | 'token_bank';
    page?: string;
    pageSize?: string;
    cursor?: string;
}
export declare class WalletLedgerEntriesApi {
    private client;
    readonly cash: WalletLedgerEntriesCashApi;
    readonly points: WalletLedgerEntriesPointsApi;
    readonly allocations: WalletLedgerEntriesAllocationsApi;
    constructor(client: HttpClient);
    list(params?: WalletLedgerEntriesListParams, requestOptions?: ApiRequestOptions): Promise<WalletLedgerListData>;
    retrieve(ledgerEntryId: string, requestOptions?: ApiRequestOptions): Promise<WalletLedgerEntryItem>;
}
export declare class WalletAccountsPointsApi {
    private client;
    constructor(client: HttpClient);
    retrieve(requestOptions?: ApiRequestOptions): Promise<PointsAccountItem>;
}
export declare class WalletAccountsCashApi {
    private client;
    constructor(client: HttpClient);
    retrieve(requestOptions?: ApiRequestOptions): Promise<CashAccountItem>;
}
export interface WalletAccountsListParams {
    assetType?: 'cash' | 'points' | 'token_bank';
}
export declare class WalletAccountsApi {
    private client;
    readonly cash: WalletAccountsCashApi;
    readonly points: WalletAccountsPointsApi;
    constructor(client: HttpClient);
    list(params?: WalletAccountsListParams, requestOptions?: ApiRequestOptions): Promise<WalletAccountListData>;
}
export declare class WalletOverviewApi {
    private client;
    constructor(client: HttpClient);
    retrieve(requestOptions?: ApiRequestOptions): Promise<WalletOverviewItem>;
}
export declare class WalletApi {
    readonly overview: WalletOverviewApi;
    readonly accounts: WalletAccountsApi;
    readonly ledgerEntries: WalletLedgerEntriesApi;
    readonly points: WalletPointsApi;
    readonly holds: WalletHoldsApi;
    readonly portfolio: WalletPortfolioApi;
    constructor(client: HttpClient);
}
export declare function createWalletApi(client: HttpClient): WalletApi;
//# sourceMappingURL=wallet.d.ts.map