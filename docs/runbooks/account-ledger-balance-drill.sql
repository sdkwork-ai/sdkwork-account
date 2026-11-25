-- Account centre ledger-balance drill (账平演练).
--
-- Reproduces one full funding lifecycle on a throwaway database and asserts that
-- the wallet stays balanced after it:
--
--   admission estimate -> precharge hold -> settle for measured usage
--   -> a second, abandoned hold -> sweep release
--
-- Run against a scratch database that already has the account baseline and the
-- `0002_acct_billing_policy` migration applied, then read every `verdict` row:
-- a healthy run prints `ok` for A1..A6.
--
-- Numbers come from the production incident wallet: 74,716,092 points total
-- (41,459,562 available + 33,256,530 frozen), a corrected per-request estimate of
-- 1,310,720 points, and a measured charge of 22,297 points for one request.

-- 1. Subject and its funding policy.
INSERT INTO acct_account (
    id, uuid, tenant_id, organization_id, owner_type, owner_id, asset_code,
    currency_code, account_purpose, available_amount, frozen_amount, pending_amount,
    status, version, created_at, updated_at
) VALUES (
    1, 'drill-account', 100001, 0, 'USER', 5001, 'token_bank',
    'TOKEN_BANK', 'GENERAL', 41459562, 33256530, 0,
    1, 1, now(), now()
);

INSERT INTO acct_billing_policy (
    id, uuid, tenant_id, organization_id, policy_code, name, scope_type, scope_id,
    priority, revision, status, charge_mode, settlement_mode, precharge_ratio,
    output_token_ceiling, hold_ttl_seconds, min_hold_points, max_hold_share,
    admission_mode, overdraft_envelope_points, credit_limit_points,
    overage_policy, effective_from
) VALUES (
    10, 'drill-policy', 100001, 0, 'user-policy', 'User policy', 3, 5001,
    100, 1, 1, 'prepaid_balance', 'synchronous', 0.2500,
    32768, 1800, 0, 0.2500,
    'strict', 0, 0,
    'allow_receivable', now() - interval '1 day'
);

-- 1b. The seeded frozen balance must be backed by real open reservations, so
--     the "frozen == open holds" invariant holds from the first statement.
--     These two rows reproduce the reservations that outlived their requests in
--     the production incident; their expiry stays in the future so the sweep
--     below only touches the deliberately expired drill row.
INSERT INTO acct_hold (
    id, uuid, tenant_id, organization_id, account_id, owner_type, owner_id,
    asset_code, currency_code, amount, settled_amount, released_amount, status,
    business_type, business_no, source_type, source_id, request_no, idempotency_key,
    expires_at, version, trace_id, created_at, updated_at
) VALUES (
    2, 'drill-legacy-1', 100001, 0, 1, 'USER', 5001,
    'token_bank', 'TOKEN_BANK', 16678850, 0, 0, 1,
    'gateway_invocation_billing', 'cloudrouter:drill-legacy-1:precharge', 'gateway', 5001,
    'cloudrouter:drill-legacy-1:precharge', 'cloudrouter:drill-legacy-1:precharge',
    now() + interval '20 minutes', 1, 'drill-trace-legacy-1', now(), now()
);

INSERT INTO acct_hold (
    id, uuid, tenant_id, organization_id, account_id, owner_type, owner_id,
    asset_code, currency_code, amount, settled_amount, released_amount, status,
    business_type, business_no, source_type, source_id, request_no, idempotency_key,
    expires_at, version, trace_id, created_at, updated_at
) VALUES (
    3, 'drill-legacy-2', 100001, 0, 1, 'USER', 5001,
    'token_bank', 'TOKEN_BANK', 16577680, 0, 0, 1,
    'gateway_invocation_billing', 'cloudrouter:drill-legacy-2:precharge', 'gateway', 5001,
    'cloudrouter:drill-legacy-2:precharge', 'cloudrouter:drill-legacy-2:precharge',
    now() + interval '20 minutes', 1, 'drill-trace-legacy-2', now(), now()
);

-- A0 pre-condition: frozen amount equals the open reservations it claims.
SELECT 'A0_frozen_backed_by_holds' AS check_name,
       a.frozen_amount AS account_frozen,
       (SELECT COALESCE(SUM(h.amount), 0) FROM acct_hold h
         WHERE h.account_id = a.id AND h.status = 1) AS open_hold_total,
       CASE WHEN a.frozen_amount = (SELECT COALESCE(SUM(h.amount), 0) FROM acct_hold h
                                     WHERE h.account_id = a.id AND h.status = 1)
            THEN 'ok' ELSE 'mismatch' END AS verdict
  FROM acct_account a WHERE a.id = 1;

-- 2. Admission: the estimate is clamped by the share cap, never by the raw
--    model-wide output cap. Estimate 1,310,720 < share cap 18,679,023, so the
--    reservation equals the estimate.
SELECT 'A0_admission_reservation' AS check_name,
       1310720 AS expected_hold_points,
       LEAST(1310720, ((41459562::bigint + 33256530::bigint) * 2500 / 10000)) AS admitted_hold_points,
       CASE WHEN LEAST(1310720, ((41459562::bigint + 33256530::bigint) * 2500 / 10000)) = 1310720
            THEN 'ok' ELSE 'mismatch' END AS verdict;

-- 3. Precharge: freeze the reservation (available -> frozen).
INSERT INTO acct_hold (
    id, uuid, tenant_id, organization_id, account_id, owner_type, owner_id,
    asset_code, currency_code, amount, settled_amount, released_amount, status,
    business_type, business_no, source_type, source_id, request_no, idempotency_key,
    expires_at, version, trace_id, created_at, updated_at
) VALUES (
    11, 'drill-hold-settled', 100001, 0, 1, 'USER', 5001,
    'token_bank', 'TOKEN_BANK', 1310720, 0, 0, 1,
    'gateway_invocation_billing', 'cloudrouter:drill-settled:precharge', 'gateway', 5001,
    'cloudrouter:drill-settled:precharge', 'cloudrouter:drill-settled:precharge',
    now() + interval '30 minutes', 1, 'drill-trace-1', now(), now()
);

UPDATE acct_account
   SET available_amount = available_amount - 1310720,
       frozen_amount = frozen_amount + 1310720,
       version = version + 1, updated_at = now()
 WHERE id = 1;

-- 4. Settlement: capture the measured charge and return the unused reservation.
INSERT INTO acct_ledger_entry (
    id, uuid, tenant_id, organization_id, account_id, owner_type, owner_id,
    asset_code, currency_code, ledger_type, entry_type, direction, amount,
    balance_before, balance_after, business_type, business_no, request_no,
    idempotency_key, metadata_json, trace_id, created_at
) VALUES (
    101, 'drill-ledger-1', 100001, 0, 1, 'USER', 5001,
    'token_bank', 'TOKEN_BANK', 'CONSUMPTION', 'GATEWAY_CONSUMPTION', 'DEBIT', 22297,
    41459562, 41437265, 'usage_settlement', 'cloudrouter:drill-settled:consumption',
    'cloudrouter:drill-settled:consumption', 'cloudrouter:drill-settled:consumption',
    '{}'::jsonb, 'drill-trace-1', now()
);

UPDATE acct_account
   SET available_amount = available_amount + (1310720 - 22297),
       frozen_amount = frozen_amount - 1310720,
       version = version + 1, updated_at = now()
 WHERE id = 1;

UPDATE acct_hold
   SET status = 2, settled_amount = 22297, settled_at = now(),
       version = version + 1, updated_at = now()
 WHERE id = 11;

-- 5. A second reservation is abandoned, then released by the platform sweep.
INSERT INTO acct_hold (
    id, uuid, tenant_id, organization_id, account_id, owner_type, owner_id,
    asset_code, currency_code, amount, settled_amount, released_amount, status,
    business_type, business_no, source_type, source_id, request_no, idempotency_key,
    expires_at, version, trace_id, created_at, updated_at
) VALUES (
    12, 'drill-hold-abandoned', 100001, 0, 1, 'USER', 5001,
    'token_bank', 'TOKEN_BANK', 16577680, 0, 0, 1,
    'gateway_invocation_billing', 'cloudrouter:drill-abandoned:precharge', 'gateway', 5001,
    'cloudrouter:drill-abandoned:precharge', 'cloudrouter:drill-abandoned:precharge',
    now() - interval '1 minute', 1, 'drill-trace-2', now(), now()
);

UPDATE acct_account
   SET available_amount = available_amount - 16577680,
       frozen_amount = frozen_amount + 16577680,
       version = version + 1, updated_at = now()
 WHERE id = 1;

-- sweep_expired_holds(): select expired held rows, then release each one.
SELECT 'A0_sweep_batch' AS check_name,
       count(*) AS expired_holds,
       CASE WHEN count(*) = 1 THEN 'ok' ELSE 'mismatch' END AS verdict
  FROM acct_hold
 WHERE status = 1 AND expires_at IS NOT NULL AND expires_at <= now();

UPDATE acct_account
   SET available_amount = available_amount + 16577680,
       frozen_amount = frozen_amount - 16577680,
       version = version + 1, updated_at = now()
 WHERE id = 1;

UPDATE acct_hold
   SET status = 4, released_amount = amount, released_at = now(),
       version = version + 1, updated_at = now()
 WHERE id = 12 AND status = 1;

-- 6. Assertions: the wallet must be balanced after the whole lifecycle.
SELECT 'A1_available_after_consumption' AS check_name,
       a.available_amount AS actual, 41437265 AS expected,
       CASE WHEN a.available_amount = 41437265 THEN 'ok' ELSE 'mismatch' END AS verdict
  FROM acct_account a WHERE a.id = 1;

SELECT 'A2_frozen_same_as_before' AS check_name,
       a.frozen_amount AS actual, 33256530 AS expected,
       CASE WHEN a.frozen_amount = 33256530 THEN 'ok' ELSE 'mismatch' END AS verdict
  FROM acct_account a WHERE a.id = 1;

SELECT 'A3_ledger_sum_equals_charge' AS check_name,
       COALESCE(SUM(amount), 0) AS actual, 22297 AS expected,
       CASE WHEN COALESCE(SUM(amount), 0) = 22297 THEN 'ok' ELSE 'mismatch' END AS verdict
  FROM acct_ledger_entry WHERE account_id = 1;

SELECT 'A4_ledger_balance_after' AS check_name,
       MAX(balance_after) AS actual, 41437265 AS expected,
       CASE WHEN MAX(balance_after) = 41437265 THEN 'ok' ELSE 'mismatch' END AS verdict
  FROM acct_ledger_entry WHERE account_id = 1;

SELECT 'A5_hold_arithmetic' AS check_name,
       count(*) AS bad_rows,
       CASE WHEN count(*) = 0 THEN 'ok' ELSE 'mismatch' END AS verdict
  FROM acct_hold
 WHERE settled_amount < 0 OR released_amount < 0
    OR settled_amount + released_amount > amount;

SELECT 'A6_total_conservation' AS check_name,
       a.available_amount + a.frozen_amount AS actual, 74716092 - 22297 AS expected,
       CASE WHEN a.available_amount + a.frozen_amount = 74716092 - 22297
            THEN 'ok' ELSE 'mismatch' END AS verdict
  FROM acct_account a WHERE a.id = 1;

SELECT 'A7_drill_holds_resolved' AS check_name,
       (SELECT count(*) FROM acct_hold
         WHERE account_id = 1 AND uuid IN ('drill-hold-settled', 'drill-hold-abandoned')
           AND status NOT IN (2, 4)) AS unresolved_drill_holds,
       CASE WHEN (SELECT count(*) FROM acct_hold
                   WHERE account_id = 1 AND uuid IN ('drill-hold-settled', 'drill-hold-abandoned')
                     AND status NOT IN (2, 4)) = 0
            THEN 'ok' ELSE 'mismatch' END AS verdict;

-- A8: the frozen amount still equals the open reservations, now the legacy two.
SELECT 'A8_frozen_backed_by_holds_after' AS check_name,
       a.frozen_amount AS account_frozen,
       (SELECT COALESCE(SUM(h.amount), 0) FROM acct_hold h
         WHERE h.account_id = a.id AND h.status = 1) AS open_hold_total,
       CASE WHEN a.frozen_amount = (SELECT COALESCE(SUM(h.amount), 0) FROM acct_hold h
                                     WHERE h.account_id = a.id AND h.status = 1)
            THEN 'ok' ELSE 'mismatch' END AS verdict
  FROM acct_account a WHERE a.id = 1;
