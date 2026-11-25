import { useCallback, useEffect, useState } from "react";

import type {
  SdkworkAccountAdminPageInfo,
  SdkworkAccountAdminPointsLot,
  SdkworkAccountAdminScope,
  SdkworkAccountAdminService,
} from "../account-admin-service.ts";
import { AdminEmpty, AdminFailure, AdminLoading, AdminSection, AdminTable } from "../components/AdminPageChrome.tsx";

/** Copy the allowance page renders. */
export interface SdkworkAllowancesPageCopy {
  title: string;
  subtitle: string;
  refresh: string;
  empty: string;
  failure: string;
  page: string;
  columns: {
    source: string;
    granted: string;
    remaining: string;
    expiresAt: string;
    status: string;
    createdAt: string;
  };
}

export interface SdkworkAllowancesPageProps {
  service: SdkworkAccountAdminService;
  scope: SdkworkAccountAdminScope;
  copy: SdkworkAllowancesPageCopy;
  pageSize?: number;
}

/**
 * Allowance lots view.
 *
 * Points arrive in expiring lots, so the remaining amount and the expiry of each
 * lot matter more than the account total. The page is read-only on purpose: the
 * funding path for `plan_allowance` and `free_trial` is fail-closed until lots are
 * wired as an admission source, and an operator must be able to see which lots
 * would fund a request before that switch is flipped.
 */
export function SdkworkAllowancesPage({ service, scope, copy, pageSize = 50 }: SdkworkAllowancesPageProps) {
  const [lots, setLots] = useState<readonly SdkworkAccountAdminPointsLot[]>([]);
  const [pageInfo, setPageInfo] = useState<SdkworkAccountAdminPageInfo | null>(null);
  const [page, setPage] = useState(1);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(
    async (target: number) => {
      setLoading(true);
      setError(null);
      try {
        const result = await service.listPointsLots(scope, { page: target, pageSize });
        setLots(result.items);
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

      {lots.length === 0 && !loading ? (
        <AdminEmpty>{copy.empty}</AdminEmpty>
      ) : (
        <AdminTable
          columns={[
            copy.columns.source,
            copy.columns.granted,
            copy.columns.remaining,
            copy.columns.expiresAt,
            copy.columns.status,
            copy.columns.createdAt,
          ]}
        >
          {lots.map((lot) => (
            <tr key={lot.id}>
              <td>
                {lot.sourceType} / {lot.sourceId}
              </td>
              <td>{lot.grantedAmount}</td>
              <td>{lot.remainingAmount}</td>
              <td>{lot.expiresAt ?? "—"}</td>
              <td>{lot.status}</td>
              <td>{lot.createdAt}</td>
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
