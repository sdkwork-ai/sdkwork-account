import { useCallback, useEffect, useState } from "react";

import type {
  SdkworkAccountAdminScope,
  SdkworkAccountAdminService,
  SdkworkAccountAdminWalletAccount,
} from "../account-admin-service.ts";
import { AdminEmpty, AdminFailure, AdminLoading, AdminSection, AdminTable } from "../components/AdminPageChrome.tsx";

/** Copy the directory page renders. */
export interface SdkworkAccountsDirectoryPageCopy {
  title: string;
  subtitle: string;
  refresh: string;
  empty: string;
  failure: string;
  filters: {
    owner: string;
    organization: string;
    ownerType: string;
    assetType: string;
    apply: string;
  };
  columns: {
    owner: string;
    assetType: string;
    currency: string;
    available: string;
    frozen: string;
    pending: string;
    status: string;
  };
}

export interface SdkworkAccountsDirectoryPageProps {
  service: SdkworkAccountAdminService;
  scope: SdkworkAccountAdminScope;
  copy: SdkworkAccountsDirectoryPageCopy;
  formatAmount?: (amount: string) => string;
}

const ASSET_TYPES = ["", "cash", "points", "token_bank"] as const;

/**
 * Account directory.
 *
 * The overview answers "how much is frozen"; this page answers "which wallet is
 * it": an operator narrows by owner, organization and asset before acting. Owner
 * and organization are plain text inputs because the backend accepts ids and the
 * service decides what a caller may see.
 */
export function SdkworkAccountsDirectoryPage({
  service,
  scope,
  copy,
  formatAmount = (amount) => amount,
}: SdkworkAccountsDirectoryPageProps) {
  const [ownerUserId, setOwnerUserId] = useState(scope.ownerUserId ?? "");
  const [organizationId, setOrganizationId] = useState(scope.organizationId ?? "");
  const [ownerType, setOwnerType] = useState<"" | "USER" | "ORGANIZATION">(scope.ownerType ?? "");
  const [assetType, setAssetType] = useState<"" | "cash" | "points" | "token_bank">(
    scope.assetType ?? "",
  );
  const [accounts, setAccounts] = useState<readonly SdkworkAccountAdminWalletAccount[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setAccounts(
        await service.listAccounts({
          ownerUserId: ownerUserId.trim() === "" ? undefined : ownerUserId.trim(),
          organizationId: organizationId.trim() === "" ? undefined : organizationId.trim(),
          ownerType: ownerType === "" ? undefined : ownerType,
          assetType: assetType === "" ? undefined : assetType,
        }),
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, [service, ownerUserId, organizationId, ownerType, assetType]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <AdminSection
      title={copy.title}
      subtitle={copy.subtitle}
      actions={
        <button type="button" onClick={() => void load()} disabled={loading}>
          {copy.refresh}
        </button>
      }
    >
      {error === null ? null : <AdminFailure label={copy.failure} message={error} />}

      <div className="grid grid-cols-4 items-end gap-3">
        <label className="flex flex-col gap-1">
          <span className="text-xs opacity-70">{copy.filters.owner}</span>
          <input value={ownerUserId} onChange={(event) => setOwnerUserId(event.target.value)} />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs opacity-70">{copy.filters.organization}</span>
          <input value={organizationId} onChange={(event) => setOrganizationId(event.target.value)} />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs opacity-70">{copy.filters.ownerType}</span>
          <select
            value={ownerType}
            onChange={(event) =>
              setOwnerType(event.target.value === "" ? "" : (event.target.value as "USER" | "ORGANIZATION"))
            }
          >
            <option value="">—</option>
            <option value="USER">USER</option>
            <option value="ORGANIZATION">ORGANIZATION</option>
          </select>
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs opacity-70">{copy.filters.assetType}</span>
          <select
            value={assetType}
            onChange={(event) =>
              setAssetType(event.target.value === "" ? "" : (event.target.value as typeof assetType))
            }
          >
            {ASSET_TYPES.map((value) => (
              <option key={value || "any"} value={value}>
                {value === "" ? "—" : value}
              </option>
            ))}
          </select>
        </label>
      </div>

      {loading ? <AdminLoading>{copy.refresh}</AdminLoading> : null}
      {accounts.length === 0 && !loading ? (
        <AdminEmpty>{copy.empty}</AdminEmpty>
      ) : (
        <AdminTable
          columns={[
            copy.columns.owner,
            copy.columns.assetType,
            copy.columns.currency,
            copy.columns.available,
            copy.columns.frozen,
            copy.columns.pending,
            copy.columns.status,
          ]}
        >
          {accounts.map((account) => (
            <tr key={account.id}>
              <td>
                {account.ownerUserId}
                {account.organizationId === null ? "" : ` / ${account.organizationId}`}
              </td>
              <td>{account.assetType}</td>
              <td>{account.currencyCode ?? "—"}</td>
              <td>{formatAmount(account.availableAmount)}</td>
              <td>{formatAmount(account.frozenAmount)}</td>
              <td>{formatAmount(account.pendingAmount)}</td>
              <td>{account.status}</td>
            </tr>
          ))}
        </AdminTable>
      )}
    </AdminSection>
  );
}
