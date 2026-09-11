import { createHttpClient } from './http/client';
import { createWalletApi } from './api/wallet';
import { createBillingApi } from './api/billing';
import { createAccountsApi } from './api/accounts';
import { createTokenBankApi } from './api/token-bank';
export class SdkworkAccountAppClient {
    httpClient;
    wallet;
    billing;
    accounts;
    tokenBank;
    constructor(config) {
        this.httpClient = createHttpClient(config);
        this.wallet = createWalletApi(this.httpClient);
        this.billing = createBillingApi(this.httpClient);
        this.accounts = createAccountsApi(this.httpClient);
        this.tokenBank = createTokenBankApi(this.httpClient);
    }
    setAuthToken(token) {
        this.httpClient.setAuthToken(token);
        return this;
    }
    setAccessToken(token) {
        this.httpClient.setAccessToken(token);
        return this;
    }
    setTokenManager(manager) {
        this.httpClient.setTokenManager(manager);
        return this;
    }
    get http() {
        return this.httpClient;
    }
}
export function createClient(config) {
    return new SdkworkAccountAppClient(config);
}
export default SdkworkAccountAppClient;
//# sourceMappingURL=sdk.js.map