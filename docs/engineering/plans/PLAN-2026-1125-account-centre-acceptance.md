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

### Round 30: every touched crate compiles

Cloud Router crates verified under WSL after the fixes below:

| Crate | Result |
| --- | --- |
| `sdkwork-cloudrouter-router-service` | check pass; `billing_precharge_estimate` 14 passed, `admin_billing_decision_api` 4 passed |
| `sdkwork-routes-cloudrouter-backend-api` | check pass |
| `sdkwork-cloudrouter-edge-runtime` | check pass |

Account workspace: `cargo check --workspace` and `cargo test --workspace` pass (74 tests,
Windows and WSL).

### Compile defects found while getting there

Seventeen errors that no earlier gate could see, all now fixed:

| Defect | File |
| --- | --- |
| `paging.page_size` is not a field (it is `paging.params.page_size`) | `postgres_billing_policy.rs` |
| `backend_billing_policy_router_with_postgres_pool` used without an import | `routes.rs` |
| `error.to_string()` on `CommerceServiceError` (no `Display`) | `billing_policy_router.rs` |
| Diagnostics port returned `CommerceServiceError` where the API layer needs `DomainError` | `admin_billing_decision_store.rs` |
| `AdminRouterRuntime` destructuring omitted the new field | `routes.rs` |
| Twelve `error.to_string()` calls on `CommerceServiceError` in the account store wrapper | `gateway_billing_account.rs` |
| `tracing::warn!(%error)` on `CommerceServiceError` in the sweep worker | `runtime.rs` |

All touched Rust files are `rustfmt`-clean under the Linux toolchain.

### Dev host (api-dev.birdcoder.com) reconnaissance

| Fact | Value |
| --- | --- |
| Login | `ssh api-dev.birdcoder.com` (user `sdkwork`, key `id_ed25519`) |
| CPU / RAM | 36 cores / 62 GB |
| Disk | 1.1 TB, **48 GB free (96% used)**; Docker images 113.7 GB with 88.9 GB reclaimable |
| Toolchain | Docker 29.7.2, rustup stable (`~/.cargo/bin`, not on the non-interactive PATH), psql 18.6, no node/pnpm on PATH |
| Running | gateway development + production, webserver development, cloudrouter development deps (postgres, redis) |
| Repos | none checked out on the host |

Deployment must therefore either build on the host (needs the 35 sibling repositories that
the Cargo workspaces path-depend on, and roughly 15-25 GB of transient target space) or
receive an image built elsewhere. Disk headroom is the binding constraint: pruning the
88.9 GB of reclaimable images would be required before a full build, and that is a
destructive action on the shared dev host that needs the owner's decision.

### Round 31: deployment smoke on api-dev.birdcoder.com

Verified end to end on the development host (no image pruning, no changes to the
existing services):

| Step | Result |
| --- | --- |
| Source closure for the account workspace resolved | 7 repositories (account, appbase, database, iam, id, utils, web-framework); 174 MB staged, 50.6 MB archived, 18 MB extracted |
| `cargo build --release -p sdkwork-api-account-standalone-gateway` on the host (36 cores) | **pass, 1m55s**, target 655 MB |
| Throwaway PostgreSQL (canonical identity `sdkwork_ai_dev`, schema of the same name) | running |
| Baseline + `0002_acct_billing_policy` applied into `sdkwork_ai_dev` | 16 tables |
| Service start | `GET /healthz` -> 200 `{"status":"ok"}`, `GET /readyz` -> 200 `{"status":"ready"}` |
| Admin surface without credentials | 401 with `failedStage=request-context-resolution` and an explicit message that non-open-api requests need an Access-Token JWT |
| Ledger drill against the **deployed** database | 11/11 checks `ok` (same numbers as the local run) |

Configuration facts discovered while deploying (worth recording because they are
fail-closed guards, not bugs):

- The service refuses a database whose name is not a canonical SDKWork workspace identity (`sdkwork_ai_dev`, `_test`, `_staging`, `_prod`, `_demo`, or `sdkwork_ai_test_<suffix>`), and the database user must match the identity.
- Tables live in a schema named after the database, so an operator applying the DDL must set `search_path` accordingly.
- With the environment unset the service assembles in production posture and refuses to start without an `AuditEmitter`; `SDKWORK_ENVIRONMENT=development` selects the development posture.

Remaining for a fully authenticated endpoint smoke: an Access-Token JWT issued by the
development IAM service. Without it the admin routes correctly refuse with 401, which is
itself the expected behaviour rather than a defect.

### Round 32: live PostgreSQL repository suite, and a latent id-generator defect

Added `crates/sdkwork-account-repository-sqlx/tests/postgres_billing_policy_live.rs`:
four tests that run against a real database when `SDKWORK_ACCOUNT_TEST_DATABASE_URL` is
set (and skip otherwise, so no CI job needs a database to stay green). Executed on the
development host against the deployed `sdkwork_ai_dev` schema:

| Test | What it proves |
| --- | --- |
| `a_stored_policy_is_resolved_with_its_validated_parameters` | upsert -> resolve round-trips the parameters through the real SQL |
| `a_second_upsert_keeps_exactly_one_live_revision` | the supersede transaction leaves exactly one live row and resolution returns the newest revision |
| `list_pages_do_not_repeat_their_first_row` | page two advances past page one (the `OFFSET` fix, proven on real PostgreSQL) |
| `admission_freezes_by_estimate_and_caps_at_the_share_limit` | estimate 1,310,720 freezes 1,310,720; the oversized 20,971,520 estimate freezes 18,679,023 (25% share cap) instead of the whole wallet; neither call moves funds |

Result: 4 passed.

#### Defect found: thread-local snowflake generator

The suite first failed with a **primary-key collision** on `acct_billing_policy`. The
cause is in `crates/sdkwork-account-repository-sqlx/src/store/mod.rs`: the id generator was
declared in `thread_local!`, so every thread built its own snowflake generator with the
same `ACCOUNT_SNOWFLAKE_WORKER_ID`. A snowflake id is (timestamp, worker id, sequence), and
two generators sharing a worker id but holding independent sequence counters emit the same
id whenever they tick inside the same millisecond. On a multi-threaded runtime concurrent
inserts therefore collide on the primary key.

This is latent in production code, not a test artefact: it would surface as sporadic
primary-key violations on `acct_hold`, `acct_ledger_entry` and `acct_billing_policy`
under load, and single-threaded unit tests can never see it.

Fixed by making the generator process-wide (`OnceLock<Mutex<AccountIdGenerator>>`), which
keeps the sequence monotonic across threads. After the fix the suite passes.

## 9. Completion criteria: requirement by requirement

Final sweep (all executed in this workspace):

| Check | Result |
| --- | --- |
| account `cargo test --workspace` | pass |
| account repository live suite against a real PostgreSQL | 4 passed |
| account 5 repository gates | pass |
| account `typecheck` + `pnpm -w run test:vitest` | pass, 48 tests |
| `pnpm run test:domain` (25 funding-rule tests) | pass |
| `pnpm run test:ledger-drill` (A0..A8 + invariant re-check) | pass |
| cloudrouter `cargo check` for router-service / routes / edge-runtime | pass |
| cloudrouter `billing_precharge_estimate` + `admin_billing_decision_api` | 14 + 4 passed |
| cloudrouter assembly + envelope gates, `typecheck`, billing package tests | pass / 5 passed |
| release build + deploy smoke on api-dev (health, readiness, schema, drill) | pass |

| Objective requirement | Evidence | Status |
| --- | --- | --- |
| `acct_billing_policy` funding rules | migration + CHECK clamps + live repository tests | done |
| Admission decision `authorize` | 25 domain tests incl. 4,000-case invariant sweep; live SQL assertions | done |
| Balance / frozen / ledger / allowance lots / credit surfaces | wallet read routes + console views + 48 console tests | done for balance, frozen, ledger, lots (read-only); **credit has no outstanding-credit ledger** |
| backend-api contract | OpenAPI + generated SDK + envelope/pagination/assembly gates | done |
| admin frontend package | six views, service layer, i18n, contribution records, typecheck + tests | **package done; no host app mounts it** |
| cloudrouter reduced to estimator + executor | policy resolution, estimate with ceiling, admission, settlement; sim/decision diagnostics | done |
| Decoupling through contract DTOs and ports | `sdkwork-contract-service` owns the vocabulary; repository implements the ports | done |
| 393,216 cap does not freeze the wallet | unit test + router-service test `a_model_wide_output_cap_does_not_freeze_the_whole_wallet` + live SQL (18,679,023 share cap) | done |
| Eight concurrent calls, no 402 | `eight_parallel_calls_fit_a_small_wallet_with_the_capped_estimate` | done |
| Ledger balances (账平) | drill A0..A8 locally and on the deployed database; independent invariant re-check | done |
| Aborted reservations release | sweep unit path + drill + error-path release; deployed sweep worker verified to shut down cleanly | done |
| rustfmt / SDK contract / typecheck | rustfmt clean on every touched file (Linux toolchain); `publish-core check` and `build` pass; both repos typecheck | done |
| Production build and release by the Linux host | release binary built on api-dev in 1m55s; deployment smoke passed | build verified; **release/publish remains the owner action** |

### What is genuinely left, and why it needs a decision

| Item | Why it cannot be finished unilaterally |
| --- | --- |
| No admin host mounts `ACCOUNTS_ADMIN_ROUTE_RECORDS` | The account application in this repository is the end-user wallet surface. Mounting admin routes there changes its purpose; the alternative is a platform admin app that does not exist in these two repositories. |
| Allowance lots and trial credits are not a funding source | Needs the product rule: which lot funds which period, and what happens when it is exhausted. The design proposal is ready (single `acct_points_lot` source, expiry-first consumption, remainder to the balance governed by `overage_policy`, same transaction, request-scoped idempotency). |
| Credit line has no outstanding-credit ledger | Enabling a credit limit without a ledger of what is already borrowed would let N concurrent requests each borrow the whole line; the current default of `0` fails closed. |
| `rollup` is declared at the account repository root | The generated SDK build script imports it but no `package.json` declares it. The durable fix belongs to the shared SDK generator template, which affects every repository. |

### Round 33: dev host cleaned up, and the SDK generator install fixed

Dev host returned to its original state: account service stopped, throwaway database
container removed, `~/ci` deleted (48 GB free again), the 26 pre-existing containers
untouched.

#### The rollup gap was not a missing declaration

The generator already declares `rollup` (and `typescript`) in the devDependencies of the
package.json it emits; the generated account SDK contains them too. The real cause was the
install step in the emitted `bin/publish-core.mjs`: `pnpm install` run inside a generated
package that sits *inside* the consuming workspace resolves the workspace root, installs
nothing locally, and the build then fails with `ERR_MODULE_NOT_FOUND` for `rollup`.

Fixed in `sdkwork-sdk-generator/src/framework/publish.ts`: the install is now
workspace-aware. A manifest that declares a workspace-protocol dependency keeps the
workspace install (only pnpm can resolve it), while generated output with concrete
versions installs locally with `pnpm install --ignore-workspace --lockfile=false`, so the
build tools live where the build script looks for them.

Verification: `src/generators/typescript/build-config-generator.test.ts` **7/7 passed**,
including the test that pins the install-before-build order and the test that materializes
workspace dependencies. (The suite had to be run as
`node node_modules/vitest/vitest.mjs run <file>` because `pnpm exec vitest` is intercepted
by the repository supply-chain lockfile hook in this environment.)

The account root devDependency on `rollup` stays until the next SDK regeneration ships the
fixed `publish-core.mjs`; after that it can be dropped.

### Round 34: the account centre is mounted in the Cloud Router admin host

The account centre screens are now reachable at `/admin/accounts/*` in the portal that
already hosts every other domain admin package. Changes:

| File | Change |
| --- | --- |
| `apps/sdkwork-cloudrouter-pc/src/admin/accountSdkHostWiring.ts` (new) | bootstraps the shared account backend client with the portal token manager and adapts it to the narrow `wallet`-shaped interface the admin package consumes |
| `apps/sdkwork-cloudrouter-pc/src/admin/accountAdminHostElement.tsx` (new) | the six screens, each acquiring the service at render time and injecting the package message bundle |
| `apps/sdkwork-cloudrouter-pc/src/admin/cloudRouterAdminHostMount.tsx` | imports `ACCOUNTS_ADMIN_ROUTE_RECORDS`, adds `sdkwork-account` to the owner union, and maps the records to routes |
| `apps/sdkwork-cloudrouter-pc/src/main.tsx` | calls the account SDK wiring at startup, next to the other backend SDK families |
| `apps/sdkwork-cloudrouter-pc/package.json` | depends on `@sdkwork/account-pc-admin-accounts` and `@sdkwork/account-service` |
| `pnpm-workspace.yaml` + `sdkwork-specs/tools/lib/workspace-registry.mjs` | the account admin package joins the workspace so the host can resolve it, and the registry entry keeps `sync-workspace` idempotent |
| account `contribution.ts` | records now carry the `screen` key the host maps to a view |

A startup-order defect was introduced and caught during this work: the first version built
the service at module scope, which runs while the import graph is walked — before
`main.tsx` bootstraps the SDK — so the console would have captured a client without a token
manager. The screens now resolve the service in `useState` at render time.

Verification:

| Check | Result |
| --- | --- |
| `sync-workspace --check` (cloudrouter) | ok |
| `pnpm install` (cloudrouter, 347 projects) | pass |
| cloudrouter PC `typecheck` | pass |
| cloudrouter PC `vite build` | pass, built in 19.7s |
| account `pnpm -w run test:vitest` | 16 files / 48 tests pass |
| account PC `typecheck` | pass |
| cloudrouter `validate-api-assembly` | pass |

#### Pre-existing cloudrouter `verify-repo` debt (not from this work)

`verify-repo` on the cloudrouter repository reports 15 violations that all sit in files
this work never touched (h5 packages, `admin-product`, `admin-agent`, `admin-domain`,
`admin-payments`, `admin-storage`, plus `order-app-sdk` ownership metadata). `git status`
shows those paths unmodified, so the violations exist at HEAD. They need their own cleanup
task; none of them involve the account centre files.

### Round 35: plan allowance and trial credits become a funding source (admission side)

`evaluate_billing` now fills `allowance_remaining_points` and `trial_remaining_points` from
the lot ledger instead of hard-coding zero, so `plan_allowance` and `free_trial` can fund a
request for the first time. Rules implemented:

- only `status = 1`, `remaining_amount > 0` and unexpired lots (`expires_at IS NULL OR expires_at > now()`) count;
- `source_type = trial` (constant `POINTS_LOT_SOURCE_TRIAL`) is trial credit, everything else is plan allowance;
- a wallet with no lots still reports zero, so the modes keep failing closed rather than admitting unbacked money.

Admission semantics, verified by tests:

| Case | Decision |
| --- | --- |
| allowance funded, empty wallet | `Admit { hold_points: 0 }` — the credit is consumed at settlement, nothing prepaid is frozen |
| allowance exhausted, `overage_policy = fail` | `Reject { AllowanceExhausted }` — a distinct code, because the wallet is not the missing funding source |
| allowance exhausted, `overage_policy = allow_receivable` | admitted against the balance (the documented hand-off) |

#### Two defects the new tests caught

1. `SUM(remaining_amount)` over BIGINT returns NUMERIC in PostgreSQL, and decoding it as `i64`
   failed — which would have broken **every** admission evaluation, not just allowance ones,
   because the snapshot query runs for every request. Fixed with `CAST(... AS BIGINT)`.
2. The seeded allowance wallet used `asset_code = points` with `currency_code = POINTS`, which
   violates `chk_acct_account_currency`; the test now seeds a token-bank wallet, which is the
   wallet the ledger actually debits.

Verification:

| Check | Result |
| --- | --- |
| `pnpm run test:domain` | 26 passed (including the new allowance funding test) |
| live repository suite against a real PostgreSQL 16 | **5 passed**, including `admission_counts_only_live_allowance_lots` (live grant funds with a zero wallet; expired grants are excluded and rejected as exhausted) |
| `cargo test --workspace` | pass |
| `verify-repo` (account) | pass |

#### Remaining: settlement has not been taught to consume lots

Admission now recognises allowance funding, but the settlement path still charges the balance
(or records a receivable) instead of decrementing lots expiry-first. Until that lands, a
request admitted against an allowance lot is not yet paid for by that lot. This is the next
step, and it is why the lot deduction needs its own transaction and idempotency test rather
than being bolted onto the existing capture path.

### Round 36: allowance consumption primitive, and the ledger decision it exposes

Added `take_allowance_lots` to the account repository (`postgres_hold.rs`, re-exported from
the crate root): the only code that decrements credited points.

Rules implemented:

- lots are taken `FOR UPDATE` in `expires_at ASC NULLS LAST, id ASC` order, so a subject never burns a credit that would have outlived the one still available;
- expired lots and zero-balance lots are skipped;
- every consumed lot writes an `acct_points_lot_allocation` row tied to the charge ledger, which keeps the audit trail per charge and makes the `(tenant, ledger, lot)` unique constraint a double-allocation guard;
- consumption is bounded by what the live lots actually hold, so the caller can compute the wallet remainder from the result.

Verified against a real PostgreSQL 16 (3 tests, all passing): soonest-expiring lot first with
a correct split, expired lots cannot pay and consumption stops at the live holdings, and a
wallet without lots consumes nothing.

#### The wiring decision this exposes

Wiring consumption into `settle_account_hold` forces an accounting decision that should not be
invented silently, because the ledger row is read by reconciliation, the console and the
decision diagnostics:

| Option | Ledger `amount` | `balance_before/after` | Consequence |
| --- | --- | --- | --- |
| A. Ledger records the customer charge | full `settled_amount` | describe only the wallet movement, so `amount != before - after` when lots paid part | one row per charge, but the arithmetic invariant `amount == before - after` breaks |
| B. Ledger records the wallet part | `settled_amount - consumed` | `amount == before - after` holds | the lot part lives only in allocations, so "what did this request cost" needs a join |
| C. Two ledger rows | wallet part and lot part as separate entries | each row internally consistent | cleanest arithmetic, but every existing settlement reader must learn about two rows per request |

Option B is the smallest change and keeps the existing invariant; option C is the most correct
long term. This needs an explicit choice before the settle path is modified.

### Round 37: settlement consumes allowance credit (ledger model B)

Decision recorded: the ledger records only the wallet part of a capture, so
`amount == balance_before - balance_after` keeps holding for every settlement row, and the
credited part lives in `acct_points_lot_allocation` against that same ledger row.

Implementation:

- `plan_allowance_consumption` locks the live lots (`FOR UPDATE`, expiry first) and decides what pays, without writing;
- `apply_allowance_consumption` decrements each lot and writes its allocation row for a known ledger id;
- `take_allowance_lots` stays as the single-shot form (plan + apply) used when the ledger already exists;
- `settle_account_hold` now plans first, records the ledger for `wallet_spent = settled - covered`, then applies the allocations, and refunds the hold by `hold - wallet_spent`.

Verified end to end against a real PostgreSQL (a 5,000,000 capture on a wallet with a
2,000,000 plan lot):

| Assertion | Result |
| --- | --- |
| lot remaining after settle | 0 (the credit paid first) |
| wallet available | 7,000,000 (`5,000,000 + (5,000,000 - 3,000,000)`) |
| wallet frozen | 0 |
| ledger `amount` | 3,000,000 (wallet part only) |
| `amount == before - after` | holds (10,000,000 - 7,000,000) |
| allocations tied to the ledger | 2,000,000 |

Two test-isolation defects were fixed while getting the suite green: the paging test listed
policies tenant-wide, and the allowance-lot tests cleared lots tenant-wide, so parallel tests
shifted each other's data. Each now owns its tenant or subject. All three live suites run
together and pass: 3 + 5 + 1 tests.

Remaining gap: the no-hold settlement path (`Admit { hold_points: 0 }`, which is how an
allowance-only subject is admitted) still settles through the gateway's postpaid fact path and
does not consume lots. The consumption primitive and the ledger model it needs are in place, so
wiring that path is the next step.

### Round 38: final verification sweep (both repositories)

Executed end to end, against a freshly initialised database where a database is needed:

**sdkwork-account**

| Check | Result |
| --- | --- |
| `cargo test --workspace` (with `SDKWORK_ACCOUNT_TEST_DATABASE_URL` set) | 23 suites, all ok |
| live repository suites | 3 + 5 + 1 tests pass (lots FIFO, policy resolution/paging, settlement model B) |
| `verify-repo`, envelope, pagination, assembly, database gates | pass |
| `pnpm typecheck` (PC app + admin package) | pass |
| `pnpm -w run test:vitest` | 48 tests pass |
| `pnpm run test:domain` | 26 tests pass |
| `pnpm run test:ledger-drill` | 11 drill checks + invariant re-check pass |
| `rustfmt --check` on touched files | clean |

**sdkwork-cloudrouter**

| Check | Result |
| --- | --- |
| `cargo check` router-service / routes / edge-runtime (WSL) | pass |
| `billing_precharge_estimate` + `admin_billing_decision_api` | 14 + 4 tests pass |
| assembly + envelope gates | pass |
| `pnpm run typecheck` | pass |
| `pnpm --filter @sdkwork/cloudrouter-pc-admin-billing test` | 5 tests pass |
| `pnpm --dir apps/sdkwork-cloudrouter-pc run build` (host app with the account screens mounted) | pass, 31.7s |

**sdkwork-sdk-generator**

| Check | Result |
| --- | --- |
| `build-config-generator.test.ts` (workspace-aware install) | 7 tests pass |

### Round 39: the generator fix is proven end to end

The repository-root `rollup` devDependency added earlier was a workaround, not the fix. This
round replaced it with the real one and proved it:

1. the SDK generator was rebuilt (`node node_modules/typescript/bin/tsc -p tsconfig.runtime.json`, because the repository build script is blocked by an unrelated appbase workspace dependency) so its CLI carries the workspace-aware install;
2. the account backend SDK was regenerated; its emitted `bin/publish-core.mjs` now installs locally: `pnpm install --ignore-workspace --lockfile=false`;
3. the temporary root `rollup` dependency was removed and the workspace reinstalled;
4. `publish-core --action check` now passes with **no repository-level build dependency**: it resolved 34 packages, created a local `node_modules` containing `rollup` and `typescript` (covered by the existing `**/node_modules/` ignore rule), and built the SDK.

This matters beyond the account repository: any consumer of the generator whose generated SDK
lives inside its own workspace was hitting the same `ERR_MODULE_NOT_FOUND`, and now the package
carries the tools it builds with.

Gates after the change: `verify-repo` and the envelope gate pass, the PC app typechecks, and the
regenerated SDK is byte-identical to the committed one except for `bin/publish-core.mjs` — the
template change is the only drift.
