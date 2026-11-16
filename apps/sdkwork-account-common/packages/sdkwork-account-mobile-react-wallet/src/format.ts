/** Locale-stable formatting shared by the wallet and points screens. */
export function formatWalletAmount(value: number | null | undefined): string {
  return (value ?? 0).toLocaleString("zh-CN", {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
  });
}

export function formatWalletInteger(value: number | null | undefined): string {
  return (value ?? 0).toLocaleString("zh-CN");
}

export function formatWalletTimestamp(value: string | undefined): string {
  if (!value) {
    return "";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
