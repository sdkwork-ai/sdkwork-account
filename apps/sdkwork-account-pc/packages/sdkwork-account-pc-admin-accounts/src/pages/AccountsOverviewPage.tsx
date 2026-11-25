import { useCallback, useEffect, useState } from "react";

import type {
  SdkworkAccountAdminOverview,
  SdkworkAccountAdminScope,
  SdkworkAccountAdminService,
} from "../account-admin-service.ts";

/** Copy the overview renders; kept as props so the host owns localization. */
export interface SdkworkAccountsOverviewCopy {
  title: string;
  subtitle: string;
  refresh: string;
  availableLabel: string;
  frozenLabel: string;
  frozenShareLabel: string;
  accountsHeading: string;
  empty: string;
  failure: string;
  ownerColumn: string;
  assetColumn: string;
  availableColumn: string;
  frozenColumn: string;
  statusColumn: string;
}

export interface SdkworkAccountsOverviewPageProps {
  service: SdkworkAccountAdminService;
  scope: SdkworkAccountAdminScope;
  copy: SdkworkAccountsOverviewCopy;
  /** Renders a points amount in the operator's unit (points are exact integers). */
  formatPoints?: (points: string) => string;
}

function defaultFormatPoints(points: string): string {
  return points;
}

function formatShare(bps: number | null): string {
  if (bps === null) return "—";
  return `${(bps / 100).toFixed(2)}%`;
}

/**
 * Account centre overview.
 *
 * The page exists to answer one question an operator could not answer during the
 * production incident: how much of the wallet is frozen right now. Balance and
 * frozen amount are shown together with their ratio, because a wallet that looks
 * funded can still refuse every request when its frozen share is high.
 */
export function SdkworkAccountsOverviewPage({
  service,
  scope,
  copy,
  formatPoints = defaultFormatPoints,
}: SdkworkAccountsOverviewPageProps) {
  const [overview, setOverview] = useState<SdkworkAccountAdminOverview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setOverview(await service.loadOverview(scope));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, [service, scope]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <section className="flex flex-col gap-4">
      <header className="flex flex-col gap-1">
        <h1 className="text-lg font-semibold">{copy.title}</h1>
        <p className="text-sm opacity-70">{copy.subtitle}</p>
      </header>

      {error === null ? null : (
        <p role="alert" className="rounded border border-red-300 px-3 py-2 text-sm text-red-700">
          {copy.failure}: {error}
        </p>
      )}

      <div className="grid grid-cols-3 gap-3">
        <Summary label={copy.availableLabel} value={formatPoints(overview?.availablePoints ?? "0")} />
        <Summary label={copy.frozenLabel} value={formatPoints(overview?.frozenPoints ?? "0")} />
        <Summary label={copy.frozenShareLabel} value={formatShare(overview?.frozenShareBps ?? null)} />
      </div>

      <div className="flex items-center justify-between">
        <h2 className="text-sm font-medium">{copy.accountsHeading}</h2>
        <button type="button" onClick={() => void load()} disabled={loading}>
          {copy.refresh}
        </button>
      </div>

      {overview !== null && overview.accounts.length === 0 ? (
        <p className="text-sm opacity-70">{copy.empty}</p>
      ) : (
        <table className="w-full text-sm">
          <thead>
            <tr>
              <th scope="col">{copy.ownerColumn}</th>
              <th scope="col">{copy.assetColumn}</th>
              <th scope="col">{copy.availableColumn}</th>
              <th scope="col">{copy.frozenColumn}</th>
              <th scope="col">{copy.statusColumn}</th>
            </tr>
          </thead>
          <tbody>
            {(overview?.accounts ?? []).map((account) => (
              <tr key={account.id}>
                <td>
                  {account.ownerUserId}
                  {account.organizationId === null ? "" : ` / ${account.organizationId}`}
                </td>
                <td>{account.assetType}</td>
                <td>{formatPoints(account.availableAmount)}</td>
                <td>{formatPoints(account.frozenAmount)}</td>
                <td>{account.status}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function Summary({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded border px-3 py-2">
      <div className="text-xs opacity-70">{label}</div>
      <div className="text-base font-semibold">{value}</div>
    </div>
  );
}
