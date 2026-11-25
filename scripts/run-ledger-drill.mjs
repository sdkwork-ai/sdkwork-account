#!/usr/bin/env node
/**
 * Runs the ledger-balance drill against a throwaway PostgreSQL cluster.
 *
 * `docs/runbooks/account-ledger-balance-drill.sql` walks one full funding lifecycle
 * (admission -> precharge -> settlement -> abandoned hold -> sweep) and asserts the
 * wallet stays balanced. Running it by hand means creating a cluster, applying the
 * baseline plus migrations and reading eleven verdict rows; this script does that and
 * fails on the first `mismatch`, so "the ledger balances" becomes a command instead of
 * a paragraph.
 *
 * Requirements: a PostgreSQL distribution with `initdb` and `postgres` binaries.
 * Point `SDKWORK_PG_BIN` at its `bin` directory, or install it at
 * `<workspace>/.local-pg/bin`.
 *
 * Exit codes: 0 pass, 1 assertion failure, 3 PostgreSQL binaries not found.
 */

import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { existsSync } from 'node:fs';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = path.resolve(repoRoot, '..');

const BASELINE = 'database/ddl/baseline/postgres/0001_account_baseline.sql';
const MIGRATION = 'database/migrations/postgres/0002_acct_billing_policy.up.sql';
const DRILL = 'docs/runbooks/account-ledger-balance-drill.sql';
const EXPECTED_VERDICTS = 11;

function resolvePgBin() {
  const candidates = [
    process.env.SDKWORK_PG_BIN,
    path.join(workspaceRoot, '.local-pg', 'bin'),
    'C:/Program Files/PostgreSQL/16/bin',
    'C:/Program Files/PostgreSQL/15/bin',
  ].filter((candidate) => typeof candidate === 'string' && candidate.length > 0);
  for (const candidate of candidates) {
    const binary = path.join(candidate, process.platform === 'win32' ? 'initdb.exe' : 'initdb');
    if (existsSync(binary)) return candidate;
  }
  return null;
}

const pgBin = resolvePgBin();
if (pgBin === null) {
  console.error(
    '[ledger-drill] SKIPPED: no PostgreSQL binaries found. Set SDKWORK_PG_BIN to the bin directory of a PostgreSQL install.',
  );
  process.exit(3);
}

const exe = (name) => path.join(pgBin, process.platform === 'win32' ? `${name}.exe` : name);
const workDir = mkdtempSync(path.join(tmpdir(), 'sdkwork-ledger-drill-'));

/**
 * `postgres --single` accepts one statement per line and rejects `;`-separated
 * batches, so each file is flattened: comments stripped, whitespace collapsed, split
 * on the terminator.
 */
function flattenToStatements(sql) {
  const withoutComments = sql
    .split('\n')
    .filter((line) => !line.trim().startsWith('--'))
    .join('\n');
  return withoutComments
    .replace(/\s+/gu, ' ')
    .split(';')
    .map((statement) => statement.trim())
    .filter((statement) => statement.length > 0);
}

function runSingleUser(statements) {
  const file = path.join(workDir, `batch-${Math.random().toString(36).slice(2)}.sql`);
  writeFileSync(file, `${statements.join('\n')}\n`, 'utf8');
  const result = spawnSync(exe('postgres'), ['--single', '-D', workDir, '-E', 'postgres'], {
    input: readFileSync(file, 'utf8'),
    encoding: 'utf8',
  });
  const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
  if (result.status !== 0 || /ERROR:/u.test(output)) {
    console.error(`[ledger-drill] a statement batch failed:\n${output}`);
    process.exit(1);
  }
  return output;
}

function readSql(relativePath) {
  return readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

try {
  mkdirSync(workDir, { recursive: true });

  const init = spawnSync(exe('initdb'), ['-D', workDir, '-U', 'postgres', '--auth=trust', '-E', 'UTF8', '--locale=C'], {
    encoding: 'utf8',
  });
  if (init.status !== 0) {
    console.error(`[ledger-drill] initdb failed:\n${init.stdout ?? ''}${init.stderr ?? ''}`);
    process.exit(1);
  }

  runSingleUser(flattenToStatements(readSql(BASELINE)));
  runSingleUser(flattenToStatements(readSql(MIGRATION)));

  const drillOutput = runSingleUser(flattenToStatements(readSql(DRILL)));

  const verdicts = [...drillOutput.matchAll(/verdict = "([a-z]+)"/gu)].map((match) => match[1]);
  const mismatches = verdicts.filter((verdict) => verdict !== 'ok');
  if (verdicts.length !== EXPECTED_VERDICTS) {
    console.error(
      `[ledger-drill] expected ${EXPECTED_VERDICTS} verdict rows, saw ${verdicts.length}. The drill and the schema likely drifted.`,
    );
    process.exit(1);
  }
  if (mismatches.length > 0) {
    console.error(`[ledger-drill] ${mismatches.length} check(s) reported mismatch: ${mismatches.join(', ')}`);
    process.exit(1);
  }

  // Independently re-derive the invariants from the final state, so a drill that
  // stopped asserting (for example a removed SELECT) cannot pass silently.
  const aggregate = [
    "SELECT CASE WHEN",
    '(SELECT available_amount FROM acct_account WHERE id = 1) = 41437265',
    'AND (SELECT frozen_amount FROM acct_account WHERE id = 1) = 33256530',
    'AND (SELECT COALESCE(SUM(amount), 0) FROM acct_ledger_entry WHERE account_id = 1) = 22297',
    'AND (SELECT COALESCE(MAX(balance_after), 0) FROM acct_ledger_entry WHERE account_id = 1) = 41437265',
    'AND (SELECT count(*) FROM acct_hold WHERE settled_amount < 0 OR released_amount < 0',
    'OR settled_amount + released_amount > amount) = 0',
    'AND (SELECT available_amount + frozen_amount FROM acct_account WHERE id = 1) = 74716092 - 22297',
    "AND (SELECT count(*) FROM acct_hold WHERE account_id = 1",
    "AND uuid IN ('drill-hold-settled', 'drill-hold-abandoned') AND status NOT IN (2, 4)) = 0",
    'AND (SELECT frozen_amount FROM acct_account WHERE id = 1) =',
    '(SELECT COALESCE(SUM(amount), 0) FROM acct_hold WHERE account_id = 1 AND status = 1)',
    "THEN 'ok' ELSE 'fail' END AS result;",
  ].join(' ');
  const aggregateOutput = runSingleUser([aggregate]);
  if (!/result = "ok"/u.test(aggregateOutput)) {
    console.error(`[ledger-drill] the independent invariant re-check failed:\n${aggregateOutput}`);
    process.exit(1);
  }

  console.log(`[ledger-drill] ${EXPECTED_VERDICTS} drill checks and the invariant re-check all passed`);
} finally {
  rmSync(workDir, { recursive: true, force: true });
}
