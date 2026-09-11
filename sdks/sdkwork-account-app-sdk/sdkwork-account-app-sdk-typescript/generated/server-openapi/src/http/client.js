import { BaseHttpClient, buildAuthHeaders, withRetry } from '@sdkwork/sdk-common';
import { sha256Hash } from '@sdkwork/utils';
export class HttpClient extends BaseHttpClient {
    static ACCESS_TOKEN_HEADER = 'Access-Token';
    static SDKWORK_V3_UNWRAP = true;
    static SDKWORK_V3_REQUEST_FINGERPRINTS = true;
    static REQUIRES_SDKWORK_ACCESS_TOKEN = true;
    constructor(config) {
        super(config);
    }
    static normalizeCredential(value) {
        return typeof value === 'string' && value.trim().length > 0 ? value.trim() : undefined;
    }
    getInternalAuthConfig() {
        const self = this;
        self.authConfig = self.authConfig || {};
        return self.authConfig;
    }
    getInternalHeaders() {
        const self = this;
        self.config = self.config || {};
        self.config.headers = self.config.headers || {};
        return self.config.headers;
    }
    buildRequestHeaders(headers, contentType) {
        const mergedHeaders = {
            ...(headers ?? {}),
        };
        if (contentType && contentType.toLowerCase() !== 'multipart/form-data') {
            mergedHeaders['Content-Type'] = contentType;
        }
        return Object.keys(mergedHeaders).length > 0 ? mergedHeaders : undefined;
    }
    async applySdkworkRequestBodyFingerprint(headers, body) {
        if (!HttpClient.SDKWORK_V3_REQUEST_FINGERPRINTS
            || body == null
            || !this.hasNonEmptyHeader(headers, 'Idempotency-Key')
            || this.hasNonEmptyHeader(headers, 'X-Content-SHA256')
            || this.hasNonEmptyHeader(headers, 'X-Idempotency-Fingerprint')) {
            return headers;
        }
        const fingerprint = await this.createSdkworkRequestBodyFingerprint(body);
        if (!fingerprint) {
            return headers;
        }
        const normalizedFingerprintHeader = fingerprint.header.toLowerCase();
        const preparedHeaders = Object.fromEntries(Object.entries(headers ?? {}).filter(([headerName]) => headerName.toLowerCase() !== normalizedFingerprintHeader));
        return {
            ...preparedHeaders,
            [fingerprint.header]: fingerprint.value,
        };
    }
    hasNonEmptyHeader(headers, name) {
        const normalizedName = name.toLowerCase();
        return Object.entries(headers ?? {}).some(([headerName, value]) => headerName.toLowerCase() === normalizedName && value.trim().length > 0);
    }
    async createSdkworkRequestBodyFingerprint(body) {
        if (typeof FormData !== 'undefined' && body instanceof FormData) {
            const canonicalForm = await this.serializeSdkworkFormData(body);
            return {
                header: 'X-Idempotency-Fingerprint',
                value: await this.sha256Hex(new TextEncoder().encode(canonicalForm)),
            };
        }
        const bytes = await this.serializeSdkworkRequestBodyBytes(body);
        if (!bytes) {
            return undefined;
        }
        return {
            header: 'X-Content-SHA256',
            value: await this.sha256Hex(bytes),
        };
    }
    async serializeSdkworkRequestBodyBytes(body) {
        if (typeof URLSearchParams !== 'undefined' && body instanceof URLSearchParams) {
            return new TextEncoder().encode(body.toString());
        }
        if (typeof Blob !== 'undefined' && body instanceof Blob) {
            return new Uint8Array(await body.arrayBuffer());
        }
        if (typeof ArrayBuffer !== 'undefined' && body instanceof ArrayBuffer) {
            return new Uint8Array(body.slice(0));
        }
        if (typeof ArrayBuffer !== 'undefined' && ArrayBuffer.isView(body)) {
            return new Uint8Array(new Uint8Array(body.buffer, body.byteOffset, body.byteLength));
        }
        if (typeof body === 'string') {
            return new TextEncoder().encode(body);
        }
        const serialized = JSON.stringify(body);
        return serialized === undefined ? undefined : new TextEncoder().encode(serialized);
    }
    async serializeSdkworkFormData(body) {
        const parts = [];
        for (const [name, value] of body.entries()) {
            if (typeof value === 'string') {
                parts.push({ kind: 'field', name, value });
                continue;
            }
            const bytes = new Uint8Array(await value.arrayBuffer());
            parts.push({
                kind: 'file',
                name,
                fileName: 'name' in value ? String(value.name) : '',
                contentType: value.type,
                size: value.size,
                contentSha256: await this.sha256Hex(bytes),
            });
        }
        return JSON.stringify(parts);
    }
    async sha256Hex(bytes) {
        return sha256Hash(bytes);
    }
    buildHeaders(config, skipAuth = false) {
        const headers = super.buildHeaders(config, skipAuth);
        if (config?.accessTokenOnly) {
            this.stripCredentialHeaders(headers, true);
            return headers;
        }
        if (!skipAuth && !config?.skipAuth) {
            return headers;
        }
        this.stripCredentialHeaders(headers, false);
        return headers;
    }
    stripCredentialHeaders(headers, preserveAccessToken) {
        [
            ...(preserveAccessToken ? [] : [HttpClient.ACCESS_TOKEN_HEADER, 'Access-Token']),
            'Authorization',
            ['X', 'API', 'Key'].join('-'),
            'X-Tenant-Id',
            'X-App-Id',
            'X-Organization-Id',
            'X-Platform',
            'X-User-Id',
            'X-Sdkwork-Tenant-Id',
            'X-Sdkwork-App-Id',
            'X-Sdkwork-User-Id',
            'X-Sdkwork-Organization-Id',
            'X-Sdkwork-Actor-Id',
            'X-Sdkwork-Actor-Kind',
            'X-Sdkwork-Session-Id',
            'X-Sdkwork-Environment',
            'X-Sdkwork-Deployment-Profile',
            'X-Sdkwork-Deployment-Mode',
            'X-Sdkwork-Runtime-Target',
            'X-Sdkwork-Auth-Level',
            'X-Sdkwork-Data-Scope',
            'X-Sdkwork-Permission-Scope',
            'X-Sdkwork-Device-Id',
            'X-Sdkwork-Context-Signature',
            'X-Sdkwork-Operation-Id',
            'X-Sdkwork-Subject-Tenant-Id',
            'X-Sdkwork-Subject-Organization-Id',
            'X-Sdkwork-Subject-User-Id',
            'X-Sdkwork-Subject-Timestamp',
            'X-Sdkwork-Subject-Signature',
        ].forEach((key) => {
            delete headers[key];
        });
    }
    buildRequestBody(body, contentType) {
        if (body == null) {
            return body;
        }
        const normalizedContentType = (contentType ?? '').toLowerCase();
        if (normalizedContentType === 'application/x-www-form-urlencoded') {
            return this.encodeFormBody(body);
        }
        if (normalizedContentType === 'multipart/form-data') {
            return this.encodeMultipartBody(body);
        }
        return body;
    }
    encodeMultipartBody(body) {
        if (body instanceof FormData) {
            return body;
        }
        const formData = new FormData();
        if (body instanceof Map) {
            for (const [key, value] of body.entries()) {
                this.appendMultipartValue(formData, String(key), value);
            }
            return formData;
        }
        if (typeof body === 'object') {
            const record = body;
            for (const [key, value] of Object.entries(record)) {
                if (this.isMultipartMetadataField(key)) {
                    continue;
                }
                this.appendMultipartValue(formData, key, value, this.resolveMultipartFileName(record, key));
            }
            return formData;
        }
        this.appendMultipartValue(formData, 'value', body);
        return formData;
    }
    appendMultipartValue(formData, key, value, fileName) {
        if (value == null) {
            return;
        }
        if (Array.isArray(value)) {
            value.forEach((item) => this.appendMultipartValue(formData, key, item, fileName));
            return;
        }
        if (value instanceof Blob) {
            if (fileName) {
                formData.append(key, value, fileName);
                return;
            }
            formData.append(key, value);
            return;
        }
        if (value instanceof Date) {
            formData.append(key, value.toISOString());
            return;
        }
        if (typeof value === 'object') {
            formData.append(key, JSON.stringify(value));
            return;
        }
        formData.append(key, String(value));
    }
    resolveMultipartFileName(record, key) {
        const fieldSpecificName = record[`${key}FileName`];
        if (typeof fieldSpecificName === 'string' && fieldSpecificName.trim()) {
            return fieldSpecificName.trim();
        }
        const genericName = record.fileName;
        if (key === 'file' && typeof genericName === 'string' && genericName.trim()) {
            return genericName.trim();
        }
        return undefined;
    }
    isMultipartMetadataField(key) {
        return key === 'fileName' || key.endsWith('FileName');
    }
    encodeFormBody(body) {
        if (body instanceof URLSearchParams) {
            return body.toString();
        }
        if (typeof body === 'string') {
            return body;
        }
        const params = new URLSearchParams();
        if (body instanceof Map) {
            for (const [key, value] of body.entries()) {
                this.appendFormValue(params, String(key), value);
            }
            return params.toString();
        }
        if (typeof body === 'object') {
            for (const [key, value] of Object.entries(body)) {
                this.appendFormValue(params, key, value);
            }
            return params.toString();
        }
        params.append('value', String(body));
        return params.toString();
    }
    appendFormValue(params, key, value) {
        if (value == null) {
            return;
        }
        if (Array.isArray(value)) {
            value.forEach((item) => this.appendFormValue(params, key, item));
            return;
        }
        if (value instanceof Date) {
            params.append(key, value.toISOString());
            return;
        }
        if (typeof value === 'object') {
            params.append(key, JSON.stringify(value));
            return;
        }
        params.append(key, String(value));
    }
    setAuthToken(token) {
        super.setAuthToken(token);
    }
    setAccessToken(token) {
        const headers = this.getInternalHeaders();
        headers[HttpClient.ACCESS_TOKEN_HEADER] = token;
        super.setAccessToken(token);
    }
    setTokenManager(manager) {
        const baseProto = Object.getPrototypeOf(HttpClient.prototype);
        if (typeof baseProto.setTokenManager === 'function') {
            baseProto.setTokenManager.call(this, manager);
            return;
        }
        this.getInternalAuthConfig().tokenManager = manager;
    }
    applyAccessTokenOnlyHeaders(headers) {
        const authConfig = this.getInternalAuthConfig();
        const tokenManager = authConfig.tokenManager;
        const accessToken = tokenManager?.getAccessToken?.();
        if (typeof accessToken !== 'string' || accessToken.trim().length === 0) {
            throw new Error('access-token-only request requires Access-Token before request dispatch');
        }
        const result = { ...(headers ?? {}) };
        this.stripCredentialHeaders(result, false);
        result[HttpClient.ACCESS_TOKEN_HEADER] = accessToken.trim();
        return result;
    }
    applySdkworkAuthHeaders(headers) {
        const authConfig = this.getInternalAuthConfig();
        const tokenManager = authConfig.tokenManager;
        const accessToken = HttpClient.normalizeCredential(tokenManager?.getAccessToken?.());
        const authToken = HttpClient.normalizeCredential(tokenManager?.getAuthToken?.());
        if (HttpClient.REQUIRES_SDKWORK_ACCESS_TOKEN
            && (typeof accessToken !== 'string' || accessToken.trim().length === 0)) {
            throw new Error('non-open-api request requires Access-Token before request dispatch');
        }
        if (!accessToken && !authToken) {
            return headers;
        }
        const authHeaders = buildAuthHeaders('dual-token', undefined, tokenManager);
        return Object.keys(authHeaders).length > 0
            ? { ...(headers ?? {}), ...authHeaders }
            : headers;
    }
    unwrapSdkworkV3Payload(payload, unwrapKind = 'data') {
        if (!HttpClient.SDKWORK_V3_UNWRAP || payload == null || typeof payload !== 'object') {
            return payload;
        }
        const record = payload;
        if (record.code !== 0 || !('data' in record)) {
            return this.unwrapSdkworkV3Data(record, unwrapKind);
        }
        const data = record.data;
        if (!data || typeof data !== 'object') {
            return data;
        }
        return this.unwrapSdkworkV3Data(data, unwrapKind);
    }
    unwrapSdkworkV3Data(data, unwrapKind) {
        if (unwrapKind === 'void') {
            return undefined;
        }
        if (unwrapKind === 'item' && 'item' in data) {
            return data.item;
        }
        return data;
    }
    async request(path, options = {}) {
        const execute = this.execute;
        if (typeof execute !== 'function') {
            throw new Error('BaseHttpClient execute method is not available');
        }
        const { body, headers, contentType, method = 'GET', skipAuth, accessTokenOnly, sdkworkUnwrapKind = 'data', ...rest } = options;
        const requestHeaders = accessTokenOnly
            ? this.applyAccessTokenOnlyHeaders(headers)
            : skipAuth
                ? headers
                : this.applySdkworkAuthHeaders(headers);
        const requestBody = this.buildRequestBody(body, contentType);
        const preparedHeaders = await this.applySdkworkRequestBodyFingerprint(this.buildRequestHeaders(requestHeaders, body == null ? undefined : contentType), requestBody);
        const payload = await withRetry(() => execute.call(this, {
            url: path,
            method,
            ...rest,
            ...(skipAuth !== undefined ? { skipAuth } : {}),
            ...(accessTokenOnly !== undefined ? { accessTokenOnly } : {}),
            ...(requestBody !== undefined ? { body: requestBody } : {}),
            ...(preparedHeaders !== undefined ? { headers: preparedHeaders } : {}),
        }), 
        // Per-request retry overrides (e.g. disabling 5xx retries for
        // idempotent-terminal operations like turn execution) flow through
        // options.retry; the default keeps maxRetries: 3.
        { maxRetries: 3, ...options.retry });
        return this.unwrapSdkworkV3Payload(payload, sdkworkUnwrapKind);
    }
    async *streamJson(path, options = {}) {
        const stream = BaseHttpClient.prototype.stream;
        if (typeof stream !== 'function') {
            throw new Error('BaseHttpClient stream method is not available');
        }
        const { body, headers, contentType, method = 'GET', skipAuth, accessTokenOnly, ...rest } = options;
        const authHeaders = accessTokenOnly
            ? this.applyAccessTokenOnlyHeaders(headers)
            : skipAuth
                ? headers
                : this.applySdkworkAuthHeaders(headers);
        const requestBody = this.buildRequestBody(body, contentType);
        const requestHeaders = await this.applySdkworkRequestBodyFingerprint(this.buildRequestHeaders({ Accept: 'text/event-stream', ...(authHeaders ?? {}) }, body == null ? undefined : contentType), requestBody);
        for await (const data of stream.call(this, path, {
            method,
            ...rest,
            ...(skipAuth !== undefined ? { skipAuth } : {}),
            ...(accessTokenOnly !== undefined ? { accessTokenOnly } : {}),
            ...(requestBody !== undefined ? { body: requestBody } : {}),
            ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}),
        })) {
            if (data === '[DONE]') {
                return;
            }
            if (typeof data !== 'string' || data.trim().length === 0) {
                continue;
            }
            yield JSON.parse(data);
        }
    }
    async get(path, params, headers) {
        return this.request(path, {
            method: 'GET',
            ...(params !== undefined ? { params } : {}),
            ...(headers !== undefined ? { headers } : {}),
        });
    }
    async post(path, body, params, headers, contentType) {
        return this.request(path, {
            method: 'POST',
            ...(body !== undefined ? { body } : {}),
            ...(params !== undefined ? { params } : {}),
            ...(headers !== undefined ? { headers } : {}),
            ...(contentType !== undefined ? { contentType } : {}),
        });
    }
    async put(path, body, params, headers, contentType) {
        return this.request(path, {
            method: 'PUT',
            ...(body !== undefined ? { body } : {}),
            ...(params !== undefined ? { params } : {}),
            ...(headers !== undefined ? { headers } : {}),
            ...(contentType !== undefined ? { contentType } : {}),
        });
    }
    async delete(path, params, headers) {
        return this.request(path, {
            method: 'DELETE',
            ...(params !== undefined ? { params } : {}),
            ...(headers !== undefined ? { headers } : {}),
        });
    }
    async patch(path, body, params, headers, contentType) {
        return this.request(path, {
            method: 'PATCH',
            ...(body !== undefined ? { body } : {}),
            ...(params !== undefined ? { params } : {}),
            ...(headers !== undefined ? { headers } : {}),
            ...(contentType !== undefined ? { contentType } : {}),
        });
    }
}
export function createHttpClient(config) {
    return new HttpClient(config);
}
//# sourceMappingURL=client.js.map