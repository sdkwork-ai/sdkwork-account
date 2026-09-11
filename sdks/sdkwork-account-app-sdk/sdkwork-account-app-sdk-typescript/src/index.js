import { createClient as createGeneratedAppClient, SdkworkAccountAppClient, } from '../generated/server-openapi/src/index';
export { SdkworkAccountAppClient, SdkworkAccountAppClient as SdkworkAppClient, createGeneratedAppClient, };
export * from '../generated/server-openapi/src/types';
export * from '../generated/server-openapi/src/api';
export * from '../generated/server-openapi/src/http';
export * from '../generated/server-openapi/src/auth';
export function createClient(config) {
    return createGeneratedAppClient(config);
}
//# sourceMappingURL=index.js.map