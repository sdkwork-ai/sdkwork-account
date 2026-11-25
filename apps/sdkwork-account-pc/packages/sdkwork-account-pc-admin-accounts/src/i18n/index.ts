/**
 * Package-local operator messages for the account centre.
 *
 * Layout follows I18N_SPEC §6.1: one fragment per capability slice, per locale,
 * under `i18n/<locale>/commerce/accounts/`. The host owns language selection;
 * this package owns the copy so a view never hardcodes an operator-facing string.
 */
import { accountsAdminOverview as overviewZh } from './zh-CN/commerce/accounts/overview.ts';
import { accountsAdminDirectory as directoryZh } from './zh-CN/commerce/accounts/directory.ts';
import { accountsAdminLedger as ledgerZh } from './zh-CN/commerce/accounts/ledger.ts';
import { accountsAdminAllowances as allowancesZh } from './zh-CN/commerce/accounts/allowances.ts';
import { accountsAdminPolicy as policyZh } from './zh-CN/commerce/accounts/policy.ts';
import { accountsAdminSimulate as simulateZh } from './zh-CN/commerce/accounts/simulate.ts';
import { accountsAdminRoutes as routesZh } from './zh-CN/commerce/accounts/routes.ts';
import { accountsAdminOverview as overviewEn } from './en-US/commerce/accounts/overview.ts';
import { accountsAdminDirectory as directoryEn } from './en-US/commerce/accounts/directory.ts';
import { accountsAdminLedger as ledgerEn } from './en-US/commerce/accounts/ledger.ts';
import { accountsAdminAllowances as allowancesEn } from './en-US/commerce/accounts/allowances.ts';
import { accountsAdminPolicy as policyEn } from './en-US/commerce/accounts/policy.ts';
import { accountsAdminSimulate as simulateEn } from './en-US/commerce/accounts/simulate.ts';
import { accountsAdminRoutes as routesEn } from './en-US/commerce/accounts/routes.ts';

export type SdkworkAccountsAdminLocale = "zh-CN" | "en-US";

/**
 * Widens literal copy so any locale satisfies the same bundle type.
 *
 * The fragments are `as const` (they are catalogs, and literal keys catch typos),
 * but a locale bundle must not be pinned to another locale's exact strings.
 */
type WidenStrings<T> = {
  [K in keyof T]: T[K] extends string ? string : WidenStrings<T[K]>;
};

export interface SdkworkAccountsAdminMessages {
  overview: WidenStrings<typeof overviewZh>;
  directory: WidenStrings<typeof directoryZh>;
  ledger: WidenStrings<typeof ledgerZh>;
  allowances: WidenStrings<typeof allowancesZh>;
  policy: WidenStrings<typeof policyZh>;
  simulate: WidenStrings<typeof simulateZh>;
  routes: WidenStrings<typeof routesZh>;
}

export const SDKWORK_ACCOUNTS_ADMIN_MESSAGES: Record<SdkworkAccountsAdminLocale, SdkworkAccountsAdminMessages> = {
  "zh-CN": { overview: overviewZh, directory: directoryZh, ledger: ledgerZh, allowances: allowancesZh, policy: policyZh, simulate: simulateZh, routes: routesZh },
  "en-US": { overview: overviewEn, directory: directoryEn, ledger: ledgerEn, allowances: allowancesEn, policy: policyEn, simulate: simulateEn, routes: routesEn },
};

/** Returns the message bundle for a locale, falling back to English. */
export function accountsAdminMessages(locale: SdkworkAccountsAdminLocale): SdkworkAccountsAdminMessages {
  return SDKWORK_ACCOUNTS_ADMIN_MESSAGES[locale] ?? SDKWORK_ACCOUNTS_ADMIN_MESSAGES["en-US"];
}
