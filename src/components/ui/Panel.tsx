import type { ReactNode } from "react";

export function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="rounded-lg border border-white/10 bg-card p-5">
      <h3 className="mb-4 font-semibold">{title}</h3>
      {children}
    </div>
  );
}

export const fieldClass =
  "h-10 rounded-md border border-white/10 bg-white/5 px-3 text-sm text-slate-100 outline-none ring-primary/30 placeholder:text-slate-500 focus:ring-2";

export const textareaClass =
  "min-h-20 rounded-md border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none ring-primary/30 placeholder:text-slate-500 focus:ring-2";

export const primaryButtonClass =
  "rounded-md bg-primary px-4 py-2 text-sm font-medium text-slate-950 disabled:cursor-not-allowed disabled:opacity-50";

export const secondaryButtonClass =
  "rounded-md border border-white/10 px-3 py-2 text-sm text-slate-200 hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-50";

export const codeClass =
  "font-mono rounded bg-white/5 px-1.5 py-0.5 text-xs text-sky-200";
