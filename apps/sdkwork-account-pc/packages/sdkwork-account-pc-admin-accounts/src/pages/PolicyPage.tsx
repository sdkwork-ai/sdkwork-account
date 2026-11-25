import { useCallback, useEffect, useState } from "react";

import type {
  SdkworkAccountAdminBillingPolicy,
  SdkworkAccountAdminBillingPolicyInput,
  SdkworkAccountAdminScope,
  SdkworkAccountAdminService,
} from "../account-admin-service.ts";
import {
  AdminEmpty,
  AdminFailure,
  AdminLoading,
  AdminSection,
  AdminTable,
} from "../components/AdminPageChrome.tsx";

/** Copy the policy editor renders. */
export interface SdkworkPolicyPageCopy {
  title: string;
  subtitle: string;
  refresh: string;
  empty: string;
  failure: string;
  save: string;
  saving: string;
  activate: string;
  deactivate: string;
  columns: {
    code: string;
    scope: string;
    mode: string;
    ratio: string;
    ceiling: string;
    revision: string;
    status: string;
    actions: string;
  };
  form: {
    heading: string;
    code: string;
    name: string;
    chargeMode: string;
    settlementMode: string;
    prechargeRatio: string;
    outputTokenCeiling: string;
    holdTtlSeconds: string;
    maxHoldShare: string;
    admissionMode: string;
    overagePolicy: string;
    scopeType: string;
  };
}

export interface SdkworkPolicyPageProps {
  service: SdkworkAccountAdminService;
  scope: SdkworkAccountAdminScope;
  copy: SdkworkPolicyPageCopy;
  /** Initial form values; the host may prefill from the effective policy. */
  initialInput?: Partial<SdkworkAccountAdminBillingPolicyInput>;
}

const DEFAULT_INPUT: SdkworkAccountAdminBillingPolicyInput = {
  scopeType: 3,
  policyCode: "user-policy",
  name: "User policy",
  chargeMode: "prepaid_balance",
  settlementMode: "synchronous",
  prechargeRatio: "1.0000",
  outputTokenCeiling: 32768,
  holdTtlSeconds: 1800,
  maxHoldShare: "0.2500",
  admissionMode: "strict",
  overagePolicy: "allow_receivable",
};

/**
 * Billing policy editor.
 *
 * A policy is a funding rule, so the form states the two things an operator must
 * never confuse: where the money comes from (`chargeMode`) and how much a single
 * request may reserve (`prechargeRatio`, `outputTokenCeiling`, `maxHoldShare`).
 * Saving supersedes the previous revision instead of editing it in place, which is
 * what makes an audit trail and a rollback possible.
 */
export function SdkworkPolicyPage({ service, scope, copy, initialInput }: SdkworkPolicyPageProps) {
  const [policies, setPolicies] = useState<readonly SdkworkAccountAdminBillingPolicy[]>([]);
  const [input, setInput] = useState<SdkworkAccountAdminBillingPolicyInput>({
    ...DEFAULT_INPUT,
    ...initialInput,
  });
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const page = await service.listPolicies(scope, { pageSize: 50 });
      setPolicies(page.items);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, [service, scope]);

  useEffect(() => {
    void load();
  }, [load]);

  async function save(event: { preventDefault(): void }) {
    event.preventDefault();
    setSaving(true);
    setError(null);
    try {
      await service.upsertPolicy({ ...input, organizationId: scope.organizationId ?? null });
      await load();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  }

  async function toggle(policy: SdkworkAccountAdminBillingPolicy) {
    setError(null);
    try {
      if (policy.status === 1) {
        await service.deactivatePolicy(policy.id);
      } else {
        await service.activatePolicy(policy.id);
      }
      await load();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

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

      <form className="grid grid-cols-3 gap-3" onSubmit={(event) => void save(event)}>
        <Field label={copy.form.code}>
          <input
            value={input.policyCode}
            onChange={(event) => setInput({ ...input, policyCode: event.target.value })}
            required
          />
        </Field>
        <Field label={copy.form.name}>
          <input
            value={input.name}
            onChange={(event) => setInput({ ...input, name: event.target.value })}
            required
          />
        </Field>
        <Field label={copy.form.scopeType}>
          <select
            value={String(input.scopeType)}
            onChange={(event) => setInput({ ...input, scopeType: Number(event.target.value) })}
          >
            <option value="0">0 platform</option>
            <option value="1">1 tenant</option>
            <option value="2">2 organization</option>
            <option value="3">3 user</option>
            <option value="4">4 api key</option>
          </select>
        </Field>
        <Field label={copy.form.chargeMode}>
          <select
            value={input.chargeMode}
            onChange={(event) => setInput({ ...input, chargeMode: event.target.value })}
          >
            {["prepaid_balance", "credit_line", "postpaid_invoice", "plan_allowance", "free_trial", "byok"].map(
              (mode) => (
                <option key={mode} value={mode}>
                  {mode}
                </option>
              ),
            )}
          </select>
        </Field>
        <Field label={copy.form.settlementMode}>
          <select
            value={input.settlementMode}
            onChange={(event) => setInput({ ...input, settlementMode: event.target.value })}
          >
            {["synchronous", "asynchronous"].map((mode) => (
              <option key={mode} value={mode}>
                {mode}
              </option>
            ))}
          </select>
        </Field>
        <Field label={copy.form.admissionMode}>
          <select
            value={input.admissionMode}
            onChange={(event) => setInput({ ...input, admissionMode: event.target.value })}
          >
            {["strict", "strict_with_envelope", "bounded_overdraft"].map((mode) => (
              <option key={mode} value={mode}>
                {mode}
              </option>
            ))}
          </select>
        </Field>
        <Field label={copy.form.prechargeRatio}>
          <input
            value={input.prechargeRatio}
            onChange={(event) => setInput({ ...input, prechargeRatio: event.target.value })}
            required
          />
        </Field>
        <Field label={copy.form.maxHoldShare}>
          <input
            value={input.maxHoldShare}
            onChange={(event) => setInput({ ...input, maxHoldShare: event.target.value })}
            required
          />
        </Field>
        <Field label={copy.form.outputTokenCeiling}>
          <input
            inputMode="numeric"
            value={String(input.outputTokenCeiling)}
            onChange={(event) =>
              setInput({ ...input, outputTokenCeiling: Number(event.target.value || "0") })
            }
          />
        </Field>
        <Field label={copy.form.holdTtlSeconds}>
          <input
            inputMode="numeric"
            value={String(input.holdTtlSeconds)}
            onChange={(event) => setInput({ ...input, holdTtlSeconds: Number(event.target.value || "0") })}
          />
        </Field>
        <Field label={copy.form.overagePolicy}>
          <select
            value={input.overagePolicy}
            onChange={(event) => setInput({ ...input, overagePolicy: event.target.value })}
          >
            {["allow_receivable", "fail", "degrade"].map((mode) => (
              <option key={mode} value={mode}>
                {mode}
              </option>
            ))}
          </select>
        </Field>
        <button type="submit" disabled={saving}>
          {saving ? copy.saving : copy.save}
        </button>
      </form>

      {loading ? <AdminLoading>{copy.refresh}</AdminLoading> : null}
      {policies.length === 0 && !loading ? (
        <AdminEmpty>{copy.empty}</AdminEmpty>
      ) : (
        <AdminTable
          columns={[
            copy.columns.code,
            copy.columns.scope,
            copy.columns.mode,
            copy.columns.ratio,
            copy.columns.ceiling,
            copy.columns.revision,
            copy.columns.status,
            copy.columns.actions,
          ]}
        >
          {policies.map((policy) => (
            <tr key={policy.id}>
              <td>{policy.policyCode}</td>
              <td>
                {policy.scopeType} / {policy.scopeId}
              </td>
              <td>
                {policy.chargeMode} · {policy.settlementMode}
              </td>
              <td>{policy.prechargeRatio}</td>
              <td>{policy.outputTokenCeiling}</td>
              <td>{policy.revision}</td>
              <td>{policy.status === 1 ? "live" : "retired"}</td>
              <td>
                <button type="button" onClick={() => void toggle(policy)}>
                  {policy.status === 1 ? copy.deactivate : copy.activate}
                </button>
              </td>
            </tr>
          ))}
        </AdminTable>
      )}
    </AdminSection>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs opacity-70">{label}</span>
      {children}
    </label>
  );
}
