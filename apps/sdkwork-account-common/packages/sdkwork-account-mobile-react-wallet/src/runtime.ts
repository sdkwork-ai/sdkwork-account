/**
 * Host-injected asset data plane for the wallet / points surface.
 *
 * The account domain owns the read models (cash balance, cash ledger, points
 * balance and lots) but not the commerce transport. A host injects one runtime
 * that talks to its own account service; before injection the screens fail
 * closed instead of rendering placeholder balances.
 */
export interface AccountWalletOverview {
  /** Cash balance in major units (yuan); `null` when the service has no value. */
  cashBalanceCny: number | null;
  pointsBalance: number | null;
}

export interface AccountWalletLedgerEntry {
  id: string;
  /** -credit / +debit: positive values are income. */
  amountCny: number | null;
  occurredAt?: string;
  summary: string;
}

export interface AccountWalletPointsLot {
  id: string;
  expiresAt?: string;
  points: number | null;
}

export interface AccountWalletPointsSummary {
  balance: number | null;
  lots: readonly AccountWalletPointsLot[];
}

export interface AccountWalletRuntime {
  loadCashLedger(): Promise<readonly AccountWalletLedgerEntry[]>;
  loadOverview(): Promise<AccountWalletOverview>;
  loadPoints(): Promise<AccountWalletPointsSummary>;
}

let runtime: AccountWalletRuntime | null = null;

/** Wires the host asset data plane for the wallet and points screens. */
export function configureAccountWalletRuntime(nextRuntime: AccountWalletRuntime | null): void {
  runtime = nextRuntime;
}

/** Returns the injected runtime, or `null` when the host did not compose one. */
export function getAccountWalletRuntime(): AccountWalletRuntime | null {
  return runtime;
}
