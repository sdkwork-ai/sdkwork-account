import { appApiPath } from './paths';
export class BillingHistoryApi {
    client;
    constructor(client) {
        this.client = client;
    }
    async list(requestOptions) {
        return this.client.request(appApiPath(`/billing/history`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
}
export class BillingApi {
    history;
    constructor(client) {
        this.history = new BillingHistoryApi(client);
    }
}
export function createBillingApi(client) {
    return new BillingApi(client);
}
//# sourceMappingURL=billing.js.map