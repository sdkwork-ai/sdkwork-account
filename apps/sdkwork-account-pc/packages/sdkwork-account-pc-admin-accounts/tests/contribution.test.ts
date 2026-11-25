import assert from "node:assert/strict";
import { it } from "vitest";

import {
  ACCOUNTS_ADMIN_MANAGE_PERMISSION,
  ACCOUNTS_ADMIN_READ_PERMISSION,
  ACCOUNTS_ADMIN_ROUTE_RECORDS,
} from "../src/contribution.ts";
import { sdkworkAccountPcAdminAccountsRoutes } from "../src/routes.ts";

it("exposes one contribution record per declared route", () => {
  assert.equal(ACCOUNTS_ADMIN_ROUTE_RECORDS.length, sdkworkAccountPcAdminAccountsRoutes.length);
  assert.deepEqual(
    ACCOUNTS_ADMIN_ROUTE_RECORDS.map((record) => record.path),
    sdkworkAccountPcAdminAccountsRoutes.map((route) => route.path),
  );
});

it("only the policy editor demands the manage permission", () => {
  for (const route of sdkworkAccountPcAdminAccountsRoutes) {
    const record = ACCOUNTS_ADMIN_ROUTE_RECORDS.find((item) => item.path === route.path);
    assert.ok(record, `missing contribution for ${route.path}`);
    assert.equal(
      record.requiredPermission,
      route.screen === "policy" ? ACCOUNTS_ADMIN_MANAGE_PERMISSION : ACCOUNTS_ADMIN_READ_PERMISSION,
      route.path,
    );
  }
});

it("every record stays on the backend-admin surface and names its SDK family", () => {
  for (const record of ACCOUNTS_ADMIN_ROUTE_RECORDS) {
    assert.equal(record.adminPackage, "@sdkwork/account-pc-admin-accounts");
    assert.equal(record.backendSdkFamily, "sdkwork-account-backend-sdk");
    assert.ok(record.path.startsWith("/admin/accounts"), record.path);
  }
});
