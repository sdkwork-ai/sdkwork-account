#!/usr/bin/env node
/**
 * Runs the money-path domain tests without cargo.
 *
 * `cargo test` cannot run on every machine that develops this repository: the
 * database driver in the dependency graph needs a native TLS library that fails to
 * link on some hosts (MSVC `LNK1104` on `aws-lc-sys`). The two crates that own the
 * funding rules have no such dependency — `sdkwork-account-service` depends only on
 * `sdkwork-contract-service` — so they can be compiled directly with `rustc` and
 * exercised on any host with a Rust toolchain.
 *
 * This script is the entry point for that: build the contract crate as an rlib, then
 * build the account service crate as a test binary against it, then run it. It is a
 * development aid, not a replacement for `cargo test`, which stays the authority on
 * the build host.
 */

import { mkdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const workDir = path.join(tmpdir(), 'sdkwork-account-domain-tests');

const contractCrate = path.join(repoRoot, 'crates', 'sdkwork-contract-service', 'src', 'lib.rs');
const accountCrate = path.join(repoRoot, 'crates', 'sdkwork-account-service', 'src', 'lib.rs');
const contractRlib = path.join(workDir, 'libsdkwork_contract_service.rlib');
const testsBinary = path.join(workDir, process.platform === 'win32' ? 'account-service-tests.exe' : 'account-service-tests');

function run(command, args, label) {
  const result = spawnSync(command, args, { stdio: 'inherit', cwd: repoRoot });
  if (result.error) {
    console.error(`[domain-tests] ${label} could not start: ${result.error.message}`);
    process.exit(1);
  }
  if (result.status !== 0) {
    console.error(`[domain-tests] ${label} failed with exit code ${result.status}`);
    process.exit(result.status ?? 1);
  }
}

/**
 * Two crates are compiled, so a stale rlib from an older contract would silently
 * hide a change; the directory is recreated on every run.
 */
rmSync(workDir, { recursive: true, force: true });
mkdirSync(workDir, { recursive: true });

run(
  'rustc',
  ['--edition', '2021', '--crate-type', 'rlib', '--crate-name', 'sdkwork_contract_service', '-o', contractRlib, contractCrate],
  'contract service rlib',
);

run(
  'rustc',
  [
    '--edition',
    '2021',
    '--test',
    '--crate-name',
    'sdkwork_account_service',
    '--extern',
    `sdkwork_contract_service=${contractRlib}`,
    '-o',
    testsBinary,
    accountCrate,
  ],
  'account service test binary',
);

run(testsBinary, [], 'account service tests');

rmSync(workDir, { recursive: true, force: true });
console.log('[domain-tests] funding-rule tests passed');
