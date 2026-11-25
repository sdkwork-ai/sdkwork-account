/** en-US · commerce/accounts/policy. */
export const accountsAdminPolicy = {
  "title": "Billing policy",
  "subtitle": "A policy decides where the money comes from and how much one request may freeze. Settlement always charges measured usage.",
  "refresh": "Refresh",
  "empty": "No stored policy; the platform defaults govern this subject.",
  "failure": "Save failed",
  "save": "Save as new revision",
  "saving": "Saving",
  "activate": "Activate",
  "deactivate": "Retire",
  "columns": {
    "code": "Code",
    "scope": "Scope",
    "mode": "Charge mode",
    "ratio": "Precharge ratio",
    "ceiling": "Output ceiling",
    "revision": "Revision",
    "status": "Status",
    "actions": "Actions"
  },
  "form": {
    "heading": "Policy parameters",
    "code": "Policy code",
    "name": "Name",
    "chargeMode": "Charge mode",
    "settlementMode": "Settlement mode",
    "prechargeRatio": "Precharge ratio",
    "outputTokenCeiling": "Output token ceiling",
    "holdTtlSeconds": "Hold lifetime (s)",
    "maxHoldShare": "Max hold share",
    "admissionMode": "Admission mode",
    "overagePolicy": "Overage policy",
    "scopeType": "Scope type"
  }
} as const;
