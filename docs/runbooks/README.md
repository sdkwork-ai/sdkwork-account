# Account Runbooks

Operational procedures for `sdkwork-account` in production.

## Startup

```bash
# Required
export SDKWORK_DATABASE_ENGINE=postgres
export SDKWORK_DATABASE_URL=postgres://sdkwork_ai_dev:sdkworkdev123@127.0.0.1:5432/sdkwork_ai_dev
export ACCOUNT_SNOWFLAKE_WORKER_ID=1   # unique per instance (0-1023)

# Recommended
export SDKWORK_CORS_ALLOWED_ORIGINS=https://app.example.com
export ACCOUNT_API_BIND=0.0.0.0:18095

pnpm db:migrate
pnpm start
```

Local development CORS (never use in production):

```bash
export SDKWORK_CORS_ALLOWED_ORIGINS=*
export ACCOUNT_CORS_PERMISSIVE_DEV=1
```

## Health and readiness

| Probe | URL | Pass criteria |
| --- | --- | --- |
| Liveness | `GET /healthz` (platform router) | HTTP 200 |
| Readiness | `GET /readyz` | HTTP 200 when DB answers `SELECT 1` |
| Wallet health | `GET /backend/v3/api/wallet/health` | HTTP 200, `data.item.database=up` |

Degraded DB returns HTTP 503 on wallet health; use HTTP status for alerting, not envelope `code` alone.

## Graceful shutdown

The standalone gateway drains in-flight HTTP requests on SIGINT/SIGTERM, then closes the database pool (30s timeout). Orchestrators should use a termination grace period of at least 45s.

## Outbox relay

Account does not embed an outbox worker. Schedule an external job:

```http
POST /backend/v3/api/wallet/outbox/dispatch
Content-Type: application/json

{ "batchSize": 100 }
```

Forward returned events to your message bus using `eventKey` for consumer idempotency.

## Idempotency lock recovery

Write paths use `acct_idempotency_record` with a 5-minute lock TTL (`locked_until`). Stale locks auto-expire; clients should retry with the same idempotency key after TTL.

## Points reconciliation

```http
POST /backend/v3/api/wallet/points/reconciliation
```

Scans points accounts in batches of 100. Review `mismatchCount` and `mismatches` in the response.

## Incident checklist

1. Check readiness (`/readyz`) and wallet health.
2. Inspect `outboxPendingLag` — schedule dispatch if lag grows.
3. Verify `ACCOUNT_SNOWFLAKE_WORKER_ID` is unique per replica (duplicate IDs cause collision).
4. Run points reconciliation for tenant/org scope if balances look wrong.
5. Review application logs for `account.id`, `account.security`, `account.readiness` targets.


<!-- scaffold-module-runbooks:index -->
## Docker 运维四件套（bin/ 标准，OPERATIONS_SPEC.md §7）

| Runbook | 内容 |
| --- | --- |
| [deploy.md](deploy.md) / [deploy.en.md](deploy.en.md) | 安装 / 升级 / 回滚 / 下线（bin/docker-deploy.sh + bin/docker-image.sh） |
| [troubleshooting.md](troubleshooting.md) / [troubleshooting.en.md](troubleshooting.en.md) | 症状 → doctor 检查 → 处置 |
| [backup-restore.md](backup-restore.md) / [backup-restore.en.md](backup-restore.en.md) | 备份 / 校验 / 恢复 / 演练（bin/backup.sh） |
| [log-reference.md](log-reference.md) / [log-reference.en.md](log-reference.en.md) | 健康日志特征与失败签名（bin/docker-deploy.sh logs） |
