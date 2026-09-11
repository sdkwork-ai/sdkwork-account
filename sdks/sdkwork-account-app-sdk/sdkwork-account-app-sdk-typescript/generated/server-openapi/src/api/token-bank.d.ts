import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { AccountHoldListData, TokenBankBalanceItem, WalletAccountItem, WalletLedgerListData } from '../types';
export interface TokenBankHoldsListParams {
    accountId?: string;
    status?: string;
    page?: string;
    pageSize?: string;
}
export declare class TokenBankHoldsApi {
    private client;
    constructor(client: HttpClient);
    list(params?: TokenBankHoldsListParams, requestOptions?: ApiRequestOptions): Promise<AccountHoldListData>;
}
export interface TokenBankLedgerEntriesListParams {
    accountId?: string;
    page?: string;
    pageSize?: string;
    cursor?: string;
}
export declare class TokenBankLedgerEntriesApi {
    private client;
    constructor(client: HttpClient);
    list(params?: TokenBankLedgerEntriesListParams, requestOptions?: ApiRequestOptions): Promise<WalletLedgerListData>;
}
export declare class TokenBankOverviewApi {
    private client;
    constructor(client: HttpClient);
    retrieve(requestOptions?: ApiRequestOptions): Promise<TokenBankBalanceItem>;
}
export declare class TokenBankAccountApi {
    private client;
    constructor(client: HttpClient);
    retrieve(requestOptions?: ApiRequestOptions): Promise<WalletAccountItem>;
}
export declare class TokenBankApi {
    readonly account: TokenBankAccountApi;
    readonly overview: TokenBankOverviewApi;
    readonly ledgerEntries: TokenBankLedgerEntriesApi;
    readonly holds: TokenBankHoldsApi;
    constructor(client: HttpClient);
}
export declare function createTokenBankApi(client: HttpClient): TokenBankApi;
//# sourceMappingURL=token-bank.d.ts.map