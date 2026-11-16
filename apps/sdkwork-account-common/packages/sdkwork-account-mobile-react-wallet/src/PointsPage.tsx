import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Sparkles } from "lucide-react";

import { formatWalletInteger, formatWalletTimestamp } from "./format";
import {
  getAccountWalletRuntime,
  type AccountWalletPointsSummary,
} from "./runtime";

interface ScreenState {
  error: string | null;
  loading: boolean;
  summary: AccountWalletPointsSummary | null;
}

/**
 * Points screen: available-points hero plus the expiring point lots returned by
 * the host. Reads through {@link AccountWalletRuntime}; no host-local point
 * budget logic.
 */
export function AccountWalletPointsPage() {
  const { t } = useTranslation();
  const [state, setState] = useState<ScreenState>({ error: null, loading: true, summary: null });

  const load = useCallback(async () => {
    const runtime = getAccountWalletRuntime();
    if (!runtime) {
      setState((previous) => ({
        ...previous,
        error: t("accountWallet.unavailable", "积分能力尚未接入"),
        loading: false,
      }));
      return;
    }
    setState((previous) => ({ ...previous, error: null, loading: true }));
    try {
      setState({ error: null, loading: false, summary: await runtime.loadPoints() });
    } catch (cause: unknown) {
      setState((previous) => ({
        ...previous,
        error: cause instanceof Error ? cause.message : t("accountWallet.pointsLoadFailed", "积分加载失败"),
        loading: false,
      }));
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="min-h-full bg-bg-color pb-6">
      <h1 className="sr-only">{t("accountWallet.pointsTitle", "我的积分")}</h1>

      {state.loading ? (
        <p className="py-10 text-center text-[13px] text-text-sub">{t("accountWallet.pointsLoading", "加载积分...")}</p>
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
            <p className="text-[12px] opacity-85">{t("accountWallet.pointsAvailable", "可用积分")}</p>
            <p className="mt-1 text-[30px] font-bold leading-tight">
              {formatWalletInteger(state.summary?.balance)}
              <span className="ml-1 text-[14px] font-medium opacity-85">{t("accountWallet.pointsUnit", "分")}</span>
            </p>
            <div className="mt-4 border-t border-white/20 pt-3">
              <span className="text-[12px] opacity-85">
                {t("accountWallet.pointsHint", "积分可在下单时抵扣现金")}
              </span>
            </div>
          </section>

          <section className="mt-4 px-3">
            <h2 className="mb-2 text-[15px] font-semibold text-text-main">
              {t("accountWallet.lotsTitle", "积分批次")}
            </h2>
            {(state.summary?.lots.length ?? 0) === 0 ? (
              <div className="flex flex-col items-center gap-2 rounded-xl bg-chat-other-bg py-10">
                <span className="flex h-11 w-11 items-center justify-center rounded-full bg-fill text-text-sub">
                  <Sparkles aria-hidden="true" size={20} />
                </span>
                <p className="text-[13px] text-text-sub">{t("accountWallet.lotsEmpty", "暂无积分批次")}</p>
              </div>
            ) : (
              <div className="rounded-xl bg-chat-other-bg px-3">
                {state.summary?.lots.map((lot) => (
                  <div
                    className="flex items-center gap-3 border-b border-border-color py-3 last:border-b-0"
                    key={lot.id}
                  >
                    <span
                      aria-hidden="true"
                      className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-primary-blue/10 text-[12px] font-semibold text-primary-blue"
                    >
                      积
                    </span>
                    <div className="min-w-0 flex-1">
                      <p className="text-[13px] text-text-main">
                        {formatWalletInteger(lot.points)} {t("accountWallet.pointsUnit", "分")}
                      </p>
                      <p className="text-[11px] text-text-sub">
                        {lot.expiresAt
                          ? t("accountWallet.lotExpires", "{{date}} 到期", {
                              date: formatWalletTimestamp(lot.expiresAt),
                            })
                          : t("accountWallet.lotLongTerm", "长期有效")}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>
        </>
      ) : null}
    </div>
  );
}
