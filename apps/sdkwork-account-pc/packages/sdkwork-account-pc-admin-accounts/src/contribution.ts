/**
 * Route contribution for an admin host.
 *
 * Mirrors the host pattern used by the Cloud Router console
 * (`src/admin/*HostMount.tsx` consumes each admin package's route records): the
 * host owns navigation, layout and permission checks, while this package declares
 * what it can render.
 *
 * The account application in this repository is the end-user wallet surface and
 * therefore does not mount these records; the platform admin host does.
 */

import { sdkworkAccountPcAdminAccountsRoutes } from "./routes.ts";

/** Permission a host checks before rendering an account-centre route. */
export const ACCOUNTS_ADMIN_READ_PERMISSION = "wallet.accounts.read";
/** Permission a host checks before rendering the policy editor. */
export const ACCOUNTS_ADMIN_MANAGE_PERMISSION = "wallet.billing_policies.manage";

/** One route the admin host may mount for the account centre. */
export interface SdkworkAccountsAdminRouteRecord {
  /** Absolute admin path, e.g. `/admin/accounts/overview`. */
  path: string;
  /** Machine key into this package's message bundle (`routes.<screen>.title`). */
  titleKey: string;
  /** Permission the host must verify before rendering the route. */
  requiredPermission: string;
  /** Admin package that provides the view. */
  adminPackage: "@sdkwork/account-pc-admin-accounts";
  /** Backend SDK family the view reads. */
  backendSdkFamily: "sdkwork-account-backend-sdk";
}

/**
 * Records a host can mount directly.
 *
 * The policy editor is the only screen that edits state; it requires the manage
 * permission, while every other screen is read-only.
 */
export const ACCOUNTS_ADMIN_ROUTE_RECORDS: readonly SdkworkAccountsAdminRouteRecord[] =
  sdkworkAccountPcAdminAccountsRoutes.map((route) => ({
    path: route.path,
    titleKey: route.titleKey,
    requiredPermission:
      route.screen === "policy" ? ACCOUNTS_ADMIN_MANAGE_PERMISSION : ACCOUNTS_ADMIN_READ_PERMISSION,
    adminPackage: "@sdkwork/account-pc-admin-accounts",
    backendSdkFamily: "sdkwork-account-backend-sdk",
  }));

export type SdkworkAccountsAdminRouteId = (typeof sdkworkAccountPcAdminAccountsRoutes)[number]["id"];
