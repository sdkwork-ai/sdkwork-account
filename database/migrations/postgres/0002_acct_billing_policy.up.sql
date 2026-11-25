-- sdkwork:migration
-- id: 0002_acct_billing_policy
-- engine: postgres
-- module: sdkwork-account
-- purpose: Introduce the account billing policy aggregate. A policy row states,
--   for one subject scope, where a charge is funded from (charge_mode), when it
--   is deducted (settlement_mode), and how much prepaid funding a single request
--   may reserve (admission parameters). The row is the single source of truth
--   for those rules: pricing owns unit prices and never states funding rules,
--   and the routing runtime consumes this row instead of holding its own copy.
--   An empty table means "platform default": the service resolves the documented
--   defaults, so existing accounts keep their current behaviour byte for byte.
-- reversible: false
-- rollback: forward-fix (the table holds live funding rules; a destructive
--   down migration is refused. Supersede rows by writing the next revision,
--   and retire a scope with status = 0 or a soft delete.)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

CREATE TABLE IF NOT EXISTS acct_billing_policy (
    id BIGINT PRIMARY KEY,
    uuid VARCHAR(64) NOT NULL,
    tenant_id BIGINT NOT NULL DEFAULT 0,
    organization_id BIGINT NOT NULL DEFAULT 0,
    data_scope SMALLINT NOT NULL DEFAULT 0,
    status SMALLINT NOT NULL DEFAULT 1,
    version BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMPTZ,
    deleted_by BIGINT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    policy_code VARCHAR(64) NOT NULL,
    name VARCHAR(128) NOT NULL,
    -- Resolution scope. 0 is the platform default row, 1 tenant, 2 organization,
    -- 3 user, 4 api key. `scope_id` is the owner id of that scope (0 for the
    -- platform default). The resolver picks the most specific matching scope,
    -- then the highest `priority`, then the newest `revision`.
    scope_type SMALLINT NOT NULL DEFAULT 0,
    scope_id BIGINT NOT NULL DEFAULT 0,
    priority INTEGER NOT NULL DEFAULT 100,
    revision BIGINT NOT NULL DEFAULT 1,
    -- Where the charge is funded from.
    charge_mode VARCHAR(32) NOT NULL DEFAULT 'prepaid_balance',
    -- When the charge is deducted.
    settlement_mode VARCHAR(16) NOT NULL DEFAULT 'synchronous',
    -- Admission (how much is reserved before dispatch). These never change the
    -- settled amount: settlement always charges measured usage.
    precharge_ratio NUMERIC(10,4) NOT NULL DEFAULT 1.0000,
    output_token_ceiling BIGINT NOT NULL DEFAULT 32768,
    input_token_source VARCHAR(16) NOT NULL DEFAULT 'bytes_div4',
    hold_ttl_seconds BIGINT NOT NULL DEFAULT 1800,
    min_hold_points BIGINT NOT NULL DEFAULT 0,
    max_hold_share NUMERIC(5,4) NOT NULL DEFAULT 0.2500,
    admission_mode VARCHAR(24) NOT NULL DEFAULT 'strict',
    overdraft_envelope_points BIGINT NOT NULL DEFAULT 0,
    -- Credit and allowance funding.
    credit_limit_points BIGINT NOT NULL DEFAULT 0,
    allowance_period VARCHAR(16),
    allowance_quota BIGINT,
    allowance_reset_mode VARCHAR(16),
    -- What happens when measured usage exceeds the frozen reservation.
    overage_policy VARCHAR(24) NOT NULL DEFAULT 'allow_receivable',
    effective_from TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    effective_to TIMESTAMPTZ,
    CONSTRAINT ck_acct_billing_policy_tenant_scope
        CHECK (tenant_id >= 0 AND organization_id >= 0 AND (tenant_id > 0 OR organization_id = 0)),
    CONSTRAINT ck_acct_billing_policy_charge_mode
        CHECK (charge_mode IN ('prepaid_balance', 'credit_line', 'postpaid_invoice',
                               'plan_allowance', 'free_trial', 'byok')),
    CONSTRAINT ck_acct_billing_policy_settlement_mode
        CHECK (settlement_mode IN ('synchronous', 'asynchronous')),
    CONSTRAINT ck_acct_billing_policy_admission_mode
        CHECK (admission_mode IN ('strict', 'strict_with_envelope', 'bounded_overdraft')),
    CONSTRAINT ck_acct_billing_policy_overage_policy
        CHECK (overage_policy IN ('allow_receivable', 'fail', 'degrade')),
    CONSTRAINT ck_acct_billing_policy_allowance_period
        CHECK (allowance_period IS NULL OR allowance_period IN ('day', 'week', 'month', 'none')),
    CONSTRAINT ck_acct_billing_policy_allowance_reset
        CHECK (allowance_reset_mode IS NULL OR allowance_reset_mode IN ('calendar', 'rolling')),
    CONSTRAINT ck_acct_billing_policy_input_token_source
        CHECK (input_token_source IN ('bytes_div4', 'tokenizer')),
    CONSTRAINT ck_acct_billing_policy_precharge_ratio
        CHECK (precharge_ratio >= 0.0000 AND precharge_ratio <= 10.0000),
    CONSTRAINT ck_acct_billing_policy_max_hold_share
        CHECK (max_hold_share > 0.0000 AND max_hold_share <= 1.0000),
    CONSTRAINT ck_acct_billing_policy_output_token_ceiling
        CHECK (output_token_ceiling >= 1 AND output_token_ceiling <= 1000000),
    CONSTRAINT ck_acct_billing_policy_hold_ttl
        CHECK (hold_ttl_seconds >= 60 AND hold_ttl_seconds <= 86400),
    CONSTRAINT ck_acct_billing_policy_non_negative
        CHECK (min_hold_points >= 0 AND overdraft_envelope_points >= 0 AND credit_limit_points >= 0
               AND (allowance_quota IS NULL OR allowance_quota >= 0)),
    CONSTRAINT ck_acct_billing_policy_effective_window
        CHECK (effective_to IS NULL OR effective_to > effective_from)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_acct_billing_policy_uuid
    ON acct_billing_policy (uuid);

-- One live policy per (scope, priority): a newer revision supersedes an older
-- one instead of both staying live. "Live" means enabled and not deleted, so
-- retiring a policy (`status = 0`) frees the slot for a replacement instead of
-- leaving the scope permanently occupied by a disabled row.
CREATE UNIQUE INDEX IF NOT EXISTS uk_acct_billing_policy_scope_priority
    ON acct_billing_policy (tenant_id, organization_id, scope_type, scope_id, priority)
    WHERE deleted_at IS NULL AND status = 1;

CREATE INDEX IF NOT EXISTS idx_acct_billing_policy_lookup
    ON acct_billing_policy (tenant_id, organization_id, scope_type, scope_id, status,
                            effective_from DESC, revision DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_acct_billing_policy_code
    ON acct_billing_policy (tenant_id, policy_code)
    WHERE deleted_at IS NULL;

COMMIT;
