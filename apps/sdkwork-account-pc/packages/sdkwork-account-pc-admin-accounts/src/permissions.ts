/**
 * Permission codes for the account centre administration surface.
 *
 * Codes mirror the account service contract operation families
 * (`wallet.accounts.*`, `wallet.billing_policies.*`, `billing.admission.*`) so a
 * backend authorization change and this navigation hint never drift apart. They
 * are navigation and affordance hints only: the backend remains authoritative.
 */
export const SDKWORK_ACCOUNT_ADMIN_PERMISSIONS = {
  /** Read wallet accounts, ledger entries and allowance lots. */
  accountsRead: "wallet.accounts.read",
  /** Read stored billing policies and resolve the effective one. */
  policiesRead: "wallet.billing_policies.read",
  /** Create, activate or retire a billing policy. */
  policiesManage: "wallet.billing_policies.manage",
  /** Run the admission simulation. */
  admissionSimulate: "billing.admission.read",
} as const;

export type SdkworkAccountAdminPermission =
  (typeof SDKWORK_ACCOUNT_ADMIN_PERMISSIONS)[keyof typeof SDKWORK_ACCOUNT_ADMIN_PERMISSIONS];
