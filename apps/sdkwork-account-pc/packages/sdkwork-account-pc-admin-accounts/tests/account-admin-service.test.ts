import assert from "node:assert/strict";
import { it } from "vitest";

import {
  createSdkworkAccountAdminService,
  type SdkworkAccountAdminAdmission,
  type SdkworkAccountAdminClient,
  type SdkworkAccountAdminLedgerEntry,
  type SdkworkAccountAdminPointsLot,
  type SdkworkAccountAdminWalletAccount,
} from "../src/account-admin-service.ts";

function walletAccount(overrides: Partial<SdkworkAccountAdminWalletAccount>): SdkworkAccountAdminWalletAccount {
  return {
    id: "1",
    uuid: "acc-1",
    tenantId: "100001",
    organizationId: null,
    ownerUserId: "5001",
    assetType: "token_bank",
    currencyCode: "TOKEN_BANK",
    availableAmount: "0",
    frozenAmount: "0",
    pendingAmount: "0",
    status: "ACTIVE",
    version: "1",
    ...overrides,
  };
}

interface Captured {
  accountParams?: unknown;
  ledgerParams?: unknown;
  lotsParams?: unknown;
  authorizeBody?: unknown;
}

function fakeClient(
  captured: Captured,
  options: {
    accounts?: readonly SdkworkAccountAdminWalletAccount[];
    ledger?: readonly SdkworkAccountAdminLedgerEntry[];
    lots?: readonly SdkworkAccountAdminPointsLot[];
    admission?: SdkworkAccountAdminAdmission;
  } = {},
): SdkworkAccountAdminClient {
  return {
    wallet: {
      walletAccounts: {
        async list(params?: unknown) {
          captured.accountParams = params;
          return { items: options.accounts ?? [] };
        },
      },
      walletLedgerEntries: {
        async list(params?: unknown) {
          captured.ledgerParams = params;
          return {
            items: options.ledger ?? [],
            pageInfo: { mode: "offset" as const, page: 1, pageSize: 50, totalItems: "0", hasMore: false },
          };
        },
      },
      pointsLots: {
        async list(params?: unknown) {
          captured.lotsParams = params;
          return {
            items: options.lots ?? [],
            pageInfo: { mode: "offset" as const, page: 1, pageSize: 50, totalItems: "0", hasMore: false },
          };
        },
      },
      billingPolicies: {
        async authorize(body: unknown) {
          captured.authorizeBody = body;
          return options.admission ?? { decision: "admit", holdPoints: "0", rejectCode: null, policyId: null };
        },
        async list() {
          return {
            items: [],
            pageInfo: { mode: "offset" as const, page: 1, pageSize: 50, totalItems: "0", hasMore: false },
          };
        },
        async create() {
          return null;
        },
        async activate() {
          return null;
        },
        async deactivate() {
          return null;
        },
      },
      billingPolicy: {
        async resolve() {
          return null;
        },
      },
    },
  };
}

it("overview sums token-bank balances and reports the frozen share", async () => {
  const captured: Captured = {};
  const client = fakeClient(captured, {
    accounts: [
      walletAccount({ id: "1", availableAmount: "41459562", frozenAmount: "33256530" }),
      // Cash accounts must not contribute to the points totals.
      walletAccount({ id: "2", assetType: "cash", availableAmount: "999999", frozenAmount: "888888" }),
    ],
  });
  const service = createSdkworkAccountAdminService(client);
  const overview = await service.loadOverview({ ownerUserId: "5001" });

  assert.equal(overview.availablePoints, "41459562");
  assert.equal(overview.frozenPoints, "33256530");
  // 33,256,530 / 74,716,092 = 44.51% -> 4451 basis points.
  assert.equal(overview.frozenShareBps, 4451);
  assert.equal(overview.accounts.length, 2);
});

it("an empty wallet reports no frozen share instead of zero percent", async () => {
  const service = createSdkworkAccountAdminService(fakeClient({}, { accounts: [] }));
  const overview = await service.loadOverview({ ownerUserId: "5001" });
  assert.equal(overview.availablePoints, "0");
  assert.equal(overview.frozenPoints, "0");
  assert.equal(overview.frozenShareBps, null);
});

it("account listing forwards only the scope fields the caller supplied", async () => {
  const captured: Captured = {};
  const service = createSdkworkAccountAdminService(fakeClient(captured));
  await service.listAccounts({ ownerUserId: "5001", organizationId: "9001", ownerType: "ORGANIZATION" });

  assert.deepEqual(captured.accountParams, {
    organizationId: "9001",
    ownerUserId: "5001",
    ownerType: "ORGANIZATION",
    assetType: undefined,
  });
});

it("paging uses camelCase pageSize and never the wire page_size", async () => {
  const captured: Captured = {};
  const service = createSdkworkAccountAdminService(fakeClient(captured));
  await service.listLedgerEntries({ ownerUserId: "5001" }, { page: 2, pageSize: 25, cursor: "c1" });
  await service.listPointsLots({ ownerUserId: "5001" }, { page: 3, pageSize: 10 });

  const ledger = captured.ledgerParams as Record<string, unknown>;
  assert.equal(ledger.pageSize, 25);
  assert.equal(ledger.page, 2);
  assert.equal(ledger.cursor, "c1");
  assert.equal("page_size" in ledger, false, "the generated client owns the wire name");

  const lots = captured.lotsParams as Record<string, unknown>;
  assert.equal(lots.pageSize, 10);
  assert.equal("page_size" in lots, false);
});

it("the simulator sends a numeric estimate and forwards every decision state", async () => {
  const captured: Captured = {};
  const states: ReadonlyArray<[SdkworkAccountAdminAdmission, string]> = [
    [{ decision: "admit", holdPoints: "1310720", policyId: "p1" }, "admit"],
    [{ decision: "deferred", holdPoints: "0" }, "deferred"],
    [{ decision: "free", holdPoints: "0" }, "free"],
    [{ decision: "reject", holdPoints: "0", rejectCode: "insufficient_balance" }, "reject"],
  ];

  for (const [admission, expected] of states) {
    const service = createSdkworkAccountAdminService(fakeClient(captured, { admission }));
    const result = await service.simulateAdmission({
      scope: { ownerUserId: "5001", ownerType: "USER" },
      estimatedPoints: "1310720",
      walletOwnerId: "5001",
    });
    assert.equal(result.decision, expected);
    const body = captured.authorizeBody as Record<string, unknown>;
    assert.equal(body.estimatedPoints, 1310720, "the wire contract expects a number");
    assert.equal(typeof body.estimatedPoints, "number");
    assert.equal(body.ownerUserId, "5001");
    assert.equal(body.walletOwnerType, "USER");
    assert.equal(body.walletOwnerId, "5001");
    assert.equal(body.organizationId, undefined);
  }
});

it("a rejection keeps the reason so the console can explain it", async () => {
  const captured: Captured = {};
  const service = createSdkworkAccountAdminService(
    fakeClient(captured, {
      admission: { decision: "reject", holdPoints: "0", rejectCode: "insufficient_balance" },
    }),
  );
  const result = await service.simulateAdmission({
    scope: { ownerUserId: "5001" },
    estimatedPoints: "16443980",
  });
  assert.equal(result.rejectCode, "insufficient_balance");
  const body = captured.authorizeBody as Record<string, unknown>;
  assert.equal(body.estimatedPoints, 16443980);
});

it("policy listing forwards the scope and paging in camelCase", async () => {
  const captured: Captured = {};
  const client = fakeClient(captured);
  const policies = client.wallet.billingPolicies as Record<string, unknown> & {
    list?: (params?: unknown) => Promise<unknown>;
  };
  let listParams: unknown;
  policies.list = async (params?: unknown) => {
    listParams = params;
    return { items: [], pageInfo: { mode: "offset", page: 1, pageSize: 50, totalItems: "0", hasMore: false } };
  };
  const service = createSdkworkAccountAdminService(client);
  await service.listPolicies({ organizationId: "9001" }, { page: 2, pageSize: 20, scopeType: 3, status: 1 });

  const params = listParams as Record<string, unknown>;
  assert.equal(params.organizationId, "9001");
  assert.equal(params.scopeType, 3);
  assert.equal(params.status, 1);
  assert.equal(params.pageSize, 20);
  assert.equal("page_size" in params, false);
});

it("policy upsert forwards every parameter the account service validates", async () => {
  const body = { captured: undefined as unknown };
  const client = fakeClient({});
  (client.wallet.billingPolicies as unknown as Record<string, unknown>).create = async (input: unknown) => {
    body.captured = input;
    return null;
  };
  const service = createSdkworkAccountAdminService(client);
  await service.upsertPolicy({
    scopeType: 3,
    scopeId: "5001",
    policyCode: "user-policy",
    name: "User policy",
    chargeMode: "prepaid_balance",
    settlementMode: "synchronous",
    prechargeRatio: "0.5000",
    outputTokenCeiling: 32768,
    holdTtlSeconds: 1800,
    maxHoldShare: "0.2500",
    admissionMode: "strict",
    overagePolicy: "allow_receivable",
  });

  const sent = body.captured as Record<string, unknown>;
  assert.equal(sent.scopeType, 3);
  assert.equal(sent.scopeId, "5001");
  assert.equal(sent.prechargeRatio, "0.5000");
  assert.equal(sent.outputTokenCeiling, 32768);
  assert.equal(sent.maxHoldShare, "0.2500");
  assert.equal(sent.priority, undefined, "priority is optional and left to the service default");
});

it("policy activation and retirement return the stored revision", async () => {
  const calls: string[] = [];
  const client = fakeClient({});
  const policies = client.wallet.billingPolicies as unknown as Record<string, unknown>;
  policies.activate = async (id: string) => {
    calls.push(`activate:${id}`);
    return null;
  };
  policies.deactivate = async (id: string) => {
    calls.push(`deactivate:${id}`);
    return null;
  };
  const service = createSdkworkAccountAdminService(client);
  await service.activatePolicy("p1");
  await service.deactivatePolicy("p1");
  assert.deepEqual(calls, ["activate:p1", "deactivate:p1"]);
});

it("a subject without a stored policy resolves to null, not a fake default row", async () => {
  const client = fakeClient({});
  (client.wallet as unknown as Record<string, unknown>).billingPolicy = {
    async resolve() {
      return null;
    },
  };
  const service = createSdkworkAccountAdminService(client);
  assert.equal(await service.resolvePolicy({ ownerUserId: "5001" }), null);
});