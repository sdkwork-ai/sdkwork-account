/**
 * Account centre administration service.
 *
 * The service owns orchestration and response normalization for the operator
 * surface; it never constructs a transport. A caller injects the generated
 * backend SDK client (or a fake with the same shape), which keeps the package
 * testable and satisfies the backend-UI rule that business code holds no raw
 * HTTP, no manual auth headers and no generated-SDK edits.
 *
 * Every amount crossing this boundary is a string: token-bank points are exact
 * integers and must not round through a JavaScript number. The one exception is
 * the estimate sent to the simulator, where the wire contract is an `int64`
 * number and the value is bounded well below the safe-integer limit.
 */

/** One wallet account row. */
export interface SdkworkAccountAdminWalletAccount {
  id: string;
  uuid: string;
  tenantId: string;
  organizationId: string | null;
  ownerUserId: string;
  assetType: "cash" | "points" | "token_bank";
  currencyCode: string | null;
  availableAmount: string;
  frozenAmount: string;
  pendingAmount: string;
  status: string;
  version: string;
}

/** Paging metadata shared by the list operations. */
export interface SdkworkAccountAdminPageInfo {
  mode: "offset" | "cursor";
  page?: number;
  pageSize?: number;
  totalItems?: string;
  hasMore?: boolean;
}

/** One ledger entry. */
export interface SdkworkAccountAdminLedgerEntry {
  id: string;
  uuid: string;
  accountId: string;
  tenantId: string;
  organizationId: string | null;
  ownerUserId: string;
  assetType: "cash" | "points" | "token_bank";
  direction: "credit" | "debit";
  amount: string;
  balanceBefore: string;
  balanceAfter: string;
  businessType: string;
  transactionNo: string;
  requestNo: string;
  idempotencyKey: string;
  createdAt: string;
}

/** One allowance points lot. */
export interface SdkworkAccountAdminPointsLot {
  id: string;
  uuid: string;
  accountId: string;
  grantedAmount: string;
  remainingAmount: string;
  sourceType: string;
  sourceId: string;
  expiresAt: string | null;
  status: string;
  createdAt: string;
  updatedAt: string;
}

/** Admission decision vocabulary returned by the simulator. */
export type SdkworkAccountAdmissionDecision = "admit" | "deferred" | "free" | "reject";

/** Result of the admission simulation. */
export interface SdkworkAccountAdminAdmission {
  decision: SdkworkAccountAdmissionDecision;
  holdPoints: string;
  rejectCode?: string | null;
  policyId?: string | null;
}

/** Subject a read or simulation applies to. */
export interface SdkworkAccountAdminScope {
  organizationId?: string | null;
  ownerUserId?: string;
  ownerType?: "USER" | "ORGANIZATION";
  assetType?: "cash" | "points" | "token_bank";
}

/** Paging request shared by the list operations. */
export interface SdkworkAccountAdminPageRequest {
  page?: number;
  pageSize?: number;
  cursor?: string;
}

/** The generated backend SDK surface this service consumes. */
/** One stored billing policy revision. */
export interface SdkworkAccountAdminBillingPolicy {
  id: string;
  uuid: string;
  tenantId: string;
  organizationId: string;
  policyCode: string;
  name: string;
  scopeType: number;
  scopeId: string;
  priority: number;
  revision: string;
  status: number;
  chargeMode: string;
  settlementMode: string;
  prechargeRatio: string;
  outputTokenCeiling: string;
  inputTokenSource: string;
  holdTtlSeconds: string;
  minHoldPoints: string;
  maxHoldShare: string;
  admissionMode: string;
  overdraftEnvelopePoints: string;
  creditLimitPoints: string;
  allowancePeriod: string | null;
  allowanceQuota: string | null;
  allowanceResetMode: string | null;
  overagePolicy: string;
  effectiveFrom: string;
  effectiveTo: string | null;
  updatedAt: string;
}

/** Fields an operator can set when storing a policy revision. */
export interface SdkworkAccountAdminBillingPolicyInput {
  organizationId?: string | null;
  scopeType: number;
  scopeId?: string;
  priority?: number;
  policyCode: string;
  name: string;
  chargeMode: string;
  settlementMode: string;
  prechargeRatio: string;
  outputTokenCeiling: number;
  holdTtlSeconds: number;
  minHoldPoints?: number;
  maxHoldShare: string;
  admissionMode: string;
  overdraftEnvelopePoints?: number;
  creditLimitPoints?: number;
  overagePolicy: string;
  effectiveFrom?: string | null;
  effectiveTo?: string | null;
}

export interface SdkworkAccountAdminClient {
  wallet: {
    walletAccounts: {
      list(params?: unknown): Promise<{ items: readonly SdkworkAccountAdminWalletAccount[] }>;
    };
    walletLedgerEntries: {
      list(params?: unknown): Promise<{
        items: readonly SdkworkAccountAdminLedgerEntry[];
        pageInfo: SdkworkAccountAdminPageInfo;
      }>;
    };
    pointsLots: {
      list(params?: unknown): Promise<{
        items: readonly SdkworkAccountAdminPointsLot[];
        pageInfo: SdkworkAccountAdminPageInfo;
      }>;
    };
    billingPolicies: {
      authorize(body: unknown): Promise<SdkworkAccountAdminAdmission>;
      list(params?: unknown): Promise<{
        items: readonly SdkworkAccountAdminBillingPolicy[];
        pageInfo: SdkworkAccountAdminPageInfo;
      }>;
      create(body: unknown): Promise<SdkworkAccountAdminBillingPolicy | null>;
      activate(policyId: string): Promise<SdkworkAccountAdminBillingPolicy | null>;
      deactivate(policyId: string): Promise<SdkworkAccountAdminBillingPolicy | null>;
    };
    billingPolicy: {
      resolve(params?: unknown): Promise<SdkworkAccountAdminBillingPolicy | null>;
    };
  };
}

/** Snapshot an operator sees on the account centre overview. */
export interface SdkworkAccountAdminOverview {
  accounts: readonly SdkworkAccountAdminWalletAccount[];
  /** Sum of available token-bank points across the accounts. */
  availablePoints: string;
  /** Sum of frozen token-bank points across the accounts. */
  frozenPoints: string;
  /** Frozen share of the wallet, in basis points, or `null` when it is empty. */
  frozenShareBps: number | null;
}

function toPoints(value: string | null | undefined): bigint {
  if (value === null || value === undefined) return 0n;
  const text = String(value).trim();
  if (text === "") return 0n;
  try {
    return BigInt(text);
  } catch {
    return 0n;
  }
}

/**
 * Builds the account centre service over an injected generated client.
 *
 * Reads are normalized here so views never do arithmetic on strings, and the
 * overview reports the frozen share because that figure is what the production
 * incident needed and nobody could see.
 */
export function createSdkworkAccountAdminService(client: SdkworkAccountAdminClient) {
  async function listAccounts(
    scope: SdkworkAccountAdminScope,
  ): Promise<readonly SdkworkAccountAdminWalletAccount[]> {
    const result = await client.wallet.walletAccounts.list({
      organizationId: scope.organizationId ?? undefined,
      ownerUserId: scope.ownerUserId,
      ownerType: scope.ownerType,
      assetType: scope.assetType,
    });
    return result.items;
  }

  return {
    listAccounts,

    /** Computes the account centre overview from the wallet rows. */
    async loadOverview(scope: SdkworkAccountAdminScope): Promise<SdkworkAccountAdminOverview> {
      const accounts = await listAccounts(scope);
      let available = 0n;
      let frozen = 0n;
      for (const account of accounts) {
        if (account.assetType !== "token_bank") continue;
        available += toPoints(account.availableAmount);
        frozen += toPoints(account.frozenAmount);
      }
      const total = available + frozen;
      return {
        accounts,
        availablePoints: available.toString(),
        frozenPoints: frozen.toString(),
        frozenShareBps: total > 0n ? Number((frozen * 10_000n) / total) : null,
      };
    },

    /** Lists ledger entries for one owner. */
    async listLedgerEntries(
      scope: SdkworkAccountAdminScope,
      page: SdkworkAccountAdminPageRequest = {},
    ): Promise<{
      items: readonly SdkworkAccountAdminLedgerEntry[];
      pageInfo: SdkworkAccountAdminPageInfo;
    }> {
      return client.wallet.walletLedgerEntries.list({
        organizationId: scope.organizationId ?? undefined,
        ownerUserId: scope.ownerUserId,
        ownerType: scope.ownerType,
        assetType: scope.assetType,
        cursor: page.cursor,
        page: page.page,
        // The generated client takes camelCase and maps it to the `page_size`
        // wire parameter, so the service must not send the wire name itself.
        pageSize: page.pageSize,
      });
    },

    /** Lists allowance points lots for one owner. */
    async listPointsLots(
      scope: SdkworkAccountAdminScope,
      page: SdkworkAccountAdminPageRequest = {},
    ): Promise<{
      items: readonly SdkworkAccountAdminPointsLot[];
      pageInfo: SdkworkAccountAdminPageInfo;
    }> {
      return client.wallet.pointsLots.list({
        organizationId: scope.organizationId ?? undefined,
        ownerUserId: scope.ownerUserId,
        page: page.page,
        pageSize: page.pageSize,
      });
    },

    /**
     * Asks the account side what it would decide for an estimated charge.
     *
     * This is the diagnostic that turns a support ticket ("the balance is
     * sufficient but the request failed") into one sentence: the decision, the
     * amount it would freeze, and the reject reason when it refuses.
     */
    async simulateAdmission(request: {
      scope: SdkworkAccountAdminScope;
      estimatedPoints: string;
      walletOwnerId?: string | null;
    }): Promise<SdkworkAccountAdminAdmission> {
      return client.wallet.billingPolicies.authorize({
        organizationId: request.scope.organizationId ?? undefined,
        ownerUserId: request.scope.ownerUserId ?? "",
        // The wire contract types this field as an int64 number; point amounts in
        // this console stay far below `Number.MAX_SAFE_INTEGER`.
        estimatedPoints: Number(request.estimatedPoints),
        walletOwnerType: request.scope.ownerType,
        walletOwnerId: request.walletOwnerId ?? undefined,
      });
    },

    /** Lists stored billing policy revisions for the operator surface. */
    async listPolicies(
      scope: SdkworkAccountAdminScope,
      page: SdkworkAccountAdminPageRequest & { scopeType?: number; status?: number } = {},
    ): Promise<{
      items: readonly SdkworkAccountAdminBillingPolicy[];
      pageInfo: SdkworkAccountAdminPageInfo;
    }> {
      return client.wallet.billingPolicies.list({
        organizationId: scope.organizationId ?? undefined,
        scopeType: page.scopeType,
        status: page.status,
        page: page.page,
        pageSize: page.pageSize,
      });
    },

    /**
     * Resolves the policy that governs one subject right now.
     *
     * A `null` row is meaningful: it means no stored policy applies and the
     * documented platform defaults govern the subject.
     */
    async resolvePolicy(scope: SdkworkAccountAdminScope): Promise<SdkworkAccountAdminBillingPolicy | null> {
      return client.wallet.billingPolicy.resolve({
        organizationId: scope.organizationId ?? undefined,
        ownerUserId: scope.ownerUserId ?? "",
        apiKeyId: undefined,
      });
    },

    /**
     * Stores the next revision of a policy.
     *
     * The account service retires the previous live revision in the same
     * transaction, so this call replaces the policy for its (scope, priority)
     * instead of stacking two live rows.
     */
    async upsertPolicy(input: SdkworkAccountAdminBillingPolicyInput): Promise<SdkworkAccountAdminBillingPolicy | null> {
      return client.wallet.billingPolicies.create({
        organizationId: input.organizationId ?? undefined,
        scopeType: input.scopeType,
        scopeId: input.scopeId ?? "",
        priority: input.priority,
        policyCode: input.policyCode,
        name: input.name,
        chargeMode: input.chargeMode,
        settlementMode: input.settlementMode,
        prechargeRatio: input.prechargeRatio,
        outputTokenCeiling: input.outputTokenCeiling,
        holdTtlSeconds: input.holdTtlSeconds,
        minHoldPoints: input.minHoldPoints,
        maxHoldShare: input.maxHoldShare,
        admissionMode: input.admissionMode,
        overdraftEnvelopePoints: input.overdraftEnvelopePoints,
        creditLimitPoints: input.creditLimitPoints,
        overagePolicy: input.overagePolicy,
        effectiveFrom: input.effectiveFrom ?? undefined,
        effectiveTo: input.effectiveTo ?? undefined,
      });
    },

    /** Enables a stored policy revision. */
    async activatePolicy(policyId: string): Promise<SdkworkAccountAdminBillingPolicy | null> {
      return client.wallet.billingPolicies.activate(policyId);
    },

    /** Retires a stored policy revision and frees its scope slot. */
    async deactivatePolicy(policyId: string): Promise<SdkworkAccountAdminBillingPolicy | null> {
      return client.wallet.billingPolicies.deactivate(policyId);
    },
  };
}

export type SdkworkAccountAdminService = ReturnType<typeof createSdkworkAccountAdminService>;
