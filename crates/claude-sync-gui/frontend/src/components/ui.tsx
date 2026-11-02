import { useEffect, useRef, type ButtonHTMLAttributes, type ReactNode } from "react";

type ButtonVariant = "default" | "primary" | "danger";

const BUTTON_VARIANTS: Record<ButtonVariant, string> = {
  default:
    "border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] text-[var(--color-fg-default)] hover:bg-[var(--color-btn-hover-bg)]",
  primary:
    "border border-transparent bg-[var(--color-btn-primary-bg)] text-white hover:bg-[var(--color-btn-primary-hover-bg)]",
  danger:
    "border border-[var(--color-btn-border)] bg-[var(--color-btn-bg)] text-[var(--color-danger-fg)] hover:border-[var(--color-danger-fg)] hover:bg-[var(--color-danger-fg)] hover:text-white",
};

export function Button({
  variant = "default",
  className = "",
  type = "button",
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant }) {
  return (
    <button
      type={type}
      className={`rounded-md px-3 py-1 text-xs font-medium whitespace-nowrap transition ${BUTTON_VARIANTS[variant]} ${className}`}
      {...rest}
    />
  );
}

export type BadgeTone = "neutral" | "success" | "attention" | "danger" | "accent";

const BADGE_TONES: Record<BadgeTone, string> = {
  neutral: "border-[var(--color-border-default)] text-[var(--color-fg-muted)]",
  success: "border-[#1a7f3766] text-[var(--color-success-fg)]",
  attention: "border-[#9a670066] text-[var(--color-attention-fg)]",
  danger: "border-[#d1242f66] text-[var(--color-danger-fg)]",
  accent: "border-[#0969da66] text-[var(--color-accent-fg)]",
};

export function Badge({ tone = "neutral", children }: { tone?: BadgeTone; children: ReactNode }) {
  return (
    <span
      className={`inline-flex items-center rounded-full border px-2 py-px text-[11px] leading-[18px] font-medium whitespace-nowrap ${BADGE_TONES[tone]}`}
    >
      {children}
    </span>
  );
}

export function Panel({
  title,
  actions,
  children,
  className = "",
}: {
  title?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section
      className={`overflow-hidden rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] ${className}`}
    >
      {(title || actions) && (
        <header className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-4 py-2.5">
          <h2 className="m-0 text-sm font-semibold">{title}</h2>
          {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
        </header>
      )}
      {children}
    </section>
  );
}

export function Checkbox({
  checked,
  indeterminate = false,
  onChange,
  disabled,
  label,
}: {
  checked: boolean;
  indeterminate?: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  label?: string;
}) {
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (ref.current) ref.current.indeterminate = indeterminate;
  }, [indeterminate]);
  return (
    <input
      ref={ref}
      type="checkbox"
      aria-label={label}
      checked={checked}
      disabled={disabled}
      onChange={(e) => onChange(e.target.checked)}
      className="h-3.5 w-3.5 shrink-0 accent-[var(--color-accent-fg)]"
    />
  );
}

export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
}) {
  return (
    <label className="toggle-switch" aria-label={label}>
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span className="slider" />
    </label>
  );
}

export function EmptyRow({ children }: { children: ReactNode }) {
  return <div className="px-4 py-8 text-center text-sm text-[var(--color-fg-muted)]">{children}</div>;
}

export function Notice({ tone, children }: { tone: "attention" | "danger" | "accent"; children: ReactNode }) {
  const tones = {
    attention: "border-[#d4a72c66] bg-[#fff8c5] text-[var(--color-fg-default)]",
    danger: "border-[#ff818266] bg-[#ffebe9] text-[var(--color-fg-default)]",
    accent: "border-[#54aeff66] bg-[#ddf4ff] text-[var(--color-fg-default)]",
  };
  return <div className={`rounded-md border px-4 py-3 text-sm ${tones[tone]}`}>{children}</div>;
}

export const inputClass =
  "rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas-default)] px-2.5 py-1 text-sm text-[var(--color-fg-default)]";

export const tableHeadClass =
  "border-b border-[var(--color-border-default)] bg-[var(--color-canvas-subtle)] px-4 py-2 text-left text-xs font-semibold text-[var(--color-fg-muted)]";

export const tableCellClass = "border-b border-[var(--color-border-muted)] px-4 py-2 align-top";
