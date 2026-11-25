import { backendApiPath } from './paths';
import type { ApiRequestOptions, HttpClient } from '../http/client';

import type { AuthorizeWalletBillingRequest, CreateAccountHoldRequest, CreateAccountTransferRequest, CreateWalletAdjustmentRequest, DispatchOutboxRequest, DispatchOutboxResult, ExpireExpiredHoldsRequest, ExpireExpiredHoldsResult, ExpirePointsLotsRequest, ExpirePointsLotsResult, PointsReconciliationRequest, PointsReconciliationResult, ReleaseAccountHoldRequest, SettleAccountHoldRequest, UpsertWalletBillingPolicyRequest, WalletAccountListData, WalletAdjustmentResult, WalletBillingAdmission, WalletBillingPolicy, WalletBillingPolicyListResponse, WalletHealthItem, WalletHoldMutationResult, WalletLedgerEntryListData, WalletPointsLotListData, WalletTransferMutationResult } from '../types';


export interface WalletLedgerEntriesListParams {
  organizationId?: string;
  ownerUserId?: string;
  ownerType?: 'USER' | 'ORGANIZATION';
  assetType?: 'cash' | 'points' | 'token_bank';
  accountId?: string;
  cursor?: string;
  page?: number;
  pageSize?: number;
}

export class WalletLedgerEntriesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


async list(params?: WalletLedgerEntriesListParams, requestOptions?: ApiRequestOptions): Promise<WalletLedgerEntryListData> {
    const query = buildQueryString([
      { name: 'organizationId', value: params?.organizationId, style: 'form', explode: true, allowReserved: false },
      { name: 'ownerUserId', value: params?.ownerUserId, style: 'form', explode: true, allowReserved: false },
      { name: 'ownerType', value: params?.ownerType, style: 'form', explode: true, allowReserved: false },
      { name: 'assetType', value: params?.assetType, style: 'form', explode: true, allowReserved: false },
      { name: 'accountId', value: params?.accountId, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<WalletLedgerEntryListData>(appendQueryString(backendApiPath(`/wallet/ledger_entries`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }
}

export interface WalletAccountsListParams {
  organizationId?: string;
  ownerUserId?: string;
  ownerType?: 'USER' | 'ORGANIZATION';
  assetType?: 'cash' | 'points' | 'token_bank';
}

export class WalletAccountsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


async list(params?: WalletAccountsListParams, requestOptions?: ApiRequestOptions): Promise<WalletAccountListData> {
    const query = buildQueryString([
      { name: 'organizationId', value: params?.organizationId, style: 'form', explode: true, allowReserved: false },
      { name: 'ownerUserId', value: params?.ownerUserId, style: 'form', explode: true, allowReserved: false },
      { name: 'ownerType', value: params?.ownerType, style: 'form', explode: true, allowReserved: false },
      { name: 'assetType', value: params?.assetType, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<WalletAccountListData>(appendQueryString(backendApiPath(`/wallet/accounts`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'data' });
  }
}

export interface WalletBillingPoliciesListParams {
  organizationId?: string;
  scopeType?: number;
  status?: number;
  page?: number;
  pageSize?: number;
}

export interface WalletBillingPoliciesResolveParams {
  organizationId?: string;
  ownerUserId: string;
  apiKeyId?: string;
}

export class WalletBillingPoliciesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


async list(params?: WalletBillingPoliciesListParams, requestOptions?: ApiRequestOptions): Promise<WalletBillingPolicyListResponse> {
    const query = buildQueryString([
      { name: 'organizationId', value: params?.organizationId, style: 'form', explode: true, allowReserved: false },
      { name: 'scopeType', value: params?.scopeType, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<WalletBillingPolicyListResponse>(appendQueryString(backendApiPath(`/wallet/billing_policies`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

async create(body: UpsertWalletBillingPolicyRequest, requestOptions?: ApiRequestOptions): Promise<WalletBillingPolicy | null> {
    return this.client.request<WalletBillingPolicy | null>(backendApiPath(`/wallet/billing_policies`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

async activate(policyId: string, requestOptions?: ApiRequestOptions): Promise<WalletBillingPolicy | null> {
    return this.client.request<WalletBillingPolicy | null>(backendApiPath(`/wallet/billing_policies/${serializePathParameter(policyId, { name: 'policyId', style: 'simple', explode: false })}/activate`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, sdkworkUnwrapKind: 'item' });
  }

async deactivate(policyId: string, requestOptions?: ApiRequestOptions): Promise<WalletBillingPolicy | null> {
    return this.client.request<WalletBillingPolicy | null>(backendApiPath(`/wallet/billing_policies/${serializePathParameter(policyId, { name: 'policyId', style: 'simple', explode: false })}/deactivate`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, sdkworkUnwrapKind: 'item' });
  }

async resolve(params: WalletBillingPoliciesResolveParams, requestOptions?: ApiRequestOptions): Promise<WalletBillingPolicy | null> {
    const query = buildQueryString([
      { name: 'organizationId', value: params.organizationId, style: 'form', explode: true, allowReserved: false },
      { name: 'ownerUserId', value: params.ownerUserId, style: 'form', explode: true, allowReserved: false },
      { name: 'apiKeyId', value: params.apiKeyId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<WalletBillingPolicy | null>(appendQueryString(backendApiPath(`/wallet/billing_policy`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

async authorize(body: AuthorizeWalletBillingRequest, requestOptions?: ApiRequestOptions): Promise<WalletBillingAdmission> {
    return this.client.request<WalletBillingAdmission>(backendApiPath(`/wallet/billing_policy/authorize`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class WalletTransfersApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


async create(body: CreateAccountTransferRequest, requestOptions?: ApiRequestOptions): Promise<WalletTransferMutationResult> {
    return this.client.request<WalletTransferMutationResult>(backendApiPath(`/wallet/transfers`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class WalletHoldsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


async create(body: CreateAccountHoldRequest, requestOptions?: ApiRequestOptions): Promise<WalletHoldMutationResult> {
    return this.client.request<WalletHoldMutationResult>(backendApiPath(`/wallet/holds`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

async settle(holdId: string, body: SettleAccountHoldRequest, requestOptions?: ApiRequestOptions): Promise<WalletHoldMutationResult> {
    return this.client.request<WalletHoldMutationResult>(backendApiPath(`/wallet/holds/${serializePathParameter(holdId, { name: 'holdId', style: 'simple', explode: false })}/settle`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

async release(holdId: string, body: ReleaseAccountHoldRequest, requestOptions?: ApiRequestOptions): Promise<WalletHoldMutationResult> {
    return this.client.request<WalletHoldMutationResult>(backendApiPath(`/wallet/holds/${serializePathParameter(holdId, { name: 'holdId', style: 'simple', explode: false })}/release`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

async expire(body: ExpireExpiredHoldsRequest, requestOptions?: ApiRequestOptions): Promise<ExpireExpiredHoldsResult> {
    return this.client.request<ExpireExpiredHoldsResult>(backendApiPath(`/wallet/holds/expire`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export interface WalletPointsLotsListParams {
  organizationId?: string;
  ownerUserId?: string;
  ownerType?: 'USER' | 'ORGANIZATION';
  page?: number;
  pageSize?: number;
}

export class WalletPointsLotsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


async expire(body: ExpirePointsLotsRequest, requestOptions?: ApiRequestOptions): Promise<ExpirePointsLotsResult> {
    return this.client.request<ExpirePointsLotsResult>(backendApiPath(`/wallet/points/lots/expire`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

async list(params?: WalletPointsLotsListParams, requestOptions?: ApiRequestOptions): Promise<WalletPointsLotListData> {
    const query = buildQueryString([
      { name: 'organizationId', value: params?.organizationId, style: 'form', explode: true, allowReserved: false },
      { name: 'ownerUserId', value: params?.ownerUserId, style: 'form', explode: true, allowReserved: false },
      { name: 'ownerType', value: params?.ownerType, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<WalletPointsLotListData>(appendQueryString(backendApiPath(`/wallet/points_lots`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }
}

export class WalletPointsApi {
  private client: HttpClient;
  public readonly lots: WalletPointsLotsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.lots = new WalletPointsLotsApi(client);
  }


async reconciliation(body: PointsReconciliationRequest, requestOptions?: ApiRequestOptions): Promise<PointsReconciliationResult> {
    return this.client.request<PointsReconciliationResult>(backendApiPath(`/wallet/points/reconciliation`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class WalletAdjustmentsPointsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


async create(body: CreateWalletAdjustmentRequest, requestOptions?: ApiRequestOptions): Promise<WalletAdjustmentResult> {
    return this.client.request<WalletAdjustmentResult>(backendApiPath(`/wallet/adjustments/points`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class WalletAdjustmentsCashApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


async create(body: CreateWalletAdjustmentRequest, requestOptions?: ApiRequestOptions): Promise<WalletAdjustmentResult> {
    return this.client.request<WalletAdjustmentResult>(backendApiPath(`/wallet/adjustments/cash`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class WalletAdjustmentsApi {
  private client: HttpClient;
  public readonly cash: WalletAdjustmentsCashApi;
  public readonly points: WalletAdjustmentsPointsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.cash = new WalletAdjustmentsCashApi(client);
    this.points = new WalletAdjustmentsPointsApi(client);
  }


async create(body: CreateWalletAdjustmentRequest, requestOptions?: ApiRequestOptions): Promise<WalletAdjustmentResult> {
    return this.client.request<WalletAdjustmentResult>(backendApiPath(`/wallet/adjustments`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class WalletOutboxApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


async dispatch(body?: DispatchOutboxRequest, requestOptions?: ApiRequestOptions): Promise<DispatchOutboxResult> {
    return this.client.request<DispatchOutboxResult>(backendApiPath(`/wallet/outbox/dispatch`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, ...(body !== undefined ? { body, contentType: 'application/json' } : {}), sdkworkUnwrapKind: 'item' });
  }
}

export class WalletHealthApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


async retrieve(requestOptions?: ApiRequestOptions): Promise<WalletHealthItem> {
    return this.client.request<WalletHealthItem>(backendApiPath(`/wallet/health`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class WalletApi {
  public readonly health: WalletHealthApi;
  public readonly outbox: WalletOutboxApi;
  public readonly adjustments: WalletAdjustmentsApi;
  public readonly points: WalletPointsApi;
  public readonly holds: WalletHoldsApi;
  public readonly transfers: WalletTransfersApi;
  public readonly billingPolicies: WalletBillingPoliciesApi;
  public readonly accounts: WalletAccountsApi;
  public readonly ledgerEntries: WalletLedgerEntriesApi;

  constructor(client: HttpClient) {
    this.health = new WalletHealthApi(client);
    this.outbox = new WalletOutboxApi(client);
    this.adjustments = new WalletAdjustmentsApi(client);
    this.points = new WalletPointsApi(client);
    this.holds = new WalletHoldsApi(client);
    this.transfers = new WalletTransfersApi(client);
    this.billingPolicies = new WalletBillingPoliciesApi(client);
    this.accounts = new WalletAccountsApi(client);
    this.ledgerEntries = new WalletLedgerEntriesApi(client);
  }

}

export function createWalletApi(client: HttpClient): WalletApi {
  return new WalletApi(client);
}

function appendQueryString(path: string, rawQueryString: string): string {
  const query = rawQueryString.replace(/^\?+/, '');
  if (!query) {
    return path;
  }
  return path.includes('?') ? `${path}&${query}` : `${path}?${query}`;
}

interface PathParameterSpec {
  name: string;
  style: string;
  explode: boolean;
}

function serializePathParameter(value: unknown, spec: PathParameterSpec): string {
  if (value === undefined || value === null) {
    return '';
  }

  const style = spec.style || 'simple';
  if (Array.isArray(value)) {
    return serializePathArray(spec.name, value, style, spec.explode);
  }
  if (typeof value === 'object') {
    return serializePathObject(spec.name, value as Record<string, unknown>, style, spec.explode);
  }
  return pathPrefix(spec.name, style, false) + encodePathValue(serializePathPrimitive(value));
}

function serializePathArray(name: string, values: unknown[], style: string, explode: boolean): string {
  const serialized = values
    .filter((item) => item !== undefined && item !== null)
    .map((item) => encodePathValue(serializePathPrimitive(item)));
  if (serialized.length === 0) {
    return pathPrefix(name, style, false);
  }
  if (style === 'matrix') {
    return explode
      ? serialized.map((item) => `;${name}=${item}`).join('')
      : `;${name}=${serialized.join(',')}`;
  }
  return pathPrefix(name, style, false) + serialized.join(explode ? '.' : ',');
}

function serializePathObject(name: string, value: Record<string, unknown>, style: string, explode: boolean): string {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (entries.length === 0) {
    return pathPrefix(name, style, true);
  }
  if (style === 'matrix') {
    return explode
      ? entries.map(([key, entryValue]) => `;${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join('')
      : `;${name}=${entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',')}`;
  }
  const serialized = explode
    ? entries.map(([key, entryValue]) => `${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join(style === 'label' ? '.' : ',')
    : entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',');
  return pathPrefix(name, style, true) + serialized;
}

function pathPrefix(name: string, style: string, _objectValue: boolean): string {
  if (style === 'label') return '.';
  if (style === 'matrix') return `;${name}`;
  return '';
}

function encodePathValue(value: string): string {
  return encodeURIComponent(value);
}

function serializePathPrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}
interface QueryParameterSpec {
  name: string;
  value: unknown;
  style: string;
  explode: boolean;
  allowReserved: boolean;
  contentType?: string;
}

function buildQueryString(parameters: QueryParameterSpec[]): string {
  const pairs: string[] = [];
  for (const parameter of parameters) {
    appendSerializedParameter(pairs, parameter);
  }
  return pairs.join('&');
}

function appendSerializedParameter(pairs: string[], parameter: QueryParameterSpec): void {
  if (parameter.value === undefined || parameter.value === null) {
    return;
  }

  if (parameter.contentType) {
    pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(JSON.stringify(parameter.value), parameter.allowReserved)}`);
    return;
  }

  const style = parameter.style || 'form';
  if (style === 'deepObject') {
    appendDeepObjectParameter(pairs, parameter.name, parameter.value, parameter.allowReserved);
    return;
  }

  if (Array.isArray(parameter.value)) {
    appendArrayParameter(pairs, parameter.name, parameter.value, style, parameter.explode, parameter.allowReserved);
    return;
  }

  if (typeof parameter.value === 'object') {
    appendObjectParameter(pairs, parameter.name, parameter.value as Record<string, unknown>, style, parameter.explode, parameter.allowReserved);
    return;
  }

  pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(serializePrimitive(parameter.value), parameter.allowReserved)}`);
}

function appendArrayParameter(
  pairs: string[],
  name: string,
  value: unknown[],
  style: string,
  explode: boolean,
  allowReserved: boolean,
): void {
  const values = value
    .filter((item) => item !== undefined && item !== null)
    .map((item) => serializePrimitive(item));
  if (values.length === 0) {
    return;
  }

  if (style === 'form' && explode) {
    for (const item of values) {
      pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(item, allowReserved)}`);
    }
    return;
  }

  pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(values.join(','), allowReserved)}`);
}

function appendObjectParameter(
  pairs: string[],
  name: string,
  value: Record<string, unknown>,
  style: string,
  explode: boolean,
  allowReserved: boolean,
): void {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (entries.length === 0) {
    return;
  }

  if (style === 'form' && explode) {
    for (const [key, entryValue] of entries) {
      pairs.push(`${encodeQueryComponent(key)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
    }
    return;
  }

  const serialized = entries.flatMap(([key, entryValue]) => [key, serializePrimitive(entryValue)]).join(',');
  pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serialized, allowReserved)}`);
}

function appendDeepObjectParameter(
  pairs: string[],
  name: string,
  value: unknown,
  allowReserved: boolean,
): void {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serializePrimitive(value), allowReserved)}`);
    return;
  }

  for (const [key, entryValue] of Object.entries(value as Record<string, unknown>)) {
    if (entryValue === undefined || entryValue === null) {
      continue;
    }
    pairs.push(`${encodeQueryComponent(`${name}[${key}]`)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
  }
}

function serializePrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}

function encodeQueryComponent(value: string): string {
  return encodeURIComponent(value);
}

function encodeQueryValue(value: string, allowReserved: boolean): string {
  const encoded = encodeURIComponent(value);
  if (!allowReserved) {
    return encoded;
  }
  return encoded.replace(/%3A/gi, ':')
    .replace(/%2F/gi, '/')
    .replace(/%3F/gi, '?')
    .replace(/%23/gi, '#')
    .replace(/%5B/gi, '[')
    .replace(/%5D/gi, ']')
    .replace(/%40/gi, '@')
    .replace(/%21/gi, '!')
    .replace(/%24/gi, '$')
    .replace(/%26/gi, '&')
    .replace(/%27/gi, "'")
    .replace(/%28/gi, '(')
    .replace(/%29/gi, ')')
    .replace(/%2A/gi, '*')
    .replace(/%2B/gi, '+')
    .replace(/%2C/gi, ',')
    .replace(/%3B/gi, ';')
    .replace(/%3D/gi, '=');
}
