import { appApiPath } from './paths';
export class AccountsCurrentSummaryApi {
    client;
    constructor(client) {
        this.client = client;
    }
    async retrieve(requestOptions) {
        return this.client.request(appApiPath(`/accounts/current/summary`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'item' });
    }
}
export class AccountsCurrentApi {
    summary;
    constructor(client) {
        this.summary = new AccountsCurrentSummaryApi(client);
    }
}
export class AccountsApi {
    current;
    constructor(client) {
        this.current = new AccountsCurrentApi(client);
    }
}
export function createAccountsApi(client) {
    return new AccountsApi(client);
}
//# sourceMappingURL=accounts.js.map