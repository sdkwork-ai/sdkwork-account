import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { AccountSummaryItem } from '../types';
export declare class AccountsCurrentSummaryApi {
    private client;
    constructor(client: HttpClient);
    retrieve(requestOptions?: ApiRequestOptions): Promise<AccountSummaryItem>;
}
export declare class AccountsCurrentApi {
    readonly summary: AccountsCurrentSummaryApi;
    constructor(client: HttpClient);
}
export declare class AccountsApi {
    readonly current: AccountsCurrentApi;
    constructor(client: HttpClient);
}
export declare function createAccountsApi(client: HttpClient): AccountsApi;
//# sourceMappingURL=accounts.d.ts.map