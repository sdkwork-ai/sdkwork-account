import type { SdkworkAccountPcRouteContribution } from "@sdkwork/account-pc-core";

/**
 * Account centre administration routes.
 *
 * These are `backend-admin` destinations: they read and write another subject's
 * wallet through the backend API surface, so they never share a path prefix with
 * the customer-facing console wallet under `/app/wallet`.
 */
export const sdkworkAccountPcAdminAccountsRoutes = [
  {
    auth: "required",
    capability: "accounts",
    domain: "commerce",
    id: "admin.commerce.accounts.overview",
    packageName: "@sdkwork/account-pc-admin-accounts",
    path: "/admin/accounts/overview",
    screen: "overview",
    surface: "backend-admin",
    title: "Account overview",
    titleKey: "accountsAdmin.routes.overview.title",
  },
  {
    auth: "required",
    capability: "accounts",
    domain: "commerce",
    id: "admin.commerce.accounts.directory",
    packageName: "@sdkwork/account-pc-admin-accounts",
    path: "/admin/accounts",
    screen: "directory",
    surface: "backend-admin",
    title: "Accounts",
    titleKey: "accountsAdmin.routes.directory.title",
  },
  {
    auth: "required",
    capability: "accounts",
    domain: "commerce",
    id: "admin.commerce.accounts.ledger",
    packageName: "@sdkwork/account-pc-admin-accounts",
    path: "/admin/accounts/ledger",
    screen: "ledger",
    surface: "backend-admin",
    title: "Ledger",
    titleKey: "accountsAdmin.routes.ledger.title",
  },
  {
    auth: "required",
    capability: "accounts",
    domain: "commerce",
    id: "admin.commerce.accounts.allowances",
    packageName: "@sdkwork/account-pc-admin-accounts",
    path: "/admin/accounts/allowances",
    screen: "allowances",
    surface: "backend-admin",
    title: "Allowance lots",
    titleKey: "accountsAdmin.routes.allowances.title",
  },
  {
    auth: "required",
    capability: "accounts",
    domain: "commerce",
    id: "admin.commerce.accounts.policy",
    packageName: "@sdkwork/account-pc-admin-accounts",
    path: "/admin/accounts/policy",
    screen: "policy",
    surface: "backend-admin",
    title: "Billing policy",
    titleKey: "accountsAdmin.routes.policy.title",
  },
  {
    auth: "required",
    capability: "accounts",
    domain: "commerce",
    id: "admin.commerce.accounts.simulate",
    packageName: "@sdkwork/account-pc-admin-accounts",
    path: "/admin/accounts/simulate",
    screen: "simulate",
    surface: "backend-admin",
    title: "Admission simulation",
    titleKey: "accountsAdmin.routes.simulate.title",
  },
] as const satisfies readonly SdkworkAccountPcRouteContribution[];
