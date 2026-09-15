# Developer Guide

Local setup and verification for `sdkwork-account`.

## Prerequisites

- Rust toolchain (workspace `Cargo.toml`)
- Node 20+ and pnpm 10+

## Setup

```powershell
cd <workspace-root>/sdkwork-account
pnpm install
```

## Run gateway locally

```powershell
pnpm start
# default bind: ACCOUNT_API_BIND=0.0.0.0:18095
```

## Run PC wallet surface

```powershell
pnpm dev
# optional env (apps/sdkwork-account-pc):
# VITE_SDKWORK_ACCOUNT_API_BASE=http://127.0.0.1:18095
# VITE_SDKWORK_PAYMENT_CHECKOUT_BASE=/checkout
# VITE_SDKWORK_ORDER_WITHDRAWAL_REQUEST_BASE=/withdrawals/requests
```

Recharge buttons delegate to order checkout/payment collection routes. Withdraw buttons delegate to order-owned withdrawal request routes. Run `sdkwork-order` and `sdkwork-payment` (or a composition app such as mall) for full account-value checkout flows.

## Verification

```powershell
pnpm verify
cargo test --workspace
pnpm db:validate
node ..\sdkwork-specs\tools\check-api-response-envelope.mjs --workspace .
```

## Database

Migrations and contract live under `database/`. Use `pnpm db:migrate` after configuring the sdkwork-database CLI for this app root.

See `docs/architecture/tech/TECH_ARCHITECTURE.md` for layer map and API route inventory.
