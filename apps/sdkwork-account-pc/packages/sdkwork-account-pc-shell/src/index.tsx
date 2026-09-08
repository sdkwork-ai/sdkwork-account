import { Suspense, lazy, useEffect } from "react";
import { SdkworkThemeProvider } from "@sdkwork/ui-pc-react/theme";
import { bootstrapSdkworkAccountPcSdk } from "@sdkwork/account-pc-core/sdk";
import { resolveBaseUrl } from "@sdkwork/sdk-common";

const env = (import.meta as ImportMeta & { env?: Record<string, string | undefined> }).env;

const DEFAULT_ACCOUNT_API_BASE =
  env?.VITE_SDKWORK_ACCOUNT_API_BASE ?? resolveBaseUrl().url;
const DEFAULT_PAYMENT_CHECKOUT_BASE = env?.VITE_SDKWORK_PAYMENT_CHECKOUT_BASE ?? "/checkout";
const DEFAULT_ORDER_WITHDRAWAL_REQUEST_BASE =
  env?.VITE_SDKWORK_ORDER_WITHDRAWAL_REQUEST_BASE ?? "/withdrawals/requests";

const SdkworkWalletPage = lazy(async () => {
  const module = await import("@sdkwork/account-pc-wallet");
  return { default: module.SdkworkWalletPage };
});

function navigateCommerceRoute(route: string): void {
  if (/^https?:\/\//u.test(route)) {
    window.location.assign(route);
    return;
  }

  window.location.assign(route);
}

export function AccountAppShell() {
  useEffect(() => {
    bootstrapSdkworkAccountPcSdk({
      baseUrl: DEFAULT_ACCOUNT_API_BASE,
    });
  }, []);

  return (
    <SdkworkThemeProvider defaultTheme="light">
      <Suspense fallback={<div role="status">Loading...</div>}>
        <SdkworkWalletPage
          checkoutBasePath={DEFAULT_PAYMENT_CHECKOUT_BASE}
          onNavigate={navigateCommerceRoute}
          rechargeFlow="checkout"
          withdrawalFlow="checkout"
          withdrawalRequestBasePath={DEFAULT_ORDER_WITHDRAWAL_REQUEST_BASE}
        />
      </Suspense>
    </SdkworkThemeProvider>
  );
}
