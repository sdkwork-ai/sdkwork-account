import type { ReactNode } from "react";

/**
 * Small presentational pieces shared by the account centre pages.
 *
 * They carry no business meaning beyond layout and the three data states every
 * operator table needs (loading, empty, failure), so the pages stay readable and
 * behave consistently. Styling uses host design-system classes only; this package
 * defines no global tokens or layout.
 */

export function AdminSection({
  title,
  subtitle,
  actions,
  children,
}: {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-4">
      <header className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <h1 className="text-lg font-semibold">{title}</h1>
          {subtitle === undefined ? null : <p className="text-sm opacity-70">{subtitle}</p>}
        </div>
        {actions}
      </header>
      {children}
    </section>
  );
}

export function AdminFailure({ label, message }: { label: string; message: string }) {
  return (
    <p role="alert" className="rounded border border-red-300 px-3 py-2 text-sm text-red-700">
      {label}: {message}
    </p>
  );
}

export function AdminEmpty({ children }: { children: ReactNode }) {
  return <p className="text-sm opacity-70">{children}</p>;
}

export function AdminLoading({ children }: { children: ReactNode }) {
  return (
    <p role="status" className="text-sm opacity-70">
      {children}
    </p>
  );
}

export function AdminTable({
  columns,
  children,
}: {
  columns: readonly string[];
  children: ReactNode;
}) {
  return (
    <table className="w-full text-sm">
      <thead>
        <tr>
          {columns.map((column) => (
            <th key={column} scope="col" className="text-left font-medium">
              {column}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>{children}</tbody>
    </table>
  );
}

export function AdminSummary({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded border px-3 py-2">
      <div className="text-xs opacity-70">{label}</div>
      <div className="text-base font-semibold">{value}</div>
    </div>
  );
}
