import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { BillingHistoryListData } from '../types';
export declare class BillingHistoryApi {
    private client;
    constructor(client: HttpClient);
    list(requestOptions?: ApiRequestOptions): Promise<BillingHistoryListData>;
}
export declare class BillingApi {
    readonly history: BillingHistoryApi;
    constructor(client: HttpClient);
}
export declare function createBillingApi(client: HttpClient): BillingApi;
//# sourceMappingURL=billing.d.ts.map