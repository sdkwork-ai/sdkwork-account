import assert from 'node:assert/strict';
import { it } from 'vitest';

import { SDKWORK_ACCOUNTS_ADMIN_MESSAGES } from '../src/i18n/index.ts';
import { sdkworkAccountPcAdminAccountsRoutes } from '../src/routes.ts';

function flatten(value: unknown, prefix = ""): string[] {
  if (value === null || typeof value !== "object") return [prefix];
  return Object.entries(value as Record<string, unknown>).flatMap(([key, child]) =>
    flatten(child, prefix === "" ? key : `${prefix}.${key}`),
  );
}

function lookup(messages: unknown, key: string): unknown {
  return key.split(".").reduce<unknown>((node, part) => {
    if (node === null || typeof node !== "object") return undefined;
    return (node as Record<string, unknown>)[part];
  }, messages);
}

const locales = Object.keys(SDKWORK_ACCOUNTS_ADMIN_MESSAGES) as Array<keyof typeof SDKWORK_ACCOUNTS_ADMIN_MESSAGES>;

it('every locale exposes the same message keys', () => {
  const [first, ...rest] = locales;
  const expected = flatten(SDKWORK_ACCOUNTS_ADMIN_MESSAGES[first]).sort();
  for (const locale of rest) {
    const actual = flatten(SDKWORK_ACCOUNTS_ADMIN_MESSAGES[locale]).sort();
    assert.deepEqual(actual, expected, `locale ${locale} key set drifted`);
  }
  assert.ok(expected.length > 60, 'the catalogue covers every view');
});

it('no message is empty or left as a placeholder', () => {
  for (const locale of locales) {
    for (const key of flatten(SDKWORK_ACCOUNTS_ADMIN_MESSAGES[locale])) {
      const value = lookup(SDKWORK_ACCOUNTS_ADMIN_MESSAGES[locale], key);
      assert.equal(typeof value, 'string', `${locale}.${key} must be a string`);
      assert.ok(String(value).trim().length > 0, `${locale}.${key} must not be empty`);
      assert.ok(!String(value).includes('TODO'), `${locale}.${key} must not be a TODO`);
    }
  }
});

it('every route title key resolves in both locales', () => {
  for (const route of sdkworkAccountPcAdminAccountsRoutes) {
    const relative = route.titleKey.replace(/^accountsAdmin\./, "");
    for (const locale of locales) {
      const value = lookup(SDKWORK_ACCOUNTS_ADMIN_MESSAGES[locale], relative);
      assert.equal(
        typeof value, 'string',
        `${locale} is missing a translation for ${route.titleKey}`,
      );
    }
  }
});

it('route records only declare the backend-admin surface', () => {
  for (const route of sdkworkAccountPcAdminAccountsRoutes) {
    assert.equal(route.surface, "backend-admin");
    assert.ok(route.path.startsWith("/admin/accounts"), route.path);
  }
});
