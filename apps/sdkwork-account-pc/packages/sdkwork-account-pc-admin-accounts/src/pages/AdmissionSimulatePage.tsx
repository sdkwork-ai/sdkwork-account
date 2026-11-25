import { useState } from "react";

import type {
  SdkworkAccountAdminAdmission,
  SdkworkAccountAdminScope,
  SdkworkAccountAdminService,
} from "../account-admin-service.ts";

/** Copy the simulator renders; kept as props so the host owns localization. */
export interface SdkworkAdmissionSimulateCopy {
  title: string;
  subtitle: string;
  ownerLabel: string;
  ownerTypeLabel: string;
  walletOwnerIdLabel: string;
  estimateLabel: string;
  estimateHint: string;
  submit: string;
  submitting: string;
  failure: string;
  decisionLabel: string;
  holdLabel: string;
  rejectCodeLabel: string;
  policyLabel: string;
  decisions: Record<"admit" | "deferred" | "free" | "reject", string>;
}

export interface SdkworkAdmissionSimulatePageProps {
  service: SdkworkAccountAdminService;
  defaultScope: SdkworkAccountAdminScope;
  copy: SdkworkAdmissionSimulateCopy;
  /** Renders a points amount in the operator's unit. */
  formatPoints?: (points: string) => string;
}

function defaultFormatPoints(points: string): string {
  return points;
}

/**
 * Admission simulator.
 *
 * An operator pastes the estimated charge of a failing request and reads what the
 * account side would decide: the amount it would freeze and, when it refuses, the
 * reason. Nothing is charged and no hold is created, so it is safe to run against
 * a live subject — this is the "why did a funded wallet get a 402" answer.
 */
export function SdkworkAdmissionSimulatePage({
  service,
  defaultScope,
  copy,
  formatPoints = defaultFormatPoints,
}: SdkworkAdmissionSimulatePageProps) {
  const [ownerUserId, setOwnerUserId] = useState(defaultScope.ownerUserId ?? "");
  const [ownerType, setOwnerType] = useState<"USER" | "ORGANIZATION">(
    defaultScope.ownerType ?? "USER",
  );
  const [walletOwnerId, setWalletOwnerId] = useState("");
  const [estimatedPoints, setEstimatedPoints] = useState("");
  const [result, setResult] = useState<SdkworkAccountAdminAdmission | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function submit(event: { preventDefault(): void }) {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      setResult(
        await service.simulateAdmission({
          scope: { ownerUserId, ownerType, organizationId: defaultScope.organizationId },
          estimatedPoints,
          walletOwnerId: walletOwnerId.trim() === "" ? null : walletOwnerId.trim(),
        }),
      );
    } catch (cause) {
      setResult(null);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <section className="flex flex-col gap-4">
      <header className="flex flex-col gap-1">
        <h1 className="text-lg font-semibold">{copy.title}</h1>
        <p className="text-sm opacity-70">{copy.subtitle}</p>
      </header>

      <form className="flex flex-col gap-3" onSubmit={(event) => void submit(event)}>
        <label className="flex flex-col gap-1">
          <span className="text-xs opacity-70">{copy.ownerLabel}</span>
          <input value={ownerUserId} onChange={(event) => setOwnerUserId(event.target.value)} required />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs opacity-70">{copy.ownerTypeLabel}</span>
          <select
            value={ownerType}
            onChange={(event) => setOwnerType(event.target.value === "ORGANIZATION" ? "ORGANIZATION" : "USER")}
          >
            <option value="USER">USER</option>
            <option value="ORGANIZATION">ORGANIZATION</option>
          </select>
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs opacity-70">{copy.walletOwnerIdLabel}</span>
          <input value={walletOwnerId} onChange={(event) => setWalletOwnerId(event.target.value)} />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs opacity-70">{copy.estimateLabel}</span>
          <input
            value={estimatedPoints}
            onChange={(event) => setEstimatedPoints(event.target.value)}
            inputMode="numeric"
            required
          />
          <span className="text-xs opacity-60">{copy.estimateHint}</span>
        </label>
        <button type="submit" disabled={submitting}>
          {submitting ? copy.submitting : copy.submit}
        </button>
      </form>

      {error === null ? null : (
        <p role="alert" className="rounded border border-red-300 px-3 py-2 text-sm text-red-700">
          {copy.failure}: {error}
        </p>
      )}

      {result === null ? null : (
        <dl className="grid grid-cols-2 gap-2 text-sm" data-decision={result.decision}>
          <Row label={copy.decisionLabel} value={copy.decisions[result.decision]} />
          <Row label={copy.holdLabel} value={formatPoints(result.holdPoints)} />
          {result.rejectCode === null || result.rejectCode === undefined ? null : (
            <Row label={copy.rejectCodeLabel} value={result.rejectCode} />
          )}
          {result.policyId === null || result.policyId === undefined ? null : (
            <Row label={copy.policyLabel} value={result.policyId} />
          )}
        </dl>
      )}
    </section>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col">
      <dt className="text-xs opacity-70">{label}</dt>
      <dd className="text-sm font-medium">{value}</dd>
    </div>
  );
}
