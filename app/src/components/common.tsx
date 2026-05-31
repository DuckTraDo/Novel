import { ReactNode } from "react";

export function Spinner() {
  return <span className="spinner" aria-label="处理中" />;
}

export function LogBox({ log }: { log: string }) {
  if (!log) return null;
  return <pre className="logbox">{log}</pre>;
}

export function Banner({
  kind,
  children,
}: {
  kind: "ok" | "err" | "info";
  children: ReactNode;
}) {
  return <div className={`banner banner-${kind}`}>{children}</div>;
}

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      {children}
      {hint ? <span className="field-hint">{hint}</span> : null}
    </label>
  );
}
