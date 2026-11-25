/** zh-CN · commerce/accounts/policy. */
export const accountsAdminPolicy = {
  "title": "计费策略",
  "subtitle": "策略只决定资金从哪里来、单笔请求最多冻结多少；结算始终按实际用量扣费。",
  "refresh": "刷新",
  "empty": "暂无计费策略，当前由平台默认策略生效。",
  "failure": "保存失败",
  "save": "保存为新版本",
  "saving": "保存中",
  "activate": "启用",
  "deactivate": "停用",
  "columns": {
    "code": "策略代码",
    "scope": "作用域",
    "mode": "扣费模式",
    "ratio": "预扣倍率",
    "ceiling": "输出上限",
    "revision": "版本",
    "status": "状态",
    "actions": "操作"
  },
  "form": {
    "heading": "策略参数",
    "code": "策略代码",
    "name": "名称",
    "chargeMode": "扣费模式",
    "settlementMode": "结算方式",
    "prechargeRatio": "预扣倍率",
    "outputTokenCeiling": "输出 token 上限",
    "holdTtlSeconds": "冻结存活（秒）",
    "maxHoldShare": "最大冻结占比",
    "admissionMode": "准入模式",
    "overagePolicy": "超额策略",
    "scopeType": "作用域类型"
  }
} as const;
