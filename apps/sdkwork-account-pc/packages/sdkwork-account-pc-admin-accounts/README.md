# @sdkwork/account-pc-admin-accounts

Account centre administration for the SDKWork account service.

- Surface: `backend-admin` (`/backend/v3/api`, generated backend SDK)
- Domain: `commerce` / capability `accounts`
- Routes: `/admin/accounts`, `/admin/accounts/overview`, `/ledger`, `/allowances`,
  `/policy`, `/simulate`

## What it owns

- the operator views over wallet accounts, ledger entries and allowance lots;
- the billing-policy editor (charge mode, settlement mode, admission parameters,
  credit line, allowance policy, overage policy);
- the admission simulator, which answers "would this charge be admitted, and how
  much would it freeze" **without creating a hold** — the diagnostic that turns a
  "balance is sufficient but the request failed" report into one sentence.

## What it must not own

- wallet arithmetic or admission rules (the account service decides; this package
  only calls it and renders the answer);
- unit prices, price books or catalogue defaults (pricing owner);
- recharge catalogue or exchange rates (recharge owner in the same service).

## Wiring

The service takes an injected generated backend client
(`createSdkworkAccountAdminService(client)`); views receive the service through
props. No raw `fetch`, no manual auth headers, no edits to generated SDK output.

## Shipped views

All six declared routes have a view:

- `AccountsOverviewPage` — available / frozen / frozen-share summary plus the
  account table; the frozen share is the figure the production incident needed and
  nobody could see.
- `AccountsDirectoryPage` — owner / organization / asset filters over the same
  rows, so an operator can find *which* wallet is frozen.
- `LedgerPage` — paged ledger entries with balance-after per row.
- `AllowancesPage` — paged allowance lots with remaining amount and expiry
  (read-only until lots become a funding source).
- `PolicyPage` — billing-policy editor: charge mode, settlement mode, admission
  parameters, overage policy, plus activate/retire of stored revisions.
- `AdmissionSimulatePage` — the standalone simulator: estimate in, decision and
  freeze amount out, with the reject reason when it refuses. Nothing is charged.

## Localization

Operator copy lives in the package: src/i18n/<locale>/commerce/accounts/<fragment>.ts for zh-CN and n-US, aggregated by ccountsAdminMessages(locale). Each slice is structurally the copy object its view expects, so a host passes ccountsAdminMessages(locale).overview straight into AccountsOverviewPage. Switching language is a host decision; the package never hardcodes an operator-facing string.

## Verification

- `tests/account-admin-service.test.ts` runs on the Node test runner with type
  stripping: `node --test tests/account-admin-service.test.ts` (10 cases covering
  the frozen-share math, empty-wallet null share, scope forwarding, paging
  parameter naming, numeric estimates, every admission decision state, policy
  listing, the full upsert payload, activation/retirement and the null resolution).
- Every source file parses with a TSX loader.

## Known limitations

- `typecheck` requires the workspace install. The account workspace dependency
  graph declares `@sdkwork/app-topology` in the root `package.json` while the
  package is absent from the specs workspace registry, so `pnpm install` fails
  until that declaration or the registry entry is reconciled.
- Allowance lots are rendered read-only: the funding path for `plan_allowance`
  and `free_trial` is fail-closed in the account service until lots are wired as a
  funding source.
