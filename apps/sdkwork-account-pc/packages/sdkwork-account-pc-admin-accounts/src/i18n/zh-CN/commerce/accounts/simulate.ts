/** zh-CN · commerce/accounts/simulate. */
export const accountsAdminSimulate = {
  "title": "准入试算",
  "subtitle": "输入一次请求的估算点数，查看账户侧会放行还是拒绝、会冻结多少——不会产生任何冻结或扣费。",
  "ownerLabel": "主体 ID",
  "ownerTypeLabel": "主体类型",
  "walletOwnerIdLabel": "钱包归属 ID（团队计费时填组织）",
  "estimateLabel": "估算点数",
  "estimateHint": "取该请求在结算时可能产生的算力额度点数。",
  "submit": "试算",
  "submitting": "试算中",
  "failure": "试算失败",
  "decisionLabel": "裁决",
  "holdLabel": "将冻结点数",
  "rejectCodeLabel": "拒绝原因",
  "policyLabel": "生效策略",
  "decisions": {
    "admit": "放行",
    "deferred": "放行（延后结算）",
    "free": "放行（不收费）",
    "reject": "拒绝"
  }
} as const;
