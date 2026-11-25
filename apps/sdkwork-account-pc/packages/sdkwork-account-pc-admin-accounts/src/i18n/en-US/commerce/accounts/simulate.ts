/** en-US · commerce/accounts/simulate. */
export const accountsAdminSimulate = {
  "title": "Admission simulation",
  "subtitle": "Enter the estimated points of one request to see what the account side would decide and how much it would freeze. Nothing is held or charged.",
  "ownerLabel": "Owner id",
  "ownerTypeLabel": "Owner type",
  "walletOwnerIdLabel": "Wallet owner id (organization for team billing)",
  "estimateLabel": "Estimated points",
  "estimateHint": "The compute points this request would consume at settlement.",
  "submit": "Simulate",
  "submitting": "Simulating",
  "failure": "Simulation failed",
  "decisionLabel": "Decision",
  "holdLabel": "Points to freeze",
  "rejectCodeLabel": "Reject reason",
  "policyLabel": "Effective policy",
  "decisions": {
    "admit": "Admit",
    "deferred": "Admit (settle later)",
    "free": "Admit (not charged)",
    "reject": "Reject"
  }
} as const;
