import { HttpClient } from './http/client';
import type { SdkworkAppConfig } from './types/common';
import type { AuthTokenManager } from '@sdkwork/sdk-common';
import { WalletApi } from './api/wallet';
import { BillingApi } from './api/billing';
import { AccountsApi } from './api/accounts';
import { TokenBankApi } from './api/token-bank';
export declare class SdkworkAccountAppClient {
    private httpClient;
    readonly wallet: WalletApi;
    readonly billing: BillingApi;
    readonly accounts: AccountsApi;
    readonly tokenBank: TokenBankApi;
    constructor(config: SdkworkAppConfig);
    setAuthToken(token: string): this;
    setAccessToken(token: string): this;
    setTokenManager(manager: AuthTokenManager): this;
    get http(): HttpClient;
}
export declare function createClient(config: SdkworkAppConfig): SdkworkAccountAppClient;
export default SdkworkAccountAppClient;
//# sourceMappingURL=sdk.d.ts.map