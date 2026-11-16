import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Wallet } from "lucide-react";

import { formatWalletAmount, formatWalletInteger, formatWalletTimestamp } from "./format";
import {
  getAccountWalletRuntime,
  type AccountWalletLedgerEntry,
  type AccountWalletOverview,
} from "./runtime";

interface ScreenState {
  error: string | null;
  ledger: readonly AccountWalletLedgerEntry[];
  loading: boolean;
  overview: AccountWalletOverview | null;
}

/**
 * Cash wallet screen: balance hero (cash + points), cash ledger and the
 * host-provided status footnote. Every read goes through the injected
 * {@link AccountWalletRuntime}; a missing runtime renders a load failure rather
 * than fake numbers.
 */
export function AccountWalletWalletPage() {
  const { t } = useTranslation();
  const [state, setState] = useState<ScreenState>({
    error: null,
    ledger: [],
    loading: true,
    overview: null,
  });

  const load = useCallback(async () => {
    const runtime = getAccountWalletRuntime();
    if (!runtime) {
      setState((previous) => ({
        ...previous,
        error: t("accountWallet.unavailable", "钱包能力尚未接入"),
        loading: false,
      }));
      return;
    }
    setState((previous) => ({ ...previous, error: null, loading: true }));
    try {
      const [overview, ledger] = await Promise.all([runtime.loadOverview(), runtime.loadCashLedger()]);
      setState({ error: null, ledger, loading: false, overview });
    } catch (cause: unknown) {
      setState((previous) => ({
        ...previous,
        error: cause instanceof Error ? cause.message : t("accountWallet.loadFailed", "钱包加载失败"),
        loading: false,
      }));
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="min-h-full bg-bg-color pb-6">
      <h1 className="sr-only">{t("accountWallet.title", "钱包")}</h1>

      {state.loading ? (
        <p className="py-10 text-center text-[13px] text-text-sub">{t("accountWallet.loading", "加载钱包...")}</p>
      ) : null}

      {state.error ? (
        <div className="mx-3 mt-3 flex flex-col items-center gap-3 rounded-xl bg-chat-other-bg p-4 text-center">
          <p className="text-[13px] text-text-sub">{state.error}</p>
          <button
            className="min-h-9 rounded-full border border-border-color px-4 py-1.5 text-[13px] text-text-main active:bg-active-bg"
            onClick={() => void load()}
            type="button"
          >
            {t("accountWallet.retry", "重试")}
          </button>
        </div>
      ) : null}

      {!state.loading && !state.error ? (
        <>
          <section className="mx-3 mt-3 rounded-xl bg-primary-blue p-5 text-white">
            <p className="text-[12px] opacity-85">{t("accountWallet.cashLabel", "现金余额（元）")}</p>
            <p className="mt-1 text-[30px] font-bold leading-tight">
              ¥{formatWalletAmount(state.overview?.cashBalanceCny)}
            </p>
            <div className="mt-4 flex items-baseline justify-between border-t border-white/20 pt-3">
              <span className="text-[12px] opacity-85">{t("accountWallet.pointsLabel", "积分余额")}</span>
              <strong className="text-[17px] font-bold">{formatWalletInteger(state.overview?.pointsBalance)}</strong>
            </div>
          </section>

          <section className="mt-4 px-3">
            <h2 className="mb-2 text-[15px] font-semibold text-text-main">
              {t("accountWallet.ledgerTitle", "现金流水")}
            </h2>
            {state.ledger.length === 0 ? (
              <div className="flex flex-col items-center gap-2 rounded-xl bg-chat-other-bg py-10">
                <span className="flex h-11 w-11 items-center justify-center rounded-full bg-fill text-text-sub">
                  <Wallet aria-hidden="true" size={20} />
                </span>
                <p className="text-[13px] text-text-sub">{t("accountWallet.ledgerEmpty", "暂无流水记录")}</p>
              </div>
            ) : (
              <div className="rounded-xl bg-chat-other-bg px-3">
                {state.ledger.map((entry) => {
                  const positive = (entry.amountCny ?? 0) >= 0;
                  return (
                    <div
                      className="flex items-center gap-3 border-b border-border-color py-3 last:border-b-0"
                      key={entry.id}
                    >
                      <span
                        className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-[13px] font-semibold ${
                          positive ? "bg-primary-blue/10 text-primary-blue" : "bg-fill text-text-sub"
                        }`}
                        aria-hidden="true"
                      >
                        {positive ? "收" : "支"}
                      </span>
                      <div className="min-w-0 flex-1">
                        <p className="truncate text-[13px] text-text-main">{entry.summary}</p>
                        {entry.occurredAt ? (
                          <p className="text-[11px] text-text-sub">{formatWalletTimestamp(entry.occurredAt)}</p>
                        ) : null}
                      </div>
                      <strong
                        className={`whitespace-nowrap text-[14px] font-semibold ${
                          positive ? "text-primary-blue" : "text-text-main"
                        }`}
                      >
                        {entry.amountCny != null
                          ? `${positive ? "+" : "-"}¥${formatWalletAmount(Math.abs(entry.amountCny))}`
                          : "--"}
                      </strong>
                    </div>
                  );
                })}
              </div>
            )}
          </section>

          <p className="mt-4 px-3 text-[12px] text-text-sub">
            {t("accountWallet.footnote", "充值与提现将在支付渠道接入后开放。")}
          </p>
        </>
      ) : null}
    </div>
  );
}
