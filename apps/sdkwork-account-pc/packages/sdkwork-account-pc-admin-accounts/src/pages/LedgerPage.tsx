import { useCallback, useEffect, useState } from "react";

import type {
  SdkworkAccountAdminLedgerEntry,
  SdkworkAccountAdminPageInfo,
  SdkworkAccountAdminScope,
  SdkworkAccountAdminService,
} from "../account-admin-service.ts";
import { AdminEmpty, AdminFailure, AdminLoading, AdminSection, AdminTable } from "../components/AdminPageChrome.tsx";

/** Copy the ledger page renders. */
export interface SdkworkLedgerPageCopy {
  title: string;
  subtitle: string;
  refresh: string;
  empty: string;
  failure: string;
  page: string;
  columns: {
    createdAt: string;
    direction: string;
    amount: string;
    balanceAfter: string;
    businessType: string;
    requestNo: string;
    assetType: string;
  };
}

export interface SdkworkLedgerPageProps {
  service: SdkworkAccountAdminService;
  scope: SdkworkAccountAdminScope;
  copy: SdkworkLedgerPageCopy;
  pageSize?: number;
  formatAmount?: (amount: string) => string;
}

/**
 * Ledger view.
 *
 * Every wallet movement is one row, so an operator can reconcile a support report
 * against real debits and credits instead of trusting a balance snapshot. The page
 * is paging-aware because a busy wallet produces far more rows than one screen.
 */
export function SdkworkLedgerPage({
  service,
  scope,
  copy,
  pageSize = 50,
  formatAmount = (amount) => amount,
}: SdkworkLedgerPageProps) {
  const [entries, setEntries] = useState<readonly SdkworkAccountAdminLedgerEntry[]>([]);
  const [pageInfo, setPageInfo] = useState<SdkworkAccountAdminPageInfo | null>(null);
  const [page, setPage] = useState(1);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(
    async (target: number) => {
      setLoading(true);
      setError(null);
      try {
        const result = await service.listLedgerEntries(scope, { page: target, pageSize });
        setEntries(result.items);
        setPageInfo(result.pageInfo);
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        setLoading(false);
      }
    },
    [service, scope, pageSize],
  );

  useEffect(() => {
    void load(page);
  }, [load, page]);

  return (
    <AdminSection
      title={copy.title}
      subtitle={copy.subtitle}
      actions={
        <button type="button" onClick={() => void load(page)} disabled={loading}>
          {copy.refresh}
        </button>
      }
    >
      {error === null ? null : <AdminFailure label={copy.failure} message={error} />}
      {loading ? <AdminLoading>{copy.refresh}</AdminLoading> : null}

      {entries.length === 0 && !loading ? (
        <AdminEmpty>{copy.empty}</AdminEmpty>
      ) : (
        <AdminTable
          columns={[
            copy.columns.createdAt,
            copy.columns.direction,
            copy.columns.assetType,
            copy.columns.amount,
            copy.columns.balanceAfter,
            copy.columns.businessType,
            copy.columns.requestNo,
          ]}
        >
          {entries.map((entry) => (
            <tr key={entry.id}>
              <td>{entry.createdAt}</td>
              <td>{entry.direction}</td>
              <td>{entry.assetType}</td>
              <td>{formatAmount(entry.amount)}</td>
              <td>{formatAmount(entry.balanceAfter)}</td>
              <td>{entry.businessType}</td>
              <td>{entry.requestNo}</td>
            </tr>
          ))}
        </AdminTable>
      )}

      <footer className="flex items-center gap-3 text-sm">
        <button type="button" disabled={page <= 1 || loading} onClick={() => setPage(page - 1)}>
          ‹
        </button>
        <span>
          {copy.page} {page}
          {pageInfo?.totalItems === undefined ? "" : ` / ${pageInfo.totalItems}`}
        </span>
        <button type="button" disabled={pageInfo?.hasMore !== true || loading} onClick={() => setPage(page + 1)}>
          ›
        </button>
      </footer>
    </AdminSection>
  );
}
