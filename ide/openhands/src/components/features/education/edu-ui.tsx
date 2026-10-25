import { useRef } from "react";
import type { ReactNode } from "react";
import { cn } from "#/utils/utils";

/**
 * Primitives za ELIMU — hufuata tokens za theme ya Agent Canvas (hakuna rangi
 * zilizoandikwa kwa mkono, ili UI ibaki sawa na sehemu nyingine za app).
 */

export function EduCard({
  children,
  className,
  testId,
}: {
  children: ReactNode;
  className?: string;
  testId?: string;
}) {
  return (
    <div
      data-testid={testId}
      className={cn(
        "rounded-lg border border-border bg-surface p-4 shadow-sm",
        className,
      )}
    >
      {children}
    </div>
  );
}

/**
 * Kadi ya 3D — huinuka na kuzunguka kwa mwendo wa kursa (mouse) na kwa
 * keyboard (focus). Hupunguza mwendo kwa `prefers-reduced-motion`.
 */
export function EduCard3D({
  children,
  className,
  testId,
  accent,
}: {
  children: ReactNode;
  className?: string;
  testId?: string;
  accent?: boolean;
}) {
  const ref = useRef<HTMLDivElement | null>(null);

  const onMove = (clientX: number, clientY: number) => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const px = (clientX - r.left) / r.width - 0.5;
    const py = (clientY - r.top) / r.height - 0.5;
    el.style.setProperty("--c3d-rx", `${(-py * 10).toFixed(2)}deg`);
    el.style.setProperty("--c3d-ry", `${(px * 12).toFixed(2)}deg`);
    el.style.setProperty("--c3d-tz", "18px");
  };

  const reset = () => {
    const el = ref.current;
    if (!el) return;
    el.style.setProperty("--c3d-rx", "0deg");
    el.style.setProperty("--c3d-ry", "0deg");
    el.style.setProperty("--c3d-tz", "0px");
  };

  return (
    <div className="mtechos-3d-scene">
      <div
        ref={ref}
        data-testid={testId}
        onMouseMove={(e) => onMove(e.clientX, e.clientY)}
        onMouseLeave={reset}
        onFocus={(e) => {
          const r = e.currentTarget.getBoundingClientRect();
          onMove(r.left + r.width / 2, r.top + r.height / 2);
        }}
        onBlur={reset}
        tabIndex={0}
        className={cn(
          "mtechos-3d-card rounded-lg border bg-surface p-4 shadow-md",
          accent ? "border-primary/60" : "border-border",
          className,
        )}
      >
        {children}
      </div>
    </div>
  );
}

export function EduButton({
  children,
  onClick,
  disabled,
  variant = "primary",
  testId,
  type = "button",
  className,
}: {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  variant?: "primary" | "ghost";
  testId?: string;
  type?: "button" | "submit";
  className?: string;
}) {
  const styles =
    variant === "primary"
      ? "bg-primary text-on-primary hover:opacity-90"
      : "border border-border text-content hover:bg-surface-raised";
  return (
    <button
      type={type}
      data-testid={testId}
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "inline-flex items-center justify-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-50",
        styles,
        className,
      )}
    >
      {children}
    </button>
  );
}

export function EduField({
  label,
  children,
  hint,
}: {
  label: string;
  children: ReactNode;
  hint?: string;
}) {
  return (
    <label className="flex flex-col gap-1 text-sm">
      <span className="font-medium text-content">{label}</span>
      {children}
      {hint ? <span className="text-xs text-muted">{hint}</span> : null}
    </label>
  );
}

const inputClass =
  "rounded-md border border-border bg-surface-raised px-2.5 py-1.5 text-sm text-content outline-none focus:border-focus disabled:opacity-60";

export function EduInput(
  props: React.InputHTMLAttributes<HTMLInputElement> & { testId?: string },
) {
  const { testId, className, ...rest } = props;
  return (
    <input
      data-testid={testId}
      className={cn(inputClass, className)}
      {...rest}
    />
  );
}

export function EduSelect(
  props: React.SelectHTMLAttributes<HTMLSelectElement> & { testId?: string },
) {
  const { testId, className, children, ...rest } = props;
  return (
    <select
      data-testid={testId}
      className={cn(inputClass, className)}
      {...rest}
    >
      {children}
    </select>
  );
}

export function EduTextarea(
  props: React.TextareaHTMLAttributes<HTMLTextAreaElement> & {
    testId?: string;
  },
) {
  const { testId, className, ...rest } = props;
  return (
    <textarea
      data-testid={testId}
      className={cn(inputClass, "min-h-24 resize-y", className)}
      {...rest}
    />
  );
}

export function EduBadge({
  children,
  testId,
}: {
  children: ReactNode;
  testId?: string;
}) {
  return (
    <span
      data-testid={testId}
      className="rounded-full bg-surface-raised px-2 py-0.5 text-xs text-muted"
    >
      {children}
    </span>
  );
}

export function EduSpinner({ label }: { label?: string }) {
  return (
    <div className="flex items-center gap-2 text-sm text-muted">
      <span className="size-4 animate-spin rounded-full border-2 border-border border-t-primary" />
      {label ? <span>{label}</span> : null}
    </div>
  );
}

export function EduError({ message }: { message: string }) {
  return (
    <div
      data-testid="edu-error"
      className="rounded-md border border-feedback-error/40 bg-feedback-error/10 px-3 py-2 text-sm text-feedback-error"
    >
      {message}
    </div>
  );
}

export function EduEmpty({ children }: { children: ReactNode }) {
  return (
    <p className="py-6 text-center text-sm text-muted">{children}</p>
  );
}
