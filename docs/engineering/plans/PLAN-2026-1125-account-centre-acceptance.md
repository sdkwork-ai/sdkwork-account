# PLAN-2026-1125 Account centre commercial acceptance

Status: implementation complete in-repo, pending Linux build-host verification.
Owner: account domain (funding rules) with cloudrouter as estimating consumer.

## 1. Ownership result

| Concern | Owner | Evidence |
| --- | --- | --- |
| Funding policy data (`acct_billing_policy`) | `sdkwork-account` | `database/migrations/postgres/0002_acct_billing_policy.up.sql` |
| Admission rule (who may spend, how much to freeze) | `sdkwork-account` | `crates/sdkwork-account-service/src/domain/billing_policy.rs` (`authorize`) |
| Shared vocabulary (modes, parameters, decisions) | `sdkwork-contract-service` | `crates/sdkwork-contract-service/src/billing_policy.rs` |
| Policy storage, wallet snapshot, decision | `sdkwork-account` | `crates/sdkwork-account-repository-sqlx/src/postgres_billing_policy.rs` |
| Charge estimation (unit price × quantity) | `sdkwork-cloudrouter` | `application/invocation/billing_transaction.rs` (`estimated_amount`) |
| Hold settlement and overage capture | `sdkwork-cloudrouter` | `billing_transaction.rs`, `gateway_billing_account.rs` |
| Hold reconciliation (abandoned reservations) | `sdkwork-account` rule, cloudrouter schedule | `postgres_hold.rs` (`sweep_expired_holds`), `runtime.rs` (`maybe_spawn_postgres_hold_sweep_worker`) |

The routing runtime no longer holds funding rules: it resolves the account policy,
estimates the charge with the policy's output ceiling, and asks the account side
for a decision. The account side never learns a price, a model or a vendor.

## 2. Acceptance matrix

| # | Criterion | Evidence | Status |
| --- | --- | --- | --- |
| 1 | A model-wide `max_tokens` (393,216) cannot freeze the wallet | `eight_parallel_calls_fit_a_small_wallet_with_the_capped_estimate`, `the_account_policy_ceiling_bounds_the_reservation` | pass (executed) |
| 2 | Eight concurrent agent calls on a 7.5 CNY wallet produce no 402 | `eight_parallel_calls_fit_a_small_wallet_with_the_capped_estimate` (8 admits, balance left) | pass (executed) |
| 3 | Legacy behaviour is reproducible for contrast | `the_legacy_uncapped_estimate_wedges_the_same_wallet` (2 admits, 6 rejects) | pass (executed) |
| 4 | Abandoned holds release without operator action | `sweep_expired_holds` + worker; SQL verified against the baseline schema | pass (SQL verified) |
| 5 | Frozen balance equals open holds | reconciliation query §3 | pass (SQL verified) |
| 5b | One full lifecycle keeps the wallet balanced | docs/runbooks/account-ledger-balance-drill.sql (checks A0-A7) | pass (drill executed) |
| 6 | Hold amounts stay sane (`settled + released <= amount`) | reconciliation query §3 | pass (SQL verified) |
| 7 | Admission decisions reachable from the console | `POST /wallet/billing_policy/authorize` + `AdmissionSimulatePage` | pass (contract + parse) |
| 7b | One request's billing can be explained after the fact | `GET /billing/decision` (cloudrouter, read-only) + `tests/admin_billing_decision_api.rs`; SQL validated read-only against production for a settled and a rejected request | pass (SQL verified, API test in CI) |
| 8 | Policy edits are versioned, not mutated | `upsert_billing_policy` closes the old revision in-transaction; `uk_acct_billing_policy_scope_priority` | pass (SQL verified) |

Executed suites:

- `sdkwork-account-service`: 19 tests (funding rules) — runnable without the full
  workspace because the crate depends only on `sdkwork-contract-service`.
- `sdkwork-account-pc-admin-accounts`: 10 tests (console service layer).
- Repository/spec gates: `verify-repo`, `check-api-response-envelope`,
  `check-pagination`, `validate-api-assembly`,
  `check-database-framework-standard`.

## 3. Reconciliation queries (verified against `0001_account_baseline.sql`)

The end-to-end companion is [account-ledger-balance-drill.sql](../../runbooks/account-ledger-balance-drill.sql):
it seeds a wallet and a policy, then walks admission -> precharge -> settlement ->
abandoned hold -> sweep and asserts A0..A7 print `ok`.

Run after any funding incident or before a release. `ok` on all three means the
account state is internally consistent.

```sql
-- 1. Frozen balance must equal the open holds of the account.
SELECT 'frozen_vs_holds' AS check_name,
       a.frozen_amount AS account_frozen,
       (SELECT COALESCE(SUM(h.amount), 0) FROM acct_hold h
         WHERE h.account_id = a.id AND h.status = 1) AS open_hold_total,
       CASE WHEN a.frozen_amount = (SELECT COALESCE(SUM(h.amount), 0) FROM acct_hold h
                                     WHERE h.account_id = a.id AND h.status = 1)
            THEN 'ok' ELSE 'mismatch' END AS verdict
  FROM acct_account a
 WHERE a.asset_code = 'token_bank';

-- 2. Frozen share per wallet: above 4000 bps (40%) the wallet will refuse
--    concurrent requests even though it looks funded.
SELECT id, owner_type, owner_id,
       (frozen_amount * 10000) / NULLIF(available_amount + frozen_amount, 0) AS frozen_share_bps,
       CASE WHEN (frozen_amount * 10000) / NULLIF(available_amount + frozen_amount, 0) > 4000
            THEN 'high' ELSE 'ok' END AS verdict
  FROM acct_account
 WHERE asset_code = 'token_bank' AND available_amount + frozen_amount > 0;

-- 3. Hold arithmetic must never exceed the reserved amount.
SELECT count(*) AS bad_rows
  FROM acct_hold
 WHERE settled_amount < 0 OR released_amount < 0
    OR settled_amount + released_amount > amount;

-- 4. Holds that outlived their TTL and are still frozen (the sweep should clear
--    these within one interval).
SELECT count(*) AS overdue_holds, COALESCE(SUM(amount), 0) AS overdue_points
  FROM acct_hold
 WHERE status = 1 AND expires_at IS NOT NULL AND expires_at <= now();
```

## 4. Linux build-host verification sequence

Run on the build host (the environment this repository is released from).

```bash
# account: funding rules, contract, repository, routes, SDK
cd sdkwork-account
node ../sdkwork-specs/tools/sync-workspace.mjs --repo sdkwork-account --root .
pnpm install
cargo test -p sdkwork-account-service
cargo test -p sdkwork-account-repository-sqlx
cargo test -p sdkwork-routes-account-backend-api
node ../sdkwork-specs/tools/verify-repo.mjs --root .
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
node ../sdkwork-specs/tools/check-pagination.mjs --workspace .
node ../sdkwork-specs/tools/validate-api-assembly.mjs --root .
node ../sdkwork-specs/tools/check-database-framework-standard.mjs --root .
node scripts/sync-account-openapi-to-sdks.mjs
node sdks/sdkwork-account-backend-sdk/bin/generate-sdk.mjs
node sdks/sdkwork-account-backend-sdk/sdkwork-account-backend-sdk-typescript/generated/server-openapi/bin/publish-core.mjs \
  --language typescript --project-dir . --action check
node --test apps/sdkwork-account-pc/packages/sdkwork-account-pc-admin-accounts/tests/account-admin-service.test.ts

# cloudrouter: estimation + admission consumption
cd ../sdkwork-cloudrouter
cargo check -p sdkwork-cloudrouter-router-service
cargo test  -p sdkwork-cloudrouter-router-service --test billing_precharge_estimate
cargo test  -p sdkwork-cloudrouter-router-service --test admin_billing_decision_api
cargo check -p sdkwork-cloudrouter-edge-runtime
cargo fmt --check

# cloudrouter contract chain (master OpenAPI -> generated -> authority -> SDK)
pnpm run api:materialize:write
pnpm run api:http-route-manifest:write
node sdks/cloudrouter-backend-sdk/bin/generate-sdk.mjs
pnpm run api:materialize:check
pnpm run api:http-route-manifest:check
node ../sdkwork-specs/tools/validate-api-assembly.mjs --root .
```

### Running the ledger-balance drill

The drill in section 3 needs a disposable database. One command creates a temporary
cluster, applies the baseline and migrations, runs the drill, then re-derives every
invariant from the final state:

```bash
SDKWORK_PG_BIN=/path/to/postgres/bin pnpm run test:ledger-drill
```

Exit codes: `0` all eleven drill checks and the independent re-check passed, `1` a
mismatch, `3` no PostgreSQL binaries were found.

### Running the funding-rule tests without cargo

`cargo test` is the authority on the build host, but the two crates that own the
funding rules have no native dependency, so they can be compiled directly:

```bash
pnpm run test:domain
pnpm run test:ledger-drill   # needs PostgreSQL binaries
```

The script builds `sdkwork-contract-service` as an rlib with `rustc`, builds
`sdkwork-account-service` as a test binary against it and runs it (21 tests at the
time of writing, including the invariant sweep). It exists because the full
dependency graph needs a native TLS library that fails to link on some hosts; it is a
development aid, and `cargo test` on the build host remains the release gate.
## 5. Rollout

1. Deploy account first: the policy table and admission port are additive, and an
   empty table resolves to the documented platform defaults, so behaviour is
   unchanged for every existing subject.
2. Deploy cloudrouter second: it starts asking the account side for decisions. A
   deployment without the account port keeps the previous full-estimate hold
   through the trait default.
3. Watch `frozen_share_bps` and `overdue_holds` (queries 2 and 4) for the first
   hour; a rising overdue count means the sweep worker is not running
   (`SDKWORK_CLOUDROUTER_HOLD_SWEEP_INTERVAL_SECONDS`).

Rollback: restoring a policy is a new revision with the previous values; disabling
one is `status = 0`, which frees its scope slot. The sweep worker can be disabled
with interval `0` without touching any stored data.

## 6. Known blockers

- `pnpm install` fails in the account workspace: the root `package.json` declares
  `@sdkwork/app-topology: workspace:*` while the package is absent from the specs
  workspace registry (and no code imports it). Either add the registry entry or
  drop the dependency and regenerate `pnpm-lock.yaml`.
- Allowance lots and trial credits are not a funding source yet: `plan_allowance`
  and `free_trial` fail closed, and the console renders lots read-only.

## 7. Delivery summary

### What was built

| Area | Delivered |
| --- | --- |
| Funding rules | `acct_billing_policy` table + CHECK clamps + live-row unique index; six charge modes, two settlement modes, three admission modes, three overage policies |
| Admission rule | `authorize()` in the account service; affordability is judged on the discounted estimate, the freeze is bounded by the estimate, the share cap and the balance |
| Shared contract | Modes, parameters, capacity, decision and the authorization query live in `sdkwork-contract-service`; both repos depend on the contract, not on each other |
| Policy storage + wallet snapshot | `PostgresBillingPolicyStore` resolves the most specific live row, closes the previous revision on write, and reads the wallet internally |
| Account centre API | Policy CRUD (5 routes), admission simulation, wallet accounts / ledger / points lots read models, platform-wide hold sweep |
| Console package | `@sdkwork/account-pc-admin-accounts` with six views, injected SDK service, ten service tests |
| Cloudrouter consumer | Resolves the policy, estimates with its output ceiling, asks for a decision, maps `Admit` / `Deferred` / `Free` / `Reject`; no funding rules remain there |
| Diagnostics | Cloudrouter read-only `GET /billing/decision` (gateway master OpenAPI -> generated -> authority -> SDK `BillingDecisionApi`) |
| Recovery | `sweep_expired_holds()` across tenants plus a runtime worker (default 60s, env-tunable, shutdown-aware) |

### Evidence, grouped by how it was verified

Executed on this machine:

- `sdkwork-account-service`: 21 tests, including the 393,216 cap, eight concurrent admissions, the legacy contrast, and a 4,000-case invariant sweep (a reservation never exceeds the share cap, the spendable balance or the discounted estimate; a rejection never carries a reservation; more balance never turns an admission into a rejection; free and postpaid modes never freeze prepaid funds).
- `@sdkwork/account-pc-admin-accounts`: 10 service tests (frozen share, empty wallet, scope forwarding, paging wire name, every admission decision, policy upsert payload).
- `docs/runbooks/account-ledger-balance-drill.sql`: checks A0..A8, all `ok` against `0001_account_baseline.sql` plus the new migration.
- Reconciliation queries (section 3) against the same schema.
- Repository gates: `verify-repo`, `check-api-response-envelope`, `check-pagination`, `validate-api-assembly`, `check-database-framework-standard`; cloudrouter additionally `api:materialize:check` and `api:http-route-manifest:check`.
- Contract chain: master OpenAPI -> generated -> authority -> regenerated SDK exposing `BillingDecisionApi`.

Verified read-only against production Postgres:

- The four-stage decision snapshot for one settled request (hold 16,443,980 -> settled 22,297, three charge lines, `settled`) and one rejected request (`not_billed`).
- The wallet figures used by every test above (74,716,092 total, 33,256,530 frozen).

CI / build host only:

- `cargo check` / `cargo test` for the repository, route and edge-runtime crates (this host cannot link `aws-lc-sys`).
- `cargo test --test admin_billing_decision_api` and `--test billing_precharge_estimate`.
- `pnpm install`, `typecheck`, SDK `publish-core check`, and the release build.

### What remains

1. Run the build-host sequence in section 4 and record the results in the release evidence.
2. Allowance lots and trial credits are not a funding source yet: `plan_allowance` and `free_trial` fail closed, and the console renders lots read-only. Wiring them needs a product decision on which lot funds which period.

## 8. Fix log and open issues

### Fixed after the first acceptance pass

| Issue | Impact | Fix | Evidence |
| --- | --- | --- | --- |
| Policy list ignored paging offset | The console showed page 1 again for every later page | `LIMIT $5 OFFSET $6` with `paging.sql_offset` bound | SQL on the real schema: page 1 = `5,4,3`, page 2 = `2,1` |
| `AdmitFree` still charged at settlement | A waived admission (bring-your-own-key) created no hold, so settlement fell into the "no hold, charge as postpaid fact" branch and debited the wallet | Set `invocation.charging.waived = Some("account_admission_free")`, reusing the existing waived path that releases and never writes a ledger entry | Code path only; needs the build host to compile |
| `free_trial` admitted as deferred | A trial user would have been billed from the balance instead of the trial pool | Admit as `AdmitFree`; test updated with the coupling documented | `trial_and_byok_modes_need_no_reservation`, 21/21 pass |
| Inverted effective window reached the database | A policy with ffective_to <= effective_from passed application validation and failed later as a storage error | Validated in UpsertBillingPolicyCommand::validate; the table CHECK stays as the second line of defence | 4 new tests; SQL confirms ck_acct_billing_policy_effective_window rejects the same row |
| Credit line treated as fully available per request | N concurrent requests could each borrow the whole line | Documented as a limitation at the snapshot site; credit modes stay at their `0` default, which fails closed | Code comment + section 6 |

### Open issues, with owner

| # | Issue | Owner | Severity |
| --- | --- | --- | --- |
| 1 | `pnpm install` blocked by the unused `@sdkwork/app-topology: workspace:*` dependency (nothing imports it; `topology:validate` uses a relative path) | repository maintainer: drop the dependency and regenerate the lockfile, or register the package | blocks typecheck, SDK publish check and release |
| 2 | Neither repository can be compiled on the current workstation (`aws-lc-sys` link failure, MSVC `LNK1104`) | build host | blocks `cargo check/test` and every integration test |
| 3 | Credit line has no outstanding-credit ledger to subtract from | product/architecture decision | medium: credit modes are disabled, not silently wrong |
| 4 | Allowance lots and trial credits are not a funding source | product decision | medium: fail closed |
| 5 | `check-pagination` flags two pre-existing violations in the generated C# open SDK (`VectorStoresApi`, `VideoApi` emit a `limit` query) | open SDK generator owner | low for this work, blocks a clean pagination gate for the repository |

### Checked and found not to be defects

- Admin routers are absent from the route manifest by design: the pre-existing `/billing/finance/ledger` is missing exactly like the new `/billing/decision`, so the manifest is not the record of admin routes. Adding `x-sdkwork-source-route-crate` / `x-source-file` keeps the ownership explicit and passes every gate.
- Policy resolution already filters `status`, `effective_from`/`effective_to` and orders by scope, priority, revision and effective date.
- The abort path already releases a reservation and now also handles the waived case explicitly.

### Fixed once the workspace could actually install and typecheck

Removing the unused `@sdkwork/app-topology` dev dependency (it was declared `workspace:*` while absent from `sdkwork-specs` `FOUNDATION_PNPM_PACKAGES`, and nothing imports it) unblocked `pnpm install`. That turned four latent defects from invisible into test failures, and all four are fixed:

| Defect | Symptom | Fix |
| --- | --- | --- |
| Colliding operation id | The generator emitted `WalletPointsLotsApi` twice and the SDK stopped compiling | Renamed `pointsLots.list` to `points.lots.list`, matching the existing `points.lots.expire`; the two operations now form one class |
| Literal-pinned locale types | The English catalogue was rejected against a Chinese-literal bundle type | `WidenStrings<T>` widens every string leaf so any locale satisfies one bundle type |
| Incomplete fake client | The console service test lost contract coverage when policy methods were added | Fake client now implements the whole policy surface |
| Wrong test registration | Two suites used `node:test` while the repository runner is vitest, so `pnpm -w run test:vitest` failed | Converted to vitest `it()`; assertions keep using `node:assert` |

Round-24 verification (all executed):

| Check | Result |
| --- | --- |
| `pnpm install` (account, 39 projects) | pass |
| `pnpm typecheck` (account PC, includes the admin package) | pass |
| `pnpm -w run test:vitest` (account) | 15 files / 45 tests pass |
| `pnpm install` + `pnpm run typecheck` (cloudrouter, 347 projects) | pass |
| `pnpm --filter @sdkwork/cloudrouter-pc-admin-billing test` | 5 tests pass |
| account gates: verify-repo, envelope, pagination, assembly, database | pass |
| cloudrouter gates: assembly, envelope, materialize | pass |

### Newly discovered, not caused by this work

- `publish-core --action check` cannot build: `custom/build-runtime.mjs` imports `rollup`, and no `package.json` in the account repository declares it. Fix belongs to the SDK generator template (declare `rollup` in the generated package) or the repository root.
- `@sdkwork/cloudrouter-pc-admin-billing` was not registered in the app, so the app typecheck did not cover it. Registered in the app dependencies, given its own `tsconfig.json` and a `test` script, and added to the app vitest `include` list; it now typechecks and its five tests run through the app runner.

### Round 25: contract chain and host wiring

Executed:

| Check | Result |
| --- | --- |
| `pnpm run sdk:generate` (both SDKs) | pass |
| `publish-core --action check` (account backend SDK) | pass |
| `publish-core --action build` (account backend SDK) | pass |
| `pnpm --dir apps/sdkwork-account-pc run build` (vite) | pass, 256 modules |
| `pnpm -w run test:vitest` (account) | 16 files / 48 tests pass |
| `pnpm typecheck` (account PC) | pass |
| account gates (5) | pass |

Fixes and additions:

- `rollup` added as a **repository-root devDependency**: the generated SDK build script imports it and no `package.json` declared it. The durable fix belongs in the SDK generator template (declare `rollup` in the generated package), so this repository entry is a local unblock and should be reconciled with that template fix.
- `@sdkwork/account-pc-admin-accounts` now publishes `./contribution` and `./i18n` subpaths. The contribution record carries path, title key, required permission (`wallet.accounts.read`, or `wallet.billing_policies.manage` for the policy editor), owning package and backend SDK family.
- Contribution mapping is covered by tests: one record per route, manage permission only for the policy editor, every record on `/admin/accounts` and bound to the account backend SDK.

### Open integration issue: no admin host in this repository

The account repository ships two applications: `sdkwork-account-common` and `sdkwork-account-pc`. The latter renders the end-user wallet surface (`AccountAppShell` mounts `SdkworkWalletPage`); it has no admin host, and nothing imports `@sdkwork/account-pc-admin-accounts`. The Cloud Router console solves this with `src/admin/cloudRouterAdminHostMount.tsx`, which consumes each admin package route contribution.

Until a host mounts `ACCOUNTS_ADMIN_ROUTE_RECORDS`, the six account-centre screens are a verified library rather than a reachable surface. Wiring needs one of:

1. a platform admin app that mounts the records (preferred, mirrors Cloud Router);
2. an `admin` entry inside `sdkwork-account-pc` with its own layout and permission guard.

The package side is ready: routes, permissions, contribution records, views, service and message bundles are all in place and typechecked.

### Round 28-29: cargo is available, and it caught four real defects

The earlier conclusion that this workstation cannot compile Rust was too broad. The
WSL Ubuntu 22.04 distribution on this host carries cargo/rustc 1.99.1, and the account
workspace compiles under plain Windows cargo as well; only the Cloud Router dependency
graph needs the Linux toolchain (its `aws-lc-sys` build probe needs `/std:c11` under
MSVC because the probe uses `alignas` and `-WX` promotes the resulting C4013/C4100
diagnostics to errors).

Executed:

| Check | Result |
| --- | --- |
| `cargo check --workspace` (account, Windows) | pass |
| `cargo test --workspace` (account, Windows and WSL) | pass, 74 tests |
| `cargo check -p sdkwork-cloudrouter-router-service` (WSL) | pass |
| `cargo test --test billing_precharge_estimate` (WSL) | 14 passed, including `a_model_wide_output_cap_does_not_freeze_the_whole_wallet` |
| `cargo test --test admin_billing_decision_api` (WSL) | 4 passed |
| `rustfmt --check` on every Rust file this work touched | clean |

Defects cargo found that no earlier gate could see:

| Defect | Where | Why it was invisible |
| --- | --- | --- |
| `paging.page_size` does not exist on `ListSqlPaging` (it is `paging.params.page_size`) | `postgres_billing_policy.rs` | `rustfmt` does not type-check; the crate never reached cargo |
| `backend_billing_policy_router_with_postgres_pool` used without an import | `routes.rs` | same |
| `error.to_string()` on `CommerceServiceError`, which has no `Display` | `billing_policy_router.rs` | same |
| Diagnostics port returned `CommerceServiceError` where the API layer needs `DomainError` | `admin_billing_decision_store.rs` | same |

All four are fixed; the tables above were produced after the fixes.

### Pre-existing repository issues, not caused by this work

`cargo fmt --check` in the Cloud Router repository reports drift in files this work
never touched: `sdkwork-api-cloudrouter-assembly/src/bootstrap.rs`,
`sdkwork-cloudrouter-config/src/request_limits.rs`,
`sdkwork-cloudrouter-edge-runtime/src/passthrough.rs`,
`sdkwork-cloudrouter-edge-runtime/src/provider_passthrough_transport.rs`,
`sdkwork-routes-cloudrouter-app-api/src/routes.rs`.

### One-command verification entry points

| Command | Covers | Needs |
| --- | --- | --- |
| `cargo test --workspace` | every account crate | Rust toolchain |
| `pnpm run test:domain` | the 25 funding-rule tests | Rust toolchain only |
| `pnpm run test:ledger-drill` | the full lifecycle drill plus an independent invariant re-check | PostgreSQL binaries |
| `pnpm -w run test:vitest` | console packages (48 tests) | workspace install |
| `pnpm typecheck` | console type safety | workspace install |
| `publish-core --action check` | generated SDK contract | `rollup` (repo root devDependency) |
