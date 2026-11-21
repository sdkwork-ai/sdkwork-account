-- sdkwork:migration
-- id: 0001_organization_id_not_null
-- engine: postgres
-- module: sdkwork-account
-- purpose: Enforce organization_id NOT NULL DEFAULT on all tables in the
--   consolidated baseline. NULL rows (pre-standard data anomalies) are
--   backfilled with the platform sentinel before NOT NULL is set, and
--   NOT NULL columns without an explicit default receive the sentinel
--   default, keeping existing deployments consistent with fresh baseline
--   installs.
-- reversible: false
-- rollback: forward-fix (sentinel backfill is the canonical fix; NULL
--   organization rows are data anomalies)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

ALTER TABLE acct_account ADD COLUMN IF NOT EXISTS organization_id BIGINT NOT NULL DEFAULT 0;
UPDATE acct_account SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE acct_account ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE acct_account ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE acct_ledger_entry ADD COLUMN IF NOT EXISTS organization_id BIGINT NOT NULL DEFAULT 0;
UPDATE acct_ledger_entry SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE acct_ledger_entry ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE acct_ledger_entry ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE acct_hold ADD COLUMN IF NOT EXISTS organization_id BIGINT NOT NULL DEFAULT 0;
UPDATE acct_hold SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE acct_hold ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE acct_hold ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE acct_transfer ADD COLUMN IF NOT EXISTS organization_id BIGINT NOT NULL DEFAULT 0;
UPDATE acct_transfer SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE acct_transfer ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE acct_transfer ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE acct_token_bank_settlement_snapshot ADD COLUMN IF NOT EXISTS organization_id BIGINT NOT NULL DEFAULT 0;
UPDATE acct_token_bank_settlement_snapshot SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE acct_token_bank_settlement_snapshot ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE acct_token_bank_settlement_snapshot ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE acct_billing_history ADD COLUMN IF NOT EXISTS organization_id BIGINT NOT NULL DEFAULT 0;
UPDATE acct_billing_history SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE acct_billing_history ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE acct_billing_history ALTER COLUMN organization_id SET NOT NULL;

COMMIT;
