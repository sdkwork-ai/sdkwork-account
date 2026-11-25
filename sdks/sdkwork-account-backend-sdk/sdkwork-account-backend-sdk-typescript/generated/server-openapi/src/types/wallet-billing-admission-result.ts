import type { WalletBillingAdmission } from './wallet-billing-admission';

/** Billing admission simulation result schema exposed by the account service. */
export interface WalletBillingAdmissionResult {
  code: 0;
  data: { item: WalletBillingAdmission; };
  traceId: string;
}
